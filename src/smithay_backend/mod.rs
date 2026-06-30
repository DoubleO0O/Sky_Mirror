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
/// Linux-only adapter-led ledger admission ownership proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_ledger_admission_owner;
/// Linux-only live callback observation 到 coordinator admission queue 的 owner seam。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_live_toplevel_admission_owner;
/// Linux-only controlled `new_toplevel` callback observation proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_new_toplevel_callback_observation;
/// Linux Smithay 资源与纯数据 runtime 的组合探针。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_runtime;
/// Linux-only live callback 到 pending ledger admission intent 的桥接 seam。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_toplevel_admission_bridge;
/// Linux-only pending admission intent 的 owner 消费 seam。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_toplevel_admission_consumer;
/// Linux-only controlled pending admission pump seam。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_toplevel_admission_pump;
/// Linux-only runtime-owned pending admission queue seam。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_toplevel_admission_runtime_queue;
/// Linux-only adapter-owned `new_toplevel` identity registration proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_toplevel_identity_registration;
/// Linux-only Wayland client 依赖与类型 import 编译边界。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_wayland_client_endpoint;
/// Linux-only `wl_compositor` state owner 与 per-client compositor data seam。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_wl_compositor;
/// Linux-only controlled client `wl_compositor` bind proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_wl_compositor_client_bind;
/// Linux-only controlled `wl_surface` creation 与 adapter-owned identity proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_wl_surface_identity;
/// Linux-only xdg lifecycle callback-like signal到 identity lookup observation boundary。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_xdg_lifecycle_observation;
/// Linux-only xdg-shell global 与 request-handler 的编译边界；不注册真实 global。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_xdg_shell;
/// Linux-only controlled client `xdg_surface` creation proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_xdg_surface_client_bind;
/// Linux-only controlled client `xdg_toplevel` creation proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_xdg_toplevel_client_bind;
/// Linux-only Smithay toplevel ObjectId 到 AdapterToplevelId 的 ownership registry。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_xdg_toplevel_identity;
/// Linux-only controlled client `xdg_wm_base` bind proof。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_xdg_wm_base_client_bind;
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
/// Surface/XDG protocol object 接纳的跨平台纯数据 identity 与 mapping contract。
pub mod surface_xdg_admission;
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
/// XDG toplevel lifecycle identity lookup 的跨平台纯数据 observation report。
pub mod xdg_lifecycle_observation;
/// Adapter-owned xdg toplevel identity registry 的跨平台纯数据事务核心。
pub mod xdg_toplevel_identity;

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
pub use linux_ledger_admission_owner::{
    AdapterLedgerAdmissionBlocker, AdapterLedgerAdmissionError, AdapterLedgerAdmissionOperation,
    AdapterLedgerAdmissionReport, adapter_ledger_admission_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_live_toplevel_admission_owner::{
    LiveToplevelAdmissionOwnerBlocker, LiveToplevelAdmissionOwnerObservation,
    LiveToplevelAdmissionOwnerOperation, LiveToplevelAdmissionOwnerReport,
    enqueue_live_toplevel_admission_from_display, enqueue_live_toplevel_admission_from_observation,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_new_toplevel_callback_observation::{
    ControlledNewToplevelCallbackObservationBlocker, ControlledNewToplevelCallbackObservationError,
    ControlledNewToplevelCallbackObservationOperation,
    ControlledNewToplevelCallbackObservationReport,
    controlled_new_toplevel_callback_observation_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_runtime::SmithayLinuxRuntimeProbe;
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_toplevel_admission_bridge::{
    LiveToplevelAdmissionBridgeBlocker, LiveToplevelAdmissionBridgeInput,
    LiveToplevelAdmissionBridgeOperation, LiveToplevelAdmissionBridgeReport,
    PendingXdgToplevelAdmission, ToplevelAdmissionBridgeQueue,
    live_toplevel_admission_bridge_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_toplevel_admission_consumer::{
    PendingToplevelAdmissionConsumerBlocker, PendingToplevelAdmissionConsumerInput,
    PendingToplevelAdmissionConsumerOperation, PendingToplevelAdmissionConsumerReport,
    consume_pending_toplevel_admission,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_toplevel_admission_pump::{
    ControlledToplevelAdmissionPumpBlocker, ControlledToplevelAdmissionPumpInput,
    ControlledToplevelAdmissionPumpOperation, ControlledToplevelAdmissionPumpReport,
    pump_controlled_toplevel_admission,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_toplevel_admission_runtime_queue::{
    RuntimeToplevelAdmissionDrainReport, RuntimeToplevelAdmissionDrainTick,
    RuntimeToplevelAdmissionEnqueueReport, RuntimeToplevelAdmissionQueueBlocker,
    RuntimeToplevelAdmissionQueueOperation, RuntimeToplevelAdmissionQueueOwner,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_toplevel_identity_registration::{
    AdapterToplevelIdentityRegistrationBlocker, AdapterToplevelIdentityRegistrationError,
    AdapterToplevelIdentityRegistrationOperation, AdapterToplevelIdentityRegistrationReport,
    adapter_toplevel_identity_registration_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_wayland_client_endpoint::{
    LinuxWaylandClientEndpointCompileReport, WaylandClientEndpointCompileBlocker,
    linux_wayland_client_endpoint_compile_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_wl_compositor::{
    LinuxWlCompositorGlobalBlocker, LinuxWlCompositorGlobalInitError,
    LinuxWlCompositorReadinessReport, linux_wl_compositor_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_wl_compositor_client_bind::{
    ControlledWlCompositorBindBlocker, ControlledWlCompositorBindError,
    ControlledWlCompositorBindOperation, ControlledWlCompositorBindReport,
    controlled_wl_compositor_bind_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_wl_surface_identity::{
    AdapterSurfaceIdentityMapping, ControlledWlSurfaceCreationBlocker,
    ControlledWlSurfaceCreationError, ControlledWlSurfaceCreationOperation,
    ControlledWlSurfaceCreationReport, SurfaceIdentityError, SurfaceIdentityKey,
    controlled_wl_surface_creation_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_xdg_lifecycle_observation::{
    LinuxXdgToplevelLifecycleBlocker, LinuxXdgToplevelLifecycleReadinessReport,
    linux_xdg_toplevel_lifecycle_readiness_report, observe_toplevel_lifecycle,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_xdg_shell::{
    LinuxXdgShellCompileBlocker, LinuxXdgShellCompileReport, LinuxXdgShellGlobalBlocker,
    LinuxXdgShellGlobalInitError, LinuxXdgShellGlobalInitReport, LinuxXdgShellStateSkeleton,
    linux_xdg_shell_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_xdg_surface_client_bind::{
    ControlledXdgSurfaceCreationBlocker, ControlledXdgSurfaceCreationError,
    ControlledXdgSurfaceCreationOperation, ControlledXdgSurfaceCreationReport,
    controlled_xdg_surface_creation_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_xdg_toplevel_client_bind::{
    ControlledXdgToplevelCreationBlocker, ControlledXdgToplevelCreationError,
    ControlledXdgToplevelCreationOperation, ControlledXdgToplevelCreationReport,
    controlled_xdg_toplevel_creation_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_xdg_toplevel_identity::{
    LinuxXdgToplevelIdentityBlocker, LinuxXdgToplevelIdentityKey,
    LinuxXdgToplevelIdentityOperationError, LinuxXdgToplevelIdentityReadinessReport,
    LinuxXdgToplevelIdentityRegistry, LinuxXdgToplevelIdentitySourceError,
    linux_xdg_toplevel_identity_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use linux_xdg_wm_base_client_bind::{
    ControlledXdgWmBaseBindBlocker, ControlledXdgWmBaseBindError, ControlledXdgWmBaseBindOperation,
    ControlledXdgWmBaseBindReport, controlled_xdg_wm_base_bind_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_runtime_coordinator::{
    NestedRuntimeAdmissionPumpReport, NestedRuntimeCoordinator, NestedRuntimeCoordinatorBlocker,
    NestedRuntimeCoordinatorReadinessReport, NestedRuntimeLiveAdmissionPumpReport,
    NestedRuntimePumpError, NestedRuntimePumpErrorKind, NestedRuntimePumpReport,
    nested_runtime_coordinator_readiness_report,
};
#[allow(unused_imports)]
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub use nested_runtime_loop::{
    NestedRuntimeLiveAdmissionRunSummary, NestedRuntimeLoop, NestedRuntimeLoopBlocker,
    NestedRuntimeLoopConfig, NestedRuntimeLoopError, NestedRuntimeLoopExitReason,
    NestedRuntimeLoopReadinessReport, NestedRuntimeLoopReport, NestedRuntimeLoopStopHandle,
    NestedRuntimeWakeupReport, nested_runtime_loop_readiness_report,
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
pub use surface_xdg_admission::{
    AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId, SurfaceAdmissionIntent,
    SurfaceAdmissionMapping, SurfaceXdgAdmissionBlocker, SurfaceXdgAdmissionError,
    SurfaceXdgAdmissionLedger, SurfaceXdgAdmissionReadinessReport, SurfaceXdgAdmissionReport,
    SurfaceXdgLifecycleReadinessReport, SurfaceXdgRemovalError, SurfaceXdgRemovalReport,
    ToplevelAdmissionMapping, XdgToplevelAdmissionIntent, XdgToplevelUnmapIntent,
    surface_xdg_admission_readiness_report, surface_xdg_lifecycle_readiness_report,
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
#[allow(unused_imports)]
pub use xdg_lifecycle_observation::{
    XdgToplevelLifecycleObservation, XdgToplevelLifecycleObservationError,
    XdgToplevelLifecycleObservationReport, XdgToplevelLifecycleSignal,
};
#[allow(unused_imports)]
pub use xdg_toplevel_identity::{XdgToplevelIdentityError, XdgToplevelIdentityMapping};

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
            "NestedRuntimeLiveAdmissionRunSummary",
            "live_admission: loop_report.live_admission",
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
            "pub struct NestedRuntimeLiveAdmissionRunSummary",
            "pub struct NestedRuntimeLoopStopHandle",
            "pub enum NestedRuntimeLoopExitReason",
            "pub fn run_for_iterations",
            "pub fn stop_handle",
            "RuntimeToplevelAdmissionDrainTick",
            "pump_once_with_live_toplevel_admission_drain(",
            "RuntimeToplevelAdmissionDrainTick::phase52y_default(",
            "NestedRuntimeLiveAdmissionRunSummary::from_live_pump",
            "live_admission",
            ".lifecycle_report",
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
            ".coordinator.pump_once(state, config.pump_timeout)".to_string(),
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

    /// Phase 52Z coordinator 必须拥有 runtime admission queue owner，并在 lifecycle 后 drain。
    #[test]
    fn nested_runtime_coordinator_admission_drain_source_uses_runtime_queue_owner() {
        let source = include_str!("nested_runtime_coordinator.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "RuntimeToplevelAdmissionQueueOwner",
            "RuntimeToplevelAdmissionDrainTick",
            "RuntimeToplevelAdmissionDrainReport",
            "RuntimeToplevelAdmissionEnqueueReport",
            "PendingXdgToplevelAdmission",
            "admission_queue_owner: RuntimeToplevelAdmissionQueueOwner",
            "RuntimeToplevelAdmissionQueueOwner::new(",
            "pub struct NestedRuntimeAdmissionPumpReport",
            "pub fn enqueue_pending_toplevel_admission(",
            "pub fn pump_once_with_toplevel_admission_drain(",
            "let lifecycle_report = self.pump_once(state, timeout);",
            ".drain_pending_toplevel_admission_once(state, tick)",
            "pub fn admission_pending_count(",
            "pub fn admission_surface_mapping(",
            "pub fn admission_toplevel_mapping(",
        ] {
            assert!(
                code.contains(required),
                "Phase 52Z coordinator admission drain seam 缺少证据: {required}"
            );
        }

        for forbidden in [
            "BackendEvent::",
            "CoreCommand::",
            ".workspaces",
            ".slots",
            ".stacks",
            "insert_window",
            "WindowRegistry",
            "SeatHandler",
            "SmithayWaylandDisplayProbe",
            "UnixStream",
            "Connection::",
            "registry_queue_init",
            "flush_clients_once",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52Z coordinator admission drain 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 53A live callback admission owner API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn live_toplevel_admission_owner_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_live_toplevel_admission_owner;")
            .expect("Phase 53A live toplevel admission owner module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_live_toplevel_admission_owner::{")
            .expect("Phase 53A live toplevel admission owner re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 53A owner 必须把 live callback observation 入队到 coordinator admission queue。
    #[test]
    fn live_toplevel_admission_owner_source_enqueues_coordinator_from_callback_observation() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_live_toplevel_admission_owner.rs"),
        )
        .expect("Phase 53A live toplevel admission owner module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct LiveToplevelAdmissionOwnerReport",
            "pub struct LiveToplevelAdmissionOwnerObservation",
            "pub enum LiveToplevelAdmissionOwnerBlocker",
            "pub enum LiveToplevelAdmissionOwnerOperation",
            "pub fn enqueue_live_toplevel_admission_from_display(",
            "pub fn enqueue_live_toplevel_admission_from_observation(",
            "server: &SmithayWaylandDisplayProbe",
            "coordinator: &mut NestedRuntimeCoordinator",
            "LiveToplevelAdmissionOwnerObservation::from_display(server)",
            ".last_new_toplevel_callback_observation_sequence()",
            ".last_adapter_toplevel_identity_registration_observation()",
            "DuplicateNewToplevelCallbackObservation",
            "coordinator.has_seen_live_toplevel_callback_sequence(callback_sequence)",
            "LiveToplevelAdmissionBridgeInput::from_registered_identity(",
            "live_toplevel_admission_bridge_report(",
            ".enqueue_pending_toplevel_admission(",
            "coordinator.mark_live_toplevel_callback_sequence_seen(callback_sequence)",
            "coordinator.admission_pending_count()",
            "handler_state_touched: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "window_id_allocated: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 53A live admission owner 缺少 callback->queue 证据: {required}"
            );
        }

        for forbidden in [
            "BackendEvent::",
            "CoreCommand::",
            ".workspaces",
            ".slots",
            ".stacks",
            "insert_window",
            "WindowRegistry",
            "SeatHandler",
            "UnixStream",
            "Connection::",
            "registry_queue_init",
            "dispatch_clients_once",
            "flush_clients_once",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 53A live admission owner 生产代码包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 53B coordinator 必须组合 live owner enqueue 与 runtime admission drain。
    #[test]
    fn nested_runtime_live_admission_pump_source_uses_live_owner_before_drain() {
        let source = include_str!("nested_runtime_coordinator.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "LiveToplevelAdmissionOwnerReport",
            "enqueue_live_toplevel_admission_from_observation",
            "pub struct NestedRuntimeLiveAdmissionPumpReport",
            "pub live_admission_owner_report: LiveToplevelAdmissionOwnerReport",
            "pub fn pump_once_with_live_toplevel_admission_drain(",
            "seen_live_toplevel_callback_sequences",
            "pub fn has_seen_live_toplevel_callback_sequence(",
            "pub fn mark_live_toplevel_callback_sequence_seen(",
            "let lifecycle_report = self.pump_once(state, timeout);",
            "let observation = self.flow.live_toplevel_admission_observation();",
            "enqueue_live_toplevel_admission_from_observation(observation, self)",
            ".drain_pending_toplevel_admission_once(state, tick)",
        ] {
            assert!(
                code.contains(required),
                "Phase 53B live admission pump 缺少 coordinator seam 证据: {required}"
            );
        }

        for forbidden in [
            "SeatHandler",
            "registry_queue_init",
            "Connection::",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 53B live admission pump 生产代码包含禁止 token: {forbidden}"
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

    /// 验证 xdg-shell 编译边界的声明与导出都严格保持 Linux-only。
    #[test]
    fn linux_xdg_shell_compile_boundary_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod linux_xdg_shell;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub use linux_xdg_shell::{")
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 Linux-only handler 只建立编译形状，不触发 ledger 或核心 mutation。
    #[test]
    fn linux_xdg_shell_boundary_does_not_trigger_ledger_unmap() {
        let source = include_str!("linux_xdg_shell.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for forbidden in [
            "SurfaceXdgAdmissionLedger",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::DetachWindowFromSurface",
            "State::detach_window_from_surface",
            "SurfaceRegistry",
            "WindowRegistry",
            "Workspace",
        ] {
            assert!(
                !production_code.contains(forbidden),
                "xdg-shell compile seam 包含禁止生产调用: {forbidden}"
            );
        }

        for conservative_field in [
            "xdg_unmap_callback_observed: false",
            "ledger_unmap_invoked_from_linux_boundary: false",
            "real_xdg_shell_runtime_available: false",
            "protocol_dispatch_started: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(conservative_field),
                "xdg-shell compile seam 缺少保守真值: {conservative_field}"
            );
        }
    }

    /// 验证 xdg toplevel identity mapping 的声明与导出严格保持 Linux-only。
    #[test]
    fn linux_xdg_toplevel_identity_mapping_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod linux_xdg_toplevel_identity;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub use linux_xdg_toplevel_identity::{")
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 Linux identity wrapper 只保存 ObjectId key，不触发 ledger/core/input。
    #[test]
    fn linux_xdg_toplevel_identity_keeps_production_boundary_conservative() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/smithay_backend/linux_xdg_toplevel_identity.rs");
        let source = std::fs::read_to_string(path).expect("Phase 52F Linux identity 模块必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "ObjectId",
            "ToplevelSurface",
            ".xdg_toplevel().id()",
            "IdentitySourceNotStable",
            "SmithayIdentityUnavailable",
            "identity_mapping_available: true",
            "identity_source_stable: true",
            "ledger_unmap_invoked: false",
            "callback_observed: false",
            "real_xdg_shell_runtime_available: false",
            "protocol_dispatch_started: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(required),
                "Phase 52F identity wrapper 缺少边界证据: {required}"
            );
        }

        for forbidden in [
            "ToplevelSurface>",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent",
            "CoreCommand",
            "crate::core",
            "SeatHandler",
            "XdgShellState::new",
        ] {
            assert!(
                !production_code.contains(forbidden),
                "Phase 52F identity wrapper 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// 验证 lifecycle callback identity lookup 模块严格保持 Linux-only。
    #[test]
    fn linux_xdg_lifecycle_observation_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub mod linux_xdg_lifecycle_observation;")
            .collect::<Vec<_>>();
        let reexport_lines = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "pub use linux_xdg_lifecycle_observation::{")
            .collect::<Vec<_>>();

        assert_eq!(module_lines.len(), 1);
        assert_eq!(reexport_lines.len(), 1);
        assert_eq!(lines[module_lines[0].0 - 1], required_gate);
        assert_eq!(lines[reexport_lines[0].0 - 1], required_gate);
    }

    /// 验证 callback observation 只读 lookup 并保持 ledger/core/runtime capability false。
    #[test]
    fn linux_xdg_lifecycle_observation_keeps_production_boundary_conservative() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let observation_source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_xdg_lifecycle_observation.rs"),
        )
        .expect("Phase 52G Linux observation 模块必须存在");
        let shell_source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_xdg_shell.rs"))
                .expect("Linux xdg-shell handler 模块必须存在");
        let production = observation_source
            .split_once("#[cfg(test)]")
            .map_or(observation_source.as_str(), |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "ToplevelSurface",
            "observe_toplevel_lifecycle",
            ".lookup_toplevel(toplevel)",
            "callback_identity_lookup_available: true",
            "callback_observed: false",
            "ledger_unmap_invoked: false",
            "core_detach_invoked: false",
            "real_xdg_shell_runtime_available: false",
            "protocol_dispatch_started: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(required),
                "Phase 52G observation 缺少边界证据: {required}"
            );
        }

        for required in [
            "fn toplevel_destroyed(&mut self, surface: ToplevelSurface)",
            "observe_toplevel_lifecycle(",
            "last_toplevel_lifecycle_observation",
        ] {
            assert!(
                shell_source.contains(required),
                "xdg-shell handler 缺少 observation wiring: {required}"
            );
        }

        for forbidden in [
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent",
            "CoreCommand",
            "crate::core",
            ".remove(",
            "SeatHandler",
            "XdgShellState::new",
        ] {
            assert!(
                !production_code.contains(forbidden),
                "Phase 52G observation 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// 验证 Phase 52I global owner 只存在于 Linux-only 编译边界。
    #[test]
    fn linux_xdg_shell_global_owner_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let shell_module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_xdg_shell;")
            .expect("Linux xdg-shell module 必须存在");
        let shell_reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_xdg_shell::{")
            .expect("Linux xdg-shell re-export 必须存在");

        assert_eq!(lines[shell_module.0 - 1], required_gate);
        assert_eq!(lines[shell_reexport.0 - 1], required_gate);
    }

    /// 验证显式 global 初始化由 display owner 驱动，且不越界启动 runtime。
    #[test]
    fn linux_xdg_shell_global_owner_source_is_explicit_and_conservative() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let shell_source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_xdg_shell.rs"))
                .expect("Linux xdg-shell handler 模块必须存在");
        let display_source =
            std::fs::read_to_string(root.join("src/smithay_backend/wayland_display.rs"))
                .expect("Linux Wayland display owner 模块必须存在");
        let shell_production = shell_source
            .split_once("#[cfg(test)]")
            .map_or(shell_source.as_str(), |(production, _)| production);
        let display_production = display_source
            .split_once("#[cfg(test)]")
            .map_or(display_source.as_str(), |(production, _)| production);
        let shell_code = shell_production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let display_code = display_production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let init_code = shell_code
            .split_once("pub(crate) fn initialize_xdg_shell_global")
            .and_then(|(_, tail)| tail.split_once("fn xdg_shell_state_mut"))
            .map(|(init, _)| init)
            .expect("Phase 52I crate-private init 边界必须可定位");
        let display_init_code = display_code
            .split_once("pub fn initialize_xdg_shell_global")
            .and_then(|(_, tail)| tail.split_once("pub fn is_xdg_shell_global_initialized"))
            .map(|(init, _)| init)
            .expect("Phase 52I public owner init 边界必须可定位");

        for required in [
            "LinuxXdgShellGlobalInitError",
            "LinuxXdgShellGlobalInitReport",
            "LinuxXdgShellGlobalBlocker",
            "pub(crate) fn initialize_xdg_shell_global",
            "XdgShellState::new::<LinuxXdgShellStateSkeleton>(display_handle)",
            "is_xdg_shell_global_initialized",
            "xdg_shell_state_new_invoked: initialized",
            "xdg_shell_global_initialized: initialized",
            "xdg_shell_state_owned: initialized",
            "client_harness_available: false",
            "new_toplevel_registration_owner_available: true",
            "callback_observed: false",
            "ledger_unmap_invoked: false",
            "core_detach_invoked: false",
            "protocol_dispatch_started: false",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                shell_code.contains(required),
                "Phase 52I shell owner 缺少边界证据: {required}"
            );
        }

        for required in [
            "pub fn initialize_xdg_shell_global",
            "self.display.handle()",
            "self.state.initialize_xdg_shell_global(&display_handle)",
            "pub fn is_xdg_shell_global_initialized",
            "pub fn xdg_shell_global_readiness_report",
        ] {
            assert!(
                display_code.contains(required),
                "Phase 52I display owner 缺少显式初始化证据: {required}"
            );
        }

        let constructor = display_code
            .split_once("pub fn new()")
            .and_then(|(_, tail)| tail.split_once("pub fn display_handle"))
            .map(|(constructor, _)| constructor)
            .expect("Display owner constructor 边界必须可定位");
        assert!(
            !constructor.contains("initialize_xdg_shell_global"),
            "Display constructor 不得自动初始化 xdg-shell global"
        );

        for forbidden in [
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::DetachWindowFromSurface",
            "SeatHandler",
            ".dispatch_clients(",
            ".insert_client(",
            ".register_toplevel(",
            "observe_toplevel_lifecycle(",
        ] {
            assert!(
                !init_code.contains(forbidden) && !display_init_code.contains(forbidden),
                "Phase 52I production owner 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 52M-B compositor owner 必须同时受 feature 与 Linux target 双重隔离。
    #[test]
    fn linux_wl_compositor_owner_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_wl_compositor;")
            .expect("Phase 52M-B Linux wl_compositor owner module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_wl_compositor::{")
            .expect("Phase 52M-B Linux wl_compositor owner re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52M-B owner/data seam 必须显式初始化并保持 runtime/core 边界关闭。
    #[test]
    fn linux_wl_compositor_owner_source_is_explicit_and_conservative() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let compositor_source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_compositor.rs"))
                .expect("Phase 52M-B Linux wl_compositor owner module 必须存在");
        let shell_source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_xdg_shell.rs"))
                .expect("Linux display state module 必须存在");
        let display_source =
            std::fs::read_to_string(root.join("src/smithay_backend/wayland_display.rs"))
                .expect("Linux Wayland display owner module 必须存在");
        let client_source =
            std::fs::read_to_string(root.join("src/smithay_backend/client_insert.rs"))
                .expect("Linux client data owner module 必须存在");
        let production = |source: &str| {
            source
                .split_once("#[cfg(test)]")
                .map_or(source, |(production, _)| production)
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let compositor_code = production(&compositor_source);
        let shell_code = production(&shell_source);
        let display_code = production(&display_source);
        let client_code = production(&client_source);
        let shell_init_code = shell_code
            .split_once("pub(crate) fn initialize_wl_compositor_global")
            .and_then(|(_, tail)| tail.split_once("fn wl_compositor_state_mut"))
            .map(|(code, _)| code)
            .expect("Phase 52M-B compositor init seam 必须可定位");
        let shell_handler_code = shell_code
            .split_once("impl CompositorHandler for LinuxXdgShellStateSkeleton")
            .and_then(|(_, tail)| {
                tail.split_once("smithay::delegate_compositor!(LinuxXdgShellStateSkeleton)")
            })
            .map(|(code, _)| code)
            .expect("Phase 52M-B compositor handler seam 必须可定位");

        for required in [
            "LinuxWlCompositorGlobalInitError",
            "LinuxWlCompositorReadinessReport",
            "global_owner_available: true",
            "client_connection_created: false",
            "registry_bind_attempted: false",
            "client_bound_wl_compositor: false",
            "protocol_dispatch_started: false",
            "real_compositor_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                compositor_code.contains(required),
                "Phase 52M-B report 缺少保守能力证据: {required}"
            );
        }

        for required in [
            "compositor_state: Option<CompositorState>",
            "pub(crate) fn initialize_wl_compositor_global",
            "CompositorState::new::<LinuxXdgShellStateSkeleton>(display_handle)",
            "impl CompositorHandler for LinuxXdgShellStateSkeleton",
            "smithay::delegate_compositor!(LinuxXdgShellStateSkeleton)",
        ] {
            assert!(
                shell_code.contains(required),
                "Phase 52M-B display state 缺少 owner/handler 证据: {required}"
            );
        }

        for required in [
            "pub fn initialize_wl_compositor_global",
            "self.display.handle()",
            "self.state.initialize_wl_compositor_global(&display_handle)",
            "pub fn wl_compositor_readiness_report",
        ] {
            assert!(
                display_code.contains(required),
                "Phase 52M-B display owner 缺少显式初始化证据: {required}"
            );
        }

        for required in [
            "compositor_state: CompositorClientState",
            "CompositorClientState::default()",
            "pub(crate) fn compositor_state(&self) -> &CompositorClientState",
        ] {
            assert!(
                client_code.contains(required),
                "Phase 52M-B client data owner 缺少 per-client state 证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "Connection::from_socket",
            ".new_event_queue(",
            ".get_registry(",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SeatHandler",
        ] {
            assert!(
                !compositor_code.contains(forbidden)
                    && !shell_init_code.contains(forbidden)
                    && !shell_handler_code.contains(forbidden)
                    && !display_code.contains(forbidden)
                    && !client_code.contains(forbidden),
                "Phase 52M-B production seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 52N controlled bind proof 必须同时受 feature 与 Linux target 双重隔离。
    #[test]
    fn controlled_wl_compositor_bind_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_wl_compositor_client_bind;")
            .expect("Phase 52N controlled bind module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_wl_compositor_client_bind::{")
            .expect("Phase 52N controlled bind re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52N 只允许 controlled wl_compositor bind，不得创建 surface 或进入 xdg/core。
    #[test]
    fn controlled_wl_compositor_bind_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_wl_compositor_client_bind.rs"),
        )
        .expect("Phase 52N controlled bind module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<ControlledClientState>(&connection)",
            ".bind::<WlCompositor, _, _>",
            "event_queue.roundtrip(&mut client_state)",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "client_connection_created: true",
            "event_queue_created: true",
            "registry_roundtrip_started: true",
            "registry_bind_attempted: true",
            "client_bound_wl_compositor: true",
            "client_bound_xdg_wm_base: false",
            "wl_surface_created: false",
            "protocol_dispatch_started: true",
            "real_compositor_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52N controlled bind 缺少证明证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "XdgWmBase",
            "xdg_wm_base::",
            "WlSurface",
            ".create_surface(",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "AdapterSurfaceId",
            "SurfaceId",
            "SeatHandler",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52N controlled bind 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52O controlled surface proof 必须同时受 feature 与 Linux target 双重隔离。
    #[test]
    fn controlled_wl_surface_creation_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_wl_surface_identity;")
            .expect("Phase 52O controlled surface identity module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_wl_surface_identity::{")
            .expect("Phase 52O controlled surface identity re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52O 只允许受控 surface creation 与 adapter identity，不得进入 xdg/core。
    #[test]
    fn controlled_wl_surface_creation_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 52O controlled surface identity module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<ControlledSurfaceClientState>(&connection)",
            ".bind::<WlCompositor, _, _>",
            "MissingClientWlCompositorBind",
            ".create_surface(&queue_handle, ())",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "wl_surface_create_attempted: true",
            "wl_surface_created: true",
            "server_surface_observed: true",
            "adapter_surface_identity_allocated: true",
            "client_bound_xdg_wm_base: false",
            "xdg_surface_created: false",
            "xdg_toplevel_created: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52O controlled surface proof 缺少证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "XdgWmBase",
            "xdg_wm_base::",
            "xdg_surface::",
            "xdg_toplevel::",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SurfaceRegistry",
            "WindowRegistry",
            "WindowId",
            "SeatHandler",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52O controlled surface proof 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52P controlled xdg_wm_base bind API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn controlled_xdg_wm_base_bind_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_xdg_wm_base_client_bind;")
            .expect("Phase 52P controlled xdg_wm_base bind module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_xdg_wm_base_client_bind::{")
            .expect("Phase 52P controlled xdg_wm_base bind re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52P 只允许 bind xdg_wm_base，不得创建 shell object 或进入 ledger/core。
    #[test]
    fn controlled_xdg_wm_base_bind_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_xdg_wm_base_client_bind.rs"),
        )
        .expect("Phase 52P controlled xdg_wm_base bind module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "server.is_xdg_shell_global_initialized()",
            "server.is_wl_compositor_global_initialized()",
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<ControlledXdgWmBaseClientState>(&connection)",
            ".bind::<WlCompositor, _, _>",
            ".bind::<XdgWmBase, _, _>",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "client_bound_wl_compositor: true",
            "client_bound_xdg_wm_base: true",
            "wl_surface_created: false",
            "adapter_surface_identity_allocated: false",
            "xdg_surface_create_attempted: false",
            "xdg_surface_created: false",
            "xdg_toplevel_create_attempted: false",
            "xdg_toplevel_created: false",
            "new_toplevel_callback_observed: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "protocol_dispatch_started: true",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52P controlled xdg_wm_base bind 缺少证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            ".create_surface(",
            ".get_xdg_surface(",
            ".get_toplevel(",
            "Event::Ping",
            ".pong(",
            "xdg_surface::XdgSurface",
            "xdg_toplevel::XdgToplevel",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SurfaceRegistry",
            "WindowRegistry",
            "WindowId",
            "SeatHandler",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52P controlled xdg_wm_base bind 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52Q controlled xdg_surface creation API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn controlled_xdg_surface_creation_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_xdg_surface_client_bind;")
            .expect("Phase 52Q controlled xdg_surface creation module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_xdg_surface_client_bind::{")
            .expect("Phase 52Q controlled xdg_surface creation re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52Q 只允许受控 xdg_surface creation，不得创建 toplevel 或进入 ledger/core。
    #[test]
    fn controlled_xdg_surface_creation_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_xdg_surface_client_bind.rs"),
        )
        .expect("Phase 52Q controlled xdg_surface creation module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "server.is_xdg_shell_global_initialized()",
            "server.is_wl_compositor_global_initialized()",
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<ControlledXdgSurfaceClientState>(&connection)",
            ".bind::<WlCompositor, _, _>",
            ".create_surface(&queue_handle, ())",
            ".bind::<XdgWmBase, _, _>",
            ".get_xdg_surface(&surface, &queue_handle, ())",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "client_bound_wl_compositor: true",
            "wl_surface_create_attempted: true",
            "wl_surface_created: true",
            "server_surface_observed: true",
            "adapter_surface_identity_allocated: true",
            "client_bound_xdg_wm_base: true",
            "xdg_surface_create_attempted: true",
            "xdg_surface_created: true",
            "xdg_toplevel_create_attempted: false",
            "xdg_toplevel_created: false",
            "new_toplevel_callback_observed: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "window_id_allocated: false",
            "protocol_dispatch_started: true",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52Q controlled xdg_surface proof 缺少证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            ".get_toplevel(",
            "xdg_toplevel::XdgToplevel",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SurfaceRegistry",
            "WindowRegistry",
            "WindowId",
            "SeatHandler",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52Q controlled xdg_surface proof 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52R controlled xdg_toplevel creation API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn controlled_xdg_toplevel_creation_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_xdg_toplevel_client_bind;")
            .expect("Phase 52R controlled xdg_toplevel creation module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_xdg_toplevel_client_bind::{")
            .expect("Phase 52R controlled xdg_toplevel creation re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52R 只允许受控 xdg_toplevel object creation，不得注册 identity 或进入 ledger/core。
    #[test]
    fn controlled_xdg_toplevel_creation_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_xdg_toplevel_client_bind.rs"),
        )
        .expect("Phase 52R controlled xdg_toplevel creation module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "server.is_xdg_shell_global_initialized()",
            "server.is_wl_compositor_global_initialized()",
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<ControlledXdgToplevelClientState>(&connection)",
            ".bind::<WlCompositor, _, _>",
            ".create_surface(&queue_handle, ())",
            ".bind::<XdgWmBase, _, _>",
            ".get_xdg_surface(&surface, &queue_handle, ())",
            ".get_toplevel(&queue_handle, ())",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "client_bound_wl_compositor: true",
            "wl_surface_create_attempted: true",
            "wl_surface_created: true",
            "server_surface_observed: true",
            "adapter_surface_identity_allocated: true",
            "client_bound_xdg_wm_base: true",
            "xdg_surface_create_attempted: true",
            "xdg_surface_created: true",
            "xdg_toplevel_create_attempted: true",
            "xdg_toplevel_created: true",
            "new_toplevel_callback_observed: false",
            "adapter_toplevel_identity_registered: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "window_id_allocated: false",
            "protocol_dispatch_started: true",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52R controlled xdg_toplevel proof 缺少证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "LinuxXdgToplevelIdentityRegistry",
            ".insert(",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SurfaceRegistry",
            "WindowRegistry",
            "WindowId",
            "SeatHandler",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52R controlled xdg_toplevel proof 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52S controlled new_toplevel callback observation API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn controlled_new_toplevel_callback_observation_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_new_toplevel_callback_observation;")
            .expect("Phase 52S controlled new_toplevel callback observation module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_new_toplevel_callback_observation::{")
            .expect("Phase 52S controlled new_toplevel callback observation re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52S 仍证明 `new_toplevel` callback；Phase 52T 后报告也读取 adapter registration。
    #[test]
    fn controlled_new_toplevel_callback_observation_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_new_toplevel_callback_observation.rs"),
        )
        .expect("Phase 52S controlled new_toplevel callback observation module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "server.is_xdg_shell_global_initialized()",
            "server.is_wl_compositor_global_initialized()",
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<ControlledNewToplevelCallbackClientState>(&connection)",
            ".bind::<WlCompositor, _, _>",
            ".create_surface(&queue_handle, ())",
            ".bind::<XdgWmBase, _, _>",
            ".get_xdg_surface(&surface, &queue_handle, ())",
            ".get_toplevel(&queue_handle, ())",
            "server.new_toplevel_callback_observation_count()",
            ".last_new_toplevel_callback_observation_sequence()",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "client_bound_wl_compositor: true",
            "wl_surface_create_attempted: true",
            "wl_surface_created: true",
            "server_surface_observed: true",
            "adapter_surface_identity_allocated: true",
            "client_bound_xdg_wm_base: true",
            "xdg_surface_create_attempted: true",
            "xdg_surface_created: true",
            "xdg_toplevel_create_attempted: true",
            "xdg_toplevel_created: true",
            "new_toplevel_callback_expected: true",
            "new_toplevel_callback_observed: true",
            "new_toplevel_callback_count: callback_count",
            ".last_adapter_toplevel_identity_registration_observation()",
            "adapter_toplevel_identity_registered: registration.is_some()",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "window_id_allocated: false",
            "protocol_dispatch_started: true",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52S controlled callback observation 缺少证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "LinuxXdgToplevelIdentityRegistry",
            ".insert(",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SurfaceRegistry",
            "WindowRegistry",
            "WindowId",
            "SeatHandler",
            "libinput",
            "drm",
            "gbm",
            "ToplevelSurface",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52S controlled callback observation 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52T adapter toplevel identity registration API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn adapter_toplevel_identity_registration_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_toplevel_identity_registration;")
            .expect("Phase 52T adapter toplevel identity registration module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_toplevel_identity_registration::{")
            .expect("Phase 52T adapter toplevel identity registration re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52T 只允许 adapter-owned toplevel identity registration，不得进入 ledger/core/window。
    #[test]
    fn adapter_toplevel_identity_registration_source_stays_within_proof_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_toplevel_identity_registration.rs"),
        )
        .expect("Phase 52T adapter toplevel identity registration module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "server.is_xdg_shell_global_initialized()",
            "server.is_wl_compositor_global_initialized()",
            "Connection::from_socket(client_stream)",
            "registry_queue_init::<",
            "AdapterToplevelIdentityRegistrationClientState",
            ".bind::<WlCompositor, _, _>",
            ".create_surface(&queue_handle, ())",
            ".bind::<XdgWmBase, _, _>",
            ".get_xdg_surface(&surface, &queue_handle, ())",
            ".get_toplevel(&queue_handle, ())",
            "server.new_toplevel_callback_observation_count()",
            ".last_adapter_toplevel_identity_registration_observation()",
            "NestedClientInsertCompileBoundary::new(server.display_handle())",
            ".insert_client(server_stream, session)",
            ".dispatch_clients_once()",
            ".flush_clients_once()",
            "client_bound_wl_compositor: true",
            "wl_surface_create_attempted: true",
            "wl_surface_created: true",
            "server_surface_observed: true",
            "adapter_surface_identity_available: true",
            "client_bound_xdg_wm_base: true",
            "xdg_surface_created: true",
            "xdg_toplevel_created: true",
            "new_toplevel_callback_observed: true",
            "toplevel_identity_source_available: true",
            "adapter_toplevel_identity_registration_attempted: true",
            "adapter_toplevel_identity_registered: true",
            "adapter_toplevel_id_allocated: true",
            "adapter_surface_id_linked: registration.adapter_surface",
            "== surface_mapping.adapter_surface_id",
            "ledger_admit_invoked: false",
            "ledger_unmap_invoked: false",
            "core_register_invoked: false",
            "core_detach_invoked: false",
            "window_id_allocated: false",
            "protocol_dispatch_started: true",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52T adapter toplevel identity registration 缺少证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SurfaceRegistry",
            "WindowRegistry",
            "WindowId",
            "SeatHandler",
            "libinput",
            "drm",
            "gbm",
            "last_toplevel_surface",
            "Option<ToplevelSurface>",
            "Vec<ToplevelSurface>",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52T adapter toplevel identity registration 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52T 的 handler state 只能注册 adapter identity，不得保存 `ToplevelSurface` 或触碰 core。
    #[test]
    fn linux_new_toplevel_callback_handler_state_registers_adapter_identity_only() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(root.join("src/smithay_backend/linux_xdg_shell.rs"))
            .expect("Linux xdg shell handler state 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "new_toplevel_callback_count: u64",
            "last_new_toplevel_callback_observation_sequence: Option<u64>",
            "last_toplevel_identity_registration:",
            "pub(crate) const fn new_toplevel_callback_observation_count(&self) -> u64",
            "pub(crate) const fn last_new_toplevel_callback_observation_sequence(&self) -> Option<u64>",
            "pub(crate) fn last_adapter_toplevel_identity_registration_observation(",
            "fn record_new_toplevel_callback_observation(&mut self)",
            "fn register_new_toplevel_identity(&mut self, surface: &ToplevelSurface)",
            "self.new_toplevel_callback_count += 1",
            "self.last_new_toplevel_callback_observation_sequence = Some(sequence)",
            "LinuxXdgToplevelIdentityRegistry::key_for_toplevel(surface)",
            ".observe_surface(surface.wl_surface())",
            ".register(identity, adapter_surface)",
            "fn new_toplevel(&mut self, surface: ToplevelSurface)",
            "self.record_new_toplevel_callback_observation();",
            "self.register_new_toplevel_identity(&surface);",
        ] {
            assert!(
                code.contains(required),
                "Phase 52T handler state 缺少 adapter identity registration 证据: {required}"
            );
        }

        for forbidden in [
            "last_new_toplevel_surface",
            "Option<ToplevelSurface>",
            "Vec<ToplevelSurface>",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "CoreCommand::",
            "BackendEvent::",
            "WindowId",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52S handler state 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 52V live callback admission bridge API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn live_toplevel_admission_bridge_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_toplevel_admission_bridge;")
            .expect("Phase 52V live toplevel admission bridge module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_toplevel_admission_bridge::{")
            .expect("Phase 52V live toplevel admission bridge re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52V bridge 只能生成 pending admission intent，不得在 handler 层消费 ledger/core。
    #[test]
    fn live_toplevel_admission_bridge_source_stays_b_lite() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_toplevel_admission_bridge.rs"),
        )
        .expect("Phase 52V live toplevel admission bridge module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct PendingXdgToplevelAdmission",
            "pub struct ToplevelAdmissionBridgeQueue",
            "pub struct LiveToplevelAdmissionBridgeReport",
            "pub enum LiveToplevelAdmissionBridgeBlocker",
            "pub fn live_toplevel_admission_bridge_report(",
            "new_toplevel_callback_observed",
            "adapter_surface_identity_available",
            "adapter_toplevel_identity_registered",
            "pending_admission_intent_created",
            "pending_admission_count",
            "ledger_owner_available: false",
            "ledger_consume_attempted: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "window_id_allocated: false",
            "render_support: false",
            "input_support: false",
            "real_compositor_runtime_available: false",
            "real_xdg_shell_runtime_available: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52V bridge 缺少 B-lite 证据: {required}"
            );
        }

        for forbidden in [
            "SurfaceXdgAdmissionLedger",
            ".admit_surface(",
            ".admit_toplevel(",
            "BackendEvent::",
            "CoreCommand::",
            "state::State",
            "&mut State",
            "workspace::WindowId",
            "WindowRegistry",
            "SurfaceRegistry",
            "SeatHandler",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52V bridge 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52W pending admission consumer owner API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn pending_admission_consumer_owner_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_toplevel_admission_consumer;")
            .expect("Phase 52W pending admission consumer owner module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_toplevel_admission_consumer::{")
            .expect("Phase 52W pending admission consumer owner re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52W consumer owner 必须通过 ledger 消费 pending intent，不能回到 handler 直接改 core。
    #[test]
    fn pending_admission_consumer_owner_source_uses_ledger_seam() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_toplevel_admission_consumer.rs"),
        )
        .expect("Phase 52W pending admission consumer owner module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct PendingToplevelAdmissionConsumerInput",
            "pub struct PendingToplevelAdmissionConsumerReport",
            "pub enum PendingToplevelAdmissionConsumerBlocker",
            "pub enum PendingToplevelAdmissionConsumerOperation",
            "pub fn consume_pending_toplevel_admission(",
            "queue: &mut ToplevelAdmissionBridgeQueue",
            "ledger: &mut SurfaceXdgAdmissionLedger",
            "state: &mut State",
            "queue.pop_front()",
            ".admit_surface(",
            ".admit_toplevel(",
            "ledger_consume_attempted: true",
            "ledger_admit_surface_invoked: true",
            "ledger_admit_invoked: true",
            "core_register_invoked: true",
            "window_id_allocated: true",
            "render_support: false",
            "input_support: false",
            "real_compositor_runtime_available: false",
            "real_xdg_shell_runtime_available: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52W consumer owner 缺少 ledger seam 证据: {required}"
            );
        }

        for forbidden in [
            "BackendEvent::",
            "CoreCommand::",
            ".workspaces",
            ".slots",
            ".stacks",
            "insert_window",
            "WindowRegistry",
            "SeatHandler",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52W consumer owner 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52X controlled admission pump API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn controlled_admission_pump_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_toplevel_admission_pump;")
            .expect("Phase 52X controlled admission pump module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_toplevel_admission_pump::{")
            .expect("Phase 52X controlled admission pump re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52X pump 必须串联 registration report -> bridge queue -> consumer owner。
    #[test]
    fn controlled_admission_pump_source_uses_bridge_and_consumer_owner() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_toplevel_admission_pump.rs"),
        )
        .expect("Phase 52X controlled admission pump module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct ControlledToplevelAdmissionPumpInput",
            "pub struct ControlledToplevelAdmissionPumpReport",
            "pub enum ControlledToplevelAdmissionPumpBlocker",
            "pub enum ControlledToplevelAdmissionPumpOperation",
            "pub fn pump_controlled_toplevel_admission(",
            "registration: AdapterToplevelIdentityRegistrationReport",
            "ledger: &mut SurfaceXdgAdmissionLedger",
            "state: &mut State",
            "LiveToplevelAdmissionBridgeInput::from(&registration)",
            "live_toplevel_admission_bridge_report(bridge_input)",
            "ToplevelAdmissionBridgeQueue::new()",
            "queue.push(pending)",
            "consume_pending_toplevel_admission(",
            "PendingToplevelAdmissionConsumerInput",
            "ledger_consume_attempted: consumer_report.ledger_consume_attempted",
            "pending_admission_consumed: consumer_report.pending_admission_consumed",
            "ledger_admit_surface_invoked: consumer_report.ledger_admit_surface_invoked",
            "ledger_admit_invoked: consumer_report.ledger_admit_invoked",
            "core_register_invoked: consumer_report.core_register_invoked",
            "window_id_allocated: consumer_report.window_id_allocated",
            "handler_state_touched: false",
            "render_support: false",
            "input_support: false",
            "real_compositor_runtime_available: false",
            "real_xdg_shell_runtime_available: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52X pump 缺少 bridge/consumer owner 证据: {required}"
            );
        }

        for forbidden in [
            "BackendEvent::",
            "CoreCommand::",
            ".workspaces",
            ".slots",
            ".stacks",
            "insert_window",
            "WindowRegistry",
            "SeatHandler",
            "SmithayWaylandDisplayProbe",
            "UnixStream",
            "Connection::",
            "registry_queue_init",
            "dispatch_clients_once",
            "flush_clients_once",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52X pump 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52Y runtime admission queue owner API 必须同时受 feature 与 Linux target 隔离。
    #[test]
    fn runtime_admission_queue_owner_api_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_toplevel_admission_runtime_queue;")
            .expect("Phase 52Y runtime admission queue owner module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_toplevel_admission_runtime_queue::{")
            .expect("Phase 52Y runtime admission queue owner re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);
    }

    /// Phase 52Y owner 必须由 runtime 层持有 queue + ledger，并按 tick drain consumer。
    #[test]
    fn runtime_admission_queue_owner_source_drains_consumer_from_owned_queue() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_toplevel_admission_runtime_queue.rs"),
        )
        .expect("Phase 52Y runtime admission queue owner module 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "pub struct RuntimeToplevelAdmissionQueueOwner",
            "queue: ToplevelAdmissionBridgeQueue",
            "ledger: SurfaceXdgAdmissionLedger",
            "next_core_surface_id: SurfaceId",
            "pub struct RuntimeToplevelAdmissionDrainTick",
            "pub struct RuntimeToplevelAdmissionEnqueueReport",
            "pub struct RuntimeToplevelAdmissionDrainReport",
            "pub enum RuntimeToplevelAdmissionQueueBlocker",
            "pub enum RuntimeToplevelAdmissionQueueOperation",
            "pub fn enqueue_pending_toplevel_admission(",
            "pub fn drain_pending_toplevel_admission_once(",
            "state: &mut State",
            "PendingToplevelAdmissionConsumerInput",
            "consume_pending_toplevel_admission(",
            "&mut self.queue",
            "&mut self.ledger",
            "self.next_core_surface_id = self.next_core_surface_id.saturating_add(1)",
            "runtime_queue_owned: true",
            "runtime_ledger_owned: true",
            "handler_state_touched: false",
            "render_support: false",
            "input_support: false",
            "real_compositor_runtime_available: false",
            "real_xdg_shell_runtime_available: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 52Y runtime queue owner 缺少 owner/drain 证据: {required}"
            );
        }

        for forbidden in [
            "BackendEvent::",
            "CoreCommand::",
            ".workspaces",
            ".slots",
            ".stacks",
            "insert_window",
            "WindowRegistry",
            "SeatHandler",
            "SmithayWaylandDisplayProbe",
            "UnixStream",
            "Connection::",
            "registry_queue_init",
            "dispatch_clients_once",
            "flush_clients_once",
            "libinput",
            "drm",
            "gbm",
        ] {
            assert!(
                !code.contains(forbidden),
                "Phase 52Y runtime queue owner 包含禁止生产 token: {forbidden}"
            );
        }
    }

    /// Phase 52L client compile seam 必须同时受 feature 与 Linux target 双重隔离。
    #[test]
    fn linux_wayland_client_compile_seam_is_linux_only() {
        let source = include_str!("mod.rs");
        let lines = source.lines().collect::<Vec<_>>();
        let required_gate = "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]";
        let module = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub mod linux_wayland_client_endpoint;")
            .expect("Phase 52L Linux client compile module 必须存在");
        let reexport = lines
            .iter()
            .enumerate()
            .find(|(_, line)| **line == "pub use linux_wayland_client_endpoint::{")
            .expect("Phase 52L Linux client compile re-export 必须存在");

        assert_eq!(lines[module.0 - 1], required_gate);
        assert_eq!(lines[reexport.0 - 1], required_gate);

        let cargo = include_str!("../../Cargo.toml");
        for required in [
            "\"dep:wayland-client\"",
            "\"dep:wayland-protocols\"",
            "wayland-client = { version = \"0.31\", optional = true }",
            "wayland-protocols = { version = \"0.32\", features = [\"client\"], optional = true }",
        ] {
            assert!(
                cargo.contains(required),
                "Phase 52L Cargo gate 缺少依赖证据: {required}"
            );
        }
    }

    /// Compile/import 证据不得偷偷创建 connection、bind global 或进入 ledger/core。
    #[test]
    fn linux_wayland_client_compile_seam_keeps_runtime_boundary_false() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/smithay_backend/linux_wayland_client_endpoint.rs");
        let source = std::fs::read_to_string(path)
            .expect("Phase 52L Linux client compile seam 源码必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let production_code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        for required in [
            "wayland_client::Connection",
            "wl_compositor::WlCompositor",
            "wl_registry::WlRegistry",
            "wl_surface::WlSurface",
            "xdg_wm_base::XdgWmBase",
            "xdg_surface::XdgSurface",
            "xdg_toplevel::XdgToplevel",
            "wayland_client_dependency_available: true",
            "wayland_protocols_client_feature_available: true",
            "linux_client_imports_compile: true",
            "runtime_connection_created: false",
            "event_queue_created: false",
            "registry_bind_attempted: false",
            "client_harness_available: false",
            "callback_observed: false",
            "ledger_admit_invoked: false",
            "ledger_unmap_invoked: false",
            "core_register_invoked: false",
            "core_detach_invoked: false",
            "protocol_dispatch_started: false",
            "real_xdg_shell_runtime_available: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                production_code.contains(required),
                "Phase 52L compile seam 缺少保守证据: {required}"
            );
        }

        for forbidden in [
            "Connection::connect_to_env",
            "Connection::connect_to_name",
            "Connection::from_socket",
            ".new_event_queue(",
            ".get_registry(",
            ".bind(",
            "SurfaceXdgAdmissionLedger",
            ".admit_toplevel(",
            ".unmap_toplevel(",
            "BackendEvent::ToplevelMapped",
            "BackendEvent::ToplevelUnmapped",
            "CoreCommand::RegisterWindowForSurface",
            "CoreCommand::DetachWindowFromSurface",
            "SeatHandler",
        ] {
            assert!(
                !production_code.contains(forbidden),
                "Phase 52L compile seam 包含禁止 runtime token: {forbidden}"
            );
        }
    }
}
