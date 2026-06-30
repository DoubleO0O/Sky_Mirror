//! Linux-only live callback observation to coordinator admission queue owner.
//!
//! This module reads the latest adapter-owned `new_toplevel` identity
//! observation from the Wayland display owner, converts it through the Phase
//! 52V pending admission bridge, and enqueues the resulting intent into
//! `NestedRuntimeCoordinator`. Handler code remains on the callback/identity
//! side of the boundary; ledger and core admission remain owned by the runtime
//! coordinator drain path.

use super::{
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
}

impl LiveToplevelAdmissionOwnerObservation {
    /// 从 display owner 读取 live callback/identity observation 的纯数据快照。
    pub fn from_display(server: &SmithayWaylandDisplayProbe) -> Self {
        Self {
            new_toplevel_callback_sequence: server
                .last_new_toplevel_callback_observation_sequence(),
            adapter_toplevel_identity_registration: server
                .last_adapter_toplevel_identity_registration_observation(),
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

    if !blockers.is_empty() {
        operations.push(LiveToplevelAdmissionOwnerOperation::BuildReport);
        return LiveToplevelAdmissionOwnerReport {
            new_toplevel_callback_observed: callback_sequence.is_some(),
            new_toplevel_callback_sequence: callback_sequence,
            adapter_toplevel_identity_observation_available,
            adapter_toplevel_identity_registered: false,
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
        coordinator_enqueue_report = Some(coordinator.enqueue_pending_toplevel_admission(
            pending_admission.expect("pending admission 已由 blocker 检查"),
        ));
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
    use std::time::Duration;

    use crate::{
        core::state::State,
        smithay_backend::{
            linux_live_toplevel_admission_owner::{
                LiveToplevelAdmissionOwnerBlocker, enqueue_live_toplevel_admission_from_display,
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
}
