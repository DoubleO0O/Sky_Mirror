//! Phase 51K Linux-only nested lifecycle single-pump coordinator。
//!
//! coordinator 只按固定顺序编排现有 [`NestedRealAcceptFlow`]：accept/insert 与
//! connected bridge、一次 Display dispatch、disconnected bridge。它不直接修改 core
//! registry，也不把单次 pump 冒充长期 compositor event loop。

use std::{
    collections::{BTreeSet, VecDeque},
    io,
    time::Duration,
};

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
            RuntimeToplevelUnmapDrainReport,
        },
        linux_wl_surface_identity::{
            AdapterSurfaceCommitObservation, SurfaceIdentityError, SurfaceIdentityKey,
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

/// 一次 lifecycle pump 后追加 live toplevel unmap drain 的组合报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLiveUnmapPumpReport {
    /// 既有 accept/connected/dispatch/disconnected lifecycle pump report。
    pub lifecycle_report: NestedRuntimePumpReport,

    /// live destroyed observation 经 runtime owner 调用 ledger unmap 的 report。
    pub unmap_drain_report: RuntimeToplevelUnmapDrainReport,
}

/// 一次 runtime pump 后追加 `wl_surface.commit` pure-data backlog drain 的报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitDrainReport {
    /// runtime commit drain seam 是否被调用。
    pub drain_invoked: bool,

    /// display owner 是否交出了一条 pending commit observation。
    pub commit_observation_present: bool,

    /// commit observation 是否成功解析为 adapter-owned surface identity。
    pub commit_observation_resolved: bool,

    /// commit observation 是否记录为 adapter identity error。
    pub commit_observation_failed: bool,

    /// 成功解析出的 adapter-only surface identity；不是 core `SurfaceId`。
    pub adapter_surface_id: Option<AdapterSurfaceId>,

    /// 成功解析出的 adapter-only surface identity key。
    pub surface_identity_key: Option<SurfaceIdentityKey>,

    /// 成功解析出的 FIFO commit sequence。
    pub commit_sequence: Option<u64>,

    /// 失败时保存 adapter identity error；只用于诊断。
    pub surface_identity_error: Option<SurfaceIdentityError>,

    /// 本次 commit 是否携带 buffer attach/remove evidence；只保留纯数据。
    pub buffer_attach_observed: bool,

    /// 本次 commit 是否携带真实 buffer presence evidence；不代表可 render。
    pub buffer_present: bool,

    /// 本次 commit 是否携带 `attach(NULL)` / buffer removal evidence。
    pub buffer_removed: bool,

    /// 本次 commit 是否已可作为 renderable buffer；Phase 54D 固定为 false。
    pub renderable_buffer: bool,

    /// 本次 commit 是否携带 damage / damage_buffer evidence；不代表已提交 render damage。
    pub damage_observed: bool,

    /// 本次 commit 中 surface-coordinate damage rectangle 数量。
    pub surface_damage_rects: usize,

    /// 本次 commit 中 buffer-coordinate damage rectangle 数量。
    pub buffer_damage_rects: usize,

    /// 本次 commit 是否携带 frame callback request evidence；不代表已发送 callback。
    pub frame_callback_observed: bool,

    /// 本次 commit 中 frame callback request 数量。
    pub frame_callback_count: usize,

    /// 是否处理 buffer attach；本阶段固定为 false。
    pub buffer_attached: bool,

    /// 是否处理 damage；本阶段固定为 false。
    pub damage_submitted: bool,

    /// 是否处理/request frame callback；本阶段固定为 false。
    pub frame_callback_requested: bool,

    /// 是否调用 render；本阶段固定为 false。
    pub render_invoked: bool,

    /// 是否调用 input；本阶段固定为 false。
    pub input_invoked: bool,

    /// 是否调用 admission ledger 或 core mutation；本阶段固定为 false。
    pub core_mutation_invoked: bool,
}

/// 从一次 `wl_surface.commit` drain report 派生出的 render-dirty/readiness 纯数据意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderDirtyReadinessIntent {
    /// adapter-only surface identity；不是 core `SurfaceId`。
    pub adapter_surface_id: AdapterSurfaceId,

    /// adapter-only surface identity key。
    pub surface_identity_key: SurfaceIdentityKey,

    /// 触发该 intent 的 FIFO commit sequence。
    pub commit_sequence: u64,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带真实 buffer presence evidence；不代表已 import。
    pub buffer_present: bool,

    /// commit 是否携带 `attach(NULL)` / buffer removal evidence。
    pub buffer_removed: bool,

    /// commit 是否已可作为 renderable buffer；当前仍固定为 false。
    pub renderable_buffer: bool,

    /// commit 是否携带 damage / damage_buffer evidence。
    pub damage_observed: bool,

    /// surface-coordinate damage rectangle 数量。
    pub surface_damage_rects: usize,

    /// buffer-coordinate damage rectangle 数量。
    pub buffer_damage_rects: usize,

    /// commit 是否携带 frame callback request evidence。
    pub frame_callback_observed: bool,

    /// frame callback request 数量。
    pub frame_callback_count: usize,

    /// 是否 import buffer；Phase 54G 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54G 固定为 false。
    pub texture_created: bool,

    /// 是否提交 render；Phase 54G 固定为 false。
    pub render_submitted: bool,

    /// 是否发送 frame callback done；Phase 54G 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54G 固定为 false。
    pub input_support: bool,
}

/// Runtime render-dirty intent queue 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderDirtyIntentQueueOperation {
    /// 从 commit drain report 派生 render-dirty/readiness intent。
    BuildIntent,
    /// 将 intent 放入 runtime-owned queue。
    EnqueueIntent,
    /// 读取 runtime-owned queue。
    ReadRuntimeQueue,
    /// 从 runtime-owned queue drain 一条 intent。
    DrainIntent,
    /// 生成保守 capability report。
    BuildReport,
}

/// Runtime render-dirty intent queue 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderDirtyIntentQueueBlocker {
    /// 当前没有可入队或可 drain 的 render-dirty/readiness intent。
    MissingRenderDirtyIntent,
}

/// Runtime-owned render-dirty/readiness intent queue 的一次 drain 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderDirtyIntentDrainReport {
    /// runtime 是否拥有 queue。
    pub runtime_queue_owned: bool,

    /// 本轮是否尝试从 commit drain report 入队。
    pub enqueue_invoked: bool,

    /// 本轮是否尝试 drain runtime-owned queue。
    pub drain_invoked: bool,

    /// 来源 commit observation 是否成功解析。
    pub source_commit_observation_resolved: bool,

    /// 入队前 pending intent 数量。
    pub pending_intent_count_before_enqueue: usize,

    /// 入队后 pending intent 数量。
    pub pending_intent_count_after_enqueue: usize,

    /// drain 前 pending intent 数量。
    pub pending_intent_count_before_drain: usize,

    /// drain 后 pending intent 数量。
    pub pending_intent_count_after_drain: usize,

    /// 本轮是否从 commit drain report 成功入队 intent。
    pub intent_enqueued: bool,

    /// 本轮是否从 runtime-owned queue 成功 drain intent。
    pub intent_drained: bool,

    /// 被 drain 的 pure-data render-dirty/readiness intent。
    pub drained_intent: Option<RuntimeSurfaceCommitRenderDirtyReadinessIntent>,

    /// 是否 import buffer；Phase 54H 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54H 固定为 false。
    pub texture_created: bool,

    /// 是否提交 render；Phase 54H 固定为 false。
    pub render_submitted: bool,

    /// 是否发送 frame callback done；Phase 54H 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54H 固定为 false。
    pub input_support: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderDirtyIntentQueueOperation>,

    /// 失败或未完成原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderDirtyIntentQueueBlocker>,
}

/// Runtime-owned render-dirty/readiness intent FIFO queue。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRenderDirtyIntentQueueOwner {
    queue: VecDeque<RuntimeSurfaceCommitRenderDirtyReadinessIntent>,
}

impl RuntimeSurfaceCommitRenderDirtyIntentQueueOwner {
    /// 创建空 runtime-owned render-dirty intent queue。
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回当前 pending render-dirty intent 数量。
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// 从 commit drain report 入队一条 intent，然后从 runtime queue drain 一条 intent。
    pub fn enqueue_from_commit_drain_and_drain_once(
        &mut self,
        report: &RuntimeSurfaceCommitDrainReport,
    ) -> RuntimeSurfaceCommitRenderDirtyIntentDrainReport {
        let pending_intent_count_before_enqueue = self.pending_count();
        let mut operations = vec![RuntimeSurfaceCommitRenderDirtyIntentQueueOperation::BuildIntent];
        let intent = render_dirty_readiness_intent_from_commit_drain_report(report);
        let intent_enqueued = if let Some(intent) = intent {
            operations.push(RuntimeSurfaceCommitRenderDirtyIntentQueueOperation::EnqueueIntent);
            self.queue.push_back(intent);
            true
        } else {
            false
        };
        let pending_intent_count_after_enqueue = self.pending_count();
        let pending_intent_count_before_drain = self.pending_count();
        operations.push(RuntimeSurfaceCommitRenderDirtyIntentQueueOperation::ReadRuntimeQueue);
        let drained_intent = self.queue.pop_front();
        let intent_drained = drained_intent.is_some();
        if intent_drained {
            operations.push(RuntimeSurfaceCommitRenderDirtyIntentQueueOperation::DrainIntent);
        }
        let pending_intent_count_after_drain = self.pending_count();
        operations.push(RuntimeSurfaceCommitRenderDirtyIntentQueueOperation::BuildReport);

        let blockers = if intent_enqueued || intent_drained {
            Vec::new()
        } else {
            vec![RuntimeSurfaceCommitRenderDirtyIntentQueueBlocker::MissingRenderDirtyIntent]
        };

        RuntimeSurfaceCommitRenderDirtyIntentDrainReport {
            runtime_queue_owned: true,
            enqueue_invoked: true,
            drain_invoked: true,
            source_commit_observation_resolved: report.commit_observation_resolved,
            pending_intent_count_before_enqueue,
            pending_intent_count_after_enqueue,
            pending_intent_count_before_drain,
            pending_intent_count_after_drain,
            intent_enqueued,
            intent_drained,
            drained_intent,
            buffer_imported: false,
            texture_created: false,
            render_submitted: false,
            frame_callback_done_sent: false,
            input_support: false,
            operations,
            blockers,
        }
    }
}

/// Renderer-admission seam 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererAdmissionOperation {
    /// 读取 runtime queue drain report。
    ReadRenderDirtyIntentDrain,
    /// 从 drained render-dirty intent 创建 renderer work intent。
    BuildRendererWorkIntent,
    /// 生成 renderer-admission report。
    BuildReport,
}

/// Renderer-admission seam 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererAdmissionBlocker {
    /// 本轮没有 drained render-dirty/readiness intent 可供 admission。
    MissingRenderDirtyIntent,
}

/// 从 drained render-dirty/readiness intent 派生出的 renderer work 纯数据意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererAdmissionWorkIntent {
    /// adapter-only surface identity；不是 core `SurfaceId`。
    pub adapter_surface_id: AdapterSurfaceId,

    /// adapter-only surface identity key。
    pub surface_identity_key: SurfaceIdentityKey,

    /// 触发该 work intent 的 FIFO commit sequence。
    pub commit_sequence: u64,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带真实 buffer presence evidence；不代表已 import。
    pub buffer_present: bool,

    /// commit 是否携带 `attach(NULL)` / buffer removal evidence。
    pub buffer_removed: bool,

    /// commit 是否已可作为 renderable buffer；当前仍固定为 false。
    pub renderable_buffer: bool,

    /// commit 是否携带 damage / damage_buffer evidence。
    pub damage_observed: bool,

    /// surface-coordinate damage rectangle 数量。
    pub surface_damage_rects: usize,

    /// buffer-coordinate damage rectangle 数量。
    pub buffer_damage_rects: usize,

    /// commit 是否携带 frame callback request evidence。
    pub frame_callback_observed: bool,

    /// frame callback request 数量。
    pub frame_callback_count: usize,

    /// 是否 import buffer；Phase 54I 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54I 固定为 false。
    pub texture_created: bool,

    /// 是否提交 render；Phase 54I 固定为 false。
    pub render_submitted: bool,

    /// 是否提交 damage；Phase 54I 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54I 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54I 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54I 固定为 false。
    pub core_mutation_invoked: bool,
}

/// Renderer-admission seam 的一次纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererAdmissionReport {
    /// 本轮是否执行 renderer-admission seam。
    pub renderer_admission_invoked: bool,

    /// 来源 render-dirty runtime queue drain 是否提供了 drained intent。
    pub source_render_dirty_intent_drained: bool,

    /// 本轮是否创建 renderer work intent。
    pub work_intent_created: bool,

    /// 从 drained render-dirty intent 派生出的 renderer work 纯数据意图。
    pub work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// 是否 import buffer；Phase 54I 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54I 固定为 false。
    pub texture_created: bool,

    /// 是否提交 render；Phase 54I 固定为 false。
    pub render_submitted: bool,

    /// 是否提交 damage；Phase 54I 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54I 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54I 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54I 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRendererAdmissionOperation>,

    /// 失败或未完成原因。
    pub blockers: Vec<RuntimeSurfaceCommitRendererAdmissionBlocker>,
}

/// Renderer owner boundary 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererOwnerBoundaryOperation {
    /// 消费 renderer-admission work intent。
    ConsumeWorkIntent,
    /// 检查 renderer owner 边界是否具备后续依赖。
    CheckRendererOwnerBoundary,
    /// 生成 blocked readiness report。
    BuildBlockedReport,
}

/// Renderer owner boundary 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererOwnerBoundaryBlocker {
    /// 本轮没有 renderer-admission work intent 可消费。
    MissingRendererWorkIntent,
    /// 真实 renderer owner 尚未接入。
    MissingRendererOwner,
    /// buffer importer 尚未接入。
    MissingBufferImporter,
    /// texture 创建支持尚未接入。
    MissingTextureSupport,
}

/// Renderer owner boundary 消费 work intent 后生成的 blocked readiness report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererOwnerBoundaryReport {
    /// renderer owner 边界是否已定义。
    pub owner_boundary_defined: bool,

    /// 本轮是否尝试消费 renderer-admission work intent。
    pub consume_invoked: bool,

    /// 本轮是否消费到 renderer-admission work intent。
    pub work_intent_consumed: bool,

    /// 被消费的 renderer-admission pure-data work intent。
    pub consumed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// 真实 renderer owner 是否可用；Phase 54J 固定为 false。
    pub renderer_owner_available: bool,

    /// buffer importer 是否可用；Phase 54J 固定为 false。
    pub buffer_importer_available: bool,

    /// texture 创建支持是否可用；Phase 54J 固定为 false。
    pub texture_support_available: bool,

    /// 是否 import buffer；Phase 54J 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54J 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54J 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54J 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54J 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54J 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54J 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRendererOwnerBoundaryOperation>,

    /// 阻止进入真实 render 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRendererOwnerBoundaryBlocker>,
}

/// Renderer owner shell readiness 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererOwnerShellOperation {
    /// 读取 renderer owner boundary report。
    ObserveOwnerBoundaryReport,
    /// 绑定 runtime-owned renderer owner shell。
    BindRendererOwnerShell,
    /// 生成 shell readiness report。
    BuildReadinessReport,
}

/// Renderer owner shell readiness 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererOwnerShellBlocker {
    /// 本轮没有 renderer work intent 可观察。
    MissingRendererWorkIntent,
    /// renderer owner shell 尚未可用。
    MissingRendererOwner,
    /// buffer importer 尚未接入。
    MissingBufferImporter,
    /// texture 创建支持尚未接入。
    MissingTextureSupport,
}

/// Runtime-owned renderer owner shell readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererOwnerShellReadinessReport {
    /// 本轮是否执行 shell readiness seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 renderer owner boundary report。
    pub owner_boundary_report_observed: bool,

    /// 上游 boundary 是否消费到 renderer work intent。
    pub owner_boundary_work_intent_consumed: bool,

    /// 从上游 boundary 观察到的 renderer work intent。
    pub observed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// runtime-owned renderer owner shell 是否可用；仍不代表真实 renderer 可调用。
    pub renderer_owner_shell_available: bool,

    /// buffer importer 是否可用；Phase 54K 固定为 false。
    pub buffer_importer_available: bool,

    /// texture 创建支持是否可用；Phase 54K 固定为 false。
    pub texture_support_available: bool,

    /// 是否 import buffer；Phase 54K 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54K 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54K 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54K 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54K 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54K 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54K 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRendererOwnerShellOperation>,

    /// 阻止进入真实 render 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRendererOwnerShellBlocker>,
}

/// Runtime-owned renderer owner shell；不持有真实 renderer 或 core state。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRendererOwnerShell;

impl RuntimeSurfaceCommitRendererOwnerShell {
    /// 创建 runtime-owned renderer owner shell readiness 边界。
    pub fn new() -> Self {
        Self
    }

    /// 从 owner boundary report 派生 shell readiness report；不触发真实 render。
    pub fn renderer_owner_shell_readiness_from_owner_boundary(
        &mut self,
        report: &RuntimeSurfaceCommitRendererOwnerBoundaryReport,
    ) -> RuntimeSurfaceCommitRendererOwnerShellReadinessReport {
        renderer_owner_shell_readiness_from_owner_boundary(report)
    }
}

/// 从 renderer owner boundary report 派生 shell readiness report；不触发真实 render。
pub fn renderer_owner_shell_readiness_from_owner_boundary(
    report: &RuntimeSurfaceCommitRendererOwnerBoundaryReport,
) -> RuntimeSurfaceCommitRendererOwnerShellReadinessReport {
    let observed_work_intent = report.consumed_work_intent.clone();
    let mut blockers = Vec::new();
    if observed_work_intent.is_none() {
        blockers.push(RuntimeSurfaceCommitRendererOwnerShellBlocker::MissingRendererWorkIntent);
    }
    blockers.extend([
        RuntimeSurfaceCommitRendererOwnerShellBlocker::MissingBufferImporter,
        RuntimeSurfaceCommitRendererOwnerShellBlocker::MissingTextureSupport,
    ]);

    RuntimeSurfaceCommitRendererOwnerShellReadinessReport {
        readiness_invoked: true,
        owner_boundary_report_observed: report.owner_boundary_defined,
        owner_boundary_work_intent_consumed: report.work_intent_consumed,
        observed_work_intent,
        renderer_owner_shell_available: true,
        buffer_importer_available: false,
        texture_support_available: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRendererOwnerShellOperation::ObserveOwnerBoundaryReport,
            RuntimeSurfaceCommitRendererOwnerShellOperation::BindRendererOwnerShell,
            RuntimeSurfaceCommitRendererOwnerShellOperation::BuildReadinessReport,
        ],
        blockers,
    }
}

/// Buffer importer shell readiness 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImporterShellOperation {
    /// 读取 renderer owner shell readiness report。
    ObserveRendererOwnerShellReadiness,
    /// 绑定 runtime-owned buffer importer shell。
    BindBufferImporterShell,
    /// 生成 importer shell readiness report。
    BuildReadinessReport,
}

/// Buffer importer shell readiness 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImporterShellBlocker {
    /// 本轮没有 renderer work intent 可观察。
    MissingRendererWorkIntent,
    /// renderer owner shell 尚未可用。
    MissingRendererOwnerShell,
    /// buffer importer shell 尚未可用。
    MissingBufferImporter,
    /// texture 创建支持尚未接入。
    MissingTextureSupport,
}

/// Runtime-owned buffer importer shell readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImporterShellReadinessReport {
    /// 本轮是否执行 importer shell readiness seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 renderer owner shell report。
    pub renderer_owner_shell_report_observed: bool,

    /// 上游 renderer owner shell 是否可用。
    pub renderer_owner_shell_available: bool,

    /// 从上游 owner shell 观察到的 renderer work intent。
    pub observed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// runtime-owned buffer importer shell 是否可用；不代表已 import buffer。
    pub buffer_importer_shell_available: bool,

    /// buffer importer 边界是否可用；不代表已 import buffer。
    pub buffer_importer_available: bool,

    /// texture 创建支持是否可用；Phase 54L 固定为 false。
    pub texture_support_available: bool,

    /// 是否 import buffer；Phase 54L 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54L 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54L 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54L 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54L 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54L 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54L 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImporterShellOperation>,

    /// 阻止进入真实 render 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImporterShellBlocker>,
}

/// Runtime-owned buffer importer shell；不持有真实 buffer importer 或 renderer state。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImporterShell;

impl RuntimeSurfaceCommitBufferImporterShell {
    /// 创建 runtime-owned buffer importer shell readiness 边界。
    pub fn new() -> Self {
        Self
    }

    /// 从 owner shell readiness report 派生 importer shell readiness report；不 import buffer。
    pub fn buffer_importer_shell_readiness_from_renderer_owner_shell(
        &mut self,
        report: &RuntimeSurfaceCommitRendererOwnerShellReadinessReport,
    ) -> RuntimeSurfaceCommitBufferImporterShellReadinessReport {
        buffer_importer_shell_readiness_from_renderer_owner_shell(report)
    }
}

/// 从 renderer owner shell readiness report 派生 importer shell readiness report；不 import buffer。
pub fn buffer_importer_shell_readiness_from_renderer_owner_shell(
    report: &RuntimeSurfaceCommitRendererOwnerShellReadinessReport,
) -> RuntimeSurfaceCommitBufferImporterShellReadinessReport {
    let observed_work_intent = report.observed_work_intent.clone();
    let renderer_owner_shell_available = report.renderer_owner_shell_available;
    let mut blockers = Vec::new();
    if observed_work_intent.is_none() {
        blockers.push(RuntimeSurfaceCommitBufferImporterShellBlocker::MissingRendererWorkIntent);
    }
    if !renderer_owner_shell_available {
        blockers.push(RuntimeSurfaceCommitBufferImporterShellBlocker::MissingRendererOwnerShell);
    }
    blockers.push(RuntimeSurfaceCommitBufferImporterShellBlocker::MissingTextureSupport);

    RuntimeSurfaceCommitBufferImporterShellReadinessReport {
        readiness_invoked: true,
        renderer_owner_shell_report_observed: report.readiness_invoked,
        renderer_owner_shell_available,
        observed_work_intent,
        buffer_importer_shell_available: true,
        buffer_importer_available: true,
        texture_support_available: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImporterShellOperation::ObserveRendererOwnerShellReadiness,
            RuntimeSurfaceCommitBufferImporterShellOperation::BindBufferImporterShell,
            RuntimeSurfaceCommitBufferImporterShellOperation::BuildReadinessReport,
        ],
        blockers,
    }
}

/// Runtime-owned renderer-admission work intent consumer。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRendererAdmissionOwner;

impl RuntimeSurfaceCommitRendererAdmissionOwner {
    /// 创建不持有 renderer/input/core 状态的 renderer-admission owner boundary。
    pub fn new() -> Self {
        Self
    }

    /// 消费 renderer-admission work intent，并返回 blocked readiness report。
    pub fn consume_renderer_admission_work_intent(
        &mut self,
        report: &RuntimeSurfaceCommitRendererAdmissionReport,
    ) -> RuntimeSurfaceCommitRendererOwnerBoundaryReport {
        let consumed_work_intent = report.work_intent.clone();
        let work_intent_consumed = consumed_work_intent.is_some();
        let mut operations = vec![
            RuntimeSurfaceCommitRendererOwnerBoundaryOperation::ConsumeWorkIntent,
            RuntimeSurfaceCommitRendererOwnerBoundaryOperation::CheckRendererOwnerBoundary,
        ];
        operations.push(RuntimeSurfaceCommitRendererOwnerBoundaryOperation::BuildBlockedReport);

        let mut blockers = Vec::new();
        if !work_intent_consumed {
            blockers
                .push(RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingRendererWorkIntent);
        }
        blockers.extend([
            RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingRendererOwner,
            RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingBufferImporter,
            RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingTextureSupport,
        ]);

        RuntimeSurfaceCommitRendererOwnerBoundaryReport {
            owner_boundary_defined: true,
            consume_invoked: true,
            work_intent_consumed,
            consumed_work_intent,
            renderer_owner_available: false,
            buffer_importer_available: false,
            texture_support_available: false,
            buffer_imported: false,
            texture_created: false,
            renderer_called: false,
            damage_submitted: false,
            frame_callback_done_sent: false,
            input_support: false,
            core_mutation_invoked: false,
            operations,
            blockers,
        }
    }
}

/// 从 runtime queue drain report 派生 renderer-admission report；不触发真实 render。
pub fn renderer_admission_report_from_render_dirty_intent_drain(
    report: &RuntimeSurfaceCommitRenderDirtyIntentDrainReport,
) -> RuntimeSurfaceCommitRendererAdmissionReport {
    let mut operations =
        vec![RuntimeSurfaceCommitRendererAdmissionOperation::ReadRenderDirtyIntentDrain];
    let source_render_dirty_intent_drained = report.intent_drained;
    let work_intent = report.drained_intent.as_ref().map(|intent| {
        operations.push(RuntimeSurfaceCommitRendererAdmissionOperation::BuildRendererWorkIntent);
        RuntimeSurfaceCommitRendererAdmissionWorkIntent {
            adapter_surface_id: intent.adapter_surface_id,
            surface_identity_key: intent.surface_identity_key,
            commit_sequence: intent.commit_sequence,
            buffer_attach_observed: intent.buffer_attach_observed,
            buffer_present: intent.buffer_present,
            buffer_removed: intent.buffer_removed,
            renderable_buffer: intent.renderable_buffer,
            damage_observed: intent.damage_observed,
            surface_damage_rects: intent.surface_damage_rects,
            buffer_damage_rects: intent.buffer_damage_rects,
            frame_callback_observed: intent.frame_callback_observed,
            frame_callback_count: intent.frame_callback_count,
            buffer_imported: false,
            texture_created: false,
            render_submitted: false,
            damage_submitted: false,
            frame_callback_done_sent: false,
            input_support: false,
            core_mutation_invoked: false,
        }
    });
    let work_intent_created = work_intent.is_some();
    operations.push(RuntimeSurfaceCommitRendererAdmissionOperation::BuildReport);
    let blockers = if work_intent_created {
        Vec::new()
    } else {
        vec![RuntimeSurfaceCommitRendererAdmissionBlocker::MissingRenderDirtyIntent]
    };

    RuntimeSurfaceCommitRendererAdmissionReport {
        renderer_admission_invoked: true,
        source_render_dirty_intent_drained,
        work_intent_created,
        work_intent,
        buffer_imported: false,
        texture_created: false,
        render_submitted: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations,
        blockers,
    }
}

/// 从 commit drain report 派生 render-dirty/readiness intent；不触发真实 render。
pub fn render_dirty_readiness_intent_from_commit_drain_report(
    report: &RuntimeSurfaceCommitDrainReport,
) -> Option<RuntimeSurfaceCommitRenderDirtyReadinessIntent> {
    let (Some(adapter_surface_id), Some(surface_identity_key), Some(commit_sequence)) = (
        report.adapter_surface_id,
        report.surface_identity_key,
        report.commit_sequence,
    ) else {
        return None;
    };

    if !report.commit_observation_resolved {
        return None;
    }

    Some(RuntimeSurfaceCommitRenderDirtyReadinessIntent {
        adapter_surface_id,
        surface_identity_key,
        commit_sequence,
        buffer_attach_observed: report.buffer_attach_observed,
        buffer_present: report.buffer_present,
        buffer_removed: report.buffer_removed,
        renderable_buffer: report.renderable_buffer,
        damage_observed: report.damage_observed,
        surface_damage_rects: report.surface_damage_rects,
        buffer_damage_rects: report.buffer_damage_rects,
        frame_callback_observed: report.frame_callback_observed,
        frame_callback_count: report.frame_callback_count,
        buffer_imported: false,
        texture_created: false,
        render_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
    })
}

impl RuntimeSurfaceCommitDrainReport {
    fn from_observation(
        observation: Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>>,
    ) -> Self {
        let mut report = Self {
            drain_invoked: true,
            commit_observation_present: observation.is_some(),
            commit_observation_resolved: false,
            commit_observation_failed: false,
            adapter_surface_id: None,
            surface_identity_key: None,
            commit_sequence: None,
            surface_identity_error: None,
            buffer_attach_observed: false,
            buffer_present: false,
            buffer_removed: false,
            renderable_buffer: false,
            damage_observed: false,
            surface_damage_rects: 0,
            buffer_damage_rects: 0,
            frame_callback_observed: false,
            frame_callback_count: 0,
            buffer_attached: false,
            damage_submitted: false,
            frame_callback_requested: false,
            render_invoked: false,
            input_invoked: false,
            core_mutation_invoked: false,
        };

        match observation {
            Some(Ok(commit)) => {
                report.commit_observation_resolved = true;
                report.adapter_surface_id = Some(commit.adapter_surface_id);
                report.surface_identity_key = Some(commit.surface_identity_key);
                report.commit_sequence = Some(commit.commit_sequence);
                report.buffer_attach_observed = commit.buffer_attach_observed;
                report.buffer_present = commit.buffer_present;
                report.buffer_removed = commit.buffer_removed;
                report.renderable_buffer = commit.renderable_buffer;
                report.damage_observed = commit.damage_observed;
                report.surface_damage_rects = commit.surface_damage_rects;
                report.buffer_damage_rects = commit.buffer_damage_rects;
                report.frame_callback_observed = commit.frame_callback_observed;
                report.frame_callback_count = commit.frame_callback_count;
            }
            Some(Err(error)) => {
                report.commit_observation_failed = true;
                report.surface_identity_error = Some(error);
            }
            None => {}
        }

        report
    }
}

/// 一次 lifecycle pump 后同时追加 live admission、live unmap 与 surface commit drain 的组合报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLiveAdmissionUnmapPumpReport {
    /// 既有 accept/connected/dispatch/disconnected lifecycle pump report。
    pub lifecycle_report: NestedRuntimePumpReport,

    /// Phase 53A live callback admission owner 的 enqueue report。
    pub live_admission_owner_report: LiveToplevelAdmissionOwnerReport,

    /// live owner 入队后由 runtime admission queue owner drain 的 report。
    pub admission_drain_report: RuntimeToplevelAdmissionDrainReport,

    /// live destroyed observation 经 runtime owner 调用 ledger unmap 的 report。
    pub unmap_drain_report: RuntimeToplevelUnmapDrainReport,

    /// `wl_surface.commit` observation backlog 的 pure-data drain report。
    pub surface_commit_drain_report: RuntimeSurfaceCommitDrainReport,

    /// render-dirty/readiness intent runtime-owned queue drain report。
    pub render_dirty_intent_drain_report: RuntimeSurfaceCommitRenderDirtyIntentDrainReport,

    /// renderer-admission pure-data work intent report。
    pub renderer_admission_report: RuntimeSurfaceCommitRendererAdmissionReport,

    /// renderer owner boundary blocked readiness report。
    pub renderer_owner_boundary_report: RuntimeSurfaceCommitRendererOwnerBoundaryReport,

    /// renderer owner shell readiness report。
    pub renderer_owner_shell_readiness_report:
        RuntimeSurfaceCommitRendererOwnerShellReadinessReport,

    /// buffer importer shell readiness report。
    pub buffer_importer_shell_readiness_report:
        RuntimeSurfaceCommitBufferImporterShellReadinessReport,
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
    render_dirty_intent_queue_owner: RuntimeSurfaceCommitRenderDirtyIntentQueueOwner,
    renderer_admission_owner: RuntimeSurfaceCommitRendererAdmissionOwner,
    renderer_owner_shell: RuntimeSurfaceCommitRendererOwnerShell,
    buffer_importer_shell: RuntimeSurfaceCommitBufferImporterShell,
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
            render_dirty_intent_queue_owner: RuntimeSurfaceCommitRenderDirtyIntentQueueOwner::new(),
            renderer_admission_owner: RuntimeSurfaceCommitRendererAdmissionOwner::new(),
            renderer_owner_shell: RuntimeSurfaceCommitRendererOwnerShell::new(),
            buffer_importer_shell: RuntimeSurfaceCommitBufferImporterShell::new(),
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

    /// 返回 coordinator render-dirty intent owner 中的 pending 数量。
    pub fn render_dirty_intent_pending_count(&self) -> usize {
        self.render_dirty_intent_queue_owner.pending_count()
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

    /// 执行一次 lifecycle pump，然后读取 live destroyed observation 并 drain ledger unmap。
    ///
    /// handler/display 只提供纯数据 lifecycle observation；coordinator 把它交给
    /// runtime admission owner，由后者在拥有 ledger + `State` 的边界内执行 core detach。
    pub fn pump_once_with_live_toplevel_unmap_drain(
        &mut self,
        state: &mut State,
        timeout: Duration,
    ) -> NestedRuntimeLiveUnmapPumpReport {
        let lifecycle_report = self.pump_once(state, timeout);
        let observation = self.flow.take_next_live_toplevel_unmap_observation();
        let unmap_drain_report = self
            .admission_queue_owner
            .drain_live_toplevel_unmap_once(state, observation);

        NestedRuntimeLiveUnmapPumpReport {
            lifecycle_report,
            unmap_drain_report,
        }
    }

    /// 执行一次 lifecycle pump，然后依次 drain live admission、live toplevel unmap 与 commit backlog。
    ///
    /// 该组合 seam 让 bounded loop 每轮只执行一次 lifecycle pump，同时把 callback
    /// admission、destroyed unmap 与 `wl_surface.commit` observation 交给 runtime owners。
    /// handler/display 仍只提供纯数据 observation，不持有 `State` 或 admission ledger。
    pub fn pump_once_with_live_toplevel_admission_and_unmap_drain(
        &mut self,
        state: &mut State,
        timeout: Duration,
        tick: RuntimeToplevelAdmissionDrainTick,
    ) -> NestedRuntimeLiveAdmissionUnmapPumpReport {
        let lifecycle_report = self.pump_once(state, timeout);
        let admission_observation = self.flow.take_next_live_toplevel_admission_observation();
        let live_admission_owner_report =
            enqueue_live_toplevel_admission_from_observation(admission_observation, self);
        let admission_drain_report = self
            .admission_queue_owner
            .drain_pending_toplevel_admission_once(state, tick);
        let unmap_observation = self.flow.take_next_live_toplevel_unmap_observation();
        let unmap_drain_report = self
            .admission_queue_owner
            .drain_live_toplevel_unmap_once(state, unmap_observation);
        let surface_commit_drain_report = RuntimeSurfaceCommitDrainReport::from_observation(
            self.flow.take_next_wl_surface_commit_observation(),
        );
        let render_dirty_intent_drain_report = self
            .render_dirty_intent_queue_owner
            .enqueue_from_commit_drain_and_drain_once(&surface_commit_drain_report);
        let renderer_admission_report = renderer_admission_report_from_render_dirty_intent_drain(
            &render_dirty_intent_drain_report,
        );
        let renderer_owner_boundary_report = self
            .renderer_admission_owner
            .consume_renderer_admission_work_intent(&renderer_admission_report);
        let renderer_owner_shell_readiness_report = self
            .renderer_owner_shell
            .renderer_owner_shell_readiness_from_owner_boundary(&renderer_owner_boundary_report);
        let buffer_importer_shell_readiness_report = self
            .buffer_importer_shell
            .buffer_importer_shell_readiness_from_renderer_owner_shell(
                &renderer_owner_shell_readiness_report,
            );

        NestedRuntimeLiveAdmissionUnmapPumpReport {
            lifecycle_report,
            live_admission_owner_report,
            admission_drain_report,
            unmap_drain_report,
            surface_commit_drain_report,
            render_dirty_intent_drain_report,
            renderer_admission_report,
            renderer_owner_boundary_report,
            renderer_owner_shell_readiness_report,
            buffer_importer_shell_readiness_report,
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

    /// 验证 admitted live toplevel 的 destroyed observation 只在 owner 层触发 ledger unmap。
    #[test]
    fn nested_runtime_live_unmap_pump_detaches_admitted_toplevel() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-live-unmap");
        let mut coordinator =
            NestedRuntimeCoordinator::with_socket_name_and_admission_surface_start(
                &socket_name,
                15_000,
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

        let admission = coordinator.pump_once_with_live_toplevel_admission_drain(
            &mut state,
            Duration::ZERO,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(57),
        );
        let core_window = admission
            .admission_drain_report
            .core_window_id
            .expect("live admission 必须创建 core window");
        let unmap = coordinator
            .pump_once_with_live_toplevel_unmap_drain(&mut state, Duration::from_millis(1));

        assert!(admission.lifecycle_report.is_successful());
        assert!(admission.admission_drain_report.pending_admission_consumed);
        assert!(unmap.lifecycle_report.is_successful());
        assert!(unmap.unmap_drain_report.live_unmap_observation_present);
        assert!(unmap.unmap_drain_report.adapter_toplevel_id_resolved);
        assert!(unmap.unmap_drain_report.ledger_unmap_invoked);
        assert!(unmap.unmap_drain_report.core_detach_invoked);
        assert_eq!(
            unmap.unmap_drain_report.adapter_surface_id,
            Some(registration.adapter_surface_id)
        );
        assert_eq!(
            unmap.unmap_drain_report.adapter_toplevel_id,
            Some(registration.adapter_toplevel_id)
        );
        assert_eq!(unmap.unmap_drain_report.core_surface_id, Some(15_000));
        assert_eq!(unmap.unmap_drain_report.core_window_id, Some(core_window));
        assert!(
            unmap
                .unmap_drain_report
                .surface_mapping_retained_after_unmap
        );
        assert!(
            unmap
                .unmap_drain_report
                .toplevel_mapping_removed_after_unmap
        );
        assert!(unmap.unmap_drain_report.surface_remains_alive);
        assert_eq!(
            coordinator.admission_surface_mapping(registration.adapter_surface_id),
            Some(15_000)
        );
        assert_eq!(
            coordinator.admission_toplevel_mapping(registration.adapter_toplevel_id),
            None
        );
        assert!(state.surfaces.is_alive(15_000));
        assert!(!state.registry.is_alive(core_window));
        assert!(state.validate().is_clean());
        assert!(unmap.unmap_drain_report.blockers.is_empty());
    }
}
