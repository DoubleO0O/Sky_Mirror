//! Smithay 集成层的 feature-gated 骨架。
//!
//! `smithay-probe` 只编译纯数据事件适配、driver、runtime 和场景测试，不依赖
//! Smithay crate。`smithay-linux` 在 Linux 上额外编译 Display、socket 和
//! bootstrap 等真实系统资源探针；旧 `smithay-backend` 名称是它的兼容别名。
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
/// Smithay client connection 事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod client_event;
/// Smithay client ID 分配器探针。
#[cfg(feature = "smithay-probe")]
pub mod client_id;
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
/// Smithay 输出尺寸变化事件适配探针。
#[cfg(feature = "smithay-probe")]
pub mod output_event;
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
#[cfg(feature = "smithay-probe")]
pub use client_event::{
    SmithayClientConnectionDescriptor, SmithayClientConnectionMode, SmithayClientConnectionProbe,
};
#[allow(unused_imports)]
#[cfg(feature = "smithay-probe")]
pub use client_id::{SmithayClientIdAllocatorMode, SmithayClientIdAllocatorProbe};
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
#[cfg(feature = "smithay-probe")]
pub use output_event::{
    SmithayOutputEventMode, SmithayOutputEventProbe, SmithayOutputResizeDescriptor,
};
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

use crate::core::{
    backend_driver::BackendDriverRunner, backend_event::BackendEvent,
    runtime_bridge::RuntimeEventResult, state::State,
};

/// Smithay 集成探针。
///
/// 该结构目前不持有真实 Smithay display、socket、seat 或 surface。它只为未来
/// Smithay backend 预留统一位置，并证明相关代码可以在 feature gate 下独立编译。
pub struct SmithayBackendProbe;

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
pub fn smithay_compile_probe() -> &'static str {
    "smithay-backend-probe"
}

#[cfg(test)]
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
