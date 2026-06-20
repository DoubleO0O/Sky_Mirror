//! Smithay 集成层的 feature-gated 骨架与跨平台纯数据边界。
//!
//! `client_session` 在 default build 中编译，用于保持 adapter session identity
//! 与核心 client identity 分离。`smithay-probe` 只额外编译纯数据事件适配、
//! driver、runtime 和场景测试，不依赖 Smithay crate。`smithay-linux` 在 Linux
//! 上额外编译 Display、socket 和 bootstrap 等真实系统资源探针；旧
//! `smithay-backend` 名称是它的兼容别名。
//!
//! Feature invariant: `smithay-probe` 不得因本模块的声明或 re-export 引入 Smithay
//! 或平台图形栈。Linux 资源能被构造也不代表 compositor 已启动、client 已接收或
//! protocol global 已注册。任何真实回调都必须先转换为 `BackendEvent`，再通过
//! `BackendDriverRunner` 进入核心状态。

#[cfg(all(feature = "smithay-linux", not(target_os = "linux")))]
compile_error!("smithay-linux 只能在 Linux 上启用；macOS 请使用 smithay-probe。");

/// Linux 资源测试共用的环境检查和唯一名称生成器。
#[cfg(all(test, feature = "smithay-linux", target_os = "linux"))]
pub(crate) mod test_support {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(1);

    /// 确认 Linux 资源测试具备有效的 Wayland 运行时目录。
    ///
    /// 缺失该目录时测试必须明确失败，不能通过提前返回掩盖资源路径未执行。
    pub fn assert_runtime_dir() {
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");

        assert!(
            std::path::Path::new(&runtime_dir).is_dir(),
            "XDG_RUNTIME_DIR 必须指向已存在的目录"
        );
    }

    /// 为并行测试生成当前进程内唯一的 Wayland socket 名称。
    pub fn unique_socket_name(label: &str) -> String {
        let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);

        format!("wayland-sky-mirror-{label}-{}-{id}", std::process::id())
    }
}

/// Smithay 动作请求事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod action_event;
/// Smithay Display 与 Wayland socket 的组合 bootstrap 探针。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod bootstrap;
/// Linux-only 真实 client disconnect callback 前置条件报告。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod client_disconnect;
/// Smithay client connection 事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod client_event;
/// Smithay client ID 分配器探针。
#[cfg(feature = "smithay-probe")]
pub mod client_id;
/// Linux-only `ClientData` owner 与 `DisplayHandle::insert_client` 编译验证边界。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod client_insert;
/// Nested client session 的跨平台纯数据身份与映射边界。
pub mod client_session;
/// Smithay 诊断请求事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod diagnostic_event;
/// Smithay 后端驱动接口探针。
#[cfg(feature = "smithay-probe")]
pub mod driver;
/// Linux 专属 Smithay adapter 结构骨架；不代表真实 compositor 支持。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_adapter;
/// Smithay handler trait 边界的隔离类型形状与 blocker evidence 探针。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_handler_probe;
/// Linux Smithay 资源与纯数据 runtime 的组合探针。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_runtime;
/// Linux-only accept/dispatch/disconnect lifecycle 的单次 pump coordinator。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod nested_runtime_coordinator;
/// Linux-only nested lifecycle coordinator 的 bounded runtime loop。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod nested_runtime_loop;
/// Linux-only nested runtime 的 start/run/stop orchestration boundary。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod nested_runtime_orchestrator;
/// Linux-only nested socket probe 到核心 lifecycle bridge 的受控 flow proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod nested_socket_flow;
/// Linux-only nested socket connection 的纯数据 session event probe。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod nested_socket_probe;
/// Smithay 输出尺寸变化事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod output_event;
/// Linux-only 真实 socket callback 到 connected core lifecycle 的最小边界。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod real_accept_flow;
/// Linux-only disconnect callback queue 到既有 core close seam 的桥接边界。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod real_disconnect_flow;
/// Smithay 纯数据运行时探针。
#[cfg(feature = "smithay-probe")]
pub mod runtime;
/// 后端中立的运行时门面与结构化启动报告。
#[cfg(feature = "smithay-probe")]
pub mod runtime_facade;
/// Smithay runtime 场景回放探针。
#[cfg(feature = "smithay-probe")]
pub mod scenario;
/// Surface 接纳预检管线的稳定契约快照。
#[cfg(feature = "smithay-probe")]
pub mod surface_admission_contract;
/// Surface 生命周期到窗口接纳预检的纯数据集成管线。
#[cfg(feature = "smithay-probe")]
pub mod surface_admission_pipeline;
/// Smithay surface 创建事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod surface_event;
/// Smithay surface ID 分配器探针。
#[cfg(feature = "smithay-probe")]
pub mod surface_id;
/// Surface 生命周期纯数据预备层。
#[cfg(feature = "smithay-probe")]
pub mod surface_lifecycle;
/// Surface 生命周期事件轨迹与 mock adapter。
#[cfg(feature = "smithay-probe")]
pub mod surface_trace;
/// Surface 到窗口候选意图的纯数据规划层。
#[cfg(feature = "smithay-probe")]
pub mod surface_window_intent;
/// Smithay toplevel 映射事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod toplevel_event;
/// Wayland server Display 的最小构造探针。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod wayland_display;
/// Wayland listening socket 的最小构造探针。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod wayland_socket;
/// 窗口候选意图到核心接纳动作的纯数据预检层。
#[cfg(feature = "smithay-probe")]
pub mod window_admission_preview;

// 这些 re-export 构成 feature 模块的公共门面；二进制入口当前不直接消费全部类型。
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use action_event::{
    SmithayActionEventMode, SmithayActionEventProbe, SmithayActionRequestDescriptor,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use bootstrap::{SmithayBootstrapMode, SmithayBootstrapProbe};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use client_disconnect::{
    NestedClientDisconnectCallbackBlocker, NestedClientDisconnectCallbackReadinessReport,
    nested_client_disconnect_callback_readiness_report,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use client_event::{
    SmithayClientConnectionDescriptor, SmithayClientConnectionMode, SmithayClientConnectionProbe,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use client_id::{SmithayClientIdAllocatorMode, SmithayClientIdAllocatorProbe};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use client_insert::{
    NestedClientCallbackEventQueue, NestedClientDataOwner, NestedClientInsertCompileBoundary,
    NestedClientInsertCompileProofReport, nested_client_insert_compile_proof_report,
    nested_session_for_inserted_client,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use diagnostic_event::{
    SmithayDiagnosticEventMode, SmithayDiagnosticEventProbe, SmithayDiagnosticRequestDescriptor,
    SmithayDiagnosticRequestKind,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use driver::{SmithayBackendDriverMode, SmithayBackendDriverProbe};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_adapter::{
    SmithayLinuxAdapterActivationAttemptLedgerReport,
    SmithayLinuxAdapterActivationAttemptObservation, SmithayLinuxAdapterActivationAttemptOutcome,
    SmithayLinuxAdapterActivationBlocker, SmithayLinuxAdapterActivationDecision,
    SmithayLinuxAdapterActivationGateReport, SmithayLinuxAdapterActivationReport,
    SmithayLinuxAdapterActivationTarget, SmithayLinuxAdapterCapabilities,
    SmithayLinuxAdapterClientSessionId, SmithayLinuxAdapterClientSessionLedgerReport,
    SmithayLinuxAdapterClientSessionObservation, SmithayLinuxAdapterClientSessionOutcome,
    SmithayLinuxAdapterClientSessionState, SmithayLinuxAdapterClientUnsupportedReason,
    SmithayLinuxAdapterDiagnostic, SmithayLinuxAdapterError,
    SmithayLinuxAdapterGlobalHandlerBlocker, SmithayLinuxAdapterGlobalHandlerBoundaryReport,
    SmithayLinuxAdapterGlobalHandlerKind, SmithayLinuxAdapterGlobalHandlerReadiness,
    SmithayLinuxAdapterGlobalHandlerReadinessReport, SmithayLinuxAdapterGlobalKind,
    SmithayLinuxAdapterGlobalPlan, SmithayLinuxAdapterGlobalPlanReport,
    SmithayLinuxAdapterGlobalRegistrationOperation, SmithayLinuxAdapterGlobalRegistrationReport,
    SmithayLinuxAdapterGlobalRegistrationState, SmithayLinuxAdapterLifecycle,
    SmithayLinuxAdapterOperation, SmithayLinuxAdapterProtocolRequestKind,
    SmithayLinuxAdapterProtocolRequestLedgerReport, SmithayLinuxAdapterProtocolRequestObservation,
    SmithayLinuxAdapterProtocolRequestOutcome, SmithayLinuxAdapterPumpOperation,
    SmithayLinuxAdapterPumpResult, SmithayLinuxAdapterPumpState, SmithayLinuxAdapterPumpStats,
    SmithayLinuxAdapterRealGlobalRegistrationBlocker,
    SmithayLinuxAdapterRealGlobalRegistrationMode, SmithayLinuxAdapterRealGlobalRegistrationReport,
    SmithayLinuxAdapterSkeleton, SmithayLinuxAdapterSnapshot,
    SmithayLinuxAdapterUnsupportedRequestReason,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_handler_probe::{
    SmithayLinuxBindClientIdentityBlocker, SmithayLinuxBindClientIdentityReport,
    SmithayLinuxBindClientIdentitySource, SmithayLinuxBindClientIdentityState,
    SmithayLinuxBindClientSyntheticId, SmithayLinuxBindGlobalDataBlocker,
    SmithayLinuxBindGlobalDataReport, SmithayLinuxBindGlobalDataSource,
    SmithayLinuxBindGlobalDataState, SmithayLinuxBindGlobalDataSyntheticId,
    SmithayLinuxBindGlobalResourceIdentityBlocker, SmithayLinuxBindGlobalResourceIdentityReport,
    SmithayLinuxBindGlobalResourceIdentitySource, SmithayLinuxBindGlobalResourceIdentityState,
    SmithayLinuxBindGlobalResourceSyntheticId, SmithayLinuxBindHandlerStateBlocker,
    SmithayLinuxBindHandlerStateReport, SmithayLinuxBindHandlerStateSource,
    SmithayLinuxBindHandlerStateState, SmithayLinuxBindHandlerStateSyntheticId,
    SmithayLinuxDispatchRequestBoundaryBlocker, SmithayLinuxDispatchRequestBoundaryDecision,
    SmithayLinuxDispatchRequestBoundaryFamilyReport,
    SmithayLinuxDispatchRequestBoundaryPrecondition,
    SmithayLinuxDispatchRequestBoundaryPreconditionItem,
    SmithayLinuxDispatchRequestBoundaryPreconditionState,
    SmithayLinuxDispatchRequestBoundaryReport, SmithayLinuxDispatchRequestBoundaryScope,
    SmithayLinuxDispatchRequestFamily, SmithayLinuxDisplayHandleAccessBlocker,
    SmithayLinuxDisplayHandleAccessPolicy, SmithayLinuxDisplayHandleAccessReport,
    SmithayLinuxDisplayHandleInternalAccessBlocker,
    SmithayLinuxDisplayHandleInternalAccessDecision,
    SmithayLinuxDisplayHandleInternalAccessGateReport,
    SmithayLinuxDisplayHandleInternalAccessPrecondition,
    SmithayLinuxDisplayHandleInternalAccessPreconditionItem,
    SmithayLinuxDisplayHandleInternalAccessPreconditionState,
    SmithayLinuxDisplayHandleInternalAccessTarget,
    SmithayLinuxDisplayHandleInternalOwnershipDecision,
    SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem,
    SmithayLinuxDisplayHandleInternalOwnershipEvidenceReport,
    SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource,
    SmithayLinuxDisplayHandleInternalOwnershipEvidenceState,
    SmithayLinuxDisplayHandleInternalOwnershipLimitation,
    SmithayLinuxDisplayHandlePublicApiEvidenceItem,
    SmithayLinuxDisplayHandlePublicApiEvidenceLimitation,
    SmithayLinuxDisplayHandlePublicApiEvidenceReport,
    SmithayLinuxDisplayHandlePublicApiEvidenceState,
    SmithayLinuxDisplayHandlePublicApiExposureDecision, SmithayLinuxDisplayHandlePublicApiSurface,
    SmithayLinuxDisplayHandleRedaction, SmithayLinuxGlobalDispatchBindBlocker,
    SmithayLinuxGlobalDispatchBindFinalBlocker, SmithayLinuxGlobalDispatchBindFinalSealReport,
    SmithayLinuxGlobalDispatchBindInput, SmithayLinuxGlobalDispatchBindReadiness,
    SmithayLinuxGlobalDispatchBindSealedInputItem, SmithayLinuxGlobalDispatchBindSealedInputState,
    SmithayLinuxGlobalDispatchBindShapeItem, SmithayLinuxGlobalDispatchBindShapeReport,
    SmithayLinuxGlobalDispatchTraitBoundaryBlocker,
    SmithayLinuxGlobalDispatchTraitBoundaryDecision,
    SmithayLinuxGlobalDispatchTraitBoundaryPrecondition,
    SmithayLinuxGlobalDispatchTraitBoundaryPreconditionItem,
    SmithayLinuxGlobalDispatchTraitBoundaryPreconditionState,
    SmithayLinuxGlobalDispatchTraitBoundaryReport, SmithayLinuxGlobalDispatchTraitBoundaryScope,
    SmithayLinuxGlobalRegistrationPromotionBlocker,
    SmithayLinuxGlobalRegistrationPromotionDecision,
    SmithayLinuxGlobalRegistrationPromotionPrecondition,
    SmithayLinuxGlobalRegistrationPromotionPreconditionItem,
    SmithayLinuxGlobalRegistrationPromotionPreconditionState,
    SmithayLinuxGlobalRegistrationPromotionReport, SmithayLinuxGlobalRegistrationPromotionTarget,
    SmithayLinuxGlobalRegistrationPromotionTargetReport, SmithayLinuxHandlerProbeBlocker,
    SmithayLinuxHandlerProbeKind, SmithayLinuxHandlerProbeReport,
    SmithayLinuxHandlerReductionCandidate, SmithayLinuxHandlerReductionCandidateReport,
    SmithayLinuxHandlerReductionDecision, SmithayLinuxHandlerReductionPlanReport,
    SmithayLinuxHandlerReductionRisk, SmithayLinuxHandlerRequirement,
    SmithayLinuxHandlerRequirementEvidence, SmithayLinuxHandlerRequirementMatrixItem,
    SmithayLinuxHandlerRequirementMatrixReport, SmithayLinuxHandlerRequirementState,
    SmithayLinuxInertHandlerProbe, smithay_linux_bind_client_identity_report,
    smithay_linux_bind_global_data_report, smithay_linux_bind_global_resource_identity_report,
    smithay_linux_bind_handler_state_report, smithay_linux_dispatch_request_boundary_report,
    smithay_linux_display_handle_access_report,
    smithay_linux_display_handle_internal_access_gate_report,
    smithay_linux_display_handle_internal_ownership_evidence_report,
    smithay_linux_display_handle_public_api_evidence_report,
    smithay_linux_global_dispatch_bind_final_seal_report,
    smithay_linux_global_dispatch_bind_shape_report,
    smithay_linux_global_dispatch_trait_boundary_report,
    smithay_linux_global_registration_promotion_report, smithay_linux_handler_probe_report,
    smithay_linux_handler_reduction_plan_report, smithay_linux_handler_requirement_matrix_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_runtime::SmithayLinuxRuntimeProbe;
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_runtime_coordinator::{
    NestedRuntimeCoordinator, NestedRuntimeCoordinatorBlocker,
    NestedRuntimeCoordinatorReadinessReport, NestedRuntimePumpError, NestedRuntimePumpErrorKind,
    NestedRuntimePumpReport, nested_runtime_coordinator_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_runtime_loop::{
    NestedRuntimeLoop, NestedRuntimeLoopBlocker, NestedRuntimeLoopConfig, NestedRuntimeLoopError,
    NestedRuntimeLoopExitReason, NestedRuntimeLoopReadinessReport, NestedRuntimeLoopReport,
    NestedRuntimeLoopStopHandle, NestedRuntimeWakeupReport, nested_runtime_loop_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_runtime_orchestrator::{
    NestedRuntimeLifecycleReport, NestedRuntimeLifecycleState, NestedRuntimeOrchestrator,
    NestedRuntimeOrchestratorBlocker, NestedRuntimeOrchestratorConfig,
    NestedRuntimeOrchestratorError, NestedRuntimeOrchestratorOperation,
    NestedRuntimeOrchestratorReadinessReport, NestedRuntimeStartReport, NestedRuntimeStopReport,
    nested_runtime_orchestrator_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_socket_flow::{NestedSocketProbeBridgeFlow, NestedSocketProbeBridgeFlowReport};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_socket_probe::{
    NestedSocketAcceptProbe, NestedSocketAcceptProbeBlocker, NestedSocketAcceptProbeReport,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use output_event::{
    SmithayOutputEventMode, SmithayOutputEventProbe, SmithayOutputResizeDescriptor,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use real_accept_flow::{
    NestedAcceptedClientAttempt, NestedAcceptedClientFailureReason, NestedAcceptedClientMapping,
    NestedRealAcceptConnectedBridgeBlocker, NestedRealAcceptConnectedBridgeReadinessReport,
    NestedRealAcceptFlow, NestedRealAcceptPumpReport,
    nested_real_accept_connected_bridge_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use real_disconnect_flow::NestedRealDisconnectCallbackReport;
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use runtime::{SmithayRuntimeMode, SmithayRuntimeProbe};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use runtime_facade::{
    BackendBootstrapMode, BackendRuntimeCapabilities, BackendRuntimeDiagnostic,
    BackendRuntimeReport,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use scenario::{
    SmithayRuntimeScenario, SmithayRuntimeScenarioMode, SmithayRuntimeScenarioReport,
    SmithayRuntimeScenarioStepReport,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_admission_contract::{
    BackendSurfaceAdmissionContractScenario, BackendSurfaceAdmissionContractSnapshot,
    SurfaceAdmissionContractRunner,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_admission_pipeline::{
    BackendSurfaceAdmissionPipelineReport, BackendSurfaceAdmissionPipelineStatus,
    BackendSurfaceAdmissionPipelineSummary, SurfaceAdmissionPipelineRunner,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_event::{
    SmithaySurfaceCreationDescriptor, SmithaySurfaceEventMode, SmithaySurfaceEventProbe,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_id::{SmithaySurfaceIdAllocatorMode, SmithaySurfaceIdAllocatorProbe};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_lifecycle::{
    BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleEvent,
    BackendSurfaceLifecycleState, BackendSurfaceRecord, BackendSurfaceRegistry, BackendSurfaceSize,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_trace::{
    BackendSurfaceMockAdapter, BackendSurfaceTrace, BackendSurfaceTraceReport,
    BackendSurfaceTraceRunner, BackendSurfaceTraceScenario,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use surface_window_intent::{
    BackendWindowCandidateId, BackendWindowCandidateIntent, BackendWindowCandidateIntentReport,
    SurfaceWindowIntentPlanner,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use toplevel_event::{
    SmithayToplevelEventMode, SmithayToplevelEventProbe, SmithayToplevelMapDescriptor,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use wayland_display::{SmithayWaylandDisplayProbe, SmithayWaylandState};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use wayland_socket::{SmithayWaylandSocketProbe, SmithayWaylandSocketProbeMode};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use window_admission_preview::{
    BackendWindowAdmissionPreviewAction, BackendWindowAdmissionPreviewReport,
    BackendWindowAdmissionPreviewWarning, WindowAdmissionPreviewPlanner,
};

#[cfg(feature = "smithay-probe")]
use crate::core::{
    backend_driver::BackendDriverRunner, backend_event::BackendEvent,
    runtime_bridge::RuntimeEventResult, state::State,
};

/// Smithay 集成探针。
///
/// 该结构目前不持有真实 Smithay display、socket、seat 或 surface。它只为未来
/// Smithay backend 预留统一位置，并证明相关代码可以在 feature gate 下独立编译。
#[cfg(feature = "smithay-probe")]
pub struct SmithayBackendProbe;

#[cfg(feature = "smithay-probe")]
impl SmithayBackendProbe {
    /// 创建 Smithay 集成探针。
    ///
    /// 当前不会初始化 Smithay，也不会读取或修改 `State`。
    pub fn new() -> Self {
        Self
    }

    /// 将一条已经构造好的后端事件提交给核心运行时桥。
    ///
    /// 未来真实 Smithay 回调应该先转换为 `BackendEvent`，再通过单事件 driver
    /// 和 `BackendDriverRunner` 进入核心，不能直接修改 workspace、窗口注册表
    /// 或 surface 注册表。
    pub fn handle_event(&mut self, state: &mut State, event: BackendEvent) -> RuntimeEventResult {
        let mut driver = SmithayBackendDriverProbe::with_events([event]);
        BackendDriverRunner::run_once(state, &mut driver)
            .runtime_result
            .expect("单事件 Smithay 探针必须产生运行时结果")
    }

    /// 返回 Smithay backend 当前是否只是探针模式。
    ///
    /// 该方法用于测试和调试，明确当前阶段尚未启动真实 compositor。
    pub fn is_probe_only(&self) -> bool {
        true
    }
}

/// 编译期确认 Smithay 纯数据探针已经启用。
///
/// 启用 `smithay-probe` 时，该函数可用但不会拉入 Smithay crate，也不会构造
/// 真实协议或渲染状态。
#[cfg(feature = "smithay-probe")]
pub fn smithay_compile_probe() -> &'static str {
    "smithay-backend-probe"
}

#[cfg(all(test, feature = "smithay-probe"))]
mod tests {
    use super::{SmithayBackendProbe, smithay_compile_probe};
    use crate::core::{backend_event::BackendEvent, state::State};

    /// 验证 Smithay 探针默认只处于编译探针模式。
    #[test]
    fn smithay_backend_probe_is_probe_only() {
        let probe = SmithayBackendProbe::new();

        // 探针不得暗示已经启动真实 Wayland compositor。
        assert!(probe.is_probe_only());

        // feature 构建必须能够调用轻量编译探针。
        assert_eq!(smithay_compile_probe(), "smithay-backend-probe");
    }

    /// 验证 Smithay 探针通过后端事件和运行时桥进入核心状态。
    #[test]
    fn smithay_backend_probe_handles_backend_event_through_runtime_bridge() {
        let mut state = State::new();
        let mut probe = SmithayBackendProbe::new();

        let result = probe.handle_event(
            &mut state,
            BackendEvent::OutputResized {
                width: 1024,
                height: 768,
            },
        );

        // 合法后端事件通过统一桥接路径后必须保持状态有效。
        assert!(result.validation.is_valid());

        let output = state.compositor.current_output_size();

        // 输出变化证明探针复用了 BackendEvent 与 BackendDriverRunner 链路。
        assert_eq!(output.width, 1024);
        assert_eq!(output.height, 768);
    }
}

#[cfg(test)]
mod nested_socket_probe_gate_tests {
    /// 验证 runtime orchestrator 的声明与公共导出都保持 Linux-only。
    #[test]
    fn runtime_orchestrator_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod nested_runtime_orchestrator;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use nested_runtime_orchestrator::{"))
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 orchestrator 只编排现有 loop，并保守描述完整 runtime 能力。
    #[test]
    fn runtime_orchestrator_source_preserves_loop_seam() {
        let source = include_str!("nested_runtime_orchestrator.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct NestedRuntimeOrchestrator",
            "pub struct NestedRuntimeOrchestratorConfig",
            "pub struct NestedRuntimeLifecycleReport",
            "pub struct NestedRuntimeStartReport",
            "pub struct NestedRuntimeStopReport",
            "pub enum NestedRuntimeLifecycleState",
            "pub enum NestedRuntimeOrchestratorError",
            "pub fn start",
            "pub fn run",
            "pub fn stop",
            "pub fn stop_handle",
            "NestedRuntimeLoop::with_socket_name",
            ".run_for_iterations(state, self.config.loop_config)",
            ".request_stop_and_wakeup()",
            "validation_is_clean",
        ] {
            assert!(
                production_code.contains(required),
                "runtime orchestrator 缺少必要 seam token: {required}"
            );
        }

        for conservative in [
            "long_running_loop_available: false",
            "accepts_clients: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(conservative),
                "B 路线缺少保守 orchestration capability: {conservative}"
            );
        }

        for forbidden in [
            ["State", "::handle_command"].concat(),
            [".", "clients"].concat(),
            [".", "surfaces"].concat(),
            [".", "registry"].concat(),
            ["Backend", "Event::RuntimeStarted"].concat(),
            ["Core", "Command::StartRuntime"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "runtime orchestrator 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// Linux lifecycle proof 通过后，只允许上调已被证明的 orchestration capability。
    #[test]
    fn runtime_orchestrator_proof_capabilities_are_precise() {
        let source = include_str!("nested_runtime_orchestrator.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        for proven in [
            "runtime_orchestrator_available: true",
            "start_run_stop_available: true",
            "external_stop_supported: true",
            "clean_shutdown_supported: true",
        ] {
            assert!(
                production.contains(proven),
                "C 路线缺少 Linux proof 支持的 capability: {proven}"
            );
        }

        for still_unproven in [
            "long_running_loop_available: false",
            "accepts_clients: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production.contains(still_unproven),
                "C 路线不得上调未证明 capability: {still_unproven}"
            );
        }
    }

    /// 验证 nested runtime loop 的声明与公共导出都保持 Linux-only。
    #[test]
    fn nested_runtime_loop_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod nested_runtime_loop;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use nested_runtime_loop::{"))
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 bounded loop 只重复编排 coordinator，并保守描述完整 runtime 能力。
    #[test]
    fn nested_runtime_loop_source_preserves_coordinator_seam() {
        let source = include_str!("nested_runtime_loop.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct NestedRuntimeLoop",
            "pub struct NestedRuntimeLoopConfig",
            "pub struct NestedRuntimeLoopReport",
            "pub struct NestedRuntimeLoopStopHandle",
            "pub enum NestedRuntimeLoopExitReason",
            "pub fn run_for_iterations",
            "pub fn stop_handle",
            ".coordinator.pump_once(state, config.pump_timeout)",
            "max_iterations",
            "stop_when_idle",
            "continue_after_error",
            "validation_is_clean",
        ] {
            assert!(
                production_code.contains(required),
                "nested runtime loop 缺少必要 seam token: {required}"
            );
        }

        for conservative in [
            "long_running_loop_available: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(conservative),
                "B 路线缺少保守 capability: {conservative}"
            );
        }

        for forbidden in [
            ["State", "::handle_command"].concat(),
            [".", "clients"].concat(),
            [".", "surfaces"].concat(),
            [".", "registry"].concat(),
            ["Backend", "Event::NestedClient"].concat(),
            ["Core", "Command::RunNestedLoop"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "nested runtime loop 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 Linux CI proof 只解锁 bounded loop 与 cooperative stop 的精确能力位。
    #[test]
    fn nested_runtime_loop_proof_capabilities_are_precise() {
        let source = include_str!("nested_runtime_loop.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        for proven in [
            "nested_runtime_loop_available: true",
            "bounded_loop_available: true",
            "stop_requested_supported: true",
        ] {
            assert!(
                production.contains(proven),
                "Linux bounded-loop proof 尚未反映精确 capability: {proven}"
            );
        }

        for conservative in [
            "long_running_loop_available: false",
            "accepts_clients: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production.contains(conservative),
                "C 路线越级上调了未证明 capability: {conservative}"
            );
        }
    }

    /// 验证 wakeup boundary 使用真实 calloop signal，不以轮询或 core mutation 伪造中断。
    #[test]
    fn nested_runtime_wakeup_source_preserves_interrupt_seam() {
        let source = include_str!("nested_runtime_loop.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct NestedRuntimeWakeupReport",
            "pub fn request_stop_and_wakeup",
            "pub fn is_waiting",
            "LoopSignal",
            ".wakeup()",
            "NestedRuntimeLoopExitReason::Interrupted",
            "wait_interrupted",
            "elapsed_before_exit",
            "exited_before_timeout",
        ] {
            assert!(
                production_code.contains(required),
                "runtime wakeup 缺少必要 interrupt seam token: {required}"
            );
        }

        for conservative in [
            "long_running_loop_available: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "surface_support: false",
            "render_support: false",
        ] {
            assert!(
                production_code.contains(conservative),
                "B 路线缺少保守 wakeup capability: {conservative}"
            );
        }

        for forbidden in [
            "thread::sleep".to_owned(),
            "spin_loop".to_owned(),
            ["State", "::handle_command"].concat(),
            [".", "clients"].concat(),
            [".", "surfaces"].concat(),
            [".", "registry"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "runtime wakeup 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 Linux CI proof 只解锁 wakeup/interruptible wait 的三个精确能力位。
    #[test]
    fn nested_runtime_wakeup_proof_capabilities_are_precise() {
        let source = include_str!("nested_runtime_loop.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        for proven in [
            "wakeup_supported: true",
            "interruptible_wait_available: true",
            "stop_can_interrupt_wait: true",
        ] {
            assert!(
                production.contains(proven),
                "Linux wakeup proof 尚未反映精确 capability: {proven}"
            );
        }

        for conservative in [
            "long_running_loop_available: false",
            "accepts_clients: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production.contains(conservative),
                "C 路线越级上调了未证明 capability: {conservative}"
            );
        }
    }

    /// 验证 nested runtime coordinator 的声明与公共导出都保持 Linux-only。
    #[test]
    fn nested_runtime_coordinator_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod nested_runtime_coordinator;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use nested_runtime_coordinator::{"))
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 single-pump coordinator 只编排已有 flow，并保守描述长期能力。
    #[test]
    fn nested_runtime_coordinator_source_preserves_lifecycle_seams() {
        let source = include_str!("nested_runtime_coordinator.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct NestedRuntimeCoordinator",
            "pub struct NestedRuntimePumpReport",
            "pub enum NestedRuntimePumpErrorKind",
            "pub fn pump_once",
            ".pump_once(state, timeout)",
            ".dispatch_wayland_clients_once()",
            ".bridge_pending_disconnects(state)",
            "validation_is_clean",
        ] {
            assert!(
                production_code.contains(required),
                "nested runtime coordinator 缺少必要 seam token: {required}"
            );
        }

        for conservative in [
            "accepts_clients: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "long_running_loop_available: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(conservative),
                "nested runtime coordinator 缺少保守 capability: {conservative}"
            );
        }

        for forbidden in [
            ["State", "::handle_command"].concat(),
            [".", "clients"].concat(),
            [".", "surfaces"].concat(),
            [".", "registry"].concat(),
            ["Backend", "Event::NestedClient"].concat(),
            ["Core", "Command::PumpClient"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "nested runtime coordinator 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 Linux CI proof 只解锁 single-pump coordinator 的五个精确能力位。
    #[test]
    fn nested_runtime_coordinator_proof_capabilities_are_precise() {
        let source = include_str!("nested_runtime_coordinator.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        for proven in [
            "nested_runtime_coordinator_available: true",
            "single_pump_available: true",
            "connected_bridge_invoked: true",
            "disconnect_bridge_invoked: true",
            "display_dispatch_invoked: true",
        ] {
            assert!(
                production.contains(proven),
                "Linux lifecycle proof 尚未反映精确 capability: {proven}"
            );
        }

        for conservative in [
            "accepts_clients: false",
            "runtime_accept_loop_started: false",
            "protocol_dispatch_started: false",
            "long_running_loop_available: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production.contains(conservative),
                "C 路线越级上调了未证明 capability: {conservative}"
            );
        }
    }

    /// 验证真实 disconnect callback bridge 的模块声明与公共导出都保持 Linux-only。
    #[test]
    fn real_disconnect_callback_bridge_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod real_disconnect_flow;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use real_disconnect_flow::"))
            .collect::<Vec<_>>();

        // ClientData/Wayland 类型不能进入 default 或 smithay-probe 编译面。
        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 disconnect flow 只消费 session event 并复用既有 core bridge。
    #[test]
    fn real_disconnect_callback_flow_source_preserves_core_seam() {
        let source = include_str!("real_disconnect_flow.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "NestedClientSessionEvent::Disconnected",
            "NestedClientSessionCoreBridge",
            ".handle_record(state, &record)",
            ".remove_session(session)",
            "nested_client_disconnect_callback_readiness_report()",
        ] {
            assert!(
                production_code.contains(required),
                "disconnect flow 缺少既有 seam token: {required}"
            );
        }

        // flow 只能借用 State 交给 bridge，不能直接写 registry 或发明同义事件/命令。
        for forbidden in [
            ["Backend", "Event::ClientClosed"].concat(),
            ["Core", "Command::DisconnectClient"].concat(),
            ["State", "::handle_command"].concat(),
            [".", "clients"].concat(),
            [".", "surfaces"].concat(),
            [".", "registry"].concat(),
            ["xdg", "_toplevel"].concat(),
            ["frame", "_callback"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "disconnect flow 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 Phase 51J-C runtime proof 必须真实关闭 peer 并 dispatch Display。
    #[test]
    fn runtime_disconnect_proof_source_requires_peer_close_and_display_dispatch() {
        let display_source = include_str!("wayland_display.rs");
        let accept_source = include_str!("real_accept_flow.rs");

        // Display probe 只暴露一次真实 backend dispatch，不允许用 kill_client 伪造断开。
        assert!(display_source.contains("pub(crate) fn dispatch_clients_once"));
        assert!(display_source.contains("self.display.dispatch_clients(&mut self.state)"));
        assert!(!display_source.contains("kill_client"));

        // Linux-only test 必须按 peer close -> dispatch -> queue -> bridge 顺序取证。
        for required in [
            "fn runtime_disconnect_callback_closes_core_client()",
            "drop(client_stream);",
            ".dispatch_wayland_clients_once()",
            ".bridge_pending_disconnects(&mut state)",
            "NestedClientSessionEventKind::Disconnected",
        ] {
            assert!(
                accept_source.contains(required),
                "runtime disconnect proof 缺少必要 token: {required}"
            );
        }
    }

    /// 验证真实 Linux proof 只提升 disconnect callback 两项能力，不越级宣称 accept。
    #[test]
    fn runtime_disconnect_proof_capabilities_are_precise() {
        let source = include_str!("client_disconnect.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        assert!(production.contains("blockers: Vec::new()"));
        assert!(production.contains("real_disconnect_callback_observed: true"));
        assert!(production.contains("core_close_invoked_from_real_callback: true"));
        assert!(production.contains("accepts_clients: false"));
        assert!(!production.contains("accepts_clients: true"));
    }

    /// 验证真实 accept connected flow 的模块声明与公共导出都保持 Linux-only。
    #[test]
    fn real_accept_connected_flow_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod real_accept_flow;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use real_accept_flow::{"))
            .collect::<Vec<_>>();

        // 真实 UnixStream/calloop API 不能进入 default 或 smithay-probe 编译面。
        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 real accept flow 使用同版本 calloop 和既有 core bridge，且能力保持保守。
    #[test]
    fn real_accept_connected_flow_source_preserves_runtime_boundary() {
        let source = include_str!("real_accept_flow.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        // callback 必须使用 Smithay re-export 的 calloop，并复用 insertion/core seam。
        for required in [
            "calloop::{self, EventLoop}",
            "ListeningSocketSource",
            ".insert_source(socket_source",
            "NestedClientInsertCompileBoundary",
            ".drain_connected()",
            "pub fn bridge_pending_disconnects",
            ".drain_disconnected()",
            "bridge_disconnected_events",
            "NestedClientSessionCoreBridge",
            ".handle_record(state, &record)",
            "MissingRealAcceptLoop",
            "MissingLinuxRuntimeProof",
        ] {
            assert!(
                production_code.contains(required),
                "real accept flow 缺少必要边界 token: {required}"
            );
        }

        // Linux runtime 证明尚未在当前分支完成，因此这些字段必须显式保持 false。
        for conservative in [
            "real_accept_loop_available: false",
            "accepted_stream_available: false",
            "display_handle_insert_client_runtime_available: false",
            "inserted_client_mapping_available: false",
            "connected_event_bridged_to_core: false",
            "validation_report_available: false",
            "accepts_clients: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "protocol_dispatch_started: false",
        ] {
            assert!(
                production_code.contains(conservative),
                "real accept flow 缺少保守 capability: {conservative}"
            );
        }

        // flow 可以借用 State 交给既有 bridge，但不能直接写 registry 或新增同义事件。
        for forbidden in [
            ["Backend", "Event::ClientAccepted"].concat(),
            ["Core", "Command::InsertClient"].concat(),
            ["State", "::handle_command"].concat(),
            [".", "clients"].concat(),
            [".", "surfaces"].concat(),
            ["Surface", "Registry"].concat(),
            ["Window", "Registry"].concat(),
            ["Global", "Dispatch"].concat(),
            ["impl ", "Dispatch"].concat(),
            ["xdg", "_toplevel"].concat(),
            ["frame", "_callback"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "real accept flow 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 insertion queue 提供只消费 Connected、保留 Disconnected 的窄接口。
    #[test]
    fn client_insert_queue_exposes_connected_only_drain() {
        let source = include_str!("client_insert.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        // Phase 51I-C 只能推进 connected；真实 disconnect 必须留给 Phase 51J-A。
        assert!(production.contains("pub fn drain_connected"));
        assert!(production.contains("NestedClientSessionEvent::Connected"));
        assert!(production.contains("deferred.push_back(event)"));
    }

    /// 验证 insertion queue 提供只消费 Disconnected、保留 Connected 的窄接口。
    #[test]
    fn client_insert_queue_exposes_disconnected_only_drain() {
        let source = include_str!("client_insert.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        // disconnect coordinator 只能取走 Disconnected，不能吞掉稍后仍需注册的 Connected。
        assert!(production.contains("pub fn drain_disconnected"));
        assert!(production.contains("NestedClientSessionEvent::Disconnected"));
        assert!(production.contains("connected.push_back(event)"));
    }

    /// 验证 inserted-client 编译边界的模块声明与公共导出都保持 Linux-only。
    #[test]
    fn client_insert_compile_boundary_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod client_insert;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use client_insert::{"))
            .collect::<Vec<_>>();

        // 模块和 re-export 必须各自带完整 Linux-only gate，不能污染 default/probe。
        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 B 路线引用真实 insertion API，但不伪造 accept 或越过 session seam。
    #[test]
    fn client_insert_compile_boundary_source_stays_conservative() {
        let source = include_str!("client_insert.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        // B 路线必须有真实锁定版本 API 调用点、owner 与双向 session event 形状。
        for required in [
            "impl ClientData for NestedClientDataOwner",
            "DisplayHandle",
            ".insert_client(stream, owner)",
            "NestedClientSessionEvent::Connected",
            "NestedClientSessionEvent::Disconnected",
            "display_handle_available: true",
            "insert_client_api_available: true",
            "client_data_owner_defined: true",
        ] {
            assert!(
                production_code.contains(required),
                "insert compile boundary 缺少必要 token: {required}"
            );
        }

        // 没有真实 socket callback / Linux runtime proof 时，所有能力位必须保持关闭。
        for conservative in [
            "real_accept_loop_available: false",
            "real_client_insert_observed: false",
            "inserted_client_mapping_available: false",
            "connected_event_bridged_to_core: false",
            "accepts_clients: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "protocol_dispatch_started: false",
        ] {
            assert!(
                production_code.contains(conservative),
                "insert compile boundary 缺少保守字段: {conservative}"
            );
        }

        // 本边界接收外部提供的 stream，不拥有 accept loop，也不能直接写 core。
        for forbidden in [
            [".", "accept", "("].concat(),
            ["Listening", "SocketSource"].concat(),
            ["crate", "::core"].concat(),
            ["Backend", "Event"].concat(),
            ["Core", "Command"].concat(),
            ["Surface", "Registry"].concat(),
            ["Window", "Registry"].concat(),
        ] {
            assert!(
                !production_code.contains(&forbidden),
                "insert compile boundary 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 disconnect callback readiness 的模块声明与公共导出都保持 Linux-only。
    ///
    /// readiness 虽然只描述纯数据 blocker，但它表达的是未来真实 `ClientData`
    /// callback 的 Linux runtime 前置条件，因此不能进入 default 或 `smithay-probe`。
    #[test]
    fn client_disconnect_callback_readiness_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod client_disconnect;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use client_disconnect::{"))
            .collect::<Vec<_>>();

        // 模块和 re-export 都必须各自带完整 Linux-only gate，不能只保护其中一侧。
        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 Phase 51J-C 报告只提升经 Linux runtime proof 的 callback 能力。
    #[test]
    fn client_disconnect_callback_runtime_proof_source_stays_precise() {
        let source = include_str!("client_disconnect.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        // 六类 blocker 必须逐项可见，避免用一个模糊的 NotReady 掩盖真实缺口。
        for required_blocker in [
            "MissingRealAcceptLoop",
            "MissingDisplayHandleInsertClient",
            "MissingClientDataOwner",
            "MissingRealClientSessionMapping",
            "MissingDisconnectCallbackSource",
            "MissingLinuxRuntimeProof",
        ] {
            assert!(
                production_code.contains(required_blocker),
                "disconnect readiness 缺少 blocker: {required_blocker}"
            );
        }

        // 项目级 accept、长期 dispatch 与越级 surface/render 能力继续保持 false。
        for conservative_field in [
            "accepts_clients: false",
            "surface_support: false",
            "shell_role_support: false",
            "render_support: false",
            "protocol_dispatch_started: false",
            "runtime_accept_loop_started: false",
        ] {
            assert!(
                production_code.contains(conservative_field),
                "disconnect readiness 缺少保守字段赋值: {conservative_field}"
            );
        }

        // baseline seam 与本轮真实 callback proof 必须逐项保留精确证据。
        for boundary_evidence in [
            "blockers: Vec::new()",
            "real_disconnect_callback_observed: true",
            "core_close_invoked_from_real_callback: true",
            "real_client_data_callback_owned: true",
            "real_inserted_client_mapping_available: true",
            "disconnect_event_bridge_available: true",
        ] {
            assert!(
                production_code.contains(boundary_evidence),
                "disconnect readiness 缺少已建立的边界证据: {boundary_evidence}"
            );
        }

        // readiness module 只能描述前置条件，不能越过 session event seam 直接写 core。
        for forbidden in [
            "crate::core",
            "BackendEvent",
            "CoreCommand",
            "State",
            "ClientRegistry",
            "SurfaceRegistry",
            "WindowRegistry",
            ".clients",
            ".surfaces",
            ".registry",
        ] {
            assert!(
                !production_code.contains(forbidden),
                "disconnect readiness 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 socket accept event probe 的声明与导出都保持 Linux-only feature gate。
    #[test]
    fn nested_socket_probe_is_not_visible_to_default_or_smithay_probe_builds() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod nested_socket_probe;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub use nested_socket_probe::{")
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 production probe 只引用 session event，不引用 core 或协议/runtime 入口。
    ///
    /// 该测试只读取 Linux-only 文件源码，不编译其类型，因此 default 与
    /// `smithay-probe` 仍不会看到 Linux API。
    #[test]
    fn nested_socket_probe_production_source_stays_on_session_event_seam() {
        let source = include_str!("nested_socket_probe.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let forbidden_tokens = [
            ["Wl", "Surface"].concat(),
            ["xdg", "_toplevel"].concat(),
            ["Xdg", "Toplevel"].concat(),
            ["insert", "_client"].concat(),
            ["DisplayHandle", "::", "insert_client"].concat(),
            ["delegate", "_"].concat(),
            ["impl ", "Dispatch"].concat(),
            ["impl ", "GlobalDispatch"].concat(),
            ["render", "er"].concat(),
            ["frame", "_callback"].concat(),
            ["lib", "input"].concat(),
            ["D", "rm"].concat(),
            ["G", "bm"].concat(),
            ["E", "gl"].concat(),
            ["Vul", "kan"].concat(),
            ["Backend", "Event"].concat(),
            ["Core", "Command"].concat(),
            ["NestedClientSessionCore", "Bridge"].concat(),
            [".", "accept", "("].concat(),
        ];

        for forbidden in forbidden_tokens {
            assert!(
                !production_code.contains(&forbidden),
                "nested socket probe 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// 验证 Phase 51G flow 的声明与导出都保持 Linux-only feature gate。
    #[test]
    fn nested_socket_flow_is_not_visible_to_default_or_smithay_probe_builds() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod nested_socket_flow;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("pub use nested_socket_flow::{"))
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 flow 只组合 probe 和 core bridge，不引入真实平台 runtime 入口。
    #[test]
    fn nested_socket_flow_production_source_stays_on_probe_bridge_seam() {
        let source = include_str!("nested_socket_flow.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let forbidden_tokens = [
            ["smithay", "::"].concat(),
            ["Listening", "SocketSource"].concat(),
            ["Display", "Handle"].concat(),
            ["insert", "_client"].concat(),
            [".", "accept", "("].concat(),
            ["wl", "_client"].concat(),
            ["Wl", "Surface"].concat(),
            ["wl", "_surface"].concat(),
            ["xdg", "_toplevel"].concat(),
            ["Xdg", "Toplevel"].concat(),
            ["delegate", "_"].concat(),
            ["impl ", "Dispatch"].concat(),
            ["impl ", "GlobalDispatch"].concat(),
            ["render", "er"].concat(),
            ["frame", "_callback"].concat(),
            ["key", "board"].concat(),
            ["point", "er"].concat(),
            ["lib", "input"].concat(),
            ["D", "RM"].concat(),
            ["G", "BM"].concat(),
            ["E", "GL"].concat(),
            ["Vul", "kan"].concat(),
            ["Backend", "Event"].concat(),
            ["Core", "Command"].concat(),
            ["handle", "_command"].concat(),
            ["register", "_client"].concat(),
            [".", "clients"].concat(),
            ["Surface", "Registry"].concat(),
        ];

        for forbidden in forbidden_tokens {
            assert!(
                !production_code.contains(&forbidden),
                "nested socket flow 生产代码包含禁止 token: {forbidden}"
            );
        }
    }
}
