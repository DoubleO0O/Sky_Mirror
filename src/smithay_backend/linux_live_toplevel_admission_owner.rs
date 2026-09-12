//! Linux-only live callback observation to coordinator admission queue owner.
//!
//! This module reads the latest adapter-owned `new_toplevel` identity
//! observation from the Wayland display owner, converts it through the Phase
//! 52V pending admission bridge, and enqueues the resulting intent into
//! `NestedRuntimeCoordinator`. Handler code remains on the callback/identity
//! side of the boundary; ledger and core admission remain owned by the runtime
//! coordinator drain path.

use super::{
    client_session::NestedClientSessionId,
    linux_toplevel_admission_bridge::{
        LiveToplevelAdmissionBridgeBlocker, LiveToplevelAdmissionBridgeInput,
        LiveToplevelAdmissionBridgeReport, live_toplevel_admission_bridge_report,
    },
    linux_toplevel_admission_runtime_queue::RuntimeToplevelAdmissionEnqueueReport,
    linux_toplevel_identity_registration::AdapterToplevelIdentityRegistrationError,
    nested_runtime_coordinator::NestedRuntimeCoordinator,
    wayland_display::SmithayWaylandDisplayProbe,
    xdg_toplevel_identity::XdgToplevelIdentityMapping,
};

/// Phase 53A live admission owner 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveToplevelAdmissionOwnerOperation {
    /// 读取 display owner 中的 live callback/identity observation。
    ReadDisplayObservation,
    /// 构造 Phase 52V bridge input。
    BuildBridgeInput,
    /// 调用 Phase 52V bridge report。
    BuildBridgeReport,
    /// 将 pending admission intent 入队到 coordinator runtime owner。
    EnqueueCoordinatorAdmission,
    /// 生成保守 report。
    BuildReport,
}

/// Phase 53A live admission owner 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveToplevelAdmissionOwnerBlocker {
    /// display owner 尚未观察到 `new_toplevel` callback。
    MissingNewToplevelCallbackObservation,
    /// display owner 尚未保存 adapter toplevel identity registration observation。
    MissingAdapterToplevelIdentityRegistrationObservation,
    /// adapter toplevel identity registration observation 是结构化失败。
    AdapterToplevelIdentityRegistrationFailed(AdapterToplevelIdentityRegistrationError),
    /// Phase 52V bridge 返回了 blocker。
    BridgeBlocked(Vec<LiveToplevelAdmissionBridgeBlocker>),
    /// Phase 52V bridge 没有产出 pending admission intent。
    MissingBridgePendingAdmission,
    /// 当前 callback sequence 已经被 coordinator admission owner 处理过。
    DuplicateNewToplevelCallbackObservation(u64),
    /// production callback 没有携带可解析的 source session。
    MissingSourceSession,
    /// production callback 的 source session 不在同一 active flow bridge 中。
    UnknownSourceSession(NestedClientSessionId),
}

/// Phase 53A live callback admission owner 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveToplevelAdmissionOwnerReport {
    /// 是否观察到 `new_toplevel` callback。
    pub new_toplevel_callback_observed: bool,
    /// 最近一次 callback observation 序号。
    pub new_toplevel_callback_sequence: Option<u64>,
    /// 是否存在 adapter toplevel identity observation。
    pub adapter_toplevel_identity_observation_available: bool,
    /// 是否成功读取 adapter toplevel identity registration。
    pub adapter_toplevel_identity_registered: bool,
    /// 是否构造 bridge input。
    pub bridge_input_created: bool,
    /// Phase 52V bridge report。
    pub bridge_report: Option<LiveToplevelAdmissionBridgeReport>,
    /// 是否生成 pending admission intent。
    pub pending_admission_intent_created: bool,
    /// 是否调用 coordinator enqueue seam。
    pub coordinator_enqueue_invoked: bool,
    /// coordinator enqueue report。
    pub coordinator_enqueue_report: Option<RuntimeToplevelAdmissionEnqueueReport>,
    /// 入队前 coordinator admission queue 的 pending 数量。
    pub coordinator_pending_admission_count_before: usize,
    /// 入队后 coordinator admission queue 的 pending 数量。
    pub coordinator_pending_admission_count_after: usize,
    /// handler 是否被要求直接接触 runtime queue；本 phase 固定 false。
    pub handler_state_touched: bool,
    /// 是否调用 ledger admit；本 phase 固定 false。
    pub ledger_admit_invoked: bool,
    /// 是否触发 core register；本 phase 固定 false。
    pub core_register_invoked: bool,
    /// 是否分配 core window identity；本 phase 固定 false。
    pub window_id_allocated: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 是否已有真实 compositor runtime。
    pub real_compositor_runtime_available: bool,
    /// 是否已有真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// 执行过的操作。
    pub operations: Vec<LiveToplevelAdmissionOwnerOperation>,
    /// 失败或未完成原因。
    pub blockers: Vec<LiveToplevelAdmissionOwnerBlocker>,
}

/// Phase 53B coordinator 从 display owner 读取出的 live admission observation 快照。
///
/// 该快照只携带纯数据 callback sequence 与 adapter identity mapping。coordinator 先
/// 读取快照释放 display 借用，再把快照交给 owner 入队，避免同时借用 display 与
/// `NestedRuntimeCoordinator`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveToplevelAdmissionOwnerObservation {
    /// 最近一次 `new_toplevel` callback observation 序号。
    pub new_toplevel_callback_sequence: Option<u64>,
    /// 最近一次 adapter toplevel identity registration observation。
    pub adapter_toplevel_identity_registration:
        Option<Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError>>,
    /// 与同一 callback sequence 绑定的 adapter session。它不是 Core client ID；
    /// coordinator 必须经 session/core bridge 显式解析，不能转换或猜测数值。
    pub source_session: Option<NestedClientSessionId>,
}

impl LiveToplevelAdmissionOwnerObservation {
    /// 从 display owner 读取 live callback/identity observation 的纯数据快照。
    pub fn from_display(server: &SmithayWaylandDisplayProbe) -> Self {
        Self {
            new_toplevel_callback_sequence: server
                .last_new_toplevel_callback_observation_sequence(),
            adapter_toplevel_identity_registration: server
                .last_adapter_toplevel_identity_registration_observation(),
            source_session: server.last_toplevel_callback_session(),
        }
    }
}

/// 从 display owner 的 live callback observation 入队一条 coordinator admission intent。
///
/// 本函数不创建 client harness，不 dispatch Wayland requests，也不消费 ledger/core。
/// 它只读取 display owner 已保存的纯数据 observation，然后调用 coordinator 的
/// runtime admission queue enqueue seam。
pub fn enqueue_live_toplevel_admission_from_display(
    server: &SmithayWaylandDisplayProbe,
    coordinator: &mut NestedRuntimeCoordinator,
) -> LiveToplevelAdmissionOwnerReport {
    let observation = LiveToplevelAdmissionOwnerObservation::from_display(server);
    enqueue_live_toplevel_admission_from_observation(observation, coordinator)
}

/// 从已读取的 live admission observation 快照入队一条 coordinator admission intent。
///
/// coordinator 组合 pump 使用该 seam 避免同时借用 display owner 与 coordinator
/// runtime owner；语义与 [`enqueue_live_toplevel_admission_from_display`] 保持一致。
pub fn enqueue_live_toplevel_admission_from_observation(
    observation: LiveToplevelAdmissionOwnerObservation,
    coordinator: &mut NestedRuntimeCoordinator,
) -> LiveToplevelAdmissionOwnerReport {
    let mut operations = vec![LiveToplevelAdmissionOwnerOperation::ReadDisplayObservation];
    let coordinator_pending_admission_count_before = coordinator.admission_pending_count();
    let callback_sequence = observation.new_toplevel_callback_sequence;
    let mut blockers = Vec::new();
    let mut adapter_toplevel_identity_observation_available = false;

    if callback_sequence.is_none() {
        blockers.push(LiveToplevelAdmissionOwnerBlocker::MissingNewToplevelCallbackObservation);
    }
    if let Some(callback_sequence) = callback_sequence
        && coordinator.has_seen_live_toplevel_callback_sequence(callback_sequence)
    {
        blockers.push(
            LiveToplevelAdmissionOwnerBlocker::DuplicateNewToplevelCallbackObservation(
                callback_sequence,
            ),
        );
    }

    let registration = match observation.adapter_toplevel_identity_registration {
        Some(Ok(registration)) => {
            adapter_toplevel_identity_observation_available = true;
            Some(registration)
        }
        Some(Err(error)) => {
            adapter_toplevel_identity_observation_available = true;
            blockers.push(
                LiveToplevelAdmissionOwnerBlocker::AdapterToplevelIdentityRegistrationFailed(error),
            );
            None
        }
        None => {
            blockers.push(
                LiveToplevelAdmissionOwnerBlocker::MissingAdapterToplevelIdentityRegistrationObservation,
            );
            None
        }
    };

    // production bootstrap 下，每一条 callback 都必须携带可解析的 source session。
    // active session 数量不能放宽这个条件：0/1/多 client 都无法安全猜测归属。受控
    // helper 则使用非-production coordinator，保留 adapter-only proof 的既有范围。
    let production_core_client = if coordinator.production_protocol_bootstrap_report().is_some() {
        match observation.source_session {
            Some(source_session) => match coordinator.core_client_for_session(source_session) {
                Some(core_client) => Some(core_client),
                None => {
                    blockers.push(LiveToplevelAdmissionOwnerBlocker::UnknownSourceSession(
                        source_session,
                    ));
                    None
                }
            },
            None => {
                blockers.push(LiveToplevelAdmissionOwnerBlocker::MissingSourceSession);
                None
            }
        }
    } else {
        None
    };

    if !blockers.is_empty() {
        operations.push(LiveToplevelAdmissionOwnerOperation::BuildReport);
        return LiveToplevelAdmissionOwnerReport {
            new_toplevel_callback_observed: callback_sequence.is_some(),
            new_toplevel_callback_sequence: callback_sequence,
            adapter_toplevel_identity_observation_available,
            adapter_toplevel_identity_registered: registration.is_some(),
            bridge_input_created: false,
            bridge_report: None,
            pending_admission_intent_created: false,
            coordinator_enqueue_invoked: false,
            coordinator_enqueue_report: None,
            coordinator_pending_admission_count_before,
            coordinator_pending_admission_count_after: coordinator.admission_pending_count(),
            handler_state_touched: false,
            ledger_admit_invoked: false,
            core_register_invoked: false,
            window_id_allocated: false,
            render_support: false,
            input_support: false,
            real_compositor_runtime_available: false,
            real_xdg_shell_runtime_available: false,
            operations,
            blockers,
        };
    }

    let registration = registration.expect("registration 已由 blocker 检查");
    let callback_sequence = callback_sequence.expect("callback sequence 已由 blocker 检查");
    operations.push(LiveToplevelAdmissionOwnerOperation::BuildBridgeInput);
    let bridge_input = LiveToplevelAdmissionBridgeInput::from_registered_identity(
        registration.adapter_surface,
        registration.adapter_toplevel,
        callback_sequence,
    );
    operations.push(LiveToplevelAdmissionOwnerOperation::BuildBridgeReport);
    let bridge_report = live_toplevel_admission_bridge_report(bridge_input);
    let blocking_bridge_blockers = bridge_report
        .blockers
        .iter()
        .copied()
        .filter(|blocker| {
            !matches!(
                blocker,
                LiveToplevelAdmissionBridgeBlocker::MissingLedgerOwner
                    | LiveToplevelAdmissionBridgeBlocker::MissingStateOwner
            )
        })
        .collect::<Vec<_>>();
    if !blocking_bridge_blockers.is_empty() {
        blockers.push(LiveToplevelAdmissionOwnerBlocker::BridgeBlocked(
            blocking_bridge_blockers,
        ));
    }

    let pending_admission = bridge_report.pending_admission;
    if pending_admission.is_none() {
        blockers.push(LiveToplevelAdmissionOwnerBlocker::MissingBridgePendingAdmission);
    }

    let mut coordinator_enqueue_report = None;
    let mut coordinator_enqueue_invoked = false;
    if blockers.is_empty() {
        operations.push(LiveToplevelAdmissionOwnerOperation::EnqueueCoordinatorAdmission);
        coordinator_enqueue_invoked = true;
        let pending_admission = pending_admission.expect("pending admission 已由 blocker 检查");
        let pending_admission = production_core_client
            .map(|core_client| pending_admission.with_core_client(core_client))
            .unwrap_or(pending_admission);
        coordinator_enqueue_report =
            Some(coordinator.enqueue_pending_toplevel_admission(pending_admission));
        if coordinator_enqueue_report
            .as_ref()
            .is_some_and(|report| report.pending_admission_enqueued)
        {
            coordinator.mark_live_toplevel_callback_sequence_seen(callback_sequence);
        }
    }
    operations.push(LiveToplevelAdmissionOwnerOperation::BuildReport);

    LiveToplevelAdmissionOwnerReport {
        new_toplevel_callback_observed: true,
        new_toplevel_callback_sequence: Some(callback_sequence),
        adapter_toplevel_identity_observation_available: true,
        adapter_toplevel_identity_registered: true,
        bridge_input_created: true,
        pending_admission_intent_created: pending_admission.is_some(),
        bridge_report: Some(bridge_report),
        coordinator_enqueue_invoked,
        coordinator_enqueue_report,
        coordinator_pending_admission_count_before,
        coordinator_pending_admission_count_after: coordinator.admission_pending_count(),
        handler_state_touched: false,
        ledger_admit_invoked: false,
        core_register_invoked: false,
        window_id_allocated: false,
        render_support: false,
        input_support: false,
        real_compositor_runtime_available: false,
        real_xdg_shell_runtime_available: false,
        operations,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixStream, path::Path, time::Duration};

    use crate::{
        core::state::State,
        smithay_backend::{
            client_session::NestedClientSessionId,
            linux_live_toplevel_admission_owner::{
                LiveToplevelAdmissionOwnerBlocker, LiveToplevelAdmissionOwnerObservation,
                enqueue_live_toplevel_admission_from_display,
                enqueue_live_toplevel_admission_from_observation,
            },
            linux_toplevel_admission_bridge::LiveToplevelAdmissionBridgeBlocker,
            linux_toplevel_admission_runtime_queue::RuntimeToplevelAdmissionDrainTick,
            linux_toplevel_identity_registration::adapter_toplevel_identity_registration_report,
            nested_runtime_coordinator::NestedRuntimeCoordinator,
            test_support::{assert_runtime_dir, unique_socket_name},
            wayland_display::SmithayWaylandDisplayProbe,
        },
    };

    #[test]
    fn live_admission_owner_requires_callback_observation_before_enqueue() {
        assert_runtime_dir();
        let server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        let socket_name = unique_socket_name("phase53a-live-admission-missing");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                10_000,
            )
            .expect("coordinator 必须绑定测试 socket");

        let report = enqueue_live_toplevel_admission_from_display(&server, &mut coordinator);

        assert!(!report.new_toplevel_callback_observed);
        assert_eq!(report.new_toplevel_callback_sequence, None);
        assert!(!report.adapter_toplevel_identity_observation_available);
        assert!(!report.adapter_toplevel_identity_registered);
        assert!(!report.bridge_input_created);
        assert_eq!(report.bridge_report, None);
        assert!(!report.pending_admission_intent_created);
        assert!(!report.coordinator_enqueue_invoked);
        assert_eq!(report.coordinator_pending_admission_count_before, 0);
        assert_eq!(report.coordinator_pending_admission_count_after, 0);
        assert_eq!(coordinator.admission_pending_count(), 0);
        assert!(
            report.blockers.contains(
                &LiveToplevelAdmissionOwnerBlocker::MissingNewToplevelCallbackObservation
            )
        );
        assert!(report.blockers.contains(
            &LiveToplevelAdmissionOwnerBlocker::MissingAdapterToplevelIdentityRegistrationObservation
        ));
    }

    #[test]
    fn live_admission_owner_enqueues_observed_callback_for_coordinator_drain() {
        assert_runtime_dir();
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");
        let registration = adapter_toplevel_identity_registration_report(&mut server)
            .expect("adapter identity registration proof 必须完成");
        let socket_name = unique_socket_name("phase53a-live-admission-enqueue");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                11_000,
            )
            .expect("coordinator 必须绑定测试 socket");

        let report = enqueue_live_toplevel_admission_from_display(&server, &mut coordinator);

        assert!(report.new_toplevel_callback_observed);
        assert_eq!(
            report.new_toplevel_callback_sequence,
            Some(registration.new_toplevel_callback_sequence)
        );
        assert!(report.adapter_toplevel_identity_observation_available);
        assert!(report.adapter_toplevel_identity_registered);
        assert!(report.bridge_input_created);
        assert!(report.pending_admission_intent_created);
        assert!(report.coordinator_enqueue_invoked);
        let bridge = report
            .bridge_report
            .as_ref()
            .expect("owner 必须保留 Phase 52V bridge report");
        assert!(
            bridge
                .blockers
                .contains(&LiveToplevelAdmissionBridgeBlocker::MissingLedgerOwner)
        );
        assert!(
            bridge
                .blockers
                .contains(&LiveToplevelAdmissionBridgeBlocker::MissingStateOwner)
        );
        assert_eq!(report.coordinator_pending_admission_count_before, 0);
        assert_eq!(report.coordinator_pending_admission_count_after, 1);
        assert_eq!(coordinator.admission_pending_count(), 1);
        assert!(!report.handler_state_touched);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.blockers.is_empty());
        let enqueue = report
            .coordinator_enqueue_report
            .expect("owner 必须返回 coordinator enqueue report");
        assert!(enqueue.pending_admission_enqueued);
        assert_eq!(enqueue.pending_admission_count_after, 1);

        let mut state = State::new();
        let drain = coordinator.pump_once_with_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(53),
        );

        assert!(drain.lifecycle_report.is_successful());
        assert!(drain.admission_drain_report.pending_admission_consumed);
        assert_eq!(drain.admission_drain_report.core_surface_id, Some(11_000));
        assert_eq!(
            drain.admission_drain_report.pending_admission_count_after,
            0
        );
        let core_window = drain
            .admission_drain_report
            .core_window_id
            .expect("admission drain 必须返回 core window");
        assert_eq!(
            coordinator.admission_surface_mapping(registration.adapter_surface_id),
            Some(11_000)
        );
        assert_eq!(
            coordinator.admission_toplevel_mapping(registration.adapter_toplevel_id),
            Some(core_window)
        );
        assert!(state.validate().is_clean());
    }

    /// Red：callback 关联的 adapter session 未经 active session bridge 解析时，不能
    /// 仅凭 adapter ObjectId 将 toplevel 放入 admission queue。错误 session 必须在
    /// coordinator 边界被拒绝，既不新增 pending，也不触发任何 Core mutation。
    #[test]
    fn live_admission_owner_rejects_unknown_source_session_before_enqueue() {
        assert_runtime_dir();
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        let registration = adapter_toplevel_identity_registration_report(&mut server)
            .expect("受控 client 必须得到真实 callback identity");
        let socket_name = unique_socket_name("phase56q-unknown-session");
        let mut coordinator =
            NestedRuntimeCoordinator::with_production_protocol_bootstrap(&socket_name)
                .expect("production coordinator 必须绑定测试 socket");
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").expect("需要 XDG_RUNTIME_DIR");
        let peer = UnixStream::connect(Path::new(&runtime_dir).join(coordinator.socket_name()))
            .expect("unknown-session test peer 必须连接 production socket");
        let mut state = State::new();
        let connected = coordinator.pump_once(&mut state, Duration::from_secs(1));
        assert_eq!(connected.registered_core_clients.len(), 1);

        let report = enqueue_live_toplevel_admission_from_observation(
            LiveToplevelAdmissionOwnerObservation {
                new_toplevel_callback_sequence: Some(registration.new_toplevel_callback_sequence),
                adapter_toplevel_identity_registration: Some(Ok(
                    crate::smithay_backend::xdg_toplevel_identity::XdgToplevelIdentityMapping {
                        adapter_toplevel: registration.adapter_toplevel_id,
                        adapter_surface: registration.adapter_surface_id,
                    },
                )),
                source_session: NestedClientSessionId::new(9_999),
            },
            &mut coordinator,
        );

        assert!(report.new_toplevel_callback_observed);
        assert!(report.adapter_toplevel_identity_registered);
        assert!(
            !report.coordinator_enqueue_invoked,
            "unknown adapter session 绝不能进入 coordinator admission queue"
        );
        let unknown_session = NestedClientSessionId::new(9_999).expect("session id 有效");
        assert!(report.blockers.iter().any(|blocker| {
            *blocker == LiveToplevelAdmissionOwnerBlocker::UnknownSourceSession(unknown_session)
        }));
        assert_eq!(coordinator.admission_pending_count(), 0);
        drop(peer);
    }

    /// Red：只要 coordinator 已按 production protocol bootstrap 建立，缺失
    /// source_session 就必须 fail closed；active session 数量不能成为放行条件。三个
    /// case 覆盖没有 peer、一个 peer 与多个 peer，确保不会把 callback 猜配给任意 Core
    /// client，也不会污染 queue、ledger 或 State。
    #[test]
    fn production_admission_owner_rejects_missing_source_session_for_zero_one_and_many_clients() {
        assert_runtime_dir();

        for active_client_count in [0usize, 1, 2] {
            let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
            server
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            server
                .initialize_xdg_shell_global()
                .expect("测试 xdg-shell global 必须初始化");
            let registration = adapter_toplevel_identity_registration_report(&mut server)
                .expect("受控 identity registration 必须完成");
            let socket_name = unique_socket_name("phase56q-missing-source-session");
            let mut coordinator =
                NestedRuntimeCoordinator::with_production_protocol_bootstrap(&socket_name)
                    .expect("production coordinator 必须绑定测试 socket");
            let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").expect("需要 XDG_RUNTIME_DIR");
            let socket_path = Path::new(&runtime_dir).join(coordinator.socket_name());
            let mut peers = Vec::new();
            let mut state = State::new();
            for _ in 0..active_client_count {
                peers.push(UnixStream::connect(&socket_path).expect("测试 peer 必须连接 socket"));
                let lifecycle = coordinator.pump_once(&mut state, Duration::from_secs(1));
                assert_eq!(lifecycle.registered_core_clients.len(), 1);
            }
            let surface_count_before = state.surfaces.records().len();
            let window_count_before = state.registry.records().len();

            let report = enqueue_live_toplevel_admission_from_observation(
                LiveToplevelAdmissionOwnerObservation {
                    new_toplevel_callback_sequence: Some(
                        registration.new_toplevel_callback_sequence,
                    ),
                    adapter_toplevel_identity_registration: Some(Ok(
                        crate::smithay_backend::xdg_toplevel_identity::XdgToplevelIdentityMapping {
                            adapter_toplevel: registration.adapter_toplevel_id,
                            adapter_surface: registration.adapter_surface_id,
                        },
                    )),
                    source_session: None,
                },
                &mut coordinator,
            );

            assert!(
                report
                    .blockers
                    .contains(&LiveToplevelAdmissionOwnerBlocker::MissingSourceSession)
            );
            assert!(!report.coordinator_enqueue_invoked);
            assert_eq!(coordinator.admission_pending_count(), 0);
            assert_eq!(state.surfaces.records().len(), surface_count_before);
            assert_eq!(state.registry.records().len(), window_count_before);
            assert!(state.validate().is_clean());
            drop(peers);
        }
    }
}
