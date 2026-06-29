//! Linux-only pending admission intent 的 owner 消费 seam。
//!
//! 本模块证明只有同时拥有 pending queue、admission ledger 与 core State 的 owner
//! 才能消费 Phase 52V intent。callback handler 仍不持有 State，也不直接修改 core。

use crate::core::{
    client::ClientId,
    state::State,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::WindowId,
};

use super::{
    linux_toplevel_admission_bridge::{PendingXdgToplevelAdmission, ToplevelAdmissionBridgeQueue},
    surface_xdg_admission::{
        SurfaceAdmissionIntent, SurfaceXdgAdmissionError, SurfaceXdgAdmissionLedger,
        SurfaceXdgAdmissionReport, XdgToplevelAdmissionIntent,
    },
};

/// Pending admission consumer owner 的输入。
///
/// `core_surface_id`、metadata 与 kind 由 owner 层提供，避免 callback handler
/// 伪造 core identity。`WindowId` 仍由 `SurfaceXdgAdmissionLedger::admit_toplevel`
/// 内部的既有 core seam 分配。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingToplevelAdmissionConsumerInput {
    /// owner 准备交给 ledger 的 core surface identity。
    pub core_surface_id: SurfaceId,
    /// 可选 core client 归属。
    pub client: Option<ClientId>,
    /// surface role；本 phase 只消费 xdg toplevel intent。
    pub role: SurfaceRole,
    /// 进入 core window registry 的 title metadata。
    pub title: String,
    /// 可选 application id metadata。
    pub app_id: Option<String>,
    /// core window kind；测试 proof 使用 mock kind，不代表 real runtime。
    pub kind: WindowKind,
}

impl PendingToplevelAdmissionConsumerInput {
    /// 构造 Phase 52W 默认 proof 输入。
    pub fn phase52w_default(core_surface_id: SurfaceId) -> Self {
        Self {
            core_surface_id,
            client: None,
            role: SurfaceRole::XdgToplevel,
            title: "Phase 52W pending toplevel admission".to_string(),
            app_id: Some("sky-mirror-phase52w".to_string()),
            kind: WindowKind::Mock,
        }
    }
}

/// Pending admission consumer owner 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingToplevelAdmissionConsumerBlocker {
    /// queue 中没有 pending admission intent。
    MissingPendingAdmission,
    /// surface admission 被 ledger/core seam 拒绝。
    SurfaceAdmissionRejected(SurfaceXdgAdmissionError),
    /// toplevel admission 被 ledger/core seam 拒绝。
    ToplevelAdmissionRejected(SurfaceXdgAdmissionError),
    /// ledger report 没有返回 core window identity。
    MissingCoreWindow,
}

/// Pending admission consumer owner 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingToplevelAdmissionConsumerOperation {
    /// 读取 pending intent queue。
    ReadPendingQueue,
    /// 准备 surface admission intent。
    BuildSurfaceAdmissionIntent,
    /// 调用 `SurfaceXdgAdmissionLedger::admit_surface`。
    AdmitSurfaceThroughLedger,
    /// 准备 xdg toplevel admission intent。
    BuildToplevelAdmissionIntent,
    /// 调用 `SurfaceXdgAdmissionLedger::admit_toplevel`。
    AdmitToplevelThroughLedger,
    /// 从 queue 中移除已成功消费的 pending intent。
    RemoveConsumedPendingIntent,
    /// 生成保守 capability report。
    BuildReport,
}

/// Pending admission consumer owner 的能力报告。
///
/// 成功报告证明 pending intent 已由 owner 层通过 ledger 消费，并经既有
/// `SurfaceXdgAdmissionLedger` 产生 core `WindowId`。render/input/real runtime 仍为 false。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingToplevelAdmissionConsumerReport {
    /// owner 是否尝试消费 queue。
    pub ledger_consume_attempted: bool,
    /// 是否从 queue 中成功取出并消费 pending intent。
    pub pending_admission_consumed: bool,
    /// 被消费的 pending intent。
    pub pending_admission: Option<PendingXdgToplevelAdmission>,
    /// 消费后 queue 中剩余 pending intent 数量。
    pub pending_admission_count_after: usize,
    /// 是否调用 ledger admit_surface。
    pub ledger_admit_surface_invoked: bool,
    /// 是否调用 ledger admit_toplevel。
    pub ledger_admit_invoked: bool,
    /// surface admission 的 ledger report。
    pub surface_report: Option<SurfaceXdgAdmissionReport>,
    /// toplevel admission 的 ledger report。
    pub toplevel_report: Option<SurfaceXdgAdmissionReport>,
    /// core surface identity。
    pub core_surface_id: Option<SurfaceId>,
    /// core window identity；由 ledger 内部 core seam 返回。
    pub core_window_id: Option<WindowId>,
    /// 是否触发既有 core register seam。
    pub core_register_invoked: bool,
    /// 是否分配 core window identity。
    pub window_id_allocated: bool,
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
    /// 本次 owner 消费执行过的操作。
    pub operations: Vec<PendingToplevelAdmissionConsumerOperation>,
    /// 失败或未完成原因。
    pub blockers: Vec<PendingToplevelAdmissionConsumerBlocker>,
}

impl PendingToplevelAdmissionConsumerReport {
    fn blocked(blocker: PendingToplevelAdmissionConsumerBlocker, pending_count: usize) -> Self {
        Self {
            ledger_consume_attempted: true,
            pending_admission_consumed: false,
            pending_admission: None,
            pending_admission_count_after: pending_count,
            ledger_admit_surface_invoked: false,
            ledger_admit_invoked: false,
            surface_report: None,
            toplevel_report: None,
            core_surface_id: None,
            core_window_id: None,
            core_register_invoked: false,
            window_id_allocated: false,
            handler_state_touched: false,
            ledger_bypassed: false,
            render_support: false,
            input_support: false,
            real_compositor_runtime_available: false,
            real_xdg_shell_runtime_available: false,
            operations: vec![
                PendingToplevelAdmissionConsumerOperation::ReadPendingQueue,
                PendingToplevelAdmissionConsumerOperation::BuildReport,
            ],
            blockers: vec![blocker],
        }
    }
}

/// 消费下一条 pending toplevel admission intent。
///
/// 该函数是 Phase 52W 的 owner seam：调用方必须同时拥有 queue、ledger 与 `State`。
/// 这里不让 handler 长期持有 `&mut State`，也不直接修改 workspace/slot/stack；
/// 所有 core mutation 都经由 `SurfaceXdgAdmissionLedger` 的既有 admission API。
pub fn consume_pending_toplevel_admission(
    queue: &mut ToplevelAdmissionBridgeQueue,
    ledger: &mut SurfaceXdgAdmissionLedger,
    state: &mut State,
    input: PendingToplevelAdmissionConsumerInput,
) -> PendingToplevelAdmissionConsumerReport {
    let mut operations = vec![PendingToplevelAdmissionConsumerOperation::ReadPendingQueue];
    let Some(pending) = queue.front().copied() else {
        return PendingToplevelAdmissionConsumerReport::blocked(
            PendingToplevelAdmissionConsumerBlocker::MissingPendingAdmission,
            queue.pending_count(),
        );
    };

    operations.push(PendingToplevelAdmissionConsumerOperation::BuildSurfaceAdmissionIntent);
    let surface_intent = SurfaceAdmissionIntent {
        adapter_surface: pending.adapter_surface_id,
        core_surface: input.core_surface_id,
        client: input.client,
        role: input.role,
    };
    operations.push(PendingToplevelAdmissionConsumerOperation::AdmitSurfaceThroughLedger);
    let surface_report = match ledger.admit_surface(state, surface_intent) {
        Ok(report) => report,
        Err(source) => {
            operations.push(PendingToplevelAdmissionConsumerOperation::BuildReport);
            return PendingToplevelAdmissionConsumerReport {
                ledger_consume_attempted: true,
                pending_admission_consumed: false,
                pending_admission: Some(pending),
                pending_admission_count_after: queue.pending_count(),
                ledger_admit_surface_invoked: true,
                ledger_admit_invoked: false,
                surface_report: None,
                toplevel_report: None,
                core_surface_id: Some(input.core_surface_id),
                core_window_id: None,
                core_register_invoked: false,
                window_id_allocated: false,
                handler_state_touched: false,
                ledger_bypassed: false,
                render_support: false,
                input_support: false,
                real_compositor_runtime_available: false,
                real_xdg_shell_runtime_available: false,
                operations,
                blockers: vec![
                    PendingToplevelAdmissionConsumerBlocker::SurfaceAdmissionRejected(source),
                ],
            };
        }
    };

    operations.push(PendingToplevelAdmissionConsumerOperation::BuildToplevelAdmissionIntent);
    let toplevel_intent = XdgToplevelAdmissionIntent {
        adapter_toplevel: pending.adapter_toplevel_id,
        adapter_surface: pending.adapter_surface_id,
        title: input.title,
        app_id: input.app_id,
        kind: input.kind,
    };
    operations.push(PendingToplevelAdmissionConsumerOperation::AdmitToplevelThroughLedger);
    let toplevel_report = match ledger.admit_toplevel(state, toplevel_intent) {
        Ok(report) => report,
        Err(source) => {
            operations.push(PendingToplevelAdmissionConsumerOperation::BuildReport);
            return PendingToplevelAdmissionConsumerReport {
                ledger_consume_attempted: true,
                pending_admission_consumed: false,
                pending_admission: Some(pending),
                pending_admission_count_after: queue.pending_count(),
                ledger_admit_surface_invoked: true,
                ledger_admit_invoked: true,
                surface_report: Some(surface_report),
                toplevel_report: None,
                core_surface_id: Some(input.core_surface_id),
                core_window_id: None,
                core_register_invoked: false,
                window_id_allocated: false,
                handler_state_touched: false,
                ledger_bypassed: false,
                render_support: false,
                input_support: false,
                real_compositor_runtime_available: false,
                real_xdg_shell_runtime_available: false,
                operations,
                blockers: vec![
                    PendingToplevelAdmissionConsumerBlocker::ToplevelAdmissionRejected(source),
                ],
            };
        }
    };

    let Some(core_window_id) = toplevel_report.core_window() else {
        operations.push(PendingToplevelAdmissionConsumerOperation::BuildReport);
        return PendingToplevelAdmissionConsumerReport {
            ledger_consume_attempted: true,
            pending_admission_consumed: false,
            pending_admission: Some(pending),
            pending_admission_count_after: queue.pending_count(),
            ledger_admit_surface_invoked: true,
            ledger_admit_invoked: true,
            surface_report: Some(surface_report),
            toplevel_report: Some(toplevel_report),
            core_surface_id: Some(input.core_surface_id),
            core_window_id: None,
            core_register_invoked: false,
            window_id_allocated: false,
            handler_state_touched: false,
            ledger_bypassed: false,
            render_support: false,
            input_support: false,
            real_compositor_runtime_available: false,
            real_xdg_shell_runtime_available: false,
            operations,
            blockers: vec![PendingToplevelAdmissionConsumerBlocker::MissingCoreWindow],
        };
    };

    let consumed = queue.pop_front();
    let consumed =
        consumed.expect("front confirmed pending intent exists before successful admission");
    operations.push(PendingToplevelAdmissionConsumerOperation::RemoveConsumedPendingIntent);
    operations.push(PendingToplevelAdmissionConsumerOperation::BuildReport);

    PendingToplevelAdmissionConsumerReport {
        ledger_consume_attempted: true,
        pending_admission_consumed: true,
        pending_admission: Some(consumed),
        pending_admission_count_after: queue.pending_count(),
        ledger_admit_surface_invoked: true,
        ledger_admit_invoked: true,
        surface_report: Some(surface_report),
        toplevel_report: Some(toplevel_report),
        core_surface_id: Some(input.core_surface_id),
        core_window_id: Some(core_window_id),
        core_register_invoked: true,
        window_id_allocated: true,
        handler_state_touched: false,
        ledger_bypassed: false,
        render_support: false,
        input_support: false,
        real_compositor_runtime_available: false,
        real_xdg_shell_runtime_available: false,
        operations,
        blockers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::core::state::State;
    use crate::smithay_backend::{
        linux_toplevel_admission_bridge::{
            PendingXdgToplevelAdmission, ToplevelAdmissionBridgeQueue,
        },
        surface_xdg_admission::{
            AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId, SurfaceXdgAdmissionLedger,
        },
    };

    use super::{
        PendingToplevelAdmissionConsumerBlocker, PendingToplevelAdmissionConsumerInput,
        consume_pending_toplevel_admission,
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 surface identity 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(
            ProtocolObjectId::new(value).expect("测试 toplevel identity 必须非零"),
        )
    }

    /// owner 同时持有 queue、ledger、State 时，可以消费 pending intent 并产生 core WindowId。
    #[test]
    fn owner_consumes_pending_intent_through_ledger() {
        let pending = PendingXdgToplevelAdmission::new(surface(301), toplevel(401), Some(9));
        let mut queue = ToplevelAdmissionBridgeQueue::new();
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        queue.push(pending);

        let report = consume_pending_toplevel_admission(
            &mut queue,
            &mut ledger,
            &mut state,
            PendingToplevelAdmissionConsumerInput::phase52w_default(77),
        );

        assert!(report.ledger_consume_attempted);
        assert!(report.pending_admission_consumed);
        assert_eq!(report.pending_admission, Some(pending));
        assert_eq!(report.pending_admission_count_after, 0);
        assert!(report.ledger_admit_surface_invoked);
        assert!(report.ledger_admit_invoked);
        assert!(report.core_register_invoked);
        assert!(report.window_id_allocated);
        assert_eq!(report.core_surface_id, Some(77));
        let core_window = report
            .core_window_id
            .expect("ledger 必须返回 core WindowId");
        assert_eq!(ledger.surface_mapping(pending.adapter_surface_id), Some(77));
        assert_eq!(
            ledger.toplevel_mapping(pending.adapter_toplevel_id),
            Some(core_window)
        );
        assert!(report.blockers.is_empty());
    }

    /// consumer owner 不得夸大 render/input/real runtime，也不得声明 handler 触碰 State。
    #[test]
    fn consumer_report_keeps_runtime_capabilities_false() {
        let mut queue = ToplevelAdmissionBridgeQueue::new();
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        queue.push(PendingXdgToplevelAdmission::new(
            surface(302),
            toplevel(402),
            Some(10),
        ));

        let report = consume_pending_toplevel_admission(
            &mut queue,
            &mut ledger,
            &mut state,
            PendingToplevelAdmissionConsumerInput::phase52w_default(78),
        );

        assert!(!report.handler_state_touched);
        assert!(!report.ledger_bypassed);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
    }

    /// 空 queue 时 consumer 不调用 ledger，也不修改 core。
    #[test]
    fn empty_queue_does_not_invoke_ledger() {
        let mut queue = ToplevelAdmissionBridgeQueue::new();
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();

        let report = consume_pending_toplevel_admission(
            &mut queue,
            &mut ledger,
            &mut state,
            PendingToplevelAdmissionConsumerInput::phase52w_default(79),
        );

        assert!(report.ledger_consume_attempted);
        assert!(!report.pending_admission_consumed);
        assert!(!report.ledger_admit_surface_invoked);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
        assert!(
            report
                .blockers
                .contains(&PendingToplevelAdmissionConsumerBlocker::MissingPendingAdmission)
        );
    }
}
