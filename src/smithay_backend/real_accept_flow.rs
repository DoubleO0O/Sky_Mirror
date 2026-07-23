//! Phase 51I-C 真实 accept callback 到 core client lifecycle 的 Linux-only 边界。
//!
//! 本模块使用 Smithay re-export 的 calloop 0.14 注册 `ListeningSocketSource`，避免把
//! 该 source 错接到项目现有的 calloop 0.13。socket callback 只把 `UnixStream` 交给
//! [`NestedClientInsertCompileBoundary`]；成功 insertion 产生的纯数据 `Connected`
//! event 随后由 [`NestedClientSessionCoreBridge`] 提交到既有 core seam。
//!
//! 当前代码建立真实 callback 的 compile/runtime boundary，并为 production 路径按
//! owner 生命周期初始化 `wl_compositor` 与 `xdg_wm_base` globals。旧 constructor
//! 继续保持 controlled/probe 语义；无论走哪条路径，既有 readiness 与 render/input/
//! core capability 都不会因为 server-side global bootstrap 而被推高。

use std::{
    collections::{HashMap, VecDeque},
    os::unix::net::UnixStream,
    time::Duration,
};

use smithay::{
    reexports::{
        calloop::{self, EventLoop},
        wayland_server::backend::ClientId as WaylandClientId,
    },
    wayland::socket::ListeningSocketSource,
};

use crate::{
    core::{
        client::ClientId as CoreClientId,
        runtime_bridge::{NestedClientSessionBridgeOutcome, NestedClientSessionCoreBridge},
        state::State,
    },
    smithay_backend::{
        client_insert::NestedClientInsertCompileBoundary,
        client_session::{
            NestedClientSessionEvent, NestedClientSessionEventLog, NestedClientSessionEventRecord,
            NestedClientSessionId,
        },
        linux_live_toplevel_admission_owner::LiveToplevelAdmissionOwnerObservation,
        linux_wl_compositor::LinuxWlCompositorGlobalInitError,
        linux_wl_surface_identity::{AdapterSurfaceCommitObservation, SurfaceIdentityError},
        linux_xdg_shell::LinuxXdgShellGlobalInitError,
        real_disconnect_flow::{NestedRealDisconnectCallbackReport, bridge_disconnected_events},
        wayland_display::SmithayWaylandDisplayProbe,
        wayland_socket::SmithayWaylandSocketProbe,
        xdg_lifecycle_observation::XdgToplevelLifecycleObservationReport,
    },
};

/// Production protocol bootstrap 在 server owner 边界上的只读事实快照。
///
/// 这个报告只描述 flow 持有的 Display 已注册哪些 globals，以及 socket 是否在两项
/// 注册都成功之后才绑定。它不把“server 已发布 global”外推成“外部 client 已发现/
/// bind”，更不能外推到 buffer、renderer、damage、frame callback、input 或 core。
#[must_use = "production protocol bootstrap report 必须按 server/client 观察边界解释"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ProductionProtocolBootstrapReport {
    /// production owner 已实际进入 bootstrap helper。
    pub(crate) bootstrap_attempted: bool,
    /// owner 已成功注册 `wl_compositor` global。
    pub(crate) wl_compositor_initialized: bool,
    /// owner 已在 compositor 之后成功注册 `xdg_wm_base` global。
    pub(crate) xdg_wm_base_initialized: bool,
    /// socket 仅在两个 globals 都成功之后才完成绑定。
    pub(crate) socket_bound_after_bootstrap: bool,
    /// 尚无外部 registry roundtrip 证明同时发现两个 globals。
    pub(crate) external_registry_discovered_both_globals: bool,
    /// 尚无外部 client 成功 bind 两个 globals 的证明。
    pub(crate) external_client_bound_both_globals: bool,
    /// bootstrap 不读取或导入 client buffer。
    pub(crate) buffer_import_attempted: bool,
    /// bootstrap 不声称已有 buffer import 成功。
    pub(crate) buffer_imported: bool,
    /// bootstrap 不创建 renderer texture。
    pub(crate) texture_created: bool,
    /// bootstrap 不调用 renderer。
    pub(crate) renderer_called: bool,
    /// bootstrap 不提交 damage。
    pub(crate) damage_submitted: bool,
    /// bootstrap 不发送 frame callback done。
    pub(crate) frame_callback_done_sent: bool,
    /// globals 可见性不等于 input 支持。
    pub(crate) input_support: bool,
    /// bootstrap 与 socket/source assembly 都不得修改 core state。
    pub(crate) core_mutation_invoked: bool,
}

/// Production global 初始化失败的精确阶段与底层结构化原因。
///
/// 两个 variant 的顺序语义与 bootstrap 顺序一致：compositor 失败会立即短路，只有
/// compositor 成功后才可能出现 `XdgWmBase` 错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionProtocolBootstrapError {
    /// 第一阶段 `wl_compositor` 初始化失败。
    WlCompositor {
        source: LinuxWlCompositorGlobalInitError,
    },
    /// 第二阶段 `xdg_wm_base` 初始化失败。
    XdgWmBase {
        source: LinuxXdgShellGlobalInitError,
    },
}

impl std::fmt::Display for ProductionProtocolBootstrapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WlCompositor { source } => {
                write!(formatter, "wl_compositor global 初始化失败: {source:?}")
            }
            Self::XdgWmBase { source } => {
                write!(formatter, "xdg_wm_base global 初始化失败: {source:?}")
            }
        }
    }
}

impl std::error::Error for ProductionProtocolBootstrapError {}

/// 在同一个 Display owner 上严格按 compositor → xdg 的顺序注册 production globals。
///
/// helper 不接触 socket 或 calloop，因此任一阶段失败时，尚不存在可被外部 client
/// 观察的 socket path；按值持有 Display 的上层 constructor 随错误返回并完成清理。
fn bootstrap_production_protocol_globals(
    display: &mut SmithayWaylandDisplayProbe,
) -> Result<ProductionProtocolBootstrapReport, ProductionProtocolBootstrapError> {
    let compositor = display
        .initialize_wl_compositor_global()
        .map_err(|source| ProductionProtocolBootstrapError::WlCompositor { source })?;
    let xdg = display
        .initialize_xdg_shell_global()
        .map_err(|source| ProductionProtocolBootstrapError::XdgWmBase { source })?;

    Ok(ProductionProtocolBootstrapReport {
        bootstrap_attempted: true,
        wl_compositor_initialized: compositor.wl_compositor_global_initialized,
        xdg_wm_base_initialized: xdg.xdg_shell_global_initialized,
        socket_bound_after_bootstrap: false,
        external_registry_discovered_both_globals: false,
        external_client_bound_both_globals: false,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
    })
}

/// 真实 accept-connected runtime 仍缺失的可独立诊断前置条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRealAcceptConnectedBridgeBlocker {
    /// 尚无 Linux test/CI 证明 callback 实际接收到 listening socket stream。
    MissingRealAcceptLoop,

    /// 尚无 Linux test/CI 证明 accepted stream ownership 已进入 insertion flow。
    MissingAcceptedStreamOwnership,

    /// 尚无 Linux test/CI 证明 accepted stream 成功调用 `insert_client`。
    MissingDisplayHandleInsertClientRuntimeProof,

    /// 尚无 Linux test/CI 证明 backend client identity 到 session 的运行时映射。
    MissingRealClientSessionMapping,

    /// 尚无 Linux test/CI 证明 real insertion 生成的 record 已进入 core bridge。
    MissingConnectedEventBridgeFromRealInsert,

    /// 尚无真实 disconnect callback 到 session event 的验收证明。
    MissingDisconnectCallbackSource,

    /// 当前分支尚无完整 Linux runtime check/test 结果。
    MissingLinuxRuntimeProof,
}

/// Phase 51I-C B 路线的保守 runtime readiness 报告。
///
/// `listening_socket_callback_available` 与 `calloop_version_boundary_clear` 只表示源码
/// 已建立同版本 callback 边界；其余字段在 Linux 真实测试通过前保持 `false`。
#[must_use = "readiness 报告不能代替真实 accepted-client Linux proof"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRealAcceptConnectedBridgeReadinessReport {
    /// 当前仍阻止 C 路线验收的全部 blocker。
    pub blockers: Vec<NestedRealAcceptConnectedBridgeBlocker>,

    /// 是否已定义 `ListeningSocketSource` 的 calloop callback 边界。
    pub listening_socket_callback_available: bool,

    /// Smithay source 与 event loop 是否统一使用其 re-export 的 calloop 0.14。
    pub calloop_version_boundary_clear: bool,

    /// 是否已由真实 runtime 观察到 accepted stream。
    pub accepted_stream_available: bool,

    /// 是否已由 Linux test/CI 证明真实 listening socket callback 被触发。
    pub real_accept_loop_available: bool,

    /// 是否已由真实 runtime 证明 accepted stream 调用 `insert_client` 成功。
    pub display_handle_insert_client_runtime_available: bool,

    /// 是否已有保存 [`NestedClientSessionId`] 的 `ClientData` owner 类型。
    pub client_data_owner_defined: bool,

    /// 是否已由真实 runtime 证明 backend client/session mapping。
    pub inserted_client_mapping_available: bool,

    /// 是否已由真实 insertion event 调用 core bridge。
    pub connected_event_bridged_to_core: bool,

    /// 是否已在真实 insertion bridge 后取得 clean validation。
    pub validation_report_available: bool,

    /// 是否已具备项目级真实 client accept 能力。
    pub accepts_clients: bool,

    /// 是否支持真实 surface；本阶段固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；本阶段固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实 render；本阶段固定为 `false`。
    pub render_support: bool,

    /// 是否启动真实 protocol dispatch；本阶段固定为 `false`。
    pub protocol_dispatch_started: bool,
}

impl NestedRealAcceptConnectedBridgeReadinessReport {
    /// 判断 C 路线需要的真实 accept-connected runtime proof 是否全部成立。
    pub fn is_runtime_ready(&self) -> bool {
        self.blockers.is_empty()
            && self.listening_socket_callback_available
            && self.calloop_version_boundary_clear
            && self.accepted_stream_available
            && self.real_accept_loop_available
            && self.display_handle_insert_client_runtime_available
            && self.client_data_owner_defined
            && self.inserted_client_mapping_available
            && self.connected_event_bridged_to_core
            && self.validation_report_available
            && self.accepts_clients
    }
}

/// 返回当前 B 路线的保守 accept-connected readiness 报告。
#[must_use = "调用方必须检查 blockers 与 runtime capability false 字段"]
pub fn nested_real_accept_connected_bridge_readiness_report()
-> NestedRealAcceptConnectedBridgeReadinessReport {
    NestedRealAcceptConnectedBridgeReadinessReport {
        blockers: vec![
            NestedRealAcceptConnectedBridgeBlocker::MissingRealAcceptLoop,
            NestedRealAcceptConnectedBridgeBlocker::MissingAcceptedStreamOwnership,
            NestedRealAcceptConnectedBridgeBlocker::MissingDisplayHandleInsertClientRuntimeProof,
            NestedRealAcceptConnectedBridgeBlocker::MissingRealClientSessionMapping,
            NestedRealAcceptConnectedBridgeBlocker::MissingConnectedEventBridgeFromRealInsert,
            NestedRealAcceptConnectedBridgeBlocker::MissingDisconnectCallbackSource,
            NestedRealAcceptConnectedBridgeBlocker::MissingLinuxRuntimeProof,
        ],
        listening_socket_callback_available: true,
        calloop_version_boundary_clear: true,
        accepted_stream_available: false,
        real_accept_loop_available: false,
        display_handle_insert_client_runtime_available: false,
        client_data_owner_defined: true,
        inserted_client_mapping_available: false,
        connected_event_bridged_to_core: false,
        validation_report_available: false,
        accepts_clients: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        protocol_dispatch_started: false,
    }
}

/// accepted stream 在 insertion 边界失败时的结构化原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedAcceptedClientFailureReason {
    /// adapter session ID 空间已经耗尽，因此没有调用 insertion。
    SessionIdExhausted,

    /// `DisplayHandle::insert_client` 返回错误；字符串只用于诊断。
    InsertClientFailed {
        /// Wayland backend 返回的错误文本。
        message: String,
    },
}

/// 单个真实 socket callback 的 insertion 结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedAcceptedClientAttempt {
    /// callback 是否已经拥有一个 accepted `UnixStream`。
    pub accepted_stream_observed: bool,

    /// 已分配的 adapter session；ID 耗尽时为空。
    pub session: Option<NestedClientSessionId>,

    /// `insert_client` 是否返回成功。
    pub insert_succeeded: bool,

    /// backend client identity 是否已保存到 adapter mapping。
    pub mapping_saved: bool,

    /// 失败原因；成功时为空。
    pub failure_reason: Option<NestedAcceptedClientFailureReason>,
}

/// Wayland backend client identity 到 adapter session identity 的 Linux-only mapping。
///
/// mapping 不保存 core `ClientId`。core identity 只能由
/// [`NestedClientSessionCoreBridge`] 注册成功后返回，避免 adapter 伪造核心身份。
#[derive(Debug, Clone, Default)]
pub struct NestedAcceptedClientMapping {
    sessions: HashMap<WaylandClientId, NestedClientSessionId>,
}

impl NestedAcceptedClientMapping {
    /// 创建空 backend-client/session mapping。
    pub fn new() -> Self {
        Self::default()
    }

    /// 保存一次成功 insertion 的 backend client/session 关系。
    ///
    /// 如果同一 backend client 已存在，返回旧 session；调用方可据此诊断覆盖。
    pub fn insert(
        &mut self,
        client: WaylandClientId,
        session: NestedClientSessionId,
    ) -> Option<NestedClientSessionId> {
        self.sessions.insert(client, session)
    }

    /// 查询 backend client 当前对应的 adapter session。
    pub fn lookup(&self, client: &WaylandClientId) -> Option<NestedClientSessionId> {
        self.sessions.get(client).copied()
    }

    /// 移除 backend client mapping，并返回原 session。
    pub fn remove(&mut self, client: &WaylandClientId) -> Option<NestedClientSessionId> {
        self.sessions.remove(client)
    }

    /// 按 adapter session 移除 backend client mapping，并返回 backend identity。
    ///
    /// disconnect callback 只携带 owner 保存的 session，因此 coordinator 必须用该
    /// identity 清理 mapping；它不能把 backend ID 数值猜成 core client ID。
    pub fn remove_session(&mut self, session: NestedClientSessionId) -> Option<WaylandClientId> {
        let client = self.sessions.iter().find_map(|(client, mapped_session)| {
            (*mapped_session == session).then(|| client.clone())
        })?;
        self.sessions.remove(&client);
        Some(client)
    }

    /// 返回当前 mapping 数量。
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// 当前 mapping 是否为空。
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

/// 一轮真实 accept callback pump 的结构化结果。
///
/// report 只保存纯数据 attempt、event record、core outcome 和 capability 快照；
/// 不保存 `UnixStream`、Wayland `Client`、Display 或 calloop source。
#[must_use = "pump report 包含 insertion 失败与 core validation 结果，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRealAcceptPumpReport {
    /// 本轮 callback 观察到的全部 accepted-stream insertion attempt。
    pub attempts: Vec<NestedAcceptedClientAttempt>,

    /// 本轮成功 insertion 产生的 connected records。
    pub connected_records: Vec<NestedClientSessionEventRecord>,

    /// connected records 经既有 session/core bridge 得到的结果。
    pub bridge_outcomes: Vec<NestedClientSessionBridgeOutcome>,

    /// 有核心 validation 时表示它们是否全部 clean；无 connected record 时为 `None`。
    pub all_observed_validations_clean: Option<bool>,

    /// 当前阶段的保守 runtime capability 快照。
    pub readiness: NestedRealAcceptConnectedBridgeReadinessReport,
}

impl NestedRealAcceptPumpReport {
    /// 返回本轮真实 callback 观察到的 accepted stream 数量。
    pub fn accepted_stream_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| attempt.accepted_stream_observed)
            .count()
    }

    /// 返回本轮成功 insertion 的数量。
    pub fn inserted_client_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| attempt.insert_succeeded)
            .count()
    }

    /// 返回本轮由 core bridge 成功注册的核心 client IDs。
    pub fn registered_core_clients(&self) -> Vec<CoreClientId> {
        self.bridge_outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                NestedClientSessionBridgeOutcome::Connected { client, .. } => Some(*client),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug)]
struct NestedRealAcceptLoopData {
    insert_boundary: NestedClientInsertCompileBoundary,
    next_session_value: Option<u64>,
    mapping: NestedAcceptedClientMapping,
    attempts: VecDeque<NestedAcceptedClientAttempt>,
}

impl NestedRealAcceptLoopData {
    // callback data 只持有 adapter-owned insertion 与 mapping，不持有 core State。
    fn new(insert_boundary: NestedClientInsertCompileBoundary) -> Self {
        Self {
            insert_boundary,
            next_session_value: Some(1),
            mapping: NestedAcceptedClientMapping::new(),
            attempts: VecDeque::new(),
        }
    }

    // 每个 socket callback 先分配 session；只有 insertion 成功才保存 backend mapping。
    fn accept_stream(&mut self, stream: UnixStream) {
        let Some(session) = self.allocate_session() else {
            self.attempts.push_back(NestedAcceptedClientAttempt {
                accepted_stream_observed: true,
                session: None,
                insert_succeeded: false,
                mapping_saved: false,
                failure_reason: Some(NestedAcceptedClientFailureReason::SessionIdExhausted),
            });
            return;
        };

        match self.insert_boundary.insert_client(stream, session) {
            Ok(client) => {
                let previous = self.mapping.insert(client.id(), session);
                self.attempts.push_back(NestedAcceptedClientAttempt {
                    accepted_stream_observed: true,
                    session: Some(session),
                    insert_succeeded: true,
                    mapping_saved: previous.is_none(),
                    failure_reason: None,
                });
            }
            Err(error) => {
                self.attempts.push_back(NestedAcceptedClientAttempt {
                    accepted_stream_observed: true,
                    session: Some(session),
                    insert_succeeded: false,
                    mapping_saved: false,
                    failure_reason: Some(NestedAcceptedClientFailureReason::InsertClientFailed {
                        message: error.to_string(),
                    }),
                });
            }
        }
    }

    // `Option<u64>` 让耗尽状态显式可见，避免 max ID 后回绕并复用 session identity。
    fn allocate_session(&mut self) -> Option<NestedClientSessionId> {
        let value = self.next_session_value?;
        let session = NestedClientSessionId::new(value)?;
        self.next_session_value = value.checked_add(1);
        Some(session)
    }

    // attempt 只属于当前 pump；取出后保留 persistent insertion boundary 与 mapping。
    fn take_attempts(&mut self) -> Vec<NestedAcceptedClientAttempt> {
        self.attempts.drain(..).collect()
    }
}

/// 真实 listening socket callback 到 connected core lifecycle 的最小 Linux-only flow。
///
/// 该 flow 拥有独立 calloop 0.14 event loop、Display probe、callback data、session log
/// 与 core bridge。调用方仍显式传入 core [`State`]；唯一 mutation 入口是
/// `NestedClientSessionCoreBridge::handle_record`，callback 本身不能访问 core。
pub struct NestedRealAcceptFlow {
    event_loop: EventLoop<'static, NestedRealAcceptLoopData>,
    loop_data: NestedRealAcceptLoopData,
    display: SmithayWaylandDisplayProbe,
    socket_name: String,
    /// `Some` 只属于 production bootstrap 路径；旧 controlled/probe 路径保持 `None`。
    production_protocol_bootstrap_report: Option<ProductionProtocolBootstrapReport>,
    event_log: NestedClientSessionEventLog,
    core_bridge: NestedClientSessionCoreBridge,
}

/// 消费一个已创建的 Display owner，先完成 production globals，再进入 socket assembly。
///
/// Display 按值进入本函数，保证 bootstrap 失败时 owner 在返回 `Err` 前被销毁；由于
/// common assembly 尚未运行，失败路径不会绑定 socket，也不会留下 calloop source。
fn with_production_protocol_display(
    name: &str,
    mut display: SmithayWaylandDisplayProbe,
) -> Result<NestedRealAcceptFlow, Box<dyn std::error::Error>> {
    let report = bootstrap_production_protocol_globals(&mut display)?;
    assemble_real_accept_flow(name, display, Some(report))
}

/// 组装两条 constructor 共用的 socket、source 与 flow owner。
///
/// production 调用者必须先完成两个 globals；这里真实绑定 socket 后立刻记录
/// `socket_bound_after_bootstrap`，再消费 socket 生成 source 并插入 calloop。
/// controlled/probe 调用者传入 `None`，因此旧路径不会伪造 production report。
fn assemble_real_accept_flow(
    name: &str,
    display: SmithayWaylandDisplayProbe,
    mut production_protocol_bootstrap_report: Option<ProductionProtocolBootstrapReport>,
) -> Result<NestedRealAcceptFlow, Box<dyn std::error::Error>> {
    let socket = SmithayWaylandSocketProbe::with_name(name)?;
    let socket_name = socket.socket_name_string();

    // 此赋值发生在真实 bind 成功之后、socket 被消费成 source 之前，精确记录暴露边界。
    if let Some(report) = production_protocol_bootstrap_report.as_mut() {
        report.socket_bound_after_bootstrap = true;
    }

    let socket_source: ListeningSocketSource = socket.into_source();
    let insert_boundary = NestedClientInsertCompileBoundary::new(display.display_handle());
    let event_loop: EventLoop<'static, NestedRealAcceptLoopData> = EventLoop::try_new()?;

    event_loop
        .handle()
        .insert_source(socket_source, |stream, _, data| data.accept_stream(stream))
        .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;

    Ok(NestedRealAcceptFlow {
        event_loop,
        loop_data: NestedRealAcceptLoopData::new(insert_boundary),
        display,
        socket_name,
        production_protocol_bootstrap_report,
        event_log: NestedClientSessionEventLog::new(),
        core_bridge: NestedClientSessionCoreBridge::new(),
    })
}

impl NestedRealAcceptFlow {
    /// 使用指定 Wayland socket 名称创建真实 callback boundary。
    ///
    /// 构造过程会真实绑定 listening socket 并把 source 注册到 Smithay re-export 的
    /// calloop 0.14，但不会自行运行循环、注册 globals 或 dispatch protocol requests；
    /// 因此它继续作为既有 controlled/probe seam，不产生 production bootstrap report。
    ///
    /// # Errors
    ///
    /// Display、socket、event loop 初始化或 source 注册失败时返回原始错误链。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let display = SmithayWaylandDisplayProbe::new()?;
        assemble_real_accept_flow(name, display, None)
    }

    /// 创建 production flow，并在 socket 对外可见之前注册两个 server globals。
    ///
    /// owner 顺序固定为 Display → `wl_compositor` → `xdg_wm_base` → socket bind →
    /// calloop source；任一 global 失败都会在 socket bind 之前返回结构化错误。
    pub(crate) fn with_production_protocol_bootstrap(
        name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let display = SmithayWaylandDisplayProbe::new()?;
        with_production_protocol_display(name, display)
    }

    /// 返回 production server bootstrap 的只读事实；旧 constructor 永远返回 `None`。
    ///
    /// 即使报告为 `Some`，client discovery/bind 与 buffer/render/input/core 字段仍保持
    /// `false`，直到各自边界取得独立运行时证明。
    pub(crate) fn production_protocol_bootstrap_report(
        &self,
    ) -> Option<ProductionProtocolBootstrapReport> {
        self.production_protocol_bootstrap_report
    }

    /// 返回已真实绑定的 Wayland socket 名称。
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    /// 返回可唤醒本 flow calloop poll 的 cloneable signal。
    ///
    /// signal 只中断 event-loop wait，不接触 callback data、Display 或 core state。
    pub(crate) fn loop_signal(&self) -> calloop::LoopSignal {
        self.event_loop.get_signal()
    }

    /// 运行一轮 accept callback，并把成功 insertion 的 connected event 提交到 core。
    ///
    /// `timeout` 只控制本轮 poll 等待；本方法不会进入长期运行循环。callback 只写
    /// adapter data，本方法返回 callback 后才通过既有 bridge 修改 core。
    ///
    /// # Errors
    ///
    /// calloop poll 或 event source processing 失败时返回对应错误。
    pub fn pump_once(
        &mut self,
        state: &mut State,
        timeout: Duration,
    ) -> calloop::Result<NestedRealAcceptPumpReport> {
        self.event_loop.dispatch(timeout, &mut self.loop_data)?;

        let attempts = self.loop_data.take_attempts();
        let connected_events = self
            .loop_data
            .insert_boundary
            .event_queue()
            .drain_connected();
        let (connected_records, bridge_outcomes) = bridge_connected_events(
            connected_events,
            &self.socket_name,
            &mut self.event_log,
            &mut self.core_bridge,
            state,
        );
        let all_observed_validations_clean = observed_validations_clean(&bridge_outcomes);

        Ok(NestedRealAcceptPumpReport {
            attempts,
            connected_records,
            bridge_outcomes,
            all_observed_validations_clean,
            readiness: nested_real_accept_connected_bridge_readiness_report(),
        })
    }

    /// 桥接 callback queue 中当前待处理的 disconnected session events。
    ///
    /// 本方法不会触发或伪造 `ClientData::disconnected`；它只消费 owner callback 已写入
    /// 的事实，并复用 [`NestedClientSessionCoreBridge`] 进入既有 core close seam。
    /// 因当前分支尚无真实 runtime callback 触发证明，返回报告中的真实能力保持 false。
    pub fn bridge_pending_disconnects(
        &mut self,
        state: &mut State,
    ) -> NestedRealDisconnectCallbackReport {
        let events = self
            .loop_data
            .insert_boundary
            .event_queue()
            .drain_disconnected();

        bridge_disconnected_events(
            events,
            &self.socket_name,
            &mut self.event_log,
            &mut self.core_bridge,
            &mut self.loop_data.mapping,
            state,
        )
    }

    /// 让 coordinator 执行一次 Display backend client dispatch。
    ///
    /// 本 seam 只观察 socket readiness/EOF；callback 仍只写 session event，不能直接
    /// 接触 core。dispatch 成功后立即 flush pending client events，但一次调用仍不代表
    /// 长期 protocol dispatch loop 已启动，也不会抬高既有 readiness。
    pub(crate) fn dispatch_wayland_clients_once(&mut self) -> std::io::Result<usize> {
        let dispatched = self.display.dispatch_clients_once()?;
        self.display.flush_clients_once()?;
        Ok(dispatched)
    }

    /// 读取 display owner 中最近一次 live toplevel admission observation 的纯数据快照。
    ///
    /// flow 只暴露 copyable report 数据，不把 Smithay display 或 handler state 泄漏给
    /// ledger/core owner；coordinator 随后才能安全地可变借用自身执行 enqueue/drain。
    pub(crate) fn live_toplevel_admission_observation(
        &self,
    ) -> LiveToplevelAdmissionOwnerObservation {
        LiveToplevelAdmissionOwnerObservation::from_display(&self.display)
    }

    /// 消费 display owner 中下一条 live toplevel admission observation。
    ///
    /// 该 seam 维持 callback arrival order：多个 callback 在一次 coordinator pump 前到达时，
    /// runtime pump 每次只读取并处理最早的一条。
    pub(crate) fn take_next_live_toplevel_admission_observation(
        &mut self,
    ) -> LiveToplevelAdmissionOwnerObservation {
        self.display.take_next_live_toplevel_admission_observation()
    }

    /// 消费 display owner 中下一条 live toplevel unmap observation。
    ///
    /// flow 只转发纯数据 lifecycle report；ledger/core detach 仍必须由 runtime owner
    /// 在同时持有 admission ledger 与 `State` 时执行。
    pub(crate) fn take_next_live_toplevel_unmap_observation(
        &mut self,
    ) -> Option<XdgToplevelLifecycleObservationReport> {
        self.display.take_next_live_toplevel_unmap_observation()
    }

    /// 消费 display owner 中下一条 `wl_surface.commit` observation。
    ///
    /// flow 只转发 adapter-owned pure-data commit observation；这里不读取 buffer、
    /// damage、frame callback，也不调用 render、input、ledger 或 core。
    pub(crate) fn take_next_wl_surface_commit_observation(
        &mut self,
    ) -> Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>> {
        self.display.take_next_wl_surface_commit_observation()
    }

    /// 只读访问 persistent backend-client/session mapping。
    pub fn mapping(&self) -> &NestedAcceptedClientMapping {
        &self.loop_data.mapping
    }

    /// 返回 core bridge 当前保存的 active session 数量。
    pub fn active_core_session_count(&self) -> usize {
        self.core_bridge.active_session_count()
    }

    /// 返回 Display 是否仍由 flow 持有并保持 probe-only protocol 状态。
    pub fn display_is_probe_only(&self) -> bool {
        self.display.is_probe_only()
    }
}

// 该 helper 是唯一 real-insert event -> core seam；它不翻译或直接执行 CoreCommand。
fn bridge_connected_events(
    events: Vec<NestedClientSessionEvent>,
    socket_name: &str,
    event_log: &mut NestedClientSessionEventLog,
    core_bridge: &mut NestedClientSessionCoreBridge,
    state: &mut State,
) -> (
    Vec<NestedClientSessionEventRecord>,
    Vec<NestedClientSessionBridgeOutcome>,
) {
    let mut records = Vec::with_capacity(events.len());
    let mut outcomes = Vec::with_capacity(events.len());

    for event in events {
        let record = event_log
            .record(
                event,
                Some(socket_name.to_string()),
                Some("accepted stream inserted by Linux-only callback boundary".to_string()),
            )
            .clone();
        let outcome = core_bridge.handle_record(state, &record);
        records.push(record);
        outcomes.push(outcome);
    }

    (records, outcomes)
}

// 只汇总实际携带 RuntimeEventResult 的 outcome；没有 core mutation 时返回 None。
fn observed_validations_clean(outcomes: &[NestedClientSessionBridgeOutcome]) -> Option<bool> {
    let validations = outcomes.iter().filter_map(|outcome| match outcome {
        NestedClientSessionBridgeOutcome::Connected { runtime, .. }
        | NestedClientSessionBridgeOutcome::Disconnected { runtime, .. }
        | NestedClientSessionBridgeOutcome::RegistrationFailed { runtime, .. } => {
            Some(runtime.validation.is_clean())
        }
        _ => None,
    });
    let mut observed = false;
    let mut all_clean = true;

    for clean in validations {
        observed = true;
        all_clean &= clean;
    }

    observed.then_some(all_clean)
}

#[cfg(test)]
impl NestedRealAcceptFlow {
    /// 测试专用：让 Linux controlled proof 在 flow 持有的 display 上制造 observation。
    ///
    /// production coordinator 只读取 [`Self::live_toplevel_admission_observation`] 返回的
    /// 纯数据快照；该 mutable accessor 不参与 runtime pump。
    pub(crate) fn display_mut_for_controlled_toplevel_registration(
        &mut self,
    ) -> &mut SmithayWaylandDisplayProbe {
        &mut self.display
    }
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixStream, path::Path, time::Duration};

    use smithay::reexports::wayland_server::Display;

    use super::{
        NestedAcceptedClientFailureReason, NestedAcceptedClientMapping,
        NestedRealAcceptConnectedBridgeBlocker, NestedRealAcceptFlow, NestedRealAcceptLoopData,
        ProductionProtocolBootstrapError, ProductionProtocolBootstrapReport,
        bootstrap_production_protocol_globals, bridge_connected_events,
        nested_real_accept_connected_bridge_readiness_report, with_production_protocol_display,
    };
    use crate::{
        core::{
            backend_event::BackendEvent, command::CoreCommand,
            runtime_bridge::NestedClientSessionBridgeOutcome, state::State,
        },
        smithay_backend::{
            client_insert::NestedClientInsertCompileBoundary,
            client_session::{
                NestedClientSessionEvent, NestedClientSessionEventKind,
                NestedClientSessionEventLog, NestedClientSessionId,
            },
            linux_wl_compositor::LinuxWlCompositorGlobalInitError,
            linux_xdg_shell::LinuxXdgShellGlobalInitError,
            test_support::{assert_runtime_dir, unique_socket_name},
            wayland_display::SmithayWaylandDisplayProbe,
        },
    };

    fn session(value: u64) -> NestedClientSessionId {
        NestedClientSessionId::new(value).expect("测试 session ID 必须非零")
    }

    /// compositor 是 production bootstrap 的第一阶段；该阶段失败必须阻止 xdg 初始化。
    #[test]
    fn production_protocol_bootstrap_compositor_error_is_structured_and_short_circuits_xdg() {
        let mut display = SmithayWaylandDisplayProbe::new().expect("测试 Display 必须能构造");
        display
            .initialize_wl_compositor_global()
            .expect("预置 compositor global 必须成功");

        let error = bootstrap_production_protocol_globals(&mut display)
            .expect_err("重复 compositor 初始化必须返回结构化错误");

        assert_eq!(
            error,
            ProductionProtocolBootstrapError::WlCompositor {
                source: LinuxWlCompositorGlobalInitError::AlreadyInitialized,
            }
        );
        assert!(!display.is_xdg_shell_global_initialized());
    }

    /// xdg 是 production bootstrap 的第二阶段；其失败前 compositor 必须已成功初始化。
    #[test]
    fn production_protocol_bootstrap_xdg_error_occurs_after_compositor_initialization() {
        let mut display = SmithayWaylandDisplayProbe::new().expect("测试 Display 必须能构造");
        display
            .initialize_xdg_shell_global()
            .expect("预置 xdg_wm_base global 必须成功");

        let error = bootstrap_production_protocol_globals(&mut display)
            .expect_err("重复 xdg 初始化必须返回结构化错误");

        assert_eq!(
            error,
            ProductionProtocolBootstrapError::XdgWmBase {
                source: LinuxXdgShellGlobalInitError::AlreadyInitialized,
            }
        );
        assert!(display.is_wl_compositor_global_initialized());
    }

    /// fresh Display 只允许一次完整 bootstrap，production constructor 也只执行一次。
    #[test]
    fn production_protocol_bootstrap_is_exactly_once_and_constructor_reports_server_state() {
        assert_runtime_dir();
        let mut display = SmithayWaylandDisplayProbe::new().expect("测试 Display 必须能构造");

        let report = bootstrap_production_protocol_globals(&mut display)
            .expect("fresh Display 的 production globals 必须初始化成功");
        assert_eq!(
            report,
            ProductionProtocolBootstrapReport {
                bootstrap_attempted: true,
                wl_compositor_initialized: true,
                xdg_wm_base_initialized: true,
                ..ProductionProtocolBootstrapReport::default()
            }
        );
        assert_eq!(
            bootstrap_production_protocol_globals(&mut display),
            Err(ProductionProtocolBootstrapError::WlCompositor {
                source: LinuxWlCompositorGlobalInitError::AlreadyInitialized,
            })
        );

        let socket_name = unique_socket_name("production-protocol-bootstrap-once");
        let flow = NestedRealAcceptFlow::with_production_protocol_bootstrap(&socket_name)
            .expect("production constructor 必须完成一次 globals bootstrap 后再绑定 socket");
        assert_eq!(
            flow.production_protocol_bootstrap_report(),
            Some(ProductionProtocolBootstrapReport {
                bootstrap_attempted: true,
                wl_compositor_initialized: true,
                xdg_wm_base_initialized: true,
                socket_bound_after_bootstrap: true,
                ..ProductionProtocolBootstrapReport::default()
            })
        );
    }

    /// bootstrap 失败时 Display owner 必须销毁，且 socket/source 从未对外暴露或残留。
    #[test]
    fn production_protocol_bootstrap_failure_cleans_up_before_socket_exposure() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("production-protocol-bootstrap-cleanup");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(&socket_name);
        let mut display = SmithayWaylandDisplayProbe::new().expect("测试 Display 必须能构造");
        display
            .initialize_xdg_shell_global()
            .expect("预置 xdg_wm_base global 必须成功");

        let result = with_production_protocol_display(&socket_name, display);

        assert!(result.is_err());
        assert!(
            !socket_path.exists(),
            "bootstrap 失败发生在 socket bind 之前，不得暴露 socket path"
        );

        let flow = NestedRealAcceptFlow::with_production_protocol_bootstrap(&socket_name)
            .expect("失败 owner/source 清理后，同名 fresh production flow 必须可启动");
        assert!(socket_path.exists());
        drop(flow);
        assert!(
            !socket_path.exists(),
            "成功 flow drop 后 socket owner 必须移除 path"
        );
    }

    /// 验证 B 路线报告保留全部真实 runtime blockers 和 capability false。
    #[test]
    fn real_accept_boundary_keeps_accepts_clients_false_until_runtime_proof() {
        let report = nested_real_accept_connected_bridge_readiness_report();

        assert_eq!(
            report.blockers,
            vec![
                NestedRealAcceptConnectedBridgeBlocker::MissingRealAcceptLoop,
                NestedRealAcceptConnectedBridgeBlocker::MissingAcceptedStreamOwnership,
                NestedRealAcceptConnectedBridgeBlocker::MissingDisplayHandleInsertClientRuntimeProof,
                NestedRealAcceptConnectedBridgeBlocker::MissingRealClientSessionMapping,
                NestedRealAcceptConnectedBridgeBlocker::MissingConnectedEventBridgeFromRealInsert,
                NestedRealAcceptConnectedBridgeBlocker::MissingDisconnectCallbackSource,
                NestedRealAcceptConnectedBridgeBlocker::MissingLinuxRuntimeProof,
            ]
        );
        assert!(report.listening_socket_callback_available);
        assert!(report.calloop_version_boundary_clear);
        assert!(report.client_data_owner_defined);
        assert!(!report.accepted_stream_available);
        assert!(!report.real_accept_loop_available);
        assert!(!report.display_handle_insert_client_runtime_available);
        assert!(!report.inserted_client_mapping_available);
        assert!(!report.connected_event_bridged_to_core);
        assert!(!report.validation_report_available);
        assert!(!report.accepts_clients);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.is_runtime_ready());
    }

    /// 验证 backend client/session mapping 支持 insert、lookup 与 remove。
    #[test]
    fn inserted_client_mapping_round_trips_session_identity() {
        // Arrange：真实插入 UnixStream 取得锁定版本 backend ClientId。
        let display = Display::<()>::new().expect("Wayland Display 必须能构造");
        let mut insert = NestedClientInsertCompileBoundary::new(display.handle());
        let (server_stream, _client_stream) =
            UnixStream::pair().expect("UnixStream pair 必须能构造");
        let session = session(71);
        let client = insert
            .insert_client(server_stream, session)
            .expect("测试 client 必须成功插入 Display");
        let client_id = client.id();
        let mut mapping = NestedAcceptedClientMapping::new();

        // Act 与 Assert
        assert_eq!(mapping.insert(client_id.clone(), session), None);
        assert_eq!(mapping.lookup(&client_id), Some(session));
        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping.remove(&client_id), Some(session));
        assert!(mapping.is_empty());
    }

    /// 验证 disconnect session identity 可以移除对应 backend client mapping。
    #[test]
    fn disconnected_session_removes_inserted_client_mapping() {
        // Arrange：真实插入 stream 取得不可伪造的 backend ClientId。
        let display = Display::<()>::new().expect("Wayland Display 必须能构造");
        let mut insert = NestedClientInsertCompileBoundary::new(display.handle());
        let (server_stream, _client_stream) =
            UnixStream::pair().expect("UnixStream pair 必须能构造");
        let session = session(73);
        let client = insert
            .insert_client(server_stream, session)
            .expect("测试 client 必须成功插入 Display");
        let client_id = client.id();
        let mut mapping = NestedAcceptedClientMapping::new();
        assert_eq!(mapping.insert(client_id.clone(), session), None);

        // Act 与 Assert：callback 只知道 session，也能精确清理对应 backend mapping。
        assert_eq!(mapping.remove_session(session), Some(client_id));
        assert!(mapping.is_empty());
    }

    /// 验证 duplicate connected 仍复用 session core bridge，不重复注册核心 client。
    #[test]
    fn accepted_client_duplicate_connect_does_not_register_twice() {
        // Arrange
        let session = session(72);
        let events = vec![
            NestedClientSessionEvent::Connected { session },
            NestedClientSessionEvent::Connected { session },
        ];
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = crate::core::runtime_bridge::NestedClientSessionCoreBridge::new();
        let mut state = State::new();

        // Act
        let (_, outcomes) = bridge_connected_events(
            events,
            "controlled-duplicate",
            &mut log,
            &mut bridge,
            &mut state,
        );

        // Assert
        assert!(matches!(
            outcomes[0],
            NestedClientSessionBridgeOutcome::Connected { session: actual, .. } if actual == session
        ));
        assert!(matches!(
            outcomes[1],
            NestedClientSessionBridgeOutcome::DuplicateConnected { session: actual, .. }
                if actual == session
        ));
        assert_eq!(bridge.active_session_count(), 1);
    }

    /// 验证 accepted stream 无法分配 session 时不会生成 connected event 或注册 core。
    #[test]
    fn accepted_stream_session_failure_does_not_register_core_client() {
        // Arrange：使用真实 UnixStream，但把 allocator 置于明确耗尽状态。
        let display = Display::<()>::new().expect("Wayland Display 必须能构造");
        let insert = NestedClientInsertCompileBoundary::new(display.handle());
        let mut loop_data = NestedRealAcceptLoopData::new(insert);
        loop_data.next_session_value = None;
        let (server_stream, _client_stream) =
            UnixStream::pair().expect("UnixStream pair 必须能构造");
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = crate::core::runtime_bridge::NestedClientSessionCoreBridge::new();
        let mut state = State::new();

        // Act：callback data 观察 stream，但不会调用 insert_client 或发布 Connected。
        loop_data.accept_stream(server_stream);
        let attempts = loop_data.take_attempts();
        let connected = loop_data.insert_boundary.event_queue().drain_connected();
        let (_, outcomes) = bridge_connected_events(
            connected,
            "controlled-session-exhaustion",
            &mut log,
            &mut bridge,
            &mut state,
        );

        // Assert
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].failure_reason,
            Some(NestedAcceptedClientFailureReason::SessionIdExhausted)
        );
        assert!(outcomes.is_empty());
        assert_eq!(bridge.active_session_count(), 0);
        assert!(state.clients.records().is_empty());
    }

    /// 验证 insertion 未发布 Connected 时，flow 不会凭空注册核心 client。
    #[test]
    fn insert_failure_without_connected_event_does_not_register_core_client() {
        // Arrange：client_insert 模块已单独证明 Err 不会发布 Connected；这里封板消费侧。
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = crate::core::runtime_bridge::NestedClientSessionCoreBridge::new();
        let mut state = State::new();

        // Act：没有 Connected 输入时，flow 不得猜测 session 或直接写 ClientRegistry。
        let (records, outcomes) = bridge_connected_events(
            Vec::new(),
            "controlled-insert-failure",
            &mut log,
            &mut bridge,
            &mut state,
        );

        // Assert
        assert!(records.is_empty());
        assert!(outcomes.is_empty());
        assert_eq!(bridge.active_session_count(), 0);
        assert!(state.clients.records().is_empty());
    }

    /// Linux-only 真实证明：socket callback 接收 stream，insert 后通过 bridge 注册 core。
    #[test]
    fn real_accepted_client_connected_event_registers_core_client() {
        // Arrange
        assert_runtime_dir();
        let socket_name = unique_socket_name("real-accept-connected");
        let mut flow = NestedRealAcceptFlow::with_socket_name(&socket_name)
            .expect("真实 accept flow 必须绑定 socket 并注册 callback source");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(flow.socket_name());
        let _client_stream =
            UnixStream::connect(socket_path).expect("测试 client 必须连接真实 Wayland socket");
        let mut state = State::new();

        // Act
        let report = flow
            .pump_once(&mut state, Duration::from_secs(1))
            .expect("event loop 必须处理真实 listening socket readiness");

        // Assert：报告来自真实 callback，但 capability 仍等待本分支 Linux CI 验收。
        assert_eq!(report.accepted_stream_count(), 1);
        assert_eq!(report.inserted_client_count(), 1);
        assert!(report.attempts[0].mapping_saved);
        assert_eq!(report.connected_records.len(), 1);
        assert_eq!(report.bridge_outcomes.len(), 1);
        assert_eq!(report.all_observed_validations_clean, Some(true));
        assert_eq!(flow.mapping().len(), 1);
        assert_eq!(flow.active_core_session_count(), 1);
        assert!(flow.display_is_probe_only());

        let core_clients = report.registered_core_clients();
        assert_eq!(core_clients.len(), 1);
        assert!(state.clients.is_alive(core_clients[0]));
        assert!(!report.readiness.accepts_clients);
        assert!(!report.readiness.real_accept_loop_available);
    }

    /// Linux-only 真实证明：peer close 经 Display dispatch 触发 callback 并关闭 core client。
    #[test]
    fn runtime_disconnect_callback_closes_core_client() {
        // Arrange：真实 socket callback 必须先完成 accept、insert、session mapping 与 core register。
        assert_runtime_dir();
        let socket_name = unique_socket_name("runtime-disconnect-callback");
        let mut flow = NestedRealAcceptFlow::with_socket_name(&socket_name)
            .expect("真实 accept flow 必须绑定 socket 并注册 callback source");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(flow.socket_name());
        let client_stream =
            UnixStream::connect(socket_path).expect("测试 peer 必须连接真实 Wayland socket");
        let mut state = State::new();
        let connected = flow
            .pump_once(&mut state, Duration::from_secs(1))
            .expect("event loop 必须处理真实 listening socket readiness");
        let session = connected.connected_records[0]
            .session
            .expect("真实 insertion record 必须保留 session identity");
        let core_client = connected.registered_core_clients()[0];
        assert!(state.clients.is_alive(core_client));
        assert_eq!(flow.mapping().len(), 1);
        assert_eq!(flow.active_core_session_count(), 1);

        // Act：只关闭真实 peer；随后 Display dispatch 从 EOF 触发 ClientData callback。
        // callback 只能把 session event 写入队列，core close 仍由 coordinator bridge 执行。
        drop(client_stream);
        flow.dispatch_wayland_clients_once()
            .expect("Display 必须处理 peer EOF 并触发 disconnect callback");
        assert_eq!(
            flow.loop_data.insert_boundary.event_queue().len(),
            1,
            "真实 callback 必须先产生一个待 bridge 的 Disconnected event"
        );
        let disconnected = flow.bridge_pending_disconnects(&mut state);

        // Assert：record/session、既有 event-command seam、mapping cleanup 与 validation 同时成立。
        assert_eq!(disconnected.disconnected_count(), 1);
        assert_eq!(disconnected.disconnected_records[0].session, Some(session));
        assert_eq!(
            disconnected.disconnected_records[0].kind,
            NestedClientSessionEventKind::Disconnected
        );
        assert_eq!(disconnected.closed_core_clients(), vec![core_client]);
        let NestedClientSessionBridgeOutcome::Disconnected { runtime, .. } =
            &disconnected.bridge_outcomes[0]
        else {
            panic!("真实 callback record 必须生成 Disconnected outcome");
        };
        assert_eq!(
            runtime.event,
            BackendEvent::ClientDisconnected {
                client: core_client
            }
        );
        assert_eq!(runtime.command, CoreCommand::CloseClient(core_client));
        assert!(runtime.validation.is_clean());
        assert_eq!(disconnected.all_observed_validations_clean, Some(true));
        assert_eq!(disconnected.removed_backend_mapping_count, 1);
        assert!(disconnected.readiness.real_disconnect_callback_observed);
        assert!(disconnected.readiness.core_close_invoked_from_real_callback);
        assert!(!disconnected.readiness.accepts_clients);
        assert!(flow.mapping().is_empty());
        assert_eq!(flow.active_core_session_count(), 0);
        assert!(!state.clients.is_alive(core_client));
        assert!(state.clients.get(core_client).is_some());
        assert!(state.validate().is_clean());

        // callback event 已被消费；重复 pump 不会制造第二次 close。
        let duplicate = flow.bridge_pending_disconnects(&mut state);
        assert_eq!(duplicate.disconnected_count(), 0);
        assert!(duplicate.closed_core_clients().is_empty());
        assert!(state.validate().is_clean());
    }
}
