//! Linux-only runtime-owned pending admission queue seam.
//!
//! The owner in this module is the first runtime-shaped holder of both the
//! pending toplevel admission queue and the ledger. It can drain one queued
//! admission intent per tick through the Phase 52W consumer, but it does not
//! create protocol clients, dispatch server requests, or mutate handler state.

use crate::core::{
    client::ClientId,
    state::State,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::WindowId,
};

use super::{
    linux_toplevel_admission_bridge::{PendingXdgToplevelAdmission, ToplevelAdmissionBridgeQueue},
    linux_toplevel_admission_consumer::{
        PendingToplevelAdmissionConsumerBlocker, PendingToplevelAdmissionConsumerInput,
        PendingToplevelAdmissionConsumerReport, consume_pending_toplevel_admission,
    },
    surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId, SurfaceXdgAdmissionLedger},
};

/// Runtime admission queue owner 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeToplevelAdmissionQueueOperation {
    /// 初始化 runtime owner。
    InitializeOwner,
    /// 将 pending admission intent 放入 runtime-owned queue。
    EnqueuePendingAdmission,
    /// 读取 runtime-owned queue。
    ReadRuntimeQueue,
    /// 构造 pending admission consumer input。
    BuildConsumerInput,
    /// 调用 Phase 52W consumer。
    DrainConsumer,
    /// 成功消费后推进下一次 core surface identity。
    AdvanceCoreSurfaceId,
    /// 生成保守 capability report。
    BuildReport,
}

/// Runtime admission queue owner 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeToplevelAdmissionQueueBlocker {
    /// queue 中没有 pending admission intent。
    MissingPendingAdmission,
    /// Phase 52W consumer 返回了 blocker。
    ConsumerBlocked(Vec<PendingToplevelAdmissionConsumerBlocker>),
}

/// Runtime tick 提供给 drain 的 metadata。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeToplevelAdmissionDrainTick {
    /// runtime tick 序号；仅用于报告和稳定 metadata。
    pub tick_index: u64,
    /// 可选 core client 归属。
    pub client: Option<ClientId>,
    /// 进入 core window registry 的 title metadata。
    pub title: String,
    /// 可选 application id metadata。
    pub app_id: Option<String>,
    /// core window kind；当前 proof 仍使用 mock kind。
    pub kind: WindowKind,
}

impl RuntimeToplevelAdmissionDrainTick {
    /// 构造 Phase 52Y 默认 tick metadata。
    pub fn phase52y_default(tick_index: u64) -> Self {
        Self {
            tick_index,
            client: None,
            title: format!("Phase 52Y runtime admission tick {tick_index}"),
            app_id: Some("sky-mirror-phase52y".to_string()),
            kind: WindowKind::Mock,
        }
    }
}

/// Runtime queue enqueue 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeToplevelAdmissionEnqueueReport {
    /// runtime 是否拥有 queue。
    pub runtime_queue_owned: bool,
    /// runtime 是否拥有 ledger。
    pub runtime_ledger_owned: bool,
    /// 入队前 pending 数量。
    pub pending_admission_count_before: usize,
    /// 入队后 pending 数量。
    pub pending_admission_count_after: usize,
    /// 是否成功入队。
    pub pending_admission_enqueued: bool,
    /// 入队的 pending intent。
    pub pending_admission: PendingXdgToplevelAdmission,
    /// 执行过的操作。
    pub operations: Vec<RuntimeToplevelAdmissionQueueOperation>,
}

/// Runtime queue drain 的能力报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeToplevelAdmissionDrainReport {
    /// runtime tick 序号。
    pub tick_index: u64,
    /// runtime 是否拥有 queue。
    pub runtime_queue_owned: bool,
    /// runtime 是否拥有 ledger。
    pub runtime_ledger_owned: bool,
    /// 本轮是否尝试 drain。
    pub drain_invoked: bool,
    /// drain 前 pending 数量。
    pub pending_admission_count_before: usize,
    /// drain 后 pending 数量。
    pub pending_admission_count_after: usize,
    /// Phase 52W consumer report。
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
    /// 本轮 drain 使用的 core surface identity。
    pub core_surface_id: Option<SurfaceId>,
    /// ledger/core admission 返回的 core window identity。
    pub core_window_id: Option<WindowId>,
    /// drain 后下一次 core surface identity。
    pub next_core_surface_id_after: SurfaceId,
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
    /// 执行过的操作。
    pub operations: Vec<RuntimeToplevelAdmissionQueueOperation>,
    /// 失败或未完成原因。
    pub blockers: Vec<RuntimeToplevelAdmissionQueueBlocker>,
}

/// Runtime-owned pending toplevel admission queue owner。
///
/// 该 owner 同时持有 queue 与 ledger。它可以被后续 nested runtime coordinator tick
/// 调用，但本 phase 不修改 coordinator 的 accept/dispatch path。
#[derive(Debug)]
pub struct RuntimeToplevelAdmissionQueueOwner {
    queue: ToplevelAdmissionBridgeQueue,
    ledger: SurfaceXdgAdmissionLedger,
    next_core_surface_id: SurfaceId,
}

impl RuntimeToplevelAdmissionQueueOwner {
    /// 创建 runtime admission queue owner。
    pub fn new(next_core_surface_id: SurfaceId) -> Self {
        Self {
            queue: ToplevelAdmissionBridgeQueue::new(),
            ledger: SurfaceXdgAdmissionLedger::new(),
            next_core_surface_id,
        }
    }

    /// 返回当前 pending admission 数量。
    pub fn pending_count(&self) -> usize {
        self.queue.pending_count()
    }

    /// 返回下一次将使用的 core surface identity。
    pub fn next_core_surface_id(&self) -> SurfaceId {
        self.next_core_surface_id
    }

    /// 查询 adapter surface 到 core surface 的 ledger mapping。
    pub fn surface_mapping(&self, adapter_surface: AdapterSurfaceId) -> Option<SurfaceId> {
        self.ledger.surface_mapping(adapter_surface)
    }

    /// 查询 adapter toplevel 到 core window 的 ledger mapping。
    pub fn toplevel_mapping(&self, adapter_toplevel: AdapterToplevelId) -> Option<WindowId> {
        self.ledger.toplevel_mapping(adapter_toplevel)
    }

    /// 将 pending toplevel admission intent 放入 runtime-owned queue。
    pub fn enqueue_pending_toplevel_admission(
        &mut self,
        pending: PendingXdgToplevelAdmission,
    ) -> RuntimeToplevelAdmissionEnqueueReport {
        let pending_admission_count_before = self.queue.pending_count();
        self.queue.push(pending);

        RuntimeToplevelAdmissionEnqueueReport {
            runtime_queue_owned: true,
            runtime_ledger_owned: true,
            pending_admission_count_before,
            pending_admission_count_after: self.queue.pending_count(),
            pending_admission_enqueued: true,
            pending_admission: pending,
            operations: vec![
                RuntimeToplevelAdmissionQueueOperation::EnqueuePendingAdmission,
                RuntimeToplevelAdmissionQueueOperation::BuildReport,
            ],
        }
    }

    /// 从 runtime-owned queue 中 drain 一条 pending toplevel admission intent。
    pub fn drain_pending_toplevel_admission_once(
        &mut self,
        state: &mut State,
        tick: RuntimeToplevelAdmissionDrainTick,
    ) -> RuntimeToplevelAdmissionDrainReport {
        let pending_admission_count_before = self.queue.pending_count();
        let mut operations = vec![RuntimeToplevelAdmissionQueueOperation::ReadRuntimeQueue];

        if pending_admission_count_before == 0 {
            operations.push(RuntimeToplevelAdmissionQueueOperation::BuildReport);
            return RuntimeToplevelAdmissionDrainReport {
                tick_index: tick.tick_index,
                runtime_queue_owned: true,
                runtime_ledger_owned: true,
                drain_invoked: true,
                pending_admission_count_before,
                pending_admission_count_after: self.queue.pending_count(),
                consumer_report: None,
                ledger_consume_attempted: false,
                pending_admission_consumed: false,
                ledger_admit_surface_invoked: false,
                ledger_admit_invoked: false,
                core_register_invoked: false,
                window_id_allocated: false,
                core_surface_id: None,
                core_window_id: None,
                next_core_surface_id_after: self.next_core_surface_id,
                handler_state_touched: false,
                ledger_bypassed: false,
                render_support: false,
                input_support: false,
                real_compositor_runtime_available: false,
                real_xdg_shell_runtime_available: false,
                operations,
                blockers: vec![RuntimeToplevelAdmissionQueueBlocker::MissingPendingAdmission],
            };
        }

        let core_surface_id = self.next_core_surface_id;
        operations.push(RuntimeToplevelAdmissionQueueOperation::BuildConsumerInput);
        let consumer_input = PendingToplevelAdmissionConsumerInput {
            core_surface_id,
            client: tick.client,
            role: SurfaceRole::XdgToplevel,
            title: tick.title,
            app_id: tick.app_id,
            kind: tick.kind,
        };

        operations.push(RuntimeToplevelAdmissionQueueOperation::DrainConsumer);
        let consumer_report = consume_pending_toplevel_admission(
            &mut self.queue,
            &mut self.ledger,
            state,
            consumer_input,
        );
        let mut blockers = Vec::new();
        if !consumer_report.blockers.is_empty() {
            blockers.push(RuntimeToplevelAdmissionQueueBlocker::ConsumerBlocked(
                consumer_report.blockers.clone(),
            ));
        }
        if consumer_report.pending_admission_consumed {
            self.next_core_surface_id = self.next_core_surface_id.saturating_add(1);
            operations.push(RuntimeToplevelAdmissionQueueOperation::AdvanceCoreSurfaceId);
        }
        operations.push(RuntimeToplevelAdmissionQueueOperation::BuildReport);

        RuntimeToplevelAdmissionDrainReport {
            tick_index: tick.tick_index,
            runtime_queue_owned: true,
            runtime_ledger_owned: true,
            drain_invoked: true,
            pending_admission_count_before,
            pending_admission_count_after: self.queue.pending_count(),
            consumer_report: Some(consumer_report.clone()),
            ledger_consume_attempted: consumer_report.ledger_consume_attempted,
            pending_admission_consumed: consumer_report.pending_admission_consumed,
            ledger_admit_surface_invoked: consumer_report.ledger_admit_surface_invoked,
            ledger_admit_invoked: consumer_report.ledger_admit_invoked,
            core_register_invoked: consumer_report.core_register_invoked,
            window_id_allocated: consumer_report.window_id_allocated,
            core_surface_id: consumer_report.core_surface_id,
            core_window_id: consumer_report.core_window_id,
            next_core_surface_id_after: self.next_core_surface_id,
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
}

#[cfg(test)]
mod tests {
    use crate::{
        core::state::State,
        smithay_backend::{
            linux_toplevel_admission_bridge::PendingXdgToplevelAdmission,
            linux_toplevel_admission_runtime_queue::{
                RuntimeToplevelAdmissionDrainTick, RuntimeToplevelAdmissionQueueBlocker,
                RuntimeToplevelAdmissionQueueOwner,
            },
            surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId},
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

    /// runtime owner 持有 queue + ledger，并可以 drain 一条 pending admission。
    #[test]
    fn runtime_owner_drains_one_pending_admission() {
        let adapter_surface = surface(701);
        let adapter_toplevel = toplevel(801);
        let pending = PendingXdgToplevelAdmission::new(adapter_surface, adapter_toplevel, Some(12));
        let mut owner = RuntimeToplevelAdmissionQueueOwner::new(6_000);
        let mut state = State::new();

        let enqueue = owner.enqueue_pending_toplevel_admission(pending);
        assert!(enqueue.runtime_queue_owned);
        assert!(enqueue.runtime_ledger_owned);
        assert_eq!(enqueue.pending_admission_count_before, 0);
        assert_eq!(enqueue.pending_admission_count_after, 1);
        assert!(enqueue.pending_admission_enqueued);

        let report = owner.drain_pending_toplevel_admission_once(
            &mut state,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(1),
        );

        assert!(report.runtime_queue_owned);
        assert!(report.runtime_ledger_owned);
        assert!(report.drain_invoked);
        assert_eq!(report.pending_admission_count_before, 1);
        assert_eq!(report.pending_admission_count_after, 0);
        assert!(report.ledger_consume_attempted);
        assert!(report.pending_admission_consumed);
        assert!(report.ledger_admit_surface_invoked);
        assert!(report.ledger_admit_invoked);
        assert!(report.core_register_invoked);
        assert!(report.window_id_allocated);
        assert_eq!(report.core_surface_id, Some(6_000));
        assert_eq!(report.next_core_surface_id_after, 6_001);
        let core_window = report
            .core_window_id
            .expect("runtime owner 必须返回 core WindowId");
        assert_eq!(owner.surface_mapping(adapter_surface), Some(6_000));
        assert_eq!(owner.toplevel_mapping(adapter_toplevel), Some(core_window));
        assert_eq!(owner.pending_count(), 0);
        assert_eq!(owner.next_core_surface_id(), 6_001);
        assert!(state.validate().is_clean());
        assert!(report.blockers.is_empty());
    }

    /// 空 queue 的 drain 不调用 ledger，也不推进 core surface id。
    #[test]
    fn runtime_owner_empty_drain_does_not_consume_ledger() {
        let mut owner = RuntimeToplevelAdmissionQueueOwner::new(7_000);
        let mut state = State::new();

        let report = owner.drain_pending_toplevel_admission_once(
            &mut state,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(2),
        );

        assert!(report.runtime_queue_owned);
        assert!(report.runtime_ledger_owned);
        assert!(report.drain_invoked);
        assert_eq!(report.pending_admission_count_before, 0);
        assert_eq!(report.pending_admission_count_after, 0);
        assert!(!report.ledger_consume_attempted);
        assert!(!report.pending_admission_consumed);
        assert!(!report.ledger_admit_surface_invoked);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
        assert_eq!(report.next_core_surface_id_after, 7_000);
        assert!(
            report
                .blockers
                .contains(&RuntimeToplevelAdmissionQueueBlocker::MissingPendingAdmission)
        );
    }
}
