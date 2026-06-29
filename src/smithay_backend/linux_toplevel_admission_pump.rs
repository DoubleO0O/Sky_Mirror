//! Linux-only controlled pending admission pump seam.
//!
//! This module connects the existing controlled adapter toplevel registration
//! report to the Phase 52V bridge queue and Phase 52W owner consumer. It does
//! not create protocol clients, dispatch server requests, or touch handler
//! state directly.

use crate::core::{
    state::State,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::WindowId,
};

use super::{
    linux_toplevel_admission_bridge::{
        LiveToplevelAdmissionBridgeInput, LiveToplevelAdmissionBridgeReport,
        PendingXdgToplevelAdmission, ToplevelAdmissionBridgeQueue,
        live_toplevel_admission_bridge_report,
    },
    linux_toplevel_admission_consumer::{
        PendingToplevelAdmissionConsumerBlocker, PendingToplevelAdmissionConsumerInput,
        PendingToplevelAdmissionConsumerReport, consume_pending_toplevel_admission,
    },
    linux_toplevel_identity_registration::AdapterToplevelIdentityRegistrationReport,
    surface_xdg_admission::SurfaceXdgAdmissionLedger,
};

/// Controlled admission pump owner 的输入。
///
/// Adapter identity 来自 `AdapterToplevelIdentityRegistrationReport`；本输入只提供
/// ledger/core admission 需要的 core surface identity 和 metadata。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledToplevelAdmissionPumpInput {
    /// owner 准备交给 ledger 的 core surface identity。
    pub core_surface_id: SurfaceId,
    /// 进入 core window registry 的 title metadata。
    pub title: String,
    /// 可选 application id metadata。
    pub app_id: Option<String>,
    /// core window kind；本 proof 仍使用 mock kind。
    pub kind: WindowKind,
}

impl ControlledToplevelAdmissionPumpInput {
    /// 构造 Phase 52X 默认 controlled pump 输入。
    pub fn phase52x_default(core_surface_id: SurfaceId) -> Self {
        Self {
            core_surface_id,
            title: "Phase 52X controlled admission pump".to_string(),
            app_id: Some("sky-mirror-phase52x".to_string()),
            kind: WindowKind::Mock,
        }
    }
}

/// Controlled admission pump 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlledToplevelAdmissionPumpBlocker {
    /// registration report 没有观察到 `new_toplevel` callback。
    MissingNewToplevelCallbackObservation,
    /// registration report 没有 adapter surface identity。
    MissingAdapterSurfaceIdentity,
    /// registration report 没有 adapter toplevel identity registration。
    MissingAdapterToplevelIdentityRegistration,
    /// adapter surface identity 没有链接到 toplevel registration。
    AdapterSurfaceIdentityMismatch,
    /// bridge 没有产出 pending admission intent。
    MissingBridgePendingAdmission,
    /// consumer owner 返回了结构化 blocker。
    ConsumerBlocked(Vec<PendingToplevelAdmissionConsumerBlocker>),
}

/// Controlled admission pump 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledToplevelAdmissionPumpOperation {
    /// 读取 controlled registration report。
    ReadRegistrationReport,
    /// 构造 live callback admission bridge input。
    BuildBridgeInput,
    /// 生成 bridge report。
    BuildBridgeReport,
    /// 创建 owner queue。
    CreateOwnerQueue,
    /// 将 bridge pending intent 放入 owner queue。
    QueuePendingAdmissionIntent,
    /// 构造 owner consumer input。
    BuildConsumerInput,
    /// 调用 pending admission consumer。
    ConsumePendingAdmission,
    /// 生成保守 capability report。
    BuildReport,
}

/// Controlled admission pump 的能力报告。
///
/// 成功表示 controlled registration report 已经经 bridge queue 进入 owner consumer，
/// 并通过 ledger/core admission seam 得到 core `WindowId`。这仍不代表 render/input
/// 或长期 compositor runtime 已完成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledToplevelAdmissionPumpReport {
    /// 输入 registration 是否观察到 callback。
    pub new_toplevel_callback_observed: bool,
    /// 输入 registration 是否有 adapter surface identity。
    pub adapter_surface_identity_available: bool,
    /// 输入 registration 是否有 adapter toplevel identity。
    pub adapter_toplevel_identity_registered: bool,
    /// 输入 registration 的 surface/toplevel link 是否一致。
    pub adapter_surface_id_linked: bool,
    /// controlled protocol proof 是否已经启动过 dispatch。
    pub protocol_dispatch_started: bool,
    /// 是否构造 bridge input。
    pub bridge_input_created: bool,
    /// bridge report。
    pub bridge_report: LiveToplevelAdmissionBridgeReport,
    /// bridge 是否产出 pending admission intent。
    pub pending_admission_intent_created: bool,
    /// 消费前 queue 中的 pending 数量。
    pub pending_admission_count_before_consume: usize,
    /// 消费后 queue 中的 pending 数量。
    pub pending_admission_count_after_consume: usize,
    /// 被 pump 传递给 owner consumer 的 pending intent。
    pub pending_admission: Option<PendingXdgToplevelAdmission>,
    /// owner consumer report。
    pub consumer_report: Option<PendingToplevelAdmissionConsumerReport>,
    /// owner 是否尝试消费 ledger。
    pub ledger_consume_attempted: bool,
    /// pending intent 是否被消费。
    pub pending_admission_consumed: bool,
    /// 是否调用 ledger admit_surface。
    pub ledger_admit_surface_invoked: bool,
    /// 是否调用 ledger admit_toplevel。
    pub ledger_admit_invoked: bool,
    /// 是否触发既有 core register seam。
    pub core_register_invoked: bool,
    /// 是否分配 core window identity。
    pub window_id_allocated: bool,
    /// core surface identity。
    pub core_surface_id: Option<SurfaceId>,
    /// core window identity。
    pub core_window_id: Option<WindowId>,
    /// handler 是否被要求直接接触 State；本 phase 固定 false。
    pub handler_state_touched: bool,
    /// 是否绕过 ledger；本 phase 固定 false。
    pub ledger_bypassed: bool,
    /// 是否已有 render 支持。
    pub render_support: bool,
    /// 是否已有 input 支持。
    pub input_support: bool,
    /// 是否已有真实 compositor runtime。
    pub real_compositor_runtime_available: bool,
    /// 是否已有真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// pump 执行过的操作。
    pub operations: Vec<ControlledToplevelAdmissionPumpOperation>,
    /// 失败或未完成原因。
    pub blockers: Vec<ControlledToplevelAdmissionPumpBlocker>,
}

/// 执行一次 controlled toplevel admission owner pump。
///
/// 调用方提供已经完成的 controlled registration report，以及 owner 持有的 ledger 和
/// `State`。本函数只编排 Phase 52V bridge 与 Phase 52W consumer，不创建或 dispatch
/// Wayland protocol objects。
pub fn pump_controlled_toplevel_admission(
    registration: AdapterToplevelIdentityRegistrationReport,
    ledger: &mut SurfaceXdgAdmissionLedger,
    state: &mut State,
    input: ControlledToplevelAdmissionPumpInput,
) -> ControlledToplevelAdmissionPumpReport {
    let mut operations = vec![ControlledToplevelAdmissionPumpOperation::ReadRegistrationReport];
    let mut blockers = registration_blockers(&registration);

    operations.push(ControlledToplevelAdmissionPumpOperation::BuildBridgeInput);
    let bridge_input = LiveToplevelAdmissionBridgeInput::from(&registration);
    operations.push(ControlledToplevelAdmissionPumpOperation::BuildBridgeReport);
    let bridge_report = live_toplevel_admission_bridge_report(bridge_input);

    operations.push(ControlledToplevelAdmissionPumpOperation::CreateOwnerQueue);
    let mut queue = ToplevelAdmissionBridgeQueue::new();
    let pending = bridge_report.pending_admission;
    if let Some(pending) = pending {
        queue.push(pending);
        operations.push(ControlledToplevelAdmissionPumpOperation::QueuePendingAdmissionIntent);
    } else {
        blockers.push(ControlledToplevelAdmissionPumpBlocker::MissingBridgePendingAdmission);
    }

    if !blockers.is_empty() {
        operations.push(ControlledToplevelAdmissionPumpOperation::BuildReport);
        return ControlledToplevelAdmissionPumpReport {
            new_toplevel_callback_observed: registration.new_toplevel_callback_observed,
            adapter_surface_identity_available: registration.adapter_surface_identity_available,
            adapter_toplevel_identity_registered: registration.adapter_toplevel_identity_registered,
            adapter_surface_id_linked: registration.adapter_surface_id_linked,
            protocol_dispatch_started: registration.protocol_dispatch_started,
            bridge_input_created: true,
            bridge_report,
            pending_admission_intent_created: pending.is_some(),
            pending_admission_count_before_consume: queue.pending_count(),
            pending_admission_count_after_consume: queue.pending_count(),
            pending_admission: pending,
            consumer_report: None,
            ledger_consume_attempted: false,
            pending_admission_consumed: false,
            ledger_admit_surface_invoked: false,
            ledger_admit_invoked: false,
            core_register_invoked: false,
            window_id_allocated: false,
            core_surface_id: Some(input.core_surface_id),
            core_window_id: None,
            handler_state_touched: false,
            ledger_bypassed: false,
            render_support: false,
            input_support: false,
            real_compositor_runtime_available: false,
            real_xdg_shell_runtime_available: false,
            operations,
            blockers,
        };
    }

    let pending_admission_count_before_consume = queue.pending_count();
    operations.push(ControlledToplevelAdmissionPumpOperation::BuildConsumerInput);
    let consumer_input = PendingToplevelAdmissionConsumerInput {
        core_surface_id: input.core_surface_id,
        client: None,
        role: SurfaceRole::XdgToplevel,
        title: input.title,
        app_id: input.app_id,
        kind: input.kind,
    };
    operations.push(ControlledToplevelAdmissionPumpOperation::ConsumePendingAdmission);
    let consumer_report =
        consume_pending_toplevel_admission(&mut queue, ledger, state, consumer_input);
    if !consumer_report.blockers.is_empty() {
        blockers.push(ControlledToplevelAdmissionPumpBlocker::ConsumerBlocked(
            consumer_report.blockers.clone(),
        ));
    }
    operations.push(ControlledToplevelAdmissionPumpOperation::BuildReport);

    ControlledToplevelAdmissionPumpReport {
        new_toplevel_callback_observed: registration.new_toplevel_callback_observed,
        adapter_surface_identity_available: registration.adapter_surface_identity_available,
        adapter_toplevel_identity_registered: registration.adapter_toplevel_identity_registered,
        adapter_surface_id_linked: registration.adapter_surface_id_linked,
        protocol_dispatch_started: registration.protocol_dispatch_started,
        bridge_input_created: true,
        bridge_report,
        pending_admission_intent_created: pending.is_some(),
        pending_admission_count_before_consume,
        pending_admission_count_after_consume: queue.pending_count(),
        pending_admission: pending,
        consumer_report: Some(consumer_report.clone()),
        ledger_consume_attempted: consumer_report.ledger_consume_attempted,
        pending_admission_consumed: consumer_report.pending_admission_consumed,
        ledger_admit_surface_invoked: consumer_report.ledger_admit_surface_invoked,
        ledger_admit_invoked: consumer_report.ledger_admit_invoked,
        core_register_invoked: consumer_report.core_register_invoked,
        window_id_allocated: consumer_report.window_id_allocated,
        core_surface_id: consumer_report.core_surface_id,
        core_window_id: consumer_report.core_window_id,
        handler_state_touched: false,
        ledger_bypassed: false,
        render_support: false,
        input_support: false,
        real_compositor_runtime_available: false,
        real_xdg_shell_runtime_available: false,
        operations,
        blockers,
    }
}

fn registration_blockers(
    registration: &AdapterToplevelIdentityRegistrationReport,
) -> Vec<ControlledToplevelAdmissionPumpBlocker> {
    let mut blockers = Vec::new();
    if !registration.new_toplevel_callback_observed {
        blockers
            .push(ControlledToplevelAdmissionPumpBlocker::MissingNewToplevelCallbackObservation);
    }
    if !registration.adapter_surface_identity_available {
        blockers.push(ControlledToplevelAdmissionPumpBlocker::MissingAdapterSurfaceIdentity);
    }
    if !registration.adapter_toplevel_identity_registered {
        blockers.push(
            ControlledToplevelAdmissionPumpBlocker::MissingAdapterToplevelIdentityRegistration,
        );
    }
    if !registration.adapter_surface_id_linked {
        blockers.push(ControlledToplevelAdmissionPumpBlocker::AdapterSurfaceIdentityMismatch);
    }
    blockers
}

#[cfg(test)]
mod tests {
    use crate::{
        core::state::State,
        smithay_backend::{
            linux_toplevel_admission_pump::{
                ControlledToplevelAdmissionPumpInput, pump_controlled_toplevel_admission,
            },
            linux_toplevel_identity_registration::adapter_toplevel_identity_registration_report,
            surface_xdg_admission::SurfaceXdgAdmissionLedger,
            wayland_display::SmithayWaylandDisplayProbe,
        },
    };

    fn controlled_registration_report()
    -> crate::smithay_backend::linux_toplevel_identity_registration::AdapterToplevelIdentityRegistrationReport
    {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        adapter_toplevel_identity_registration_report(&mut server)
            .expect("controlled toplevel identity registration proof 必须完成")
    }

    /// controlled registration report 可以经 bridge queue 被 owner consumer 消费。
    #[test]
    fn controlled_registration_pump_consumes_pending_admission() {
        let registration = controlled_registration_report();
        let adapter_surface = registration.adapter_surface_id;
        let adapter_toplevel = registration.adapter_toplevel_id;
        let core_surface = 5_200 + registration.adapter_surface_id.value();
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();

        let report = pump_controlled_toplevel_admission(
            registration,
            &mut ledger,
            &mut state,
            ControlledToplevelAdmissionPumpInput::phase52x_default(core_surface),
        );

        assert!(report.new_toplevel_callback_observed);
        assert!(report.adapter_surface_identity_available);
        assert!(report.adapter_toplevel_identity_registered);
        assert!(report.adapter_surface_id_linked);
        assert!(report.protocol_dispatch_started);
        assert!(report.bridge_input_created);
        assert!(report.pending_admission_intent_created);
        assert_eq!(report.pending_admission_count_before_consume, 1);
        assert_eq!(report.pending_admission_count_after_consume, 0);
        assert!(report.ledger_consume_attempted);
        assert!(report.pending_admission_consumed);
        assert!(report.ledger_admit_surface_invoked);
        assert!(report.ledger_admit_invoked);
        assert!(report.core_register_invoked);
        assert!(report.window_id_allocated);
        assert_eq!(report.core_surface_id, Some(core_surface));
        let core_window = report.core_window_id.expect("pump 必须返回 core WindowId");
        assert_eq!(ledger.surface_mapping(adapter_surface), Some(core_surface));
        assert_eq!(ledger.toplevel_mapping(adapter_toplevel), Some(core_window));
        assert!(state.validate().is_clean());
        assert!(report.blockers.is_empty());
    }

    /// controlled pump 不得把 admission 成功夸大为 render/input/runtime 能力。
    #[test]
    fn controlled_pump_report_keeps_runtime_capabilities_false() {
        let registration = controlled_registration_report();
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();

        let report = pump_controlled_toplevel_admission(
            registration,
            &mut ledger,
            &mut state,
            ControlledToplevelAdmissionPumpInput::phase52x_default(52),
        );

        assert!(!report.handler_state_touched);
        assert!(!report.ledger_bypassed);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
    }
}
