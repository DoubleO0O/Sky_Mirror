//! Phase 51K Linux-only nested lifecycle single-pump coordinator。
//!
//! coordinator 只按固定顺序编排现有 [`NestedRealAcceptFlow`]：accept/insert 与
//! connected bridge、一次 Display dispatch、disconnected bridge。它不直接修改 core
//! registry，也不把单次 pump 冒充长期 compositor event loop。

use std::{collections::BTreeSet, io, time::Duration};

use crate::{
    core::{
        client::ClientId as CoreClientId, state::State, surface::SurfaceId, workspace::WindowId,
    },
    smithay_backend::{
        linux_live_toplevel_admission_owner::{
            LiveToplevelAdmissionOwnerReport, enqueue_live_toplevel_admission_from_observation,
        },
        linux_toplevel_admission_bridge::PendingXdgToplevelAdmission,
        linux_toplevel_admission_runtime_queue::{
            RuntimeToplevelAdmissionDrainReport, RuntimeToplevelAdmissionDrainTick,
            RuntimeToplevelAdmissionEnqueueReport, RuntimeToplevelAdmissionQueueOwner,
        },
        real_accept_flow::NestedRealAcceptFlow,
        surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId},
    },
};

/// Phase 51K coordinator 尚未满足的独立能力条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeCoordinatorBlocker {
    /// 尚无 Linux test/CI 证明一次 coordinator pump 串通完整 client lifecycle。
    MissingLinuxLifecyclePumpProof,

    /// 尚无可持续运行、具备停止语义的长期 runtime loop。
    MissingLongRunningLoop,
}

/// Phase 51K coordinator 的保守 capability 报告。
///
/// `coordinator_boundary_defined` 说明接口已存在；五个 single-pump 字段由 Linux CI
/// lifecycle proof 支持。长期 loop、surface 和 render 不属于本阶段。
#[must_use = "必须区分 single-pump boundary、Linux proof 与长期 compositor loop"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeCoordinatorReadinessReport {
    /// 当前仍存在的 coordinator/runtime blockers。
    pub blockers: Vec<NestedRuntimeCoordinatorBlocker>,

    /// 是否已定义 Linux-only coordinator interface。
    pub coordinator_boundary_defined: bool,

    /// 是否已有 Linux proof 支持的 nested runtime coordinator。
    pub nested_runtime_coordinator_available: bool,

    /// 是否已有 Linux proof 支持的 single pump。
    pub single_pump_available: bool,

    /// connected bridge 是否已由 coordinator runtime proof 调用。
    pub connected_bridge_invoked: bool,

    /// disconnect bridge 是否已由 coordinator runtime proof 调用。
    pub disconnect_bridge_invoked: bool,

    /// Display dispatch 是否已由 coordinator runtime proof 调用。
    pub display_dispatch_invoked: bool,

    /// 是否已具备项目级 client accept 能力；本阶段固定为 `false`。
    pub accepts_clients: bool,

    /// 是否已启动长期 accept loop；本阶段固定为 `false`。
    pub runtime_accept_loop_started: bool,

    /// 是否已启动长期 protocol dispatch；本阶段固定为 `false`。
    pub protocol_dispatch_started: bool,

    /// 是否已有长期 runtime loop；本阶段固定为 `false`。
    pub long_running_loop_available: bool,

    /// 是否支持真实 surface；本阶段固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；本阶段固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实 render；本阶段固定为 `false`。
    pub render_support: bool,

    /// 是否支持真实 input；本阶段固定为 `false`。
    pub input_support: bool,
}

impl NestedRuntimeCoordinatorReadinessReport {
    /// 判断 Linux single-pump lifecycle proof 是否已完整成立。
    pub fn is_single_pump_ready(&self) -> bool {
        self.nested_runtime_coordinator_available
            && self.single_pump_available
            && self.connected_bridge_invoked
            && self.disconnect_bridge_invoked
            && self.display_dispatch_invoked
    }
}

/// 返回 Phase 51K C 路线经 Linux lifecycle proof 支持的 coordinator readiness。
#[must_use = "single-pump proof 不能代替长期 runtime loop"]
pub fn nested_runtime_coordinator_readiness_report() -> NestedRuntimeCoordinatorReadinessReport {
    NestedRuntimeCoordinatorReadinessReport {
        blockers: vec![NestedRuntimeCoordinatorBlocker::MissingLongRunningLoop],
        coordinator_boundary_defined: true,
        nested_runtime_coordinator_available: true,
        single_pump_available: true,
        connected_bridge_invoked: true,
        disconnect_bridge_invoked: true,
        display_dispatch_invoked: true,
        accepts_clients: false,
        runtime_accept_loop_started: false,
        protocol_dispatch_started: false,
        long_running_loop_available: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        input_support: false,
    }
}

/// single pump 中可结构化返回的错误阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimePumpErrorKind {
    /// listening socket/calloop accept source pump 失败。
    AcceptSourcePump,

    /// Display client dispatch 失败。
    DisplayDispatch,
}

/// single pump 的结构化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimePumpError {
    /// 错误发生的 coordinator 阶段。
    pub kind: NestedRuntimePumpErrorKind,

    /// 底层错误文本，仅用于诊断。
    pub message: String,
}

/// 一次 nested lifecycle pump 的纯数据汇总报告。
///
/// report 不持有 Display、socket、client 或 `State` 引用；核心 client IDs 只来自既有
/// session bridge outcome，coordinator 不猜测或分配 core identity。
#[must_use = "single-pump report 包含错误、bridge 结果和 validation，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimePumpReport {
    /// accept source 本轮观察到的 client 数量。
    pub accepted_clients: usize,

    /// 成功 insert 到 Display 的 client 数量。
    pub inserted_clients: usize,

    /// 本轮 drain 的 connected event 数量。
    pub connected_events_drained: usize,

    /// connected bridge 注册得到的 core client IDs。
    pub registered_core_clients: Vec<CoreClientId>,

    /// 本轮是否尝试调用 Display dispatch。
    pub dispatch_clients_called: bool,

    /// dispatch 成功时返回的 request 数量；失败时为 `None`。
    pub dispatched_requests: Option<usize>,

    /// 本轮 drain 的 disconnected event 数量。
    pub disconnected_events_drained: usize,

    /// disconnected bridge 关闭的 core client IDs。
    pub closed_core_clients: Vec<CoreClientId>,

    /// pump 结束时核心状态是否通过 ValidationReport。
    pub validation_is_clean: bool,

    /// accept/dispatch 阶段的结构化错误，按发生顺序保存。
    pub errors: Vec<NestedRuntimePumpError>,

    /// 当前 coordinator capability 快照。
    pub readiness: NestedRuntimeCoordinatorReadinessReport,
}

impl NestedRuntimePumpReport {
    /// 本轮是否没有错误且最终 validation clean。
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty() && self.validation_is_clean
    }
}

/// 一次 lifecycle pump 后追加 runtime admission drain 的组合报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeAdmissionPumpReport {
    /// 既有 accept/connected/dispatch/disconnected lifecycle pump report。
    pub lifecycle_report: NestedRuntimePumpReport,

    /// lifecycle 完成后由 runtime admission queue owner drain 的 report。
    pub admission_drain_report: RuntimeToplevelAdmissionDrainReport,
}

/// 一次 lifecycle pump 后追加 live admission owner 入队与 runtime admission drain 的组合报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLiveAdmissionPumpReport {
    /// 既有 accept/connected/dispatch/disconnected lifecycle pump report。
    pub lifecycle_report: NestedRuntimePumpReport,

    /// Phase 53A live callback admission owner 的 enqueue report。
    pub live_admission_owner_report: LiveToplevelAdmissionOwnerReport,

    /// live owner 入队后由 runtime admission queue owner drain 的 report。
    pub admission_drain_report: RuntimeToplevelAdmissionDrainReport,
}

/// Linux-only nested client lifecycle single-pump coordinator。
///
/// coordinator 只拥有并编排 [`NestedRealAcceptFlow`]。connected/disconnected mutation
/// 继续由 flow 内的 session bridge 走 `BackendEvent -> CoreCommand -> State`；本类型
/// 不直接写任何 core registry。调用方可以周期调用 [`Self::pump_once`]，但该接口本身
/// 没有 run/stop/wakeup 语义，因此不等于长期 runtime loop。
pub struct NestedRuntimeCoordinator {
    flow: NestedRealAcceptFlow,
    admission_queue_owner: RuntimeToplevelAdmissionQueueOwner,
    seen_live_toplevel_callback_sequences: BTreeSet<u64>,
}

impl NestedRuntimeCoordinator {
    /// 使用指定 Wayland socket 名称创建 coordinator。
    ///
    /// # Errors
    ///
    /// Display、socket、calloop source 或 accept flow 初始化失败时返回原始错误链。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_socket_name_and_admission_surface_start(name, 1)
    }

    /// 使用指定 Wayland socket 名称与 admission core surface 起始 ID 创建 coordinator。
    ///
    /// # Errors
    ///
    /// Display、socket、calloop source 或 accept flow 初始化失败时返回原始错误链。
    pub fn with_socket_name_and_admission_surface_start(
        name: &str,
        next_core_surface_id: SurfaceId,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            flow: NestedRealAcceptFlow::with_socket_name(name)?,
            admission_queue_owner: RuntimeToplevelAdmissionQueueOwner::new(next_core_surface_id),
            seen_live_toplevel_callback_sequences: BTreeSet::new(),
        })
    }

    /// 返回 coordinator 已绑定的 Wayland socket 名称。
    pub fn socket_name(&self) -> &str {
        self.flow.socket_name()
    }

    /// 返回只用于唤醒 accept-source poll 的 cloneable calloop signal。
    pub(crate) fn loop_signal(&self) -> smithay::reexports::calloop::LoopSignal {
        self.flow.loop_signal()
    }

    /// 将 pending xdg_toplevel admission intent 入队到 coordinator 持有的 runtime owner。
    pub fn enqueue_pending_toplevel_admission(
        &mut self,
        pending: PendingXdgToplevelAdmission,
    ) -> RuntimeToplevelAdmissionEnqueueReport {
        self.admission_queue_owner
            .enqueue_pending_toplevel_admission(pending)
    }

    /// 返回 coordinator admission owner 中的 pending 数量。
    pub fn admission_pending_count(&self) -> usize {
        self.admission_queue_owner.pending_count()
    }

    /// 返回 coordinator admission owner 下一次将使用的 core surface identity。
    pub fn admission_next_core_surface_id(&self) -> SurfaceId {
        self.admission_queue_owner.next_core_surface_id()
    }

    /// 查询 adapter surface 到 core surface 的 admission ledger mapping。
    pub fn admission_surface_mapping(
        &self,
        adapter_surface: AdapterSurfaceId,
    ) -> Option<SurfaceId> {
        self.admission_queue_owner.surface_mapping(adapter_surface)
    }

    /// 查询 adapter toplevel 到 core window 的 admission ledger mapping。
    pub fn admission_toplevel_mapping(
        &self,
        adapter_toplevel: AdapterToplevelId,
    ) -> Option<WindowId> {
        self.admission_queue_owner
            .toplevel_mapping(adapter_toplevel)
    }

    /// 返回 live callback sequence 是否已被 coordinator admission owner 处理过。
    pub fn has_seen_live_toplevel_callback_sequence(&self, sequence: u64) -> bool {
        self.seen_live_toplevel_callback_sequences
            .contains(&sequence)
    }

    /// 标记 live callback sequence 已经被 admission enqueue seam 接收。
    pub fn mark_live_toplevel_callback_sequence_seen(&mut self, sequence: u64) -> bool {
        self.seen_live_toplevel_callback_sequences.insert(sequence)
    }

    /// 执行一次 accept/connected → Display dispatch → disconnected lifecycle pump。
    ///
    /// accept 与 dispatch 错误会进入 report；coordinator 不 panic，也不会绕过既有
    /// bridge。即使 dispatch 失败，已由 callback 产生的 disconnect event 仍会被安全
    /// drain，避免把 active session mapping 留在半完成状态。
    pub fn pump_once(&mut self, state: &mut State, timeout: Duration) -> NestedRuntimePumpReport {
        self.pump_once_with_dispatch(state, timeout, |flow| flow.dispatch_wayland_clients_once())
    }

    /// 执行一次既有 lifecycle pump，然后从 runtime admission queue drain 一条 intent。
    ///
    /// 该方法不改变 [`Self::pump_once`] 的语义；admission drain 被明确追加在
    /// accept/dispatch/disconnect lifecycle 之后。
    pub fn pump_once_with_toplevel_admission_drain(
        &mut self,
        state: &mut State,
        timeout: Duration,
        tick: RuntimeToplevelAdmissionDrainTick,
    ) -> NestedRuntimeAdmissionPumpReport {
        let lifecycle_report = self.pump_once(state, timeout);
        let admission_drain_report = self
            .admission_queue_owner
            .drain_pending_toplevel_admission_once(state, tick);

        NestedRuntimeAdmissionPumpReport {
            lifecycle_report,
            admission_drain_report,
        }
    }

    /// 执行一次 lifecycle pump，然后读取 live callback observation 入队并 drain admission。
    ///
    /// coordinator 先从 flow 持有的 display 读取纯数据 observation 快照，再调用
    /// admission owner 入队，最后由 runtime queue owner 消费 intent。该顺序避免
    /// handler/display 直接持有 `State` 或 admission ledger，也不改变普通
    /// [`Self::pump_once`] 的语义。
    pub fn pump_once_with_live_toplevel_admission_drain(
        &mut self,
        state: &mut State,
        timeout: Duration,
        tick: RuntimeToplevelAdmissionDrainTick,
    ) -> NestedRuntimeLiveAdmissionPumpReport {
        let lifecycle_report = self.pump_once(state, timeout);
        let observation = self.flow.take_next_live_toplevel_admission_observation();
        let live_admission_owner_report =
            enqueue_live_toplevel_admission_from_observation(observation, self);
        let admission_drain_report = self
            .admission_queue_owner
            .drain_pending_toplevel_admission_once(state, tick);

        NestedRuntimeLiveAdmissionPumpReport {
            lifecycle_report,
            live_admission_owner_report,
            admission_drain_report,
        }
    }

    // 内部 seam 只允许测试注入 dispatch error；production 始终调用真实 Display dispatch。
    fn pump_once_with_dispatch<F>(
        &mut self,
        state: &mut State,
        timeout: Duration,
        dispatch: F,
    ) -> NestedRuntimePumpReport
    where
        F: FnOnce(&mut NestedRealAcceptFlow) -> io::Result<usize>,
    {
        let mut accepted_clients = 0;
        let mut inserted_clients = 0;
        let mut connected_events_drained = 0;
        let mut registered_core_clients = Vec::new();
        let mut errors = Vec::new();

        match self.flow.pump_once(state, timeout) {
            Ok(report) => {
                accepted_clients = report.accepted_stream_count();
                inserted_clients = report.inserted_client_count();
                connected_events_drained = report.connected_records.len();
                registered_core_clients = report.registered_core_clients();
            }
            Err(error) => errors.push(NestedRuntimePumpError {
                kind: NestedRuntimePumpErrorKind::AcceptSourcePump,
                message: error.to_string(),
            }),
        }

        let dispatched_requests = match dispatch(&mut self.flow) {
            Ok(count) => Some(count),
            Err(error) => {
                errors.push(NestedRuntimePumpError {
                    kind: NestedRuntimePumpErrorKind::DisplayDispatch,
                    message: error.to_string(),
                });
                None
            }
        };

        // callback 只写 session event；coordinator 必须回到既有 disconnect bridge。
        let disconnected = self.flow.bridge_pending_disconnects(state);
        let disconnected_events_drained = disconnected.disconnected_count();
        let closed_core_clients = disconnected.closed_core_clients();

        NestedRuntimePumpReport {
            accepted_clients,
            inserted_clients,
            connected_events_drained,
            registered_core_clients,
            dispatch_clients_called: true,
            dispatched_requests,
            disconnected_events_drained,
            closed_core_clients,
            validation_is_clean: state.validate().is_clean(),
            errors,
            readiness: nested_runtime_coordinator_readiness_report(),
        }
    }
}

#[cfg(test)]
impl NestedRuntimeCoordinator {
    /// 测试专用：让 loop/orchestrator proof 在 coordinator 持有的 display 上制造 observation。
    ///
    /// production 仍只能通过 coordinator pump 读取 flow 暴露的纯数据 observation。
    pub(crate) fn display_mut_for_controlled_toplevel_registration(
        &mut self,
    ) -> &mut crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe {
        self.flow.display_mut_for_controlled_toplevel_registration()
    }
}

#[cfg(test)]
mod tests {
    use std::{io, os::unix::net::UnixStream, path::Path, time::Duration};

    use super::{
        NestedRuntimeCoordinator, NestedRuntimeCoordinatorBlocker, NestedRuntimePumpErrorKind,
        nested_runtime_coordinator_readiness_report,
    };
    use crate::{
        core::state::State,
        smithay_backend::{
            linux_live_toplevel_admission_owner::LiveToplevelAdmissionOwnerOperation,
            linux_toplevel_admission_bridge::PendingXdgToplevelAdmission,
            linux_toplevel_admission_runtime_queue::{
                RuntimeToplevelAdmissionDrainTick, RuntimeToplevelAdmissionQueueBlocker,
            },
            linux_toplevel_identity_registration::adapter_toplevel_identity_registration_report,
            surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId},
            test_support::{assert_runtime_dir, unique_socket_name},
        },
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 surface identity 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(
            ProtocolObjectId::new(value).expect("测试 toplevel identity 必须非零"),
        )
    }

    /// 验证 C 路线只上调 Linux proof 支持的 single-pump 字段，不冒充长期 loop。
    #[test]
    fn nested_runtime_coordinator_proof_capabilities_are_precise() {
        let report = nested_runtime_coordinator_readiness_report();

        assert_eq!(
            report.blockers,
            vec![NestedRuntimeCoordinatorBlocker::MissingLongRunningLoop]
        );
        assert!(report.coordinator_boundary_defined);
        assert!(report.nested_runtime_coordinator_available);
        assert!(report.single_pump_available);
        assert!(report.connected_bridge_invoked);
        assert!(report.disconnect_bridge_invoked);
        assert!(report.display_dispatch_invoked);
        assert!(!report.accepts_clients);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.long_running_loop_available);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.is_single_pump_ready());
    }

    /// 验证没有 client 的 single pump 安全返回，不 panic、不制造 core mutation。
    #[test]
    fn nested_runtime_pump_noop_is_safe() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-noop");
        let mut coordinator = NestedRuntimeCoordinator::with_socket_name(&socket_name)
            .expect("coordinator 必须绑定测试 socket");
        let mut state = State::new();

        let report = coordinator.pump_once(&mut state, Duration::ZERO);

        assert_eq!(report.accepted_clients, 0);
        assert_eq!(report.inserted_clients, 0);
        assert_eq!(report.connected_events_drained, 0);
        assert!(report.registered_core_clients.is_empty());
        assert!(report.dispatch_clients_called);
        assert_eq!(report.disconnected_events_drained, 0);
        assert!(report.closed_core_clients.is_empty());
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(state.clients.records().is_empty());
    }

    /// Linux-only 真实 proof：一次 coordinator pump 串通 connected 与 disconnected lifecycle。
    #[test]
    fn nested_runtime_coordinator_single_pump_runs_full_lifecycle() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-lifecycle");
        let mut coordinator = NestedRuntimeCoordinator::with_socket_name(&socket_name)
            .expect("coordinator 必须绑定测试 socket");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(coordinator.socket_name());
        let client_stream =
            UnixStream::connect(socket_path).expect("测试 peer 必须连接真实 Wayland socket");
        let mut state = State::new();

        // peer 在 pump 前关闭；server 仍会 accept 已建立连接，随后 dispatch 观察 EOF。
        drop(client_stream);
        let report = coordinator.pump_once(&mut state, Duration::from_secs(1));

        assert_eq!(report.accepted_clients, 1);
        assert_eq!(report.inserted_clients, 1);
        assert_eq!(report.connected_events_drained, 1);
        assert_eq!(report.registered_core_clients.len(), 1);
        assert!(report.dispatch_clients_called);
        assert_eq!(report.disconnected_events_drained, 1);
        assert_eq!(report.closed_core_clients, report.registered_core_clients);
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(report.readiness.is_single_pump_ready());
        assert!(!report.readiness.accepts_clients);
        assert!(!report.readiness.runtime_accept_loop_started);
        assert!(!report.readiness.protocol_dispatch_started);
        assert!(!report.readiness.long_running_loop_available);
        let client = report.registered_core_clients[0];
        assert!(!state.clients.is_alive(client));
        assert!(state.clients.get(client).is_some());

        // 第二次周期调用必须 no-op，不重复注册或关闭。
        let record_count = state.clients.records().len();
        let duplicate = coordinator.pump_once(&mut state, Duration::ZERO);
        assert_eq!(duplicate.connected_events_drained, 0);
        assert_eq!(duplicate.disconnected_events_drained, 0);
        assert!(duplicate.registered_core_clients.is_empty());
        assert!(duplicate.closed_core_clients.is_empty());
        assert_eq!(state.clients.records().len(), record_count);
        assert!(duplicate.validation_is_clean);
    }

    /// 验证 Display dispatch error 返回结构化报告，同时保持 core validation clean。
    #[test]
    fn nested_runtime_pump_reports_dispatch_failure() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-dispatch-error");
        let mut coordinator = NestedRuntimeCoordinator::with_socket_name(&socket_name)
            .expect("coordinator 必须绑定测试 socket");
        let mut state = State::new();

        let report = coordinator.pump_once_with_dispatch(&mut state, Duration::ZERO, |_| {
            Err(io::Error::other("controlled dispatch failure"))
        });

        assert!(report.dispatch_clients_called);
        assert_eq!(report.dispatched_requests, None);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(
            report.errors[0].kind,
            NestedRuntimePumpErrorKind::DisplayDispatch
        );
        assert!(
            report.errors[0]
                .message
                .contains("controlled dispatch failure")
        );
        assert!(!report.is_successful());
        assert!(report.validation_is_clean);
        assert!(state.clients.records().is_empty());
    }

    /// 验证 admission drain 被追加到既有 lifecycle 后，空 queue 不制造 core mutation。
    #[test]
    fn nested_runtime_admission_drain_empty_queue_preserves_lifecycle_noop() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-admission-empty");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                8_000,
            )
            .expect("coordinator 必须绑定测试 socket");
        let mut state = State::new();

        let report = coordinator.pump_once_with_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(52),
        );

        assert_eq!(report.lifecycle_report.accepted_clients, 0);
        assert_eq!(report.lifecycle_report.inserted_clients, 0);
        assert!(report.lifecycle_report.dispatch_clients_called);
        assert!(report.lifecycle_report.is_successful());
        assert_eq!(
            report.admission_drain_report.pending_admission_count_before,
            0
        );
        assert_eq!(
            report.admission_drain_report.pending_admission_count_after,
            0
        );
        assert!(!report.admission_drain_report.ledger_consume_attempted);
        assert_eq!(
            report.admission_drain_report.blockers.as_slice(),
            &[RuntimeToplevelAdmissionQueueBlocker::MissingPendingAdmission]
        );
        assert_eq!(coordinator.admission_pending_count(), 0);
        assert_eq!(coordinator.admission_next_core_surface_id(), 8_000);
        assert!(state.validate().is_clean());
    }

    /// 验证 coordinator 可以在 lifecycle 后经 runtime owner drain pending admission。
    #[test]
    fn nested_runtime_admission_drain_consumes_pending_after_lifecycle() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-admission-pending");
        let adapter_surface = surface(1_701);
        let adapter_toplevel = toplevel(1_801);
        let pending = PendingXdgToplevelAdmission::new(adapter_surface, adapter_toplevel, Some(52));
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                9_000,
            )
            .expect("coordinator 必须绑定测试 socket");
        let mut state = State::new();

        let enqueue = coordinator.enqueue_pending_toplevel_admission(pending);
        assert!(enqueue.pending_admission_enqueued);
        assert_eq!(enqueue.pending_admission_count_before, 0);
        assert_eq!(enqueue.pending_admission_count_after, 1);
        assert_eq!(coordinator.admission_pending_count(), 1);

        let report = coordinator.pump_once_with_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(53),
        );

        assert!(report.lifecycle_report.is_successful());
        assert_eq!(report.lifecycle_report.accepted_clients, 0);
        assert_eq!(report.lifecycle_report.disconnected_events_drained, 0);
        assert_eq!(
            report.admission_drain_report.pending_admission_count_before,
            1
        );
        assert_eq!(
            report.admission_drain_report.pending_admission_count_after,
            0
        );
        assert!(report.admission_drain_report.pending_admission_consumed);
        assert!(report.admission_drain_report.ledger_admit_surface_invoked);
        assert!(report.admission_drain_report.ledger_admit_invoked);
        assert!(report.admission_drain_report.core_register_invoked);
        assert!(report.admission_drain_report.window_id_allocated);
        assert_eq!(report.admission_drain_report.core_surface_id, Some(9_000));
        assert_eq!(
            report.admission_drain_report.next_core_surface_id_after,
            9_001
        );
        let core_window = report
            .admission_drain_report
            .core_window_id
            .expect("coordinator admission drain 必须返回 core WindowId");
        assert_eq!(coordinator.admission_pending_count(), 0);
        assert_eq!(coordinator.admission_next_core_surface_id(), 9_001);
        assert_eq!(
            coordinator.admission_surface_mapping(adapter_surface),
            Some(9_000)
        );
        assert_eq!(
            coordinator.admission_toplevel_mapping(adapter_toplevel),
            Some(core_window)
        );
        assert!(state.validate().is_clean());
        assert!(report.admission_drain_report.blockers.is_empty());
    }

    /// 验证 coordinator 可在同一轮 pump 中读取 live callback observation、入队并 drain admission。
    #[test]
    fn nested_runtime_live_admission_pump_enqueues_and_drains_observed_callback() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-live-admission");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                12_000,
            )
            .expect("coordinator 必须绑定测试 socket");
        let registration = {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_xdg_shell_global()
                .expect("测试 xdg-shell global 必须初始化");
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            adapter_toplevel_identity_registration_report(display)
                .expect("adapter identity registration proof 必须完成")
        };
        let mut state = State::new();

        let report = coordinator.pump_once_with_live_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(53),
        );

        assert!(report.lifecycle_report.is_successful());
        assert_eq!(
            report
                .live_admission_owner_report
                .new_toplevel_callback_sequence,
            Some(registration.new_toplevel_callback_sequence)
        );
        assert!(
            report
                .live_admission_owner_report
                .pending_admission_intent_created
        );
        assert!(
            report
                .live_admission_owner_report
                .coordinator_enqueue_invoked
        );
        assert!(
            report
                .live_admission_owner_report
                .operations
                .contains(&LiveToplevelAdmissionOwnerOperation::EnqueueCoordinatorAdmission)
        );
        assert!(report.admission_drain_report.pending_admission_consumed);
        assert!(report.admission_drain_report.ledger_admit_surface_invoked);
        assert!(report.admission_drain_report.ledger_admit_invoked);
        assert!(report.admission_drain_report.core_register_invoked);
        assert!(report.admission_drain_report.window_id_allocated);
        assert_eq!(report.admission_drain_report.core_surface_id, Some(12_000));
        assert_eq!(
            report.admission_drain_report.pending_admission_count_before,
            1
        );
        assert_eq!(
            report.admission_drain_report.pending_admission_count_after,
            0
        );
        let core_window = report
            .admission_drain_report
            .core_window_id
            .expect("live admission drain 必须返回 core window");
        assert_eq!(coordinator.admission_pending_count(), 0);
        assert_eq!(
            coordinator.admission_surface_mapping(registration.adapter_surface_id),
            Some(12_000)
        );
        assert_eq!(
            coordinator.admission_toplevel_mapping(registration.adapter_toplevel_id),
            Some(core_window)
        );
        assert!(state.validate().is_clean());
    }

    /// 验证同一个 coordinator/display 可连续接收不同 live callback，并分别 admission。
    #[test]
    fn nested_runtime_live_admission_pump_accepts_distinct_callback_observations() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-live-admission-multi");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                13_000,
            )
            .expect("coordinator 必须绑定测试 socket");
        {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_xdg_shell_global()
                .expect("测试 xdg-shell global 必须初始化");
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
        }
        let first_registration = {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            adapter_toplevel_identity_registration_report(display)
                .expect("首次 adapter identity registration proof 必须完成")
        };
        let mut state = State::new();

        let first_report = coordinator.pump_once_with_live_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(53),
        );
        let second_registration = {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            adapter_toplevel_identity_registration_report(display)
                .expect("第二次 adapter identity registration proof 必须完成")
        };
        let second_report = coordinator.pump_once_with_live_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(54),
        );

        assert!(first_report.lifecycle_report.is_successful());
        assert!(second_report.lifecycle_report.is_successful());
        assert_ne!(
            first_registration.new_toplevel_callback_sequence,
            second_registration.new_toplevel_callback_sequence
        );
        assert_ne!(
            first_registration.adapter_surface_id,
            second_registration.adapter_surface_id
        );
        assert_ne!(
            first_registration.adapter_toplevel_id,
            second_registration.adapter_toplevel_id
        );
        assert_eq!(
            first_report
                .live_admission_owner_report
                .new_toplevel_callback_sequence,
            Some(first_registration.new_toplevel_callback_sequence)
        );
        assert_eq!(
            second_report
                .live_admission_owner_report
                .new_toplevel_callback_sequence,
            Some(second_registration.new_toplevel_callback_sequence)
        );
        assert!(
            first_report
                .live_admission_owner_report
                .coordinator_enqueue_invoked
        );
        assert!(
            second_report
                .live_admission_owner_report
                .coordinator_enqueue_invoked
        );
        assert!(
            first_report
                .admission_drain_report
                .pending_admission_consumed
        );
        assert!(
            second_report
                .admission_drain_report
                .pending_admission_consumed
        );
        assert_eq!(
            first_report.admission_drain_report.core_surface_id,
            Some(13_000)
        );
        assert_eq!(
            second_report.admission_drain_report.core_surface_id,
            Some(13_001)
        );
        assert_eq!(
            first_report
                .admission_drain_report
                .next_core_surface_id_after,
            13_001
        );
        assert_eq!(
            second_report
                .admission_drain_report
                .next_core_surface_id_after,
            13_002
        );
        assert_eq!(
            coordinator.admission_surface_mapping(first_registration.adapter_surface_id),
            Some(13_000)
        );
        assert_eq!(
            coordinator.admission_surface_mapping(second_registration.adapter_surface_id),
            Some(13_001)
        );
        assert!(
            coordinator
                .admission_toplevel_mapping(first_registration.adapter_toplevel_id)
                .is_some()
        );
        assert!(
            coordinator
                .admission_toplevel_mapping(second_registration.adapter_toplevel_id)
                .is_some()
        );
        assert_eq!(coordinator.admission_pending_count(), 0);
        assert_eq!(coordinator.admission_next_core_surface_id(), 13_002);
        assert!(state.surfaces.get(13_000).is_some());
        assert!(state.surfaces.get(13_001).is_some());
        assert_eq!(state.surfaces.records().len(), 2);
        assert!(state.validate().is_clean());
    }

    /// 验证 pump 读取前累积的 live callback observations 不会只保留最后一条。
    #[test]
    fn nested_runtime_live_admission_pump_drains_backlogged_callback_observations() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-live-admission-backlog");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                14_000,
            )
            .expect("coordinator 必须绑定测试 socket");
        {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_xdg_shell_global()
                .expect("测试 xdg-shell global 必须初始化");
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
        }
        let first_registration = {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            adapter_toplevel_identity_registration_report(display)
                .expect("首次 adapter identity registration proof 必须完成")
        };
        let second_registration = {
            let display = coordinator
                .flow
                .display_mut_for_controlled_toplevel_registration();
            adapter_toplevel_identity_registration_report(display)
                .expect("第二次 adapter identity registration proof 必须完成")
        };
        let mut state = State::new();

        let first_report = coordinator.pump_once_with_live_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(55),
        );
        let second_report = coordinator.pump_once_with_live_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(56),
        );

        assert!(first_report.lifecycle_report.is_successful());
        assert!(second_report.lifecycle_report.is_successful());
        assert_eq!(
            first_report
                .live_admission_owner_report
                .new_toplevel_callback_sequence,
            Some(first_registration.new_toplevel_callback_sequence)
        );
        assert_eq!(
            second_report
                .live_admission_owner_report
                .new_toplevel_callback_sequence,
            Some(second_registration.new_toplevel_callback_sequence)
        );
        assert!(
            first_report
                .live_admission_owner_report
                .coordinator_enqueue_invoked
        );
        assert!(
            second_report
                .live_admission_owner_report
                .coordinator_enqueue_invoked
        );
        assert!(
            first_report
                .admission_drain_report
                .pending_admission_consumed
        );
        assert!(
            second_report
                .admission_drain_report
                .pending_admission_consumed
        );
        assert_eq!(
            first_report.admission_drain_report.core_surface_id,
            Some(14_000)
        );
        assert_eq!(
            second_report.admission_drain_report.core_surface_id,
            Some(14_001)
        );
        assert_eq!(
            coordinator.admission_surface_mapping(first_registration.adapter_surface_id),
            Some(14_000)
        );
        assert_eq!(
            coordinator.admission_surface_mapping(second_registration.adapter_surface_id),
            Some(14_001)
        );
        assert_eq!(coordinator.admission_pending_count(), 0);
        assert_eq!(coordinator.admission_next_core_surface_id(), 14_002);
        assert!(state.surfaces.get(14_000).is_some());
        assert!(state.surfaces.get(14_001).is_some());
        assert_eq!(state.surfaces.records().len(), 2);
        assert!(state.validate().is_clean());
    }
}
