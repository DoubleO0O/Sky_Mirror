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
        linux_shm_buffer_import_adapter::{
            LinuxShmFirstBufferImportAdapterSkeleton,
            RuntimeSurfaceCommitDamageToTextureMappingAuditReport,
            RuntimeSurfaceCommitRendererBackendInstanceAuditReport,
            RuntimeSurfaceCommitShmBufferMetadataReport,
            RuntimeSurfaceCommitShmFirstBufferImportAdapterReport,
            RuntimeSurfaceCommitTextureCreationNoopReport,
            RuntimeSurfaceCommitTextureCreationPreconditionAuditReport,
            RuntimeSurfaceCommitTextureImportRouteDecisionReport,
            RuntimeSurfaceCommitTextureOwnerBoundaryReport,
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

/// Texture support shell readiness 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureSupportShellOperation {
    /// 读取 buffer importer shell readiness report。
    ObserveBufferImporterShellReadiness,
    /// 绑定 runtime-owned texture support shell。
    BindTextureSupportShell,
    /// 生成 texture support shell readiness report。
    BuildReadinessReport,
}

/// Texture support shell readiness 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureSupportShellBlocker {
    /// 本轮没有 renderer work intent 可观察。
    MissingRendererWorkIntent,
    /// buffer importer shell 尚未可用。
    MissingBufferImporterShell,
    /// texture support shell 尚未可用。
    MissingTextureSupport,
}

/// Runtime-owned texture support shell readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureSupportShellReadinessReport {
    /// 本轮是否执行 texture support shell readiness seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 buffer importer shell report。
    pub buffer_importer_shell_report_observed: bool,

    /// 上游 buffer importer shell 是否可用。
    pub buffer_importer_shell_available: bool,

    /// 从上游 importer shell 观察到的 renderer work intent。
    pub observed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// runtime-owned texture support shell 是否可用；不代表已创建 texture。
    pub texture_support_shell_available: bool,

    /// texture support 边界是否可用；不代表已创建 texture。
    pub texture_support_available: bool,

    /// 是否 import buffer；Phase 54M 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54M 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54M 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54M 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54M 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54M 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54M 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitTextureSupportShellOperation>,

    /// 阻止进入真实 render 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitTextureSupportShellBlocker>,
}

/// Runtime-owned texture support shell；不持有真实 texture 或 renderer state。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitTextureSupportShell;

impl RuntimeSurfaceCommitTextureSupportShell {
    /// 创建 runtime-owned texture support shell readiness 边界。
    pub fn new() -> Self {
        Self
    }

    /// 从 importer shell readiness report 派生 texture support shell readiness report；不创建 texture。
    pub fn texture_support_shell_readiness_from_buffer_importer_shell(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImporterShellReadinessReport,
    ) -> RuntimeSurfaceCommitTextureSupportShellReadinessReport {
        texture_support_shell_readiness_from_buffer_importer_shell(report)
    }
}

/// 从 buffer importer shell readiness report 派生 texture support shell readiness report；不创建 texture。
pub fn texture_support_shell_readiness_from_buffer_importer_shell(
    report: &RuntimeSurfaceCommitBufferImporterShellReadinessReport,
) -> RuntimeSurfaceCommitTextureSupportShellReadinessReport {
    let observed_work_intent = report.observed_work_intent.clone();
    let buffer_importer_shell_available = report.buffer_importer_shell_available;
    let mut blockers = Vec::new();
    if observed_work_intent.is_none() {
        blockers.push(RuntimeSurfaceCommitTextureSupportShellBlocker::MissingRendererWorkIntent);
    }
    if !buffer_importer_shell_available || !report.buffer_importer_available {
        blockers.push(RuntimeSurfaceCommitTextureSupportShellBlocker::MissingBufferImporterShell);
    }

    RuntimeSurfaceCommitTextureSupportShellReadinessReport {
        readiness_invoked: true,
        buffer_importer_shell_report_observed: report.readiness_invoked,
        buffer_importer_shell_available,
        observed_work_intent,
        texture_support_shell_available: true,
        texture_support_available: true,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitTextureSupportShellOperation::ObserveBufferImporterShellReadiness,
            RuntimeSurfaceCommitTextureSupportShellOperation::BindTextureSupportShell,
            RuntimeSurfaceCommitTextureSupportShellOperation::BuildReadinessReport,
        ],
        blockers,
    }
}

/// Render operation readiness 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderOperationOperation {
    /// 读取 texture support shell readiness report。
    ObserveTextureSupportShellReadiness,
    /// 从 readiness evidence 构建 render operation intent。
    BuildRenderOperationIntent,
    /// 生成 render execution readiness report。
    BuildReadinessReport,
}

/// Render operation readiness 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderOperationBlocker {
    /// 本轮没有 renderer work intent 可观察。
    MissingRendererWorkIntent,
    /// texture support shell 尚未可用。
    MissingTextureSupportShell,
    /// texture support readiness 尚未成立。
    MissingTextureSupport,
}

/// 从 texture support shell readiness 派生出的 render operation 纯数据意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderOperationIntent {
    /// adapter-only surface identity；不是 core `SurfaceId`。
    pub adapter_surface_id: AdapterSurfaceId,

    /// adapter-only surface identity key。
    pub surface_identity_key: SurfaceIdentityKey,

    /// 触发该 operation intent 的 FIFO commit sequence。
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

    /// surface + buffer damage rectangle 总数；仍只是 evidence count。
    pub damage_rect_count: usize,

    /// commit 是否携带 frame callback request evidence。
    pub frame_callback_observed: bool,

    /// frame callback request 数量。
    pub frame_callback_count: usize,

    /// 是否 import buffer；Phase 54N 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54N 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54N 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54N 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54N 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54N 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54N 固定为 false。
    pub core_mutation_invoked: bool,
}

/// Render operation / render execution readiness 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderOperationReadinessReport {
    /// 本轮是否执行 render operation readiness seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 texture support shell report。
    pub source_texture_support_shell_report_observed: bool,

    /// 上游 texture support shell 是否可用。
    pub source_texture_support_shell_available: bool,

    /// 上游 texture support readiness 是否可用。
    pub source_texture_support_available: bool,

    /// 本轮是否创建 render operation intent。
    pub render_operation_intent_created: bool,

    /// 从 texture support shell readiness 派生出的 render operation 纯数据意图。
    pub render_operation_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// 是否 import buffer；Phase 54N 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54N 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54N 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54N 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54N 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54N 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54N 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderOperationOperation>,

    /// 阻止进入真实 render execution 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderOperationBlocker>,
}

/// 从 texture support shell readiness report 派生 render operation readiness；不执行 render。
pub fn render_operation_readiness_from_texture_support_shell(
    report: &RuntimeSurfaceCommitTextureSupportShellReadinessReport,
) -> RuntimeSurfaceCommitRenderOperationReadinessReport {
    let render_operation_intent = report.observed_work_intent.clone().map(|work_intent| {
        RuntimeSurfaceCommitRenderOperationIntent {
            adapter_surface_id: work_intent.adapter_surface_id,
            surface_identity_key: work_intent.surface_identity_key,
            commit_sequence: work_intent.commit_sequence,
            buffer_attach_observed: work_intent.buffer_attach_observed,
            buffer_present: work_intent.buffer_present,
            buffer_removed: work_intent.buffer_removed,
            renderable_buffer: work_intent.renderable_buffer,
            damage_observed: work_intent.damage_observed,
            surface_damage_rects: work_intent.surface_damage_rects,
            buffer_damage_rects: work_intent.buffer_damage_rects,
            damage_rect_count: work_intent
                .surface_damage_rects
                .saturating_add(work_intent.buffer_damage_rects),
            frame_callback_observed: work_intent.frame_callback_observed,
            frame_callback_count: work_intent.frame_callback_count,
            buffer_imported: false,
            texture_created: false,
            renderer_called: false,
            damage_submitted: false,
            frame_callback_done_sent: false,
            input_support: false,
            core_mutation_invoked: false,
        }
    });
    let mut blockers = Vec::new();
    if render_operation_intent.is_none() {
        blockers.push(RuntimeSurfaceCommitRenderOperationBlocker::MissingRendererWorkIntent);
    }
    if !report.texture_support_shell_available {
        blockers.push(RuntimeSurfaceCommitRenderOperationBlocker::MissingTextureSupportShell);
    }
    if !report.texture_support_available {
        blockers.push(RuntimeSurfaceCommitRenderOperationBlocker::MissingTextureSupport);
    }

    RuntimeSurfaceCommitRenderOperationReadinessReport {
        readiness_invoked: true,
        source_texture_support_shell_report_observed: report.readiness_invoked,
        source_texture_support_shell_available: report.texture_support_shell_available,
        source_texture_support_available: report.texture_support_available,
        render_operation_intent_created: render_operation_intent.is_some(),
        render_operation_intent,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRenderOperationOperation::ObserveTextureSupportShellReadiness,
            RuntimeSurfaceCommitRenderOperationOperation::BuildRenderOperationIntent,
            RuntimeSurfaceCommitRenderOperationOperation::BuildReadinessReport,
        ],
        blockers,
    }
}

/// Runtime-owned render operation intent queue 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderOperationIntentQueueOperation {
    /// 读取 render operation readiness report。
    ReadRenderOperationReadiness,
    /// 入队 render operation intent。
    EnqueueIntent,
    /// 读取 runtime-owned FIFO queue。
    ReadRuntimeQueue,
    /// 从 runtime-owned FIFO queue drain intent。
    DrainIntent,
    /// 生成 drain report。
    BuildReport,
}

/// Runtime-owned render operation intent queue 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderOperationIntentQueueBlocker {
    /// 本轮没有 render operation intent 可入队或 drain。
    MissingRenderOperationIntent,
}

/// Runtime-owned render operation intent queue 的一次 drain 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderOperationIntentDrainReport {
    /// runtime 是否拥有 queue。
    pub runtime_queue_owned: bool,

    /// 本轮是否尝试从 render operation readiness report 入队。
    pub enqueue_invoked: bool,

    /// 本轮是否尝试 drain runtime-owned queue。
    pub drain_invoked: bool,

    /// 来源 render operation readiness 是否创建了 intent。
    pub source_render_operation_intent_created: bool,

    /// 入队前 pending intent 数量。
    pub pending_intent_count_before_enqueue: usize,

    /// 入队后 pending intent 数量。
    pub pending_intent_count_after_enqueue: usize,

    /// drain 前 pending intent 数量。
    pub pending_intent_count_before_drain: usize,

    /// drain 后 pending intent 数量。
    pub pending_intent_count_after_drain: usize,

    /// 本轮是否从 readiness report 成功入队 intent。
    pub intent_enqueued: bool,

    /// 本轮是否从 runtime-owned queue 成功 drain intent。
    pub intent_drained: bool,

    /// 被 drain 的 pure-data render operation intent。
    pub drained_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// 是否 import buffer；Phase 54O 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54O 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54O 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54O 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54O 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54O 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54O 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderOperationIntentQueueOperation>,

    /// 失败或未完成原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderOperationIntentQueueBlocker>,
}

/// Runtime-owned render operation intent FIFO queue。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRenderOperationIntentQueueOwner {
    queue: VecDeque<RuntimeSurfaceCommitRenderOperationIntent>,
}

impl RuntimeSurfaceCommitRenderOperationIntentQueueOwner {
    /// 创建空 runtime-owned render operation intent queue。
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回当前 pending render operation intent 数量。
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// 从 render operation readiness report 入队一条 intent，然后从 runtime queue drain 一条。
    pub fn enqueue_from_render_operation_readiness_and_drain_once(
        &mut self,
        report: &RuntimeSurfaceCommitRenderOperationReadinessReport,
    ) -> RuntimeSurfaceCommitRenderOperationIntentDrainReport {
        let pending_intent_count_before_enqueue = self.pending_count();
        let mut operations = vec![
            RuntimeSurfaceCommitRenderOperationIntentQueueOperation::ReadRenderOperationReadiness,
        ];
        let intent_enqueued = if let Some(intent) = report.render_operation_intent.clone() {
            operations.push(RuntimeSurfaceCommitRenderOperationIntentQueueOperation::EnqueueIntent);
            self.queue.push_back(intent);
            true
        } else {
            false
        };
        let pending_intent_count_after_enqueue = self.pending_count();
        let pending_intent_count_before_drain = self.pending_count();
        operations.push(RuntimeSurfaceCommitRenderOperationIntentQueueOperation::ReadRuntimeQueue);
        let drained_intent = self.queue.pop_front();
        let intent_drained = drained_intent.is_some();
        if intent_drained {
            operations.push(RuntimeSurfaceCommitRenderOperationIntentQueueOperation::DrainIntent);
        }
        let pending_intent_count_after_drain = self.pending_count();
        operations.push(RuntimeSurfaceCommitRenderOperationIntentQueueOperation::BuildReport);

        let blockers = if intent_enqueued || intent_drained {
            Vec::new()
        } else {
            vec![
                RuntimeSurfaceCommitRenderOperationIntentQueueBlocker::MissingRenderOperationIntent,
            ]
        };

        RuntimeSurfaceCommitRenderOperationIntentDrainReport {
            runtime_queue_owned: true,
            enqueue_invoked: true,
            drain_invoked: true,
            source_render_operation_intent_created: report.render_operation_intent_created,
            pending_intent_count_before_enqueue,
            pending_intent_count_after_enqueue,
            pending_intent_count_before_drain,
            pending_intent_count_after_drain,
            intent_enqueued,
            intent_drained,
            drained_intent,
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

/// Render execution owner boundary 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderExecutionOwnerBoundaryOperation {
    /// 消费 runtime-drained render operation intent。
    ConsumeRenderOperationIntent,
    /// 检查 render execution owner boundary 是否具备真实执行能力。
    CheckRenderExecutionBoundary,
    /// 生成 blocked readiness report。
    BuildBlockedReport,
}

/// Render execution owner boundary 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker {
    /// 本轮没有 render operation intent 可消费。
    MissingRenderOperationIntent,
    /// 尚无 runtime-owned render execution owner。
    MissingRenderExecutionOwner,
    /// 尚未接入 buffer import。
    MissingBufferImport,
    /// 尚未接入 texture creation。
    MissingTextureCreation,
    /// 尚未接入 renderer call。
    MissingRendererCall,
    /// 尚未接入 damage submit。
    MissingDamageSubmit,
    /// 尚未接入 frame callback done。
    MissingFrameCallbackDone,
}

/// Render execution owner boundary 的一次 pure-data blocked/readiness report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport {
    /// 是否已定义 render execution owner boundary。
    pub owner_boundary_defined: bool,

    /// 本轮是否尝试消费 render operation intent。
    pub consume_invoked: bool,

    /// 本轮是否消费到 render operation intent。
    pub render_operation_intent_consumed: bool,

    /// 被消费的 render operation pure-data intent。
    pub consumed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// 是否已有真实 render execution owner；Phase 54P 固定为 false。
    pub render_execution_owner_available: bool,

    /// 是否已有 buffer import 能力；Phase 54P 固定为 false。
    pub buffer_import_available: bool,

    /// 是否已有 texture creation 能力；Phase 54P 固定为 false。
    pub texture_creation_available: bool,

    /// 是否已有 renderer call 能力；Phase 54P 固定为 false。
    pub renderer_call_available: bool,

    /// 是否已有 damage submit 能力；Phase 54P 固定为 false。
    pub damage_submit_available: bool,

    /// 是否已有 frame callback done 能力；Phase 54P 固定为 false。
    pub frame_callback_done_available: bool,

    /// 是否 import buffer；Phase 54P 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54P 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54P 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54P 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54P 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54P 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54P 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderExecutionOwnerBoundaryOperation>,

    /// 阻止进入真实 render execution 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker>,
}

/// Runtime-owned render execution owner boundary consumer。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRenderExecutionOwnerBoundary;

impl RuntimeSurfaceCommitRenderExecutionOwnerBoundary {
    /// 创建不持有 renderer/input/core 状态的 render execution owner boundary。
    pub fn new() -> Self {
        Self
    }

    /// 消费 render operation intent，并返回 blocked readiness report。
    pub fn consume_render_operation_intent(
        &mut self,
        report: &RuntimeSurfaceCommitRenderOperationIntentDrainReport,
    ) -> RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport {
        let consumed_intent = report.drained_intent.clone();
        let render_operation_intent_consumed = consumed_intent.is_some();
        let operations = vec![
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryOperation::ConsumeRenderOperationIntent,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryOperation::CheckRenderExecutionBoundary,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryOperation::BuildBlockedReport,
        ];

        let mut blockers = Vec::new();
        if !render_operation_intent_consumed {
            blockers.push(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingRenderOperationIntent,
            );
        }
        blockers.extend([
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingRenderExecutionOwner,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingBufferImport,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingTextureCreation,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingRendererCall,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingDamageSubmit,
            RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingFrameCallbackDone,
        ]);

        RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport {
            owner_boundary_defined: true,
            consume_invoked: true,
            render_operation_intent_consumed,
            consumed_intent,
            render_execution_owner_available: false,
            buffer_import_available: false,
            texture_creation_available: false,
            renderer_call_available: false,
            damage_submit_available: false,
            frame_callback_done_available: false,
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

/// Render execution owner shell readiness 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderExecutionOwnerShellOperation {
    /// 读取 render execution owner boundary report。
    ObserveRenderExecutionOwnerBoundaryReport,
    /// 绑定 runtime-owned render execution owner shell。
    BindRenderExecutionOwnerShell,
    /// 生成 shell readiness report。
    BuildReadinessReport,
}

/// Render execution owner shell readiness 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// buffer import 尚未接入。
    MissingBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned render execution owner shell readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport {
    /// 本轮是否执行 shell readiness seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 render execution owner boundary report。
    pub owner_boundary_report_observed: bool,

    /// 上游 boundary 是否消费到 render operation intent。
    pub owner_boundary_render_operation_intent_consumed: bool,

    /// 从上游 boundary 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned render execution owner shell 是否可用；仍不代表真实 render 可调用。
    pub render_execution_owner_shell_available: bool,

    /// buffer import 是否可用；Phase 54Q 固定为 false。
    pub buffer_import_available: bool,

    /// texture creation 是否可用；Phase 54Q 固定为 false。
    pub texture_creation_available: bool,

    /// renderer call 是否可用；Phase 54Q 固定为 false。
    pub renderer_call_available: bool,

    /// damage submit 是否可用；Phase 54Q 固定为 false。
    pub damage_submit_available: bool,

    /// frame callback done 是否可用；Phase 54Q 固定为 false。
    pub frame_callback_done_available: bool,

    /// 是否 import buffer；Phase 54Q 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 54Q 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 54Q 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 54Q 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 54Q 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 54Q 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 54Q 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderExecutionOwnerShellOperation>,

    /// 阻止进入真实 render execution 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker>,
}

/// Runtime-owned render execution owner shell；不持有真实 renderer 或 core state。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRenderExecutionOwnerShell;

impl RuntimeSurfaceCommitRenderExecutionOwnerShell {
    /// 创建 runtime-owned render execution owner shell readiness 边界。
    pub fn new() -> Self {
        Self
    }

    /// 从 render execution owner boundary report 派生 shell readiness report；不触发真实 render。
    pub fn render_execution_owner_shell_readiness_from_owner_boundary(
        &mut self,
        report: &RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport,
    ) -> RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport {
        render_execution_owner_shell_readiness_from_owner_boundary(report)
    }
}

/// 从 render execution owner boundary report 派生 shell readiness report；不触发真实 render。
pub fn render_execution_owner_shell_readiness_from_owner_boundary(
    report: &RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport,
) -> RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport {
    let observed_intent = report.consumed_intent.clone();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers.push(
            RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingRenderOperationIntent,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingBufferImport,
        RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingRendererCall,
        RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport {
        readiness_invoked: true,
        owner_boundary_report_observed: report.owner_boundary_defined,
        owner_boundary_render_operation_intent_consumed: report.render_operation_intent_consumed,
        observed_intent,
        render_execution_owner_shell_available: true,
        buffer_import_available: false,
        texture_creation_available: false,
        renderer_call_available: false,
        damage_submit_available: false,
        frame_callback_done_available: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRenderExecutionOwnerShellOperation::ObserveRenderExecutionOwnerBoundaryReport,
            RuntimeSurfaceCommitRenderExecutionOwnerShellOperation::BindRenderExecutionOwnerShell,
            RuntimeSurfaceCommitRenderExecutionOwnerShellOperation::BuildReadinessReport,
        ],
        blockers,
    }
}

/// Basic render pipeline skeleton 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderPipelineSkeletonOperation {
    /// 读取 render execution owner shell readiness report。
    ObserveRenderExecutionOwnerShellReadiness,
    /// 绑定 runtime-owned render pipeline skeleton owner。
    BindRenderPipelineSkeletonOwner,
    /// 生成 render pipeline skeleton readiness report。
    BuildReadinessReport,
}

/// Basic render pipeline skeleton 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderPipelineSkeletonBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// 上游 render execution owner shell 尚未可用。
    MissingRenderExecutionOwnerShell,
    /// buffer import 尚未接入。
    MissingBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned basic render pipeline skeleton readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport {
    /// 本轮是否执行 render pipeline skeleton seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 render execution owner shell report。
    pub source_render_execution_owner_shell_report_observed: bool,

    /// 上游 render execution owner shell 是否可用。
    pub source_render_execution_owner_shell_available: bool,

    /// 上游 shell 是否观察到 render operation intent。
    pub source_render_operation_intent_observed: bool,

    /// 从上游 shell 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned renderer pipeline skeleton owner 是否可用；不代表真实 renderer 可调用。
    pub renderer_pipeline_owner_available: bool,

    /// 是否 import buffer；Phase 55A 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 55A 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 55A 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 55A 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 55A 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 55A 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 55A 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderPipelineSkeletonOperation>,

    /// 阻止进入真实 render pipeline 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderPipelineSkeletonBlocker>,
}

/// Runtime-owned render pipeline skeleton owner；不持有真实 renderer、buffer、texture 或 core state。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRenderPipelineSkeletonOwner;

impl RuntimeSurfaceCommitRenderPipelineSkeletonOwner {
    /// 创建 runtime-owned render pipeline skeleton owner。
    pub fn new() -> Self {
        Self
    }

    /// 从 render execution owner shell readiness 派生 skeleton readiness report；不执行 render。
    pub fn render_pipeline_skeleton_readiness_from_execution_owner_shell(
        &mut self,
        report: &RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport,
    ) -> RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport {
        render_pipeline_skeleton_readiness_from_execution_owner_shell(report)
    }
}

/// 从 render execution owner shell readiness 派生 skeleton readiness report；不执行 render。
pub fn render_pipeline_skeleton_readiness_from_execution_owner_shell(
    report: &RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport,
) -> RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport {
    let observed_intent = report.observed_intent.clone();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers
            .push(RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingRenderOperationIntent);
    }
    if !report.render_execution_owner_shell_available {
        blockers.push(
            RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingRenderExecutionOwnerShell,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingBufferImport,
        RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingRendererCall,
        RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport {
        readiness_invoked: true,
        source_render_execution_owner_shell_report_observed: report.readiness_invoked,
        source_render_execution_owner_shell_available: report.render_execution_owner_shell_available,
        source_render_operation_intent_observed: observed_intent.is_some(),
        observed_intent,
        renderer_pipeline_owner_available: true,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRenderPipelineSkeletonOperation::ObserveRenderExecutionOwnerShellReadiness,
            RuntimeSurfaceCommitRenderPipelineSkeletonOperation::BindRenderPipelineSkeletonOwner,
            RuntimeSurfaceCommitRenderPipelineSkeletonOperation::BuildReadinessReport,
        ],
        blockers,
    }
}

/// Phase 55B render backend capability report 可识别的后端种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderBackendKind {
    /// 未来 Linux/Smithay renderer backend；Phase 55B 只声明枚举，不注册实例。
    SmithayLinux,
}

/// Render backend capability report 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderBackendCapabilityOperation {
    /// 读取 render pipeline skeleton readiness report。
    ObserveRenderPipelineSkeletonReadiness,
    /// 绑定 runtime-owned render backend capability owner。
    BindRenderBackendCapabilityOwner,
    /// 构建 backend capability inventory。
    BuildBackendCapabilityInventory,
    /// 生成 render backend capability report。
    BuildCapabilityReport,
}

/// Render backend capability report 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRenderBackendCapabilityBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// 上游 render pipeline skeleton 尚未可用。
    MissingRenderPipelineSkeleton,
    /// 尚未注册真实 renderer backend。
    MissingRendererBackendRegistration,
    /// buffer import 尚未接入。
    MissingBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned render backend capability 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRenderBackendCapabilityReport {
    /// 本轮是否执行 render backend capability seam。
    pub report_invoked: bool,

    /// 是否观察到上游 render pipeline skeleton report。
    pub source_render_pipeline_skeleton_report_observed: bool,

    /// 上游 render pipeline skeleton owner 是否可用。
    pub source_renderer_pipeline_owner_available: bool,

    /// 上游 skeleton 是否观察到 render operation intent。
    pub source_render_operation_intent_observed: bool,

    /// 从上游 skeleton 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned render backend capability owner 是否可用；不代表 renderer backend 已注册。
    pub render_backend_capability_owner_available: bool,

    /// 是否已注册真实 renderer backend；Phase 55B 固定为 false。
    pub renderer_backend_registered: bool,

    /// 已注册 renderer backend 种类；Phase 55B 固定为 None。
    pub renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 是否 import buffer；Phase 55B 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 55B 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 55B 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 55B 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 55B 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 55B 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 55B 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRenderBackendCapabilityOperation>,

    /// 阻止进入真实 render backend 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRenderBackendCapabilityBlocker>,
}

/// Runtime-owned render backend capability owner；不持有真实 renderer、buffer、texture 或 core state。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRenderBackendCapabilityOwner;

impl RuntimeSurfaceCommitRenderBackendCapabilityOwner {
    /// 创建 runtime-owned render backend capability owner。
    pub fn new() -> Self {
        Self
    }

    /// 从 render pipeline skeleton readiness 派生 backend capability report；不执行 render。
    pub fn render_backend_capability_report_from_pipeline_skeleton(
        &mut self,
        report: &RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport,
    ) -> RuntimeSurfaceCommitRenderBackendCapabilityReport {
        render_backend_capability_report_from_pipeline_skeleton(report)
    }
}

/// 从 render pipeline skeleton readiness 派生 backend capability report；不执行 render。
pub fn render_backend_capability_report_from_pipeline_skeleton(
    report: &RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport,
) -> RuntimeSurfaceCommitRenderBackendCapabilityReport {
    let observed_intent = report.observed_intent.clone();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers
            .push(RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRenderOperationIntent);
    }
    if !report.renderer_pipeline_owner_available {
        blockers.push(
            RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRenderPipelineSkeleton,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRendererBackendRegistration,
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingBufferImport,
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRendererCall,
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitRenderBackendCapabilityReport {
        report_invoked: true,
        source_render_pipeline_skeleton_report_observed: report.readiness_invoked,
        source_renderer_pipeline_owner_available: report.renderer_pipeline_owner_available,
        source_render_operation_intent_observed: observed_intent.is_some(),
        observed_intent,
        render_backend_capability_owner_available: true,
        renderer_backend_registered: false,
        renderer_backend_kind: None,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRenderBackendCapabilityOperation::ObserveRenderPipelineSkeletonReadiness,
            RuntimeSurfaceCommitRenderBackendCapabilityOperation::BindRenderBackendCapabilityOwner,
            RuntimeSurfaceCommitRenderBackendCapabilityOperation::BuildBackendCapabilityInventory,
            RuntimeSurfaceCommitRenderBackendCapabilityOperation::BuildCapabilityReport,
        ],
        blockers,
    }
}

/// Renderer backend registration descriptor seam 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererBackendRegistrationOperation {
    /// 读取上游 render backend capability report。
    ObserveRenderBackendCapabilityReport,
    /// 绑定 runtime-owned renderer backend registration owner。
    BindRendererBackendRegistrationOwner,
    /// 注册 renderer backend descriptor。
    RegisterRendererBackendDescriptor,
    /// 生成 renderer backend registration report。
    BuildRegistrationReport,
}

/// Renderer backend registration descriptor seam 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererBackendRegistrationBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// 上游 render backend capability owner 尚未可用。
    MissingRenderBackendCapabilityOwner,
    /// buffer import 尚未接入。
    MissingBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned renderer backend registration descriptor 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererBackendRegistrationReport {
    /// 本轮是否执行 renderer backend registration descriptor seam。
    pub registration_invoked: bool,

    /// 是否观察到上游 render backend capability report。
    pub source_render_backend_capability_report_observed: bool,

    /// 上游 render backend capability owner 是否可用。
    pub source_render_backend_capability_owner_available: bool,

    /// 上游 capability report 是否已注册 renderer backend。
    pub source_renderer_backend_registered: bool,

    /// 从上游 capability report 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned renderer backend registration owner 是否可用。
    pub renderer_backend_registration_owner_available: bool,

    /// 是否注册 renderer backend descriptor；不代表真实 renderer 已可调用。
    pub renderer_backend_descriptor_available: bool,

    /// 是否注册 renderer backend descriptor；Phase 55C 可为 true，但仍不是真实 renderer call。
    pub renderer_backend_registered: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 是否 import buffer；Phase 55C 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 55C 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 55C 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 55C 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 55C 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 55C 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 55C 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRendererBackendRegistrationOperation>,

    /// 阻止进入真实 renderer backend resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRendererBackendRegistrationBlocker>,
}

/// Runtime-owned renderer backend registration owner；只注册 descriptor，不持有 renderer。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRendererBackendRegistrationOwner;

impl RuntimeSurfaceCommitRendererBackendRegistrationOwner {
    /// 创建 runtime-owned renderer backend registration owner。
    pub fn new() -> Self {
        Self
    }

    /// 从 render backend capability report 派生 registration descriptor report；不执行 render。
    pub fn renderer_backend_registration_report_from_backend_capability(
        &mut self,
        report: &RuntimeSurfaceCommitRenderBackendCapabilityReport,
    ) -> RuntimeSurfaceCommitRendererBackendRegistrationReport {
        renderer_backend_registration_report_from_backend_capability(report)
    }
}

/// 从 render backend capability report 派生 registration descriptor report；不执行 render。
pub fn renderer_backend_registration_report_from_backend_capability(
    report: &RuntimeSurfaceCommitRenderBackendCapabilityReport,
) -> RuntimeSurfaceCommitRendererBackendRegistrationReport {
    let observed_intent = report.observed_intent.clone();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingRenderOperationIntent,
        );
    }
    if !report.render_backend_capability_owner_available {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingRenderBackendCapabilityOwner,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingBufferImport,
        RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingRendererCall,
        RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitRendererBackendRegistrationReport {
        registration_invoked: true,
        source_render_backend_capability_report_observed: report.report_invoked,
        source_render_backend_capability_owner_available: report
            .render_backend_capability_owner_available,
        source_renderer_backend_registered: report.renderer_backend_registered,
        observed_intent,
        renderer_backend_registration_owner_available: true,
        renderer_backend_descriptor_available: true,
        renderer_backend_registered: true,
        registered_renderer_backend_kind: Some(RuntimeSurfaceCommitRenderBackendKind::SmithayLinux),
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRendererBackendRegistrationOperation::ObserveRenderBackendCapabilityReport,
            RuntimeSurfaceCommitRendererBackendRegistrationOperation::BindRendererBackendRegistrationOwner,
            RuntimeSurfaceCommitRendererBackendRegistrationOperation::RegisterRendererBackendDescriptor,
            RuntimeSurfaceCommitRendererBackendRegistrationOperation::BuildRegistrationReport,
        ],
        blockers,
    }
}

/// Renderer backend owner shell readiness seam 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererBackendOwnerShellOperation {
    /// 读取上游 renderer backend registration descriptor report。
    ObserveRendererBackendRegistrationReport,
    /// 绑定 runtime-owned renderer backend owner shell。
    BindRendererBackendOwnerShell,
    /// 观察已注册 renderer backend descriptor。
    ObserveRendererBackendDescriptor,
    /// 生成 renderer backend owner shell readiness report。
    BuildOwnerShellReadinessReport,
}

/// Renderer backend owner shell readiness seam 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererBackendOwnerShellBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// 上游 renderer backend descriptor 尚未可用。
    MissingRendererBackendDescriptor,
    /// buffer import 尚未接入。
    MissingBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned renderer backend owner shell readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport {
    /// 本轮是否执行 renderer backend owner shell readiness seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 renderer backend registration report。
    pub source_renderer_backend_registration_report_observed: bool,

    /// 上游 renderer backend descriptor 是否可用。
    pub source_renderer_backend_descriptor_available: bool,

    /// 上游 renderer backend descriptor 是否已注册。
    pub source_renderer_backend_registered: bool,

    /// 从上游 registration report 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned renderer backend owner shell 是否可用。
    pub renderer_backend_owner_shell_available: bool,

    /// renderer backend owner shell 是否已绑定 descriptor；不代表真实 renderer 已可调用。
    pub renderer_backend_owner_shell_bound: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 是否 import buffer；Phase 55D 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 55D 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 55D 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 55D 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 55D 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 55D 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 55D 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitRendererBackendOwnerShellOperation>,

    /// 阻止进入真实 renderer backend resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitRendererBackendOwnerShellBlocker>,
}

/// Runtime-owned renderer backend owner shell；只绑定 descriptor，不持有真实 renderer。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitRendererBackendOwnerShell;

impl RuntimeSurfaceCommitRendererBackendOwnerShell {
    /// 创建 runtime-owned renderer backend owner shell。
    pub fn new() -> Self {
        Self
    }

    /// 从 renderer backend registration descriptor 派生 owner shell readiness；不执行 render。
    pub fn renderer_backend_owner_shell_readiness_from_registration(
        &mut self,
        report: &RuntimeSurfaceCommitRendererBackendRegistrationReport,
    ) -> RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport {
        renderer_backend_owner_shell_readiness_from_registration(report)
    }
}

/// 从 renderer backend registration descriptor 派生 owner shell readiness；不执行 render。
pub fn renderer_backend_owner_shell_readiness_from_registration(
    report: &RuntimeSurfaceCommitRendererBackendRegistrationReport,
) -> RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport {
    let observed_intent = report.observed_intent.clone();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingRenderOperationIntent,
        );
    }
    if !report.renderer_backend_descriptor_available || !report.renderer_backend_registered {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingRendererBackendDescriptor,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingBufferImport,
        RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingRendererCall,
        RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport {
        readiness_invoked: true,
        source_renderer_backend_registration_report_observed: report.registration_invoked,
        source_renderer_backend_descriptor_available: report.renderer_backend_descriptor_available,
        source_renderer_backend_registered: report.renderer_backend_registered,
        observed_intent,
        renderer_backend_owner_shell_available: true,
        renderer_backend_owner_shell_bound: true,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRendererBackendOwnerShellOperation::ObserveRendererBackendRegistrationReport,
            RuntimeSurfaceCommitRendererBackendOwnerShellOperation::BindRendererBackendOwnerShell,
            RuntimeSurfaceCommitRendererBackendOwnerShellOperation::ObserveRendererBackendDescriptor,
            RuntimeSurfaceCommitRendererBackendOwnerShellOperation::BuildOwnerShellReadinessReport,
        ],
        blockers,
    }
}

/// Buffer import resource owner boundary 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportResourceOwnerOperation {
    /// 读取上游 renderer backend owner shell readiness report。
    ObserveRendererBackendOwnerShellReadiness,
    /// 绑定 runtime-owned buffer importer resource owner boundary。
    BindBufferImporterResourceOwner,
    /// 观察将来 buffer import 所需的 commit/render evidence。
    ObserveBufferImportEvidence,
    /// 生成 buffer importer owner readiness report。
    BuildBufferImportResourceOwnerReadinessReport,
}

/// Buffer import resource owner boundary 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportResourceOwnerBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// 上游 renderer backend owner shell 尚未可用或绑定。
    MissingRendererBackendOwnerShell,
    /// renderer backend descriptor evidence 尚未可用。
    MissingRendererBackendDescriptorEvidence,
    /// 真实 buffer import implementation 尚未接入。
    MissingActualBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer importer resource owner readiness 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport {
    /// 本轮是否执行 buffer import resource owner boundary seam。
    pub readiness_invoked: bool,

    /// 是否观察到上游 renderer backend owner shell readiness report。
    pub source_renderer_backend_owner_shell_readiness_observed: bool,

    /// 上游 renderer backend owner shell 是否可用。
    pub source_renderer_backend_owner_shell_available: bool,

    /// 上游 renderer backend owner shell 是否已绑定 descriptor。
    pub source_renderer_backend_owner_shell_bound: bool,

    /// 从上游 owner shell report 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned buffer importer owner boundary 是否可用。
    pub buffer_importer_owner_available: bool,

    /// buffer importer owner 是否已绑定 renderer backend owner shell；不代表真实 import 已执行。
    pub buffer_importer_owner_bound: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 是否 import buffer；Phase 55E 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 55E 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 55E 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 55E 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 55E 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 55E 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 55E 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportResourceOwnerOperation>,

    /// 阻止进入真实 buffer import / render resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportResourceOwnerBlocker>,
}

/// Runtime-owned buffer importer resource owner boundary；只记录 handoff evidence。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportResourceOwnerBoundary;

impl RuntimeSurfaceCommitBufferImportResourceOwnerBoundary {
    /// 创建 runtime-owned buffer importer resource owner boundary。
    pub fn new() -> Self {
        Self
    }

    /// 从 renderer backend owner shell readiness 派生 buffer importer owner readiness；不 import。
    pub fn buffer_import_resource_owner_readiness_from_renderer_backend_owner_shell(
        &mut self,
        report: &RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport,
    ) -> RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport {
        buffer_import_resource_owner_readiness_from_renderer_backend_owner_shell(report)
    }
}

/// 从 renderer backend owner shell readiness 派生 buffer importer owner readiness；不 import。
pub fn buffer_import_resource_owner_readiness_from_renderer_backend_owner_shell(
    report: &RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport,
) -> RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport {
    let observed_intent = report.observed_intent.clone();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers.push(
            RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRenderOperationIntent,
        );
    }
    if !report.renderer_backend_owner_shell_available || !report.renderer_backend_owner_shell_bound
    {
        blockers.push(
            RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRendererBackendOwnerShell,
        );
    }
    if !report.source_renderer_backend_descriptor_available {
        blockers.push(
            RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRendererBackendDescriptorEvidence,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingActualBufferImport,
        RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport {
        readiness_invoked: true,
        source_renderer_backend_owner_shell_readiness_observed: report.readiness_invoked,
        source_renderer_backend_owner_shell_available: report
            .renderer_backend_owner_shell_available,
        source_renderer_backend_owner_shell_bound: report.renderer_backend_owner_shell_bound,
        observed_intent,
        buffer_importer_owner_available: true,
        buffer_importer_owner_bound: true,
        renderer_backend_descriptor_evidence_available: report
            .source_renderer_backend_descriptor_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportResourceOwnerOperation::ObserveRendererBackendOwnerShellReadiness,
            RuntimeSurfaceCommitBufferImportResourceOwnerOperation::BindBufferImporterResourceOwner,
            RuntimeSurfaceCommitBufferImportResourceOwnerOperation::ObserveBufferImportEvidence,
            RuntimeSurfaceCommitBufferImportResourceOwnerOperation::BuildBufferImportResourceOwnerReadinessReport,
        ],
        blockers,
    }
}

/// Buffer import planning seam 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportPlanningOperation {
    /// 读取上游 buffer import resource owner readiness report。
    ObserveBufferImportResourceOwnerReadiness,
    /// 建立 runtime-owned buffer import plan。
    BuildBufferImportPlan,
    /// 观察 commit 中是否有 buffer import candidate evidence。
    ObserveBufferImportCandidateEvidence,
    /// 生成 buffer import planning report。
    BuildBufferImportPlanningReport,
}

/// Buffer import planning seam 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportPlanningBlocker {
    /// 本轮没有 render operation intent 可观察。
    MissingRenderOperationIntent,
    /// 上游 buffer importer owner boundary 尚未可用或绑定。
    MissingBufferImporterOwner,
    /// renderer backend descriptor evidence 尚未可用。
    MissingRendererBackendDescriptorEvidence,
    /// 本轮 commit 没有 buffer import candidate evidence。
    MissingBufferImportCandidate,
    /// 真实 buffer import implementation 尚未接入。
    MissingActualBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer import planning 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportPlanningReport {
    /// 本轮是否执行 buffer import planning seam。
    pub planning_invoked: bool,

    /// 是否观察到上游 buffer import resource owner readiness report。
    pub source_buffer_import_resource_owner_readiness_observed: bool,

    /// 上游 buffer importer owner boundary 是否可用。
    pub source_buffer_importer_owner_available: bool,

    /// 上游 buffer importer owner boundary 是否已绑定。
    pub source_buffer_importer_owner_bound: bool,

    /// 从上游 owner boundary report 观察到的 render operation intent。
    pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>,

    /// buffer import plan seam 是否可用；不代表真实 import 已执行。
    pub buffer_import_plan_available: bool,

    /// 是否已为本轮 observed intent 建立 pure-data plan。
    pub buffer_import_plan_built: bool,

    /// 是否观察到 buffer attach/presence candidate evidence。
    pub buffer_import_candidate_observed: bool,

    /// 是否计划未来真实 buffer import；当前仍不会执行 import。
    pub buffer_import_required: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 是否 import buffer；Phase 55F 固定为 false。
    pub buffer_imported: bool,

    /// 是否创建 texture；Phase 55F 固定为 false。
    pub texture_created: bool,

    /// 是否调用 renderer；Phase 55F 固定为 false。
    pub renderer_called: bool,

    /// 是否提交 damage；Phase 55F 固定为 false。
    pub damage_submitted: bool,

    /// 是否发送 frame callback done；Phase 55F 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 是否接入 input；Phase 55F 固定为 false。
    pub input_support: bool,

    /// 是否触发 core mutation；Phase 55F 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportPlanningOperation>,

    /// 阻止进入真实 buffer import / render resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportPlanningBlocker>,
}

/// Runtime-owned buffer import planner；只生成 planning report，不 import buffer。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportPlanner;

impl RuntimeSurfaceCommitBufferImportPlanner {
    /// 创建 runtime-owned buffer import planner。
    pub fn new() -> Self {
        Self
    }

    /// 从 buffer importer owner boundary 派生 planning report；不 import buffer。
    pub fn buffer_import_planning_report_from_resource_owner_boundary(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport,
    ) -> RuntimeSurfaceCommitBufferImportPlanningReport {
        buffer_import_planning_report_from_resource_owner_boundary(report)
    }
}

/// 从 buffer importer owner boundary 派生 planning report；不 import buffer。
pub fn buffer_import_planning_report_from_resource_owner_boundary(
    report: &RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport,
) -> RuntimeSurfaceCommitBufferImportPlanningReport {
    let observed_intent = report.observed_intent.clone();
    let buffer_import_candidate_observed = observed_intent
        .as_ref()
        .is_some_and(|intent| intent.buffer_attach_observed || intent.buffer_present);
    let buffer_import_required = observed_intent.as_ref().is_some_and(|intent| {
        (intent.buffer_attach_observed || intent.buffer_present) && !intent.buffer_removed
    });
    let buffer_import_plan_built = observed_intent.is_some();
    let mut blockers = Vec::new();
    if observed_intent.is_none() {
        blockers
            .push(RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingRenderOperationIntent);
    }
    if !report.buffer_importer_owner_available || !report.buffer_importer_owner_bound {
        blockers.push(RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingBufferImporterOwner);
    }
    if !report.renderer_backend_descriptor_evidence_available {
        blockers.push(
            RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingRendererBackendDescriptorEvidence,
        );
    }
    if !buffer_import_candidate_observed {
        blockers
            .push(RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingBufferImportCandidate);
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingActualBufferImport,
        RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportPlanningReport {
        planning_invoked: true,
        source_buffer_import_resource_owner_readiness_observed: report.readiness_invoked,
        source_buffer_importer_owner_available: report.buffer_importer_owner_available,
        source_buffer_importer_owner_bound: report.buffer_importer_owner_bound,
        observed_intent,
        buffer_import_plan_available: true,
        buffer_import_plan_built,
        buffer_import_candidate_observed,
        buffer_import_required,
        renderer_backend_descriptor_evidence_available: report
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportPlanningOperation::ObserveBufferImportResourceOwnerReadiness,
            RuntimeSurfaceCommitBufferImportPlanningOperation::BuildBufferImportPlan,
            RuntimeSurfaceCommitBufferImportPlanningOperation::ObserveBufferImportCandidateEvidence,
            RuntimeSurfaceCommitBufferImportPlanningOperation::BuildBufferImportPlanningReport,
        ],
        blockers,
    }
}

/// 未来真实 buffer import implementation 的最小纯数据描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportImplementationDescriptor {
    /// adapter-owned surface identity；不会手写 core `WindowId`。
    pub adapter_surface_id: AdapterSurfaceId,

    /// adapter-owned surface identity key。
    pub surface_identity_key: SurfaceIdentityKey,

    /// 对应的 wl_surface.commit FIFO sequence。
    pub commit_sequence: u64,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带 present buffer evidence。
    pub buffer_present: bool,

    /// commit 是否携带 null attach / remove evidence。
    pub buffer_removed: bool,

    /// 是否观察到未来 importer 可消费的 candidate evidence。
    pub candidate_evidence_observed: bool,

    /// 是否计划未来真实 buffer import；candidate evidence 不等于实际 import。
    pub actual_import_required: bool,

    /// renderer backend descriptor evidence 是否存在。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// importer owner boundary evidence 是否存在。
    pub importer_owner_evidence_available: bool,

    /// 本阶段是否尝试 import buffer；Phase 55G 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55G 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55G 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55G 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55G 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55G 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55G 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55G 固定为 false。
    pub core_mutation_invoked: bool,
}

/// Buffer import implementation descriptor boundary 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportImplementationOperation {
    /// 读取上游 buffer import planning report。
    ObserveBufferImportPlanningReport,
    /// 注册未来真实 importer 的最小 descriptor。
    RegisterImplementationDescriptor,
    /// 观察 candidate evidence 与 actual import required 口径。
    ObserveCandidateAndRequirementEvidence,
    /// 生成 implementation boundary report。
    BuildImplementationBoundaryReport,
}

/// Buffer import implementation descriptor boundary 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportImplementationBlocker {
    /// 上游 planning report 未提供 observed intent。
    MissingBufferImportPlanningIntent,
    /// 上游 planning report 未建成 pure-data plan。
    MissingBufferImportPlan,
    /// 上游 planning report 缺少 importer owner evidence。
    MissingImporterOwnerEvidence,
    /// 上游 planning report 缺少 renderer backend descriptor evidence。
    MissingRendererBackendDescriptorEvidence,
    /// 本轮 commit 没有 buffer import candidate evidence。
    MissingBufferImportCandidate,
    /// 真实 buffer import implementation 尚未执行。
    MissingActualBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer import implementation descriptor / adapter boundary 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportImplementationBoundaryReport {
    /// 本轮是否执行 implementation descriptor boundary seam。
    pub boundary_invoked: bool,

    /// 是否观察到 Phase 55F buffer import planning report。
    pub source_buffer_import_planning_report_observed: bool,

    /// 上游 planning report 是否已建成 pure-data plan。
    pub source_buffer_import_plan_built: bool,

    /// 上游 importer owner evidence 是否可用。
    pub importer_owner_evidence_available: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// descriptor boundary 是否可用；不代表真实 import 已执行。
    pub implementation_descriptor_available: bool,

    /// 是否为 observed planning intent 注册 descriptor。
    pub implementation_descriptor_registered: bool,

    /// 未来真实 importer 的最小 descriptor。
    pub descriptor: Option<RuntimeSurfaceCommitBufferImportImplementationDescriptor>,

    /// 是否观察到 candidate evidence；candidate evidence 不等于 actual import execution。
    pub candidate_evidence_observed: bool,

    /// 是否计划未来真实 import；本阶段仍不执行 import。
    pub actual_import_required: bool,

    /// 本阶段是否尝试 import buffer；Phase 55G 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55G 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55G 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55G 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55G 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55G 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55G 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55G 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportImplementationOperation>,

    /// 阻止进入真实 buffer import / render resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportImplementationBlocker>,
}

/// Runtime-owned buffer import implementation boundary；只注册未来 importer descriptor。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportImplementationBoundary;

impl RuntimeSurfaceCommitBufferImportImplementationBoundary {
    /// 创建 runtime-owned buffer import implementation descriptor boundary。
    pub fn new() -> Self {
        Self
    }

    /// 从 Phase 55F planning report 派生 implementation descriptor report；不 import buffer。
    pub fn buffer_import_implementation_boundary_report_from_planning_report(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportPlanningReport,
    ) -> RuntimeSurfaceCommitBufferImportImplementationBoundaryReport {
        buffer_import_implementation_boundary_report_from_planning_report(report)
    }
}

/// 从 Phase 55F planning report 派生 implementation descriptor report；不 import buffer。
pub fn buffer_import_implementation_boundary_report_from_planning_report(
    report: &RuntimeSurfaceCommitBufferImportPlanningReport,
) -> RuntimeSurfaceCommitBufferImportImplementationBoundaryReport {
    let importer_owner_evidence_available =
        report.source_buffer_importer_owner_available && report.source_buffer_importer_owner_bound;
    let descriptor = report.observed_intent.as_ref().map(|intent| {
        RuntimeSurfaceCommitBufferImportImplementationDescriptor {
            adapter_surface_id: intent.adapter_surface_id,
            surface_identity_key: intent.surface_identity_key,
            commit_sequence: intent.commit_sequence,
            buffer_attach_observed: intent.buffer_attach_observed,
            buffer_present: intent.buffer_present,
            buffer_removed: intent.buffer_removed,
            candidate_evidence_observed: report.buffer_import_candidate_observed,
            actual_import_required: report.buffer_import_required,
            renderer_backend_descriptor_evidence_available: report
                .renderer_backend_descriptor_evidence_available,
            registered_renderer_backend_kind: report.registered_renderer_backend_kind,
            importer_owner_evidence_available,
            buffer_import_attempted: false,
            buffer_imported: false,
            texture_created: false,
            renderer_called: false,
            damage_submitted: false,
            frame_callback_done_sent: false,
            input_support: false,
            core_mutation_invoked: false,
        }
    });
    let implementation_descriptor_registered = descriptor.is_some();
    let mut blockers = Vec::new();
    if report.observed_intent.is_none() {
        blockers.push(
            RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingBufferImportPlanningIntent,
        );
    }
    if report.observed_intent.is_some() {
        if !report.buffer_import_plan_built {
            blockers.push(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingBufferImportPlan,
            );
        }
        if !importer_owner_evidence_available {
            blockers.push(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingImporterOwnerEvidence,
            );
        }
        if !report.renderer_backend_descriptor_evidence_available {
            blockers.push(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingRendererBackendDescriptorEvidence,
            );
        }
        if !report.buffer_import_candidate_observed {
            blockers.push(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingBufferImportCandidate,
            );
        }
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingActualBufferImport,
        RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportImplementationBoundaryReport {
        boundary_invoked: true,
        source_buffer_import_planning_report_observed: report.planning_invoked,
        source_buffer_import_plan_built: report.buffer_import_plan_built,
        importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available: report
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        implementation_descriptor_available: true,
        implementation_descriptor_registered,
        descriptor,
        candidate_evidence_observed: report.buffer_import_candidate_observed,
        actual_import_required: report.buffer_import_required,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportImplementationOperation::ObserveBufferImportPlanningReport,
            RuntimeSurfaceCommitBufferImportImplementationOperation::RegisterImplementationDescriptor,
            RuntimeSurfaceCommitBufferImportImplementationOperation::ObserveCandidateAndRequirementEvidence,
            RuntimeSurfaceCommitBufferImportImplementationOperation::BuildImplementationBoundaryReport,
        ],
        blockers,
    }
}

/// 未来真实 buffer import adapter 可证明接收的最小纯数据证据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportAdapterProof {
    /// adapter-owned surface identity；不会手写 core `WindowId`。
    pub adapter_surface_id: AdapterSurfaceId,

    /// adapter-owned surface identity key。
    pub surface_identity_key: SurfaceIdentityKey,

    /// 对应的 wl_surface.commit FIFO sequence。
    pub commit_sequence: u64,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带 present buffer evidence。
    pub buffer_present: bool,

    /// commit 是否携带 null attach / remove evidence。
    pub buffer_removed: bool,

    /// 是否观察到未来 adapter/importer 可消费的 candidate evidence。
    pub candidate_evidence_observed: bool,

    /// 是否计划未来真实 buffer import；本阶段仍不执行 import。
    pub actual_import_required: bool,

    /// renderer backend descriptor evidence 是否存在。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// importer owner boundary evidence 是否存在。
    pub importer_owner_evidence_available: bool,

    /// 上游 implementation descriptor 是否已注册。
    pub implementation_descriptor_registered: bool,

    /// 本阶段是否尝试 import buffer；Phase 55H 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55H 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55H 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55H 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55H 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55H 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55H 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55H 固定为 false。
    pub core_mutation_invoked: bool,
}

/// Buffer import adapter proof boundary 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportAdapterProofOperation {
    /// 读取上游 implementation descriptor boundary report。
    ObserveImplementationBoundaryReport,
    /// 注册未来 adapter/importer handoff 可消费的 pure-data proof。
    RegisterAdapterProof,
    /// 观察 candidate evidence 与 actual import required 口径。
    ObserveCandidateAndRequirementEvidence,
    /// 生成 adapter proof boundary report。
    BuildAdapterProofBoundaryReport,
}

/// Buffer import adapter proof boundary 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportAdapterProofBlocker {
    /// 上游 implementation boundary 没有 descriptor。
    MissingImplementationDescriptor,
    /// 上游 descriptor 缺少 importer owner evidence。
    MissingImporterOwnerEvidence,
    /// 上游 descriptor 缺少 renderer backend descriptor evidence。
    MissingRendererBackendDescriptorEvidence,
    /// 本轮 commit 没有 buffer import candidate evidence。
    MissingBufferImportCandidate,
    /// 真实 buffer import 尚未执行。
    MissingActualBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer import adapter proof boundary 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport {
    /// 本轮是否执行 adapter proof boundary seam。
    pub boundary_invoked: bool,

    /// 是否观察到 Phase 55G implementation descriptor boundary report。
    pub source_buffer_import_implementation_report_observed: bool,

    /// 上游 implementation descriptor 是否注册成功。
    pub implementation_descriptor_registered: bool,

    /// 上游 importer owner evidence 是否可用。
    pub importer_owner_evidence_available: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// adapter proof boundary 是否可用；不代表真实 import 已执行。
    pub adapter_proof_boundary_available: bool,

    /// 是否为 observed implementation descriptor 注册 adapter proof。
    pub adapter_proof_registered: bool,

    /// 未来真实 adapter/importer handoff 的最小 pure-data proof。
    pub adapter_proof: Option<RuntimeSurfaceCommitBufferImportAdapterProof>,

    /// 是否观察到 candidate evidence；candidate evidence 不等于 actual import execution。
    pub candidate_evidence_observed: bool,

    /// 是否计划未来真实 import；本阶段仍不执行 import。
    pub actual_import_required: bool,

    /// 本阶段是否尝试 import buffer；Phase 55H 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55H 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55H 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55H 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55H 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55H 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55H 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55H 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportAdapterProofOperation>,

    /// 阻止进入真实 buffer import / render resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportAdapterProofBlocker>,
}

/// Runtime-owned buffer import adapter proof boundary；只注册 pure-data proof。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportAdapterProofBoundary;

impl RuntimeSurfaceCommitBufferImportAdapterProofBoundary {
    /// 创建 runtime-owned buffer import adapter proof boundary。
    pub fn new() -> Self {
        Self
    }

    /// 从 Phase 55G implementation boundary 派生 adapter proof report；不 import buffer。
    pub fn buffer_import_adapter_proof_boundary_report_from_implementation_report(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportImplementationBoundaryReport,
    ) -> RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport {
        buffer_import_adapter_proof_boundary_report_from_implementation_report(report)
    }
}

/// 从 Phase 55G implementation boundary 派生 adapter proof report；不 import buffer。
pub fn buffer_import_adapter_proof_boundary_report_from_implementation_report(
    report: &RuntimeSurfaceCommitBufferImportImplementationBoundaryReport,
) -> RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport {
    let adapter_proof =
        report
            .descriptor
            .as_ref()
            .map(|descriptor| RuntimeSurfaceCommitBufferImportAdapterProof {
                adapter_surface_id: descriptor.adapter_surface_id,
                surface_identity_key: descriptor.surface_identity_key,
                commit_sequence: descriptor.commit_sequence,
                buffer_attach_observed: descriptor.buffer_attach_observed,
                buffer_present: descriptor.buffer_present,
                buffer_removed: descriptor.buffer_removed,
                candidate_evidence_observed: descriptor.candidate_evidence_observed,
                actual_import_required: descriptor.actual_import_required,
                renderer_backend_descriptor_evidence_available: descriptor
                    .renderer_backend_descriptor_evidence_available,
                registered_renderer_backend_kind: descriptor.registered_renderer_backend_kind,
                importer_owner_evidence_available: descriptor.importer_owner_evidence_available,
                implementation_descriptor_registered: report.implementation_descriptor_registered,
                buffer_import_attempted: false,
                buffer_imported: false,
                texture_created: false,
                renderer_called: false,
                damage_submitted: false,
                frame_callback_done_sent: false,
                input_support: false,
                core_mutation_invoked: false,
            });
    let adapter_proof_registered = adapter_proof.is_some();
    let mut blockers = Vec::new();
    if report.descriptor.is_none() {
        blockers.push(
            RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingImplementationDescriptor,
        );
    }
    if let Some(proof) = adapter_proof.as_ref() {
        if !proof.importer_owner_evidence_available {
            blockers.push(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingImporterOwnerEvidence,
            );
        }
        if !proof.renderer_backend_descriptor_evidence_available {
            blockers.push(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingRendererBackendDescriptorEvidence,
            );
        }
        if !proof.candidate_evidence_observed {
            blockers.push(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingBufferImportCandidate,
            );
        }
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingActualBufferImport,
        RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport {
        boundary_invoked: true,
        source_buffer_import_implementation_report_observed: report.boundary_invoked,
        implementation_descriptor_registered: report.implementation_descriptor_registered,
        importer_owner_evidence_available: report.importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available: report
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        adapter_proof_boundary_available: true,
        adapter_proof_registered,
        adapter_proof,
        candidate_evidence_observed: report.candidate_evidence_observed,
        actual_import_required: report.actual_import_required,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportAdapterProofOperation::ObserveImplementationBoundaryReport,
            RuntimeSurfaceCommitBufferImportAdapterProofOperation::RegisterAdapterProof,
            RuntimeSurfaceCommitBufferImportAdapterProofOperation::ObserveCandidateAndRequirementEvidence,
            RuntimeSurfaceCommitBufferImportAdapterProofOperation::BuildAdapterProofBoundaryReport,
        ],
        blockers,
    }
}

/// Buffer import precondition gate 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportPreconditionGateOperation {
    /// 读取上游 buffer import adapter proof report。
    ObserveAdapterProofBoundaryReport,
    /// 检查 adapter proof 是否包含 present/non-removed buffer evidence。
    CheckPresentBufferEvidence,
    /// 检查 importer owner evidence。
    CheckImporterOwnerEvidence,
    /// 检查 renderer backend descriptor evidence。
    CheckRendererBackendDescriptorEvidence,
    /// 检查 future actual import requirement。
    CheckActualImportRequirement,
    /// 生成 precondition gate report。
    BuildPreconditionGateReport,
}

/// Buffer import precondition gate 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportPreconditionGateBlocker {
    /// 上游 adapter proof report 没有 proof。
    MissingAdapterProof,
    /// 上游 adapter proof 未注册。
    MissingRegisteredAdapterProof,
    /// 上游 proof 缺少 importer owner evidence。
    MissingImporterOwnerEvidence,
    /// 上游 proof 缺少 renderer backend descriptor evidence。
    MissingRendererBackendDescriptorEvidence,
    /// 本轮 commit 没有 buffer import candidate evidence。
    MissingBufferImportCandidate,
    /// 本轮 commit 不需要未来真实 buffer import。
    MissingActualImportRequirement,
    /// 真实 buffer import 尚未执行。
    MissingActualBufferImport,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer import precondition gate 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportPreconditionGateReport {
    /// 本轮是否执行 precondition gate seam。
    pub gate_invoked: bool,

    /// 是否观察到 Phase 55H adapter proof boundary report。
    pub source_buffer_import_adapter_proof_report_observed: bool,

    /// 上游 adapter proof 是否已注册。
    pub adapter_proof_registered: bool,

    /// 上游 adapter proof；只保存 pure data。
    pub observed_adapter_proof: Option<RuntimeSurfaceCommitBufferImportAdapterProof>,

    /// precondition gate 是否可用；不代表真实 import 已执行。
    pub import_precondition_gate_available: bool,

    /// 当前 proof 是否满足未来真实 importer 的最小前置条件。
    pub import_preconditions_met: bool,

    /// 与 `import_preconditions_met` 同义，用于报告“未来 import 可被调度”的意图。
    pub future_import_preconditions_met: bool,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带 present buffer evidence。
    pub buffer_present: bool,

    /// commit 是否携带 null attach / removal evidence。
    pub buffer_removed: bool,

    /// 是否观察到 candidate evidence；candidate evidence 不等于 actual import execution。
    pub candidate_evidence_observed: bool,

    /// 是否计划未来真实 import；本阶段仍不执行 import。
    pub actual_import_required: bool,

    /// importer owner evidence 是否可用。
    pub importer_owner_evidence_available: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 本阶段是否尝试 import buffer；Phase 55I 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55I 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55I 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55I 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55I 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55I 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55I 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55I 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportPreconditionGateOperation>,

    /// 阻止进入真实 buffer import / render resource path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportPreconditionGateBlocker>,
}

/// Runtime-owned buffer import precondition gate；只判断未来 import 前置条件。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportPreconditionGate;

impl RuntimeSurfaceCommitBufferImportPreconditionGate {
    /// 创建 runtime-owned buffer import precondition gate。
    pub fn new() -> Self {
        Self
    }

    /// 从 Phase 55H adapter proof report 派生 precondition gate report；不 import buffer。
    pub fn buffer_import_precondition_gate_report_from_adapter_proof(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport,
    ) -> RuntimeSurfaceCommitBufferImportPreconditionGateReport {
        buffer_import_precondition_gate_report_from_adapter_proof(report)
    }
}

/// 从 Phase 55H adapter proof report 派生 precondition gate report；不 import buffer。
pub fn buffer_import_precondition_gate_report_from_adapter_proof(
    report: &RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport,
) -> RuntimeSurfaceCommitBufferImportPreconditionGateReport {
    let observed_adapter_proof = report.adapter_proof.clone();
    let proof = observed_adapter_proof.as_ref();
    let buffer_attach_observed = proof.is_some_and(|proof| proof.buffer_attach_observed);
    let buffer_present = proof.is_some_and(|proof| proof.buffer_present);
    let buffer_removed = proof.is_some_and(|proof| proof.buffer_removed);
    let candidate_evidence_observed = proof.is_some_and(|proof| proof.candidate_evidence_observed);
    let actual_import_required = proof.is_some_and(|proof| proof.actual_import_required);
    let importer_owner_evidence_available =
        proof.is_some_and(|proof| proof.importer_owner_evidence_available);
    let renderer_backend_descriptor_evidence_available =
        proof.is_some_and(|proof| proof.renderer_backend_descriptor_evidence_available);
    let registered_renderer_backend_kind =
        proof.and_then(|proof| proof.registered_renderer_backend_kind);
    let import_preconditions_met = report.adapter_proof_registered
        && candidate_evidence_observed
        && actual_import_required
        && buffer_present
        && !buffer_removed
        && importer_owner_evidence_available
        && renderer_backend_descriptor_evidence_available;
    let future_import_preconditions_met = import_preconditions_met;
    let mut blockers = Vec::new();
    if observed_adapter_proof.is_none() {
        blockers.push(RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingAdapterProof);
    }
    if !report.adapter_proof_registered {
        blockers.push(
            RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingRegisteredAdapterProof,
        );
    }
    if proof.is_some() {
        if !importer_owner_evidence_available {
            blockers.push(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingImporterOwnerEvidence,
            );
        }
        if !renderer_backend_descriptor_evidence_available {
            blockers.push(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingRendererBackendDescriptorEvidence,
            );
        }
        if !candidate_evidence_observed {
            blockers.push(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingBufferImportCandidate,
            );
        }
        if !actual_import_required || !buffer_present || buffer_removed {
            blockers.push(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingActualImportRequirement,
            );
        }
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingActualBufferImport,
        RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportPreconditionGateReport {
        gate_invoked: true,
        source_buffer_import_adapter_proof_report_observed: report.boundary_invoked,
        adapter_proof_registered: report.adapter_proof_registered,
        observed_adapter_proof,
        import_precondition_gate_available: true,
        import_preconditions_met,
        future_import_preconditions_met,
        buffer_attach_observed,
        buffer_present,
        buffer_removed,
        candidate_evidence_observed,
        actual_import_required,
        importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportPreconditionGateOperation::ObserveAdapterProofBoundaryReport,
            RuntimeSurfaceCommitBufferImportPreconditionGateOperation::CheckPresentBufferEvidence,
            RuntimeSurfaceCommitBufferImportPreconditionGateOperation::CheckImporterOwnerEvidence,
            RuntimeSurfaceCommitBufferImportPreconditionGateOperation::CheckRendererBackendDescriptorEvidence,
            RuntimeSurfaceCommitBufferImportPreconditionGateOperation::CheckActualImportRequirement,
            RuntimeSurfaceCommitBufferImportPreconditionGateOperation::BuildPreconditionGateReport,
        ],
        blockers,
    }
}

/// Buffer import execution dry-run 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportExecutionOperation {
    /// 读取上游 buffer import precondition gate report。
    ObservePreconditionGateReport,
    /// 检查是否需要真实 import。
    CheckActualImportRequirement,
    /// 检查是否具备真实 importer implementation。
    CheckRealBufferImportImplementation,
    /// 生成 no-op execution guard report。
    BuildNoopExecutionReport,
}

/// Buffer import execution dry-run 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportExecutionBlocker {
    /// 上游 precondition gate report 未被观察到。
    MissingPreconditionGateEvidence,
    /// 上游 precondition gate 没有 adapter proof。
    MissingAdapterProof,
    /// 上游 precondition gate 未满足未来真实 import 前置条件。
    MissingImportPreconditions,
    /// 本轮 commit 不需要真实 buffer import。
    NoActualImportRequired,
    /// 尚无真实 buffer import implementation。
    MissingRealBufferImportImplementation,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer import execution dry-run / no-op guard 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportExecutionDryRunReport {
    /// 本轮是否执行 execution dry-run seam。
    pub dry_run_invoked: bool,

    /// 是否观察到 Phase 55I precondition gate report。
    pub source_buffer_import_precondition_gate_report_observed: bool,

    /// 上游 adapter proof；只保存 pure data。
    pub observed_adapter_proof: Option<RuntimeSurfaceCommitBufferImportAdapterProof>,

    /// execution guard 是否可用；不代表真实 import 已执行。
    pub execution_guard_available: bool,

    /// 本阶段是否尝试真实 execution；Phase 55J 固定为 false。
    pub execution_attempted: bool,

    /// 本阶段是否显式选择 no-op execution。
    pub execution_noop: bool,

    /// 本阶段 execution 是否被 blocked。
    pub execution_blocked: bool,

    /// 上游 precondition gate 是否满足。
    pub import_preconditions_met: bool,

    /// 上游 future precondition gate 是否满足。
    pub future_import_preconditions_met: bool,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带 present buffer evidence。
    pub buffer_present: bool,

    /// commit 是否携带 null attach / removal evidence。
    pub buffer_removed: bool,

    /// 是否观察到 candidate evidence。
    pub candidate_evidence_observed: bool,

    /// 是否计划未来真实 import；本阶段仍不执行 import。
    pub actual_import_required: bool,

    /// importer owner evidence 是否可用。
    pub importer_owner_evidence_available: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 本阶段是否尝试 import buffer；Phase 55J 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55J 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55J 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55J 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55J 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55J 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55J 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55J 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportExecutionOperation>,

    /// 阻止进入真实 buffer import execution path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportExecutionBlocker>,
}

/// Runtime-owned buffer import execution dry-run；只生成 blocked/no-op report。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportExecutionDryRun;

impl RuntimeSurfaceCommitBufferImportExecutionDryRun {
    /// 创建 runtime-owned buffer import execution dry-run guard。
    pub fn new() -> Self {
        Self
    }

    /// 从 Phase 55I precondition gate report 派生 execution dry-run report；不 import buffer。
    pub fn buffer_import_execution_dry_run_report_from_precondition_gate(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportPreconditionGateReport,
    ) -> RuntimeSurfaceCommitBufferImportExecutionDryRunReport {
        buffer_import_execution_dry_run_report_from_precondition_gate(report)
    }
}

/// 从 Phase 55I precondition gate report 派生 execution dry-run report；不 import buffer。
pub fn buffer_import_execution_dry_run_report_from_precondition_gate(
    report: &RuntimeSurfaceCommitBufferImportPreconditionGateReport,
) -> RuntimeSurfaceCommitBufferImportExecutionDryRunReport {
    let observed_adapter_proof = report.observed_adapter_proof.clone();
    let mut blockers = Vec::new();
    if !report.gate_invoked {
        blockers.push(
            RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingPreconditionGateEvidence,
        );
    }
    if observed_adapter_proof.is_none() {
        blockers.push(RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingAdapterProof);
    }
    if !report.import_preconditions_met {
        blockers.push(RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingImportPreconditions);
    }
    if report.actual_import_required {
        blockers.push(
            RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingRealBufferImportImplementation,
        );
    } else {
        blockers.push(RuntimeSurfaceCommitBufferImportExecutionBlocker::NoActualImportRequired);
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportExecutionDryRunReport {
        dry_run_invoked: true,
        source_buffer_import_precondition_gate_report_observed: report.gate_invoked,
        observed_adapter_proof,
        execution_guard_available: true,
        execution_attempted: false,
        execution_noop: true,
        execution_blocked: true,
        import_preconditions_met: report.import_preconditions_met,
        future_import_preconditions_met: report.future_import_preconditions_met,
        buffer_attach_observed: report.buffer_attach_observed,
        buffer_present: report.buffer_present,
        buffer_removed: report.buffer_removed,
        candidate_evidence_observed: report.candidate_evidence_observed,
        actual_import_required: report.actual_import_required,
        importer_owner_evidence_available: report.importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available: report
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportExecutionOperation::ObservePreconditionGateReport,
            RuntimeSurfaceCommitBufferImportExecutionOperation::CheckActualImportRequirement,
            RuntimeSurfaceCommitBufferImportExecutionOperation::CheckRealBufferImportImplementation,
            RuntimeSurfaceCommitBufferImportExecutionOperation::BuildNoopExecutionReport,
        ],
        blockers,
    }
}

/// Buffer import implementation owner shell 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportImplementationOwnerOperation {
    /// 读取上游 execution dry-run report。
    ObserveExecutionDryRunReport,
    /// 检查 owner shell 是否可生成 handoff report。
    CheckImplementationOwnerShell,
    /// 检查真实 importer implementation 是否可用。
    CheckRealImporterImplementation,
    /// 生成 implementation owner shell report。
    BuildImplementationOwnerShellReport,
}

/// Buffer import implementation owner shell 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker {
    /// 上游 execution dry-run report 未被观察到。
    MissingExecutionDryRunReport,
    /// 上游 execution dry-run 仍处于 blocked 状态。
    ExecutionDryRunBlocked,
    /// 本轮 commit 不需要真实 buffer import。
    NoActualImportRequired,
    /// 尚无真实 buffer import implementation。
    MissingRealBufferImportImplementation,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned buffer import implementation owner shell 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport {
    /// 本轮是否执行 implementation owner shell seam。
    pub owner_shell_invoked: bool,

    /// 是否观察到 Phase 55J execution dry-run report。
    pub source_buffer_import_execution_dry_run_report_observed: bool,

    /// 上游 execution dry-run report；只保存 pure data。
    pub observed_execution_dry_run_report: RuntimeSurfaceCommitBufferImportExecutionDryRunReport,

    /// implementation owner shell 是否可用；不代表真实 importer implementation 可用。
    pub implementation_owner_shell_available: bool,

    /// implementation owner shell 是否绑定到 runtime-owned report seam。
    pub implementation_owner_shell_bound: bool,

    /// 真实 importer implementation 是否可用；Phase 55K 固定为 false。
    pub real_importer_implementation_available: bool,

    /// 本阶段是否允许实际 import attempt；Phase 55K 固定为 false。
    pub actual_import_attempt_admitted: bool,

    /// 本阶段是否阻止实际 import attempt。
    pub actual_import_attempt_blocked: bool,

    /// 本轮是否需要未来真实 import。
    pub actual_import_required: bool,

    /// 上游 precondition 是否满足。
    pub import_preconditions_met: bool,

    /// 上游 future precondition gate 是否满足。
    pub future_import_preconditions_met: bool,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带 present buffer evidence。
    pub buffer_present: bool,

    /// commit 是否携带 null attach / removal evidence。
    pub buffer_removed: bool,

    /// 是否观察到 candidate evidence。
    pub candidate_evidence_observed: bool,

    /// importer owner evidence 是否可用。
    pub importer_owner_evidence_available: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 本阶段是否尝试 import buffer；Phase 55K 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55K 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55K 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55K 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55K 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55K 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55K 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55K 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportImplementationOwnerOperation>,

    /// 阻止进入真实 buffer import implementation path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker>,
}

/// Runtime-owned buffer import implementation owner shell；不执行真实 import。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportImplementationOwnerShell;

impl RuntimeSurfaceCommitBufferImportImplementationOwnerShell {
    /// 创建 runtime-owned buffer import implementation owner shell。
    pub fn new() -> Self {
        Self
    }

    /// 从 Phase 55J execution dry-run report 派生 owner shell report；不 import buffer。
    pub fn buffer_import_implementation_owner_shell_report_from_execution_dry_run(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportExecutionDryRunReport,
    ) -> RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport {
        buffer_import_implementation_owner_shell_report_from_execution_dry_run(report)
    }
}

/// 从 Phase 55J execution dry-run report 派生 owner shell report；不 import buffer。
pub fn buffer_import_implementation_owner_shell_report_from_execution_dry_run(
    report: &RuntimeSurfaceCommitBufferImportExecutionDryRunReport,
) -> RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport {
    let mut blockers = Vec::new();
    if !report.dry_run_invoked {
        blockers.push(
            RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingExecutionDryRunReport,
        );
    }
    if report.execution_blocked {
        blockers.push(
            RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::ExecutionDryRunBlocked,
        );
    }
    if report.actual_import_required {
        blockers.push(
            RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingRealBufferImportImplementation,
        );
    } else {
        blockers.push(
            RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::NoActualImportRequired,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport {
        owner_shell_invoked: true,
        source_buffer_import_execution_dry_run_report_observed: report.dry_run_invoked,
        observed_execution_dry_run_report: report.clone(),
        implementation_owner_shell_available: true,
        implementation_owner_shell_bound: true,
        real_importer_implementation_available: false,
        actual_import_attempt_admitted: false,
        actual_import_attempt_blocked: true,
        actual_import_required: report.actual_import_required,
        import_preconditions_met: report.import_preconditions_met,
        future_import_preconditions_met: report.future_import_preconditions_met,
        buffer_attach_observed: report.buffer_attach_observed,
        buffer_present: report.buffer_present,
        buffer_removed: report.buffer_removed,
        candidate_evidence_observed: report.candidate_evidence_observed,
        importer_owner_evidence_available: report.importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available: report
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportImplementationOwnerOperation::ObserveExecutionDryRunReport,
            RuntimeSurfaceCommitBufferImportImplementationOwnerOperation::CheckImplementationOwnerShell,
            RuntimeSurfaceCommitBufferImportImplementationOwnerOperation::CheckRealImporterImplementation,
            RuntimeSurfaceCommitBufferImportImplementationOwnerOperation::BuildImplementationOwnerShellReport,
        ],
        blockers,
    }
}

/// Actual buffer import attempt admission / record 中可定位的纯数据操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportActualAttemptOperation {
    /// 读取上游 implementation owner shell report。
    ObserveImplementationOwnerShellReport,
    /// 检查 actual import attempt admission。
    CheckActualAttemptAdmission,
    /// 检查真实 buffer importer implementation 是否可用。
    CheckRealBufferImportImplementation,
    /// 生成 actual attempt record。
    BuildActualAttemptRecord,
}

/// Actual buffer import attempt record 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitBufferImportActualAttemptBlocker {
    /// 上游 implementation owner shell report 未被观察到。
    MissingImplementationOwnerShellReport,
    /// 上游 implementation owner shell 仍处于 blocked 状态。
    ImplementationOwnerShellBlocked,
    /// 本轮 commit 不需要真实 buffer import。
    NoActualImportRequired,
    /// 没有 admission 允许 actual import attempt。
    MissingAttemptAdmission,
    /// 尚无真实 buffer import implementation。
    MissingRealBufferImportImplementation,
    /// texture creation 尚未接入。
    MissingTextureCreation,
    /// renderer call 尚未接入。
    MissingRendererCall,
    /// damage submit 尚未接入。
    MissingDamageSubmit,
    /// frame callback done 尚未接入。
    MissingFrameCallbackDone,
}

/// Runtime-owned actual buffer import attempt admission / record 纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitBufferImportActualAttemptRecord {
    /// 本轮是否执行 actual attempt record seam。
    pub actual_attempt_recorder_invoked: bool,

    /// 是否观察到 Phase 55K implementation owner shell report。
    pub source_buffer_import_implementation_owner_shell_report_observed: bool,

    /// 上游 implementation owner shell report；只保存 pure data。
    pub observed_implementation_owner_shell_report:
        RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport,

    /// actual attempt record 是否可用；不代表真实 import 已执行。
    pub actual_attempt_record_available: bool,

    /// 本阶段是否记录了 attempt admission decision。
    pub actual_attempt_recorded: bool,

    /// 本阶段是否检查 actual import attempt admission。
    pub actual_attempt_admission_checked: bool,

    /// 本阶段是否允许 actual import attempt；Phase 55L 固定为 false。
    pub actual_attempt_admitted: bool,

    /// 本阶段是否阻止 actual import attempt。
    pub actual_attempt_blocked: bool,

    /// 本轮是否需要未来真实 import。
    pub actual_import_required: bool,

    /// 上游 owner shell 是否可用。
    pub implementation_owner_shell_available: bool,

    /// 真实 importer implementation 是否可用；Phase 55L 固定为 false。
    pub real_importer_implementation_available: bool,

    /// commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,

    /// commit 是否携带 present buffer evidence。
    pub buffer_present: bool,

    /// commit 是否携带 null attach / removal evidence。
    pub buffer_removed: bool,

    /// 是否观察到 candidate evidence。
    pub candidate_evidence_observed: bool,

    /// importer owner evidence 是否可用。
    pub importer_owner_evidence_available: bool,

    /// renderer backend descriptor evidence 是否可用。
    pub renderer_backend_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// 本阶段是否尝试 import buffer；Phase 55L 固定为 false。
    pub buffer_import_attempted: bool,

    /// 本阶段是否完成 buffer import；Phase 55L 固定为 false。
    pub buffer_imported: bool,

    /// 本阶段是否创建 texture；Phase 55L 固定为 false。
    pub texture_created: bool,

    /// 本阶段是否调用 renderer；Phase 55L 固定为 false。
    pub renderer_called: bool,

    /// 本阶段是否提交 damage；Phase 55L 固定为 false。
    pub damage_submitted: bool,

    /// 本阶段是否发送 frame callback done；Phase 55L 固定为 false。
    pub frame_callback_done_sent: bool,

    /// 本阶段是否接入 input；Phase 55L 固定为 false。
    pub input_support: bool,

    /// 本阶段是否触发 core mutation；Phase 55L 固定为 false。
    pub core_mutation_invoked: bool,

    /// 执行过的操作。
    pub operations: Vec<RuntimeSurfaceCommitBufferImportActualAttemptOperation>,

    /// 阻止进入真实 buffer import attempt path 的原因。
    pub blockers: Vec<RuntimeSurfaceCommitBufferImportActualAttemptBlocker>,
}

/// Runtime-owned actual buffer import attempt recorder；不执行真实 import。
#[derive(Debug, Default)]
pub struct RuntimeSurfaceCommitBufferImportActualAttemptRecorder;

impl RuntimeSurfaceCommitBufferImportActualAttemptRecorder {
    /// 创建 runtime-owned actual buffer import attempt recorder。
    pub fn new() -> Self {
        Self
    }

    /// 从 Phase 55K owner shell report 派生 actual attempt record；不 import buffer。
    pub fn buffer_import_actual_attempt_record_from_owner_shell(
        &mut self,
        report: &RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport,
    ) -> RuntimeSurfaceCommitBufferImportActualAttemptRecord {
        buffer_import_actual_attempt_record_from_owner_shell(report)
    }
}

/// 从 Phase 55K owner shell report 派生 actual attempt record；不 import buffer。
pub fn buffer_import_actual_attempt_record_from_owner_shell(
    report: &RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport,
) -> RuntimeSurfaceCommitBufferImportActualAttemptRecord {
    let mut blockers = Vec::new();
    if !report.owner_shell_invoked {
        blockers.push(
            RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingImplementationOwnerShellReport,
        );
    }
    if report.actual_import_attempt_blocked {
        blockers.push(
            RuntimeSurfaceCommitBufferImportActualAttemptBlocker::ImplementationOwnerShellBlocked,
        );
    }
    if !report.actual_import_attempt_admitted {
        blockers
            .push(RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingAttemptAdmission);
    }
    if report.actual_import_required {
        blockers.push(
            RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingRealBufferImportImplementation,
        );
    } else {
        blockers.push(RuntimeSurfaceCommitBufferImportActualAttemptBlocker::NoActualImportRequired);
    }
    blockers.extend([
        RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingTextureCreation,
        RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingRendererCall,
        RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingDamageSubmit,
        RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingFrameCallbackDone,
    ]);

    RuntimeSurfaceCommitBufferImportActualAttemptRecord {
        actual_attempt_recorder_invoked: true,
        source_buffer_import_implementation_owner_shell_report_observed: report.owner_shell_invoked,
        observed_implementation_owner_shell_report: report.clone(),
        actual_attempt_record_available: true,
        actual_attempt_recorded: true,
        actual_attempt_admission_checked: true,
        actual_attempt_admitted: false,
        actual_attempt_blocked: true,
        actual_import_required: report.actual_import_required,
        implementation_owner_shell_available: report.implementation_owner_shell_available,
        real_importer_implementation_available: report.real_importer_implementation_available,
        buffer_attach_observed: report.buffer_attach_observed,
        buffer_present: report.buffer_present,
        buffer_removed: report.buffer_removed,
        candidate_evidence_observed: report.candidate_evidence_observed,
        importer_owner_evidence_available: report.importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available: report
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: report.registered_renderer_backend_kind,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitBufferImportActualAttemptOperation::ObserveImplementationOwnerShellReport,
            RuntimeSurfaceCommitBufferImportActualAttemptOperation::CheckActualAttemptAdmission,
            RuntimeSurfaceCommitBufferImportActualAttemptOperation::CheckRealBufferImportImplementation,
            RuntimeSurfaceCommitBufferImportActualAttemptOperation::BuildActualAttemptRecord,
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

    /// texture support shell readiness report。
    pub texture_support_shell_readiness_report:
        RuntimeSurfaceCommitTextureSupportShellReadinessReport,

    /// render operation / render execution readiness intent report。
    pub render_operation_readiness_report: RuntimeSurfaceCommitRenderOperationReadinessReport,

    /// render operation intent runtime-owned queue drain report。
    pub render_operation_intent_drain_report: RuntimeSurfaceCommitRenderOperationIntentDrainReport,

    /// render execution owner boundary blocked readiness report。
    pub render_execution_owner_boundary_report:
        RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport,

    /// render execution owner shell readiness report。
    pub render_execution_owner_shell_readiness_report:
        RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport,

    /// basic render pipeline skeleton readiness report。
    pub render_pipeline_skeleton_readiness_report:
        RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport,

    /// render backend capability report。
    pub render_backend_capability_report: RuntimeSurfaceCommitRenderBackendCapabilityReport,

    /// renderer backend registration descriptor report。
    pub renderer_backend_registration_report: RuntimeSurfaceCommitRendererBackendRegistrationReport,

    /// renderer backend owner shell readiness report。
    pub renderer_backend_owner_shell_readiness_report:
        RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport,

    /// buffer importer resource owner boundary readiness report。
    pub buffer_import_resource_owner_readiness_report:
        RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport,

    /// buffer import planning report。
    pub buffer_import_planning_report: RuntimeSurfaceCommitBufferImportPlanningReport,

    /// buffer import implementation descriptor / adapter boundary report。
    pub buffer_import_implementation_boundary_report:
        RuntimeSurfaceCommitBufferImportImplementationBoundaryReport,

    /// buffer import adapter proof boundary report。
    pub buffer_import_adapter_proof_boundary_report:
        RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport,

    /// buffer import precondition gate report。
    pub buffer_import_precondition_gate_report:
        RuntimeSurfaceCommitBufferImportPreconditionGateReport,

    /// buffer import execution dry-run / no-op guard report。
    pub buffer_import_execution_dry_run_report:
        RuntimeSurfaceCommitBufferImportExecutionDryRunReport,

    /// buffer import implementation owner shell report。
    pub buffer_import_implementation_owner_shell_report:
        RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport,

    /// actual buffer import attempt admission / record report。
    pub buffer_import_actual_attempt_record: RuntimeSurfaceCommitBufferImportActualAttemptRecord,

    /// Phase 56A SHM-first buffer import adapter skeleton report。
    pub shm_first_buffer_import_adapter_report:
        RuntimeSurfaceCommitShmFirstBufferImportAdapterReport,

    /// Phase 56B SHM buffer metadata evidence report。
    pub shm_buffer_metadata_report: RuntimeSurfaceCommitShmBufferMetadataReport,

    /// Phase 56E texture creation precondition audit report。
    pub texture_creation_precondition_audit_report:
        RuntimeSurfaceCommitTextureCreationPreconditionAuditReport,

    /// Phase 56F texture creation blocker / no-op skeleton report。
    pub texture_creation_noop_report: RuntimeSurfaceCommitTextureCreationNoopReport,

    /// Phase 56G texture owner boundary report。
    pub texture_owner_boundary_report: RuntimeSurfaceCommitTextureOwnerBoundaryReport,

    /// Phase 56H renderer backend instance audit report。
    pub renderer_backend_instance_audit_report:
        RuntimeSurfaceCommitRendererBackendInstanceAuditReport,

    /// Phase 56I texture import route decision report。
    pub texture_import_route_decision_report: RuntimeSurfaceCommitTextureImportRouteDecisionReport,

    /// Phase 56J damage-to-texture mapping audit report。
    pub damage_to_texture_mapping_audit_report:
        RuntimeSurfaceCommitDamageToTextureMappingAuditReport,
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
    texture_support_shell: RuntimeSurfaceCommitTextureSupportShell,
    render_operation_intent_queue_owner: RuntimeSurfaceCommitRenderOperationIntentQueueOwner,
    render_execution_owner_boundary: RuntimeSurfaceCommitRenderExecutionOwnerBoundary,
    render_execution_owner_shell: RuntimeSurfaceCommitRenderExecutionOwnerShell,
    render_pipeline_skeleton_owner: RuntimeSurfaceCommitRenderPipelineSkeletonOwner,
    render_backend_capability_owner: RuntimeSurfaceCommitRenderBackendCapabilityOwner,
    renderer_backend_registration_owner: RuntimeSurfaceCommitRendererBackendRegistrationOwner,
    renderer_backend_owner_shell: RuntimeSurfaceCommitRendererBackendOwnerShell,
    buffer_import_resource_owner_boundary: RuntimeSurfaceCommitBufferImportResourceOwnerBoundary,
    buffer_import_planner: RuntimeSurfaceCommitBufferImportPlanner,
    buffer_import_implementation_boundary: RuntimeSurfaceCommitBufferImportImplementationBoundary,
    buffer_import_adapter_proof_boundary: RuntimeSurfaceCommitBufferImportAdapterProofBoundary,
    buffer_import_precondition_gate: RuntimeSurfaceCommitBufferImportPreconditionGate,
    buffer_import_execution_dry_run: RuntimeSurfaceCommitBufferImportExecutionDryRun,
    buffer_import_implementation_owner_shell:
        RuntimeSurfaceCommitBufferImportImplementationOwnerShell,
    buffer_import_actual_attempt_recorder: RuntimeSurfaceCommitBufferImportActualAttemptRecorder,
    shm_first_buffer_import_adapter: LinuxShmFirstBufferImportAdapterSkeleton,
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
            texture_support_shell: RuntimeSurfaceCommitTextureSupportShell::new(),
            render_operation_intent_queue_owner:
                RuntimeSurfaceCommitRenderOperationIntentQueueOwner::new(),
            render_execution_owner_boundary: RuntimeSurfaceCommitRenderExecutionOwnerBoundary::new(
            ),
            render_execution_owner_shell: RuntimeSurfaceCommitRenderExecutionOwnerShell::new(),
            render_pipeline_skeleton_owner: RuntimeSurfaceCommitRenderPipelineSkeletonOwner::new(),
            render_backend_capability_owner: RuntimeSurfaceCommitRenderBackendCapabilityOwner::new(
            ),
            renderer_backend_registration_owner:
                RuntimeSurfaceCommitRendererBackendRegistrationOwner::new(),
            renderer_backend_owner_shell: RuntimeSurfaceCommitRendererBackendOwnerShell::new(),
            buffer_import_resource_owner_boundary:
                RuntimeSurfaceCommitBufferImportResourceOwnerBoundary::new(),
            buffer_import_planner: RuntimeSurfaceCommitBufferImportPlanner::new(),
            buffer_import_implementation_boundary:
                RuntimeSurfaceCommitBufferImportImplementationBoundary::new(),
            buffer_import_adapter_proof_boundary:
                RuntimeSurfaceCommitBufferImportAdapterProofBoundary::new(),
            buffer_import_precondition_gate: RuntimeSurfaceCommitBufferImportPreconditionGate::new(
            ),
            buffer_import_execution_dry_run: RuntimeSurfaceCommitBufferImportExecutionDryRun::new(),
            buffer_import_implementation_owner_shell:
                RuntimeSurfaceCommitBufferImportImplementationOwnerShell::new(),
            buffer_import_actual_attempt_recorder:
                RuntimeSurfaceCommitBufferImportActualAttemptRecorder::new(),
            shm_first_buffer_import_adapter: LinuxShmFirstBufferImportAdapterSkeleton::new(),
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

    /// 返回 coordinator render operation intent owner 中的 pending 数量。
    pub fn render_operation_intent_pending_count(&self) -> usize {
        self.render_operation_intent_queue_owner.pending_count()
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
        let texture_support_shell_readiness_report = self
            .texture_support_shell
            .texture_support_shell_readiness_from_buffer_importer_shell(
                &buffer_importer_shell_readiness_report,
            );
        let render_operation_readiness_report =
            render_operation_readiness_from_texture_support_shell(
                &texture_support_shell_readiness_report,
            );
        let render_operation_intent_drain_report = self
            .render_operation_intent_queue_owner
            .enqueue_from_render_operation_readiness_and_drain_once(
                &render_operation_readiness_report,
            );
        let render_execution_owner_boundary_report = self
            .render_execution_owner_boundary
            .consume_render_operation_intent(&render_operation_intent_drain_report);
        let render_execution_owner_shell_readiness_report = self
            .render_execution_owner_shell
            .render_execution_owner_shell_readiness_from_owner_boundary(
                &render_execution_owner_boundary_report,
            );
        let render_pipeline_skeleton_readiness_report = self
            .render_pipeline_skeleton_owner
            .render_pipeline_skeleton_readiness_from_execution_owner_shell(
                &render_execution_owner_shell_readiness_report,
            );
        let render_backend_capability_report = self
            .render_backend_capability_owner
            .render_backend_capability_report_from_pipeline_skeleton(
                &render_pipeline_skeleton_readiness_report,
            );
        let renderer_backend_registration_report = self
            .renderer_backend_registration_owner
            .renderer_backend_registration_report_from_backend_capability(
                &render_backend_capability_report,
            );
        let renderer_backend_owner_shell_readiness_report = self
            .renderer_backend_owner_shell
            .renderer_backend_owner_shell_readiness_from_registration(
                &renderer_backend_registration_report,
            );
        let buffer_import_resource_owner_readiness_report = self
            .buffer_import_resource_owner_boundary
            .buffer_import_resource_owner_readiness_from_renderer_backend_owner_shell(
                &renderer_backend_owner_shell_readiness_report,
            );
        let buffer_import_planning_report = self
            .buffer_import_planner
            .buffer_import_planning_report_from_resource_owner_boundary(
                &buffer_import_resource_owner_readiness_report,
            );
        let buffer_import_implementation_boundary_report = self
            .buffer_import_implementation_boundary
            .buffer_import_implementation_boundary_report_from_planning_report(
                &buffer_import_planning_report,
            );
        let buffer_import_adapter_proof_boundary_report = self
            .buffer_import_adapter_proof_boundary
            .buffer_import_adapter_proof_boundary_report_from_implementation_report(
                &buffer_import_implementation_boundary_report,
            );
        let buffer_import_precondition_gate_report = self
            .buffer_import_precondition_gate
            .buffer_import_precondition_gate_report_from_adapter_proof(
                &buffer_import_adapter_proof_boundary_report,
            );
        let buffer_import_execution_dry_run_report = self
            .buffer_import_execution_dry_run
            .buffer_import_execution_dry_run_report_from_precondition_gate(
                &buffer_import_precondition_gate_report,
            );
        let buffer_import_implementation_owner_shell_report = self
            .buffer_import_implementation_owner_shell
            .buffer_import_implementation_owner_shell_report_from_execution_dry_run(
                &buffer_import_execution_dry_run_report,
            );
        let buffer_import_actual_attempt_record = self
            .buffer_import_actual_attempt_recorder
            .buffer_import_actual_attempt_record_from_owner_shell(
                &buffer_import_implementation_owner_shell_report,
            );
        let shm_first_buffer_import_adapter_report = self
            .shm_first_buffer_import_adapter
            .report_from_actual_attempt_record(&buffer_import_actual_attempt_record, None);
        let shm_buffer_metadata_report = self
            .shm_first_buffer_import_adapter
            .metadata_report_from_adapter_report(&shm_first_buffer_import_adapter_report, None);
        let texture_creation_precondition_audit_report = self
            .shm_first_buffer_import_adapter
            .texture_creation_precondition_audit_from_metadata_report(&shm_buffer_metadata_report);
        let texture_creation_noop_report = self
            .shm_first_buffer_import_adapter
            .texture_creation_noop_report_from_precondition_audit(
                &texture_creation_precondition_audit_report,
            );
        let texture_owner_boundary_report = self
            .shm_first_buffer_import_adapter
            .texture_owner_boundary_report_from_noop_report(&texture_creation_noop_report);
        let renderer_backend_instance_audit_report = self
            .shm_first_buffer_import_adapter
            .renderer_backend_instance_audit_from_texture_owner_boundary_report(
                &texture_owner_boundary_report,
            );
        let texture_import_route_decision_report = self
            .shm_first_buffer_import_adapter
            .texture_import_route_decision_from_renderer_backend_instance_audit(
                &renderer_backend_instance_audit_report,
            );
        let damage_to_texture_mapping_audit_report = self
            .shm_first_buffer_import_adapter
            .damage_to_texture_mapping_audit_from_texture_import_route_decision(
                &texture_import_route_decision_report,
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
            texture_support_shell_readiness_report,
            render_operation_readiness_report,
            render_operation_intent_drain_report,
            render_execution_owner_boundary_report,
            render_execution_owner_shell_readiness_report,
            render_pipeline_skeleton_readiness_report,
            render_backend_capability_report,
            renderer_backend_registration_report,
            renderer_backend_owner_shell_readiness_report,
            buffer_import_resource_owner_readiness_report,
            buffer_import_planning_report,
            buffer_import_implementation_boundary_report,
            buffer_import_adapter_proof_boundary_report,
            buffer_import_precondition_gate_report,
            buffer_import_execution_dry_run_report,
            buffer_import_implementation_owner_shell_report,
            buffer_import_actual_attempt_record,
            shm_first_buffer_import_adapter_report,
            shm_buffer_metadata_report,
            texture_creation_precondition_audit_report,
            texture_creation_noop_report,
            texture_owner_boundary_report,
            renderer_backend_instance_audit_report,
            texture_import_route_decision_report,
            damage_to_texture_mapping_audit_report,
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
        RuntimeSurfaceCommitBufferImportActualAttemptBlocker,
        RuntimeSurfaceCommitBufferImportExecutionBlocker,
        RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker,
        RuntimeSurfaceCommitBufferImportPreconditionGateReport,
        RuntimeSurfaceCommitRenderBackendKind,
        buffer_import_actual_attempt_record_from_owner_shell,
        buffer_import_execution_dry_run_report_from_precondition_gate,
        buffer_import_implementation_owner_shell_report_from_execution_dry_run,
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

    fn precondition_gate_report(
        actual_import_required: bool,
    ) -> RuntimeSurfaceCommitBufferImportPreconditionGateReport {
        RuntimeSurfaceCommitBufferImportPreconditionGateReport {
            gate_invoked: true,
            source_buffer_import_adapter_proof_report_observed: true,
            adapter_proof_registered: true,
            observed_adapter_proof: None,
            import_precondition_gate_available: true,
            import_preconditions_met: actual_import_required,
            future_import_preconditions_met: actual_import_required,
            buffer_attach_observed: true,
            buffer_present: actual_import_required,
            buffer_removed: !actual_import_required,
            candidate_evidence_observed: true,
            actual_import_required,
            importer_owner_evidence_available: true,
            renderer_backend_descriptor_evidence_available: true,
            registered_renderer_backend_kind: Some(
                RuntimeSurfaceCommitRenderBackendKind::SmithayLinux,
            ),
            buffer_import_attempted: false,
            buffer_imported: false,
            texture_created: false,
            renderer_called: false,
            damage_submitted: false,
            frame_callback_done_sent: false,
            input_support: false,
            core_mutation_invoked: false,
            operations: Vec::new(),
            blockers: Vec::new(),
        }
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

    /// Phase 55J dry-run 从 precondition gate 派生 blocked/no-op report，但不尝试 import。
    #[test]
    fn buffer_import_execution_dry_run_derives_noop_and_missing_importer_truth() {
        let noop_report = buffer_import_execution_dry_run_report_from_precondition_gate(
            &precondition_gate_report(false),
        );

        assert!(noop_report.execution_guard_available);
        assert!(!noop_report.execution_attempted);
        assert!(noop_report.execution_noop);
        assert!(noop_report.execution_blocked);
        assert!(!noop_report.actual_import_required);
        assert!(
            noop_report.blockers.contains(
                &RuntimeSurfaceCommitBufferImportExecutionBlocker::NoActualImportRequired
            )
        );
        assert!(!noop_report.buffer_import_attempted);
        assert!(!noop_report.buffer_imported);
        assert!(!noop_report.texture_created);
        assert!(!noop_report.renderer_called);
        assert!(!noop_report.damage_submitted);
        assert!(!noop_report.frame_callback_done_sent);
        assert!(!noop_report.input_support);
        assert!(!noop_report.core_mutation_invoked);

        let blocked_report = buffer_import_execution_dry_run_report_from_precondition_gate(
            &precondition_gate_report(true),
        );

        assert!(blocked_report.execution_guard_available);
        assert!(!blocked_report.execution_attempted);
        assert!(blocked_report.execution_noop);
        assert!(blocked_report.execution_blocked);
        assert!(blocked_report.actual_import_required);
        assert!(
            blocked_report.blockers.contains(
                &RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingRealBufferImportImplementation,
            )
        );
        assert!(!blocked_report.buffer_import_attempted);
        assert!(!blocked_report.buffer_imported);
        assert!(!blocked_report.texture_created);
        assert!(!blocked_report.renderer_called);
        assert!(!blocked_report.damage_submitted);
        assert!(!blocked_report.frame_callback_done_sent);
        assert!(!blocked_report.input_support);
        assert!(!blocked_report.core_mutation_invoked);
    }

    /// Phase 55K implementation owner shell consumes dry-run reports but still blocks actual import.
    #[test]
    fn buffer_import_implementation_owner_shell_preserves_blocked_import_truth() {
        let noop_dry_run = buffer_import_execution_dry_run_report_from_precondition_gate(
            &precondition_gate_report(false),
        );
        let noop_owner_report =
            buffer_import_implementation_owner_shell_report_from_execution_dry_run(&noop_dry_run);

        assert!(noop_owner_report.implementation_owner_shell_available);
        assert!(noop_owner_report.implementation_owner_shell_bound);
        assert!(!noop_owner_report.real_importer_implementation_available);
        assert!(!noop_owner_report.actual_import_attempt_admitted);
        assert!(noop_owner_report.actual_import_attempt_blocked);
        assert!(!noop_owner_report.actual_import_required);
        assert!(noop_owner_report.blockers.contains(
            &RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::NoActualImportRequired
        ));
        assert!(!noop_owner_report.buffer_import_attempted);
        assert!(!noop_owner_report.buffer_imported);
        assert!(!noop_owner_report.texture_created);
        assert!(!noop_owner_report.renderer_called);
        assert!(!noop_owner_report.damage_submitted);
        assert!(!noop_owner_report.frame_callback_done_sent);
        assert!(!noop_owner_report.input_support);
        assert!(!noop_owner_report.core_mutation_invoked);

        let blocked_dry_run = buffer_import_execution_dry_run_report_from_precondition_gate(
            &precondition_gate_report(true),
        );
        let blocked_owner_report =
            buffer_import_implementation_owner_shell_report_from_execution_dry_run(
                &blocked_dry_run,
            );

        assert!(blocked_owner_report.implementation_owner_shell_available);
        assert!(blocked_owner_report.implementation_owner_shell_bound);
        assert!(!blocked_owner_report.real_importer_implementation_available);
        assert!(!blocked_owner_report.actual_import_attempt_admitted);
        assert!(blocked_owner_report.actual_import_attempt_blocked);
        assert!(blocked_owner_report.actual_import_required);
        assert!(
            blocked_owner_report.blockers.contains(
                &RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingRealBufferImportImplementation,
            )
        );
        assert!(!blocked_owner_report.buffer_import_attempted);
        assert!(!blocked_owner_report.buffer_imported);
        assert!(!blocked_owner_report.texture_created);
        assert!(!blocked_owner_report.renderer_called);
        assert!(!blocked_owner_report.damage_submitted);
        assert!(!blocked_owner_report.frame_callback_done_sent);
        assert!(!blocked_owner_report.input_support);
        assert!(!blocked_owner_report.core_mutation_invoked);
    }

    /// Phase 55L actual attempt record records admission decisions without importing buffers.
    #[test]
    fn buffer_import_actual_attempt_record_preserves_blocked_import_truth() {
        let noop_dry_run = buffer_import_execution_dry_run_report_from_precondition_gate(
            &precondition_gate_report(false),
        );
        let noop_owner_report =
            buffer_import_implementation_owner_shell_report_from_execution_dry_run(&noop_dry_run);
        let noop_attempt_record =
            buffer_import_actual_attempt_record_from_owner_shell(&noop_owner_report);

        assert!(noop_attempt_record.actual_attempt_record_available);
        assert!(noop_attempt_record.actual_attempt_recorded);
        assert!(noop_attempt_record.actual_attempt_admission_checked);
        assert!(!noop_attempt_record.actual_attempt_admitted);
        assert!(noop_attempt_record.actual_attempt_blocked);
        assert!(!noop_attempt_record.actual_import_required);
        assert!(noop_attempt_record.blockers.contains(
            &RuntimeSurfaceCommitBufferImportActualAttemptBlocker::NoActualImportRequired
        ));
        assert!(noop_attempt_record.blockers.contains(
            &RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingAttemptAdmission
        ));
        assert!(!noop_attempt_record.buffer_import_attempted);
        assert!(!noop_attempt_record.buffer_imported);
        assert!(!noop_attempt_record.texture_created);
        assert!(!noop_attempt_record.renderer_called);
        assert!(!noop_attempt_record.damage_submitted);
        assert!(!noop_attempt_record.frame_callback_done_sent);
        assert!(!noop_attempt_record.input_support);
        assert!(!noop_attempt_record.core_mutation_invoked);

        let blocked_dry_run = buffer_import_execution_dry_run_report_from_precondition_gate(
            &precondition_gate_report(true),
        );
        let blocked_owner_report =
            buffer_import_implementation_owner_shell_report_from_execution_dry_run(
                &blocked_dry_run,
            );
        let blocked_attempt_record =
            buffer_import_actual_attempt_record_from_owner_shell(&blocked_owner_report);

        assert!(blocked_attempt_record.actual_attempt_record_available);
        assert!(blocked_attempt_record.actual_attempt_recorded);
        assert!(blocked_attempt_record.actual_attempt_admission_checked);
        assert!(!blocked_attempt_record.actual_attempt_admitted);
        assert!(blocked_attempt_record.actual_attempt_blocked);
        assert!(blocked_attempt_record.actual_import_required);
        assert!(blocked_attempt_record.blockers.contains(
            &RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingRealBufferImportImplementation,
        ));
        assert!(blocked_attempt_record.blockers.contains(
            &RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingAttemptAdmission
        ));
        assert!(!blocked_attempt_record.buffer_import_attempted);
        assert!(!blocked_attempt_record.buffer_imported);
        assert!(!blocked_attempt_record.texture_created);
        assert!(!blocked_attempt_record.renderer_called);
        assert!(!blocked_attempt_record.damage_submitted);
        assert!(!blocked_attempt_record.frame_callback_done_sent);
        assert!(!blocked_attempt_record.input_support);
        assert!(!blocked_attempt_record.core_mutation_invoked);
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
