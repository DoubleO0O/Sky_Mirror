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
/// Linux-only SHM-first buffer import adapter skeleton；不 import buffer 或创建 texture。
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
pub mod linux_shm_buffer_import_adapter;
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
pub use linux_shm_buffer_import_adapter::{
    LinuxShmBufferTypeBoundaryEvidence, LinuxShmFirstBufferImportAdapterSkeleton,
    RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker,
    RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation,
    RuntimeSurfaceCommitShmFirstBufferImportAdapterReport, observe_wl_buffer_type_boundary,
    shm_first_buffer_import_adapter_report_from_actual_attempt_record,
};
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
    AdapterSurfaceCommitObservation, AdapterSurfaceIdentityMapping,
    ControlledWlSurfaceCommitBlocker, ControlledWlSurfaceCommitError,
    ControlledWlSurfaceCommitOperation, ControlledWlSurfaceCommitReport,
    ControlledWlSurfaceCreationBlocker, ControlledWlSurfaceCreationError,
    ControlledWlSurfaceCreationOperation, ControlledWlSurfaceCreationReport, SurfaceIdentityError,
    SurfaceIdentityKey, controlled_wl_surface_commit_observation_report,
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
    NestedRuntimeLiveAdmissionRunSummary, NestedRuntimeLiveUnmapRunSummary, NestedRuntimeLoop,
    NestedRuntimeLoopBlocker, NestedRuntimeLoopConfig, NestedRuntimeLoopError,
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
            "pub struct NestedRuntimeLiveUnmapRunSummary",
            "pub struct NestedRuntimeLoopStopHandle",
            "pub enum NestedRuntimeLoopExitReason",
            "pub fn run_for_iterations",
            "pub fn stop_handle",
            "RuntimeToplevelAdmissionDrainTick",
            "pump_once_with_live_toplevel_admission_and_unmap_drain(",
            "RuntimeToplevelAdmissionDrainTick::phase52y_default(",
            "NestedRuntimeLiveAdmissionRunSummary::from_live_pump",
            "NestedRuntimeLiveUnmapRunSummary::from_live_admission_unmap",
            "live_admission",
            "live_unmap",
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

    /// Phase 53J 必须证明 orchestrator 层不会在 live admission backlog 仍有进展时 idle 退出。
    #[test]
    fn runtime_orchestrator_idle_backlog_proof_source_exists() {
        let source = include_str!("nested_runtime_orchestrator.rs");

        for required in [
            "fn runtime_orchestrator_stop_when_idle_drains_live_admission_backlog()",
            "config.loop_config.stop_when_idle = true;",
            "let (first_registration, second_registration) =",
            "report.loop_exit_reason, NestedRuntimeLoopExitReason::Idle",
            "report.live_admission.admissions_consumed, 2",
            "state.surfaces.records().len(), 2",
        ] {
            assert!(
                source.contains(required),
                "Phase 53J orchestrator idle/backlog proof 缺少证据项: {required}"
            );
        }
    }

    /// Phase 53M 必须证明 orchestrator lifecycle report 直接暴露 live unmap 汇总。
    #[test]
    fn runtime_orchestrator_live_unmap_report_source_exists() {
        let source = include_str!("nested_runtime_orchestrator.rs");

        for required in [
            "NestedRuntimeLiveUnmapRunSummary",
            "pub live_unmap: NestedRuntimeLiveUnmapRunSummary",
            "live_unmap: loop_report.live_unmap",
            "fn runtime_orchestrator_run_reports_live_toplevel_unmap()",
            "assert_eq!(report.live_unmap, report.loop_report.live_unmap)",
            "assert_eq!(report.live_unmap.ledger_unmaps, 1)",
        ] {
            assert!(
                source.contains(required),
                "Phase 53M orchestrator live unmap report proof 缺少证据项: {required}"
            );
        }
    }

    /// Phase 53K 必须证明 live destroyed observation 只在 owner 层触发 ledger unmap。
    #[test]
    fn live_toplevel_unmap_owner_proof_source_exists() {
        let queue_source = include_str!("linux_toplevel_admission_runtime_queue.rs");
        let coordinator_source = include_str!("nested_runtime_coordinator.rs");
        let display_source = include_str!("wayland_display.rs");

        for required in [
            "RuntimeToplevelUnmapDrainReport",
            "drain_live_toplevel_unmap_once",
            "XdgToplevelUnmapIntent",
            ".unmap_toplevel(",
            "core_detach_invoked",
            "surface_mapping_retained_after_unmap",
        ] {
            assert!(
                queue_source.contains(required),
                "Phase 53K runtime unmap owner source 缺少证据项: {required}"
            );
        }

        for required in [
            "pump_once_with_live_toplevel_unmap_drain",
            "take_next_live_toplevel_unmap_observation",
            "nested_runtime_live_unmap_pump_detaches_admitted_toplevel",
        ] {
            assert!(
                coordinator_source.contains(required),
                "Phase 53K coordinator unmap proof 缺少证据项: {required}"
            );
        }

        assert!(
            display_source.contains("take_next_live_toplevel_unmap_observation"),
            "Phase 53K display owner 必须暴露 live unmap observation drain seam"
        );
    }

    /// Phase 53L 必须证明 bounded loop 每轮处理 live unmap drain 并把进展计入非 idle。
    #[test]
    fn nested_runtime_loop_live_unmap_drain_source_exists() {
        let coordinator_source = include_str!("nested_runtime_coordinator.rs");
        let loop_source = include_str!("nested_runtime_loop.rs");

        for required in [
            "NestedRuntimeLiveAdmissionUnmapPumpReport",
            "pump_once_with_live_toplevel_admission_and_unmap_drain",
            "admission_drain_report",
            "unmap_drain_report",
        ] {
            assert!(
                coordinator_source.contains(required),
                "Phase 53L coordinator combined pump 缺少证据项: {required}"
            );
        }

        for required in [
            "NestedRuntimeLiveUnmapRunSummary",
            "live_unmap",
            "from_live_admission_unmap",
            "let live_unmap_has_progress = observed_report.live_unmap.has_progress();",
            "&& !live_admission_has_progress",
            "&& !live_unmap_has_progress",
            "nested_runtime_loop_drains_live_toplevel_unmap",
            "nested_runtime_loop_stop_when_idle_counts_live_unmap_progress",
        ] {
            assert!(
                loop_source.contains(required),
                "Phase 53L loop live-unmap proof 缺少证据项: {required}"
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
            "let observation = self.flow.take_next_live_toplevel_admission_observation();",
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

    /// Phase 53G 必须保留同一 coordinator/display 上连续 distinct callback admission 的证明。
    #[test]
    fn nested_runtime_multiple_live_admission_proof_source_exists() {
        let source = include_str!("nested_runtime_coordinator.rs");

        for required in [
            "fn nested_runtime_live_admission_pump_accepts_distinct_callback_observations()",
            "let first_registration =",
            "let second_registration =",
            "assert_ne!(",
            "first_registration.new_toplevel_callback_sequence",
            "second_registration.new_toplevel_callback_sequence",
            "first_registration.adapter_surface_id",
            "second_registration.adapter_surface_id",
            "first_registration.adapter_toplevel_id",
            "second_registration.adapter_toplevel_id",
            "Some(13_000)",
            "Some(13_001)",
            "next_core_surface_id_after",
            "coordinator.admission_next_core_surface_id(), 13_002",
            "state.surfaces.records().len(), 2",
        ] {
            assert!(
                source.contains(required),
                "Phase 53G multiple live admission proof 缺少证据项: {required}"
            );
        }
    }

    /// Phase 53H 必须让 live callback observation 以 FIFO backlog 进入 coordinator。
    #[test]
    fn nested_runtime_live_admission_backlog_source_uses_fifo_observations() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let display_source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_xdg_shell.rs"))
                .expect("Linux xdg-shell state owner 必须存在");
        let flow_source =
            std::fs::read_to_string(root.join("src/smithay_backend/real_accept_flow.rs"))
                .expect("Nested real accept flow 必须存在");
        let coordinator_source = include_str!("nested_runtime_coordinator.rs");

        for required in [
            "pending_live_toplevel_admission_observations",
            ".push_back(",
            ".pop_front()",
            "take_next_live_toplevel_admission_observation",
        ] {
            assert!(
                display_source.contains(required),
                "Phase 53H display owner 缺少 live observation backlog 证据项: {required}"
            );
        }

        for required in [
            "take_next_live_toplevel_admission_observation",
            "self.display.take_next_live_toplevel_admission_observation()",
        ] {
            assert!(
                flow_source.contains(required),
                "Phase 53H accept flow 缺少 FIFO observation seam 证据项: {required}"
            );
        }

        for required in [
            "self.flow.take_next_live_toplevel_admission_observation()",
            "nested_runtime_live_admission_pump_drains_backlogged_callback_observations",
        ] {
            assert!(
                coordinator_source.contains(required),
                "Phase 53H coordinator 缺少 FIFO observation pump 证据项: {required}"
            );
        }
    }

    /// Phase 53I 的 bounded loop idle 判断必须把 live admission progress 计入非 idle。
    #[test]
    fn nested_runtime_loop_idle_source_accounts_for_live_admission_progress() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let loop_source =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Nested runtime loop 必须存在");

        for required in [
            "fn has_progress(&self) -> bool",
            "let live_admission_has_progress = observed_report.live_admission.has_progress();",
            "let live_unmap_has_progress = observed_report.live_unmap.has_progress();",
            "&& !live_admission_has_progress",
            "&& !live_unmap_has_progress",
            "nested_runtime_loop_stop_when_idle_drains_live_admission_backlog",
        ] {
            assert!(
                loop_source.contains(required),
                "Phase 53I loop idle/backlog seam 缺少证据项: {required}"
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

    /// Phase 54A 只允许受控 wl_surface commit observation，不得进入 buffer/frame/render/core。
    #[test]
    fn controlled_wl_surface_commit_observation_source_stays_within_boundary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 54A controlled surface commit module 必须存在");
        let xdg_shell =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_xdg_shell.rs"))
                .expect("Phase 54A xdg shell handler 必须存在");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(production, _)| production);
        let code = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let code_without_phase54f_frame_proof =
            code.replace("let _callback = surface.frame(&queue_handle, ());", "");

        for required in [
            "AdapterSurfaceCommitObservation",
            "controlled_wl_surface_commit_observation_report",
            ".create_surface(&queue_handle, ())",
            "surface.commit()",
            "wl_surface_commit_attempted: true",
            "server_surface_commit_observed: true",
            "adapter_surface_commit_observation_available: true",
            "buffer_attached: false",
            "damage_submitted: false",
            "frame_callback_requested: false",
            "ledger_admit_invoked: false",
            "core_register_invoked: false",
            "render_support: false",
            "input_support: false",
        ] {
            assert!(
                code.contains(required),
                "Phase 54A controlled commit proof 缺少证据: {required}"
            );
        }

        for required in [
            "fn commit(&mut self, surface: &WlSurface)",
            ".observe_surface_commit(surface)",
            "不检查 buffer/damage",
            "不发 frame callback",
            "不触发 admission ledger/core",
        ] {
            assert!(
                xdg_shell.contains(required),
                "Phase 54A commit handler boundary 缺少证据: {required}"
            );
        }

        for forbidden in [
            ["Backend", "Event"].concat(),
            ["Core", "Runtime", "Bridge"].concat(),
            [".", "handle_command"].concat(),
            ["frame", "_callback_requested: true"].concat(),
            ".frame(".to_string(),
            ["buffer", "_attached: true"].concat(),
            ["render", "_support: true"].concat(),
            ["input", "_support: true"].concat(),
        ] {
            assert!(
                !code_without_phase54f_frame_proof.contains(&forbidden),
                "Phase 54A controlled commit proof 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54B 必须保留多次 wl_surface commit observation 的 FIFO backlog。
    #[test]
    fn controlled_wl_surface_commit_backlog_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 54B controlled surface commit module 必须存在");
        let display = std::fs::read_to_string(root.join("src/smithay_backend/wayland_display.rs"))
            .expect("Phase 54B display owner 必须存在");

        for required in [
            "VecDeque<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>>",
            "pending_commit_observations.push_back(result)",
            "take_next_commit_observation",
            "fn controlled_wl_surface_commit_observations_are_fifo_backlogged()",
            "first_pending.commit_sequence, 1",
            "second_pending.commit_sequence, 2",
            "server.take_next_wl_surface_commit_observation(), None",
        ] {
            assert!(
                source.contains(required),
                "Phase 54B commit backlog proof 缺少证据: {required}"
            );
        }

        for required in [
            "take_next_wl_surface_commit_observation",
            "self.state.take_next_wl_surface_commit_observation()",
        ] {
            assert!(
                display.contains(required),
                "Phase 54B display owner backlog seam 缺少证据: {required}"
            );
        }
    }

    /// Phase 54C 必须把 wl_surface commit backlog 接入 runtime/loop/orchestrator report drain seam。
    #[test]
    fn nested_runtime_surface_commit_backlog_drain_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let real_accept_flow =
            std::fs::read_to_string(root.join("src/smithay_backend/real_accept_flow.rs"))
                .expect("Phase 54C real accept flow source 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54C coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54C loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54C orchestrator source 必须存在");

        for required in [
            "take_next_wl_surface_commit_observation",
            "self.display.take_next_wl_surface_commit_observation()",
            "不读取 buffer",
            "不调用 render、input、ledger 或 core",
        ] {
            assert!(
                real_accept_flow.contains(required),
                "Phase 54C flow commit drain seam 缺少证据: {required}"
            );
        }

        for required in [
            "pub struct RuntimeSurfaceCommitDrainReport",
            "commit_observation_resolved",
            "commit_observation_failed",
            "commit_sequence: Option<u64>",
            "surface_commit_drain_report",
            "RuntimeSurfaceCommitDrainReport::from_observation",
            "self.flow.take_next_wl_surface_commit_observation()",
            "buffer_attached: false",
            "damage_submitted: false",
            "frame_callback_requested: false",
            "render_invoked: false",
            "input_invoked: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54C coordinator commit drain report 缺少证据: {required}"
            );
        }

        for required in [
            "pub struct NestedRuntimeSurfaceCommitRunSummary",
            "pub drained_commit_sequences: Vec<u64>",
            "from_surface_commit_drain",
            "let surface_commit_has_progress = observed_report.surface_commit.has_progress();",
            "&& !surface_commit_has_progress",
            "surface_commit: loop_report.surface_commit.clone()",
            "fn nested_runtime_loop_drains_wl_surface_commit_backlog_fifo_without_render()",
            "assert_eq!(report.surface_commit.commit_observations_drained, 2)",
            "assert_eq!(report.surface_commit.drained_commit_sequences, vec![1, 2])",
            "assert!(!report.surface_commit.render_invoked)",
            "assert!(!report.surface_commit.input_invoked)",
            "assert!(!report.surface_commit.core_mutation_invoked)",
        ] {
            let source = if required == "surface_commit: loop_report.surface_commit.clone()" {
                &orchestrator
            } else {
                &runtime_loop
            };
            assert!(
                source.contains(required),
                "Phase 54C loop/orchestrator commit drain proof 缺少证据: {required}"
            );
        }

        for required in [
            "pub surface_commit: NestedRuntimeSurfaceCommitRunSummary",
            "assert_eq!(report.surface_commit, report.loop_report.surface_commit)",
            "fn runtime_orchestrator_run_reports_wl_surface_commit_backlog_drain()",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54C orchestrator commit report proof 缺少证据: {required}"
            );
        }
    }

    /// Phase 54D 必须把 wl_surface commit 的 buffer presence / attach evidence 保留为纯数据。
    #[test]
    fn wl_surface_commit_buffer_presence_observation_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let surface_identity =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 54D controlled surface commit module 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54D coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54D loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54D orchestrator source 必须存在");

        for required in [
            "pub buffer_attach_observed: bool",
            "pub buffer_present: bool",
            "pub buffer_removed: bool",
            "pub renderable_buffer: bool",
            "Some(BufferAssignment::NewBuffer(_)) =>",
            "Some(BufferAssignment::Removed) =>",
            "controlled_wl_surface_null_attach_commit_observation_report",
            "surface.attach(None, 0, 0)",
            "assert!(report.buffer_attach_observed)",
            "assert!(!report.buffer_present)",
            "assert!(report.buffer_removed)",
            "assert!(!report.renderable_buffer)",
            "assert_eq!(first_pending.buffer_removed, true)",
            "assert_eq!(second_pending.buffer_removed, false)",
        ] {
            assert!(
                surface_identity.contains(required),
                "Phase 54D commit buffer presence proof 缺少证据: {required}"
            );
        }

        for required in [
            "report.buffer_attach_observed = commit.buffer_attach_observed",
            "report.buffer_present = commit.buffer_present",
            "report.buffer_removed = commit.buffer_removed",
            "report.renderable_buffer = commit.renderable_buffer",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54D coordinator buffer evidence drain 缺少证据: {required}"
            );
        }

        for required in [
            "pub buffer_attach_observations: usize",
            "pub buffer_presence_observations: usize",
            "pub buffer_removed_observations: usize",
            "pub renderable_buffer_observations: usize",
            "buffer_attach_observations: usize::from(report.buffer_attach_observed)",
            "buffer_presence_observations: usize::from(report.buffer_present)",
            "assert_eq!(report.surface_commit.buffer_attach_observations, 1)",
            "assert_eq!(report.surface_commit.buffer_presence_observations, 0)",
            "assert_eq!(report.surface_commit.buffer_removed_observations, 1)",
            "assert_eq!(report.surface_commit.renderable_buffer_observations, 0)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54D loop buffer evidence report 缺少证据: {required}"
            );
        }

        for required in [
            "assert_eq!(report.surface_commit.buffer_attach_observations, 1)",
            "assert_eq!(report.surface_commit.renderable_buffer_observations, 0)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54D orchestrator buffer evidence report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "BufferHandler for LinuxXdgShellStateSkeleton",
            "renderable_buffer: true",
            "render_invoked: true",
            "input_invoked: true",
            "frame_callback_requested: true",
            "damage_submitted: true",
        ] {
            assert!(
                !surface_identity.contains(forbidden)
                    && !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54D buffer presence seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54E 必须把 wl_surface damage / damage_buffer evidence 保留为纯数据。
    #[test]
    fn wl_surface_commit_damage_observation_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let surface_identity =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 54E controlled surface commit module 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54E coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54E loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54E orchestrator source 必须存在");

        for required in [
            "pub damage_observed: bool",
            "pub surface_damage_rects: usize",
            "pub buffer_damage_rects: usize",
            "Damage::Surface(_) =>",
            "Damage::Buffer(_) =>",
            "controlled_wl_surface_damage_commit_observation_report",
            "surface.damage_buffer(0, 0, 32, 24)",
            "assert!(report.damage_observed)",
            "assert_eq!(report.surface_damage_rects, 0)",
            "assert_eq!(report.buffer_damage_rects, 1)",
            "assert_eq!(first_pending.buffer_damage_rects, 1)",
            "assert_eq!(second_pending.buffer_damage_rects, 0)",
        ] {
            assert!(
                surface_identity.contains(required),
                "Phase 54E commit damage observation proof 缺少证据: {required}"
            );
        }

        for required in [
            "report.damage_observed = commit.damage_observed",
            "report.surface_damage_rects = commit.surface_damage_rects",
            "report.buffer_damage_rects = commit.buffer_damage_rects",
            "damage_submitted: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54E coordinator damage evidence drain 缺少证据: {required}"
            );
        }

        for required in [
            "pub damage_observations: usize",
            "pub surface_damage_rects: usize",
            "pub buffer_damage_rects: usize",
            "damage_observations: usize::from(report.damage_observed)",
            "surface_damage_rects: report.surface_damage_rects",
            "buffer_damage_rects: report.buffer_damage_rects",
            "assert_eq!(report.surface_commit.damage_observations, 1)",
            "assert_eq!(report.surface_commit.buffer_damage_rects, 1)",
            "assert!(!report.surface_commit.damage_submitted)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54E loop damage evidence report 缺少证据: {required}"
            );
        }

        for required in [
            "assert_eq!(report.surface_commit.damage_observations, 1)",
            "assert_eq!(report.surface_commit.buffer_damage_rects, 1)",
            "assert!(!report.surface_commit.damage_submitted)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54E orchestrator damage evidence report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "DamageHandler for LinuxXdgShellStateSkeleton",
            "damage_submitted: true",
            "render_invoked: true",
            "input_invoked: true",
            "frame_callback_requested: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !surface_identity.contains(forbidden)
                    && !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54E damage observation seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54F 必须把 wl_surface frame callback request 保留为纯数据 observation。
    #[test]
    fn wl_surface_commit_frame_callback_observation_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let surface_identity =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 54F controlled surface commit module 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54F coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54F loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54F orchestrator source 必须存在");

        for required in [
            "pub frame_callback_observed: bool",
            "pub frame_callback_count: usize",
            "guard.current().frame_callbacks.len()",
            "controlled_wl_surface_frame_callback_commit_observation_report",
            "let _callback = surface.frame(&queue_handle, ())",
            "assert!(report.frame_callback_observed)",
            "assert_eq!(report.frame_callback_count, 1)",
            "assert_eq!(first_pending.frame_callback_count, 1)",
            "assert_eq!(second_pending.frame_callback_count, 0)",
            "assert!(!report.frame_callback_requested)",
        ] {
            assert!(
                surface_identity.contains(required),
                "Phase 54F commit frame callback observation proof 缺少证据: {required}"
            );
        }

        for required in [
            "report.frame_callback_observed = commit.frame_callback_observed",
            "report.frame_callback_count = commit.frame_callback_count",
            "frame_callback_requested: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54F coordinator frame callback evidence drain 缺少证据: {required}"
            );
        }

        for required in [
            "pub frame_callback_observations: usize",
            "pub frame_callback_count: usize",
            "frame_callback_observations: usize::from(report.frame_callback_observed)",
            "frame_callback_count: report.frame_callback_count",
            "assert_eq!(report.surface_commit.frame_callback_observations, 1)",
            "assert_eq!(report.surface_commit.frame_callback_count, 1)",
            "assert!(!report.surface_commit.frame_callback_requested)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54F loop frame callback evidence report 缺少证据: {required}"
            );
        }

        for required in [
            "assert_eq!(report.surface_commit.frame_callback_observations, 1)",
            "assert_eq!(report.surface_commit.frame_callback_count, 1)",
            "assert!(!report.surface_commit.frame_callback_requested)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54F orchestrator frame callback evidence report 缺少证据: {required}"
            );
        }

        for forbidden in [
            ".done(",
            "frame_callback_requested: true",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !surface_identity.contains(forbidden)
                    && !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54F frame callback observation seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54G 必须把 commit evidence 汇总为 render-dirty/readiness intent 纯数据。
    #[test]
    fn wl_surface_commit_render_dirty_readiness_intent_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let surface_identity =
            std::fs::read_to_string(root.join("src/smithay_backend/linux_wl_surface_identity.rs"))
                .expect("Phase 54G controlled surface commit module 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54G coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54G loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54G orchestrator source 必须存在");

        for required in [
            "controlled_wl_surface_render_dirty_readiness_commit_observation_report",
            "controlled_wl_surface_commit_observation_report_with_options(server, true, true, true)",
            "assert!(report.buffer_attach_observed)",
            "assert!(report.buffer_removed)",
            "assert!(report.damage_observed)",
            "assert_eq!(report.buffer_damage_rects, 1)",
            "assert!(report.frame_callback_observed)",
            "assert_eq!(report.frame_callback_count, 1)",
        ] {
            assert!(
                surface_identity.contains(required),
                "Phase 54G commit evidence source proof 缺少证据: {required}"
            );
        }

        for required in [
            "pub struct RuntimeSurfaceCommitRenderDirtyReadinessIntent",
            "pub adapter_surface_id: AdapterSurfaceId",
            "pub commit_sequence: u64",
            "pub buffer_attach_observed: bool",
            "pub buffer_present: bool",
            "pub buffer_removed: bool",
            "pub renderable_buffer: bool",
            "pub damage_observed: bool",
            "pub surface_damage_rects: usize",
            "pub buffer_damage_rects: usize",
            "pub frame_callback_observed: bool",
            "pub frame_callback_count: usize",
            "pub buffer_imported: bool",
            "pub render_submitted: bool",
            "pub frame_callback_done_sent: bool",
            "pub input_support: bool",
            "pub fn render_dirty_readiness_intent_from_commit_drain_report",
            "buffer_imported: false",
            "render_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54G coordinator render-dirty intent 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderDirtyReadinessIntent",
            "pub render_dirty_readiness_intents: Vec<RuntimeSurfaceCommitRenderDirtyReadinessIntent>",
            "render_dirty_readiness_intent_from_commit_drain_report",
            "self.render_dirty_readiness_intents",
            "render_dirty_readiness_intents.len()",
            "assert_eq!(first_intent.commit_sequence, first_commit.commit_sequence)",
            "assert_eq!(second_intent.commit_sequence, second_commit.commit_sequence)",
            "assert!(first_intent.buffer_attach_observed)",
            "assert!(first_intent.damage_observed)",
            "assert_eq!(first_intent.frame_callback_count, 1)",
            "assert!(!first_intent.buffer_imported)",
            "assert!(!first_intent.render_submitted)",
            "assert!(!first_intent.frame_callback_done_sent)",
            "assert!(!first_intent.input_support)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54G loop render-dirty intent report 缺少证据: {required}"
            );
        }

        for required in [
            "render_dirty_readiness_intents.len()",
            "assert_eq!(first_intent.commit_sequence, first_commit.commit_sequence)",
            "assert!(!first_intent.render_submitted)",
            "assert!(!first_intent.buffer_imported)",
            "assert!(!first_intent.frame_callback_done_sent)",
            "assert!(!first_intent.input_support)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54G orchestrator render-dirty intent report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "texture_created: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !surface_identity.contains(forbidden)
                    && !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54G render-dirty readiness seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54H 必须把 render-dirty/readiness intent 接入 runtime-owned FIFO queue。
    #[test]
    fn wl_surface_render_dirty_intent_runtime_queue_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54H coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54H loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54H orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderDirtyIntentQueueOwner",
            "render_dirty_intent_queue_owner: RuntimeSurfaceCommitRenderDirtyIntentQueueOwner",
            "pub struct RuntimeSurfaceCommitRenderDirtyIntentDrainReport",
            "pub enum RuntimeSurfaceCommitRenderDirtyIntentQueueOperation",
            "pub enum RuntimeSurfaceCommitRenderDirtyIntentQueueBlocker",
            "pub fn pending_count(&self) -> usize",
            "pub fn enqueue_from_commit_drain_and_drain_once",
            "render_dirty_readiness_intent_from_commit_drain_report(report)",
            "pending_intent_count_before_enqueue",
            "pending_intent_count_after_enqueue",
            "pending_intent_count_before_drain",
            "pending_intent_count_after_drain",
            "pub drained_intent: Option<RuntimeSurfaceCommitRenderDirtyReadinessIntent>",
            "buffer_imported: false",
            "texture_created: false",
            "render_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54H coordinator render-dirty queue 缺少证据: {required}"
            );
        }

        for required in [
            "pub render_dirty_queue_drain_invocations: usize",
            "pub render_dirty_intents_enqueued: usize",
            "pub render_dirty_intents_drained: usize",
            "pub render_dirty_queue_drained_intents: Vec<RuntimeSurfaceCommitRenderDirtyReadinessIntent>",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_dirty_intent_drain",
            "report.render_dirty_intent_drain_report",
            "self.render_dirty_queue_drained_intents",
            "assert_eq!(report.surface_commit.render_dirty_intents_enqueued, 2)",
            "assert_eq!(report.surface_commit.render_dirty_intents_drained, 2)",
            "assert_eq!(first_drained.commit_sequence, first_commit.commit_sequence)",
            "second_drained.commit_sequence",
            "assert!(!first_drained.render_submitted)",
            "assert!(!first_drained.buffer_imported)",
            "assert!(!first_drained.frame_callback_done_sent)",
            "assert!(!first_drained.input_support)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54H loop render-dirty queue report 缺少证据: {required}"
            );
        }

        for required in [
            "assert_eq!(report.surface_commit.render_dirty_intents_enqueued, 2)",
            "assert_eq!(report.surface_commit.render_dirty_intents_drained, 2)",
            "assert_eq!(first_drained.commit_sequence, first_commit.commit_sequence)",
            "second_drained.commit_sequence",
            "assert!(!first_drained.render_submitted)",
            "assert!(!first_drained.buffer_imported)",
            "assert!(!first_drained.frame_callback_done_sent)",
            "assert!(!first_drained.input_support)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54H orchestrator render-dirty queue report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54H render-dirty queue seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54I 必须从 render-dirty queue drain 派生 renderer-admission 纯数据 work intent。
    #[test]
    fn wl_surface_render_dirty_renderer_admission_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54I coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54I loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54I orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRendererAdmissionWorkIntent",
            "pub struct RuntimeSurfaceCommitRendererAdmissionReport",
            "pub enum RuntimeSurfaceCommitRendererAdmissionOperation",
            "pub enum RuntimeSurfaceCommitRendererAdmissionBlocker",
            "pub fn renderer_admission_report_from_render_dirty_intent_drain",
            "source_render_dirty_intent_drained",
            "pub work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub adapter_surface_id: AdapterSurfaceId",
            "pub commit_sequence: u64",
            "pub buffer_attach_observed: bool",
            "pub damage_observed: bool",
            "pub frame_callback_count: usize",
            "buffer_imported: false",
            "texture_created: false",
            "render_submitted: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54I coordinator renderer-admission seam 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRendererAdmissionWorkIntent",
            "pub renderer_admission_invocations: usize",
            "pub renderer_work_intents_created: usize",
            "pub renderer_work_intents: Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "NestedRuntimeSurfaceCommitRunSummary::from_renderer_admission",
            "report.renderer_admission_report",
            "self.renderer_work_intents",
            "assert_eq!(report.surface_commit.renderer_work_intents_created, 2)",
            "assert_eq!(first_work.commit_sequence, first_commit.commit_sequence)",
            "assert_eq!(second_work.commit_sequence, second_commit.commit_sequence)",
            "assert!(first_work.buffer_attach_observed)",
            "assert!(first_work.damage_observed)",
            "assert_eq!(first_work.frame_callback_count, 1)",
            "assert!(!first_work.render_submitted)",
            "assert!(!first_work.buffer_imported)",
            "assert!(!first_work.frame_callback_done_sent)",
            "assert!(!first_work.input_support)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54I loop renderer-admission report 缺少证据: {required}"
            );
        }

        for required in [
            "assert_eq!(report.surface_commit.renderer_work_intents_created, 2)",
            "assert_eq!(first_work.commit_sequence, first_commit.commit_sequence)",
            "assert_eq!(second_work.commit_sequence, second_commit.commit_sequence)",
            "assert!(!first_work.render_submitted)",
            "assert!(!first_work.buffer_imported)",
            "assert!(!first_work.frame_callback_done_sent)",
            "assert!(!first_work.input_support)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54I orchestrator renderer-admission report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54I renderer-admission seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54J 必须建立 renderer-admission work intent consumer / owner boundary。
    #[test]
    fn wl_surface_renderer_admission_owner_boundary_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54J coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54J loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54J orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRendererAdmissionOwner",
            "renderer_admission_owner: RuntimeSurfaceCommitRendererAdmissionOwner",
            "pub struct RuntimeSurfaceCommitRendererOwnerBoundaryReport",
            "pub enum RuntimeSurfaceCommitRendererOwnerBoundaryOperation",
            "pub enum RuntimeSurfaceCommitRendererOwnerBoundaryBlocker",
            "MissingRendererOwner",
            "MissingBufferImporter",
            "MissingTextureSupport",
            "pub fn consume_renderer_admission_work_intent",
            "pub consumed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub renderer_owner_available: bool",
            "pub buffer_importer_available: bool",
            "pub texture_support_available: bool",
            "pub renderer_called: bool",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54J coordinator renderer owner boundary 缺少证据: {required}"
            );
        }

        for required in [
            "pub renderer_owner_boundary_invocations: usize",
            "pub renderer_owner_work_intents_consumed: usize",
            "pub renderer_owner_consumed_work_intents: Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub renderer_owner_missing_renderer_owner: bool",
            "pub renderer_owner_missing_buffer_importer: bool",
            "pub renderer_owner_missing_texture_support: bool",
            "pub renderer_owner_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_renderer_owner_boundary",
            "report.renderer_owner_boundary_report",
            "let consumed_count = report.surface_commit.renderer_owner_work_intents_consumed",
            "assert_eq!(consumed_count, 2)",
            "assert_eq!(first_consumed.commit_sequence, first_commit.commit_sequence)",
            "let second_sequence = second_consumed.commit_sequence",
            "assert_eq!(second_sequence, second_commit.commit_sequence)",
            "assert!(first_consumed.buffer_attach_observed)",
            "assert!(first_consumed.damage_observed)",
            "assert_eq!(first_consumed.frame_callback_count, 1)",
            "assert!(report.surface_commit.renderer_owner_missing_renderer_owner)",
            "assert!(report.surface_commit.renderer_owner_missing_buffer_importer)",
            "assert!(report.surface_commit.renderer_owner_missing_texture_support)",
            "assert!(!report.surface_commit.renderer_owner_renderer_called)",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54J loop renderer owner boundary report 缺少证据: {required}"
            );
        }

        for required in [
            "let consumed_count = report.surface_commit.renderer_owner_work_intents_consumed",
            "assert_eq!(consumed_count, 2)",
            "assert_eq!(first_consumed.commit_sequence, first_commit.commit_sequence)",
            "let second_sequence = second_consumed.commit_sequence",
            "assert_eq!(second_sequence, second_commit.commit_sequence)",
            "assert!(report.surface_commit.renderer_owner_missing_renderer_owner)",
            "assert!(report.surface_commit.renderer_owner_missing_buffer_importer)",
            "assert!(report.surface_commit.renderer_owner_missing_texture_support)",
            "assert!(!report.surface_commit.renderer_owner_renderer_called)",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54J orchestrator renderer owner boundary report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54J renderer owner boundary seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54K 必须建立 runtime-owned renderer owner shell readiness seam。
    #[test]
    fn wl_surface_renderer_owner_shell_readiness_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54K coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54K loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54K orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRendererOwnerShell",
            "renderer_owner_shell: RuntimeSurfaceCommitRendererOwnerShell",
            "pub struct RuntimeSurfaceCommitRendererOwnerShellReadinessReport",
            "pub enum RuntimeSurfaceCommitRendererOwnerShellOperation",
            "pub enum RuntimeSurfaceCommitRendererOwnerShellBlocker",
            "pub fn renderer_owner_shell_readiness_from_owner_boundary",
            "pub owner_boundary_report_observed: bool",
            "pub observed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub renderer_owner_shell_available: bool",
            "renderer_owner_shell_available: true",
            "MissingBufferImporter",
            "MissingTextureSupport",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54K coordinator renderer owner shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "pub renderer_owner_shell_readiness_invocations: usize",
            "pub renderer_owner_shell_work_intents_observed: usize",
            "pub renderer_owner_shell_observed_work_intents:",
            "Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub renderer_owner_shell_available: bool",
            "pub renderer_owner_shell_missing_renderer_owner: bool",
            "pub renderer_owner_shell_missing_buffer_importer: bool",
            "pub renderer_owner_shell_missing_texture_support: bool",
            "pub renderer_owner_shell_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_renderer_owner_shell_readiness",
            "report.renderer_owner_shell_readiness_report",
            "renderer_owner_shell_work_intents_observed",
            "renderer_owner_shell_available",
            "renderer_owner_shell_missing_renderer_owner",
            "renderer_owner_shell_missing_buffer_importer",
            "renderer_owner_shell_missing_texture_support",
            "assert_eq!(first_shell.commit_sequence, first_commit.commit_sequence)",
            "assert_eq!(second_shell.commit_sequence, second_commit.commit_sequence)",
            "renderer_owner_shell_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54K loop renderer owner shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "renderer_owner_shell_work_intents_observed",
            "renderer_owner_shell_available",
            "renderer_owner_shell_missing_renderer_owner",
            "renderer_owner_shell_missing_buffer_importer",
            "renderer_owner_shell_missing_texture_support",
            "assert_eq!(first_shell.commit_sequence, first_commit.commit_sequence)",
            "assert_eq!(second_shell.commit_sequence, second_commit.commit_sequence)",
            "renderer_owner_shell_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54K orchestrator renderer owner shell readiness 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54K renderer owner shell readiness seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54L 必须建立 runtime-owned buffer importer shell readiness seam。
    #[test]
    fn wl_surface_buffer_importer_shell_readiness_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54L coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54L loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54L orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImporterShell",
            "buffer_importer_shell: RuntimeSurfaceCommitBufferImporterShell",
            "pub struct RuntimeSurfaceCommitBufferImporterShellReadinessReport",
            "pub enum RuntimeSurfaceCommitBufferImporterShellOperation",
            "pub enum RuntimeSurfaceCommitBufferImporterShellBlocker",
            "pub fn buffer_importer_shell_readiness_from_renderer_owner_shell",
            "pub renderer_owner_shell_report_observed: bool",
            "pub observed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub buffer_importer_shell_available: bool",
            "buffer_importer_shell_available: true",
            "pub buffer_importer_available: bool",
            "buffer_importer_available: true",
            "MissingTextureSupport",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54L coordinator buffer importer shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImporterShellReadinessReport",
            "pub buffer_importer_shell_readiness_invocations: usize",
            "pub buffer_importer_shell_work_intents_observed: usize",
            "pub buffer_importer_shell_observed_work_intents:",
            "Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub buffer_importer_shell_available: bool",
            "pub buffer_importer_shell_missing_renderer_owner_shell: bool",
            "pub buffer_importer_shell_missing_buffer_importer: bool",
            "pub buffer_importer_shell_missing_texture_support: bool",
            "pub buffer_importer_shell_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_importer_shell_readiness",
            "report.buffer_importer_shell_readiness_report",
            "buffer_importer_shell_work_intents_observed",
            "buffer_importer_shell_available",
            "buffer_importer_shell_missing_renderer_owner_shell",
            "buffer_importer_shell_missing_buffer_importer",
            "buffer_importer_shell_missing_texture_support",
            "first_importer.commit_sequence",
            "second_importer.commit_sequence",
            "buffer_importer_shell_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54L loop buffer importer shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_importer_shell_work_intents_observed",
            "buffer_importer_shell_available",
            "buffer_importer_shell_missing_renderer_owner_shell",
            "buffer_importer_shell_missing_buffer_importer",
            "buffer_importer_shell_missing_texture_support",
            "first_importer.commit_sequence",
            "second_importer.commit_sequence",
            "buffer_importer_shell_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54L orchestrator buffer importer shell readiness 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54L buffer importer shell readiness seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54M 必须建立 runtime-owned texture support shell readiness seam。
    #[test]
    fn wl_surface_texture_support_shell_readiness_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54M coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54M loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54M orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitTextureSupportShell",
            "texture_support_shell: RuntimeSurfaceCommitTextureSupportShell",
            "pub struct RuntimeSurfaceCommitTextureSupportShellReadinessReport",
            "pub enum RuntimeSurfaceCommitTextureSupportShellOperation",
            "pub enum RuntimeSurfaceCommitTextureSupportShellBlocker",
            "pub fn texture_support_shell_readiness_from_buffer_importer_shell",
            "pub buffer_importer_shell_report_observed: bool",
            "pub observed_work_intent: Option<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub texture_support_shell_available: bool",
            "texture_support_shell_available: true",
            "pub texture_support_available: bool",
            "texture_support_available: true",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54M coordinator texture support shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitTextureSupportShellReadinessReport",
            "pub texture_support_shell_readiness_invocations: usize",
            "pub texture_support_shell_work_intents_observed: usize",
            "pub texture_support_shell_observed_work_intents:",
            "Vec<RuntimeSurfaceCommitRendererAdmissionWorkIntent>",
            "pub texture_support_shell_available: bool",
            "pub texture_support_shell_missing_buffer_importer_shell: bool",
            "pub texture_support_shell_missing_texture_support: bool",
            "pub texture_support_shell_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_texture_support_shell_readiness",
            "report.texture_support_shell_readiness_report",
            "texture_support_shell_work_intents_observed",
            "texture_support_shell_available",
            "texture_support_shell_missing_buffer_importer_shell",
            "texture_support_shell_missing_texture_support",
            "first_texture.commit_sequence",
            "second_texture.commit_sequence",
            "texture_support_shell_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54M loop texture support shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "texture_support_shell_work_intents_observed",
            "texture_support_shell_available",
            "texture_support_shell_missing_buffer_importer_shell",
            "texture_support_shell_missing_texture_support",
            "first_texture.commit_sequence",
            "second_texture.commit_sequence",
            "texture_support_shell_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54M orchestrator texture support shell readiness 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54M texture support shell readiness seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54N 必须从 texture support shell readiness 派生 render operation 纯数据 intent。
    #[test]
    fn wl_surface_render_operation_intent_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54N coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54N loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54N orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderOperationIntent",
            "pub struct RuntimeSurfaceCommitRenderOperationReadinessReport",
            "pub enum RuntimeSurfaceCommitRenderOperationOperation",
            "pub enum RuntimeSurfaceCommitRenderOperationBlocker",
            "pub fn render_operation_readiness_from_texture_support_shell",
            "pub source_texture_support_shell_report_observed: bool",
            "pub render_operation_intent_created: bool",
            "pub render_operation_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub adapter_surface_id: AdapterSurfaceId",
            "pub commit_sequence: u64",
            "pub buffer_attach_observed: bool",
            "pub damage_rect_count: usize",
            "pub frame_callback_count: usize",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54N coordinator render operation intent 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderOperationReadinessReport",
            "pub render_operation_readiness_invocations: usize",
            "pub render_operation_intents_created: usize",
            "pub render_operation_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub render_operation_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_operation_readiness",
            "report.render_operation_readiness_report",
            "render_operation_intents_created",
            "first_render_operation.commit_sequence",
            "second_render_operation.commit_sequence",
            "first_render_operation.damage_rect_count",
            "render_operation_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54N loop render operation intent report 缺少证据: {required}"
            );
        }

        for required in [
            "render_operation_intents_created",
            "first_render_operation.commit_sequence",
            "second_render_operation.commit_sequence",
            "first_render_operation.damage_rect_count",
            "render_operation_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54N orchestrator render operation intent report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54N render operation intent seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54O 必须把 render operation intent 接入 runtime-owned FIFO queue。
    #[test]
    fn wl_surface_render_operation_intent_runtime_queue_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54O coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54O loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54O orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderOperationIntentQueueOwner",
            "render_operation_intent_queue_owner: RuntimeSurfaceCommitRenderOperationIntentQueueOwner",
            "pub struct RuntimeSurfaceCommitRenderOperationIntentDrainReport",
            "pub enum RuntimeSurfaceCommitRenderOperationIntentQueueOperation",
            "pub enum RuntimeSurfaceCommitRenderOperationIntentQueueBlocker",
            "pub fn enqueue_from_render_operation_readiness_and_drain_once",
            "pub drained_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub runtime_queue_owned: bool",
            "pub intent_enqueued: bool",
            "pub intent_drained: bool",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54O coordinator render operation runtime queue 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderOperationIntentDrainReport",
            "pub render_operation_queue_drain_invocations: usize",
            "pub render_operation_intents_enqueued: usize",
            "pub render_operation_intents_drained: usize",
            "pub render_operation_queue_drained_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub render_operation_queue_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_operation_intent_drain",
            "report.render_operation_intent_drain_report",
            "render_operation_intents_enqueued",
            "render_operation_intents_drained",
            "first_render_operation_drained.commit_sequence",
            "second_render_operation_drained.commit_sequence",
            "render_operation_queue_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54O loop render operation runtime queue report 缺少证据: {required}"
            );
        }

        for required in [
            "render_operation_intents_enqueued",
            "render_operation_intents_drained",
            "first_render_operation_drained.commit_sequence",
            "second_render_operation_drained.commit_sequence",
            "render_operation_queue_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54O orchestrator render operation runtime queue report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54O render operation runtime queue seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54P 必须消费 render operation intent 并生成 render execution owner boundary report。
    #[test]
    fn wl_surface_render_execution_owner_boundary_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54P coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54P loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54P orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderExecutionOwnerBoundary",
            "render_execution_owner_boundary: RuntimeSurfaceCommitRenderExecutionOwnerBoundary",
            "pub struct RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport",
            "pub enum RuntimeSurfaceCommitRenderExecutionOwnerBoundaryOperation",
            "pub enum RuntimeSurfaceCommitRenderExecutionOwnerBoundaryBlocker",
            "pub fn consume_render_operation_intent",
            "pub consumed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub render_operation_intent_consumed: bool",
            "pub render_execution_owner_available: bool",
            "pub buffer_imported: bool",
            "pub texture_created: bool",
            "pub renderer_called: bool",
            "pub damage_submitted: bool",
            "pub frame_callback_done_sent: bool",
            "pub input_support: bool",
            "pub core_mutation_invoked: bool",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54P coordinator render execution owner boundary 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderExecutionOwnerBoundaryReport",
            "pub render_execution_owner_boundary_invocations: usize",
            "pub render_execution_owner_intents_consumed: usize",
            "pub render_execution_owner_consumed_intents: Vec<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub render_execution_owner_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_execution_owner_boundary",
            "report.render_execution_owner_boundary_report",
            "render_execution_owner_intents_consumed",
            "first_render_execution.commit_sequence",
            "second_render_execution.commit_sequence",
            "render_execution_owner_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54P loop render execution owner boundary report 缺少证据: {required}"
            );
        }

        for required in [
            "render_execution_owner_intents_consumed",
            "first_render_execution.commit_sequence",
            "second_render_execution.commit_sequence",
            "render_execution_owner_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54P orchestrator render execution owner boundary report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54P render execution owner boundary seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54Q 必须从 render execution owner boundary 派生 shell readiness report。
    #[test]
    fn wl_surface_render_execution_owner_shell_readiness_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54Q coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54Q loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54Q orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderExecutionOwnerShell",
            "render_execution_owner_shell: RuntimeSurfaceCommitRenderExecutionOwnerShell",
            "pub struct RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport",
            "pub enum RuntimeSurfaceCommitRenderExecutionOwnerShellOperation",
            "pub enum RuntimeSurfaceCommitRenderExecutionOwnerShellBlocker",
            "pub fn render_execution_owner_shell_readiness_from_owner_boundary",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub render_execution_owner_shell_available: bool",
            "render_execution_owner_shell_available: true",
            "pub buffer_imported: bool",
            "pub texture_created: bool",
            "pub renderer_called: bool",
            "pub damage_submitted: bool",
            "pub frame_callback_done_sent: bool",
            "pub input_support: bool",
            "pub core_mutation_invoked: bool",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 54Q coordinator render execution owner shell readiness 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderExecutionOwnerShellReadinessReport",
            "pub render_execution_owner_shell_readiness_invocations: usize",
            "pub render_execution_owner_shell_intents_observed: usize",
            "pub render_execution_owner_shell_observed_intents:",
            "pub render_execution_owner_shell_available: bool",
            "pub render_execution_owner_shell_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_execution_owner_shell_readiness",
            "report.render_execution_owner_shell_readiness_report",
            "render_execution_owner_shell_intents_observed",
            "first_render_execution_shell.commit_sequence",
            "second_render_execution_shell.commit_sequence",
            "render_execution_owner_shell_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 54Q loop render execution owner shell readiness report 缺少证据: {required}"
            );
        }

        for required in [
            "render_execution_owner_shell_intents_observed",
            "render_execution_owner_shell_available",
            "first_render_execution_shell.commit_sequence",
            "second_render_execution_shell.commit_sequence",
            "render_execution_owner_shell_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 54Q orchestrator render execution owner shell readiness report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54Q render execution owner shell readiness seam 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 54R 必须把 Phase 54G-54Q render 前置链路整理成 Phase 55A 前的审计文档。
    #[test]
    fn render_pipeline_readiness_audit_doc_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let audit = std::fs::read_to_string(
            root.join("docs/phases/PHASE_54R_RENDER_PIPELINE_READINESS_AUDIT.md"),
        )
        .expect("Phase 54R render pipeline readiness audit doc 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 54R coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 54R loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 54R orchestrator source 必须存在");

        for required in [
            "Phase 54R - Render Pipeline Readiness Audit",
            "Phase 54G",
            "Phase 54H",
            "Phase 54I",
            "Phase 54J",
            "Phase 54K",
            "Phase 54L",
            "Phase 54M",
            "Phase 54N",
            "Phase 54O",
            "Phase 54P",
            "Phase 54Q",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
            "real renderer owner",
            "buffer importer implementation",
            "texture creation path",
            "damage submit path",
            "frame callback done path",
            "Phase 55A minimal safe entry point",
            "Do not import buffer in Phase 54R",
            "Do not create texture in Phase 54R",
            "Do not call renderer in Phase 54R",
        ] {
            assert!(
                audit.contains(required),
                "Phase 54R audit doc 缺少必需内容: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 54R readiness audit 发现 render 前置链路包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 55A 必须建立 basic render pipeline skeleton / renderer owner skeleton。
    #[test]
    fn basic_render_pipeline_skeleton_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55A coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55A loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55A orchestrator source 必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderPipelineSkeletonOwner",
            "render_pipeline_skeleton_owner: RuntimeSurfaceCommitRenderPipelineSkeletonOwner",
            "pub struct RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport",
            "pub enum RuntimeSurfaceCommitRenderPipelineSkeletonOperation",
            "pub enum RuntimeSurfaceCommitRenderPipelineSkeletonBlocker",
            "pub fn render_pipeline_skeleton_readiness_from_execution_owner_shell",
            "pub source_render_execution_owner_shell_report_observed: bool",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub renderer_pipeline_owner_available: bool",
            "renderer_pipeline_owner_available: true",
            "pub buffer_imported: bool",
            "pub texture_created: bool",
            "pub renderer_called: bool",
            "pub damage_submitted: bool",
            "pub frame_callback_done_sent: bool",
            "pub input_support: bool",
            "pub core_mutation_invoked: bool",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55A coordinator render pipeline skeleton 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderPipelineSkeletonReadinessReport",
            "pub render_pipeline_skeleton_readiness_invocations: usize",
            "pub render_pipeline_skeleton_intents_observed: usize",
            "pub render_pipeline_skeleton_observed_intents:",
            "pub render_pipeline_skeleton_owner_available: bool",
            "pub render_pipeline_skeleton_renderer_called: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_pipeline_skeleton_readiness",
            "report.render_pipeline_skeleton_readiness_report",
            "render_pipeline_skeleton_intents_observed",
            "first_render_pipeline_skeleton.commit_sequence",
            "second_render_pipeline_skeleton.commit_sequence",
            "render_pipeline_skeleton_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55A loop render pipeline skeleton report 缺少证据: {required}"
            );
        }

        for required in [
            "render_pipeline_skeleton_intents_observed",
            "render_pipeline_skeleton_owner_available",
            "first_render_pipeline_skeleton.commit_sequence",
            "second_render_pipeline_skeleton.commit_sequence",
            "render_pipeline_skeleton_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55A orchestrator render pipeline skeleton report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55A render pipeline skeleton 包含禁止 token: {forbidden}"
            );
        }
    }

    /// Phase 55B 必须建立 render backend capability report seam。
    #[test]
    fn render_backend_capability_report_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55B coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55B loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55B orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55B_RENDER_BACKEND_CAPABILITY_REPORT.md"),
        )
        .expect("Phase 55B 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRenderBackendCapabilityOwner",
            "render_backend_capability_owner: RuntimeSurfaceCommitRenderBackendCapabilityOwner",
            "pub struct RuntimeSurfaceCommitRenderBackendCapabilityReport",
            "pub enum RuntimeSurfaceCommitRenderBackendCapabilityOperation",
            "pub enum RuntimeSurfaceCommitRenderBackendCapabilityBlocker",
            "pub fn render_backend_capability_report_from_pipeline_skeleton",
            "pub source_render_pipeline_skeleton_report_observed: bool",
            "pub source_renderer_pipeline_owner_available: bool",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub render_backend_capability_owner_available: bool",
            "pub renderer_backend_registered: bool",
            "pub renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>",
            "renderer_backend_registered: false",
            "renderer_backend_kind: None",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55B coordinator render backend capability 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRenderBackendCapabilityReport",
            "pub render_backend_capability_report_invocations: usize",
            "pub render_backend_capability_intents_observed: usize",
            "pub render_backend_capability_observed_intents:",
            "pub render_backend_capability_owner_available: bool",
            "pub render_backend_capability_backend_registered: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_render_backend_capability_report",
            "report.render_backend_capability_report",
            "render_backend_capability_intents_observed",
            "first_render_backend_capability.commit_sequence",
            "second_render_backend_capability.commit_sequence",
            "render_backend_capability_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55B loop render backend capability report 缺少证据: {required}"
            );
        }

        for required in [
            "render_backend_capability_intents_observed",
            "render_backend_capability_owner_available",
            "render_backend_capability_backend_registered",
            "first_render_backend_capability.commit_sequence",
            "second_render_backend_capability.commit_sequence",
            "render_backend_capability_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55B orchestrator render backend capability report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55B render backend capability 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "renderer_backend_registered = false",
            "renderer_backend_kind = None",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55B doc 缺少 capability truth: {required}"
            );
        }
    }

    /// Phase 55C 必须建立 renderer backend registration descriptor seam。
    #[test]
    fn renderer_backend_registration_descriptor_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55C coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55C loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55C orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55C_RENDERER_BACKEND_REGISTRATION_DESCRIPTOR.md"),
        )
        .expect("Phase 55C 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRendererBackendRegistrationOwner",
            "renderer_backend_registration_owner: RuntimeSurfaceCommitRendererBackendRegistrationOwner",
            "pub struct RuntimeSurfaceCommitRendererBackendRegistrationReport",
            "pub enum RuntimeSurfaceCommitRendererBackendRegistrationOperation",
            "pub enum RuntimeSurfaceCommitRendererBackendRegistrationBlocker",
            "pub fn renderer_backend_registration_report_from_backend_capability",
            "pub source_render_backend_capability_report_observed: bool",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub renderer_backend_registration_owner_available: bool",
            "pub renderer_backend_descriptor_available: bool",
            "pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>",
            "renderer_backend_registered: true",
            "registered_renderer_backend_kind: Some(RuntimeSurfaceCommitRenderBackendKind::SmithayLinux)",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55C coordinator renderer backend registration 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRendererBackendRegistrationReport",
            "pub renderer_backend_registration_invocations: usize",
            "pub renderer_backend_registration_intents_observed: usize",
            "pub renderer_backend_registration_observed_intents:",
            "pub renderer_backend_registration_owner_available: bool",
            "pub renderer_backend_registration_backend_registered: bool",
            "pub renderer_backend_registration_descriptor_available: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_renderer_backend_registration_report",
            "report.renderer_backend_registration_report",
            "first_renderer_backend_registration.commit_sequence",
            "second_renderer_backend_registration.commit_sequence",
            "renderer_backend_registration_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55C loop renderer backend registration report 缺少证据: {required}"
            );
        }

        for required in [
            "renderer_backend_registration_intents_observed",
            "renderer_backend_registration_owner_available",
            "renderer_backend_registration_backend_registered",
            "renderer_backend_registration_descriptor_available",
            "first_renderer_backend_registration.commit_sequence",
            "second_renderer_backend_registration.commit_sequence",
            "renderer_backend_registration_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55C orchestrator renderer backend registration report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55C renderer backend registration 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "renderer_backend_registered = true",
            "registered_renderer_backend_kind = Some(SmithayLinux)",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55C doc 缺少 capability truth: {required}"
            );
        }
    }

    /// Phase 55D 必须建立 renderer backend owner shell readiness seam。
    #[test]
    fn renderer_backend_owner_shell_readiness_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55D coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55D loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55D orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55D_RENDERER_BACKEND_OWNER_SHELL.md"),
        )
        .expect("Phase 55D 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitRendererBackendOwnerShell",
            "renderer_backend_owner_shell: RuntimeSurfaceCommitRendererBackendOwnerShell",
            "pub struct RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport",
            "pub enum RuntimeSurfaceCommitRendererBackendOwnerShellOperation",
            "pub enum RuntimeSurfaceCommitRendererBackendOwnerShellBlocker",
            "pub fn renderer_backend_owner_shell_readiness_from_registration",
            "pub source_renderer_backend_registration_report_observed: bool",
            "pub source_renderer_backend_descriptor_available: bool",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub renderer_backend_owner_shell_available: bool",
            "pub renderer_backend_owner_shell_bound: bool",
            "pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>",
            "renderer_backend_owner_shell_available: true",
            "renderer_backend_owner_shell_bound: true",
            "registered_renderer_backend_kind: report.registered_renderer_backend_kind",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55D coordinator renderer backend owner shell 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport",
            "pub renderer_backend_owner_shell_readiness_invocations: usize",
            "pub renderer_backend_owner_shell_intents_observed: usize",
            "pub renderer_backend_owner_shell_observed_intents:",
            "pub renderer_backend_owner_shell_available: bool",
            "pub renderer_backend_owner_shell_bound: bool",
            "pub renderer_backend_owner_shell_descriptor_available: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_renderer_backend_owner_shell_readiness",
            "report.renderer_backend_owner_shell_readiness_report",
            "first_renderer_backend_owner_shell.commit_sequence",
            "second_renderer_backend_owner_shell.commit_sequence",
            "renderer_backend_owner_shell_renderer_called",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55D loop renderer backend owner shell report 缺少证据: {required}"
            );
        }

        for required in [
            "renderer_backend_owner_shell_intents_observed",
            "renderer_backend_owner_shell_available",
            "renderer_backend_owner_shell_bound",
            "renderer_backend_owner_shell_descriptor_available",
            "first_renderer_backend_owner_shell.commit_sequence",
            "second_renderer_backend_owner_shell.commit_sequence",
            "renderer_backend_owner_shell_renderer_called",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55D orchestrator renderer backend owner shell report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55D renderer backend owner shell 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "renderer_backend_owner_shell_available = true",
            "renderer_backend_owner_shell_bound = true",
            "registered_renderer_backend_kind = Some(SmithayLinux)",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55D doc 缺少 capability truth: {required}"
            );
        }
    }

    /// Phase 55E 必须建立 buffer importer resource owner boundary / handoff seam。
    #[test]
    fn buffer_import_resource_owner_boundary_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55E coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55E loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55E orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55E_BUFFER_IMPORT_RESOURCE_OWNER_BOUNDARY.md"),
        )
        .expect("Phase 55E 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportResourceOwnerBoundary",
            "buffer_import_resource_owner_boundary: RuntimeSurfaceCommitBufferImportResourceOwnerBoundary",
            "pub struct RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport",
            "pub enum RuntimeSurfaceCommitBufferImportResourceOwnerOperation",
            "pub enum RuntimeSurfaceCommitBufferImportResourceOwnerBlocker",
            "pub fn buffer_import_resource_owner_readiness_from_renderer_backend_owner_shell",
            "pub source_renderer_backend_owner_shell_readiness_observed: bool",
            "pub source_renderer_backend_owner_shell_available: bool",
            "pub source_renderer_backend_owner_shell_bound: bool",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub buffer_importer_owner_available: bool",
            "pub buffer_importer_owner_bound: bool",
            "pub renderer_backend_descriptor_evidence_available: bool",
            "pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>",
            "buffer_importer_owner_available: true",
            "buffer_importer_owner_bound: true",
            "renderer_backend_descriptor_evidence_available: report",
            ".source_renderer_backend_descriptor_available",
            "registered_renderer_backend_kind: report.registered_renderer_backend_kind",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55E coordinator buffer import owner boundary 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport",
            "pub buffer_import_resource_owner_readiness_invocations: usize",
            "pub buffer_import_resource_owner_intents_observed: usize",
            "pub buffer_import_resource_owner_observed_intents:",
            "pub buffer_importer_owner_available: bool",
            "pub buffer_importer_owner_bound: bool",
            "pub buffer_import_resource_owner_descriptor_evidence_available: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_resource_owner_readiness",
            "report.buffer_import_resource_owner_readiness_report",
            "first_buffer_import_resource_owner.commit_sequence",
            "second_buffer_import_resource_owner.commit_sequence",
            "buffer_import_resource_owner_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55E loop buffer import owner report 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_resource_owner_intents_observed",
            "buffer_importer_owner_available",
            "buffer_importer_owner_bound",
            "buffer_import_resource_owner_descriptor_evidence_available",
            "first_buffer_import_resource_owner.commit_sequence",
            "second_buffer_import_resource_owner.commit_sequence",
            "buffer_import_resource_owner_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55E orchestrator buffer import owner report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55E buffer import resource owner boundary 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "buffer_importer_owner_available = true",
            "buffer_importer_owner_bound = true",
            "renderer_backend_descriptor_evidence_available = true",
            "registered_renderer_backend_kind = Some(SmithayLinux)",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55E doc 缺少 capability truth: {required}"
            );
        }
    }

    /// Phase 55F 必须建立 buffer import planning/report seam。
    #[test]
    fn buffer_import_planning_report_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55F coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55F loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55F orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55F_BUFFER_IMPORT_PLANNING_REPORT.md"),
        )
        .expect("Phase 55F 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportPlanner",
            "buffer_import_planner: RuntimeSurfaceCommitBufferImportPlanner",
            "pub struct RuntimeSurfaceCommitBufferImportPlanningReport",
            "pub enum RuntimeSurfaceCommitBufferImportPlanningOperation",
            "pub enum RuntimeSurfaceCommitBufferImportPlanningBlocker",
            "pub fn buffer_import_planning_report_from_resource_owner_boundary",
            "pub source_buffer_import_resource_owner_readiness_observed: bool",
            "pub source_buffer_importer_owner_available: bool",
            "pub observed_intent: Option<RuntimeSurfaceCommitRenderOperationIntent>",
            "pub buffer_import_plan_available: bool",
            "pub buffer_import_plan_built: bool",
            "pub buffer_import_candidate_observed: bool",
            "pub buffer_import_required: bool",
            "pub renderer_backend_descriptor_evidence_available: bool",
            "buffer_import_plan_available: true",
            "let buffer_import_plan_built = observed_intent.is_some();",
            "buffer_import_plan_built,",
            "buffer_import_candidate_observed",
            "buffer_import_required",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55F coordinator buffer import planning 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportPlanningReport",
            "pub buffer_import_planning_invocations: usize",
            "pub buffer_import_planning_intents_observed: usize",
            "pub buffer_import_planning_observed_intents:",
            "pub buffer_import_plan_available: bool",
            "pub buffer_import_plan_built: bool",
            "pub buffer_import_candidates_observed: usize",
            "pub buffer_import_required_count: usize",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_planning_report",
            "report.buffer_import_planning_report",
            "first_buffer_import_plan.commit_sequence",
            "second_buffer_import_plan.commit_sequence",
            "buffer_import_planning_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55F loop buffer import planning report 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_planning_intents_observed",
            "buffer_import_plan_available",
            "buffer_import_plan_built",
            "buffer_import_candidates_observed",
            "buffer_import_required_count",
            "first_buffer_import_plan.commit_sequence",
            "second_buffer_import_plan.commit_sequence",
            "buffer_import_planning_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55F orchestrator buffer import planning report 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55F buffer import planning 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "buffer_import_plan_available = true",
            "buffer_import_plan_built = true",
            "buffer_import_candidate_observed = true",
            "buffer_import_required = true",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55F doc 缺少 capability truth: {required}"
            );
        }
    }

    /// Phase 55G 必须建立 buffer import implementation descriptor / adapter boundary seam。
    #[test]
    fn buffer_import_implementation_descriptor_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55G coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55G loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55G orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55G_BUFFER_IMPORT_IMPLEMENTATION_DESCRIPTOR.md"),
        )
        .expect("Phase 55G 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportImplementationDescriptor",
            "pub struct RuntimeSurfaceCommitBufferImportImplementationBoundary",
            "buffer_import_implementation_boundary: RuntimeSurfaceCommitBufferImportImplementationBoundary",
            "pub struct RuntimeSurfaceCommitBufferImportImplementationBoundaryReport",
            "pub enum RuntimeSurfaceCommitBufferImportImplementationOperation",
            "pub enum RuntimeSurfaceCommitBufferImportImplementationBlocker",
            "pub fn buffer_import_implementation_boundary_report_from_planning_report",
            "pub source_buffer_import_planning_report_observed: bool",
            "pub implementation_descriptor_available: bool",
            "pub implementation_descriptor_registered: bool",
            "pub candidate_evidence_observed: bool",
            "pub actual_import_required: bool",
            "pub importer_owner_evidence_available: bool",
            "pub renderer_backend_descriptor_evidence_available: bool",
            "pub descriptor: Option<RuntimeSurfaceCommitBufferImportImplementationDescriptor>",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55G coordinator buffer import implementation descriptor 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportImplementationBoundaryReport",
            "pub buffer_import_implementation_boundary_invocations: usize",
            "pub buffer_import_implementation_descriptors_observed: usize",
            "pub buffer_import_implementation_observed_descriptors:",
            "pub buffer_import_implementation_descriptor_available: bool",
            "pub buffer_import_implementation_descriptor_registered: bool",
            "pub buffer_import_implementation_candidates_observed: usize",
            "pub buffer_import_implementation_actual_required_count: usize",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_implementation_boundary_report",
            "report.buffer_import_implementation_boundary_report",
            "first_buffer_import_descriptor.commit_sequence",
            "second_buffer_import_descriptor.commit_sequence",
            "buffer_import_implementation_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55G loop buffer import implementation descriptor 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_implementation_boundary_invocations",
            "buffer_import_implementation_descriptor_available",
            "buffer_import_implementation_descriptor_registered",
            "buffer_import_implementation_candidates_observed",
            "buffer_import_implementation_actual_required_count",
            "first_buffer_import_descriptor.commit_sequence",
            "second_buffer_import_descriptor.commit_sequence",
            "buffer_import_implementation_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55G orchestrator buffer import implementation descriptor 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55G buffer import implementation descriptor 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "implementation_descriptor_available = true",
            "implementation_descriptor_registered = true",
            "candidate_evidence_observed = true",
            "actual_import_required = true",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55G doc 缺少 descriptor/capability truth: {required}"
            );
        }
    }

    /// Phase 55H 必须建立 buffer import adapter proof boundary seam。
    #[test]
    fn buffer_import_adapter_proof_boundary_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55H coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55H loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55H orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55H_BUFFER_IMPORT_ADAPTER_PROOF_BOUNDARY.md"),
        )
        .expect("Phase 55H 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportAdapterProof",
            "pub struct RuntimeSurfaceCommitBufferImportAdapterProofBoundary",
            "buffer_import_adapter_proof_boundary: RuntimeSurfaceCommitBufferImportAdapterProofBoundary",
            "pub struct RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport",
            "pub enum RuntimeSurfaceCommitBufferImportAdapterProofOperation",
            "pub enum RuntimeSurfaceCommitBufferImportAdapterProofBlocker",
            "pub fn buffer_import_adapter_proof_boundary_report_from_implementation_report",
            "pub source_buffer_import_implementation_report_observed: bool",
            "pub implementation_descriptor_registered: bool",
            "pub adapter_proof_boundary_available: bool",
            "pub adapter_proof_registered: bool",
            "pub adapter_proof: Option<RuntimeSurfaceCommitBufferImportAdapterProof>",
            "pub actual_import_required: bool",
            "pub buffer_import_attempted: bool",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55H coordinator buffer import adapter proof boundary 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport",
            "pub buffer_import_adapter_proof_boundary_invocations: usize",
            "pub buffer_import_adapter_proofs_observed: usize",
            "pub buffer_import_adapter_observed_proofs:",
            "pub buffer_import_adapter_proof_boundary_available: bool",
            "pub buffer_import_adapter_proof_registered: bool",
            "pub buffer_import_adapter_actual_required_count: usize",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_adapter_proof_boundary_report",
            "report.buffer_import_adapter_proof_boundary_report",
            "first_buffer_import_adapter_proof.commit_sequence",
            "second_buffer_import_adapter_proof.commit_sequence",
            "buffer_import_adapter_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55H loop buffer import adapter proof boundary 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_adapter_proof_boundary_invocations",
            "buffer_import_adapter_proof_boundary_available",
            "buffer_import_adapter_proof_registered",
            "buffer_import_adapter_actual_required_count",
            "first_buffer_import_adapter_proof.commit_sequence",
            "second_buffer_import_adapter_proof.commit_sequence",
            "buffer_import_adapter_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55H orchestrator buffer import adapter proof boundary 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55H buffer import adapter proof boundary 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "adapter_proof_boundary_available = true",
            "adapter_proof_registered = true",
            "actual_import_required = true",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55H doc 缺少 adapter proof/capability truth: {required}"
            );
        }
    }

    /// Phase 55I 必须建立 buffer import precondition gate seam。
    #[test]
    fn buffer_import_precondition_gate_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55I coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55I loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55I orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55I_BUFFER_IMPORT_PRECONDITION_GATE.md"),
        )
        .expect("Phase 55I 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportPreconditionGate",
            "buffer_import_precondition_gate: RuntimeSurfaceCommitBufferImportPreconditionGate",
            "pub struct RuntimeSurfaceCommitBufferImportPreconditionGateReport",
            "pub enum RuntimeSurfaceCommitBufferImportPreconditionGateOperation",
            "pub enum RuntimeSurfaceCommitBufferImportPreconditionGateBlocker",
            "pub fn buffer_import_precondition_gate_report_from_adapter_proof",
            "pub source_buffer_import_adapter_proof_report_observed: bool",
            "pub adapter_proof_registered: bool",
            "pub import_precondition_gate_available: bool",
            "pub import_preconditions_met: bool",
            "pub future_import_preconditions_met: bool",
            "pub actual_import_required: bool",
            "pub buffer_import_attempted: bool",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55I coordinator buffer import precondition gate 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportPreconditionGateReport",
            "pub buffer_import_precondition_gate_invocations: usize",
            "pub buffer_import_preconditions_met_count: usize",
            "pub buffer_import_future_preconditions_met_count: usize",
            "pub buffer_import_precondition_actual_required_count: usize",
            "pub buffer_import_precondition_gate_available: bool",
            "pub buffer_import_precondition_missing_actual_import_requirement: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_precondition_gate_report",
            "report.buffer_import_precondition_gate_report",
            "buffer_import_precondition_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55I loop buffer import precondition gate 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_precondition_gate_invocations",
            "buffer_import_preconditions_met_count",
            "buffer_import_future_preconditions_met_count",
            "buffer_import_precondition_actual_required_count",
            "buffer_import_precondition_gate_available",
            "buffer_import_precondition_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55I orchestrator buffer import precondition gate 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55I buffer import precondition gate 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "import_precondition_gate_available = true",
            "import_preconditions_met = true",
            "future_import_preconditions_met = true",
            "actual_import_required = true",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55I doc 缺少 precondition/capability truth: {required}"
            );
        }
    }

    /// Phase 55J 必须建立 buffer import execution dry-run / no-op guard seam。
    #[test]
    fn buffer_import_execution_dry_run_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55J coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55J loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55J orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55J_BUFFER_IMPORT_EXECUTION_DRY_RUN.md"),
        )
        .expect("Phase 55J 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportExecutionDryRun",
            "buffer_import_execution_dry_run: RuntimeSurfaceCommitBufferImportExecutionDryRun",
            "pub struct RuntimeSurfaceCommitBufferImportExecutionDryRunReport",
            "pub enum RuntimeSurfaceCommitBufferImportExecutionOperation",
            "pub enum RuntimeSurfaceCommitBufferImportExecutionBlocker",
            "pub fn buffer_import_execution_dry_run_report_from_precondition_gate",
            "pub source_buffer_import_precondition_gate_report_observed: bool",
            "pub observed_adapter_proof: Option<RuntimeSurfaceCommitBufferImportAdapterProof>",
            "pub execution_guard_available: bool",
            "pub execution_attempted: bool",
            "pub execution_noop: bool",
            "pub execution_blocked: bool",
            "pub actual_import_required: bool",
            "MissingRealBufferImportImplementation",
            "NoActualImportRequired",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55J coordinator buffer import execution dry-run 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportExecutionDryRunReport",
            "pub buffer_import_execution_dry_run_invocations: usize",
            "pub buffer_import_execution_dry_run_reports:",
            "Vec<RuntimeSurfaceCommitBufferImportExecutionDryRunReport>",
            "pub buffer_import_execution_guard_available: bool",
            "pub buffer_import_execution_attempted_count: usize",
            "pub buffer_import_execution_noop_count: usize",
            "pub buffer_import_execution_blocked_count: usize",
            "pub buffer_import_execution_missing_real_importer: bool",
            "pub buffer_import_execution_no_actual_import_required: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_execution_dry_run_report",
            "report.buffer_import_execution_dry_run_report",
            "buffer_import_execution_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55J loop buffer import execution dry-run 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_execution_dry_run_invocations",
            "buffer_import_execution_guard_available",
            "buffer_import_execution_attempted_count",
            "buffer_import_execution_noop_count",
            "buffer_import_execution_blocked_count",
            "buffer_import_execution_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55J orchestrator buffer import execution dry-run 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55J buffer import execution dry-run 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "execution_guard_available = true",
            "execution_attempted = false",
            "execution_noop = true",
            "execution_blocked = true",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55J doc 缺少 dry-run/capability truth: {required}"
            );
        }
    }

    /// Phase 55K 必须建立 buffer import implementation owner shell / actual import owner boundary。
    #[test]
    fn buffer_import_implementation_owner_shell_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55K coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55K loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55K orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55K_BUFFER_IMPORT_IMPLEMENTATION_OWNER_SHELL.md"),
        )
        .expect("Phase 55K 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportImplementationOwnerShell",
            "buffer_import_implementation_owner_shell:",
            "RuntimeSurfaceCommitBufferImportImplementationOwnerShell",
            "pub struct RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport",
            "pub enum RuntimeSurfaceCommitBufferImportImplementationOwnerOperation",
            "pub enum RuntimeSurfaceCommitBufferImportImplementationOwnerBlocker",
            "pub fn buffer_import_implementation_owner_shell_report_from_execution_dry_run",
            "pub source_buffer_import_execution_dry_run_report_observed: bool",
            "pub observed_execution_dry_run_report:",
            "RuntimeSurfaceCommitBufferImportExecutionDryRunReport",
            "pub implementation_owner_shell_available: bool",
            "pub real_importer_implementation_available: bool",
            "pub actual_import_attempt_admitted: bool",
            "pub actual_import_attempt_blocked: bool",
            "pub actual_import_required: bool",
            "MissingRealBufferImportImplementation",
            "ExecutionDryRunBlocked",
            "NoActualImportRequired",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55K coordinator buffer import implementation owner shell 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport",
            "pub buffer_import_implementation_owner_shell_invocations: usize",
            "pub buffer_import_implementation_owner_shell_reports:",
            "Vec<RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport>",
            "pub buffer_import_implementation_owner_shell_available: bool",
            "pub buffer_import_real_implementation_available: bool",
            "pub buffer_import_actual_attempt_admitted_count: usize",
            "pub buffer_import_actual_attempt_blocked_count: usize",
            "pub buffer_import_implementation_owner_missing_real_importer: bool",
            "pub buffer_import_implementation_owner_execution_dry_run_blocked: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_implementation_owner_shell_report",
            "report.buffer_import_implementation_owner_shell_report",
            "buffer_import_implementation_owner_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55K loop buffer import implementation owner shell 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_implementation_owner_shell_invocations",
            "buffer_import_implementation_owner_shell_available",
            "buffer_import_real_implementation_available",
            "buffer_import_actual_attempt_admitted_count",
            "buffer_import_actual_attempt_blocked_count",
            "buffer_import_implementation_owner_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55K orchestrator buffer import implementation owner shell 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55K buffer import implementation owner shell 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "implementation_owner_shell_available = true",
            "real_importer_implementation_available = false",
            "actual_import_attempt_admitted = false",
            "actual_import_attempt_blocked = true",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55K doc 缺少 owner shell/capability truth: {required}"
            );
        }
    }

    /// Phase 55L 必须建立 actual buffer import attempt admission / record 纯数据 seam。
    #[test]
    fn buffer_import_actual_attempt_record_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55L coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55L loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55L orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55L_BUFFER_IMPORT_ACTUAL_ATTEMPT_RECORD.md"),
        )
        .expect("Phase 55L 文档必须存在");

        for required in [
            "pub struct RuntimeSurfaceCommitBufferImportActualAttemptRecorder",
            "buffer_import_actual_attempt_recorder:",
            "RuntimeSurfaceCommitBufferImportActualAttemptRecorder",
            "pub struct RuntimeSurfaceCommitBufferImportActualAttemptRecord",
            "pub enum RuntimeSurfaceCommitBufferImportActualAttemptOperation",
            "pub enum RuntimeSurfaceCommitBufferImportActualAttemptBlocker",
            "pub fn buffer_import_actual_attempt_record_from_owner_shell",
            "pub source_buffer_import_implementation_owner_shell_report_observed: bool",
            "pub observed_implementation_owner_shell_report:",
            "RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport",
            "pub actual_attempt_record_available: bool",
            "pub actual_attempt_recorded: bool",
            "pub actual_attempt_admission_checked: bool",
            "pub actual_attempt_admitted: bool",
            "pub actual_attempt_blocked: bool",
            "pub actual_import_required: bool",
            "MissingImplementationOwnerShellReport",
            "ImplementationOwnerShellBlocked",
            "MissingAttemptAdmission",
            "MissingRealBufferImportImplementation",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 55L coordinator actual attempt record 缺少证据: {required}"
            );
        }

        for required in [
            "RuntimeSurfaceCommitBufferImportActualAttemptRecord",
            "pub buffer_import_actual_attempt_record_invocations: usize",
            "pub buffer_import_actual_attempt_records:",
            "Vec<RuntimeSurfaceCommitBufferImportActualAttemptRecord>",
            "pub buffer_import_actual_attempt_record_available: bool",
            "pub buffer_import_actual_attempt_recorded_count: usize",
            "pub buffer_import_actual_attempt_admission_checked_count: usize",
            "pub buffer_import_actual_attempt_missing_admission: bool",
            "pub buffer_import_actual_attempt_missing_real_importer: bool",
            "NestedRuntimeSurfaceCommitRunSummary::from_buffer_import_actual_attempt_record",
            "report.buffer_import_actual_attempt_record",
            "buffer_import_actual_attempt_buffer_imported",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 55L loop actual attempt record 缺少证据: {required}"
            );
        }

        for required in [
            "buffer_import_actual_attempt_record_invocations",
            "buffer_import_actual_attempt_record_available",
            "buffer_import_actual_attempt_recorded_count",
            "buffer_import_actual_attempt_admission_checked_count",
            "buffer_import_actual_attempt_record_blocked_count",
            "buffer_import_actual_attempt_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55L orchestrator actual attempt record 缺少证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "render_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "render_invoked: true",
            "input_invoked: true",
            "damage_submitted: true",
            "renderable_buffer: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55L actual attempt record 包含禁止 token: {forbidden}"
            );
        }

        for required in [
            "actual_attempt_record_available = true",
            "actual_attempt_recorded = true",
            "actual_attempt_admission_checked = true",
            "actual_attempt_admitted = false",
            "actual_attempt_blocked = true",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55L doc 缺少 attempt record/capability truth: {required}"
            );
        }
    }

    /// Phase 55M 必须审计真实 buffer import 边界，并防止把 shell/record/dry-run 误报为真实 import。
    #[test]
    fn real_buffer_import_boundary_audit_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55M coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55M loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55M orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55M_REAL_BUFFER_IMPORT_BOUNDARY_AUDIT.md"),
        )
        .expect("Phase 55M 文档必须存在");

        for required in [
            "Phase 55M - Real Buffer Import Boundary Audit",
            "Phase 55E",
            "Phase 55F",
            "Phase 55G",
            "Phase 55H",
            "Phase 55I",
            "Phase 55J",
            "Phase 55K",
            "Phase 55L",
            "pure-data",
            "readiness",
            "dry-run",
            "record",
            "No real buffer import has happened",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
            "Smithay and renderer resource types must remain in src/smithay_backend",
            "Linux-only adapter layer",
            "core remains abstract",
            "WindowId",
            "Geometry",
            "State",
            "Action",
            "Command",
            "wl_buffer::WlBuffer",
            "BufferHandler",
            "Renderer",
            "Texture",
            "Dmabuf",
            "EGL",
            "GLES",
            "WGPU",
            "MissingRealBufferImportImplementation",
            "MissingAttemptAdmission",
            "MissingTextureCreation",
            "MissingRendererCall",
            "MissingDamageSubmit",
            "MissingFrameCallbackDone",
            "Phase 55N",
            "Phase 56A",
            "Stop before choosing a real backend",
            "shell / record / dry-run reports are not real import",
            "Do not claim renderable window",
            "Do not claim real compositor runtime ready",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55M audit doc 缺少真实 buffer import 边界证据: {required}"
            );
        }

        for required in [
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required) || runtime_loop.contains(required),
                "Phase 55M source 缺少 capability truth false 证据: {required}"
            );
        }

        for required in [
            "buffer_import_actual_attempt_record_invocations",
            "buffer_import_actual_attempt_record_available",
            "buffer_import_actual_attempt_recorded_count",
            "buffer_import_actual_attempt_admission_checked_count",
            "buffer_import_actual_attempt_record_blocked_count",
            "buffer_import_actual_attempt_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55M orchestrator 缺少 Phase 55L record 暴露证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "damage_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            ".done(",
            "renderable_buffer: true",
            "real_compositor_runtime_ready: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55M source 包含禁止的真实执行声明: {forbidden}"
            );
        }
    }

    /// Phase 55N 只做真实 import route 决策矩阵和非执行 adapter contract。
    ///
    /// 这个 source-contract 防止把路线推荐写成真实实现：buffer import 仍未发生，
    /// texture creation 仍未发生，renderer call 仍未发生，frame callback done 仍未发送，
    /// core mutation 仍未发生。
    #[test]
    fn real_import_route_decision_matrix_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 55N coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 55N loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 55N orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_55N_REAL_IMPORT_ROUTE_DECISION_MATRIX.md"),
        )
        .expect("Phase 55N 文档必须存在");

        // 文档必须证明 Phase 55N 只提供决策矩阵和非执行 contract，不进入真实资源实现。
        for required in [
            "Phase 55N - Real Import Route Decision Matrix",
            "SHM-first nested MVP route",
            "dmabuf route",
            "EGL/GLES/GBM route",
            "WGPU route",
            "hybrid staged route",
            "Non-executing Adapter Contract",
            "input evidence",
            "output evidence",
            "adapter surface id",
            "commit sequence",
            "buffer presence evidence",
            "buffer candidate evidence",
            "actual import required",
            "precondition gate evidence",
            "execution dry-run evidence",
            "implementation owner shell evidence",
            "route selected",
            "adapter contract available",
            "real importer missing",
            "execution allowed = false",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
            "Phase 56A: minimal SHM-first buffer import adapter skeleton",
            "WindowId",
            "Geometry",
            "State",
            "Layout",
            "Action",
            "Command",
            "smithay_backend / Linux-only adapter",
            "This is a recommendation, not an implementation",
            "waiting for user authorization on Phase 56A",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 55N decision matrix 文档缺少非执行 contract 证据: {required}"
            );
        }

        // 生产 source 仍必须保持真实执行 capability 为 false；测试字符串本身不作为生产证据。
        for required in [
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
        ] {
            assert!(
                coordinator.contains(required) || runtime_loop.contains(required),
                "Phase 55N source 缺少 capability truth false 证据: {required}"
            );
        }

        // Phase 55N 不新增 production seam，但必须继续保留 55L actual attempt record 的报告边界。
        for required in [
            "buffer_import_actual_attempt_record_invocations",
            "buffer_import_actual_attempt_record_available",
            "buffer_import_actual_attempt_recorded_count",
            "buffer_import_actual_attempt_admission_checked_count",
            "buffer_import_actual_attempt_record_blocked_count",
            "buffer_import_actual_attempt_buffer_imported",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 55N orchestrator 缺少既有 actual attempt record 暴露证据: {required}"
            );
        }

        for forbidden in [
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "damage_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            "renderable_buffer: true",
            "real_compositor_runtime_ready: true",
        ] {
            assert!(
                !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden)
                    && !orchestrator.contains(forbidden),
                "Phase 55N source 包含禁止的真实执行声明: {forbidden}"
            );
        }
    }

    /// Phase 56A 建立 Linux-only SHM-first buffer import adapter skeleton。
    ///
    /// 这个 source-contract 允许真实 Smithay `WlBuffer` 类型只出现在 Linux-only
    /// adapter/glue 文件中，但仍禁止 buffer import、texture creation、renderer call、
    /// damage submit、frame callback done、input 和 core mutation。
    #[test]
    fn shm_first_buffer_import_adapter_skeleton_source_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let module = std::fs::read_to_string(
            root.join("src/smithay_backend/linux_shm_buffer_import_adapter.rs"),
        )
        .expect("Phase 56A SHM-first adapter module 必须存在");
        let coordinator =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_coordinator.rs"))
                .expect("Phase 56A coordinator source 必须存在");
        let runtime_loop =
            std::fs::read_to_string(root.join("src/smithay_backend/nested_runtime_loop.rs"))
                .expect("Phase 56A loop source 必须存在");
        let orchestrator = std::fs::read_to_string(
            root.join("src/smithay_backend/nested_runtime_orchestrator.rs"),
        )
        .expect("Phase 56A orchestrator source 必须存在");
        let phase_doc = std::fs::read_to_string(
            root.join("docs/phases/PHASE_56A_SHM_FIRST_BUFFER_IMPORT_ADAPTER_SKELETON.md"),
        )
        .expect("Phase 56A 文档必须存在");
        let mod_source = std::fs::read_to_string(root.join("src/smithay_backend/mod.rs"))
            .expect("smithay_backend mod source 必须存在");

        let production_module = module
            .split_once("#[cfg(test)]")
            .map_or(module.as_str(), |(production, _)| production);

        for required in [
            "Phase 56A - SHM-first Buffer Import Adapter Skeleton",
            "Linux-only adapter skeleton",
            "WlBuffer",
            "evidence-only",
            "blocked",
            "unsupported",
            "no-texture",
            "buffer_import_attempted = false",
            "buffer_imported = false",
            "texture_created = false",
            "renderer_called = false",
            "damage_submitted = false",
            "frame_callback_done_sent = false",
            "input_support = false",
            "core_mutation_invoked = false",
            "Phase 56B",
            "requires separate user",
        ] {
            assert!(
                phase_doc.contains(required),
                "Phase 56A 文档缺少 SHM-first skeleton 证据: {required}"
            );
        }

        for required in [
            "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]\npub mod linux_shm_buffer_import_adapter;",
            "#[cfg(all(feature = \"smithay-linux\", target_os = \"linux\"))]\npub use linux_shm_buffer_import_adapter::{",
        ] {
            assert!(
                mod_source.contains(required),
                "Phase 56A mod.rs 缺少 Linux-only gate/re-export: {required}"
            );
        }

        for required in [
            "use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;",
            "pub fn observe_wl_buffer_type_boundary(",
            "_buffer: &WlBuffer",
            "std::any::type_name::<smithay::wayland::shm::BufferData>()",
            "std::any::type_name::<\n            smithay::wayland::shm::BufferAccessError,",
            "pub struct LinuxShmFirstBufferImportAdapterSkeleton",
            "pub struct RuntimeSurfaceCommitShmFirstBufferImportAdapterReport",
            "pub enum RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker",
            "pub fn shm_first_buffer_import_adapter_report_from_actual_attempt_record",
            "shm_buffer_adapter_available: true",
            "shm_buffer_import_route_selected: true",
            "shm_buffer_import_execution_blocked: true",
            "evidence_only_report: true",
            "no_texture_report: true",
            "buffer_import_attempted: false",
            "buffer_imported: false",
            "texture_created: false",
            "renderer_called: false",
            "damage_submitted: false",
            "frame_callback_done_sent: false",
            "input_support: false",
            "core_mutation_invoked: false",
            "TextureCreationForbiddenInPhase56A",
            "RendererCallForbiddenInPhase56A",
            "DamageSubmitForbiddenInPhase56A",
            "FrameCallbackDoneForbiddenInPhase56A",
            "DrmGbmDmabufForbiddenInPhase56A",
        ] {
            assert!(
                production_module.contains(required),
                "Phase 56A adapter production source 缺少边界证据: {required}"
            );
        }

        for required in [
            "shm_first_buffer_import_adapter: LinuxShmFirstBufferImportAdapterSkeleton",
            "pub shm_first_buffer_import_adapter_report:",
            ".report_from_actual_attempt_record(&buffer_import_actual_attempt_record, None)",
            "shm_first_buffer_import_adapter_report,",
        ] {
            assert!(
                coordinator.contains(required),
                "Phase 56A coordinator 缺少 adapter skeleton 接入证据: {required}"
            );
        }

        for required in [
            "pub shm_buffer_import_adapter_invocations: usize",
            "Vec<RuntimeSurfaceCommitShmFirstBufferImportAdapterReport>",
            "pub shm_buffer_adapter_available: bool",
            "pub shm_buffer_import_route_selected: bool",
            "pub shm_buffer_type_boundary_observed: bool",
            "pub shm_buffer_import_execution_blocked: bool",
            "pub shm_buffer_import_no_texture_report: bool",
            "pub shm_buffer_import_buffer_import_attempted: bool",
            "pub shm_buffer_import_buffer_imported: bool",
            "pub shm_buffer_import_texture_created: bool",
            "pub shm_buffer_import_renderer_called: bool",
            "from_shm_first_buffer_import_adapter_report",
            "report.shm_first_buffer_import_adapter_report",
        ] {
            assert!(
                runtime_loop.contains(required),
                "Phase 56A loop 缺少 adapter skeleton 汇总证据: {required}"
            );
        }

        for required in [
            "shm_buffer_import_adapter_invocations",
            "shm_buffer_import_adapter_reports",
            "shm_buffer_adapter_available",
            "shm_buffer_import_route_selected",
            "shm_buffer_import_execution_blocked",
            "shm_buffer_import_buffer_import_attempted",
            "shm_buffer_import_buffer_imported",
            "shm_buffer_import_texture_created",
            "shm_buffer_import_renderer_called",
            "shm_buffer_import_core_mutation_invoked",
        ] {
            assert!(
                orchestrator.contains(required),
                "Phase 56A orchestrator test/report 缺少 adapter skeleton 暴露证据: {required}"
            );
        }

        for forbidden in [
            "with_buffer_contents(",
            "buffer_import_attempted: true",
            "buffer_imported: true",
            "texture_created: true",
            "renderer_called: true",
            "damage_submitted: true",
            "frame_callback_done_sent: true",
            "input_support: true",
            "core_mutation_invoked: true",
            "renderable_buffer: true",
            "real_compositor_runtime_ready: true",
            "Gles",
            "EGL",
            "WGPU",
        ] {
            assert!(
                !production_module.contains(forbidden)
                    && !coordinator.contains(forbidden)
                    && !runtime_loop.contains(forbidden),
                "Phase 56A source 包含禁止的真实执行/后端 token: {forbidden}"
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
            "fn register_new_toplevel_identity(",
            "surface: &ToplevelSurface",
            "self.new_toplevel_callback_count += 1",
            "self.last_new_toplevel_callback_observation_sequence = Some(sequence)",
            "LinuxXdgToplevelIdentityRegistry::key_for_toplevel(surface)",
            ".observe_surface(surface.wl_surface())",
            ".register(identity, adapter_surface)",
            "fn new_toplevel(&mut self, surface: ToplevelSurface)",
            "let callback_sequence = self.record_new_toplevel_callback_observation();",
            "let registration = self.register_new_toplevel_identity(&surface);",
            "self.record_pending_live_toplevel_admission_observation(callback_sequence, registration);",
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
