//! Linux-only live callback 到 ledger admission intent 的桥接 seam。
//!
//! 本模块只把 `new_toplevel` callback observation 和 adapter identity registration
//! 合成为 pending admission intent。它不持有 core State，不调用 admission ledger，
//! 也不把 adapter identity 解释成 core window identity。

use std::collections::VecDeque;

use super::{
    linux_toplevel_identity_registration::AdapterToplevelIdentityRegistrationReport,
    surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId},
};

/// Phase 52V pending xdg_toplevel admission intent。
///
/// 该 intent 是 callback/identity 到 ledger owner 的边界消息。它只保存 adapter 层
/// 的 surface/toplevel identity 和 observation 证据；`adapter_toplevel_id` 不是
/// core WindowId，后续 owner 必须再通过 `SurfaceXdgAdmissionLedger` 消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingXdgToplevelAdmission {
    /// 已观察到的 adapter surface identity。
    pub adapter_surface_id: AdapterSurfaceId,
    /// 已注册的 adapter toplevel identity；不是 core window identity。
    pub adapter_toplevel_id: AdapterToplevelId,
    /// 生成 intent 时是否已经观察到 live `new_toplevel` callback。
    pub new_toplevel_callback_observed: bool,
    /// 生成 intent 时 adapter surface identity 是否可用。
    pub adapter_surface_identity_available: bool,
    /// 生成 intent 时 adapter toplevel identity 是否已注册。
    pub adapter_toplevel_identity_registered: bool,
    /// callback observation 序号；只是 proof/report 字段，不代表真实 runtime ready。
    pub source_callback_sequence: Option<u64>,
}

impl PendingXdgToplevelAdmission {
    /// 从已满足的 callback + identity evidence 构造 pending admission intent。
    pub const fn new(
        adapter_surface_id: AdapterSurfaceId,
        adapter_toplevel_id: AdapterToplevelId,
        source_callback_sequence: Option<u64>,
    ) -> Self {
        Self {
            adapter_surface_id,
            adapter_toplevel_id,
            new_toplevel_callback_observed: true,
            adapter_surface_identity_available: true,
            adapter_toplevel_identity_registered: true,
            source_callback_sequence,
        }
    }
}

/// Phase 52V bridge 仍未进入 Route B 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveToplevelAdmissionBridgeBlocker {
    /// 尚未观察到 `new_toplevel` callback，不能生成 pending admission。
    MissingNewToplevelCallbackObservation,
    /// 尚未获得 adapter surface identity，不能链接 toplevel admission。
    MissingAdapterSurfaceIdentity,
    /// 尚未注册 adapter toplevel identity，不能生成 toplevel admission。
    MissingAdapterToplevelIdentityRegistration,
    /// adapter toplevel registration 没有链接到同一个 adapter surface。
    AdapterSurfaceIdentityMismatch,
    /// 当前 callback/handler 层没有 ledger owner，不能在这里 consume intent。
    MissingLedgerOwner,
    /// 当前 callback/handler 层没有 core State owner，不能在这里注册 core window。
    MissingStateOwner,
}

/// Phase 52V bridge report 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveToplevelAdmissionBridgeOperation {
    /// 读取 callback observation evidence。
    ReadCallbackObservation,
    /// 读取 adapter identity registration evidence。
    ReadAdapterIdentityRegistration,
    /// 创建 pending admission intent。
    CreatePendingAdmissionIntent,
    /// 将 pending intent 放入 bridge queue。
    QueuePendingAdmissionIntent,
    /// 停在 owner 消费边界。
    StopBeforeLedgerConsumption,
}

/// Phase 52V bridge 的纯数据输入。
///
/// 输入可以来自 controlled registration report，也可以由后续真实 callback owner 构造。
/// 这里故意不携带 ledger 或 core State；intent queue 是 handler 与 owner 的边界 seam。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveToplevelAdmissionBridgeInput {
    /// callback 是否已经被 handler 观察到。
    pub new_toplevel_callback_observed: bool,
    /// adapter surface identity 是否可用。
    pub adapter_surface_identity_available: bool,
    /// adapter toplevel identity 是否已注册。
    pub adapter_toplevel_identity_registered: bool,
    /// toplevel registration 是否链接到同一 adapter surface。
    pub adapter_surface_id_linked: bool,
    /// adapter surface identity。
    pub adapter_surface_id: Option<AdapterSurfaceId>,
    /// adapter toplevel identity。
    pub adapter_toplevel_id: Option<AdapterToplevelId>,
    /// callback observation 序号；只是 proof/report 字段。
    pub new_toplevel_callback_sequence: Option<u64>,
}

impl LiveToplevelAdmissionBridgeInput {
    /// 构造一个 callback 与 identity 均已满足的 bridge 输入。
    pub const fn from_registered_identity(
        adapter_surface_id: AdapterSurfaceId,
        adapter_toplevel_id: AdapterToplevelId,
        new_toplevel_callback_sequence: u64,
    ) -> Self {
        Self {
            new_toplevel_callback_observed: true,
            adapter_surface_identity_available: true,
            adapter_toplevel_identity_registered: true,
            adapter_surface_id_linked: true,
            adapter_surface_id: Some(adapter_surface_id),
            adapter_toplevel_id: Some(adapter_toplevel_id),
            new_toplevel_callback_sequence: Some(new_toplevel_callback_sequence),
        }
    }
}

impl From<&AdapterToplevelIdentityRegistrationReport> for LiveToplevelAdmissionBridgeInput {
    fn from(report: &AdapterToplevelIdentityRegistrationReport) -> Self {
        Self {
            new_toplevel_callback_observed: report.new_toplevel_callback_observed,
            adapter_surface_identity_available: report.adapter_surface_identity_available,
            adapter_toplevel_identity_registered: report.adapter_toplevel_identity_registered,
            adapter_surface_id_linked: report.adapter_surface_id_linked,
            adapter_surface_id: report
                .adapter_surface_identity_available
                .then_some(report.adapter_surface_id),
            adapter_toplevel_id: report
                .adapter_toplevel_identity_registered
                .then_some(report.adapter_toplevel_id),
            new_toplevel_callback_sequence: Some(report.new_toplevel_callback_sequence),
        }
    }
}

/// Pending admission intent 的 bridge queue。
///
/// queue 是 callback handler 与 ledger owner 之间的边界 seam。handler 只追加纯数据
/// intent；拥有 ledger 与 core State 的 owner 后续再决定是否 consume。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ToplevelAdmissionBridgeQueue {
    pending: VecDeque<PendingXdgToplevelAdmission>,
}

impl ToplevelAdmissionBridgeQueue {
    /// 创建空 pending admission queue。
    pub fn new() -> Self {
        Self::default()
    }

    /// 将 pending intent 追加到 queue 末尾。
    pub fn push(&mut self, admission: PendingXdgToplevelAdmission) {
        self.pending.push_back(admission);
    }

    /// 返回当前 pending intent 数量。
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 查看下一条待 owner 消费的 pending intent。
    pub fn front(&self) -> Option<&PendingXdgToplevelAdmission> {
        self.pending.front()
    }

    /// 取出下一条待 owner 消费的 pending intent。
    ///
    /// 只有显式 owner consumer 可以调用该方法；callback handler 仍只负责追加
    /// pending intent，不在 handler 内消费 ledger 或修改 core State。
    pub fn pop_front(&mut self) -> Option<PendingXdgToplevelAdmission> {
        self.pending.pop_front()
    }
}

/// Phase 52V live callback admission bridge 的能力报告。
///
/// 成功生成 pending intent 只证明 callback -> adapter identity -> queue seam 已接通。
/// ledger/core 字段保持 false，因为本阶段没有安全 owner 同时持有 ledger 与 core State。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveToplevelAdmissionBridgeReport {
    /// 是否观察到 `new_toplevel` callback。
    pub new_toplevel_callback_observed: bool,
    /// adapter surface identity 是否可用。
    pub adapter_surface_identity_available: bool,
    /// adapter toplevel identity 是否已注册。
    pub adapter_toplevel_identity_registered: bool,
    /// 是否创建 pending admission intent。
    pub pending_admission_intent_created: bool,
    /// pending admission queue 中的 intent 数量。
    pub pending_admission_count: usize,
    /// 下一条 pending admission intent。
    pub pending_admission: Option<PendingXdgToplevelAdmission>,
    /// 当前层是否拥有 ledger。
    pub ledger_owner_available: bool,
    /// 当前层是否尝试消费 pending intent。
    pub ledger_consume_attempted: bool,
    /// 当前层是否调用 ledger admit。
    pub ledger_admit_invoked: bool,
    /// 当前层是否触发 core register。
    pub core_register_invoked: bool,
    /// 当前层是否分配 core window identity。
    pub window_id_allocated: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 是否已有真实 compositor runtime。
    pub real_compositor_runtime_available: bool,
    /// 是否已有真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// 本报告执行过的纯数据操作。
    pub operations: Vec<LiveToplevelAdmissionBridgeOperation>,
    /// 阻止进入 Route B 的剩余 blocker。
    pub blockers: Vec<LiveToplevelAdmissionBridgeBlocker>,
}

/// 从 callback/identity evidence 生成 Phase 52V B-lite bridge report。
///
/// 本函数不直接修改 core State。它只生成 pending intent 并放入 queue，因为 handler
/// 层不能长期持有 core State，也不能直接修改 workspace、slot 或 stack。
pub fn live_toplevel_admission_bridge_report(
    input: LiveToplevelAdmissionBridgeInput,
) -> LiveToplevelAdmissionBridgeReport {
    let mut operations = vec![
        LiveToplevelAdmissionBridgeOperation::ReadCallbackObservation,
        LiveToplevelAdmissionBridgeOperation::ReadAdapterIdentityRegistration,
    ];
    let mut blockers = Vec::new();

    if !input.new_toplevel_callback_observed {
        blockers.push(LiveToplevelAdmissionBridgeBlocker::MissingNewToplevelCallbackObservation);
    }
    if !input.adapter_surface_identity_available || input.adapter_surface_id.is_none() {
        blockers.push(LiveToplevelAdmissionBridgeBlocker::MissingAdapterSurfaceIdentity);
    }
    if !input.adapter_toplevel_identity_registered || input.adapter_toplevel_id.is_none() {
        blockers
            .push(LiveToplevelAdmissionBridgeBlocker::MissingAdapterToplevelIdentityRegistration);
    }
    if !input.adapter_surface_id_linked {
        blockers.push(LiveToplevelAdmissionBridgeBlocker::AdapterSurfaceIdentityMismatch);
    }

    let mut queue = ToplevelAdmissionBridgeQueue::new();
    if blockers.is_empty() {
        let admission = PendingXdgToplevelAdmission::new(
            input
                .adapter_surface_id
                .expect("adapter surface identity 已由 blocker 检查"),
            input
                .adapter_toplevel_id
                .expect("adapter toplevel identity 已由 blocker 检查"),
            input.new_toplevel_callback_sequence,
        );
        queue.push(admission);
        operations.push(LiveToplevelAdmissionBridgeOperation::CreatePendingAdmissionIntent);
        operations.push(LiveToplevelAdmissionBridgeOperation::QueuePendingAdmissionIntent);

        blockers.push(LiveToplevelAdmissionBridgeBlocker::MissingLedgerOwner);
        blockers.push(LiveToplevelAdmissionBridgeBlocker::MissingStateOwner);
    }
    operations.push(LiveToplevelAdmissionBridgeOperation::StopBeforeLedgerConsumption);

    let pending_admission = queue.front().copied();

    LiveToplevelAdmissionBridgeReport {
        new_toplevel_callback_observed: input.new_toplevel_callback_observed,
        adapter_surface_identity_available: input.adapter_surface_identity_available,
        adapter_toplevel_identity_registered: input.adapter_toplevel_identity_registered,
        pending_admission_intent_created: pending_admission.is_some(),
        pending_admission_count: queue.pending_count(),
        pending_admission,
        ledger_owner_available: false,
        ledger_consume_attempted: false,
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
    use super::{
        LiveToplevelAdmissionBridgeBlocker, LiveToplevelAdmissionBridgeInput,
        PendingXdgToplevelAdmission, ToplevelAdmissionBridgeQueue,
        live_toplevel_admission_bridge_report,
    };
    use crate::smithay_backend::surface_xdg_admission::{
        AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId,
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 surface identity 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(
            ProtocolObjectId::new(value).expect("测试 toplevel identity 必须非零"),
        )
    }

    /// callback observation 与 adapter identity 都可用时，bridge 必须只生成 pending intent。
    #[test]
    fn callback_and_identity_create_pending_admission_intent() {
        let adapter_surface = surface(101);
        let adapter_toplevel = toplevel(202);
        let input = LiveToplevelAdmissionBridgeInput::from_registered_identity(
            adapter_surface,
            adapter_toplevel,
            7,
        );

        let report = live_toplevel_admission_bridge_report(input);

        assert!(report.new_toplevel_callback_observed);
        assert!(report.adapter_surface_identity_available);
        assert!(report.adapter_toplevel_identity_registered);
        assert!(report.pending_admission_intent_created);
        assert_eq!(report.pending_admission_count, 1);
        let pending = report
            .pending_admission
            .expect("B-lite bridge 必须生成 pending intent");
        assert_eq!(pending.adapter_surface_id, adapter_surface);
        assert_eq!(pending.adapter_toplevel_id, adapter_toplevel);
        assert_eq!(pending.source_callback_sequence, Some(7));
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
    }

    /// pending intent 只携带 adapter toplevel identity，不代表 core window identity。
    #[test]
    fn pending_intent_is_not_core_window_identity() {
        let adapter_surface = surface(11);
        let adapter_toplevel = toplevel(22);
        let pending = PendingXdgToplevelAdmission::new(adapter_surface, adapter_toplevel, Some(1));

        assert_eq!(pending.adapter_toplevel_id, adapter_toplevel);
        assert!(pending.adapter_toplevel_identity_registered);
        assert!(pending.new_toplevel_callback_observed);
    }

    /// B-lite 路径必须停止在 queue seam，不消费 ledger 或 core。
    #[test]
    fn b_lite_path_does_not_consume_ledger_or_core() {
        let input = LiveToplevelAdmissionBridgeInput::from_registered_identity(
            surface(31),
            toplevel(41),
            2,
        );

        let report = live_toplevel_admission_bridge_report(input);

        assert!(!report.ledger_owner_available);
        assert!(!report.ledger_consume_attempted);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(
            report
                .blockers
                .contains(&LiveToplevelAdmissionBridgeBlocker::MissingLedgerOwner)
        );
        assert!(
            report
                .blockers
                .contains(&LiveToplevelAdmissionBridgeBlocker::MissingStateOwner)
        );
    }

    /// 缺少 callback observation 时不能创建 pending admission。
    #[test]
    fn missing_callback_does_not_create_pending_intent() {
        let input = LiveToplevelAdmissionBridgeInput {
            new_toplevel_callback_observed: false,
            adapter_surface_identity_available: true,
            adapter_toplevel_identity_registered: true,
            adapter_surface_id_linked: true,
            adapter_surface_id: Some(surface(51)),
            adapter_toplevel_id: Some(toplevel(61)),
            new_toplevel_callback_sequence: None,
        };

        let report = live_toplevel_admission_bridge_report(input);

        assert!(!report.pending_admission_intent_created);
        assert_eq!(report.pending_admission_count, 0);
        assert!(
            report.blockers.contains(
                &LiveToplevelAdmissionBridgeBlocker::MissingNewToplevelCallbackObservation
            )
        );
    }

    /// bridge queue 保持 owner 后续消费所需的 pending intent 顺序。
    #[test]
    fn bridge_queue_preserves_pending_intent_order() {
        let first = PendingXdgToplevelAdmission::new(surface(71), toplevel(81), Some(1));
        let second = PendingXdgToplevelAdmission::new(surface(72), toplevel(82), Some(2));
        let mut queue = ToplevelAdmissionBridgeQueue::new();

        queue.push(first);
        queue.push(second);

        assert_eq!(queue.pending_count(), 2);
        assert_eq!(queue.front(), Some(&first));
        assert_eq!(queue.pop_front(), Some(first));
        assert_eq!(queue.front(), Some(&second));
    }
}
