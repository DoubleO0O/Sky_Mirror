//! Phase 51L Linux-only nested lifecycle bounded runtime loop。
//!
//! loop 只重复编排 [`NestedRuntimeCoordinator::pump_once`]，负责有限迭代、idle/error/stop
//! 退出和纯数据报告。它不直接修改 core，也不把 bounded loop 冒充完整 compositor runtime。

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use smithay::reexports::calloop::LoopSignal;

use crate::{
    core::state::State,
    smithay_backend::RuntimeToplevelAdmissionDrainTick,
    smithay_backend::nested_runtime_coordinator::{
        NestedRuntimeCoordinator, NestedRuntimeLiveAdmissionPumpReport,
        NestedRuntimeLiveAdmissionUnmapPumpReport, NestedRuntimePumpError, NestedRuntimePumpReport,
        RuntimeSurfaceCommitBufferImportActualAttemptBlocker,
        RuntimeSurfaceCommitBufferImportActualAttemptRecord,
        RuntimeSurfaceCommitBufferImportAdapterProof,
        RuntimeSurfaceCommitBufferImportAdapterProofBlocker,
        RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport,
        RuntimeSurfaceCommitBufferImportExecutionBlocker,
        RuntimeSurfaceCommitBufferImportExecutionDryRunReport,
        RuntimeSurfaceCommitBufferImportImplementationBlocker,
        RuntimeSurfaceCommitBufferImportImplementationBoundaryReport,
        RuntimeSurfaceCommitBufferImportImplementationDescriptor,
        RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker,
        RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport,
        RuntimeSurfaceCommitBufferImportPlanningBlocker,
        RuntimeSurfaceCommitBufferImportPlanningReport,
        RuntimeSurfaceCommitBufferImportPreconditionGateBlocker,
        RuntimeSurfaceCommitBufferImportPreconditionGateReport,
        RuntimeSurfaceCommitBufferImportResourceOwnerBlocker,
        RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport,
        RuntimeSurfaceCommitBufferImporterShellBlocker,
        RuntimeSurfaceCommitBufferImporterShellReadinessReport, RuntimeSurfaceCommitDrainReport,
        RuntimeSurfaceCommitRenderBackendCapabilityBlocker,
        RuntimeSurfaceCommitRenderBackendCapabilityReport, RuntimeSurfaceCommitRenderBackendKind,
        RuntimeSurfaceCommitRenderDirtyIntentDrainReport,
        RuntimeSurfaceCommitRenderDirtyReadinessIntent,
        RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker,
        RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport,
        RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker,
        RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport,
        RuntimeSurfaceCommitRenderOperationIntent,
        RuntimeSurfaceCommitRenderOperationIntentDrainReport,
        RuntimeSurfaceCommitRenderOperationReadinessReport,
        RuntimeSurfaceCommitRenderPipelineSkeletonBlocker,
        RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport,
        RuntimeSurfaceCommitRendererAdmissionReport,
        RuntimeSurfaceCommitRendererAdmissionWorkIntent,
        RuntimeSurfaceCommitRendererBackendOwnerShellBlocker,
        RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport,
        RuntimeSurfaceCommitRendererBackendRegistrationBlocker,
        RuntimeSurfaceCommitRendererBackendRegistrationReport,
        RuntimeSurfaceCommitRendererOwnerBoundaryBlocker,
        RuntimeSurfaceCommitRendererOwnerBoundaryReport,
        RuntimeSurfaceCommitRendererOwnerShellBlocker,
        RuntimeSurfaceCommitRendererOwnerShellReadinessReport,
        RuntimeSurfaceCommitTextureSupportShellBlocker,
        RuntimeSurfaceCommitTextureSupportShellReadinessReport,
        render_dirty_readiness_intent_from_commit_drain_report,
    },
};

/// Phase 51L bounded loop 尚未满足的独立能力条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeLoopBlocker {
    /// 尚无 Linux test/CI 证明 loop 可多次调用真实 coordinator pump。
    MissingLinuxBoundedLoopProof,

    /// stop flag 尚未接入 event source wakeup，不能立即打断正在等待的 pump。
    MissingWakeup,

    /// 尚无完整 compositor runtime、protocol/surface/render/input 生命周期。
    MissingCompleteRuntimeLoop,
}

/// Phase 51L bounded loop 的保守 capability 报告。
#[must_use = "必须区分 bounded loop、wakeup 与完整 compositor runtime"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopReadinessReport {
    /// 当前仍存在的 loop/runtime blockers。
    pub blockers: Vec<NestedRuntimeLoopBlocker>,

    /// 是否已定义 Linux-only bounded loop interface。
    pub loop_boundary_defined: bool,

    /// 是否已有 Linux proof 支持的 nested runtime loop。
    pub nested_runtime_loop_available: bool,

    /// 是否已有 Linux proof 支持的 bounded iteration loop。
    pub bounded_loop_available: bool,

    /// stop request 是否已由 Linux proof 支持。
    pub stop_requested_supported: bool,

    /// stop request 是否可唤醒正在阻塞的 event source；Linux wakeup proof 前为 `false`。
    pub wakeup_supported: bool,

    /// 是否已有 Linux proof 支持的 interruptible poll wait。
    pub interruptible_wait_available: bool,

    /// cooperative stop 是否可打断正在进行的 pump wait。
    pub stop_can_interrupt_wait: bool,

    /// 是否已有完整长期 compositor runtime；本阶段固定为 `false`。
    pub long_running_loop_available: bool,

    /// 是否已具备项目级 client accept 能力；本阶段固定为 `false`。
    pub accepts_clients: bool,

    /// 是否已启动长期 accept loop；本阶段固定为 `false`。
    pub runtime_accept_loop_started: bool,

    /// 是否已启动长期 protocol dispatch；本阶段固定为 `false`。
    pub protocol_dispatch_started: bool,

    /// 是否支持真实 surface；本阶段固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；本阶段固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实 render；本阶段固定为 `false`。
    pub render_support: bool,

    /// 是否支持真实 input；本阶段固定为 `false`。
    pub input_support: bool,
}

impl NestedRuntimeLoopReadinessReport {
    /// 判断 bounded loop proof 是否完整成立。
    pub fn is_bounded_loop_ready(&self) -> bool {
        self.nested_runtime_loop_available
            && self.bounded_loop_available
            && self.stop_requested_supported
    }

    /// 判断真实 wakeup/interruptible wait proof 是否完整成立。
    pub fn is_interruptible_wait_ready(&self) -> bool {
        self.wakeup_supported && self.interruptible_wait_available && self.stop_can_interrupt_wait
    }
}

/// 返回 Phase 51M C 路线经 Linux interrupt proof 支持的 wakeup readiness。
#[must_use = "wakeup proof 不能代替完整 compositor runtime"]
pub fn nested_runtime_loop_readiness_report() -> NestedRuntimeLoopReadinessReport {
    NestedRuntimeLoopReadinessReport {
        blockers: vec![NestedRuntimeLoopBlocker::MissingCompleteRuntimeLoop],
        loop_boundary_defined: true,
        nested_runtime_loop_available: true,
        bounded_loop_available: true,
        stop_requested_supported: true,
        wakeup_supported: true,
        interruptible_wait_available: true,
        stop_can_interrupt_wait: true,
        long_running_loop_available: false,
        accepts_clients: false,
        runtime_accept_loop_started: false,
        protocol_dispatch_started: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        input_support: false,
    }
}

/// bounded loop 的有限执行配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedRuntimeLoopConfig {
    /// 最多调用 coordinator pump 的次数；`0` 会立即安全退出。
    pub max_iterations: usize,

    /// 每次 coordinator pump 允许等待 accept source 的最长时间。
    pub pump_timeout: Duration,

    /// 无 lifecycle 或 protocol 活动时是否立即以 `Idle` 退出。
    pub stop_when_idle: bool,

    /// pump 返回结构化错误后是否继续下一轮。
    pub continue_after_error: bool,
}

/// bounded loop 的退出原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeLoopExitReason {
    /// 已执行完 `max_iterations`，包括零迭代配置。
    MaxIterationsReached,

    /// `stop_when_idle` 观察到无活动 pump。
    Idle,

    /// cloneable stop handle 请求停止；请求在观察后被消费。
    StopRequested,

    /// pump 报告错误且配置要求立即退出。
    Error,

    /// 外部 stop+wakeup 在 pump wait 中触发，并使 poll 提前返回。
    Interrupted,
}

/// 某次 pump 在 loop 中产生的结构化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopError {
    /// 发生错误的 1-based loop iteration。
    pub iteration: usize,

    /// 该次 coordinator pump 返回的原始结构化错误。
    pub pump_errors: Vec<NestedRuntimePumpError>,
}

#[derive(Debug)]
struct NestedRuntimeWakeupState {
    stop_requested: AtomicBool,
    waiting: AtomicBool,
    wakeup_requested: AtomicBool,
    interrupt_requested_while_waiting: AtomicBool,
}

impl NestedRuntimeWakeupState {
    fn new() -> Self {
        Self {
            stop_requested: AtomicBool::new(false),
            waiting: AtomicBool::new(false),
            wakeup_requested: AtomicBool::new(false),
            interrupt_requested_while_waiting: AtomicBool::new(false),
        }
    }
}

/// 可跨调用方 clone 的 cooperative stop/wakeup handle。
///
/// handle 不持有 coordinator 或 core。`request_stop` 保留既有 cooperative 语义；
/// [`Self::request_stop_and_wakeup`] 额外通知 calloop poll，让等待无需耗尽完整 timeout。
#[derive(Debug, Clone)]
pub struct NestedRuntimeLoopStopHandle {
    state: Arc<NestedRuntimeWakeupState>,
    loop_signal: LoopSignal,
}

impl NestedRuntimeLoopStopHandle {
    fn new(loop_signal: LoopSignal) -> Self {
        Self {
            state: Arc::new(NestedRuntimeWakeupState::new()),
            loop_signal,
        }
    }

    /// 请求 loop 在下一次 pump 边界停止。
    pub fn request_stop(&self) {
        self.state.stop_requested.store(true, Ordering::Release);
    }

    /// 请求停止并唤醒正在等待的 calloop poll。
    ///
    /// wakeup 只制造 poll notifier event；loop 返回后仍必须通过既有 coordinator seam
    /// 完成报告和 ValidationReport，不能借此直接修改 core。
    pub fn request_stop_and_wakeup(&self) {
        self.request_stop();
        self.state.wakeup_requested.store(true, Ordering::Release);
        if self.state.waiting.load(Ordering::Acquire) {
            self.state
                .interrupt_requested_while_waiting
                .store(true, Ordering::Release);
        }
        self.loop_signal.wakeup();
    }

    /// 返回尚未被 loop 消费的 stop request 状态。
    pub fn is_stop_requested(&self) -> bool {
        self.state.stop_requested.load(Ordering::Acquire)
    }

    /// 返回 loop 当前是否位于一次 coordinator pump wait 区间。
    ///
    /// 本方法只读原子状态，供外部协调 stop+wakeup；它不驱动 pump，也不访问 core。
    pub fn is_waiting(&self) -> bool {
        self.state.waiting.load(Ordering::Acquire)
    }

    fn take_stop_request(&self) -> bool {
        self.state.stop_requested.swap(false, Ordering::AcqRel)
    }

    fn begin_wait(&self) {
        self.state.waiting.store(true, Ordering::Release);
    }

    fn end_wait(&self) {
        self.state.waiting.store(false, Ordering::Release);
    }

    fn take_wakeup_request(&self) -> bool {
        self.state.wakeup_requested.swap(false, Ordering::AcqRel)
    }

    fn take_wait_interrupt(&self) -> bool {
        self.state
            .interrupt_requested_while_waiting
            .swap(false, Ordering::AcqRel)
    }
}

/// 一次 bounded run 中观察到的 wakeup/interrupt 事实。
#[must_use = "wakeup report 区分请求、真实 wait interrupt 与完整 timeout"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedRuntimeWakeupReport {
    /// 本轮是否调用了 stop+wakeup interface。
    pub wakeup_requested: bool,

    /// 本轮是否消费了 stop request。
    pub stop_requested: bool,

    /// wakeup 是否发生在 loop 标记为 waiting 的区间。
    pub wait_interrupted: bool,

    /// 从 run 进入到退出的实际耗时。
    pub elapsed_before_exit: Duration,

    /// 本轮配置的单次 pump timeout。
    pub configured_pump_timeout: Duration,

    /// 已观察到 wait interrupt，且 run 在完整 pump timeout 前退出。
    pub exited_before_timeout: bool,
}

/// 一次 bounded run 中由 live admission pump 产生的纯数据汇总。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NestedRuntimeLiveAdmissionRunSummary {
    /// live admission owner 被调用的次数。
    pub owner_invocations: usize,

    /// coordinator enqueue seam 被调用的次数。
    pub enqueue_invocations: usize,

    /// 成功入队的 pending admission 数量。
    pub admissions_enqueued: usize,

    /// runtime admission drain 被调用的次数。
    pub drain_invocations: usize,

    /// 成功消费到 ledger/core 的 admission 数量。
    pub admissions_consumed: usize,

    /// 最后一轮 drain 后 remaining pending admission 数量。
    pub pending_admissions_after: usize,
}

impl NestedRuntimeLiveAdmissionRunSummary {
    fn from_live_pump(report: &NestedRuntimeLiveAdmissionPumpReport) -> Self {
        let admissions_enqueued = report
            .live_admission_owner_report
            .coordinator_enqueue_report
            .as_ref()
            .is_some_and(|enqueue| enqueue.pending_admission_enqueued);

        Self {
            owner_invocations: 1,
            enqueue_invocations: usize::from(
                report
                    .live_admission_owner_report
                    .coordinator_enqueue_invoked,
            ),
            admissions_enqueued: usize::from(admissions_enqueued),
            drain_invocations: usize::from(report.admission_drain_report.drain_invoked),
            admissions_consumed: usize::from(
                report.admission_drain_report.pending_admission_consumed,
            ),
            pending_admissions_after: report.admission_drain_report.pending_admission_count_after,
        }
    }

    fn has_progress(&self) -> bool {
        self.enqueue_invocations > 0 || self.admissions_enqueued > 0 || self.admissions_consumed > 0
    }

    fn observe(&mut self, delta: Self) {
        self.owner_invocations = self
            .owner_invocations
            .saturating_add(delta.owner_invocations);
        self.enqueue_invocations = self
            .enqueue_invocations
            .saturating_add(delta.enqueue_invocations);
        self.admissions_enqueued = self
            .admissions_enqueued
            .saturating_add(delta.admissions_enqueued);
        self.drain_invocations = self
            .drain_invocations
            .saturating_add(delta.drain_invocations);
        self.admissions_consumed = self
            .admissions_consumed
            .saturating_add(delta.admissions_consumed);
        self.pending_admissions_after = delta.pending_admissions_after;
    }
}

/// 一次 bounded run 中由 live unmap drain 产生的纯数据汇总。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NestedRuntimeLiveUnmapRunSummary {
    /// runtime unmap drain 被调用的次数。
    pub drain_invocations: usize,

    /// 成功读取到 live destroyed observation 的次数。
    pub live_unmap_observations: usize,

    /// admission ledger unmap 被调用的次数。
    pub ledger_unmaps: usize,

    /// core detach seam 被调用的次数。
    pub core_detaches: usize,

    /// 成功 unmap 后确认 adapter surface mapping 保留的次数。
    pub surface_mappings_retained: usize,

    /// 成功 unmap 后确认 adapter toplevel mapping 移除的次数。
    pub toplevel_mappings_removed: usize,
}

impl NestedRuntimeLiveUnmapRunSummary {
    fn from_live_admission_unmap(report: &NestedRuntimeLiveAdmissionUnmapPumpReport) -> Self {
        Self {
            drain_invocations: usize::from(report.unmap_drain_report.drain_invoked),
            live_unmap_observations: usize::from(
                report.unmap_drain_report.live_unmap_observation_present,
            ),
            ledger_unmaps: usize::from(report.unmap_drain_report.ledger_unmap_invoked),
            core_detaches: usize::from(report.unmap_drain_report.core_detach_invoked),
            surface_mappings_retained: usize::from(
                report
                    .unmap_drain_report
                    .surface_mapping_retained_after_unmap,
            ),
            toplevel_mappings_removed: usize::from(
                report
                    .unmap_drain_report
                    .toplevel_mapping_removed_after_unmap,
            ),
        }
    }

    fn has_progress(&self) -> bool {
        self.live_unmap_observations > 0 || self.ledger_unmaps > 0 || self.core_detaches > 0
    }

    fn observe(&mut self, delta: Self) {
        self.drain_invocations = self
            .drain_invocations
            .saturating_add(delta.drain_invocations);
        self.live_unmap_observations = self
            .live_unmap_observations
            .saturating_add(delta.live_unmap_observations);
        self.ledger_unmaps = self.ledger_unmaps.saturating_add(delta.ledger_unmaps);
        self.core_detaches = self.core_detaches.saturating_add(delta.core_detaches);
        self.surface_mappings_retained = self
            .surface_mappings_retained
            .saturating_add(delta.surface_mappings_retained);
        self.toplevel_mappings_removed = self
            .toplevel_mappings_removed
            .saturating_add(delta.toplevel_mappings_removed);
    }
}

/// 一次 bounded run 中由 `wl_surface.commit` backlog drain 产生的纯数据汇总。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NestedRuntimeSurfaceCommitRunSummary {
    /// runtime commit drain seam 被调用的次数。
    pub drain_invocations: usize,

    /// 成功 drain 的 adapter-owned commit observation 数量。
    pub commit_observations_drained: usize,

    /// drain 到 structured adapter identity error 的数量。
    pub commit_observation_errors: usize,

    /// 按 FIFO drain 顺序保存的 commit sequence。
    pub drained_commit_sequences: Vec<u64>,

    /// 成功 drain 的 commit 中携带 buffer attach/remove evidence 的数量。
    pub buffer_attach_observations: usize,

    /// 成功 drain 的 commit 中携带真实 buffer presence evidence 的数量。
    pub buffer_presence_observations: usize,

    /// 成功 drain 的 commit 中携带 `attach(NULL)` / buffer removal evidence 的数量。
    pub buffer_removed_observations: usize,

    /// 成功 drain 的 commit 中已可作为 renderable buffer 的数量；Phase 54D 保持 0。
    pub renderable_buffer_observations: usize,

    /// 成功 drain 的 commit 中携带 damage / damage_buffer evidence 的数量。
    pub damage_observations: usize,

    /// 成功 drain 的 commit 中累计 surface-coordinate damage rectangle 数量。
    pub surface_damage_rects: usize,

    /// 成功 drain 的 commit 中累计 buffer-coordinate damage rectangle 数量。
    pub buffer_damage_rects: usize,

    /// 成功 drain 的 commit 中携带 frame callback request evidence 的数量。
    pub frame_callback_observations: usize,

    /// 成功 drain 的 commit 中累计 frame callback request 数量。
    pub frame_callback_count: usize,

    /// 按 FIFO drain 顺序保存的 render-dirty/readiness 纯数据意图。
    pub render_dirty_readiness_intents: Vec<RuntimeSurfaceCommitRenderDirtyReadinessIntent>,

    /// render-dirty intent runtime queue drain seam 被调用的次数。
    pub render_dirty_queue_drain_invocations: usize,

    /// 成功入队到 runtime-owned render-dirty queue 的 intent 数量。
    pub render_dirty_intents_enqueued: usize,

    /// 成功从 runtime-owned render-dirty queue drain 的 intent 数量。
    pub render_dirty_intents_drained: usize,

    /// 按 FIFO drain 顺序保存的 runtime queue render-dirty/readiness intent。
    pub render_dirty_queue_drained_intents: Vec<RuntimeSurfaceCommitRenderDirtyReadinessIntent>,

    /// runtime queue drain 是否 import buffer；Phase 54H 固定保持 false。
    pub render_dirty_queue_buffer_imported: bool,

    /// runtime queue drain 是否创建 texture；Phase 54H 固定保持 false。
    pub render_dirty_queue_texture_created: bool,

    /// runtime queue drain 是否提交 render；Phase 54H 固定保持 false。
    pub render_dirty_queue_render_submitted: bool,

    /// runtime queue drain 是否发送 frame callback done；Phase 54H 固定保持 false。
    pub render_dirty_queue_frame_callback_done_sent: bool,

    /// runtime queue drain 是否接入 input；Phase 54H 固定保持 false。
    pub render_dirty_queue_input_support: bool,

    /// renderer-admission seam 被调用的次数。
    pub renderer_admission_invocations: usize,

    /// 从 drained render-dirty intent 成功创建的 renderer work intent 数量。
    pub renderer_work_intents_created: usize,

    /// 按 FIFO 顺序保存的 renderer-admission pure-data work intent。
    pub renderer_work_intents: Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// renderer-admission seam 是否 import buffer；Phase 54I 固定保持 false。
    pub renderer_admission_buffer_imported: bool,

    /// renderer-admission seam 是否创建 texture；Phase 54I 固定保持 false。
    pub renderer_admission_texture_created: bool,

    /// renderer-admission seam 是否提交 render；Phase 54I 固定保持 false。
    pub renderer_admission_render_submitted: bool,

    /// renderer-admission seam 是否提交 damage；Phase 54I 固定保持 false。
    pub renderer_admission_damage_submitted: bool,

    /// renderer-admission seam 是否发送 frame callback done；Phase 54I 固定保持 false。
    pub renderer_admission_frame_callback_done_sent: bool,

    /// renderer-admission seam 是否接入 input；Phase 54I 固定保持 false。
    pub renderer_admission_input_support: bool,

    /// renderer-admission seam 是否触发 core mutation；Phase 54I 固定保持 false。
    pub renderer_admission_core_mutation_invoked: bool,

    /// renderer owner boundary seam 被调用的次数。
    pub renderer_owner_boundary_invocations: usize,

    /// renderer owner boundary 成功消费的 work intent 数量。
    pub renderer_owner_work_intents_consumed: usize,

    /// 按 FIFO 顺序保存的 renderer owner boundary consumed work intents。
    pub renderer_owner_consumed_work_intents: Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// renderer owner boundary 是否缺少真实 renderer owner。
    pub renderer_owner_missing_renderer_owner: bool,

    /// renderer owner boundary 是否缺少 buffer importer。
    pub renderer_owner_missing_buffer_importer: bool,

    /// renderer owner boundary 是否缺少 texture support。
    pub renderer_owner_missing_texture_support: bool,

    /// renderer owner boundary 是否 import buffer；Phase 54J 固定保持 false。
    pub renderer_owner_buffer_imported: bool,

    /// renderer owner boundary 是否创建 texture；Phase 54J 固定保持 false。
    pub renderer_owner_texture_created: bool,

    /// renderer owner boundary 是否调用 renderer；Phase 54J 固定保持 false。
    pub renderer_owner_renderer_called: bool,

    /// renderer owner boundary 是否提交 damage；Phase 54J 固定保持 false。
    pub renderer_owner_damage_submitted: bool,

    /// renderer owner boundary 是否发送 frame callback done；Phase 54J 固定保持 false。
    pub renderer_owner_frame_callback_done_sent: bool,

    /// renderer owner boundary 是否接入 input；Phase 54J 固定保持 false。
    pub renderer_owner_input_support: bool,

    /// renderer owner boundary 是否触发 core mutation；Phase 54J 固定保持 false。
    pub renderer_owner_core_mutation_invoked: bool,

    /// renderer owner shell readiness seam 被调用的次数。
    pub renderer_owner_shell_readiness_invocations: usize,

    /// renderer owner shell readiness 观察到的 work intent 数量。
    pub renderer_owner_shell_work_intents_observed: usize,

    /// 按 FIFO 顺序保存的 renderer owner shell observed work intents。
    pub renderer_owner_shell_observed_work_intents:
        Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// runtime-owned renderer owner shell 是否可用。
    pub renderer_owner_shell_available: bool,

    /// renderer owner shell readiness 是否仍缺少 renderer owner。
    pub renderer_owner_shell_missing_renderer_owner: bool,

    /// renderer owner shell readiness 是否仍缺少 buffer importer。
    pub renderer_owner_shell_missing_buffer_importer: bool,

    /// renderer owner shell readiness 是否仍缺少 texture support。
    pub renderer_owner_shell_missing_texture_support: bool,

    /// renderer owner shell readiness 是否 import buffer；Phase 54K 固定保持 false。
    pub renderer_owner_shell_buffer_imported: bool,

    /// renderer owner shell readiness 是否创建 texture；Phase 54K 固定保持 false。
    pub renderer_owner_shell_texture_created: bool,

    /// renderer owner shell readiness 是否调用 renderer；Phase 54K 固定保持 false。
    pub renderer_owner_shell_renderer_called: bool,

    /// renderer owner shell readiness 是否提交 damage；Phase 54K 固定保持 false。
    pub renderer_owner_shell_damage_submitted: bool,

    /// renderer owner shell readiness 是否发送 frame callback done；Phase 54K 固定保持 false。
    pub renderer_owner_shell_frame_callback_done_sent: bool,

    /// renderer owner shell readiness 是否接入 input；Phase 54K 固定保持 false。
    pub renderer_owner_shell_input_support: bool,

    /// renderer owner shell readiness 是否触发 core mutation；Phase 54K 固定保持 false。
    pub renderer_owner_shell_core_mutation_invoked: bool,

    /// buffer importer shell readiness seam 被调用的次数。
    pub buffer_importer_shell_readiness_invocations: usize,

    /// buffer importer shell readiness 观察到的 work intent 数量。
    pub buffer_importer_shell_work_intents_observed: usize,

    /// 按 FIFO 顺序保存的 buffer importer shell observed work intents。
    pub buffer_importer_shell_observed_work_intents:
        Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// runtime-owned buffer importer shell 是否可用。
    pub buffer_importer_shell_available: bool,

    /// buffer importer shell readiness 是否仍缺少 renderer owner shell。
    pub buffer_importer_shell_missing_renderer_owner_shell: bool,

    /// buffer importer shell readiness 是否仍缺少 buffer importer。
    pub buffer_importer_shell_missing_buffer_importer: bool,

    /// buffer importer shell readiness 是否仍缺少 texture support。
    pub buffer_importer_shell_missing_texture_support: bool,

    /// buffer importer shell readiness 是否 import buffer；Phase 54L 固定保持 false。
    pub buffer_importer_shell_buffer_imported: bool,

    /// buffer importer shell readiness 是否创建 texture；Phase 54L 固定保持 false。
    pub buffer_importer_shell_texture_created: bool,

    /// buffer importer shell readiness 是否调用 renderer；Phase 54L 固定保持 false。
    pub buffer_importer_shell_renderer_called: bool,

    /// buffer importer shell readiness 是否提交 damage；Phase 54L 固定保持 false。
    pub buffer_importer_shell_damage_submitted: bool,

    /// buffer importer shell readiness 是否发送 frame callback done；Phase 54L 固定保持 false。
    pub buffer_importer_shell_frame_callback_done_sent: bool,

    /// buffer importer shell readiness 是否接入 input；Phase 54L 固定保持 false。
    pub buffer_importer_shell_input_support: bool,

    /// buffer importer shell readiness 是否触发 core mutation；Phase 54L 固定保持 false。
    pub buffer_importer_shell_core_mutation_invoked: bool,

    /// texture support shell readiness seam 被调用的次数。
    pub texture_support_shell_readiness_invocations: usize,

    /// texture support shell readiness 观察到的 work intent 数量。
    pub texture_support_shell_work_intents_observed: usize,

    /// 按 FIFO 顺序保存的 texture support shell observed work intents。
    pub texture_support_shell_observed_work_intents:
        Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>,

    /// runtime-owned texture support shell 是否可用。
    pub texture_support_shell_available: bool,

    /// texture support shell readiness 是否仍缺少 buffer importer shell。
    pub texture_support_shell_missing_buffer_importer_shell: bool,

    /// texture support shell readiness 是否仍缺少 texture support。
    pub texture_support_shell_missing_texture_support: bool,

    /// texture support shell readiness 是否 import buffer；Phase 54M 固定保持 false。
    pub texture_support_shell_buffer_imported: bool,

    /// texture support shell readiness 是否创建 texture；Phase 54M 固定保持 false。
    pub texture_support_shell_texture_created: bool,

    /// texture support shell readiness 是否调用 renderer；Phase 54M 固定保持 false。
    pub texture_support_shell_renderer_called: bool,

    /// texture support shell readiness 是否提交 damage；Phase 54M 固定保持 false。
    pub texture_support_shell_damage_submitted: bool,

    /// texture support shell readiness 是否发送 frame callback done；Phase 54M 固定保持 false。
    pub texture_support_shell_frame_callback_done_sent: bool,

    /// texture support shell readiness 是否接入 input；Phase 54M 固定保持 false。
    pub texture_support_shell_input_support: bool,

    /// texture support shell readiness 是否触发 core mutation；Phase 54M 固定保持 false。
    pub texture_support_shell_core_mutation_invoked: bool,

    /// render operation readiness seam 被调用的次数。
    pub render_operation_readiness_invocations: usize,

    /// 成功创建的 render operation / render execution readiness intent 数量。
    pub render_operation_intents_created: usize,

    /// 按 FIFO 顺序保存的 render operation 纯数据意图。
    pub render_operation_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// render operation readiness 是否 import buffer；Phase 54N 固定保持 false。
    pub render_operation_buffer_imported: bool,

    /// render operation readiness 是否创建 texture；Phase 54N 固定保持 false。
    pub render_operation_texture_created: bool,

    /// render operation readiness 是否调用 renderer；Phase 54N 固定保持 false。
    pub render_operation_renderer_called: bool,

    /// render operation readiness 是否提交 damage；Phase 54N 固定保持 false。
    pub render_operation_damage_submitted: bool,

    /// render operation readiness 是否发送 frame callback done；Phase 54N 固定保持 false。
    pub render_operation_frame_callback_done_sent: bool,

    /// render operation readiness 是否接入 input；Phase 54N 固定保持 false。
    pub render_operation_input_support: bool,

    /// render operation readiness 是否触发 core mutation；Phase 54N 固定保持 false。
    pub render_operation_core_mutation_invoked: bool,

    /// render operation intent runtime queue drain seam 被调用的次数。
    pub render_operation_queue_drain_invocations: usize,

    /// 成功入队到 runtime-owned render operation queue 的 intent 数量。
    pub render_operation_intents_enqueued: usize,

    /// 成功从 runtime-owned render operation queue drain 的 intent 数量。
    pub render_operation_intents_drained: usize,

    /// 按 FIFO drain 顺序保存的 runtime queue render operation intent。
    pub render_operation_queue_drained_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// render operation queue drain 是否 import buffer；Phase 54O 固定保持 false。
    pub render_operation_queue_buffer_imported: bool,

    /// render operation queue drain 是否创建 texture；Phase 54O 固定保持 false。
    pub render_operation_queue_texture_created: bool,

    /// render operation queue drain 是否调用 renderer；Phase 54O 固定保持 false。
    pub render_operation_queue_renderer_called: bool,

    /// render operation queue drain 是否提交 damage；Phase 54O 固定保持 false。
    pub render_operation_queue_damage_submitted: bool,

    /// render operation queue drain 是否发送 frame callback done；Phase 54O 固定保持 false。
    pub render_operation_queue_frame_callback_done_sent: bool,

    /// render operation queue drain 是否接入 input；Phase 54O 固定保持 false。
    pub render_operation_queue_input_support: bool,

    /// render operation queue drain 是否触发 core mutation；Phase 54O 固定保持 false。
    pub render_operation_queue_core_mutation_invoked: bool,

    /// render execution owner boundary seam 被调用的次数。
    pub render_execution_owner_boundary_invocations: usize,

    /// render execution owner boundary 成功消费的 render operation intent 数量。
    pub render_execution_owner_intents_consumed: usize,

    /// 按 FIFO 顺序保存的 render execution owner boundary consumed intents。
    pub render_execution_owner_consumed_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// render execution owner boundary 是否缺少真实 execution owner。
    pub render_execution_owner_missing_owner: bool,

    /// render execution owner boundary 是否缺少 buffer import。
    pub render_execution_owner_missing_buffer_import: bool,

    /// render execution owner boundary 是否缺少 texture creation。
    pub render_execution_owner_missing_texture_creation: bool,

    /// render execution owner boundary 是否缺少 renderer call。
    pub render_execution_owner_missing_renderer_call: bool,

    /// render execution owner boundary 是否缺少 damage submit。
    pub render_execution_owner_missing_damage_submit: bool,

    /// render execution owner boundary 是否缺少 frame callback done。
    pub render_execution_owner_missing_frame_callback_done: bool,

    /// render execution owner boundary 是否 import buffer；Phase 54P 固定保持 false。
    pub render_execution_owner_buffer_imported: bool,

    /// render execution owner boundary 是否创建 texture；Phase 54P 固定保持 false。
    pub render_execution_owner_texture_created: bool,

    /// render execution owner boundary 是否调用 renderer；Phase 54P 固定保持 false。
    pub render_execution_owner_renderer_called: bool,

    /// render execution owner boundary 是否提交 damage；Phase 54P 固定保持 false。
    pub render_execution_owner_damage_submitted: bool,

    /// render execution owner boundary 是否发送 frame callback done；Phase 54P 固定保持 false。
    pub render_execution_owner_frame_callback_done_sent: bool,

    /// render execution owner boundary 是否接入 input；Phase 54P 固定保持 false。
    pub render_execution_owner_input_support: bool,

    /// render execution owner boundary 是否触发 core mutation；Phase 54P 固定保持 false。
    pub render_execution_owner_core_mutation_invoked: bool,

    /// render execution owner shell readiness seam 被调用的次数。
    pub render_execution_owner_shell_readiness_invocations: usize,

    /// render execution owner shell readiness 观察到的 intent 数量。
    pub render_execution_owner_shell_intents_observed: usize,

    /// 按 FIFO 顺序保存的 render execution owner shell observed intents。
    pub render_execution_owner_shell_observed_intents:
        Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned render execution owner shell 是否可用。
    pub render_execution_owner_shell_available: bool,

    /// render execution owner shell readiness 是否仍缺少 buffer import。
    pub render_execution_owner_shell_missing_buffer_import: bool,

    /// render execution owner shell readiness 是否仍缺少 texture creation。
    pub render_execution_owner_shell_missing_texture_creation: bool,

    /// render execution owner shell readiness 是否仍缺少 renderer call。
    pub render_execution_owner_shell_missing_renderer_call: bool,

    /// render execution owner shell readiness 是否仍缺少 damage submit。
    pub render_execution_owner_shell_missing_damage_submit: bool,

    /// render execution owner shell readiness 是否仍缺少 frame callback done。
    pub render_execution_owner_shell_missing_frame_callback_done: bool,

    /// render execution owner shell readiness 是否 import buffer；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_buffer_imported: bool,

    /// render execution owner shell readiness 是否创建 texture；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_texture_created: bool,

    /// render execution owner shell readiness 是否调用 renderer；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_renderer_called: bool,

    /// render execution owner shell readiness 是否提交 damage；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_damage_submitted: bool,

    /// render execution owner shell readiness 是否发送 frame callback done；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_frame_callback_done_sent: bool,

    /// render execution owner shell readiness 是否接入 input；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_input_support: bool,

    /// render execution owner shell readiness 是否触发 core mutation；Phase 54Q 固定保持 false。
    pub render_execution_owner_shell_core_mutation_invoked: bool,

    /// render pipeline skeleton readiness seam 被调用的次数。
    pub render_pipeline_skeleton_readiness_invocations: usize,

    /// render pipeline skeleton readiness 观察到的 intent 数量。
    pub render_pipeline_skeleton_intents_observed: usize,

    /// 按 FIFO 顺序保存的 render pipeline skeleton observed intents。
    pub render_pipeline_skeleton_observed_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned render pipeline skeleton owner 是否可用。
    pub render_pipeline_skeleton_owner_available: bool,

    /// render pipeline skeleton 是否仍缺少上游 execution owner shell。
    pub render_pipeline_skeleton_missing_execution_owner_shell: bool,

    /// render pipeline skeleton 是否仍缺少 buffer import。
    pub render_pipeline_skeleton_missing_buffer_import: bool,

    /// render pipeline skeleton 是否仍缺少 texture creation。
    pub render_pipeline_skeleton_missing_texture_creation: bool,

    /// render pipeline skeleton 是否仍缺少 renderer call。
    pub render_pipeline_skeleton_missing_renderer_call: bool,

    /// render pipeline skeleton 是否仍缺少 damage submit。
    pub render_pipeline_skeleton_missing_damage_submit: bool,

    /// render pipeline skeleton 是否仍缺少 frame callback done。
    pub render_pipeline_skeleton_missing_frame_callback_done: bool,

    /// render pipeline skeleton 是否 import buffer；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_buffer_imported: bool,

    /// render pipeline skeleton 是否创建 texture；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_texture_created: bool,

    /// render pipeline skeleton 是否调用 renderer；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_renderer_called: bool,

    /// render pipeline skeleton 是否提交 damage；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_damage_submitted: bool,

    /// render pipeline skeleton 是否发送 frame callback done；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_frame_callback_done_sent: bool,

    /// render pipeline skeleton 是否接入 input；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_input_support: bool,

    /// render pipeline skeleton 是否触发 core mutation；Phase 55A 固定保持 false。
    pub render_pipeline_skeleton_core_mutation_invoked: bool,

    /// render backend capability report seam 被调用的次数。
    pub render_backend_capability_report_invocations: usize,

    /// render backend capability report 观察到的 intent 数量。
    pub render_backend_capability_intents_observed: usize,

    /// 按 FIFO 顺序保存的 render backend capability observed intents。
    pub render_backend_capability_observed_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned render backend capability owner 是否可用。
    pub render_backend_capability_owner_available: bool,

    /// 是否已注册真实 renderer backend；Phase 55B 固定保持 false。
    pub render_backend_capability_backend_registered: bool,

    /// 已注册 renderer backend 种类；Phase 55B 固定保持 None。
    pub render_backend_capability_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// render backend capability 是否仍缺少 pipeline skeleton。
    pub render_backend_capability_missing_pipeline_skeleton: bool,

    /// render backend capability 是否仍缺少 renderer backend registration。
    pub render_backend_capability_missing_backend_registration: bool,

    /// render backend capability 是否仍缺少 buffer import。
    pub render_backend_capability_missing_buffer_import: bool,

    /// render backend capability 是否仍缺少 texture creation。
    pub render_backend_capability_missing_texture_creation: bool,

    /// render backend capability 是否仍缺少 renderer call。
    pub render_backend_capability_missing_renderer_call: bool,

    /// render backend capability 是否仍缺少 damage submit。
    pub render_backend_capability_missing_damage_submit: bool,

    /// render backend capability 是否仍缺少 frame callback done。
    pub render_backend_capability_missing_frame_callback_done: bool,

    /// render backend capability 是否 import buffer；Phase 55B 固定保持 false。
    pub render_backend_capability_buffer_imported: bool,

    /// render backend capability 是否创建 texture；Phase 55B 固定保持 false。
    pub render_backend_capability_texture_created: bool,

    /// render backend capability 是否调用 renderer；Phase 55B 固定保持 false。
    pub render_backend_capability_renderer_called: bool,

    /// render backend capability 是否提交 damage；Phase 55B 固定保持 false。
    pub render_backend_capability_damage_submitted: bool,

    /// render backend capability 是否发送 frame callback done；Phase 55B 固定保持 false。
    pub render_backend_capability_frame_callback_done_sent: bool,

    /// render backend capability 是否接入 input；Phase 55B 固定保持 false。
    pub render_backend_capability_input_support: bool,

    /// render backend capability 是否触发 core mutation；Phase 55B 固定保持 false。
    pub render_backend_capability_core_mutation_invoked: bool,

    /// renderer backend registration descriptor seam 被调用的次数。
    pub renderer_backend_registration_invocations: usize,

    /// renderer backend registration descriptor 观察到的 intent 数量。
    pub renderer_backend_registration_intents_observed: usize,

    /// 按 FIFO 顺序保存的 renderer backend registration observed intents。
    pub renderer_backend_registration_observed_intents:
        Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned renderer backend registration owner 是否可用。
    pub renderer_backend_registration_owner_available: bool,

    /// 是否注册 renderer backend descriptor；不代表真实 renderer 已可调用。
    pub renderer_backend_registration_backend_registered: bool,

    /// renderer backend descriptor 是否可用。
    pub renderer_backend_registration_descriptor_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub renderer_backend_registration_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// renderer backend registration 是否仍缺少 backend capability owner。
    pub renderer_backend_registration_missing_backend_capability: bool,

    /// renderer backend registration 是否仍缺少 buffer import。
    pub renderer_backend_registration_missing_buffer_import: bool,

    /// renderer backend registration 是否仍缺少 texture creation。
    pub renderer_backend_registration_missing_texture_creation: bool,

    /// renderer backend registration 是否仍缺少 renderer call。
    pub renderer_backend_registration_missing_renderer_call: bool,

    /// renderer backend registration 是否仍缺少 damage submit。
    pub renderer_backend_registration_missing_damage_submit: bool,

    /// renderer backend registration 是否仍缺少 frame callback done。
    pub renderer_backend_registration_missing_frame_callback_done: bool,

    /// renderer backend registration 是否 import buffer；Phase 55C 固定保持 false。
    pub renderer_backend_registration_buffer_imported: bool,

    /// renderer backend registration 是否创建 texture；Phase 55C 固定保持 false。
    pub renderer_backend_registration_texture_created: bool,

    /// renderer backend registration 是否调用 renderer；Phase 55C 固定保持 false。
    pub renderer_backend_registration_renderer_called: bool,

    /// renderer backend registration 是否提交 damage；Phase 55C 固定保持 false。
    pub renderer_backend_registration_damage_submitted: bool,

    /// renderer backend registration 是否发送 frame callback done；Phase 55C 固定保持 false。
    pub renderer_backend_registration_frame_callback_done_sent: bool,

    /// renderer backend registration 是否接入 input；Phase 55C 固定保持 false。
    pub renderer_backend_registration_input_support: bool,

    /// renderer backend registration 是否触发 core mutation；Phase 55C 固定保持 false。
    pub renderer_backend_registration_core_mutation_invoked: bool,

    /// renderer backend owner shell readiness seam 被调用的次数。
    pub renderer_backend_owner_shell_readiness_invocations: usize,

    /// renderer backend owner shell readiness 观察到的 intent 数量。
    pub renderer_backend_owner_shell_intents_observed: usize,

    /// 按 FIFO 顺序保存的 renderer backend owner shell observed intents。
    pub renderer_backend_owner_shell_observed_intents:
        Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned renderer backend owner shell 是否可用。
    pub renderer_backend_owner_shell_available: bool,

    /// renderer backend owner shell 是否绑定 descriptor。
    pub renderer_backend_owner_shell_bound: bool,

    /// renderer backend owner shell 是否观察到 descriptor 可用。
    pub renderer_backend_owner_shell_descriptor_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub renderer_backend_owner_shell_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// renderer backend owner shell 是否仍缺少 descriptor。
    pub renderer_backend_owner_shell_missing_descriptor: bool,

    /// renderer backend owner shell 是否仍缺少 buffer import。
    pub renderer_backend_owner_shell_missing_buffer_import: bool,

    /// renderer backend owner shell 是否仍缺少 texture creation。
    pub renderer_backend_owner_shell_missing_texture_creation: bool,

    /// renderer backend owner shell 是否仍缺少 renderer call。
    pub renderer_backend_owner_shell_missing_renderer_call: bool,

    /// renderer backend owner shell 是否仍缺少 damage submit。
    pub renderer_backend_owner_shell_missing_damage_submit: bool,

    /// renderer backend owner shell 是否仍缺少 frame callback done。
    pub renderer_backend_owner_shell_missing_frame_callback_done: bool,

    /// renderer backend owner shell 是否 import buffer；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_buffer_imported: bool,

    /// renderer backend owner shell 是否创建 texture；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_texture_created: bool,

    /// renderer backend owner shell 是否调用 renderer；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_renderer_called: bool,

    /// renderer backend owner shell 是否提交 damage；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_damage_submitted: bool,

    /// renderer backend owner shell 是否发送 frame callback done；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_frame_callback_done_sent: bool,

    /// renderer backend owner shell 是否接入 input；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_input_support: bool,

    /// renderer backend owner shell 是否触发 core mutation；Phase 55D 固定保持 false。
    pub renderer_backend_owner_shell_core_mutation_invoked: bool,

    /// buffer import resource owner boundary seam 被调用的次数。
    pub buffer_import_resource_owner_readiness_invocations: usize,

    /// buffer import resource owner boundary 观察到的 intent 数量。
    pub buffer_import_resource_owner_intents_observed: usize,

    /// 按 FIFO 顺序保存的 buffer import resource owner observed intents。
    pub buffer_import_resource_owner_observed_intents:
        Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// runtime-owned buffer importer owner boundary 是否可用。
    pub buffer_importer_owner_available: bool,

    /// buffer importer owner boundary 是否绑定 renderer backend owner shell。
    pub buffer_importer_owner_bound: bool,

    /// buffer importer owner 是否观察到 renderer backend descriptor evidence。
    pub buffer_import_resource_owner_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub buffer_import_resource_owner_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// buffer importer owner 是否仍缺少 renderer backend owner shell。
    pub buffer_import_resource_owner_missing_renderer_backend_owner_shell: bool,

    /// buffer importer owner 是否仍缺少 renderer backend descriptor evidence。
    pub buffer_import_resource_owner_missing_descriptor_evidence: bool,

    /// buffer importer owner 是否仍缺少真实 buffer import implementation。
    pub buffer_import_resource_owner_missing_actual_buffer_import: bool,

    /// buffer importer owner 是否仍缺少 texture creation。
    pub buffer_import_resource_owner_missing_texture_creation: bool,

    /// buffer importer owner 是否仍缺少 renderer call。
    pub buffer_import_resource_owner_missing_renderer_call: bool,

    /// buffer importer owner 是否仍缺少 damage submit。
    pub buffer_import_resource_owner_missing_damage_submit: bool,

    /// buffer importer owner 是否仍缺少 frame callback done。
    pub buffer_import_resource_owner_missing_frame_callback_done: bool,

    /// buffer importer owner 是否 import buffer；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_buffer_imported: bool,

    /// buffer importer owner 是否创建 texture；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_texture_created: bool,

    /// buffer importer owner 是否调用 renderer；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_renderer_called: bool,

    /// buffer importer owner 是否提交 damage；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_damage_submitted: bool,

    /// buffer importer owner 是否发送 frame callback done；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_frame_callback_done_sent: bool,

    /// buffer importer owner 是否接入 input；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_input_support: bool,

    /// buffer importer owner 是否触发 core mutation；Phase 55E 固定保持 false。
    pub buffer_import_resource_owner_core_mutation_invoked: bool,

    /// buffer import planning seam 被调用的次数。
    pub buffer_import_planning_invocations: usize,

    /// buffer import planning 观察到的 intent 数量。
    pub buffer_import_planning_intents_observed: usize,

    /// 按 FIFO 顺序保存的 buffer import planning observed intents。
    pub buffer_import_planning_observed_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>,

    /// buffer import planning seam 是否可用。
    pub buffer_import_plan_available: bool,

    /// buffer import planning 是否为 observed intent 建立 pure-data plan。
    pub buffer_import_plan_built: bool,

    /// 观察到 buffer import candidate evidence 的数量。
    pub buffer_import_candidates_observed: usize,

    /// 规划为未来需要 buffer import 的数量。
    pub buffer_import_required_count: usize,

    /// buffer import planning 是否观察到 renderer backend descriptor evidence。
    pub buffer_import_planning_descriptor_evidence_available: bool,

    /// 已注册 renderer backend descriptor 的种类。
    pub buffer_import_planning_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// buffer import planning 是否仍缺少 buffer importer owner。
    pub buffer_import_planning_missing_buffer_importer_owner: bool,

    /// buffer import planning 是否仍缺少 renderer backend descriptor evidence。
    pub buffer_import_planning_missing_descriptor_evidence: bool,

    /// buffer import planning 是否仍缺少 buffer import candidate evidence。
    pub buffer_import_planning_missing_candidate: bool,

    /// buffer import planning 是否仍缺少真实 buffer import implementation。
    pub buffer_import_planning_missing_actual_buffer_import: bool,

    /// buffer import planning 是否仍缺少 texture creation。
    pub buffer_import_planning_missing_texture_creation: bool,

    /// buffer import planning 是否仍缺少 renderer call。
    pub buffer_import_planning_missing_renderer_call: bool,

    /// buffer import planning 是否仍缺少 damage submit。
    pub buffer_import_planning_missing_damage_submit: bool,

    /// buffer import planning 是否仍缺少 frame callback done。
    pub buffer_import_planning_missing_frame_callback_done: bool,

    /// buffer import planning 是否 import buffer；Phase 55F 固定保持 false。
    pub buffer_import_planning_buffer_imported: bool,

    /// buffer import planning 是否创建 texture；Phase 55F 固定保持 false。
    pub buffer_import_planning_texture_created: bool,

    /// buffer import planning 是否调用 renderer；Phase 55F 固定保持 false。
    pub buffer_import_planning_renderer_called: bool,

    /// buffer import planning 是否提交 damage；Phase 55F 固定保持 false。
    pub buffer_import_planning_damage_submitted: bool,

    /// buffer import planning 是否发送 frame callback done；Phase 55F 固定保持 false。
    pub buffer_import_planning_frame_callback_done_sent: bool,

    /// buffer import planning 是否接入 input；Phase 55F 固定保持 false。
    pub buffer_import_planning_input_support: bool,

    /// buffer import planning 是否触发 core mutation；Phase 55F 固定保持 false。
    pub buffer_import_planning_core_mutation_invoked: bool,

    /// buffer import implementation descriptor boundary seam 被调用的次数。
    pub buffer_import_implementation_boundary_invocations: usize,

    /// buffer import implementation boundary 观察到 descriptor 的数量。
    pub buffer_import_implementation_descriptors_observed: usize,

    /// 按 FIFO 顺序保存的 buffer import implementation descriptors。
    pub buffer_import_implementation_observed_descriptors:
        Vec<RuntimeSurfaceCommitBufferImportImplementationDescriptor>,

    /// buffer import implementation descriptor boundary 是否可用。
    pub buffer_import_implementation_descriptor_available: bool,

    /// buffer import implementation descriptor 是否注册成功。
    pub buffer_import_implementation_descriptor_registered: bool,

    /// implementation descriptor boundary 观察到 candidate evidence 的数量。
    pub buffer_import_implementation_candidates_observed: usize,

    /// implementation descriptor boundary 观察到 actual import required 的数量。
    pub buffer_import_implementation_actual_required_count: usize,

    /// implementation descriptor boundary 是否观察到 importer owner evidence。
    pub buffer_import_implementation_importer_owner_evidence_available: bool,

    /// implementation descriptor boundary 是否观察到 renderer backend descriptor evidence。
    pub buffer_import_implementation_renderer_descriptor_evidence_available: bool,

    /// implementation descriptor boundary 观察到的 renderer backend kind。
    pub buffer_import_implementation_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// implementation descriptor boundary 是否仍缺少 planning intent。
    pub buffer_import_implementation_missing_planning_intent: bool,

    /// implementation descriptor boundary 是否仍缺少 import plan。
    pub buffer_import_implementation_missing_plan: bool,

    /// implementation descriptor boundary 是否仍缺少 importer owner evidence。
    pub buffer_import_implementation_missing_importer_owner_evidence: bool,

    /// implementation descriptor boundary 是否仍缺少 renderer backend descriptor evidence。
    pub buffer_import_implementation_missing_renderer_descriptor_evidence: bool,

    /// implementation descriptor boundary 是否仍缺少 candidate evidence。
    pub buffer_import_implementation_missing_candidate: bool,

    /// implementation descriptor boundary 是否仍缺少真实 buffer import。
    pub buffer_import_implementation_missing_actual_buffer_import: bool,

    /// implementation descriptor boundary 是否仍缺少 texture creation。
    pub buffer_import_implementation_missing_texture_creation: bool,

    /// implementation descriptor boundary 是否仍缺少 renderer call。
    pub buffer_import_implementation_missing_renderer_call: bool,

    /// implementation descriptor boundary 是否仍缺少 damage submit。
    pub buffer_import_implementation_missing_damage_submit: bool,

    /// implementation descriptor boundary 是否仍缺少 frame callback done。
    pub buffer_import_implementation_missing_frame_callback_done: bool,

    /// implementation descriptor boundary 是否尝试 import buffer；Phase 55G 固定保持 false。
    pub buffer_import_implementation_buffer_import_attempted: bool,

    /// implementation descriptor boundary 是否 import buffer；Phase 55G 固定保持 false。
    pub buffer_import_implementation_buffer_imported: bool,

    /// implementation descriptor boundary 是否创建 texture；Phase 55G 固定保持 false。
    pub buffer_import_implementation_texture_created: bool,

    /// implementation descriptor boundary 是否调用 renderer；Phase 55G 固定保持 false。
    pub buffer_import_implementation_renderer_called: bool,

    /// implementation descriptor boundary 是否提交 damage；Phase 55G 固定保持 false。
    pub buffer_import_implementation_damage_submitted: bool,

    /// implementation descriptor boundary 是否发送 frame callback done；Phase 55G 固定保持 false。
    pub buffer_import_implementation_frame_callback_done_sent: bool,

    /// implementation descriptor boundary 是否接入 input；Phase 55G 固定保持 false。
    pub buffer_import_implementation_input_support: bool,

    /// implementation descriptor boundary 是否触发 core mutation；Phase 55G 固定保持 false。
    pub buffer_import_implementation_core_mutation_invoked: bool,

    /// buffer import adapter proof boundary seam 被调用的次数。
    pub buffer_import_adapter_proof_boundary_invocations: usize,

    /// buffer import adapter proof boundary 观察到 proof 的数量。
    pub buffer_import_adapter_proofs_observed: usize,

    /// 按 FIFO 顺序保存的 buffer import adapter proofs。
    pub buffer_import_adapter_observed_proofs: Vec<RuntimeSurfaceCommitBufferImportAdapterProof>,

    /// buffer import adapter proof boundary 是否可用。
    pub buffer_import_adapter_proof_boundary_available: bool,

    /// buffer import adapter proof 是否注册成功。
    pub buffer_import_adapter_proof_registered: bool,

    /// adapter proof boundary 观察到 candidate evidence 的数量。
    pub buffer_import_adapter_candidates_observed: usize,

    /// adapter proof boundary 观察到 actual import required 的数量。
    pub buffer_import_adapter_actual_required_count: usize,

    /// adapter proof boundary 是否观察到 importer owner evidence。
    pub buffer_import_adapter_importer_owner_evidence_available: bool,

    /// adapter proof boundary 是否观察到 renderer backend descriptor evidence。
    pub buffer_import_adapter_renderer_descriptor_evidence_available: bool,

    /// adapter proof boundary 观察到的 renderer backend kind。
    pub buffer_import_adapter_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// adapter proof boundary 是否仍缺少 implementation descriptor。
    pub buffer_import_adapter_missing_implementation_descriptor: bool,

    /// adapter proof boundary 是否仍缺少 importer owner evidence。
    pub buffer_import_adapter_missing_importer_owner_evidence: bool,

    /// adapter proof boundary 是否仍缺少 renderer backend descriptor evidence。
    pub buffer_import_adapter_missing_renderer_descriptor_evidence: bool,

    /// adapter proof boundary 是否仍缺少 candidate evidence。
    pub buffer_import_adapter_missing_candidate: bool,

    /// adapter proof boundary 是否仍缺少真实 buffer import。
    pub buffer_import_adapter_missing_actual_buffer_import: bool,

    /// adapter proof boundary 是否仍缺少 texture creation。
    pub buffer_import_adapter_missing_texture_creation: bool,

    /// adapter proof boundary 是否仍缺少 renderer call。
    pub buffer_import_adapter_missing_renderer_call: bool,

    /// adapter proof boundary 是否仍缺少 damage submit。
    pub buffer_import_adapter_missing_damage_submit: bool,

    /// adapter proof boundary 是否仍缺少 frame callback done。
    pub buffer_import_adapter_missing_frame_callback_done: bool,

    /// adapter proof boundary 是否尝试 import buffer；Phase 55H 固定保持 false。
    pub buffer_import_adapter_buffer_import_attempted: bool,

    /// adapter proof boundary 是否 import buffer；Phase 55H 固定保持 false。
    pub buffer_import_adapter_buffer_imported: bool,

    /// adapter proof boundary 是否创建 texture；Phase 55H 固定保持 false。
    pub buffer_import_adapter_texture_created: bool,

    /// adapter proof boundary 是否调用 renderer；Phase 55H 固定保持 false。
    pub buffer_import_adapter_renderer_called: bool,

    /// adapter proof boundary 是否提交 damage；Phase 55H 固定保持 false。
    pub buffer_import_adapter_damage_submitted: bool,

    /// adapter proof boundary 是否发送 frame callback done；Phase 55H 固定保持 false。
    pub buffer_import_adapter_frame_callback_done_sent: bool,

    /// adapter proof boundary 是否接入 input；Phase 55H 固定保持 false。
    pub buffer_import_adapter_input_support: bool,

    /// adapter proof boundary 是否触发 core mutation；Phase 55H 固定保持 false。
    pub buffer_import_adapter_core_mutation_invoked: bool,

    /// buffer import precondition gate seam 被调用的次数。
    pub buffer_import_precondition_gate_invocations: usize,

    /// precondition gate 观察到 adapter proof 的数量。
    pub buffer_import_precondition_adapter_proofs_observed: usize,

    /// 按 FIFO 顺序保存的 precondition gate observed adapter proofs。
    pub buffer_import_precondition_observed_adapter_proofs:
        Vec<RuntimeSurfaceCommitBufferImportAdapterProof>,

    /// buffer import precondition gate 是否可用。
    pub buffer_import_precondition_gate_available: bool,

    /// 已满足未来真实 import 最小前置条件的数量。
    pub buffer_import_preconditions_met_count: usize,

    /// 已满足未来 import 调度前置条件的数量。
    pub buffer_import_future_preconditions_met_count: usize,

    /// precondition gate 观察到 candidate evidence 的数量。
    pub buffer_import_precondition_candidates_observed: usize,

    /// precondition gate 观察到 actual import required 的数量。
    pub buffer_import_precondition_actual_required_count: usize,

    /// precondition gate 是否观察到 importer owner evidence。
    pub buffer_import_precondition_importer_owner_evidence_available: bool,

    /// precondition gate 是否观察到 renderer backend descriptor evidence。
    pub buffer_import_precondition_renderer_descriptor_evidence_available: bool,

    /// precondition gate 观察到的 renderer backend kind。
    pub buffer_import_precondition_registered_backend_kind:
        Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// precondition gate 是否缺少 adapter proof。
    pub buffer_import_precondition_missing_adapter_proof: bool,

    /// precondition gate 是否缺少 registered adapter proof。
    pub buffer_import_precondition_missing_registered_adapter_proof: bool,

    /// precondition gate 是否缺少 importer owner evidence。
    pub buffer_import_precondition_missing_importer_owner_evidence: bool,

    /// precondition gate 是否缺少 renderer backend descriptor evidence。
    pub buffer_import_precondition_missing_renderer_descriptor_evidence: bool,

    /// precondition gate 是否缺少 candidate evidence。
    pub buffer_import_precondition_missing_candidate: bool,

    /// precondition gate 是否缺少 actual import requirement。
    pub buffer_import_precondition_missing_actual_import_requirement: bool,

    /// precondition gate 是否仍缺少真实 buffer import。
    pub buffer_import_precondition_missing_actual_buffer_import: bool,

    /// precondition gate 是否仍缺少 texture creation。
    pub buffer_import_precondition_missing_texture_creation: bool,

    /// precondition gate 是否仍缺少 renderer call。
    pub buffer_import_precondition_missing_renderer_call: bool,

    /// precondition gate 是否仍缺少 damage submit。
    pub buffer_import_precondition_missing_damage_submit: bool,

    /// precondition gate 是否仍缺少 frame callback done。
    pub buffer_import_precondition_missing_frame_callback_done: bool,

    /// precondition gate 是否尝试 import buffer；Phase 55I 固定保持 false。
    pub buffer_import_precondition_buffer_import_attempted: bool,

    /// precondition gate 是否 import buffer；Phase 55I 固定保持 false。
    pub buffer_import_precondition_buffer_imported: bool,

    /// precondition gate 是否创建 texture；Phase 55I 固定保持 false。
    pub buffer_import_precondition_texture_created: bool,

    /// precondition gate 是否调用 renderer；Phase 55I 固定保持 false。
    pub buffer_import_precondition_renderer_called: bool,

    /// precondition gate 是否提交 damage；Phase 55I 固定保持 false。
    pub buffer_import_precondition_damage_submitted: bool,

    /// precondition gate 是否发送 frame callback done；Phase 55I 固定保持 false。
    pub buffer_import_precondition_frame_callback_done_sent: bool,

    /// precondition gate 是否接入 input；Phase 55I 固定保持 false。
    pub buffer_import_precondition_input_support: bool,

    /// precondition gate 是否触发 core mutation；Phase 55I 固定保持 false。
    pub buffer_import_precondition_core_mutation_invoked: bool,

    /// buffer import execution dry-run seam 被调用的次数。
    pub buffer_import_execution_dry_run_invocations: usize,

    /// 按 FIFO 顺序保存的 buffer import execution dry-run reports。
    pub buffer_import_execution_dry_run_reports:
        Vec<RuntimeSurfaceCommitBufferImportExecutionDryRunReport>,

    /// execution guard 是否可用。
    pub buffer_import_execution_guard_available: bool,

    /// execution dry-run 是否尝试真实 execution 的数量；Phase 55J 应保持 0。
    pub buffer_import_execution_attempted_count: usize,

    /// execution dry-run no-op 的数量。
    pub buffer_import_execution_noop_count: usize,

    /// execution dry-run blocked 的数量。
    pub buffer_import_execution_blocked_count: usize,

    /// execution dry-run 观察到 actual import required 的数量。
    pub buffer_import_execution_actual_required_count: usize,

    /// execution dry-run 观察到 preconditions met 的数量。
    pub buffer_import_execution_preconditions_met_count: usize,

    /// execution dry-run 是否缺少 precondition gate evidence。
    pub buffer_import_execution_missing_precondition_gate_evidence: bool,

    /// execution dry-run 是否缺少 adapter proof。
    pub buffer_import_execution_missing_adapter_proof: bool,

    /// execution dry-run 是否缺少 import preconditions。
    pub buffer_import_execution_missing_import_preconditions: bool,

    /// execution dry-run 是否缺少真实 buffer importer implementation。
    pub buffer_import_execution_missing_real_importer: bool,

    /// execution dry-run 是否因无需 actual import 而 no-op。
    pub buffer_import_execution_no_actual_import_required: bool,

    /// execution dry-run 是否仍缺少 texture creation。
    pub buffer_import_execution_missing_texture_creation: bool,

    /// execution dry-run 是否仍缺少 renderer call。
    pub buffer_import_execution_missing_renderer_call: bool,

    /// execution dry-run 是否仍缺少 damage submit。
    pub buffer_import_execution_missing_damage_submit: bool,

    /// execution dry-run 是否仍缺少 frame callback done。
    pub buffer_import_execution_missing_frame_callback_done: bool,

    /// execution dry-run 是否尝试 import buffer；Phase 55J 固定保持 false。
    pub buffer_import_execution_buffer_import_attempted: bool,

    /// execution dry-run 是否 import buffer；Phase 55J 固定保持 false。
    pub buffer_import_execution_buffer_imported: bool,

    /// execution dry-run 是否创建 texture；Phase 55J 固定保持 false。
    pub buffer_import_execution_texture_created: bool,

    /// execution dry-run 是否调用 renderer；Phase 55J 固定保持 false。
    pub buffer_import_execution_renderer_called: bool,

    /// execution dry-run 是否提交 damage；Phase 55J 固定保持 false。
    pub buffer_import_execution_damage_submitted: bool,

    /// execution dry-run 是否发送 frame callback done；Phase 55J 固定保持 false。
    pub buffer_import_execution_frame_callback_done_sent: bool,

    /// execution dry-run 是否接入 input；Phase 55J 固定保持 false。
    pub buffer_import_execution_input_support: bool,

    /// execution dry-run 是否触发 core mutation；Phase 55J 固定保持 false。
    pub buffer_import_execution_core_mutation_invoked: bool,

    /// buffer import implementation owner shell seam 被调用的次数。
    pub buffer_import_implementation_owner_shell_invocations: usize,

    /// 按 FIFO 顺序保存的 buffer import implementation owner shell reports。
    pub buffer_import_implementation_owner_shell_reports:
        Vec<RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport>,

    /// implementation owner shell 是否可用。
    pub buffer_import_implementation_owner_shell_available: bool,

    /// 真实 buffer import implementation 是否可用；Phase 55K 固定保持 false。
    pub buffer_import_real_implementation_available: bool,

    /// actual import attempt 被允许的数量；Phase 55K 应保持 0。
    pub buffer_import_actual_attempt_admitted_count: usize,

    /// actual import attempt 被阻止的数量。
    pub buffer_import_actual_attempt_blocked_count: usize,

    /// implementation owner shell 观察到 actual import required 的数量。
    pub buffer_import_implementation_owner_actual_required_count: usize,

    /// implementation owner shell 观察到 execution dry-run report 的数量。
    pub buffer_import_implementation_owner_execution_reports_observed: usize,

    /// implementation owner shell 是否缺少 execution dry-run report。
    pub buffer_import_implementation_owner_missing_execution_dry_run_report: bool,

    /// implementation owner shell 是否因 execution dry-run blocked 而 blocked。
    pub buffer_import_implementation_owner_execution_dry_run_blocked: bool,

    /// implementation owner shell 是否因无需 actual import 而 no-op。
    pub buffer_import_implementation_owner_no_actual_import_required: bool,

    /// implementation owner shell 是否缺少真实 buffer importer implementation。
    pub buffer_import_implementation_owner_missing_real_importer: bool,

    /// implementation owner shell 是否仍缺少 texture creation。
    pub buffer_import_implementation_owner_missing_texture_creation: bool,

    /// implementation owner shell 是否仍缺少 renderer call。
    pub buffer_import_implementation_owner_missing_renderer_call: bool,

    /// implementation owner shell 是否仍缺少 damage submit。
    pub buffer_import_implementation_owner_missing_damage_submit: bool,

    /// implementation owner shell 是否仍缺少 frame callback done。
    pub buffer_import_implementation_owner_missing_frame_callback_done: bool,

    /// implementation owner shell 是否尝试 import buffer；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_buffer_import_attempted: bool,

    /// implementation owner shell 是否 import buffer；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_buffer_imported: bool,

    /// implementation owner shell 是否创建 texture；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_texture_created: bool,

    /// implementation owner shell 是否调用 renderer；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_renderer_called: bool,

    /// implementation owner shell 是否提交 damage；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_damage_submitted: bool,

    /// implementation owner shell 是否发送 frame callback done；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_frame_callback_done_sent: bool,

    /// implementation owner shell 是否接入 input；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_input_support: bool,

    /// implementation owner shell 是否触发 core mutation；Phase 55K 固定保持 false。
    pub buffer_import_implementation_owner_core_mutation_invoked: bool,

    /// actual attempt record seam 被调用的次数。
    pub buffer_import_actual_attempt_record_invocations: usize,

    /// 按 FIFO 顺序保存的 actual buffer import attempt records。
    pub buffer_import_actual_attempt_records:
        Vec<RuntimeSurfaceCommitBufferImportActualAttemptRecord>,

    /// actual attempt record 是否可用。
    pub buffer_import_actual_attempt_record_available: bool,

    /// actual attempt record 被记录的数量。
    pub buffer_import_actual_attempt_recorded_count: usize,

    /// actual import attempt admission 被检查的数量。
    pub buffer_import_actual_attempt_admission_checked_count: usize,

    /// actual import attempt 被允许的数量；Phase 55L 应保持 0。
    pub buffer_import_actual_attempt_record_admitted_count: usize,

    /// actual import attempt 被阻止的数量。
    pub buffer_import_actual_attempt_record_blocked_count: usize,

    /// actual attempt record 观察到 actual import required 的数量。
    pub buffer_import_actual_attempt_required_count: usize,

    /// actual attempt record 是否缺少 implementation owner shell report。
    pub buffer_import_actual_attempt_missing_owner_shell_report: bool,

    /// actual attempt record 是否因 owner shell blocked 而 blocked。
    pub buffer_import_actual_attempt_owner_shell_blocked: bool,

    /// actual attempt record 是否因无需 actual import 而 no-op。
    pub buffer_import_actual_attempt_no_actual_import_required: bool,

    /// actual attempt record 是否缺少 admission。
    pub buffer_import_actual_attempt_missing_admission: bool,

    /// actual attempt record 是否缺少真实 buffer importer implementation。
    pub buffer_import_actual_attempt_missing_real_importer: bool,

    /// actual attempt record 是否仍缺少 texture creation。
    pub buffer_import_actual_attempt_missing_texture_creation: bool,

    /// actual attempt record 是否仍缺少 renderer call。
    pub buffer_import_actual_attempt_missing_renderer_call: bool,

    /// actual attempt record 是否仍缺少 damage submit。
    pub buffer_import_actual_attempt_missing_damage_submit: bool,

    /// actual attempt record 是否仍缺少 frame callback done。
    pub buffer_import_actual_attempt_missing_frame_callback_done: bool,

    /// actual attempt record 是否尝试 import buffer；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_buffer_import_attempted: bool,

    /// actual attempt record 是否 import buffer；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_buffer_imported: bool,

    /// actual attempt record 是否创建 texture；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_texture_created: bool,

    /// actual attempt record 是否调用 renderer；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_renderer_called: bool,

    /// actual attempt record 是否提交 damage；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_damage_submitted: bool,

    /// actual attempt record 是否发送 frame callback done；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_frame_callback_done_sent: bool,

    /// actual attempt record 是否接入 input；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_input_support: bool,

    /// actual attempt record 是否触发 core mutation；Phase 55L 固定保持 false。
    pub buffer_import_actual_attempt_core_mutation_invoked: bool,

    /// 是否处理 buffer attach；本阶段固定保持 false。
    pub buffer_attached: bool,

    /// 是否处理 damage；本阶段固定保持 false。
    pub damage_submitted: bool,

    /// 是否处理/request frame callback；本阶段固定保持 false。
    pub frame_callback_requested: bool,

    /// 是否调用 render；本阶段固定保持 false。
    pub render_invoked: bool,

    /// 是否调用 input；本阶段固定保持 false。
    pub input_invoked: bool,

    /// 是否调用 admission ledger 或 core mutation；本阶段固定保持 false。
    pub core_mutation_invoked: bool,
}

impl NestedRuntimeSurfaceCommitRunSummary {
    fn from_surface_commit_drain(report: &RuntimeSurfaceCommitDrainReport) -> Self {
        Self {
            drain_invocations: usize::from(report.drain_invoked),
            commit_observations_drained: usize::from(report.commit_observation_resolved),
            commit_observation_errors: usize::from(report.commit_observation_failed),
            drained_commit_sequences: report.commit_sequence.into_iter().collect(),
            buffer_attach_observations: usize::from(report.buffer_attach_observed),
            buffer_presence_observations: usize::from(report.buffer_present),
            buffer_removed_observations: usize::from(report.buffer_removed),
            renderable_buffer_observations: usize::from(report.renderable_buffer),
            damage_observations: usize::from(report.damage_observed),
            surface_damage_rects: report.surface_damage_rects,
            buffer_damage_rects: report.buffer_damage_rects,
            frame_callback_observations: usize::from(report.frame_callback_observed),
            frame_callback_count: report.frame_callback_count,
            render_dirty_readiness_intents: render_dirty_readiness_intent_from_commit_drain_report(
                report,
            )
            .into_iter()
            .collect(),
            render_dirty_queue_drain_invocations: 0,
            render_dirty_intents_enqueued: 0,
            render_dirty_intents_drained: 0,
            render_dirty_queue_drained_intents: Vec::new(),
            render_dirty_queue_buffer_imported: false,
            render_dirty_queue_texture_created: false,
            render_dirty_queue_render_submitted: false,
            render_dirty_queue_frame_callback_done_sent: false,
            render_dirty_queue_input_support: false,
            renderer_admission_invocations: 0,
            renderer_work_intents_created: 0,
            renderer_work_intents: Vec::new(),
            renderer_admission_buffer_imported: false,
            renderer_admission_texture_created: false,
            renderer_admission_render_submitted: false,
            renderer_admission_damage_submitted: false,
            renderer_admission_frame_callback_done_sent: false,
            renderer_admission_input_support: false,
            renderer_admission_core_mutation_invoked: false,
            renderer_owner_boundary_invocations: 0,
            renderer_owner_work_intents_consumed: 0,
            renderer_owner_consumed_work_intents: Vec::new(),
            renderer_owner_missing_renderer_owner: false,
            renderer_owner_missing_buffer_importer: false,
            renderer_owner_missing_texture_support: false,
            renderer_owner_buffer_imported: false,
            renderer_owner_texture_created: false,
            renderer_owner_renderer_called: false,
            renderer_owner_damage_submitted: false,
            renderer_owner_frame_callback_done_sent: false,
            renderer_owner_input_support: false,
            renderer_owner_core_mutation_invoked: false,
            renderer_owner_shell_readiness_invocations: 0,
            renderer_owner_shell_work_intents_observed: 0,
            renderer_owner_shell_observed_work_intents: Vec::new(),
            renderer_owner_shell_available: false,
            renderer_owner_shell_missing_renderer_owner: false,
            renderer_owner_shell_missing_buffer_importer: false,
            renderer_owner_shell_missing_texture_support: false,
            renderer_owner_shell_buffer_imported: false,
            renderer_owner_shell_texture_created: false,
            renderer_owner_shell_renderer_called: false,
            renderer_owner_shell_damage_submitted: false,
            renderer_owner_shell_frame_callback_done_sent: false,
            renderer_owner_shell_input_support: false,
            renderer_owner_shell_core_mutation_invoked: false,
            buffer_importer_shell_readiness_invocations: 0,
            buffer_importer_shell_work_intents_observed: 0,
            buffer_importer_shell_observed_work_intents: Vec::new(),
            buffer_importer_shell_available: false,
            buffer_importer_shell_missing_renderer_owner_shell: false,
            buffer_importer_shell_missing_buffer_importer: false,
            buffer_importer_shell_missing_texture_support: false,
            buffer_importer_shell_buffer_imported: false,
            buffer_importer_shell_texture_created: false,
            buffer_importer_shell_renderer_called: false,
            buffer_importer_shell_damage_submitted: false,
            buffer_importer_shell_frame_callback_done_sent: false,
            buffer_importer_shell_input_support: false,
            buffer_importer_shell_core_mutation_invoked: false,
            texture_support_shell_readiness_invocations: 0,
            texture_support_shell_work_intents_observed: 0,
            texture_support_shell_observed_work_intents: Vec::new(),
            texture_support_shell_available: false,
            texture_support_shell_missing_buffer_importer_shell: false,
            texture_support_shell_missing_texture_support: false,
            texture_support_shell_buffer_imported: false,
            texture_support_shell_texture_created: false,
            texture_support_shell_renderer_called: false,
            texture_support_shell_damage_submitted: false,
            texture_support_shell_frame_callback_done_sent: false,
            texture_support_shell_input_support: false,
            texture_support_shell_core_mutation_invoked: false,
            render_operation_readiness_invocations: 0,
            render_operation_intents_created: 0,
            render_operation_intents: Vec::new(),
            render_operation_buffer_imported: false,
            render_operation_texture_created: false,
            render_operation_renderer_called: false,
            render_operation_damage_submitted: false,
            render_operation_frame_callback_done_sent: false,
            render_operation_input_support: false,
            render_operation_core_mutation_invoked: false,
            render_operation_queue_drain_invocations: 0,
            render_operation_intents_enqueued: 0,
            render_operation_intents_drained: 0,
            render_operation_queue_drained_intents: Vec::new(),
            render_operation_queue_buffer_imported: false,
            render_operation_queue_texture_created: false,
            render_operation_queue_renderer_called: false,
            render_operation_queue_damage_submitted: false,
            render_operation_queue_frame_callback_done_sent: false,
            render_operation_queue_input_support: false,
            render_operation_queue_core_mutation_invoked: false,
            render_execution_owner_boundary_invocations: 0,
            render_execution_owner_intents_consumed: 0,
            render_execution_owner_consumed_intents: Vec::new(),
            render_execution_owner_missing_owner: false,
            render_execution_owner_missing_buffer_import: false,
            render_execution_owner_missing_texture_creation: false,
            render_execution_owner_missing_renderer_call: false,
            render_execution_owner_missing_damage_submit: false,
            render_execution_owner_missing_frame_callback_done: false,
            render_execution_owner_buffer_imported: false,
            render_execution_owner_texture_created: false,
            render_execution_owner_renderer_called: false,
            render_execution_owner_damage_submitted: false,
            render_execution_owner_frame_callback_done_sent: false,
            render_execution_owner_input_support: false,
            render_execution_owner_core_mutation_invoked: false,
            render_execution_owner_shell_readiness_invocations: 0,
            render_execution_owner_shell_intents_observed: 0,
            render_execution_owner_shell_observed_intents: Vec::new(),
            render_execution_owner_shell_available: false,
            render_execution_owner_shell_missing_buffer_import: false,
            render_execution_owner_shell_missing_texture_creation: false,
            render_execution_owner_shell_missing_renderer_call: false,
            render_execution_owner_shell_missing_damage_submit: false,
            render_execution_owner_shell_missing_frame_callback_done: false,
            render_execution_owner_shell_buffer_imported: false,
            render_execution_owner_shell_texture_created: false,
            render_execution_owner_shell_renderer_called: false,
            render_execution_owner_shell_damage_submitted: false,
            render_execution_owner_shell_frame_callback_done_sent: false,
            render_execution_owner_shell_input_support: false,
            render_execution_owner_shell_core_mutation_invoked: false,
            render_pipeline_skeleton_readiness_invocations: 0,
            render_pipeline_skeleton_intents_observed: 0,
            render_pipeline_skeleton_observed_intents: Vec::new(),
            render_pipeline_skeleton_owner_available: false,
            render_pipeline_skeleton_missing_execution_owner_shell: false,
            render_pipeline_skeleton_missing_buffer_import: false,
            render_pipeline_skeleton_missing_texture_creation: false,
            render_pipeline_skeleton_missing_renderer_call: false,
            render_pipeline_skeleton_missing_damage_submit: false,
            render_pipeline_skeleton_missing_frame_callback_done: false,
            render_pipeline_skeleton_buffer_imported: false,
            render_pipeline_skeleton_texture_created: false,
            render_pipeline_skeleton_renderer_called: false,
            render_pipeline_skeleton_damage_submitted: false,
            render_pipeline_skeleton_frame_callback_done_sent: false,
            render_pipeline_skeleton_input_support: false,
            render_pipeline_skeleton_core_mutation_invoked: false,
            render_backend_capability_report_invocations: 0,
            render_backend_capability_intents_observed: 0,
            render_backend_capability_observed_intents: Vec::new(),
            render_backend_capability_owner_available: false,
            render_backend_capability_backend_registered: false,
            render_backend_capability_backend_kind: None,
            render_backend_capability_missing_pipeline_skeleton: false,
            render_backend_capability_missing_backend_registration: false,
            render_backend_capability_missing_buffer_import: false,
            render_backend_capability_missing_texture_creation: false,
            render_backend_capability_missing_renderer_call: false,
            render_backend_capability_missing_damage_submit: false,
            render_backend_capability_missing_frame_callback_done: false,
            render_backend_capability_buffer_imported: false,
            render_backend_capability_texture_created: false,
            render_backend_capability_renderer_called: false,
            render_backend_capability_damage_submitted: false,
            render_backend_capability_frame_callback_done_sent: false,
            render_backend_capability_input_support: false,
            render_backend_capability_core_mutation_invoked: false,
            renderer_backend_registration_invocations: 0,
            renderer_backend_registration_intents_observed: 0,
            renderer_backend_registration_observed_intents: Vec::new(),
            renderer_backend_registration_owner_available: false,
            renderer_backend_registration_backend_registered: false,
            renderer_backend_registration_descriptor_available: false,
            renderer_backend_registration_registered_backend_kind: None,
            renderer_backend_registration_missing_backend_capability: false,
            renderer_backend_registration_missing_buffer_import: false,
            renderer_backend_registration_missing_texture_creation: false,
            renderer_backend_registration_missing_renderer_call: false,
            renderer_backend_registration_missing_damage_submit: false,
            renderer_backend_registration_missing_frame_callback_done: false,
            renderer_backend_registration_buffer_imported: false,
            renderer_backend_registration_texture_created: false,
            renderer_backend_registration_renderer_called: false,
            renderer_backend_registration_damage_submitted: false,
            renderer_backend_registration_frame_callback_done_sent: false,
            renderer_backend_registration_input_support: false,
            renderer_backend_registration_core_mutation_invoked: false,
            renderer_backend_owner_shell_readiness_invocations: 0,
            renderer_backend_owner_shell_intents_observed: 0,
            renderer_backend_owner_shell_observed_intents: Vec::new(),
            renderer_backend_owner_shell_available: false,
            renderer_backend_owner_shell_bound: false,
            renderer_backend_owner_shell_descriptor_available: false,
            renderer_backend_owner_shell_registered_backend_kind: None,
            renderer_backend_owner_shell_missing_descriptor: false,
            renderer_backend_owner_shell_missing_buffer_import: false,
            renderer_backend_owner_shell_missing_texture_creation: false,
            renderer_backend_owner_shell_missing_renderer_call: false,
            renderer_backend_owner_shell_missing_damage_submit: false,
            renderer_backend_owner_shell_missing_frame_callback_done: false,
            renderer_backend_owner_shell_buffer_imported: false,
            renderer_backend_owner_shell_texture_created: false,
            renderer_backend_owner_shell_renderer_called: false,
            renderer_backend_owner_shell_damage_submitted: false,
            renderer_backend_owner_shell_frame_callback_done_sent: false,
            renderer_backend_owner_shell_input_support: false,
            renderer_backend_owner_shell_core_mutation_invoked: false,
            buffer_import_resource_owner_readiness_invocations: 0,
            buffer_import_resource_owner_intents_observed: 0,
            buffer_import_resource_owner_observed_intents: Vec::new(),
            buffer_importer_owner_available: false,
            buffer_importer_owner_bound: false,
            buffer_import_resource_owner_descriptor_evidence_available: false,
            buffer_import_resource_owner_registered_backend_kind: None,
            buffer_import_resource_owner_missing_renderer_backend_owner_shell: false,
            buffer_import_resource_owner_missing_descriptor_evidence: false,
            buffer_import_resource_owner_missing_actual_buffer_import: false,
            buffer_import_resource_owner_missing_texture_creation: false,
            buffer_import_resource_owner_missing_renderer_call: false,
            buffer_import_resource_owner_missing_damage_submit: false,
            buffer_import_resource_owner_missing_frame_callback_done: false,
            buffer_import_resource_owner_buffer_imported: false,
            buffer_import_resource_owner_texture_created: false,
            buffer_import_resource_owner_renderer_called: false,
            buffer_import_resource_owner_damage_submitted: false,
            buffer_import_resource_owner_frame_callback_done_sent: false,
            buffer_import_resource_owner_input_support: false,
            buffer_import_resource_owner_core_mutation_invoked: false,
            buffer_import_planning_invocations: 0,
            buffer_import_planning_intents_observed: 0,
            buffer_import_planning_observed_intents: Vec::new(),
            buffer_import_plan_available: false,
            buffer_import_plan_built: false,
            buffer_import_candidates_observed: 0,
            buffer_import_required_count: 0,
            buffer_import_planning_descriptor_evidence_available: false,
            buffer_import_planning_registered_backend_kind: None,
            buffer_import_planning_missing_buffer_importer_owner: false,
            buffer_import_planning_missing_descriptor_evidence: false,
            buffer_import_planning_missing_candidate: false,
            buffer_import_planning_missing_actual_buffer_import: false,
            buffer_import_planning_missing_texture_creation: false,
            buffer_import_planning_missing_renderer_call: false,
            buffer_import_planning_missing_damage_submit: false,
            buffer_import_planning_missing_frame_callback_done: false,
            buffer_import_planning_buffer_imported: false,
            buffer_import_planning_texture_created: false,
            buffer_import_planning_renderer_called: false,
            buffer_import_planning_damage_submitted: false,
            buffer_import_planning_frame_callback_done_sent: false,
            buffer_import_planning_input_support: false,
            buffer_import_planning_core_mutation_invoked: false,
            buffer_import_implementation_boundary_invocations: 0,
            buffer_import_implementation_descriptors_observed: 0,
            buffer_import_implementation_observed_descriptors: Vec::new(),
            buffer_import_implementation_descriptor_available: false,
            buffer_import_implementation_descriptor_registered: false,
            buffer_import_implementation_candidates_observed: 0,
            buffer_import_implementation_actual_required_count: 0,
            buffer_import_implementation_importer_owner_evidence_available: false,
            buffer_import_implementation_renderer_descriptor_evidence_available: false,
            buffer_import_implementation_registered_backend_kind: None,
            buffer_import_implementation_missing_planning_intent: false,
            buffer_import_implementation_missing_plan: false,
            buffer_import_implementation_missing_importer_owner_evidence: false,
            buffer_import_implementation_missing_renderer_descriptor_evidence: false,
            buffer_import_implementation_missing_candidate: false,
            buffer_import_implementation_missing_actual_buffer_import: false,
            buffer_import_implementation_missing_texture_creation: false,
            buffer_import_implementation_missing_renderer_call: false,
            buffer_import_implementation_missing_damage_submit: false,
            buffer_import_implementation_missing_frame_callback_done: false,
            buffer_import_implementation_buffer_import_attempted: false,
            buffer_import_implementation_buffer_imported: false,
            buffer_import_implementation_texture_created: false,
            buffer_import_implementation_renderer_called: false,
            buffer_import_implementation_damage_submitted: false,
            buffer_import_implementation_frame_callback_done_sent: false,
            buffer_import_implementation_input_support: false,
            buffer_import_implementation_core_mutation_invoked: false,
            buffer_import_adapter_proof_boundary_invocations: 0,
            buffer_import_adapter_proofs_observed: 0,
            buffer_import_adapter_observed_proofs: Vec::new(),
            buffer_import_adapter_proof_boundary_available: false,
            buffer_import_adapter_proof_registered: false,
            buffer_import_adapter_candidates_observed: 0,
            buffer_import_adapter_actual_required_count: 0,
            buffer_import_adapter_importer_owner_evidence_available: false,
            buffer_import_adapter_renderer_descriptor_evidence_available: false,
            buffer_import_adapter_registered_backend_kind: None,
            buffer_import_adapter_missing_implementation_descriptor: false,
            buffer_import_adapter_missing_importer_owner_evidence: false,
            buffer_import_adapter_missing_renderer_descriptor_evidence: false,
            buffer_import_adapter_missing_candidate: false,
            buffer_import_adapter_missing_actual_buffer_import: false,
            buffer_import_adapter_missing_texture_creation: false,
            buffer_import_adapter_missing_renderer_call: false,
            buffer_import_adapter_missing_damage_submit: false,
            buffer_import_adapter_missing_frame_callback_done: false,
            buffer_import_adapter_buffer_import_attempted: false,
            buffer_import_adapter_buffer_imported: false,
            buffer_import_adapter_texture_created: false,
            buffer_import_adapter_renderer_called: false,
            buffer_import_adapter_damage_submitted: false,
            buffer_import_adapter_frame_callback_done_sent: false,
            buffer_import_adapter_input_support: false,
            buffer_import_adapter_core_mutation_invoked: false,
            buffer_import_precondition_gate_invocations: 0,
            buffer_import_precondition_adapter_proofs_observed: 0,
            buffer_import_precondition_observed_adapter_proofs: Vec::new(),
            buffer_import_precondition_gate_available: false,
            buffer_import_preconditions_met_count: 0,
            buffer_import_future_preconditions_met_count: 0,
            buffer_import_precondition_candidates_observed: 0,
            buffer_import_precondition_actual_required_count: 0,
            buffer_import_precondition_importer_owner_evidence_available: false,
            buffer_import_precondition_renderer_descriptor_evidence_available: false,
            buffer_import_precondition_registered_backend_kind: None,
            buffer_import_precondition_missing_adapter_proof: false,
            buffer_import_precondition_missing_registered_adapter_proof: false,
            buffer_import_precondition_missing_importer_owner_evidence: false,
            buffer_import_precondition_missing_renderer_descriptor_evidence: false,
            buffer_import_precondition_missing_candidate: false,
            buffer_import_precondition_missing_actual_import_requirement: false,
            buffer_import_precondition_missing_actual_buffer_import: false,
            buffer_import_precondition_missing_texture_creation: false,
            buffer_import_precondition_missing_renderer_call: false,
            buffer_import_precondition_missing_damage_submit: false,
            buffer_import_precondition_missing_frame_callback_done: false,
            buffer_import_precondition_buffer_import_attempted: false,
            buffer_import_precondition_buffer_imported: false,
            buffer_import_precondition_texture_created: false,
            buffer_import_precondition_renderer_called: false,
            buffer_import_precondition_damage_submitted: false,
            buffer_import_precondition_frame_callback_done_sent: false,
            buffer_import_precondition_input_support: false,
            buffer_import_precondition_core_mutation_invoked: false,
            buffer_import_execution_dry_run_invocations: 0,
            buffer_import_execution_dry_run_reports: Vec::new(),
            buffer_import_execution_guard_available: false,
            buffer_import_execution_attempted_count: 0,
            buffer_import_execution_noop_count: 0,
            buffer_import_execution_blocked_count: 0,
            buffer_import_execution_actual_required_count: 0,
            buffer_import_execution_preconditions_met_count: 0,
            buffer_import_execution_missing_precondition_gate_evidence: false,
            buffer_import_execution_missing_adapter_proof: false,
            buffer_import_execution_missing_import_preconditions: false,
            buffer_import_execution_missing_real_importer: false,
            buffer_import_execution_no_actual_import_required: false,
            buffer_import_execution_missing_texture_creation: false,
            buffer_import_execution_missing_renderer_call: false,
            buffer_import_execution_missing_damage_submit: false,
            buffer_import_execution_missing_frame_callback_done: false,
            buffer_import_execution_buffer_import_attempted: false,
            buffer_import_execution_buffer_imported: false,
            buffer_import_execution_texture_created: false,
            buffer_import_execution_renderer_called: false,
            buffer_import_execution_damage_submitted: false,
            buffer_import_execution_frame_callback_done_sent: false,
            buffer_import_execution_input_support: false,
            buffer_import_execution_core_mutation_invoked: false,
            buffer_import_implementation_owner_shell_invocations: 0,
            buffer_import_implementation_owner_shell_reports: Vec::new(),
            buffer_import_implementation_owner_shell_available: false,
            buffer_import_real_implementation_available: false,
            buffer_import_actual_attempt_admitted_count: 0,
            buffer_import_actual_attempt_blocked_count: 0,
            buffer_import_implementation_owner_actual_required_count: 0,
            buffer_import_implementation_owner_execution_reports_observed: 0,
            buffer_import_implementation_owner_missing_execution_dry_run_report: false,
            buffer_import_implementation_owner_execution_dry_run_blocked: false,
            buffer_import_implementation_owner_no_actual_import_required: false,
            buffer_import_implementation_owner_missing_real_importer: false,
            buffer_import_implementation_owner_missing_texture_creation: false,
            buffer_import_implementation_owner_missing_renderer_call: false,
            buffer_import_implementation_owner_missing_damage_submit: false,
            buffer_import_implementation_owner_missing_frame_callback_done: false,
            buffer_import_implementation_owner_buffer_import_attempted: false,
            buffer_import_implementation_owner_buffer_imported: false,
            buffer_import_implementation_owner_texture_created: false,
            buffer_import_implementation_owner_renderer_called: false,
            buffer_import_implementation_owner_damage_submitted: false,
            buffer_import_implementation_owner_frame_callback_done_sent: false,
            buffer_import_implementation_owner_input_support: false,
            buffer_import_implementation_owner_core_mutation_invoked: false,
            buffer_import_actual_attempt_record_invocations: 0,
            buffer_import_actual_attempt_records: Vec::new(),
            buffer_import_actual_attempt_record_available: false,
            buffer_import_actual_attempt_recorded_count: 0,
            buffer_import_actual_attempt_admission_checked_count: 0,
            buffer_import_actual_attempt_record_admitted_count: 0,
            buffer_import_actual_attempt_record_blocked_count: 0,
            buffer_import_actual_attempt_required_count: 0,
            buffer_import_actual_attempt_missing_owner_shell_report: false,
            buffer_import_actual_attempt_owner_shell_blocked: false,
            buffer_import_actual_attempt_no_actual_import_required: false,
            buffer_import_actual_attempt_missing_admission: false,
            buffer_import_actual_attempt_missing_real_importer: false,
            buffer_import_actual_attempt_missing_texture_creation: false,
            buffer_import_actual_attempt_missing_renderer_call: false,
            buffer_import_actual_attempt_missing_damage_submit: false,
            buffer_import_actual_attempt_missing_frame_callback_done: false,
            buffer_import_actual_attempt_buffer_import_attempted: false,
            buffer_import_actual_attempt_buffer_imported: false,
            buffer_import_actual_attempt_texture_created: false,
            buffer_import_actual_attempt_renderer_called: false,
            buffer_import_actual_attempt_damage_submitted: false,
            buffer_import_actual_attempt_frame_callback_done_sent: false,
            buffer_import_actual_attempt_input_support: false,
            buffer_import_actual_attempt_core_mutation_invoked: false,
            buffer_attached: report.buffer_attached,
            damage_submitted: report.damage_submitted,
            frame_callback_requested: report.frame_callback_requested,
            render_invoked: report.render_invoked,
            input_invoked: report.input_invoked,
            core_mutation_invoked: report.core_mutation_invoked,
        }
    }

    fn from_render_dirty_intent_drain(
        report: &RuntimeSurfaceCommitRenderDirtyIntentDrainReport,
    ) -> Self {
        Self {
            render_dirty_queue_drain_invocations: usize::from(report.drain_invoked),
            render_dirty_intents_enqueued: usize::from(report.intent_enqueued),
            render_dirty_intents_drained: usize::from(report.intent_drained),
            render_dirty_queue_drained_intents: report.drained_intent.clone().into_iter().collect(),
            render_dirty_queue_buffer_imported: report.buffer_imported,
            render_dirty_queue_texture_created: report.texture_created,
            render_dirty_queue_render_submitted: report.render_submitted,
            render_dirty_queue_frame_callback_done_sent: report.frame_callback_done_sent,
            render_dirty_queue_input_support: report.input_support,
            ..Self::default()
        }
    }

    fn from_renderer_admission(report: &RuntimeSurfaceCommitRendererAdmissionReport) -> Self {
        Self {
            renderer_admission_invocations: usize::from(report.renderer_admission_invoked),
            renderer_work_intents_created: usize::from(report.work_intent_created),
            renderer_work_intents: report.work_intent.clone().into_iter().collect(),
            renderer_admission_buffer_imported: report.buffer_imported,
            renderer_admission_texture_created: report.texture_created,
            renderer_admission_render_submitted: report.render_submitted,
            renderer_admission_damage_submitted: report.damage_submitted,
            renderer_admission_frame_callback_done_sent: report.frame_callback_done_sent,
            renderer_admission_input_support: report.input_support,
            renderer_admission_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_renderer_owner_boundary(
        report: &RuntimeSurfaceCommitRendererOwnerBoundaryReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            renderer_owner_boundary_invocations: usize::from(report.consume_invoked),
            renderer_owner_work_intents_consumed: usize::from(report.work_intent_consumed),
            renderer_owner_consumed_work_intents: report
                .consumed_work_intent
                .clone()
                .into_iter()
                .collect(),
            renderer_owner_missing_renderer_owner: has_blocker(
                RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingRendererOwner,
            ),
            renderer_owner_missing_buffer_importer: has_blocker(
                RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingBufferImporter,
            ),
            renderer_owner_missing_texture_support: has_blocker(
                RuntimeSurfaceCommitRendererOwnerBoundaryBlocker::MissingTextureSupport,
            ),
            renderer_owner_buffer_imported: report.buffer_imported,
            renderer_owner_texture_created: report.texture_created,
            renderer_owner_renderer_called: report.renderer_called,
            renderer_owner_damage_submitted: report.damage_submitted,
            renderer_owner_frame_callback_done_sent: report.frame_callback_done_sent,
            renderer_owner_input_support: report.input_support,
            renderer_owner_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_renderer_owner_shell_readiness(
        report: &RuntimeSurfaceCommitRendererOwnerShellReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            renderer_owner_shell_readiness_invocations: usize::from(report.readiness_invoked),
            renderer_owner_shell_work_intents_observed: usize::from(
                report.observed_work_intent.is_some(),
            ),
            renderer_owner_shell_observed_work_intents: report
                .observed_work_intent
                .clone()
                .into_iter()
                .collect(),
            renderer_owner_shell_available: report.renderer_owner_shell_available,
            renderer_owner_shell_missing_renderer_owner: has_blocker(
                RuntimeSurfaceCommitRendererOwnerShellBlocker::MissingRendererOwner,
            ),
            renderer_owner_shell_missing_buffer_importer: has_blocker(
                RuntimeSurfaceCommitRendererOwnerShellBlocker::MissingBufferImporter,
            ),
            renderer_owner_shell_missing_texture_support: has_blocker(
                RuntimeSurfaceCommitRendererOwnerShellBlocker::MissingTextureSupport,
            ),
            renderer_owner_shell_buffer_imported: report.buffer_imported,
            renderer_owner_shell_texture_created: report.texture_created,
            renderer_owner_shell_renderer_called: report.renderer_called,
            renderer_owner_shell_damage_submitted: report.damage_submitted,
            renderer_owner_shell_frame_callback_done_sent: report.frame_callback_done_sent,
            renderer_owner_shell_input_support: report.input_support,
            renderer_owner_shell_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_importer_shell_readiness(
        report: &RuntimeSurfaceCommitBufferImporterShellReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_importer_shell_readiness_invocations: usize::from(report.readiness_invoked),
            buffer_importer_shell_work_intents_observed: usize::from(
                report.observed_work_intent.is_some(),
            ),
            buffer_importer_shell_observed_work_intents: report
                .observed_work_intent
                .clone()
                .into_iter()
                .collect(),
            buffer_importer_shell_available: report.buffer_importer_shell_available,
            buffer_importer_shell_missing_renderer_owner_shell: has_blocker(
                RuntimeSurfaceCommitBufferImporterShellBlocker::MissingRendererOwnerShell,
            ),
            buffer_importer_shell_missing_buffer_importer: has_blocker(
                RuntimeSurfaceCommitBufferImporterShellBlocker::MissingBufferImporter,
            ),
            buffer_importer_shell_missing_texture_support: has_blocker(
                RuntimeSurfaceCommitBufferImporterShellBlocker::MissingTextureSupport,
            ),
            buffer_importer_shell_buffer_imported: report.buffer_imported,
            buffer_importer_shell_texture_created: report.texture_created,
            buffer_importer_shell_renderer_called: report.renderer_called,
            buffer_importer_shell_damage_submitted: report.damage_submitted,
            buffer_importer_shell_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_importer_shell_input_support: report.input_support,
            buffer_importer_shell_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_texture_support_shell_readiness(
        report: &RuntimeSurfaceCommitTextureSupportShellReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            texture_support_shell_readiness_invocations: usize::from(report.readiness_invoked),
            texture_support_shell_work_intents_observed: usize::from(
                report.observed_work_intent.is_some(),
            ),
            texture_support_shell_observed_work_intents: report
                .observed_work_intent
                .clone()
                .into_iter()
                .collect(),
            texture_support_shell_available: report.texture_support_shell_available,
            texture_support_shell_missing_buffer_importer_shell: has_blocker(
                RuntimeSurfaceCommitTextureSupportShellBlocker::MissingBufferImporterShell,
            ),
            texture_support_shell_missing_texture_support: has_blocker(
                RuntimeSurfaceCommitTextureSupportShellBlocker::MissingTextureSupport,
            ),
            texture_support_shell_buffer_imported: report.buffer_imported,
            texture_support_shell_texture_created: report.texture_created,
            texture_support_shell_renderer_called: report.renderer_called,
            texture_support_shell_damage_submitted: report.damage_submitted,
            texture_support_shell_frame_callback_done_sent: report.frame_callback_done_sent,
            texture_support_shell_input_support: report.input_support,
            texture_support_shell_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_render_operation_readiness(
        report: &RuntimeSurfaceCommitRenderOperationReadinessReport,
    ) -> Self {
        Self {
            render_operation_readiness_invocations: usize::from(report.readiness_invoked),
            render_operation_intents_created: usize::from(report.render_operation_intent_created),
            render_operation_intents: report.render_operation_intent.clone().into_iter().collect(),
            render_operation_buffer_imported: report.buffer_imported,
            render_operation_texture_created: report.texture_created,
            render_operation_renderer_called: report.renderer_called,
            render_operation_damage_submitted: report.damage_submitted,
            render_operation_frame_callback_done_sent: report.frame_callback_done_sent,
            render_operation_input_support: report.input_support,
            render_operation_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_render_operation_intent_drain(
        report: &RuntimeSurfaceCommitRenderOperationIntentDrainReport,
    ) -> Self {
        Self {
            render_operation_queue_drain_invocations: usize::from(report.drain_invoked),
            render_operation_intents_enqueued: usize::from(report.intent_enqueued),
            render_operation_intents_drained: usize::from(report.intent_drained),
            render_operation_queue_drained_intents: report
                .drained_intent
                .clone()
                .into_iter()
                .collect(),
            render_operation_queue_buffer_imported: report.buffer_imported,
            render_operation_queue_texture_created: report.texture_created,
            render_operation_queue_renderer_called: report.renderer_called,
            render_operation_queue_damage_submitted: report.damage_submitted,
            render_operation_queue_frame_callback_done_sent: report.frame_callback_done_sent,
            render_operation_queue_input_support: report.input_support,
            render_operation_queue_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_render_execution_owner_boundary(
        report: &RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            render_execution_owner_boundary_invocations: usize::from(report.consume_invoked),
            render_execution_owner_intents_consumed: usize::from(
                report.render_operation_intent_consumed,
            ),
            render_execution_owner_consumed_intents: report
                .consumed_intent
                .clone()
                .into_iter()
                .collect(),
            render_execution_owner_missing_owner: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingRenderExecutionOwner,
            ),
            render_execution_owner_missing_buffer_import: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingBufferImport,
            ),
            render_execution_owner_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingTextureCreation,
            ),
            render_execution_owner_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingRendererCall,
            ),
            render_execution_owner_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingDamageSubmit,
            ),
            render_execution_owner_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker::MissingFrameCallbackDone,
            ),
            render_execution_owner_buffer_imported: report.buffer_imported,
            render_execution_owner_texture_created: report.texture_created,
            render_execution_owner_renderer_called: report.renderer_called,
            render_execution_owner_damage_submitted: report.damage_submitted,
            render_execution_owner_frame_callback_done_sent: report.frame_callback_done_sent,
            render_execution_owner_input_support: report.input_support,
            render_execution_owner_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_render_execution_owner_shell_readiness(
        report: &RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            render_execution_owner_shell_readiness_invocations: usize::from(
                report.readiness_invoked,
            ),
            render_execution_owner_shell_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            render_execution_owner_shell_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            render_execution_owner_shell_available: report.render_execution_owner_shell_available,
            render_execution_owner_shell_missing_buffer_import: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingBufferImport,
            ),
            render_execution_owner_shell_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingTextureCreation,
            ),
            render_execution_owner_shell_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingRendererCall,
            ),
            render_execution_owner_shell_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingDamageSubmit,
            ),
            render_execution_owner_shell_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker::MissingFrameCallbackDone,
            ),
            render_execution_owner_shell_buffer_imported: report.buffer_imported,
            render_execution_owner_shell_texture_created: report.texture_created,
            render_execution_owner_shell_renderer_called: report.renderer_called,
            render_execution_owner_shell_damage_submitted: report.damage_submitted,
            render_execution_owner_shell_frame_callback_done_sent: report.frame_callback_done_sent,
            render_execution_owner_shell_input_support: report.input_support,
            render_execution_owner_shell_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_render_pipeline_skeleton_readiness(
        report: &RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            render_pipeline_skeleton_readiness_invocations: usize::from(report.readiness_invoked),
            render_pipeline_skeleton_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            render_pipeline_skeleton_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            render_pipeline_skeleton_owner_available: report.renderer_pipeline_owner_available,
            render_pipeline_skeleton_missing_execution_owner_shell: has_blocker(
                RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingRenderExecutionOwnerShell,
            ),
            render_pipeline_skeleton_missing_buffer_import: has_blocker(
                RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingBufferImport,
            ),
            render_pipeline_skeleton_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingTextureCreation,
            ),
            render_pipeline_skeleton_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingRendererCall,
            ),
            render_pipeline_skeleton_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingDamageSubmit,
            ),
            render_pipeline_skeleton_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitRenderPipelineSkeletonBlocker::MissingFrameCallbackDone,
            ),
            render_pipeline_skeleton_buffer_imported: report.buffer_imported,
            render_pipeline_skeleton_texture_created: report.texture_created,
            render_pipeline_skeleton_renderer_called: report.renderer_called,
            render_pipeline_skeleton_damage_submitted: report.damage_submitted,
            render_pipeline_skeleton_frame_callback_done_sent: report.frame_callback_done_sent,
            render_pipeline_skeleton_input_support: report.input_support,
            render_pipeline_skeleton_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_render_backend_capability_report(
        report: &RuntimeSurfaceCommitRenderBackendCapabilityReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            render_backend_capability_report_invocations: usize::from(report.report_invoked),
            render_backend_capability_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            render_backend_capability_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            render_backend_capability_owner_available: report
                .render_backend_capability_owner_available,
            render_backend_capability_backend_registered: report.renderer_backend_registered,
            render_backend_capability_backend_kind: report.renderer_backend_kind,
            render_backend_capability_missing_pipeline_skeleton: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRenderPipelineSkeleton,
            ),
            render_backend_capability_missing_backend_registration: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRendererBackendRegistration,
            ),
            render_backend_capability_missing_buffer_import: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingBufferImport,
            ),
            render_backend_capability_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingTextureCreation,
            ),
            render_backend_capability_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingRendererCall,
            ),
            render_backend_capability_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingDamageSubmit,
            ),
            render_backend_capability_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitRenderBackendCapabilityBlocker::MissingFrameCallbackDone,
            ),
            render_backend_capability_buffer_imported: report.buffer_imported,
            render_backend_capability_texture_created: report.texture_created,
            render_backend_capability_renderer_called: report.renderer_called,
            render_backend_capability_damage_submitted: report.damage_submitted,
            render_backend_capability_frame_callback_done_sent: report.frame_callback_done_sent,
            render_backend_capability_input_support: report.input_support,
            render_backend_capability_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_renderer_backend_registration_report(
        report: &RuntimeSurfaceCommitRendererBackendRegistrationReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            renderer_backend_registration_invocations: usize::from(report.registration_invoked),
            renderer_backend_registration_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            renderer_backend_registration_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            renderer_backend_registration_owner_available: report
                .renderer_backend_registration_owner_available,
            renderer_backend_registration_backend_registered: report.renderer_backend_registered,
            renderer_backend_registration_descriptor_available: report
                .renderer_backend_descriptor_available,
            renderer_backend_registration_registered_backend_kind: report
                .registered_renderer_backend_kind,
            renderer_backend_registration_missing_backend_capability: has_blocker(
                RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingRenderBackendCapabilityOwner,
            ),
            renderer_backend_registration_missing_buffer_import: has_blocker(
                RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingBufferImport,
            ),
            renderer_backend_registration_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingTextureCreation,
            ),
            renderer_backend_registration_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingRendererCall,
            ),
            renderer_backend_registration_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingDamageSubmit,
            ),
            renderer_backend_registration_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitRendererBackendRegistrationBlocker::MissingFrameCallbackDone,
            ),
            renderer_backend_registration_buffer_imported: report.buffer_imported,
            renderer_backend_registration_texture_created: report.texture_created,
            renderer_backend_registration_renderer_called: report.renderer_called,
            renderer_backend_registration_damage_submitted: report.damage_submitted,
            renderer_backend_registration_frame_callback_done_sent: report
                .frame_callback_done_sent,
            renderer_backend_registration_input_support: report.input_support,
            renderer_backend_registration_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_renderer_backend_owner_shell_readiness(
        report: &RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            renderer_backend_owner_shell_readiness_invocations: usize::from(
                report.readiness_invoked,
            ),
            renderer_backend_owner_shell_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            renderer_backend_owner_shell_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            renderer_backend_owner_shell_available: report
                .renderer_backend_owner_shell_available,
            renderer_backend_owner_shell_bound: report.renderer_backend_owner_shell_bound,
            renderer_backend_owner_shell_descriptor_available: report
                .source_renderer_backend_descriptor_available,
            renderer_backend_owner_shell_registered_backend_kind: report
                .registered_renderer_backend_kind,
            renderer_backend_owner_shell_missing_descriptor: has_blocker(
                RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingRendererBackendDescriptor,
            ),
            renderer_backend_owner_shell_missing_buffer_import: has_blocker(
                RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingBufferImport,
            ),
            renderer_backend_owner_shell_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingTextureCreation,
            ),
            renderer_backend_owner_shell_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingRendererCall,
            ),
            renderer_backend_owner_shell_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingDamageSubmit,
            ),
            renderer_backend_owner_shell_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitRendererBackendOwnerShellBlocker::MissingFrameCallbackDone,
            ),
            renderer_backend_owner_shell_buffer_imported: report.buffer_imported,
            renderer_backend_owner_shell_texture_created: report.texture_created,
            renderer_backend_owner_shell_renderer_called: report.renderer_called,
            renderer_backend_owner_shell_damage_submitted: report.damage_submitted,
            renderer_backend_owner_shell_frame_callback_done_sent: report.frame_callback_done_sent,
            renderer_backend_owner_shell_input_support: report.input_support,
            renderer_backend_owner_shell_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_resource_owner_readiness(
        report: &RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_resource_owner_readiness_invocations: usize::from(
                report.readiness_invoked,
            ),
            buffer_import_resource_owner_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            buffer_import_resource_owner_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            buffer_importer_owner_available: report.buffer_importer_owner_available,
            buffer_importer_owner_bound: report.buffer_importer_owner_bound,
            buffer_import_resource_owner_descriptor_evidence_available: report
                .renderer_backend_descriptor_evidence_available,
            buffer_import_resource_owner_registered_backend_kind: report
                .registered_renderer_backend_kind,
            buffer_import_resource_owner_missing_renderer_backend_owner_shell: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRendererBackendOwnerShell,
            ),
            buffer_import_resource_owner_missing_descriptor_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRendererBackendDescriptorEvidence,
            ),
            buffer_import_resource_owner_missing_actual_buffer_import: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingActualBufferImport,
            ),
            buffer_import_resource_owner_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingTextureCreation,
            ),
            buffer_import_resource_owner_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingRendererCall,
            ),
            buffer_import_resource_owner_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingDamageSubmit,
            ),
            buffer_import_resource_owner_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportResourceOwnerBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_resource_owner_buffer_imported: report.buffer_imported,
            buffer_import_resource_owner_texture_created: report.texture_created,
            buffer_import_resource_owner_renderer_called: report.renderer_called,
            buffer_import_resource_owner_damage_submitted: report.damage_submitted,
            buffer_import_resource_owner_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_resource_owner_input_support: report.input_support,
            buffer_import_resource_owner_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_planning_report(
        report: &RuntimeSurfaceCommitBufferImportPlanningReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_planning_invocations: usize::from(report.planning_invoked),
            buffer_import_planning_intents_observed: usize::from(
                report.observed_intent.is_some(),
            ),
            buffer_import_planning_observed_intents: report
                .observed_intent
                .clone()
                .into_iter()
                .collect(),
            buffer_import_plan_available: report.buffer_import_plan_available,
            buffer_import_plan_built: report.buffer_import_plan_built,
            buffer_import_candidates_observed: usize::from(
                report.buffer_import_candidate_observed,
            ),
            buffer_import_required_count: usize::from(report.buffer_import_required),
            buffer_import_planning_descriptor_evidence_available: report
                .renderer_backend_descriptor_evidence_available,
            buffer_import_planning_registered_backend_kind: report
                .registered_renderer_backend_kind,
            buffer_import_planning_missing_buffer_importer_owner: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingBufferImporterOwner,
            ),
            buffer_import_planning_missing_descriptor_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingRendererBackendDescriptorEvidence,
            ),
            buffer_import_planning_missing_candidate: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingBufferImportCandidate,
            ),
            buffer_import_planning_missing_actual_buffer_import: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingActualBufferImport,
            ),
            buffer_import_planning_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingTextureCreation,
            ),
            buffer_import_planning_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingRendererCall,
            ),
            buffer_import_planning_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingDamageSubmit,
            ),
            buffer_import_planning_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportPlanningBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_planning_buffer_imported: report.buffer_imported,
            buffer_import_planning_texture_created: report.texture_created,
            buffer_import_planning_renderer_called: report.renderer_called,
            buffer_import_planning_damage_submitted: report.damage_submitted,
            buffer_import_planning_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_planning_input_support: report.input_support,
            buffer_import_planning_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_implementation_boundary_report(
        report: &RuntimeSurfaceCommitBufferImportImplementationBoundaryReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_implementation_boundary_invocations: usize::from(report.boundary_invoked),
            buffer_import_implementation_descriptors_observed: usize::from(
                report.descriptor.is_some(),
            ),
            buffer_import_implementation_observed_descriptors: report
                .descriptor
                .clone()
                .into_iter()
                .collect(),
            buffer_import_implementation_descriptor_available: report
                .implementation_descriptor_available,
            buffer_import_implementation_descriptor_registered: report
                .implementation_descriptor_registered,
            buffer_import_implementation_candidates_observed: usize::from(
                report.candidate_evidence_observed,
            ),
            buffer_import_implementation_actual_required_count: usize::from(
                report.actual_import_required,
            ),
            buffer_import_implementation_importer_owner_evidence_available: report
                .importer_owner_evidence_available,
            buffer_import_implementation_renderer_descriptor_evidence_available: report
                .renderer_backend_descriptor_evidence_available,
            buffer_import_implementation_registered_backend_kind: report
                .registered_renderer_backend_kind,
            buffer_import_implementation_missing_planning_intent: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingBufferImportPlanningIntent,
            ),
            buffer_import_implementation_missing_plan: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingBufferImportPlan,
            ),
            buffer_import_implementation_missing_importer_owner_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingImporterOwnerEvidence,
            ),
            buffer_import_implementation_missing_renderer_descriptor_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingRendererBackendDescriptorEvidence,
            ),
            buffer_import_implementation_missing_candidate: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingBufferImportCandidate,
            ),
            buffer_import_implementation_missing_actual_buffer_import: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingActualBufferImport,
            ),
            buffer_import_implementation_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingTextureCreation,
            ),
            buffer_import_implementation_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingRendererCall,
            ),
            buffer_import_implementation_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingDamageSubmit,
            ),
            buffer_import_implementation_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_implementation_buffer_import_attempted: report.buffer_import_attempted,
            buffer_import_implementation_buffer_imported: report.buffer_imported,
            buffer_import_implementation_texture_created: report.texture_created,
            buffer_import_implementation_renderer_called: report.renderer_called,
            buffer_import_implementation_damage_submitted: report.damage_submitted,
            buffer_import_implementation_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_implementation_input_support: report.input_support,
            buffer_import_implementation_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_adapter_proof_boundary_report(
        report: &RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_adapter_proof_boundary_invocations: usize::from(report.boundary_invoked),
            buffer_import_adapter_proofs_observed: usize::from(report.adapter_proof.is_some()),
            buffer_import_adapter_observed_proofs: report
                .adapter_proof
                .clone()
                .into_iter()
                .collect(),
            buffer_import_adapter_proof_boundary_available: report
                .adapter_proof_boundary_available,
            buffer_import_adapter_proof_registered: report.adapter_proof_registered,
            buffer_import_adapter_candidates_observed: usize::from(
                report.candidate_evidence_observed,
            ),
            buffer_import_adapter_actual_required_count: usize::from(
                report.actual_import_required,
            ),
            buffer_import_adapter_importer_owner_evidence_available: report
                .importer_owner_evidence_available,
            buffer_import_adapter_renderer_descriptor_evidence_available: report
                .renderer_backend_descriptor_evidence_available,
            buffer_import_adapter_registered_backend_kind: report.registered_renderer_backend_kind,
            buffer_import_adapter_missing_implementation_descriptor: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingImplementationDescriptor,
            ),
            buffer_import_adapter_missing_importer_owner_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingImporterOwnerEvidence,
            ),
            buffer_import_adapter_missing_renderer_descriptor_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingRendererBackendDescriptorEvidence,
            ),
            buffer_import_adapter_missing_candidate: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingBufferImportCandidate,
            ),
            buffer_import_adapter_missing_actual_buffer_import: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingActualBufferImport,
            ),
            buffer_import_adapter_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingTextureCreation,
            ),
            buffer_import_adapter_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingRendererCall,
            ),
            buffer_import_adapter_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingDamageSubmit,
            ),
            buffer_import_adapter_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportAdapterProofBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_adapter_buffer_import_attempted: report.buffer_import_attempted,
            buffer_import_adapter_buffer_imported: report.buffer_imported,
            buffer_import_adapter_texture_created: report.texture_created,
            buffer_import_adapter_renderer_called: report.renderer_called,
            buffer_import_adapter_damage_submitted: report.damage_submitted,
            buffer_import_adapter_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_adapter_input_support: report.input_support,
            buffer_import_adapter_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_precondition_gate_report(
        report: &RuntimeSurfaceCommitBufferImportPreconditionGateReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_precondition_gate_invocations: usize::from(report.gate_invoked),
            buffer_import_precondition_adapter_proofs_observed: usize::from(
                report.observed_adapter_proof.is_some(),
            ),
            buffer_import_precondition_observed_adapter_proofs: report
                .observed_adapter_proof
                .clone()
                .into_iter()
                .collect(),
            buffer_import_precondition_gate_available: report.import_precondition_gate_available,
            buffer_import_preconditions_met_count: usize::from(report.import_preconditions_met),
            buffer_import_future_preconditions_met_count: usize::from(
                report.future_import_preconditions_met,
            ),
            buffer_import_precondition_candidates_observed: usize::from(
                report.candidate_evidence_observed,
            ),
            buffer_import_precondition_actual_required_count: usize::from(
                report.actual_import_required,
            ),
            buffer_import_precondition_importer_owner_evidence_available: report
                .importer_owner_evidence_available,
            buffer_import_precondition_renderer_descriptor_evidence_available: report
                .renderer_backend_descriptor_evidence_available,
            buffer_import_precondition_registered_backend_kind: report
                .registered_renderer_backend_kind,
            buffer_import_precondition_missing_adapter_proof: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingAdapterProof,
            ),
            buffer_import_precondition_missing_registered_adapter_proof: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingRegisteredAdapterProof,
            ),
            buffer_import_precondition_missing_importer_owner_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingImporterOwnerEvidence,
            ),
            buffer_import_precondition_missing_renderer_descriptor_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingRendererBackendDescriptorEvidence,
            ),
            buffer_import_precondition_missing_candidate: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingBufferImportCandidate,
            ),
            buffer_import_precondition_missing_actual_import_requirement: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingActualImportRequirement,
            ),
            buffer_import_precondition_missing_actual_buffer_import: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingActualBufferImport,
            ),
            buffer_import_precondition_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingTextureCreation,
            ),
            buffer_import_precondition_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingRendererCall,
            ),
            buffer_import_precondition_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingDamageSubmit,
            ),
            buffer_import_precondition_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportPreconditionGateBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_precondition_buffer_import_attempted: report.buffer_import_attempted,
            buffer_import_precondition_buffer_imported: report.buffer_imported,
            buffer_import_precondition_texture_created: report.texture_created,
            buffer_import_precondition_renderer_called: report.renderer_called,
            buffer_import_precondition_damage_submitted: report.damage_submitted,
            buffer_import_precondition_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_precondition_input_support: report.input_support,
            buffer_import_precondition_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_execution_dry_run_report(
        report: &RuntimeSurfaceCommitBufferImportExecutionDryRunReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_execution_dry_run_invocations: usize::from(report.dry_run_invoked),
            buffer_import_execution_dry_run_reports: vec![report.clone()],
            buffer_import_execution_guard_available: report.execution_guard_available,
            buffer_import_execution_attempted_count: usize::from(report.execution_attempted),
            buffer_import_execution_noop_count: usize::from(report.execution_noop),
            buffer_import_execution_blocked_count: usize::from(report.execution_blocked),
            buffer_import_execution_actual_required_count: usize::from(
                report.actual_import_required,
            ),
            buffer_import_execution_preconditions_met_count: usize::from(
                report.import_preconditions_met,
            ),
            buffer_import_execution_missing_precondition_gate_evidence: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingPreconditionGateEvidence,
            ),
            buffer_import_execution_missing_adapter_proof: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingAdapterProof,
            ),
            buffer_import_execution_missing_import_preconditions: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingImportPreconditions,
            ),
            buffer_import_execution_missing_real_importer: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingRealBufferImportImplementation,
            ),
            buffer_import_execution_no_actual_import_required: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::NoActualImportRequired,
            ),
            buffer_import_execution_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingTextureCreation,
            ),
            buffer_import_execution_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingRendererCall,
            ),
            buffer_import_execution_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingDamageSubmit,
            ),
            buffer_import_execution_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportExecutionBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_execution_buffer_import_attempted: report.buffer_import_attempted,
            buffer_import_execution_buffer_imported: report.buffer_imported,
            buffer_import_execution_texture_created: report.texture_created,
            buffer_import_execution_renderer_called: report.renderer_called,
            buffer_import_execution_damage_submitted: report.damage_submitted,
            buffer_import_execution_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_execution_input_support: report.input_support,
            buffer_import_execution_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_implementation_owner_shell_report(
        report: &RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_implementation_owner_shell_invocations: usize::from(
                report.owner_shell_invoked,
            ),
            buffer_import_implementation_owner_shell_reports: vec![report.clone()],
            buffer_import_implementation_owner_shell_available: report
                .implementation_owner_shell_available,
            buffer_import_real_implementation_available: report
                .real_importer_implementation_available,
            buffer_import_actual_attempt_admitted_count: usize::from(
                report.actual_import_attempt_admitted,
            ),
            buffer_import_actual_attempt_blocked_count: usize::from(
                report.actual_import_attempt_blocked,
            ),
            buffer_import_implementation_owner_actual_required_count: usize::from(
                report.actual_import_required,
            ),
            buffer_import_implementation_owner_execution_reports_observed: usize::from(
                report.source_buffer_import_execution_dry_run_report_observed,
            ),
            buffer_import_implementation_owner_missing_execution_dry_run_report: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingExecutionDryRunReport,
            ),
            buffer_import_implementation_owner_execution_dry_run_blocked: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::ExecutionDryRunBlocked,
            ),
            buffer_import_implementation_owner_no_actual_import_required: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::NoActualImportRequired,
            ),
            buffer_import_implementation_owner_missing_real_importer: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingRealBufferImportImplementation,
            ),
            buffer_import_implementation_owner_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingTextureCreation,
            ),
            buffer_import_implementation_owner_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingRendererCall,
            ),
            buffer_import_implementation_owner_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingDamageSubmit,
            ),
            buffer_import_implementation_owner_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_implementation_owner_buffer_import_attempted: report
                .buffer_import_attempted,
            buffer_import_implementation_owner_buffer_imported: report.buffer_imported,
            buffer_import_implementation_owner_texture_created: report.texture_created,
            buffer_import_implementation_owner_renderer_called: report.renderer_called,
            buffer_import_implementation_owner_damage_submitted: report.damage_submitted,
            buffer_import_implementation_owner_frame_callback_done_sent: report
                .frame_callback_done_sent,
            buffer_import_implementation_owner_input_support: report.input_support,
            buffer_import_implementation_owner_core_mutation_invoked: report
                .core_mutation_invoked,
            ..Self::default()
        }
    }

    fn from_buffer_import_actual_attempt_record(
        report: &RuntimeSurfaceCommitBufferImportActualAttemptRecord,
    ) -> Self {
        let has_blocker = |blocker| report.blockers.contains(&blocker);
        Self {
            buffer_import_actual_attempt_record_invocations: usize::from(
                report.actual_attempt_recorder_invoked,
            ),
            buffer_import_actual_attempt_records: vec![report.clone()],
            buffer_import_actual_attempt_record_available: report.actual_attempt_record_available,
            buffer_import_actual_attempt_recorded_count: usize::from(
                report.actual_attempt_recorded,
            ),
            buffer_import_actual_attempt_admission_checked_count: usize::from(
                report.actual_attempt_admission_checked,
            ),
            buffer_import_actual_attempt_record_admitted_count: usize::from(
                report.actual_attempt_admitted,
            ),
            buffer_import_actual_attempt_record_blocked_count: usize::from(
                report.actual_attempt_blocked,
            ),
            buffer_import_actual_attempt_required_count: usize::from(
                report.actual_import_required,
            ),
            buffer_import_actual_attempt_missing_owner_shell_report: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingImplementationOwnerShellReport,
            ),
            buffer_import_actual_attempt_owner_shell_blocked: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::ImplementationOwnerShellBlocked,
            ),
            buffer_import_actual_attempt_no_actual_import_required: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::NoActualImportRequired,
            ),
            buffer_import_actual_attempt_missing_admission: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingAttemptAdmission,
            ),
            buffer_import_actual_attempt_missing_real_importer: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingRealBufferImportImplementation,
            ),
            buffer_import_actual_attempt_missing_texture_creation: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingTextureCreation,
            ),
            buffer_import_actual_attempt_missing_renderer_call: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingRendererCall,
            ),
            buffer_import_actual_attempt_missing_damage_submit: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingDamageSubmit,
            ),
            buffer_import_actual_attempt_missing_frame_callback_done: has_blocker(
                RuntimeSurfaceCommitBufferImportActualAttemptBlocker::MissingFrameCallbackDone,
            ),
            buffer_import_actual_attempt_buffer_import_attempted: report.buffer_import_attempted,
            buffer_import_actual_attempt_buffer_imported: report.buffer_imported,
            buffer_import_actual_attempt_texture_created: report.texture_created,
            buffer_import_actual_attempt_renderer_called: report.renderer_called,
            buffer_import_actual_attempt_damage_submitted: report.damage_submitted,
            buffer_import_actual_attempt_frame_callback_done_sent: report.frame_callback_done_sent,
            buffer_import_actual_attempt_input_support: report.input_support,
            buffer_import_actual_attempt_core_mutation_invoked: report.core_mutation_invoked,
            ..Self::default()
        }
    }

    fn has_progress(&self) -> bool {
        self.commit_observations_drained > 0
            || self.commit_observation_errors > 0
            || self.render_dirty_intents_enqueued > 0
            || self.render_dirty_intents_drained > 0
            || self.renderer_work_intents_created > 0
            || self.renderer_owner_work_intents_consumed > 0
            || self.renderer_owner_shell_work_intents_observed > 0
            || self.buffer_importer_shell_work_intents_observed > 0
            || self.texture_support_shell_work_intents_observed > 0
            || self.render_operation_intents_created > 0
            || self.render_operation_intents_drained > 0
            || self.render_execution_owner_intents_consumed > 0
            || self.render_execution_owner_shell_intents_observed > 0
            || self.render_pipeline_skeleton_intents_observed > 0
            || self.render_backend_capability_intents_observed > 0
            || self.renderer_backend_registration_intents_observed > 0
            || self.renderer_backend_owner_shell_intents_observed > 0
            || self.buffer_import_resource_owner_intents_observed > 0
            || self.buffer_import_planning_intents_observed > 0
            || self.buffer_import_implementation_descriptors_observed > 0
            || self.buffer_import_adapter_proofs_observed > 0
            || self.buffer_import_precondition_adapter_proofs_observed > 0
            || self.buffer_import_preconditions_met_count > 0
    }

    fn observe(&mut self, delta: Self) {
        self.drain_invocations = self
            .drain_invocations
            .saturating_add(delta.drain_invocations);
        self.commit_observations_drained = self
            .commit_observations_drained
            .saturating_add(delta.commit_observations_drained);
        self.commit_observation_errors = self
            .commit_observation_errors
            .saturating_add(delta.commit_observation_errors);
        self.drained_commit_sequences
            .extend(delta.drained_commit_sequences);
        self.buffer_attach_observations = self
            .buffer_attach_observations
            .saturating_add(delta.buffer_attach_observations);
        self.buffer_presence_observations = self
            .buffer_presence_observations
            .saturating_add(delta.buffer_presence_observations);
        self.buffer_removed_observations = self
            .buffer_removed_observations
            .saturating_add(delta.buffer_removed_observations);
        self.renderable_buffer_observations = self
            .renderable_buffer_observations
            .saturating_add(delta.renderable_buffer_observations);
        self.damage_observations = self
            .damage_observations
            .saturating_add(delta.damage_observations);
        self.surface_damage_rects = self
            .surface_damage_rects
            .saturating_add(delta.surface_damage_rects);
        self.buffer_damage_rects = self
            .buffer_damage_rects
            .saturating_add(delta.buffer_damage_rects);
        self.frame_callback_observations = self
            .frame_callback_observations
            .saturating_add(delta.frame_callback_observations);
        self.frame_callback_count = self
            .frame_callback_count
            .saturating_add(delta.frame_callback_count);
        self.render_dirty_readiness_intents
            .extend(delta.render_dirty_readiness_intents);
        self.render_dirty_queue_drain_invocations = self
            .render_dirty_queue_drain_invocations
            .saturating_add(delta.render_dirty_queue_drain_invocations);
        self.render_dirty_intents_enqueued = self
            .render_dirty_intents_enqueued
            .saturating_add(delta.render_dirty_intents_enqueued);
        self.render_dirty_intents_drained = self
            .render_dirty_intents_drained
            .saturating_add(delta.render_dirty_intents_drained);
        self.render_dirty_queue_drained_intents
            .extend(delta.render_dirty_queue_drained_intents);
        self.render_dirty_queue_buffer_imported |= delta.render_dirty_queue_buffer_imported;
        self.render_dirty_queue_texture_created |= delta.render_dirty_queue_texture_created;
        self.render_dirty_queue_render_submitted |= delta.render_dirty_queue_render_submitted;
        self.render_dirty_queue_frame_callback_done_sent |=
            delta.render_dirty_queue_frame_callback_done_sent;
        self.render_dirty_queue_input_support |= delta.render_dirty_queue_input_support;
        self.renderer_admission_invocations = self
            .renderer_admission_invocations
            .saturating_add(delta.renderer_admission_invocations);
        self.renderer_work_intents_created = self
            .renderer_work_intents_created
            .saturating_add(delta.renderer_work_intents_created);
        self.renderer_work_intents
            .extend(delta.renderer_work_intents);
        self.renderer_admission_buffer_imported |= delta.renderer_admission_buffer_imported;
        self.renderer_admission_texture_created |= delta.renderer_admission_texture_created;
        self.renderer_admission_render_submitted |= delta.renderer_admission_render_submitted;
        self.renderer_admission_damage_submitted |= delta.renderer_admission_damage_submitted;
        self.renderer_admission_frame_callback_done_sent |=
            delta.renderer_admission_frame_callback_done_sent;
        self.renderer_admission_input_support |= delta.renderer_admission_input_support;
        self.renderer_admission_core_mutation_invoked |=
            delta.renderer_admission_core_mutation_invoked;
        self.renderer_owner_boundary_invocations = self
            .renderer_owner_boundary_invocations
            .saturating_add(delta.renderer_owner_boundary_invocations);
        self.renderer_owner_work_intents_consumed = self
            .renderer_owner_work_intents_consumed
            .saturating_add(delta.renderer_owner_work_intents_consumed);
        self.renderer_owner_consumed_work_intents
            .extend(delta.renderer_owner_consumed_work_intents);
        self.renderer_owner_missing_renderer_owner |= delta.renderer_owner_missing_renderer_owner;
        self.renderer_owner_missing_buffer_importer |= delta.renderer_owner_missing_buffer_importer;
        self.renderer_owner_missing_texture_support |= delta.renderer_owner_missing_texture_support;
        self.renderer_owner_buffer_imported |= delta.renderer_owner_buffer_imported;
        self.renderer_owner_texture_created |= delta.renderer_owner_texture_created;
        self.renderer_owner_renderer_called |= delta.renderer_owner_renderer_called;
        self.renderer_owner_damage_submitted |= delta.renderer_owner_damage_submitted;
        self.renderer_owner_frame_callback_done_sent |=
            delta.renderer_owner_frame_callback_done_sent;
        self.renderer_owner_input_support |= delta.renderer_owner_input_support;
        self.renderer_owner_core_mutation_invoked |= delta.renderer_owner_core_mutation_invoked;
        self.renderer_owner_shell_readiness_invocations = self
            .renderer_owner_shell_readiness_invocations
            .saturating_add(delta.renderer_owner_shell_readiness_invocations);
        self.renderer_owner_shell_work_intents_observed = self
            .renderer_owner_shell_work_intents_observed
            .saturating_add(delta.renderer_owner_shell_work_intents_observed);
        self.renderer_owner_shell_observed_work_intents
            .extend(delta.renderer_owner_shell_observed_work_intents);
        self.renderer_owner_shell_available |= delta.renderer_owner_shell_available;
        self.renderer_owner_shell_missing_renderer_owner |=
            delta.renderer_owner_shell_missing_renderer_owner;
        self.renderer_owner_shell_missing_buffer_importer |=
            delta.renderer_owner_shell_missing_buffer_importer;
        self.renderer_owner_shell_missing_texture_support |=
            delta.renderer_owner_shell_missing_texture_support;
        self.renderer_owner_shell_buffer_imported |= delta.renderer_owner_shell_buffer_imported;
        self.renderer_owner_shell_texture_created |= delta.renderer_owner_shell_texture_created;
        self.renderer_owner_shell_renderer_called |= delta.renderer_owner_shell_renderer_called;
        self.renderer_owner_shell_damage_submitted |= delta.renderer_owner_shell_damage_submitted;
        self.renderer_owner_shell_frame_callback_done_sent |=
            delta.renderer_owner_shell_frame_callback_done_sent;
        self.renderer_owner_shell_input_support |= delta.renderer_owner_shell_input_support;
        self.renderer_owner_shell_core_mutation_invoked |=
            delta.renderer_owner_shell_core_mutation_invoked;
        self.buffer_importer_shell_readiness_invocations = self
            .buffer_importer_shell_readiness_invocations
            .saturating_add(delta.buffer_importer_shell_readiness_invocations);
        self.buffer_importer_shell_work_intents_observed = self
            .buffer_importer_shell_work_intents_observed
            .saturating_add(delta.buffer_importer_shell_work_intents_observed);
        self.buffer_importer_shell_observed_work_intents
            .extend(delta.buffer_importer_shell_observed_work_intents);
        self.buffer_importer_shell_available |= delta.buffer_importer_shell_available;
        self.buffer_importer_shell_missing_renderer_owner_shell |=
            delta.buffer_importer_shell_missing_renderer_owner_shell;
        self.buffer_importer_shell_missing_buffer_importer |=
            delta.buffer_importer_shell_missing_buffer_importer;
        self.buffer_importer_shell_missing_texture_support |=
            delta.buffer_importer_shell_missing_texture_support;
        self.buffer_importer_shell_buffer_imported |= delta.buffer_importer_shell_buffer_imported;
        self.buffer_importer_shell_texture_created |= delta.buffer_importer_shell_texture_created;
        self.buffer_importer_shell_renderer_called |= delta.buffer_importer_shell_renderer_called;
        self.buffer_importer_shell_damage_submitted |= delta.buffer_importer_shell_damage_submitted;
        self.buffer_importer_shell_frame_callback_done_sent |=
            delta.buffer_importer_shell_frame_callback_done_sent;
        self.buffer_importer_shell_input_support |= delta.buffer_importer_shell_input_support;
        self.buffer_importer_shell_core_mutation_invoked |=
            delta.buffer_importer_shell_core_mutation_invoked;
        self.texture_support_shell_readiness_invocations = self
            .texture_support_shell_readiness_invocations
            .saturating_add(delta.texture_support_shell_readiness_invocations);
        self.texture_support_shell_work_intents_observed = self
            .texture_support_shell_work_intents_observed
            .saturating_add(delta.texture_support_shell_work_intents_observed);
        self.texture_support_shell_observed_work_intents
            .extend(delta.texture_support_shell_observed_work_intents);
        self.texture_support_shell_available |= delta.texture_support_shell_available;
        self.texture_support_shell_missing_buffer_importer_shell |=
            delta.texture_support_shell_missing_buffer_importer_shell;
        self.texture_support_shell_missing_texture_support |=
            delta.texture_support_shell_missing_texture_support;
        self.texture_support_shell_buffer_imported |= delta.texture_support_shell_buffer_imported;
        self.texture_support_shell_texture_created |= delta.texture_support_shell_texture_created;
        self.texture_support_shell_renderer_called |= delta.texture_support_shell_renderer_called;
        self.texture_support_shell_damage_submitted |= delta.texture_support_shell_damage_submitted;
        self.texture_support_shell_frame_callback_done_sent |=
            delta.texture_support_shell_frame_callback_done_sent;
        self.texture_support_shell_input_support |= delta.texture_support_shell_input_support;
        self.texture_support_shell_core_mutation_invoked |=
            delta.texture_support_shell_core_mutation_invoked;
        self.render_operation_readiness_invocations = self
            .render_operation_readiness_invocations
            .saturating_add(delta.render_operation_readiness_invocations);
        self.render_operation_intents_created = self
            .render_operation_intents_created
            .saturating_add(delta.render_operation_intents_created);
        self.render_operation_intents
            .extend(delta.render_operation_intents);
        self.render_operation_buffer_imported |= delta.render_operation_buffer_imported;
        self.render_operation_texture_created |= delta.render_operation_texture_created;
        self.render_operation_renderer_called |= delta.render_operation_renderer_called;
        self.render_operation_damage_submitted |= delta.render_operation_damage_submitted;
        self.render_operation_frame_callback_done_sent |=
            delta.render_operation_frame_callback_done_sent;
        self.render_operation_input_support |= delta.render_operation_input_support;
        self.render_operation_core_mutation_invoked |= delta.render_operation_core_mutation_invoked;
        self.render_operation_queue_drain_invocations = self
            .render_operation_queue_drain_invocations
            .saturating_add(delta.render_operation_queue_drain_invocations);
        self.render_operation_intents_enqueued = self
            .render_operation_intents_enqueued
            .saturating_add(delta.render_operation_intents_enqueued);
        self.render_operation_intents_drained = self
            .render_operation_intents_drained
            .saturating_add(delta.render_operation_intents_drained);
        self.render_operation_queue_drained_intents
            .extend(delta.render_operation_queue_drained_intents);
        self.render_operation_queue_buffer_imported |= delta.render_operation_queue_buffer_imported;
        self.render_operation_queue_texture_created |= delta.render_operation_queue_texture_created;
        self.render_operation_queue_renderer_called |= delta.render_operation_queue_renderer_called;
        self.render_operation_queue_damage_submitted |=
            delta.render_operation_queue_damage_submitted;
        self.render_operation_queue_frame_callback_done_sent |=
            delta.render_operation_queue_frame_callback_done_sent;
        self.render_operation_queue_input_support |= delta.render_operation_queue_input_support;
        self.render_operation_queue_core_mutation_invoked |=
            delta.render_operation_queue_core_mutation_invoked;
        self.render_execution_owner_boundary_invocations = self
            .render_execution_owner_boundary_invocations
            .saturating_add(delta.render_execution_owner_boundary_invocations);
        self.render_execution_owner_intents_consumed = self
            .render_execution_owner_intents_consumed
            .saturating_add(delta.render_execution_owner_intents_consumed);
        self.render_execution_owner_consumed_intents
            .extend(delta.render_execution_owner_consumed_intents);
        self.render_execution_owner_missing_owner |= delta.render_execution_owner_missing_owner;
        self.render_execution_owner_missing_buffer_import |=
            delta.render_execution_owner_missing_buffer_import;
        self.render_execution_owner_missing_texture_creation |=
            delta.render_execution_owner_missing_texture_creation;
        self.render_execution_owner_missing_renderer_call |=
            delta.render_execution_owner_missing_renderer_call;
        self.render_execution_owner_missing_damage_submit |=
            delta.render_execution_owner_missing_damage_submit;
        self.render_execution_owner_missing_frame_callback_done |=
            delta.render_execution_owner_missing_frame_callback_done;
        self.render_execution_owner_buffer_imported |= delta.render_execution_owner_buffer_imported;
        self.render_execution_owner_texture_created |= delta.render_execution_owner_texture_created;
        self.render_execution_owner_renderer_called |= delta.render_execution_owner_renderer_called;
        self.render_execution_owner_damage_submitted |=
            delta.render_execution_owner_damage_submitted;
        self.render_execution_owner_frame_callback_done_sent |=
            delta.render_execution_owner_frame_callback_done_sent;
        self.render_execution_owner_input_support |= delta.render_execution_owner_input_support;
        self.render_execution_owner_core_mutation_invoked |=
            delta.render_execution_owner_core_mutation_invoked;
        self.render_execution_owner_shell_readiness_invocations = self
            .render_execution_owner_shell_readiness_invocations
            .saturating_add(delta.render_execution_owner_shell_readiness_invocations);
        self.render_execution_owner_shell_intents_observed = self
            .render_execution_owner_shell_intents_observed
            .saturating_add(delta.render_execution_owner_shell_intents_observed);
        self.render_execution_owner_shell_observed_intents
            .extend(delta.render_execution_owner_shell_observed_intents);
        self.render_execution_owner_shell_available |= delta.render_execution_owner_shell_available;
        self.render_execution_owner_shell_missing_buffer_import |=
            delta.render_execution_owner_shell_missing_buffer_import;
        self.render_execution_owner_shell_missing_texture_creation |=
            delta.render_execution_owner_shell_missing_texture_creation;
        self.render_execution_owner_shell_missing_renderer_call |=
            delta.render_execution_owner_shell_missing_renderer_call;
        self.render_execution_owner_shell_missing_damage_submit |=
            delta.render_execution_owner_shell_missing_damage_submit;
        self.render_execution_owner_shell_missing_frame_callback_done |=
            delta.render_execution_owner_shell_missing_frame_callback_done;
        self.render_execution_owner_shell_buffer_imported |=
            delta.render_execution_owner_shell_buffer_imported;
        self.render_execution_owner_shell_texture_created |=
            delta.render_execution_owner_shell_texture_created;
        self.render_execution_owner_shell_renderer_called |=
            delta.render_execution_owner_shell_renderer_called;
        self.render_execution_owner_shell_damage_submitted |=
            delta.render_execution_owner_shell_damage_submitted;
        self.render_execution_owner_shell_frame_callback_done_sent |=
            delta.render_execution_owner_shell_frame_callback_done_sent;
        self.render_execution_owner_shell_input_support |=
            delta.render_execution_owner_shell_input_support;
        self.render_execution_owner_shell_core_mutation_invoked |=
            delta.render_execution_owner_shell_core_mutation_invoked;
        self.render_pipeline_skeleton_readiness_invocations = self
            .render_pipeline_skeleton_readiness_invocations
            .saturating_add(delta.render_pipeline_skeleton_readiness_invocations);
        self.render_pipeline_skeleton_intents_observed = self
            .render_pipeline_skeleton_intents_observed
            .saturating_add(delta.render_pipeline_skeleton_intents_observed);
        self.render_pipeline_skeleton_observed_intents
            .extend(delta.render_pipeline_skeleton_observed_intents);
        self.render_pipeline_skeleton_owner_available |=
            delta.render_pipeline_skeleton_owner_available;
        self.render_pipeline_skeleton_missing_execution_owner_shell |=
            delta.render_pipeline_skeleton_missing_execution_owner_shell;
        self.render_pipeline_skeleton_missing_buffer_import |=
            delta.render_pipeline_skeleton_missing_buffer_import;
        self.render_pipeline_skeleton_missing_texture_creation |=
            delta.render_pipeline_skeleton_missing_texture_creation;
        self.render_pipeline_skeleton_missing_renderer_call |=
            delta.render_pipeline_skeleton_missing_renderer_call;
        self.render_pipeline_skeleton_missing_damage_submit |=
            delta.render_pipeline_skeleton_missing_damage_submit;
        self.render_pipeline_skeleton_missing_frame_callback_done |=
            delta.render_pipeline_skeleton_missing_frame_callback_done;
        self.render_pipeline_skeleton_buffer_imported |=
            delta.render_pipeline_skeleton_buffer_imported;
        self.render_pipeline_skeleton_texture_created |=
            delta.render_pipeline_skeleton_texture_created;
        self.render_pipeline_skeleton_renderer_called |=
            delta.render_pipeline_skeleton_renderer_called;
        self.render_pipeline_skeleton_damage_submitted |=
            delta.render_pipeline_skeleton_damage_submitted;
        self.render_pipeline_skeleton_frame_callback_done_sent |=
            delta.render_pipeline_skeleton_frame_callback_done_sent;
        self.render_pipeline_skeleton_input_support |= delta.render_pipeline_skeleton_input_support;
        self.render_pipeline_skeleton_core_mutation_invoked |=
            delta.render_pipeline_skeleton_core_mutation_invoked;
        self.render_backend_capability_report_invocations = self
            .render_backend_capability_report_invocations
            .saturating_add(delta.render_backend_capability_report_invocations);
        self.render_backend_capability_intents_observed = self
            .render_backend_capability_intents_observed
            .saturating_add(delta.render_backend_capability_intents_observed);
        self.render_backend_capability_observed_intents
            .extend(delta.render_backend_capability_observed_intents);
        self.render_backend_capability_owner_available |=
            delta.render_backend_capability_owner_available;
        self.render_backend_capability_backend_registered |=
            delta.render_backend_capability_backend_registered;
        self.render_backend_capability_backend_kind = self
            .render_backend_capability_backend_kind
            .or(delta.render_backend_capability_backend_kind);
        self.render_backend_capability_missing_pipeline_skeleton |=
            delta.render_backend_capability_missing_pipeline_skeleton;
        self.render_backend_capability_missing_backend_registration |=
            delta.render_backend_capability_missing_backend_registration;
        self.render_backend_capability_missing_buffer_import |=
            delta.render_backend_capability_missing_buffer_import;
        self.render_backend_capability_missing_texture_creation |=
            delta.render_backend_capability_missing_texture_creation;
        self.render_backend_capability_missing_renderer_call |=
            delta.render_backend_capability_missing_renderer_call;
        self.render_backend_capability_missing_damage_submit |=
            delta.render_backend_capability_missing_damage_submit;
        self.render_backend_capability_missing_frame_callback_done |=
            delta.render_backend_capability_missing_frame_callback_done;
        self.render_backend_capability_buffer_imported |=
            delta.render_backend_capability_buffer_imported;
        self.render_backend_capability_texture_created |=
            delta.render_backend_capability_texture_created;
        self.render_backend_capability_renderer_called |=
            delta.render_backend_capability_renderer_called;
        self.render_backend_capability_damage_submitted |=
            delta.render_backend_capability_damage_submitted;
        self.render_backend_capability_frame_callback_done_sent |=
            delta.render_backend_capability_frame_callback_done_sent;
        self.render_backend_capability_input_support |=
            delta.render_backend_capability_input_support;
        self.render_backend_capability_core_mutation_invoked |=
            delta.render_backend_capability_core_mutation_invoked;
        self.renderer_backend_registration_invocations = self
            .renderer_backend_registration_invocations
            .saturating_add(delta.renderer_backend_registration_invocations);
        self.renderer_backend_registration_intents_observed = self
            .renderer_backend_registration_intents_observed
            .saturating_add(delta.renderer_backend_registration_intents_observed);
        self.renderer_backend_registration_observed_intents
            .extend(delta.renderer_backend_registration_observed_intents);
        self.renderer_backend_registration_owner_available |=
            delta.renderer_backend_registration_owner_available;
        self.renderer_backend_registration_backend_registered |=
            delta.renderer_backend_registration_backend_registered;
        self.renderer_backend_registration_descriptor_available |=
            delta.renderer_backend_registration_descriptor_available;
        self.renderer_backend_registration_registered_backend_kind = self
            .renderer_backend_registration_registered_backend_kind
            .or(delta.renderer_backend_registration_registered_backend_kind);
        self.renderer_backend_registration_missing_backend_capability |=
            delta.renderer_backend_registration_missing_backend_capability;
        self.renderer_backend_registration_missing_buffer_import |=
            delta.renderer_backend_registration_missing_buffer_import;
        self.renderer_backend_registration_missing_texture_creation |=
            delta.renderer_backend_registration_missing_texture_creation;
        self.renderer_backend_registration_missing_renderer_call |=
            delta.renderer_backend_registration_missing_renderer_call;
        self.renderer_backend_registration_missing_damage_submit |=
            delta.renderer_backend_registration_missing_damage_submit;
        self.renderer_backend_registration_missing_frame_callback_done |=
            delta.renderer_backend_registration_missing_frame_callback_done;
        self.renderer_backend_registration_buffer_imported |=
            delta.renderer_backend_registration_buffer_imported;
        self.renderer_backend_registration_texture_created |=
            delta.renderer_backend_registration_texture_created;
        self.renderer_backend_registration_renderer_called |=
            delta.renderer_backend_registration_renderer_called;
        self.renderer_backend_registration_damage_submitted |=
            delta.renderer_backend_registration_damage_submitted;
        self.renderer_backend_registration_frame_callback_done_sent |=
            delta.renderer_backend_registration_frame_callback_done_sent;
        self.renderer_backend_registration_input_support |=
            delta.renderer_backend_registration_input_support;
        self.renderer_backend_registration_core_mutation_invoked |=
            delta.renderer_backend_registration_core_mutation_invoked;
        self.renderer_backend_owner_shell_readiness_invocations = self
            .renderer_backend_owner_shell_readiness_invocations
            .saturating_add(delta.renderer_backend_owner_shell_readiness_invocations);
        self.renderer_backend_owner_shell_intents_observed = self
            .renderer_backend_owner_shell_intents_observed
            .saturating_add(delta.renderer_backend_owner_shell_intents_observed);
        self.renderer_backend_owner_shell_observed_intents
            .extend(delta.renderer_backend_owner_shell_observed_intents);
        self.renderer_backend_owner_shell_available |= delta.renderer_backend_owner_shell_available;
        self.renderer_backend_owner_shell_bound |= delta.renderer_backend_owner_shell_bound;
        self.renderer_backend_owner_shell_descriptor_available |=
            delta.renderer_backend_owner_shell_descriptor_available;
        self.renderer_backend_owner_shell_registered_backend_kind = self
            .renderer_backend_owner_shell_registered_backend_kind
            .or(delta.renderer_backend_owner_shell_registered_backend_kind);
        self.renderer_backend_owner_shell_missing_descriptor |=
            delta.renderer_backend_owner_shell_missing_descriptor;
        self.renderer_backend_owner_shell_missing_buffer_import |=
            delta.renderer_backend_owner_shell_missing_buffer_import;
        self.renderer_backend_owner_shell_missing_texture_creation |=
            delta.renderer_backend_owner_shell_missing_texture_creation;
        self.renderer_backend_owner_shell_missing_renderer_call |=
            delta.renderer_backend_owner_shell_missing_renderer_call;
        self.renderer_backend_owner_shell_missing_damage_submit |=
            delta.renderer_backend_owner_shell_missing_damage_submit;
        self.renderer_backend_owner_shell_missing_frame_callback_done |=
            delta.renderer_backend_owner_shell_missing_frame_callback_done;
        self.renderer_backend_owner_shell_buffer_imported |=
            delta.renderer_backend_owner_shell_buffer_imported;
        self.renderer_backend_owner_shell_texture_created |=
            delta.renderer_backend_owner_shell_texture_created;
        self.renderer_backend_owner_shell_renderer_called |=
            delta.renderer_backend_owner_shell_renderer_called;
        self.renderer_backend_owner_shell_damage_submitted |=
            delta.renderer_backend_owner_shell_damage_submitted;
        self.renderer_backend_owner_shell_frame_callback_done_sent |=
            delta.renderer_backend_owner_shell_frame_callback_done_sent;
        self.renderer_backend_owner_shell_input_support |=
            delta.renderer_backend_owner_shell_input_support;
        self.renderer_backend_owner_shell_core_mutation_invoked |=
            delta.renderer_backend_owner_shell_core_mutation_invoked;
        self.buffer_import_resource_owner_readiness_invocations = self
            .buffer_import_resource_owner_readiness_invocations
            .saturating_add(delta.buffer_import_resource_owner_readiness_invocations);
        self.buffer_import_resource_owner_intents_observed = self
            .buffer_import_resource_owner_intents_observed
            .saturating_add(delta.buffer_import_resource_owner_intents_observed);
        self.buffer_import_resource_owner_observed_intents
            .extend(delta.buffer_import_resource_owner_observed_intents);
        self.buffer_importer_owner_available |= delta.buffer_importer_owner_available;
        self.buffer_importer_owner_bound |= delta.buffer_importer_owner_bound;
        self.buffer_import_resource_owner_descriptor_evidence_available |=
            delta.buffer_import_resource_owner_descriptor_evidence_available;
        self.buffer_import_resource_owner_registered_backend_kind = self
            .buffer_import_resource_owner_registered_backend_kind
            .or(delta.buffer_import_resource_owner_registered_backend_kind);
        self.buffer_import_resource_owner_missing_renderer_backend_owner_shell |=
            delta.buffer_import_resource_owner_missing_renderer_backend_owner_shell;
        self.buffer_import_resource_owner_missing_descriptor_evidence |=
            delta.buffer_import_resource_owner_missing_descriptor_evidence;
        self.buffer_import_resource_owner_missing_actual_buffer_import |=
            delta.buffer_import_resource_owner_missing_actual_buffer_import;
        self.buffer_import_resource_owner_missing_texture_creation |=
            delta.buffer_import_resource_owner_missing_texture_creation;
        self.buffer_import_resource_owner_missing_renderer_call |=
            delta.buffer_import_resource_owner_missing_renderer_call;
        self.buffer_import_resource_owner_missing_damage_submit |=
            delta.buffer_import_resource_owner_missing_damage_submit;
        self.buffer_import_resource_owner_missing_frame_callback_done |=
            delta.buffer_import_resource_owner_missing_frame_callback_done;
        self.buffer_import_resource_owner_buffer_imported |=
            delta.buffer_import_resource_owner_buffer_imported;
        self.buffer_import_resource_owner_texture_created |=
            delta.buffer_import_resource_owner_texture_created;
        self.buffer_import_resource_owner_renderer_called |=
            delta.buffer_import_resource_owner_renderer_called;
        self.buffer_import_resource_owner_damage_submitted |=
            delta.buffer_import_resource_owner_damage_submitted;
        self.buffer_import_resource_owner_frame_callback_done_sent |=
            delta.buffer_import_resource_owner_frame_callback_done_sent;
        self.buffer_import_resource_owner_input_support |=
            delta.buffer_import_resource_owner_input_support;
        self.buffer_import_resource_owner_core_mutation_invoked |=
            delta.buffer_import_resource_owner_core_mutation_invoked;
        self.buffer_import_planning_invocations = self
            .buffer_import_planning_invocations
            .saturating_add(delta.buffer_import_planning_invocations);
        self.buffer_import_planning_intents_observed = self
            .buffer_import_planning_intents_observed
            .saturating_add(delta.buffer_import_planning_intents_observed);
        self.buffer_import_planning_observed_intents
            .extend(delta.buffer_import_planning_observed_intents);
        self.buffer_import_plan_available |= delta.buffer_import_plan_available;
        self.buffer_import_plan_built |= delta.buffer_import_plan_built;
        self.buffer_import_candidates_observed = self
            .buffer_import_candidates_observed
            .saturating_add(delta.buffer_import_candidates_observed);
        self.buffer_import_required_count = self
            .buffer_import_required_count
            .saturating_add(delta.buffer_import_required_count);
        self.buffer_import_planning_descriptor_evidence_available |=
            delta.buffer_import_planning_descriptor_evidence_available;
        self.buffer_import_planning_registered_backend_kind = self
            .buffer_import_planning_registered_backend_kind
            .or(delta.buffer_import_planning_registered_backend_kind);
        self.buffer_import_planning_missing_buffer_importer_owner |=
            delta.buffer_import_planning_missing_buffer_importer_owner;
        self.buffer_import_planning_missing_descriptor_evidence |=
            delta.buffer_import_planning_missing_descriptor_evidence;
        self.buffer_import_planning_missing_candidate |=
            delta.buffer_import_planning_missing_candidate;
        self.buffer_import_planning_missing_actual_buffer_import |=
            delta.buffer_import_planning_missing_actual_buffer_import;
        self.buffer_import_planning_missing_texture_creation |=
            delta.buffer_import_planning_missing_texture_creation;
        self.buffer_import_planning_missing_renderer_call |=
            delta.buffer_import_planning_missing_renderer_call;
        self.buffer_import_planning_missing_damage_submit |=
            delta.buffer_import_planning_missing_damage_submit;
        self.buffer_import_planning_missing_frame_callback_done |=
            delta.buffer_import_planning_missing_frame_callback_done;
        self.buffer_import_planning_buffer_imported |= delta.buffer_import_planning_buffer_imported;
        self.buffer_import_planning_texture_created |= delta.buffer_import_planning_texture_created;
        self.buffer_import_planning_renderer_called |= delta.buffer_import_planning_renderer_called;
        self.buffer_import_planning_damage_submitted |=
            delta.buffer_import_planning_damage_submitted;
        self.buffer_import_planning_frame_callback_done_sent |=
            delta.buffer_import_planning_frame_callback_done_sent;
        self.buffer_import_planning_input_support |= delta.buffer_import_planning_input_support;
        self.buffer_import_planning_core_mutation_invoked |=
            delta.buffer_import_planning_core_mutation_invoked;
        self.buffer_import_implementation_boundary_invocations = self
            .buffer_import_implementation_boundary_invocations
            .saturating_add(delta.buffer_import_implementation_boundary_invocations);
        self.buffer_import_implementation_descriptors_observed = self
            .buffer_import_implementation_descriptors_observed
            .saturating_add(delta.buffer_import_implementation_descriptors_observed);
        self.buffer_import_implementation_observed_descriptors
            .extend(delta.buffer_import_implementation_observed_descriptors);
        self.buffer_import_implementation_descriptor_available |=
            delta.buffer_import_implementation_descriptor_available;
        self.buffer_import_implementation_descriptor_registered |=
            delta.buffer_import_implementation_descriptor_registered;
        self.buffer_import_implementation_candidates_observed = self
            .buffer_import_implementation_candidates_observed
            .saturating_add(delta.buffer_import_implementation_candidates_observed);
        self.buffer_import_implementation_actual_required_count = self
            .buffer_import_implementation_actual_required_count
            .saturating_add(delta.buffer_import_implementation_actual_required_count);
        self.buffer_import_implementation_importer_owner_evidence_available |=
            delta.buffer_import_implementation_importer_owner_evidence_available;
        self.buffer_import_implementation_renderer_descriptor_evidence_available |=
            delta.buffer_import_implementation_renderer_descriptor_evidence_available;
        self.buffer_import_implementation_registered_backend_kind = self
            .buffer_import_implementation_registered_backend_kind
            .or(delta.buffer_import_implementation_registered_backend_kind);
        self.buffer_import_implementation_missing_planning_intent |=
            delta.buffer_import_implementation_missing_planning_intent;
        self.buffer_import_implementation_missing_plan |=
            delta.buffer_import_implementation_missing_plan;
        self.buffer_import_implementation_missing_importer_owner_evidence |=
            delta.buffer_import_implementation_missing_importer_owner_evidence;
        self.buffer_import_implementation_missing_renderer_descriptor_evidence |=
            delta.buffer_import_implementation_missing_renderer_descriptor_evidence;
        self.buffer_import_implementation_missing_candidate |=
            delta.buffer_import_implementation_missing_candidate;
        self.buffer_import_implementation_missing_actual_buffer_import |=
            delta.buffer_import_implementation_missing_actual_buffer_import;
        self.buffer_import_implementation_missing_texture_creation |=
            delta.buffer_import_implementation_missing_texture_creation;
        self.buffer_import_implementation_missing_renderer_call |=
            delta.buffer_import_implementation_missing_renderer_call;
        self.buffer_import_implementation_missing_damage_submit |=
            delta.buffer_import_implementation_missing_damage_submit;
        self.buffer_import_implementation_missing_frame_callback_done |=
            delta.buffer_import_implementation_missing_frame_callback_done;
        self.buffer_import_implementation_buffer_import_attempted |=
            delta.buffer_import_implementation_buffer_import_attempted;
        self.buffer_import_implementation_buffer_imported |=
            delta.buffer_import_implementation_buffer_imported;
        self.buffer_import_implementation_texture_created |=
            delta.buffer_import_implementation_texture_created;
        self.buffer_import_implementation_renderer_called |=
            delta.buffer_import_implementation_renderer_called;
        self.buffer_import_implementation_damage_submitted |=
            delta.buffer_import_implementation_damage_submitted;
        self.buffer_import_implementation_frame_callback_done_sent |=
            delta.buffer_import_implementation_frame_callback_done_sent;
        self.buffer_import_implementation_input_support |=
            delta.buffer_import_implementation_input_support;
        self.buffer_import_implementation_core_mutation_invoked |=
            delta.buffer_import_implementation_core_mutation_invoked;
        self.buffer_import_adapter_proof_boundary_invocations = self
            .buffer_import_adapter_proof_boundary_invocations
            .saturating_add(delta.buffer_import_adapter_proof_boundary_invocations);
        self.buffer_import_adapter_proofs_observed = self
            .buffer_import_adapter_proofs_observed
            .saturating_add(delta.buffer_import_adapter_proofs_observed);
        self.buffer_import_adapter_observed_proofs
            .extend(delta.buffer_import_adapter_observed_proofs);
        self.buffer_import_adapter_proof_boundary_available |=
            delta.buffer_import_adapter_proof_boundary_available;
        self.buffer_import_adapter_proof_registered |= delta.buffer_import_adapter_proof_registered;
        self.buffer_import_adapter_candidates_observed = self
            .buffer_import_adapter_candidates_observed
            .saturating_add(delta.buffer_import_adapter_candidates_observed);
        self.buffer_import_adapter_actual_required_count = self
            .buffer_import_adapter_actual_required_count
            .saturating_add(delta.buffer_import_adapter_actual_required_count);
        self.buffer_import_adapter_importer_owner_evidence_available |=
            delta.buffer_import_adapter_importer_owner_evidence_available;
        self.buffer_import_adapter_renderer_descriptor_evidence_available |=
            delta.buffer_import_adapter_renderer_descriptor_evidence_available;
        self.buffer_import_adapter_registered_backend_kind = self
            .buffer_import_adapter_registered_backend_kind
            .or(delta.buffer_import_adapter_registered_backend_kind);
        self.buffer_import_adapter_missing_implementation_descriptor |=
            delta.buffer_import_adapter_missing_implementation_descriptor;
        self.buffer_import_adapter_missing_importer_owner_evidence |=
            delta.buffer_import_adapter_missing_importer_owner_evidence;
        self.buffer_import_adapter_missing_renderer_descriptor_evidence |=
            delta.buffer_import_adapter_missing_renderer_descriptor_evidence;
        self.buffer_import_adapter_missing_candidate |=
            delta.buffer_import_adapter_missing_candidate;
        self.buffer_import_adapter_missing_actual_buffer_import |=
            delta.buffer_import_adapter_missing_actual_buffer_import;
        self.buffer_import_adapter_missing_texture_creation |=
            delta.buffer_import_adapter_missing_texture_creation;
        self.buffer_import_adapter_missing_renderer_call |=
            delta.buffer_import_adapter_missing_renderer_call;
        self.buffer_import_adapter_missing_damage_submit |=
            delta.buffer_import_adapter_missing_damage_submit;
        self.buffer_import_adapter_missing_frame_callback_done |=
            delta.buffer_import_adapter_missing_frame_callback_done;
        self.buffer_import_adapter_buffer_import_attempted |=
            delta.buffer_import_adapter_buffer_import_attempted;
        self.buffer_import_adapter_buffer_imported |= delta.buffer_import_adapter_buffer_imported;
        self.buffer_import_adapter_texture_created |= delta.buffer_import_adapter_texture_created;
        self.buffer_import_adapter_renderer_called |= delta.buffer_import_adapter_renderer_called;
        self.buffer_import_adapter_damage_submitted |= delta.buffer_import_adapter_damage_submitted;
        self.buffer_import_adapter_frame_callback_done_sent |=
            delta.buffer_import_adapter_frame_callback_done_sent;
        self.buffer_import_adapter_input_support |= delta.buffer_import_adapter_input_support;
        self.buffer_import_adapter_core_mutation_invoked |=
            delta.buffer_import_adapter_core_mutation_invoked;
        self.buffer_import_precondition_gate_invocations = self
            .buffer_import_precondition_gate_invocations
            .saturating_add(delta.buffer_import_precondition_gate_invocations);
        self.buffer_import_precondition_adapter_proofs_observed = self
            .buffer_import_precondition_adapter_proofs_observed
            .saturating_add(delta.buffer_import_precondition_adapter_proofs_observed);
        self.buffer_import_precondition_observed_adapter_proofs
            .extend(delta.buffer_import_precondition_observed_adapter_proofs);
        self.buffer_import_precondition_gate_available |=
            delta.buffer_import_precondition_gate_available;
        self.buffer_import_preconditions_met_count = self
            .buffer_import_preconditions_met_count
            .saturating_add(delta.buffer_import_preconditions_met_count);
        self.buffer_import_future_preconditions_met_count = self
            .buffer_import_future_preconditions_met_count
            .saturating_add(delta.buffer_import_future_preconditions_met_count);
        self.buffer_import_precondition_candidates_observed = self
            .buffer_import_precondition_candidates_observed
            .saturating_add(delta.buffer_import_precondition_candidates_observed);
        self.buffer_import_precondition_actual_required_count = self
            .buffer_import_precondition_actual_required_count
            .saturating_add(delta.buffer_import_precondition_actual_required_count);
        self.buffer_import_precondition_importer_owner_evidence_available |=
            delta.buffer_import_precondition_importer_owner_evidence_available;
        self.buffer_import_precondition_renderer_descriptor_evidence_available |=
            delta.buffer_import_precondition_renderer_descriptor_evidence_available;
        self.buffer_import_precondition_registered_backend_kind = self
            .buffer_import_precondition_registered_backend_kind
            .or(delta.buffer_import_precondition_registered_backend_kind);
        self.buffer_import_precondition_missing_adapter_proof |=
            delta.buffer_import_precondition_missing_adapter_proof;
        self.buffer_import_precondition_missing_registered_adapter_proof |=
            delta.buffer_import_precondition_missing_registered_adapter_proof;
        self.buffer_import_precondition_missing_importer_owner_evidence |=
            delta.buffer_import_precondition_missing_importer_owner_evidence;
        self.buffer_import_precondition_missing_renderer_descriptor_evidence |=
            delta.buffer_import_precondition_missing_renderer_descriptor_evidence;
        self.buffer_import_precondition_missing_candidate |=
            delta.buffer_import_precondition_missing_candidate;
        self.buffer_import_precondition_missing_actual_import_requirement |=
            delta.buffer_import_precondition_missing_actual_import_requirement;
        self.buffer_import_precondition_missing_actual_buffer_import |=
            delta.buffer_import_precondition_missing_actual_buffer_import;
        self.buffer_import_precondition_missing_texture_creation |=
            delta.buffer_import_precondition_missing_texture_creation;
        self.buffer_import_precondition_missing_renderer_call |=
            delta.buffer_import_precondition_missing_renderer_call;
        self.buffer_import_precondition_missing_damage_submit |=
            delta.buffer_import_precondition_missing_damage_submit;
        self.buffer_import_precondition_missing_frame_callback_done |=
            delta.buffer_import_precondition_missing_frame_callback_done;
        self.buffer_import_precondition_buffer_import_attempted |=
            delta.buffer_import_precondition_buffer_import_attempted;
        self.buffer_import_precondition_buffer_imported |=
            delta.buffer_import_precondition_buffer_imported;
        self.buffer_import_precondition_texture_created |=
            delta.buffer_import_precondition_texture_created;
        self.buffer_import_precondition_renderer_called |=
            delta.buffer_import_precondition_renderer_called;
        self.buffer_import_precondition_damage_submitted |=
            delta.buffer_import_precondition_damage_submitted;
        self.buffer_import_precondition_frame_callback_done_sent |=
            delta.buffer_import_precondition_frame_callback_done_sent;
        self.buffer_import_precondition_input_support |=
            delta.buffer_import_precondition_input_support;
        self.buffer_import_precondition_core_mutation_invoked |=
            delta.buffer_import_precondition_core_mutation_invoked;
        self.buffer_import_execution_dry_run_invocations = self
            .buffer_import_execution_dry_run_invocations
            .saturating_add(delta.buffer_import_execution_dry_run_invocations);
        self.buffer_import_execution_dry_run_reports
            .extend(delta.buffer_import_execution_dry_run_reports);
        self.buffer_import_execution_guard_available |=
            delta.buffer_import_execution_guard_available;
        self.buffer_import_execution_attempted_count = self
            .buffer_import_execution_attempted_count
            .saturating_add(delta.buffer_import_execution_attempted_count);
        self.buffer_import_execution_noop_count = self
            .buffer_import_execution_noop_count
            .saturating_add(delta.buffer_import_execution_noop_count);
        self.buffer_import_execution_blocked_count = self
            .buffer_import_execution_blocked_count
            .saturating_add(delta.buffer_import_execution_blocked_count);
        self.buffer_import_execution_actual_required_count = self
            .buffer_import_execution_actual_required_count
            .saturating_add(delta.buffer_import_execution_actual_required_count);
        self.buffer_import_execution_preconditions_met_count = self
            .buffer_import_execution_preconditions_met_count
            .saturating_add(delta.buffer_import_execution_preconditions_met_count);
        self.buffer_import_execution_missing_precondition_gate_evidence |=
            delta.buffer_import_execution_missing_precondition_gate_evidence;
        self.buffer_import_execution_missing_adapter_proof |=
            delta.buffer_import_execution_missing_adapter_proof;
        self.buffer_import_execution_missing_import_preconditions |=
            delta.buffer_import_execution_missing_import_preconditions;
        self.buffer_import_execution_missing_real_importer |=
            delta.buffer_import_execution_missing_real_importer;
        self.buffer_import_execution_no_actual_import_required |=
            delta.buffer_import_execution_no_actual_import_required;
        self.buffer_import_execution_missing_texture_creation |=
            delta.buffer_import_execution_missing_texture_creation;
        self.buffer_import_execution_missing_renderer_call |=
            delta.buffer_import_execution_missing_renderer_call;
        self.buffer_import_execution_missing_damage_submit |=
            delta.buffer_import_execution_missing_damage_submit;
        self.buffer_import_execution_missing_frame_callback_done |=
            delta.buffer_import_execution_missing_frame_callback_done;
        self.buffer_import_execution_buffer_import_attempted |=
            delta.buffer_import_execution_buffer_import_attempted;
        self.buffer_import_execution_buffer_imported |=
            delta.buffer_import_execution_buffer_imported;
        self.buffer_import_execution_texture_created |=
            delta.buffer_import_execution_texture_created;
        self.buffer_import_execution_renderer_called |=
            delta.buffer_import_execution_renderer_called;
        self.buffer_import_execution_damage_submitted |=
            delta.buffer_import_execution_damage_submitted;
        self.buffer_import_execution_frame_callback_done_sent |=
            delta.buffer_import_execution_frame_callback_done_sent;
        self.buffer_import_execution_input_support |= delta.buffer_import_execution_input_support;
        self.buffer_import_execution_core_mutation_invoked |=
            delta.buffer_import_execution_core_mutation_invoked;
        self.buffer_import_implementation_owner_shell_invocations = self
            .buffer_import_implementation_owner_shell_invocations
            .saturating_add(delta.buffer_import_implementation_owner_shell_invocations);
        self.buffer_import_implementation_owner_shell_reports
            .extend(delta.buffer_import_implementation_owner_shell_reports);
        self.buffer_import_implementation_owner_shell_available |=
            delta.buffer_import_implementation_owner_shell_available;
        self.buffer_import_real_implementation_available |=
            delta.buffer_import_real_implementation_available;
        self.buffer_import_actual_attempt_admitted_count = self
            .buffer_import_actual_attempt_admitted_count
            .saturating_add(delta.buffer_import_actual_attempt_admitted_count);
        self.buffer_import_actual_attempt_blocked_count = self
            .buffer_import_actual_attempt_blocked_count
            .saturating_add(delta.buffer_import_actual_attempt_blocked_count);
        self.buffer_import_implementation_owner_actual_required_count = self
            .buffer_import_implementation_owner_actual_required_count
            .saturating_add(delta.buffer_import_implementation_owner_actual_required_count);
        self.buffer_import_implementation_owner_execution_reports_observed = self
            .buffer_import_implementation_owner_execution_reports_observed
            .saturating_add(delta.buffer_import_implementation_owner_execution_reports_observed);
        self.buffer_import_implementation_owner_missing_execution_dry_run_report |=
            delta.buffer_import_implementation_owner_missing_execution_dry_run_report;
        self.buffer_import_implementation_owner_execution_dry_run_blocked |=
            delta.buffer_import_implementation_owner_execution_dry_run_blocked;
        self.buffer_import_implementation_owner_no_actual_import_required |=
            delta.buffer_import_implementation_owner_no_actual_import_required;
        self.buffer_import_implementation_owner_missing_real_importer |=
            delta.buffer_import_implementation_owner_missing_real_importer;
        self.buffer_import_implementation_owner_missing_texture_creation |=
            delta.buffer_import_implementation_owner_missing_texture_creation;
        self.buffer_import_implementation_owner_missing_renderer_call |=
            delta.buffer_import_implementation_owner_missing_renderer_call;
        self.buffer_import_implementation_owner_missing_damage_submit |=
            delta.buffer_import_implementation_owner_missing_damage_submit;
        self.buffer_import_implementation_owner_missing_frame_callback_done |=
            delta.buffer_import_implementation_owner_missing_frame_callback_done;
        self.buffer_import_implementation_owner_buffer_import_attempted |=
            delta.buffer_import_implementation_owner_buffer_import_attempted;
        self.buffer_import_implementation_owner_buffer_imported |=
            delta.buffer_import_implementation_owner_buffer_imported;
        self.buffer_import_implementation_owner_texture_created |=
            delta.buffer_import_implementation_owner_texture_created;
        self.buffer_import_implementation_owner_renderer_called |=
            delta.buffer_import_implementation_owner_renderer_called;
        self.buffer_import_implementation_owner_damage_submitted |=
            delta.buffer_import_implementation_owner_damage_submitted;
        self.buffer_import_implementation_owner_frame_callback_done_sent |=
            delta.buffer_import_implementation_owner_frame_callback_done_sent;
        self.buffer_import_implementation_owner_input_support |=
            delta.buffer_import_implementation_owner_input_support;
        self.buffer_import_implementation_owner_core_mutation_invoked |=
            delta.buffer_import_implementation_owner_core_mutation_invoked;
        self.buffer_import_actual_attempt_record_invocations = self
            .buffer_import_actual_attempt_record_invocations
            .saturating_add(delta.buffer_import_actual_attempt_record_invocations);
        self.buffer_import_actual_attempt_records
            .extend(delta.buffer_import_actual_attempt_records);
        self.buffer_import_actual_attempt_record_available |=
            delta.buffer_import_actual_attempt_record_available;
        self.buffer_import_actual_attempt_recorded_count = self
            .buffer_import_actual_attempt_recorded_count
            .saturating_add(delta.buffer_import_actual_attempt_recorded_count);
        self.buffer_import_actual_attempt_admission_checked_count = self
            .buffer_import_actual_attempt_admission_checked_count
            .saturating_add(delta.buffer_import_actual_attempt_admission_checked_count);
        self.buffer_import_actual_attempt_record_admitted_count = self
            .buffer_import_actual_attempt_record_admitted_count
            .saturating_add(delta.buffer_import_actual_attempt_record_admitted_count);
        self.buffer_import_actual_attempt_record_blocked_count = self
            .buffer_import_actual_attempt_record_blocked_count
            .saturating_add(delta.buffer_import_actual_attempt_record_blocked_count);
        self.buffer_import_actual_attempt_required_count = self
            .buffer_import_actual_attempt_required_count
            .saturating_add(delta.buffer_import_actual_attempt_required_count);
        self.buffer_import_actual_attempt_missing_owner_shell_report |=
            delta.buffer_import_actual_attempt_missing_owner_shell_report;
        self.buffer_import_actual_attempt_owner_shell_blocked |=
            delta.buffer_import_actual_attempt_owner_shell_blocked;
        self.buffer_import_actual_attempt_no_actual_import_required |=
            delta.buffer_import_actual_attempt_no_actual_import_required;
        self.buffer_import_actual_attempt_missing_admission |=
            delta.buffer_import_actual_attempt_missing_admission;
        self.buffer_import_actual_attempt_missing_real_importer |=
            delta.buffer_import_actual_attempt_missing_real_importer;
        self.buffer_import_actual_attempt_missing_texture_creation |=
            delta.buffer_import_actual_attempt_missing_texture_creation;
        self.buffer_import_actual_attempt_missing_renderer_call |=
            delta.buffer_import_actual_attempt_missing_renderer_call;
        self.buffer_import_actual_attempt_missing_damage_submit |=
            delta.buffer_import_actual_attempt_missing_damage_submit;
        self.buffer_import_actual_attempt_missing_frame_callback_done |=
            delta.buffer_import_actual_attempt_missing_frame_callback_done;
        self.buffer_import_actual_attempt_buffer_import_attempted |=
            delta.buffer_import_actual_attempt_buffer_import_attempted;
        self.buffer_import_actual_attempt_buffer_imported |=
            delta.buffer_import_actual_attempt_buffer_imported;
        self.buffer_import_actual_attempt_texture_created |=
            delta.buffer_import_actual_attempt_texture_created;
        self.buffer_import_actual_attempt_renderer_called |=
            delta.buffer_import_actual_attempt_renderer_called;
        self.buffer_import_actual_attempt_damage_submitted |=
            delta.buffer_import_actual_attempt_damage_submitted;
        self.buffer_import_actual_attempt_frame_callback_done_sent |=
            delta.buffer_import_actual_attempt_frame_callback_done_sent;
        self.buffer_import_actual_attempt_input_support |=
            delta.buffer_import_actual_attempt_input_support;
        self.buffer_import_actual_attempt_core_mutation_invoked |=
            delta.buffer_import_actual_attempt_core_mutation_invoked;
        self.buffer_attached |= delta.buffer_attached;
        self.damage_submitted |= delta.damage_submitted;
        self.frame_callback_requested |= delta.frame_callback_requested;
        self.render_invoked |= delta.render_invoked;
        self.input_invoked |= delta.input_invoked;
        self.core_mutation_invoked |= delta.core_mutation_invoked;
    }
}

/// 一次 bounded loop run 的纯数据汇总报告。
#[must_use = "loop report 包含退出原因、pump 错误和 validation，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopReport {
    /// 实际调用 coordinator pump 的次数。
    pub iterations_run: usize,

    /// 按执行顺序保存的原始 single-pump reports。
    pub pump_reports: Vec<NestedRuntimePumpReport>,

    /// 所有 pump 注册的 core client 数量。
    pub connected_clients_registered: usize,

    /// 所有 pump 关闭的 core client 数量。
    pub disconnected_clients_closed: usize,

    /// 所有 pump 尝试调用 Display dispatch 的次数。
    pub dispatch_calls: usize,

    /// 按 iteration 保存的结构化 pump errors。
    pub errors: Vec<NestedRuntimeLoopError>,

    /// 本次 bounded run 的退出原因。
    pub exit_reason: NestedRuntimeLoopExitReason,

    /// loop 退出时核心状态是否通过 ValidationReport。
    pub validation_is_clean: bool,

    /// 当前 loop capability 快照。
    pub readiness: NestedRuntimeLoopReadinessReport,

    /// 本轮 stop/wakeup 与 interruptible wait 事实。
    pub wakeup: NestedRuntimeWakeupReport,

    /// 本轮 live admission enqueue/drain 事实。
    pub live_admission: NestedRuntimeLiveAdmissionRunSummary,

    /// 本轮 live toplevel unmap drain 事实。
    pub live_unmap: NestedRuntimeLiveUnmapRunSummary,

    /// 本轮 `wl_surface.commit` backlog drain 事实。
    pub surface_commit: NestedRuntimeSurfaceCommitRunSummary,
}

impl NestedRuntimeLoopReport {
    /// 本轮是否没有 pump error，且最终 validation clean。
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
            && self.validation_is_clean
            && self.exit_reason != NestedRuntimeLoopExitReason::Error
    }
}

/// Linux-only nested lifecycle bounded runtime loop。
///
/// 本模块拥有 [`NestedRuntimeCoordinator`]，但 interface 只暴露有限执行与 cooperative
/// stop。循环实现不读取或写入 core registry；每一轮 mutation 仍严格走 coordinator 内
/// 既有的 `BackendEvent -> CoreCommand -> State` bridge。
pub struct NestedRuntimeLoop {
    coordinator: NestedRuntimeCoordinator,
    stop_handle: NestedRuntimeLoopStopHandle,
}

impl NestedRuntimeLoop {
    /// 使用指定 Wayland socket 名称创建 bounded loop。
    ///
    /// # Errors
    ///
    /// coordinator 的 Display、socket、calloop source 或 accept flow 初始化失败时返回错误。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let coordinator = NestedRuntimeCoordinator::with_socket_name(name)?;
        let stop_handle = NestedRuntimeLoopStopHandle::new(coordinator.loop_signal());

        Ok(Self {
            coordinator,
            stop_handle,
        })
    }

    /// 返回 loop 已绑定的 Wayland socket 名称。
    pub fn socket_name(&self) -> &str {
        self.coordinator.socket_name()
    }

    /// 返回可供其他调用方请求 cooperative stop 的 handle。
    pub fn stop_handle(&self) -> NestedRuntimeLoopStopHandle {
        self.stop_handle.clone()
    }

    /// 在硬性 iteration 上限内重复调用 coordinator live admission/unmap pump。
    ///
    /// `max_iterations` 是防无限循环的不可绕过上限。stop、error 和 idle 只会提前退出；
    /// 本方法没有 protocol event-source wakeup，也不是完整 compositor runtime。
    pub fn run_for_iterations(
        &mut self,
        state: &mut State,
        config: NestedRuntimeLoopConfig,
    ) -> NestedRuntimeLoopReport {
        let stop_handle = self.stop_handle.clone();
        let mut admission_tick_index = 0u64;
        run_with_observed_pump(state, config, &stop_handle, |state, timeout| {
            admission_tick_index = admission_tick_index.saturating_add(1);
            ObservedNestedRuntimePumpReport::from_live_admission_unmap(
                self.coordinator
                    .pump_once_with_live_toplevel_admission_and_unmap_drain(
                        state,
                        timeout,
                        RuntimeToplevelAdmissionDrainTick::phase52y_default(admission_tick_index),
                    ),
            )
        })
    }
}

struct ObservedNestedRuntimePumpReport {
    lifecycle_report: NestedRuntimePumpReport,
    live_admission: NestedRuntimeLiveAdmissionRunSummary,
    live_unmap: NestedRuntimeLiveUnmapRunSummary,
    surface_commit: NestedRuntimeSurfaceCommitRunSummary,
}

impl ObservedNestedRuntimePumpReport {
    fn lifecycle_only(lifecycle_report: NestedRuntimePumpReport) -> Self {
        Self {
            lifecycle_report,
            live_admission: NestedRuntimeLiveAdmissionRunSummary::default(),
            live_unmap: NestedRuntimeLiveUnmapRunSummary::default(),
            surface_commit: NestedRuntimeSurfaceCommitRunSummary::default(),
        }
    }

    fn from_live_admission_unmap(report: NestedRuntimeLiveAdmissionUnmapPumpReport) -> Self {
        let mut surface_commit = NestedRuntimeSurfaceCommitRunSummary::from_surface_commit_drain(
            &report.surface_commit_drain_report,
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_dirty_intent_drain(
                &report.render_dirty_intent_drain_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_renderer_admission(
                &report.renderer_admission_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_renderer_owner_boundary(
                &report.renderer_owner_boundary_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_renderer_owner_shell_readiness(
                &report.renderer_owner_shell_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_importer_shell_readiness(
                &report.buffer_importer_shell_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_texture_support_shell_readiness(
                &report.texture_support_shell_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_operation_readiness(
                &report.render_operation_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_operation_intent_drain(
                &report.render_operation_intent_drain_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_execution_owner_boundary(
                &report.render_execution_owner_boundary_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_execution_owner_shell_readiness(
                &report.render_execution_owner_shell_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_pipeline_skeleton_readiness(
                &report.render_pipeline_skeleton_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_render_backend_capability_report(
                &report.render_backend_capability_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_renderer_backend_registration_report(
                &report.renderer_backend_registration_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_renderer_backend_owner_shell_readiness(
                &report.renderer_backend_owner_shell_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_resource_owner_readiness(
                &report.buffer_import_resource_owner_readiness_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_planning_report(
                &report.buffer_import_planning_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_implementation_boundary_report(
                &report.buffer_import_implementation_boundary_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_adapter_proof_boundary_report(
                &report.buffer_import_adapter_proof_boundary_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_precondition_gate_report(
                &report.buffer_import_precondition_gate_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_execution_dry_run_report(
                &report.buffer_import_execution_dry_run_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_implementation_owner_shell_report(
                &report.buffer_import_implementation_owner_shell_report,
            ),
        );
        surface_commit.observe(
            NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_actual_attempt_record(
                &report.buffer_import_actual_attempt_record,
            ),
        );

        Self {
            live_admission: NestedRuntimeLiveAdmissionRunSummary::from_live_pump(
                &NestedRuntimeLiveAdmissionPumpReport {
                    lifecycle_report: report.lifecycle_report.clone(),
                    live_admission_owner_report: report.live_admission_owner_report.clone(),
                    admission_drain_report: report.admission_drain_report.clone(),
                },
            ),
            live_unmap: NestedRuntimeLiveUnmapRunSummary::from_live_admission_unmap(&report),
            surface_commit,
            lifecycle_report: report.lifecycle_report,
        }
    }
}

fn run_with_pump<F>(
    state: &mut State,
    config: NestedRuntimeLoopConfig,
    stop_handle: &NestedRuntimeLoopStopHandle,
    mut pump: F,
) -> NestedRuntimeLoopReport
where
    F: FnMut(&mut State, Duration) -> NestedRuntimePumpReport,
{
    run_with_observed_pump(state, config, stop_handle, |state, timeout| {
        ObservedNestedRuntimePumpReport::lifecycle_only(pump(state, timeout))
    })
}

fn run_with_observed_pump<F>(
    state: &mut State,
    config: NestedRuntimeLoopConfig,
    stop_handle: &NestedRuntimeLoopStopHandle,
    mut pump: F,
) -> NestedRuntimeLoopReport
where
    F: FnMut(&mut State, Duration) -> ObservedNestedRuntimePumpReport,
{
    let started_at = Instant::now();
    // 不按调用方给出的上限预分配，避免极大但仍有限的 max_iterations 在 run 前触发巨额分配。
    let mut pump_reports = Vec::new();
    let mut live_admission = NestedRuntimeLiveAdmissionRunSummary::default();
    let mut live_unmap = NestedRuntimeLiveUnmapRunSummary::default();
    let mut surface_commit = NestedRuntimeSurfaceCommitRunSummary::default();
    let mut connected_clients_registered = 0usize;
    let mut disconnected_clients_closed = 0usize;
    let mut dispatch_calls = 0usize;
    let mut errors = Vec::new();
    let mut exit_reason = NestedRuntimeLoopExitReason::MaxIterationsReached;
    let mut wakeup_requested = false;
    let mut stop_requested = false;
    let mut wait_interrupted = false;

    if stop_handle.take_stop_request() {
        stop_requested = true;
        wakeup_requested = stop_handle.take_wakeup_request();
        wait_interrupted = stop_handle.take_wait_interrupt();
        exit_reason = NestedRuntimeLoopExitReason::StopRequested;
    } else {
        for _ in 0..config.max_iterations {
            // 生产路径只能通过 coordinator pump；loop 不得绕过 bridge 直接修改 core。
            stop_handle.begin_wait();
            let observed_report = pump(state, config.pump_timeout);
            stop_handle.end_wait();
            wakeup_requested |= stop_handle.take_wakeup_request();
            wait_interrupted |= stop_handle.take_wait_interrupt();
            let iteration = pump_reports.len().saturating_add(1);
            let live_admission_has_progress = observed_report.live_admission.has_progress();
            let live_unmap_has_progress = observed_report.live_unmap.has_progress();
            let surface_commit_has_progress = observed_report.surface_commit.has_progress();
            live_admission.observe(observed_report.live_admission);
            live_unmap.observe(observed_report.live_unmap);
            surface_commit.observe(observed_report.surface_commit);
            let report = observed_report.lifecycle_report;
            let report_is_idle = pump_report_is_idle(&report);
            let report_has_errors = !report.errors.is_empty();

            connected_clients_registered =
                connected_clients_registered.saturating_add(report.registered_core_clients.len());
            disconnected_clients_closed =
                disconnected_clients_closed.saturating_add(report.closed_core_clients.len());
            if report.dispatch_clients_called {
                dispatch_calls = dispatch_calls.saturating_add(1);
            }

            if report_has_errors {
                errors.push(NestedRuntimeLoopError {
                    iteration,
                    pump_errors: report.errors.clone(),
                });
            }
            pump_reports.push(report);

            if report_has_errors && !config.continue_after_error {
                exit_reason = NestedRuntimeLoopExitReason::Error;
                break;
            }
            if stop_handle.take_stop_request() {
                stop_requested = true;
                exit_reason = if wait_interrupted {
                    NestedRuntimeLoopExitReason::Interrupted
                } else {
                    NestedRuntimeLoopExitReason::StopRequested
                };
                break;
            }
            if config.stop_when_idle
                && report_is_idle
                && !live_admission_has_progress
                && !live_unmap_has_progress
                && !surface_commit_has_progress
            {
                exit_reason = NestedRuntimeLoopExitReason::Idle;
                break;
            }
        }
    }

    let elapsed_before_exit = started_at.elapsed();
    let exited_before_timeout = wait_interrupted && elapsed_before_exit < config.pump_timeout;

    NestedRuntimeLoopReport {
        iterations_run: pump_reports.len(),
        pump_reports,
        connected_clients_registered,
        disconnected_clients_closed,
        dispatch_calls,
        errors,
        exit_reason,
        validation_is_clean: state.validate().is_clean(),
        readiness: nested_runtime_loop_readiness_report(),
        wakeup: NestedRuntimeWakeupReport {
            wakeup_requested,
            stop_requested,
            wait_interrupted,
            elapsed_before_exit,
            configured_pump_timeout: config.pump_timeout,
            exited_before_timeout,
        },
        live_admission,
        live_unmap,
        surface_commit,
    }
}

fn pump_report_is_idle(report: &NestedRuntimePumpReport) -> bool {
    report.accepted_clients == 0
        && report.inserted_clients == 0
        && report.connected_events_drained == 0
        && report.registered_core_clients.is_empty()
        && report.dispatched_requests == Some(0)
        && report.disconnected_events_drained == 0
        && report.closed_core_clients.is_empty()
        && report.errors.is_empty()
}

#[cfg(test)]
impl NestedRuntimeLoop {
    /// 测试专用：让 orchestrator proof 在 loop 持有的 coordinator display 上制造 observation。
    pub(crate) fn display_mut_for_controlled_toplevel_registration(
        &mut self,
    ) -> &mut crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe {
        self.coordinator
            .display_mut_for_controlled_toplevel_registration()
    }

    /// 测试专用：读取 loop-owned coordinator admission surface mapping。
    pub(crate) fn admission_surface_mapping(
        &self,
        adapter_surface: crate::smithay_backend::surface_xdg_admission::AdapterSurfaceId,
    ) -> Option<crate::core::surface::SurfaceId> {
        self.coordinator.admission_surface_mapping(adapter_surface)
    }

    /// 测试专用：读取 loop-owned coordinator admission toplevel mapping。
    pub(crate) fn admission_toplevel_mapping(
        &self,
        adapter_toplevel: crate::smithay_backend::surface_xdg_admission::AdapterToplevelId,
    ) -> Option<crate::core::workspace::WindowId> {
        self.coordinator
            .admission_toplevel_mapping(adapter_toplevel)
    }

    /// 测试专用：读取 loop-owned coordinator pending admission count。
    pub(crate) fn admission_pending_count(&self) -> usize {
        self.coordinator.admission_pending_count()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        os::unix::net::UnixStream,
        path::Path,
        thread,
        time::{Duration, Instant},
    };

    use smithay::reexports::calloop::EventLoop;

    use super::{
        NestedRuntimeLiveAdmissionRunSummary, NestedRuntimeLiveUnmapRunSummary, NestedRuntimeLoop,
        NestedRuntimeLoopBlocker, NestedRuntimeLoopConfig, NestedRuntimeLoopExitReason,
        NestedRuntimeLoopStopHandle, NestedRuntimeSurfaceCommitRunSummary,
        ObservedNestedRuntimePumpReport, nested_runtime_loop_readiness_report,
        run_with_observed_pump, run_with_pump,
    };
    use crate::{
        core::state::State,
        smithay_backend::{
            linux_toplevel_identity_registration::adapter_toplevel_identity_registration_report,
            linux_wl_surface_identity::{
                controlled_wl_surface_commit_observation_report,
                controlled_wl_surface_damage_commit_observation_report,
                controlled_wl_surface_frame_callback_commit_observation_report,
                controlled_wl_surface_null_attach_commit_observation_report,
                controlled_wl_surface_render_dirty_readiness_commit_observation_report,
            },
            nested_runtime_coordinator::{
                NestedRuntimePumpError, NestedRuntimePumpErrorKind, NestedRuntimePumpReport,
                nested_runtime_coordinator_readiness_report,
            },
            test_support::{assert_runtime_dir, unique_socket_name},
        },
    };

    fn config(max_iterations: usize) -> NestedRuntimeLoopConfig {
        NestedRuntimeLoopConfig {
            max_iterations,
            pump_timeout: Duration::ZERO,
            stop_when_idle: false,
            continue_after_error: false,
        }
    }

    fn synthetic_pump_report(errors: Vec<NestedRuntimePumpError>) -> NestedRuntimePumpReport {
        NestedRuntimePumpReport {
            accepted_clients: 0,
            inserted_clients: 0,
            connected_events_drained: 0,
            registered_core_clients: Vec::new(),
            dispatch_clients_called: true,
            dispatched_requests: Some(0),
            disconnected_events_drained: 0,
            closed_core_clients: Vec::new(),
            validation_is_clean: true,
            errors,
            readiness: nested_runtime_coordinator_readiness_report(),
        }
    }

    fn isolated_stop_handle() -> (EventLoop<'static, ()>, NestedRuntimeLoopStopHandle) {
        let event_loop: EventLoop<'static, ()> =
            EventLoop::try_new().expect("测试 stop handle 必须拥有真实 calloop notifier");
        let stop_handle = NestedRuntimeLoopStopHandle::new(event_loop.get_signal());
        (event_loop, stop_handle)
    }

    /// 验证 51M-C 只上调 Linux proof 支持的 wakeup 字段，不冒充完整 runtime。
    #[test]
    fn nested_runtime_wakeup_proof_capabilities_are_precise() {
        let report = nested_runtime_loop_readiness_report();

        assert_eq!(
            report.blockers,
            vec![NestedRuntimeLoopBlocker::MissingCompleteRuntimeLoop]
        );
        assert!(report.loop_boundary_defined);
        assert!(report.nested_runtime_loop_available);
        assert!(report.bounded_loop_available);
        assert!(report.stop_requested_supported);
        assert!(report.wakeup_supported);
        assert!(report.interruptible_wait_available);
        assert!(report.stop_can_interrupt_wait);
        assert!(!report.long_running_loop_available);
        assert!(!report.accepts_clients);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.is_bounded_loop_ready());
        assert!(report.is_interruptible_wait_ready());
    }

    /// `max_iterations = 0` 必须安全退出，不能隐式执行一次 pump。
    #[test]
    fn nested_runtime_loop_zero_iterations_is_safe() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-zero");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let mut state = State::new();

        let report = runtime_loop.run_for_iterations(&mut state, config(0));

        assert_eq!(report.iterations_run, 0);
        assert!(report.pump_reports.is_empty());
        assert_eq!(report.dispatch_calls, 0);
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
    }

    /// max_iterations 必须成为不可绕过的硬上限，避免无限循环。
    #[test]
    fn nested_runtime_loop_respects_max_iterations() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-max");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let mut state = State::new();

        let report = runtime_loop.run_for_iterations(&mut state, config(3));

        assert_eq!(report.iterations_run, 3);
        assert_eq!(report.pump_reports.len(), 3);
        assert_eq!(report.dispatch_calls, 3);
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert!(report.validation_is_clean);
    }

    /// stop_when_idle 必须在第一次无活动 pump 后提前退出。
    #[test]
    fn nested_runtime_loop_exits_when_idle() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-idle");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let mut state = State::new();
        let mut idle_config = config(4);
        idle_config.stop_when_idle = true;

        let report = runtime_loop.run_for_iterations(&mut state, idle_config);

        assert_eq!(report.iterations_run, 1);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.validation_is_clean);
    }

    /// stop handle 可在一次 pump 后请求提前退出，且请求被消费。
    #[test]
    fn nested_runtime_loop_exits_on_stop_request() {
        let (_event_loop, stop_handle) = isolated_stop_handle();
        let stop_from_pump = stop_handle.clone();
        let mut state = State::new();
        let mut calls = 0usize;

        let report = run_with_pump(&mut state, config(4), &stop_handle, |_, _| {
            calls = calls.saturating_add(1);
            stop_from_pump.request_stop();
            synthetic_pump_report(Vec::new())
        });

        assert_eq!(calls, 1);
        assert_eq!(report.iterations_run, 1);
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::StopRequested
        );
        assert!(!stop_handle.is_stop_requested());
        assert!(report.validation_is_clean);
        assert!(report.readiness.is_bounded_loop_ready());
        assert!(report.readiness.wakeup_supported);
        assert!(!report.readiness.long_running_loop_available);
    }

    /// public stop handle 在 run 前请求停止时不得额外执行 pump。
    #[test]
    fn nested_runtime_loop_public_stop_handle_exits_before_pump() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-public-stop");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let stop_handle = runtime_loop.stop_handle();
        let mut state = State::new();

        stop_handle.request_stop();
        let report = runtime_loop.run_for_iterations(&mut state, config(3));

        assert_eq!(report.iterations_run, 0);
        assert!(report.pump_reports.is_empty());
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::StopRequested
        );
        assert!(!stop_handle.is_stop_requested());
        assert!(report.validation_is_clean);
    }

    /// pump error 必须进入 loop report，并按配置以 Error 退出而不是 panic。
    #[test]
    fn nested_runtime_loop_reports_pump_error() {
        let (_event_loop, stop_handle) = isolated_stop_handle();
        let mut state = State::new();
        let error = NestedRuntimePumpError {
            kind: NestedRuntimePumpErrorKind::DisplayDispatch,
            message: "controlled loop dispatch failure".to_owned(),
        };

        let report = run_with_pump(&mut state, config(3), &stop_handle, |_, _| {
            synthetic_pump_report(vec![error.clone()])
        });

        assert_eq!(report.iterations_run, 1);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Error);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].iteration, 1);
        assert_eq!(report.errors[0].pump_errors, vec![error]);
        assert!(!report.is_successful());
        assert!(report.validation_is_clean);
    }

    /// 真实 calloop proof：外部 wakeup 必须打断等待中的长 pump timeout。
    #[test]
    fn nested_runtime_loop_wakeup_interrupts_wait() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-wakeup");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("interruptible loop 必须绑定测试 socket");
        let wakeup_handle = runtime_loop.stop_handle();
        let configured_timeout = Duration::from_secs(5);
        let mut state = State::new();

        let interrupter = thread::spawn(move || {
            let wait_deadline = Instant::now() + Duration::from_secs(1);
            while !wakeup_handle.is_waiting() {
                assert!(
                    Instant::now() < wait_deadline,
                    "loop 必须在有界时间内进入 pump wait"
                );
                thread::sleep(Duration::from_millis(1));
            }
            wakeup_handle.request_stop_and_wakeup();
        });
        let started_at = Instant::now();
        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 1,
                pump_timeout: configured_timeout,
                stop_when_idle: false,
                continue_after_error: false,
            },
        );
        let observed_elapsed = started_at.elapsed();
        interrupter.join().expect("wakeup thread 不得 panic");

        assert_eq!(report.iterations_run, 1);
        assert_eq!(report.pump_reports.len(), 1);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Interrupted);
        assert!(report.wakeup.wakeup_requested);
        assert!(report.wakeup.stop_requested);
        assert!(report.wakeup.wait_interrupted);
        assert_eq!(report.wakeup.configured_pump_timeout, configured_timeout);
        assert!(report.wakeup.exited_before_timeout);
        assert!(report.wakeup.elapsed_before_exit < configured_timeout);
        assert!(observed_elapsed < Duration::from_secs(2));
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(report.readiness.wakeup_supported);
        assert!(report.readiness.is_interruptible_wait_ready());
        assert!(!report.readiness.long_running_loop_available);
    }

    /// Linux-only 真实 proof：bounded loop 多次 pump 并保留 connected/disconnected lifecycle。
    #[test]
    fn nested_runtime_loop_runs_lifecycle_across_multiple_pumps() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-lifecycle");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(runtime_loop.socket_name());
        let client_stream =
            UnixStream::connect(socket_path).expect("测试 peer 必须连接真实 Wayland socket");
        let mut state = State::new();

        drop(client_stream);
        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 2,
                pump_timeout: Duration::from_secs(1),
                stop_when_idle: false,
                continue_after_error: false,
            },
        );

        assert_eq!(report.iterations_run, 2);
        assert_eq!(report.connected_clients_registered, 1);
        assert_eq!(report.disconnected_clients_closed, 1);
        assert_eq!(report.dispatch_calls, 2);
        assert!(report.errors.is_empty());
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(report.readiness.is_bounded_loop_ready());
        assert!(report.readiness.wakeup_supported);
        assert!(!report.readiness.long_running_loop_available);
        let client = report.pump_reports[0].registered_core_clients[0];
        assert!(!state.clients.is_alive(client));
    }

    /// Linux-only proof：bounded loop 每轮使用 live admission pump，而不是只跑 lifecycle pump。
    #[test]
    fn nested_runtime_loop_drains_live_toplevel_admission() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-live-admission");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let registration = {
            let display = runtime_loop
                .coordinator
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

        let report = runtime_loop.run_for_iterations(&mut state, config(1));

        assert_eq!(report.iterations_run, 1);
        assert!(report.is_successful());
        assert_eq!(report.live_admission.owner_invocations, 1);
        assert_eq!(report.live_admission.enqueue_invocations, 1);
        assert_eq!(report.live_admission.admissions_enqueued, 1);
        assert_eq!(report.live_admission.drain_invocations, 1);
        assert_eq!(report.live_admission.admissions_consumed, 1);
        assert_eq!(report.live_admission.pending_admissions_after, 0);
        assert_eq!(
            runtime_loop
                .coordinator
                .admission_surface_mapping(registration.adapter_surface_id),
            Some(1)
        );
        let toplevel_mapping = runtime_loop
            .coordinator
            .admission_toplevel_mapping(registration.adapter_toplevel_id);
        if report.live_unmap.ledger_unmaps > 0 {
            assert_eq!(toplevel_mapping, None);
            assert!(report.live_unmap.core_detaches > 0);
            assert!(state.registry.records().iter().any(|record| !record.alive));
        } else {
            assert!(toplevel_mapping.is_some());
        }
        assert_eq!(runtime_loop.coordinator.admission_pending_count(), 0);
        assert!(state.surfaces.get(1).is_some());
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：同一个 live callback observation 不能在多轮 loop 中重复入队。
    #[test]
    fn nested_runtime_loop_deduplicates_live_toplevel_admission_observation() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-live-admission-dedupe");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let registration = {
            let display = runtime_loop
                .coordinator
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

        let report = runtime_loop.run_for_iterations(&mut state, config(2));

        assert_eq!(report.iterations_run, 2);
        assert!(report.is_successful());
        assert_eq!(report.live_admission.owner_invocations, 2);
        assert_eq!(report.live_admission.enqueue_invocations, 1);
        assert_eq!(report.live_admission.admissions_enqueued, 1);
        assert_eq!(report.live_admission.drain_invocations, 2);
        assert_eq!(report.live_admission.admissions_consumed, 1);
        assert_eq!(report.live_admission.pending_admissions_after, 0);
        assert_eq!(
            runtime_loop
                .coordinator
                .admission_surface_mapping(registration.adapter_surface_id),
            Some(1)
        );
        let toplevel_mapping = runtime_loop
            .coordinator
            .admission_toplevel_mapping(registration.adapter_toplevel_id);
        if report.live_unmap.ledger_unmaps > 0 {
            assert_eq!(toplevel_mapping, None);
            assert!(report.live_unmap.core_detaches > 0);
            assert!(state.registry.records().iter().any(|record| !record.alive));
        } else {
            assert!(toplevel_mapping.is_some());
        }
        assert_eq!(runtime_loop.coordinator.admission_pending_count(), 0);
        assert!(state.surfaces.get(1).is_some());
        assert_eq!(state.surfaces.records().len(), 1);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：bounded loop 每轮同时 drain live admission 与 live unmap。
    #[test]
    fn nested_runtime_loop_drains_live_toplevel_unmap() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-live-unmap");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let registration = {
            let display = runtime_loop
                .coordinator
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

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 2,
                pump_timeout: Duration::from_millis(1),
                stop_when_idle: false,
                continue_after_error: false,
            },
        );

        assert_eq!(report.iterations_run, 2);
        assert!(report.is_successful());
        assert_eq!(report.live_admission.owner_invocations, 2);
        assert_eq!(report.live_admission.enqueue_invocations, 1);
        assert_eq!(report.live_admission.admissions_enqueued, 1);
        assert_eq!(report.live_admission.drain_invocations, 2);
        assert_eq!(report.live_admission.admissions_consumed, 1);
        assert_eq!(report.live_admission.pending_admissions_after, 0);
        assert_eq!(report.live_unmap.drain_invocations, 2);
        assert_eq!(report.live_unmap.live_unmap_observations, 1);
        assert_eq!(report.live_unmap.ledger_unmaps, 1);
        assert_eq!(report.live_unmap.core_detaches, 1);
        assert_eq!(report.live_unmap.surface_mappings_retained, 1);
        assert_eq!(report.live_unmap.toplevel_mappings_removed, 1);
        assert_eq!(
            runtime_loop
                .coordinator
                .admission_surface_mapping(registration.adapter_surface_id),
            Some(1)
        );
        assert_eq!(
            runtime_loop
                .coordinator
                .admission_toplevel_mapping(registration.adapter_toplevel_id),
            None
        );
        assert!(state.surfaces.is_alive(1));
        assert!(state.registry.records().iter().any(|record| !record.alive));
        assert!(state.validate().is_clean());
    }

    /// stop_when_idle 不能忽略 live unmap drain 的进展。
    #[test]
    fn nested_runtime_loop_stop_when_idle_counts_live_unmap_progress() {
        let (_event_loop, stop_handle) = isolated_stop_handle();
        let mut state = State::new();
        let mut calls = 0usize;

        let report = run_with_observed_pump(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
            &stop_handle,
            |_, _| {
                calls = calls.saturating_add(1);
                let live_unmap = if calls == 1 {
                    NestedRuntimeLiveUnmapRunSummary {
                        drain_invocations: 1,
                        live_unmap_observations: 1,
                        ledger_unmaps: 1,
                        core_detaches: 1,
                        surface_mappings_retained: 1,
                        toplevel_mappings_removed: 1,
                    }
                } else {
                    NestedRuntimeLiveUnmapRunSummary::default()
                };

                ObservedNestedRuntimePumpReport {
                    lifecycle_report: synthetic_pump_report(Vec::new()),
                    live_admission: NestedRuntimeLiveAdmissionRunSummary::default(),
                    live_unmap,
                    surface_commit: NestedRuntimeSurfaceCommitRunSummary::default(),
                }
            },
        );

        assert_eq!(calls, 2);
        assert_eq!(report.iterations_run, 2);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.is_successful());
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.live_unmap_observations, 1);
        assert_eq!(report.live_unmap.ledger_unmaps, 1);
        assert_eq!(report.live_unmap.core_detaches, 1);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：stop_when_idle 不能在 live admission backlog 仍有进展时提前退出。
    #[test]
    fn nested_runtime_loop_stop_when_idle_drains_live_admission_backlog() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-live-admission-idle-backlog");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_registration, second_registration) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_xdg_shell_global()
                .expect("测试 xdg-shell global 必须初始化");
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_registration = adapter_toplevel_identity_registration_report(display)
                .expect("首次 adapter identity registration proof 必须完成");
            let second_registration = adapter_toplevel_identity_registration_report(display)
                .expect("第二次 adapter identity registration proof 必须完成");

            (first_registration, second_registration)
        };
        let mut state = State::new();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert_eq!(report.iterations_run, 3);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.is_successful());
        assert_eq!(report.live_admission.owner_invocations, 3);
        assert_eq!(report.live_admission.enqueue_invocations, 2);
        assert_eq!(report.live_admission.admissions_enqueued, 2);
        assert_eq!(report.live_admission.drain_invocations, 3);
        assert_eq!(report.live_admission.admissions_consumed, 2);
        assert_eq!(report.live_admission.pending_admissions_after, 0);
        assert_eq!(
            runtime_loop
                .coordinator
                .admission_surface_mapping(first_registration.adapter_surface_id),
            Some(1)
        );
        assert_eq!(
            runtime_loop
                .coordinator
                .admission_surface_mapping(second_registration.adapter_surface_id),
            Some(2)
        );
        assert_eq!(runtime_loop.coordinator.admission_pending_count(), 0);
        assert!(state.surfaces.get(1).is_some());
        assert!(state.surfaces.get(2).is_some());
        assert_eq!(state.surfaces.records().len(), 2);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：bounded loop drains pure-data `wl_surface.commit` backlog FIFO.
    #[test]
    fn nested_runtime_loop_drains_wl_surface_commit_backlog_fifo_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-surface-commit-backlog");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("首个 controlled commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 controlled commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert_eq!(report.iterations_run, 3);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.is_successful());
        assert_eq!(report.surface_commit.drain_invocations, 3);
        assert_eq!(report.surface_commit.commit_observations_drained, 2);
        assert_eq!(report.surface_commit.commit_observation_errors, 0);
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.drained_commit_sequences, vec![1, 2]);
        assert_eq!(report.surface_commit.buffer_attach_observations, 0);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 0);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 0);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 0);
        assert_eq!(report.surface_commit.frame_callback_observations, 0);
        assert_eq!(report.surface_commit.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：runtime drain report 保留 commit buffer evidence 纯数据。
    #[test]
    fn nested_runtime_loop_drains_wl_surface_commit_buffer_evidence_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-surface-commit-buffer-evidence");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit = controlled_wl_surface_null_attach_commit_observation_report(display)
                .expect("首个 null attach commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.buffer_attach_observations, 1);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 1);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 0);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 0);
        assert_eq!(report.surface_commit.frame_callback_observations, 0);
        assert_eq!(report.surface_commit.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：runtime drain report 保留 commit damage evidence 纯数据。
    #[test]
    fn nested_runtime_loop_drains_wl_surface_commit_damage_evidence_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-surface-commit-damage-evidence");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit = controlled_wl_surface_damage_commit_observation_report(display)
                .expect("首个 damage commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.buffer_attach_observations, 0);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 0);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 1);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 1);
        assert_eq!(report.surface_commit.frame_callback_observations, 0);
        assert_eq!(report.surface_commit.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：runtime drain report 保留 commit frame callback evidence 纯数据。
    #[test]
    fn nested_runtime_loop_drains_wl_surface_commit_frame_callback_evidence_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-surface-commit-frame-callback-evidence");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit =
                controlled_wl_surface_frame_callback_commit_observation_report(display)
                    .expect("首个 frame callback commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.buffer_attach_observations, 0);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 0);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 0);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 0);
        assert_eq!(report.surface_commit.frame_callback_observations, 1);
        assert_eq!(report.surface_commit.frame_callback_count, 1);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：runtime drain report 从 commit evidence 生成 render-dirty intent。
    #[test]
    fn nested_runtime_loop_builds_render_dirty_readiness_intents_fifo_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-render-dirty-readiness-intent");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(
            report.surface_commit.render_dirty_readiness_intents.len(),
            2
        );
        let first_intent = &report.surface_commit.render_dirty_readiness_intents[0];
        let second_intent = &report.surface_commit.render_dirty_readiness_intents[1];
        assert_eq!(
            first_intent.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_intent.surface_identity_key,
            first_commit.surface_identity_key
        );
        assert_eq!(first_intent.commit_sequence, first_commit.commit_sequence);
        assert_eq!(second_intent.commit_sequence, second_commit.commit_sequence);
        assert!(first_intent.buffer_attach_observed);
        assert!(!first_intent.buffer_present);
        assert!(first_intent.buffer_removed);
        assert!(!first_intent.renderable_buffer);
        assert!(first_intent.damage_observed);
        assert_eq!(first_intent.surface_damage_rects, 0);
        assert_eq!(first_intent.buffer_damage_rects, 1);
        assert!(first_intent.frame_callback_observed);
        assert_eq!(first_intent.frame_callback_count, 1);
        assert!(!first_intent.buffer_imported);
        assert!(!first_intent.texture_created);
        assert!(!first_intent.render_submitted);
        assert!(!first_intent.frame_callback_done_sent);
        assert!(!first_intent.input_support);
        assert!(!second_intent.buffer_attach_observed);
        assert!(!second_intent.damage_observed);
        assert_eq!(second_intent.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：render-dirty intent runtime queue preserves FIFO without rendering.
    #[test]
    fn nested_runtime_loop_drains_render_dirty_intent_runtime_queue_fifo_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-render-dirty-intent-queue");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.render_dirty_intents_enqueued, 2);
        assert_eq!(report.surface_commit.render_dirty_intents_drained, 2);
        assert_eq!(
            runtime_loop.coordinator.render_dirty_intent_pending_count(),
            0
        );
        let first_drained = &report.surface_commit.render_dirty_queue_drained_intents[0];
        let second_drained = &report.surface_commit.render_dirty_queue_drained_intents[1];
        assert_eq!(
            first_drained.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_drained.commit_sequence, first_commit.commit_sequence);
        assert_eq!(
            second_drained.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_drained.buffer_attach_observed);
        assert!(first_drained.buffer_removed);
        assert!(first_drained.damage_observed);
        assert_eq!(first_drained.buffer_damage_rects, 1);
        assert!(first_drained.frame_callback_observed);
        assert_eq!(first_drained.frame_callback_count, 1);
        assert!(!first_drained.render_submitted);
        assert!(!first_drained.buffer_imported);
        assert!(!first_drained.texture_created);
        assert!(!first_drained.frame_callback_done_sent);
        assert!(!first_drained.input_support);
        assert!(!second_drained.buffer_attach_observed);
        assert!(!second_drained.damage_observed);
        assert_eq!(second_drained.frame_callback_count, 0);
        assert!(!report.surface_commit.render_dirty_queue_render_submitted);
        assert!(!report.surface_commit.render_dirty_queue_buffer_imported);
        assert!(!report.surface_commit.render_dirty_queue_texture_created);
        assert!(
            !report
                .surface_commit
                .render_dirty_queue_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_dirty_queue_input_support);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：runtime reports renderer-admission work intents without rendering.
    #[test]
    fn nested_runtime_loop_reports_renderer_admission_work_intents_fifo_without_render() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-renderer-admission-work-intent");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.render_dirty_intents_drained, 2);
        assert_eq!(report.surface_commit.renderer_work_intents_created, 2);
        assert_eq!(report.surface_commit.renderer_work_intents.len(), 2);
        let first_work = &report.surface_commit.renderer_work_intents[0];
        let second_work = &report.surface_commit.renderer_work_intents[1];
        assert_eq!(
            first_work.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_work.commit_sequence, first_commit.commit_sequence);
        assert_eq!(second_work.commit_sequence, second_commit.commit_sequence);
        assert!(first_work.buffer_attach_observed);
        assert!(first_work.buffer_removed);
        assert!(first_work.damage_observed);
        assert_eq!(first_work.buffer_damage_rects, 1);
        assert!(first_work.frame_callback_observed);
        assert_eq!(first_work.frame_callback_count, 1);
        assert!(!first_work.render_submitted);
        assert!(!first_work.buffer_imported);
        assert!(!first_work.texture_created);
        assert!(!first_work.damage_submitted);
        assert!(!first_work.frame_callback_done_sent);
        assert!(!first_work.input_support);
        assert!(!first_work.core_mutation_invoked);
        assert!(!second_work.buffer_attach_observed);
        assert!(!second_work.damage_observed);
        assert_eq!(second_work.frame_callback_count, 0);
        assert!(!report.surface_commit.renderer_admission_render_submitted);
        assert!(!report.surface_commit.renderer_admission_buffer_imported);
        assert!(!report.surface_commit.renderer_admission_texture_created);
        assert!(!report.surface_commit.renderer_admission_damage_submitted);
        assert!(
            !report
                .surface_commit
                .renderer_admission_frame_callback_done_sent
        );
        assert!(!report.surface_commit.renderer_admission_input_support);
        assert!(
            !report
                .surface_commit
                .renderer_admission_core_mutation_invoked
        );
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：renderer owner boundary consumes work intents FIFO as blocked readiness.
    #[test]
    fn nested_runtime_loop_consumes_renderer_owner_work_intents_fifo_as_blocked_boundary() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-renderer-owner-boundary");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let (first_commit, second_commit) = {
            let display = runtime_loop
                .coordinator
                .display_mut_for_controlled_toplevel_registration();
            display
                .initialize_wl_compositor_global()
                .expect("测试 wl_compositor global 必须初始化");
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let mut state = State::new();
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 3,
                pump_timeout: Duration::ZERO,
                stop_when_idle: true,
                continue_after_error: false,
            },
        );

        assert!(report.is_successful());
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.renderer_work_intents_created, 2);
        let consumed_count = report.surface_commit.renderer_owner_work_intents_consumed;
        assert_eq!(consumed_count, 2);
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_consumed_work_intents
                .len(),
            2
        );
        let first_consumed = &report.surface_commit.renderer_owner_consumed_work_intents[0];
        let second_consumed = &report.surface_commit.renderer_owner_consumed_work_intents[1];
        assert_eq!(
            first_consumed.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_consumed.commit_sequence, first_commit.commit_sequence);
        let second_sequence = second_consumed.commit_sequence;
        assert_eq!(second_sequence, second_commit.commit_sequence);
        assert!(first_consumed.buffer_attach_observed);
        assert!(first_consumed.buffer_removed);
        assert!(first_consumed.damage_observed);
        assert_eq!(first_consumed.buffer_damage_rects, 1);
        assert!(first_consumed.frame_callback_observed);
        assert_eq!(first_consumed.frame_callback_count, 1);
        assert!(!second_consumed.buffer_attach_observed);
        assert!(!second_consumed.damage_observed);
        assert_eq!(second_consumed.frame_callback_count, 0);
        assert!(report.surface_commit.renderer_owner_missing_renderer_owner);
        assert!(report.surface_commit.renderer_owner_missing_buffer_importer);
        assert!(report.surface_commit.renderer_owner_missing_texture_support);
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_shell_work_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_shell_observed_work_intents
                .len(),
            2
        );
        assert!(report.surface_commit.renderer_owner_shell_available);
        assert!(
            !report
                .surface_commit
                .renderer_owner_shell_missing_renderer_owner
        );
        assert!(
            report
                .surface_commit
                .renderer_owner_shell_missing_buffer_importer
        );
        assert!(
            report
                .surface_commit
                .renderer_owner_shell_missing_texture_support
        );
        let first_shell = &report
            .surface_commit
            .renderer_owner_shell_observed_work_intents[0];
        let second_shell = &report
            .surface_commit
            .renderer_owner_shell_observed_work_intents[1];
        assert_eq!(first_shell.commit_sequence, first_commit.commit_sequence);
        assert_eq!(second_shell.commit_sequence, second_commit.commit_sequence);
        assert!(first_shell.buffer_attach_observed);
        assert!(first_shell.damage_observed);
        assert_eq!(first_shell.frame_callback_count, 1);
        assert!(!second_shell.buffer_attach_observed);
        assert!(!second_shell.damage_observed);
        assert_eq!(second_shell.frame_callback_count, 0);
        assert!(!report.surface_commit.renderer_owner_shell_buffer_imported);
        assert!(!report.surface_commit.renderer_owner_shell_texture_created);
        assert!(!report.surface_commit.renderer_owner_shell_renderer_called);
        assert!(!report.surface_commit.renderer_owner_shell_damage_submitted);
        assert!(
            !report
                .surface_commit
                .renderer_owner_shell_frame_callback_done_sent
        );
        assert!(!report.surface_commit.renderer_owner_shell_input_support);
        assert!(
            !report
                .surface_commit
                .renderer_owner_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_importer_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_importer_shell_work_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_importer_shell_observed_work_intents
                .len(),
            2
        );
        assert!(report.surface_commit.buffer_importer_shell_available);
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_missing_renderer_owner_shell
        );
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_missing_buffer_importer
        );
        assert!(
            report
                .surface_commit
                .buffer_importer_shell_missing_texture_support
        );
        let first_importer = &report
            .surface_commit
            .buffer_importer_shell_observed_work_intents[0];
        let second_importer = &report
            .surface_commit
            .buffer_importer_shell_observed_work_intents[1];
        assert_eq!(first_importer.commit_sequence, first_commit.commit_sequence);
        assert_eq!(
            second_importer.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_importer.buffer_attach_observed);
        assert!(first_importer.damage_observed);
        assert_eq!(first_importer.frame_callback_count, 1);
        assert!(!second_importer.buffer_attach_observed);
        assert!(!second_importer.damage_observed);
        assert_eq!(second_importer.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_importer_shell_buffer_imported);
        assert!(!report.surface_commit.buffer_importer_shell_texture_created);
        assert!(!report.surface_commit.buffer_importer_shell_renderer_called);
        assert!(!report.surface_commit.buffer_importer_shell_damage_submitted);
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_importer_shell_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .texture_support_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .texture_support_shell_work_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .texture_support_shell_observed_work_intents
                .len(),
            2
        );
        assert!(report.surface_commit.texture_support_shell_available);
        assert!(
            !report
                .surface_commit
                .texture_support_shell_missing_buffer_importer_shell
        );
        assert!(
            !report
                .surface_commit
                .texture_support_shell_missing_texture_support
        );
        let first_texture = &report
            .surface_commit
            .texture_support_shell_observed_work_intents[0];
        let second_texture = &report
            .surface_commit
            .texture_support_shell_observed_work_intents[1];
        assert_eq!(first_texture.commit_sequence, first_commit.commit_sequence);
        assert_eq!(
            second_texture.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_texture.buffer_attach_observed);
        assert!(first_texture.damage_observed);
        assert_eq!(first_texture.frame_callback_count, 1);
        assert!(!second_texture.buffer_attach_observed);
        assert!(!second_texture.damage_observed);
        assert_eq!(second_texture.frame_callback_count, 0);
        assert!(!report.surface_commit.texture_support_shell_buffer_imported);
        assert!(!report.surface_commit.texture_support_shell_texture_created);
        assert!(!report.surface_commit.texture_support_shell_renderer_called);
        assert!(!report.surface_commit.texture_support_shell_damage_submitted);
        assert!(
            !report
                .surface_commit
                .texture_support_shell_frame_callback_done_sent
        );
        assert!(!report.surface_commit.texture_support_shell_input_support);
        assert!(
            !report
                .surface_commit
                .texture_support_shell_core_mutation_invoked
        );
        assert_eq!(
            report.surface_commit.render_operation_readiness_invocations,
            3
        );
        assert_eq!(report.surface_commit.render_operation_intents_created, 2);
        assert_eq!(report.surface_commit.render_operation_intents.len(), 2);
        let first_render_operation = &report.surface_commit.render_operation_intents[0];
        let second_render_operation = &report.surface_commit.render_operation_intents[1];
        assert_eq!(
            first_render_operation.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_operation.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_operation.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_operation.buffer_attach_observed);
        assert!(first_render_operation.damage_observed);
        assert_eq!(
            first_render_operation.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_operation.frame_callback_count, 1);
        assert!(!second_render_operation.buffer_attach_observed);
        assert!(!second_render_operation.damage_observed);
        assert_eq!(second_render_operation.damage_rect_count, 0);
        assert_eq!(second_render_operation.frame_callback_count, 0);
        assert!(!report.surface_commit.render_operation_buffer_imported);
        assert!(!report.surface_commit.render_operation_texture_created);
        assert!(!report.surface_commit.render_operation_renderer_called);
        assert!(!report.surface_commit.render_operation_damage_submitted);
        assert!(
            !report
                .surface_commit
                .render_operation_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_operation_input_support);
        assert!(!report.surface_commit.render_operation_core_mutation_invoked);
        assert_eq!(
            report
                .surface_commit
                .render_operation_queue_drain_invocations,
            3
        );
        assert_eq!(report.surface_commit.render_operation_intents_enqueued, 2);
        assert_eq!(report.surface_commit.render_operation_intents_drained, 2);
        assert_eq!(
            report
                .surface_commit
                .render_operation_queue_drained_intents
                .len(),
            2
        );
        assert_eq!(
            runtime_loop
                .coordinator
                .render_operation_intent_pending_count(),
            0
        );
        let first_render_operation_drained =
            &report.surface_commit.render_operation_queue_drained_intents[0];
        let second_render_operation_drained =
            &report.surface_commit.render_operation_queue_drained_intents[1];
        assert_eq!(
            first_render_operation_drained.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_operation_drained.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_operation_drained.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_operation_drained.buffer_attach_observed);
        assert!(first_render_operation_drained.damage_observed);
        assert_eq!(
            first_render_operation_drained.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_operation_drained.frame_callback_count, 1);
        assert!(!second_render_operation_drained.buffer_attach_observed);
        assert!(!second_render_operation_drained.damage_observed);
        assert_eq!(second_render_operation_drained.damage_rect_count, 0);
        assert_eq!(second_render_operation_drained.frame_callback_count, 0);
        assert!(!report.surface_commit.render_operation_queue_buffer_imported);
        assert!(!report.surface_commit.render_operation_queue_texture_created);
        assert!(!report.surface_commit.render_operation_queue_renderer_called);
        assert!(
            !report
                .surface_commit
                .render_operation_queue_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_operation_queue_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_operation_queue_input_support);
        assert!(
            !report
                .surface_commit
                .render_operation_queue_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_boundary_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_intents_consumed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_consumed_intents
                .len(),
            2
        );
        let first_render_execution = &report
            .surface_commit
            .render_execution_owner_consumed_intents[0];
        let second_render_execution = &report
            .surface_commit
            .render_execution_owner_consumed_intents[1];
        assert_eq!(
            first_render_execution.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_execution.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_execution.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_execution.buffer_attach_observed);
        assert!(first_render_execution.damage_observed);
        assert_eq!(
            first_render_execution.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_execution.frame_callback_count, 1);
        assert!(!second_render_execution.buffer_attach_observed);
        assert!(!second_render_execution.damage_observed);
        assert_eq!(second_render_execution.damage_rect_count, 0);
        assert_eq!(second_render_execution.frame_callback_count, 0);
        assert!(report.surface_commit.render_execution_owner_missing_owner);
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_frame_callback_done
        );
        assert!(!report.surface_commit.render_execution_owner_buffer_imported);
        assert!(!report.surface_commit.render_execution_owner_texture_created);
        assert!(!report.surface_commit.render_execution_owner_renderer_called);
        assert!(
            !report
                .surface_commit
                .render_execution_owner_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_execution_owner_input_support);
        assert!(
            !report
                .surface_commit
                .render_execution_owner_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_shell_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_shell_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.render_execution_owner_shell_available);
        let first_render_execution_shell = &report
            .surface_commit
            .render_execution_owner_shell_observed_intents[0];
        let second_render_execution_shell = &report
            .surface_commit
            .render_execution_owner_shell_observed_intents[1];
        assert_eq!(
            first_render_execution_shell.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_execution_shell.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_execution_shell.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_execution_shell.buffer_attach_observed);
        assert!(first_render_execution_shell.damage_observed);
        assert_eq!(
            first_render_execution_shell.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_execution_shell.frame_callback_count, 1);
        assert!(!second_render_execution_shell.buffer_attach_observed);
        assert!(!second_render_execution_shell.damage_observed);
        assert_eq!(second_render_execution_shell.damage_rect_count, 0);
        assert_eq!(second_render_execution_shell.frame_callback_count, 0);
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_texture_created
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_input_support
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_pipeline_skeleton_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_pipeline_skeleton_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_pipeline_skeleton_observed_intents
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_owner_available
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_missing_execution_owner_shell
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_frame_callback_done
        );
        let first_render_pipeline_skeleton = &report
            .surface_commit
            .render_pipeline_skeleton_observed_intents[0];
        let second_render_pipeline_skeleton = &report
            .surface_commit
            .render_pipeline_skeleton_observed_intents[1];
        assert_eq!(
            first_render_pipeline_skeleton.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_pipeline_skeleton.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_pipeline_skeleton.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_pipeline_skeleton.buffer_attach_observed);
        assert!(first_render_pipeline_skeleton.damage_observed);
        assert_eq!(
            first_render_pipeline_skeleton.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_pipeline_skeleton.frame_callback_count, 1);
        assert!(!second_render_pipeline_skeleton.buffer_attach_observed);
        assert!(!second_render_pipeline_skeleton.damage_observed);
        assert_eq!(second_render_pipeline_skeleton.damage_rect_count, 0);
        assert_eq!(second_render_pipeline_skeleton.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_texture_created
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_pipeline_skeleton_input_support);
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_backend_capability_report_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_backend_capability_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_backend_capability_observed_intents
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_owner_available
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_backend_registered
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_backend_kind
                .is_none()
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_missing_pipeline_skeleton
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_backend_registration
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_frame_callback_done
        );
        let first_render_backend_capability = &report
            .surface_commit
            .render_backend_capability_observed_intents[0];
        let second_render_backend_capability = &report
            .surface_commit
            .render_backend_capability_observed_intents[1];
        assert_eq!(
            first_render_backend_capability.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_backend_capability.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_backend_capability.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_backend_capability.buffer_attach_observed);
        assert!(first_render_backend_capability.damage_observed);
        assert_eq!(
            first_render_backend_capability.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_backend_capability.frame_callback_count, 1);
        assert!(!second_render_backend_capability.buffer_attach_observed);
        assert!(!second_render_backend_capability.damage_observed);
        assert_eq!(second_render_backend_capability.damage_rect_count, 0);
        assert_eq!(second_render_backend_capability.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .render_backend_capability_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_texture_created
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_input_support
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_registration_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_registration_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_registration_observed_intents
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_owner_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_backend_registered
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_descriptor_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_missing_backend_capability
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_frame_callback_done
        );
        let first_renderer_backend_registration = &report
            .surface_commit
            .renderer_backend_registration_observed_intents[0];
        let second_renderer_backend_registration = &report
            .surface_commit
            .renderer_backend_registration_observed_intents[1];
        assert_eq!(
            first_renderer_backend_registration.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_renderer_backend_registration.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_renderer_backend_registration.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_renderer_backend_registration.buffer_attach_observed);
        assert!(first_renderer_backend_registration.damage_observed);
        assert_eq!(
            first_renderer_backend_registration.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_renderer_backend_registration.frame_callback_count, 1);
        assert!(!second_renderer_backend_registration.buffer_attach_observed);
        assert!(!second_renderer_backend_registration.damage_observed);
        assert_eq!(second_renderer_backend_registration.damage_rect_count, 0);
        assert_eq!(second_renderer_backend_registration.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_shell_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_shell_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.renderer_backend_owner_shell_available);
        assert!(report.surface_commit.renderer_backend_owner_shell_bound);
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_descriptor_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_missing_descriptor
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_frame_callback_done
        );
        let first_renderer_backend_owner_shell = &report
            .surface_commit
            .renderer_backend_owner_shell_observed_intents[0];
        let second_renderer_backend_owner_shell = &report
            .surface_commit
            .renderer_backend_owner_shell_observed_intents[1];
        assert_eq!(
            first_renderer_backend_owner_shell.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_renderer_backend_owner_shell.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_renderer_backend_owner_shell.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_renderer_backend_owner_shell.buffer_attach_observed);
        assert!(first_renderer_backend_owner_shell.damage_observed);
        assert_eq!(
            first_renderer_backend_owner_shell.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_renderer_backend_owner_shell.frame_callback_count, 1);
        assert!(!second_renderer_backend_owner_shell.buffer_attach_observed);
        assert!(!second_renderer_backend_owner_shell.damage_observed);
        assert_eq!(second_renderer_backend_owner_shell.damage_rect_count, 0);
        assert_eq!(second_renderer_backend_owner_shell.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_resource_owner_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_resource_owner_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_resource_owner_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.buffer_importer_owner_available);
        assert!(report.surface_commit.buffer_importer_owner_bound);
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_missing_renderer_backend_owner_shell
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_missing_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_frame_callback_done
        );
        let first_buffer_import_resource_owner = &report
            .surface_commit
            .buffer_import_resource_owner_observed_intents[0];
        let second_buffer_import_resource_owner = &report
            .surface_commit
            .buffer_import_resource_owner_observed_intents[1];
        assert_eq!(
            first_buffer_import_resource_owner.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_resource_owner.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_resource_owner.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_resource_owner.buffer_attach_observed);
        assert!(first_buffer_import_resource_owner.damage_observed);
        assert_eq!(
            first_buffer_import_resource_owner.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_buffer_import_resource_owner.frame_callback_count, 1);
        assert!(!second_buffer_import_resource_owner.buffer_attach_observed);
        assert!(!second_buffer_import_resource_owner.damage_observed);
        assert_eq!(second_buffer_import_resource_owner.damage_rect_count, 0);
        assert_eq!(second_buffer_import_resource_owner.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_core_mutation_invoked
        );
        assert_eq!(report.surface_commit.buffer_import_planning_invocations, 3);
        assert_eq!(
            report
                .surface_commit
                .buffer_import_planning_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_planning_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.buffer_import_plan_available);
        assert!(report.surface_commit.buffer_import_plan_built);
        assert_eq!(report.surface_commit.buffer_import_candidates_observed, 1);
        assert_eq!(report.surface_commit.buffer_import_required_count, 0);
        assert!(
            report
                .surface_commit
                .buffer_import_planning_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_missing_buffer_importer_owner
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_missing_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_frame_callback_done
        );
        let first_buffer_import_plan = &report
            .surface_commit
            .buffer_import_planning_observed_intents[0];
        let second_buffer_import_plan = &report
            .surface_commit
            .buffer_import_planning_observed_intents[1];
        assert_eq!(
            first_buffer_import_plan.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_plan.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_plan.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_plan.buffer_attach_observed);
        assert!(!first_buffer_import_plan.buffer_present);
        assert!(first_buffer_import_plan.buffer_removed);
        assert!(first_buffer_import_plan.damage_observed);
        assert_eq!(
            first_buffer_import_plan.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_buffer_import_plan.frame_callback_count, 1);
        assert!(!second_buffer_import_plan.buffer_attach_observed);
        assert!(!second_buffer_import_plan.buffer_present);
        assert!(!second_buffer_import_plan.damage_observed);
        assert_eq!(second_buffer_import_plan.damage_rect_count, 0);
        assert_eq!(second_buffer_import_plan.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_import_planning_buffer_imported);
        assert!(!report.surface_commit.buffer_import_planning_texture_created);
        assert!(!report.surface_commit.buffer_import_planning_renderer_called);
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_import_planning_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_boundary_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_descriptors_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_observed_descriptors
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_descriptor_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_descriptor_registered
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_candidates_observed,
            1
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_importer_owner_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_renderer_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_registered_backend_kind
                .is_some()
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_planning_intent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_missing_plan
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_missing_importer_owner_evidence
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_missing_renderer_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_frame_callback_done
        );
        let first_buffer_import_descriptor = &report
            .surface_commit
            .buffer_import_implementation_observed_descriptors[0];
        let second_buffer_import_descriptor = &report
            .surface_commit
            .buffer_import_implementation_observed_descriptors[1];
        assert_eq!(
            first_buffer_import_descriptor.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_descriptor.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_descriptor.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_descriptor.buffer_attach_observed);
        assert!(!first_buffer_import_descriptor.buffer_present);
        assert!(first_buffer_import_descriptor.buffer_removed);
        assert!(first_buffer_import_descriptor.candidate_evidence_observed);
        assert!(!first_buffer_import_descriptor.actual_import_required);
        assert!(first_buffer_import_descriptor.renderer_backend_descriptor_evidence_available);
        assert!(first_buffer_import_descriptor.importer_owner_evidence_available);
        assert!(!second_buffer_import_descriptor.buffer_attach_observed);
        assert!(!second_buffer_import_descriptor.buffer_present);
        assert!(!second_buffer_import_descriptor.buffer_removed);
        assert!(!second_buffer_import_descriptor.candidate_evidence_observed);
        assert!(!second_buffer_import_descriptor.actual_import_required);
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_proof_boundary_invocations,
            3
        );
        assert_eq!(
            report.surface_commit.buffer_import_adapter_proofs_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_observed_proofs
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_proof_boundary_available
        );
        assert!(report.surface_commit.buffer_import_adapter_proof_registered);
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_candidates_observed,
            1
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_importer_owner_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_renderer_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_registered_backend_kind
                .is_some()
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_implementation_descriptor
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_missing_importer_owner_evidence
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_missing_renderer_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_frame_callback_done
        );
        let first_buffer_import_adapter_proof =
            &report.surface_commit.buffer_import_adapter_observed_proofs[0];
        let second_buffer_import_adapter_proof =
            &report.surface_commit.buffer_import_adapter_observed_proofs[1];
        assert_eq!(
            first_buffer_import_adapter_proof.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_adapter_proof.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_adapter_proof.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_adapter_proof.buffer_attach_observed);
        assert!(!first_buffer_import_adapter_proof.buffer_present);
        assert!(first_buffer_import_adapter_proof.buffer_removed);
        assert!(first_buffer_import_adapter_proof.candidate_evidence_observed);
        assert!(!first_buffer_import_adapter_proof.actual_import_required);
        assert!(first_buffer_import_adapter_proof.renderer_backend_descriptor_evidence_available);
        assert!(first_buffer_import_adapter_proof.importer_owner_evidence_available);
        assert!(first_buffer_import_adapter_proof.implementation_descriptor_registered);
        assert!(!second_buffer_import_adapter_proof.buffer_attach_observed);
        assert!(!second_buffer_import_adapter_proof.buffer_present);
        assert!(!second_buffer_import_adapter_proof.buffer_removed);
        assert!(!second_buffer_import_adapter_proof.candidate_evidence_observed);
        assert!(!second_buffer_import_adapter_proof.actual_import_required);
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_buffer_import_attempted
        );
        assert!(!report.surface_commit.buffer_import_adapter_buffer_imported);
        assert!(!report.surface_commit.buffer_import_adapter_texture_created);
        assert!(!report.surface_commit.buffer_import_adapter_renderer_called);
        assert!(!report.surface_commit.buffer_import_adapter_damage_submitted);
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_import_adapter_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_gate_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_adapter_proofs_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_observed_adapter_proofs
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_gate_available
        );
        assert_eq!(
            report.surface_commit.buffer_import_preconditions_met_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_future_preconditions_met_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_candidates_observed,
            1
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_importer_owner_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_renderer_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_registered_backend_kind
                .is_some()
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_adapter_proof
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_registered_adapter_proof
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_actual_import_requirement
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_frame_callback_done
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_observed_adapter_proofs[0],
            *first_buffer_import_adapter_proof
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_observed_adapter_proofs[1],
            *second_buffer_import_adapter_proof
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_dry_run_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_dry_run_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_guard_available
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_attempted_count,
            0
        );
        assert_eq!(report.surface_commit.buffer_import_execution_noop_count, 3);
        assert_eq!(
            report.surface_commit.buffer_import_execution_blocked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_actual_required_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_preconditions_met_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_import_preconditions
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_adapter_proof
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_import_execution_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_shell_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_shell_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_shell_available
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_real_implementation_available
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_admitted_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_blocked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_actual_required_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_execution_reports_observed,
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_execution_dry_run_blocked
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_records
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_available
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_recorded_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_admission_checked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_admitted_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_blocked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_owner_shell_blocked
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_admission
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_core_mutation_invoked
        );
        assert!(!report.surface_commit.renderer_owner_buffer_imported);
        assert!(!report.surface_commit.renderer_owner_texture_created);
        assert!(!report.surface_commit.renderer_owner_renderer_called);
        assert!(!report.surface_commit.renderer_owner_damage_submitted);
        assert!(
            !report
                .surface_commit
                .renderer_owner_frame_callback_done_sent
        );
        assert!(!report.surface_commit.renderer_owner_input_support);
        assert!(!report.surface_commit.renderer_owner_core_mutation_invoked);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }
}
