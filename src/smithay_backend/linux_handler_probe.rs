#![cfg(all(feature = "smithay-linux", target_os = "linux"))]

//! Smithay handler 边界的隔离类型形状探针与 requirement matrix。
//!
//! 本模块只记录隔离类型形状的编译审计结果和 blocker evidence。它不实现
//! Smithay handler trait，不持有原生对象，也不进入 adapter、runtime 或核心状态。
//! Requirement matrix 描述建立 handler 前缺少什么，不表示这些入口已部分可用。

use super::linux_adapter::SmithayLinuxAdapterGlobalHandlerKind;

/// 隔离 handler compile probe 的完成范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxHandlerProbeKind {
    /// 只验证隔离 inert handler 类型和报告可以编译。
    TypeShapeOnly,

    /// trait 形状已审计，但因会建立可执行处理边界而被阻止。
    TraitShapeBlocked,

    /// 为受控后续实现保留的隔离 trait 编译完成状态。
    TraitShapeCompiled,
}

/// 阻止 compile probe 扩展到真实 Smithay handler trait 的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerProbeBlocker {
    /// trait 实现会定义真实 client bind 入口。
    TraitImplWouldExposeClientBind,

    /// trait 实现会定义真实 protocol request dispatch 入口。
    TraitImplWouldExposeRequestDispatch,

    /// compositor handler 会要求真实 surface 回调边界。
    TraitImplWouldRequireSurfaceHandler,

    /// shared-memory 路径会要求真实 buffer handler。
    TraitImplWouldRequireBufferHandler,

    /// shared-memory 路径会要求真实 SHM handler state。
    TraitImplWouldRequireShmHandler,

    /// XDG shell 路径会要求真实 shell handler state 和回调。
    TraitImplWouldRequireXdgShellHandler,

    /// delegate 宏会安装实际协议分发实现面。
    DelegateMacroWouldInstallDispatchSurface,

    /// trait 形状本身不能使 runtime global 可用。
    CreateGlobalStillRequiredForRuntimeUse,

    /// compile probe 禁止接入生产 adapter。
    AdapterIntegrationForbidden,
}

/// 隔离 handler compile probe 的纯数据报告。
///
/// Diagnostic-only: `compiled_type_shape` 只证明无状态占位类型可编译；它不证明
/// `GlobalDispatch`、`Dispatch`、delegate 宏或 runtime global 已建立。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerProbeReport {
    /// 当前 probe 的完成范围。
    pub kind: SmithayLinuxHandlerProbeKind,

    /// 隔离 inert handler 类型是否已编译。
    pub compiled_type_shape: bool,

    /// Smithay handler trait 形状是否已实现并编译。
    pub compiled_trait_shape: bool,

    /// 是否使用了 Smithay delegate 宏。
    pub uses_delegate_macros: bool,

    /// 是否调用了真实 global 创建入口。
    pub calls_create_global: bool,

    /// 是否调用了其他真实 global 注册入口。
    pub calls_register_global: bool,

    /// probe 是否接触生产 adapter。
    pub touches_adapter: bool,

    /// probe 是否接触纯模型核心。
    pub touches_core: bool,

    /// 阻止 trait 或 runtime 集成的稳定原因。
    pub blockers: Vec<SmithayLinuxHandlerProbeBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// 不持有任何 Smithay 原生对象或生产状态的隔离类型形状。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SmithayLinuxInertHandlerProbe;

impl SmithayLinuxInertHandlerProbe {
    /// 构造无状态的隔离类型形状。
    pub const fn new() -> Self {
        Self
    }
}

/// 建立各类 protocol global handler 前必须满足的稳定要求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerRequirement {
    /// global bind 入口所需的 trait 边界。
    GlobalDispatchBind,

    /// protocol object request 入口所需的 trait 边界。
    DispatchRequest,

    /// compositor surface 状态与回调边界。
    CompositorHandler,

    /// shared-memory buffer 生命周期回调边界。
    BufferHandler,

    /// shared-memory state 回调边界。
    ShmHandler,

    /// XDG shell state 与回调边界。
    XdgShellHandler,

    /// compositor surface request 处理能力。
    SurfaceRequestHandling,

    /// XDG surface request 处理能力。
    XdgSurfaceRequestHandling,

    /// client 与 protocol object 的身份可见性。
    ClientObjectVisibility,

    /// protocol resource 的生命周期跟踪能力。
    ProtocolResourceTracking,

    /// surface 到核心窗口接纳的映射边界。
    CoreAdmissionMapping,
}

/// Handler requirement matrix 中单项要求的保守状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerRequirementState {
    /// 必需 trait 或 handler 尚未实现。
    Missing,

    /// 当前 skeleton policy 或 activation gate 阻止该能力。
    Blocked,

    /// 为尚未进入审计的 requirement 保留。
    NotAttempted,
}

/// 支撑 handler requirement 状态判断的纯数据证据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerRequirementEvidence {
    /// 该要求会建立真实 client bind 入口。
    RequiresClientBindEntry,

    /// 该要求会建立真实 protocol request 入口。
    RequiresProtocolRequestEntry,

    /// 该要求依赖真实 surface 生命周期。
    RequiresSurfaceLifecycleSupport,

    /// 该要求依赖真实 buffer 生命周期。
    RequiresBufferLifecycleSupport,

    /// 该要求依赖 shared-memory callback。
    RequiresShmCallbackSupport,

    /// 该要求依赖 XDG shell callback。
    RequiresXdgShellCallbackSupport,

    /// 该要求依赖 client 身份映射。
    RequiresClientIdentityMapping,

    /// 该要求依赖 protocol resource 跟踪。
    RequiresResourceTracking,

    /// 该要求依赖核心窗口接纳映射。
    RequiresCoreWindowAdmission,

    /// 该要求会安装实际 protocol dispatch 实现。
    WouldInstallDispatchImplementation,

    /// 常规 Smithay handler 集成路径会需要 delegate 宏。
    WouldRequireDelegateMacro,

    /// Adapter activation gate 阻止该要求进入生产路径。
    BlockedByActivationGate,

    /// skeleton-only policy 阻止该要求进入生产路径。
    BlockedBySkeletonOnlyPolicy,
}

/// 单项 handler requirement 及其 blocker evidence。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerRequirementMatrixItem {
    /// requirement 所属的 global handler。
    pub handler: SmithayLinuxAdapterGlobalHandlerKind,

    /// 必须满足的稳定要求。
    pub requirement: SmithayLinuxHandlerRequirement,

    /// 当前要求的保守状态。
    pub state: SmithayLinuxHandlerRequirementState,

    /// 支撑当前状态判断的非空稳定证据。
    pub evidence: Vec<SmithayLinuxHandlerRequirementEvidence>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Handler requirement matrix 的纯数据报告。
///
/// Matrix 将 `Missing` 与 `Blocked` 分开：前者表示 trait/handler 不存在，后者表示
/// 即使数据形状存在也不能越过当前安全策略。两者都不等于 runtime readiness。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerRequirementMatrixReport {
    /// 按 handler 和 requirement 固定顺序排列的矩阵项。
    pub items: Vec<SmithayLinuxHandlerRequirementMatrixItem>,

    /// 尚未实现的 trait 或 handler requirement 数量。
    pub missing_count: usize,

    /// 被能力边界阻止的 requirement 数量。
    pub blocked_count: usize,

    /// 已安全就绪的 requirement 数量；当前保守矩阵恒为零。
    pub ready_count: usize,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// 返回 handler trait 审计的固定保守报告。
pub fn smithay_linux_handler_probe_report() -> SmithayLinuxHandlerProbeReport {
    SmithayLinuxHandlerProbeReport {
        kind: SmithayLinuxHandlerProbeKind::TypeShapeOnly,
        compiled_type_shape: true,
        compiled_trait_shape: false,
        uses_delegate_macros: false,
        calls_create_global: false,
        calls_register_global: false,
        touches_adapter: false,
        touches_core: false,
        blockers: vec![
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeClientBind,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeRequestDispatch,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireSurfaceHandler,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireBufferHandler,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireShmHandler,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireXdgShellHandler,
            SmithayLinuxHandlerProbeBlocker::DelegateMacroWouldInstallDispatchSurface,
            SmithayLinuxHandlerProbeBlocker::CreateGlobalStillRequiredForRuntimeUse,
            SmithayLinuxHandlerProbeBlocker::AdapterIntegrationForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回 handler requirements 和 blocker evidence 的固定矩阵。
pub fn smithay_linux_handler_requirement_matrix_report()
-> SmithayLinuxHandlerRequirementMatrixReport {
    use SmithayLinuxAdapterGlobalHandlerKind as Handler;
    use SmithayLinuxHandlerRequirement as Requirement;
    use SmithayLinuxHandlerRequirementEvidence as Evidence;
    use SmithayLinuxHandlerRequirementState as State;

    let items = vec![
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::GlobalDispatchBind,
            State::Missing,
            vec![
                Evidence::RequiresClientBindEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::DispatchRequest,
            State::Missing,
            vec![
                Evidence::RequiresProtocolRequestEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::CompositorHandler,
            State::Missing,
            vec![
                Evidence::RequiresSurfaceLifecycleSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::SurfaceRequestHandling,
            State::Blocked,
            vec![Evidence::RequiresSurfaceLifecycleSupport],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::ClientObjectVisibility,
            State::Blocked,
            vec![Evidence::RequiresClientIdentityMapping],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::ProtocolResourceTracking,
            State::Blocked,
            vec![Evidence::RequiresResourceTracking],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::CoreAdmissionMapping,
            State::Blocked,
            vec![Evidence::RequiresCoreWindowAdmission],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::GlobalDispatchBind,
            State::Missing,
            vec![
                Evidence::RequiresClientBindEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::DispatchRequest,
            State::Missing,
            vec![
                Evidence::RequiresProtocolRequestEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::BufferHandler,
            State::Missing,
            vec![
                Evidence::RequiresBufferLifecycleSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::ShmHandler,
            State::Missing,
            vec![
                Evidence::RequiresShmCallbackSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::ClientObjectVisibility,
            State::Blocked,
            vec![Evidence::RequiresClientIdentityMapping],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::ProtocolResourceTracking,
            State::Blocked,
            vec![Evidence::RequiresResourceTracking],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::GlobalDispatchBind,
            State::Missing,
            vec![
                Evidence::RequiresClientBindEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::DispatchRequest,
            State::Missing,
            vec![
                Evidence::RequiresProtocolRequestEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::XdgShellHandler,
            State::Missing,
            vec![
                Evidence::RequiresXdgShellCallbackSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::XdgSurfaceRequestHandling,
            State::Blocked,
            vec![
                Evidence::RequiresSurfaceLifecycleSupport,
                Evidence::RequiresXdgShellCallbackSupport,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::ClientObjectVisibility,
            State::Blocked,
            vec![Evidence::RequiresClientIdentityMapping],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::ProtocolResourceTracking,
            State::Blocked,
            vec![Evidence::RequiresResourceTracking],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::CoreAdmissionMapping,
            State::Blocked,
            vec![Evidence::RequiresCoreWindowAdmission],
        ),
    ];
    let missing_count = items
        .iter()
        .filter(|item| item.state == State::Missing)
        .count();
    let blocked_count = items
        .iter()
        .filter(|item| item.state == State::Blocked)
        .count();

    SmithayLinuxHandlerRequirementMatrixReport {
        items,
        missing_count,
        blocked_count,
        ready_count: 0,
        skeleton_only: true,
    }
}

fn requirement_item(
    handler: SmithayLinuxAdapterGlobalHandlerKind,
    requirement: SmithayLinuxHandlerRequirement,
    state: SmithayLinuxHandlerRequirementState,
    mut evidence: Vec<SmithayLinuxHandlerRequirementEvidence>,
) -> SmithayLinuxHandlerRequirementMatrixItem {
    evidence.extend([
        SmithayLinuxHandlerRequirementEvidence::BlockedByActivationGate,
        SmithayLinuxHandlerRequirementEvidence::BlockedBySkeletonOnlyPolicy,
    ]);

    SmithayLinuxHandlerRequirementMatrixItem {
        handler,
        requirement,
        state,
        evidence,
        skeleton_only: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayLinuxHandlerProbeBlocker, SmithayLinuxHandlerProbeKind,
        SmithayLinuxHandlerRequirement, SmithayLinuxHandlerRequirementEvidence,
        SmithayLinuxHandlerRequirementState, SmithayLinuxInertHandlerProbe,
        smithay_linux_handler_probe_report, smithay_linux_handler_requirement_matrix_report,
    };
    use crate::smithay_backend::{
        linux_adapter::{
            SmithayLinuxAdapterGlobalHandlerBlocker, SmithayLinuxAdapterGlobalHandlerKind,
            SmithayLinuxAdapterGlobalHandlerReadiness,
            SmithayLinuxAdapterRealGlobalRegistrationMode, SmithayLinuxAdapterSkeleton,
        },
        runtime_facade::{BackendBootstrapMode, BackendRuntimeReport},
        test_support::{assert_runtime_dir, unique_socket_name},
    };

    #[test]
    fn type_shape_and_report_remain_inert() {
        let probe = SmithayLinuxInertHandlerProbe::new();
        let copied_probe = probe;
        let default_probe = SmithayLinuxInertHandlerProbe::default();
        let report = smithay_linux_handler_probe_report();

        assert_eq!(probe, copied_probe);
        assert_eq!(probe, default_probe);
        assert_eq!(report.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(report.compiled_type_shape);
        assert!(!report.compiled_trait_shape);
        assert!(!report.uses_delegate_macros);
        assert!(!report.calls_create_global);
        assert!(!report.calls_register_global);
        assert!(!report.touches_adapter);
        assert!(!report.touches_core);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::AdapterIntegrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::CreateGlobalStillRequiredForRuntimeUse)
        );
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeClientBind)
        );
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeRequestDispatch)
        );
    }

    #[test]
    fn requirement_matrix_has_stable_complete_order() {
        use SmithayLinuxAdapterGlobalHandlerKind as Handler;
        use SmithayLinuxHandlerRequirement as Requirement;
        use SmithayLinuxHandlerRequirementState as State;

        let report = smithay_linux_handler_requirement_matrix_report();
        let actual = report
            .items
            .iter()
            .map(|item| (item.handler, item.requirement, item.state))
            .collect::<Vec<_>>();
        let expected = vec![
            (
                Handler::CompositorGlobalHandler,
                Requirement::GlobalDispatchBind,
                State::Missing,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::DispatchRequest,
                State::Missing,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::CompositorHandler,
                State::Missing,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::SurfaceRequestHandling,
                State::Blocked,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::ClientObjectVisibility,
                State::Blocked,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::ProtocolResourceTracking,
                State::Blocked,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::CoreAdmissionMapping,
                State::Blocked,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::GlobalDispatchBind,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::DispatchRequest,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::BufferHandler,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::ShmHandler,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::ClientObjectVisibility,
                State::Blocked,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::ProtocolResourceTracking,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::GlobalDispatchBind,
                State::Missing,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::DispatchRequest,
                State::Missing,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::XdgShellHandler,
                State::Missing,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::XdgSurfaceRequestHandling,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::ClientObjectVisibility,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::ProtocolResourceTracking,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::CoreAdmissionMapping,
                State::Blocked,
            ),
        ];

        assert_eq!(actual, expected);
        assert!(report.skeleton_only);
        assert!(!report.items.is_empty());
        assert_eq!(report.ready_count, 0);
        assert_eq!(
            report.missing_count + report.blocked_count,
            report.items.len()
        );
        assert_eq!(report.missing_count, 10);
        assert_eq!(report.blocked_count, 10);
        assert!(report.items.iter().all(|item| {
            item.skeleton_only
                && !item.evidence.is_empty()
                && item.state != SmithayLinuxHandlerRequirementState::NotAttempted
        }));
    }

    #[test]
    fn requirement_matrix_evidence_matches_requirement_semantics() {
        use SmithayLinuxHandlerRequirement as Requirement;
        use SmithayLinuxHandlerRequirementEvidence as Evidence;

        let report = smithay_linux_handler_requirement_matrix_report();
        let evidence_for = |requirement| {
            report
                .items
                .iter()
                .filter(|item| item.requirement == requirement)
                .flat_map(|item| item.evidence.iter().copied())
                .collect::<Vec<_>>()
        };

        assert!(
            evidence_for(Requirement::GlobalDispatchBind)
                .contains(&Evidence::RequiresClientBindEntry)
        );
        assert!(
            evidence_for(Requirement::DispatchRequest)
                .contains(&Evidence::RequiresProtocolRequestEntry)
        );
        assert!(
            evidence_for(Requirement::SurfaceRequestHandling)
                .contains(&Evidence::RequiresSurfaceLifecycleSupport)
        );
        assert!(
            evidence_for(Requirement::BufferHandler)
                .contains(&Evidence::RequiresBufferLifecycleSupport)
        );
        assert!(
            evidence_for(Requirement::ShmHandler).contains(&Evidence::RequiresShmCallbackSupport)
        );
        assert!(
            evidence_for(Requirement::XdgShellHandler)
                .contains(&Evidence::RequiresXdgShellCallbackSupport)
        );
        assert!(
            evidence_for(Requirement::ClientObjectVisibility)
                .contains(&Evidence::RequiresClientIdentityMapping)
        );
        assert!(
            evidence_for(Requirement::ProtocolResourceTracking)
                .contains(&Evidence::RequiresResourceTracking)
        );
        assert!(
            evidence_for(Requirement::CoreAdmissionMapping)
                .contains(&Evidence::RequiresCoreWindowAdmission)
        );
        assert!(report.items.iter().all(|item| {
            item.evidence.contains(&Evidence::BlockedByActivationGate)
                && item
                    .evidence
                    .contains(&Evidence::BlockedBySkeletonOnlyPolicy)
        }));
    }

    #[test]
    fn existing_adapter_boundaries_remain_blocked() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("phase49c-boundary");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("Phase 49C 边界测试应能构造 adapter skeleton");
        let capabilities = adapter.capabilities();

        assert!(!capabilities.runs_event_loop);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
        assert!(capabilities.is_skeleton_only);

        let handler_boundary = adapter.global_handler_boundary_report();
        assert_eq!(handler_boundary.ready_count, 0);
        assert_eq!(handler_boundary.blocked_count, 3);
        assert!(handler_boundary.skeleton_only);
        assert!(handler_boundary.reports.iter().all(|report| {
            report.readiness == SmithayLinuxAdapterGlobalHandlerReadiness::FeasibilityBlocked
                && report.skeleton_only
                && !report.blockers.is_empty()
        }));
        let matrix = smithay_linux_handler_requirement_matrix_report();
        assert_eq!(matrix.ready_count, handler_boundary.ready_count);
        assert_eq!(
            matrix
                .items
                .iter()
                .map(|item| item.handler)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            handler_boundary.blocked_count
        );
        for handler_report in &handler_boundary.reports {
            let requirements = matrix
                .items
                .iter()
                .filter(|item| item.handler == handler_report.kind)
                .map(|item| item.requirement)
                .collect::<Vec<_>>();

            assert!(requirements.contains(&SmithayLinuxHandlerRequirement::GlobalDispatchBind));
            assert!(requirements.contains(&SmithayLinuxHandlerRequirement::DispatchRequest));
            assert!(handler_report.blockers.contains(
                &SmithayLinuxAdapterGlobalHandlerBlocker::MissingGlobalDispatchImplementation
            ));
            assert!(
                handler_report.blockers.contains(
                    &SmithayLinuxAdapterGlobalHandlerBlocker::MissingDispatchImplementation
                )
            );
            assert!(
                handler_report
                    .blockers
                    .contains(&SmithayLinuxAdapterGlobalHandlerBlocker::ActivationGateBlocked)
            );
            match handler_report.kind {
                SmithayLinuxAdapterGlobalHandlerKind::CompositorGlobalHandler => {
                    assert!(
                        requirements.contains(&SmithayLinuxHandlerRequirement::CompositorHandler)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::SurfaceRequestHandling)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::CoreAdmissionMapping)
                    );
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::MissingCompositorHandler
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::SurfaceRequestsUnsupported
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::CoreIntegrationUnsupported
                    ));
                }
                SmithayLinuxAdapterGlobalHandlerKind::ShmGlobalHandler => {
                    assert!(requirements.contains(&SmithayLinuxHandlerRequirement::BufferHandler));
                    assert!(requirements.contains(&SmithayLinuxHandlerRequirement::ShmHandler));
                    assert!(
                        handler_report.blockers.contains(
                            &SmithayLinuxAdapterGlobalHandlerBlocker::MissingBufferHandler
                        )
                    );
                    assert!(
                        handler_report
                            .blockers
                            .contains(&SmithayLinuxAdapterGlobalHandlerBlocker::MissingShmHandler)
                    );
                }
                SmithayLinuxAdapterGlobalHandlerKind::XdgWmBaseGlobalHandler => {
                    assert!(
                        requirements.contains(&SmithayLinuxHandlerRequirement::XdgShellHandler)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::XdgSurfaceRequestHandling)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::CoreAdmissionMapping)
                    );
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::MissingXdgShellHandler
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::SurfaceRequestsUnsupported
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::CoreIntegrationUnsupported
                    ));
                }
            }
        }

        let registration = adapter.attempt_real_global_registration_feasibility();
        assert_eq!(
            registration.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        assert!(registration.attempted_kinds.is_empty());
        assert!(registration.succeeded_kinds.is_empty());
        assert_eq!(registration.real_registered_count, 0);
        assert!(registration.skeleton_only);

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.global_handler_boundary.ready_count, 0);
        assert!(!snapshot.capabilities.accepts_clients);
        assert!(!snapshot.capabilities.registers_protocol_globals);
        assert!(!snapshot.capabilities.dispatches_protocol_events);
        assert!(!snapshot.capabilities.supports_real_wayland_surfaces);
        assert!(!snapshot.capabilities.supports_gpu_rendering);

        let runtime_report = BackendRuntimeReport::from(&adapter);
        assert_eq!(
            runtime_report.bootstrap_mode,
            BackendBootstrapMode::ProbeOnly
        );
        assert!(!runtime_report.capabilities.supports_real_wayland_surfaces);
        assert!(!runtime_report.capabilities.supports_gpu_rendering);
    }

    #[test]
    fn probe_report_and_requirement_matrix_remain_aligned() {
        let probe = smithay_linux_handler_probe_report();
        let matrix = smithay_linux_handler_requirement_matrix_report();

        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);
        assert_eq!(matrix.ready_count, 0);
        assert!(matrix.items.iter().any(|item| {
            item.requirement == SmithayLinuxHandlerRequirement::GlobalDispatchBind
                && item.state == SmithayLinuxHandlerRequirementState::Missing
        }));
        assert!(matrix.items.iter().any(|item| {
            item.requirement == SmithayLinuxHandlerRequirement::DispatchRequest
                && item.state == SmithayLinuxHandlerRequirementState::Missing
        }));
        assert!(
            probe
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::AdapterIntegrationForbidden)
        );
        assert!(
            probe
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::CreateGlobalStillRequiredForRuntimeUse)
        );
    }

    #[test]
    fn production_sources_keep_probe_isolated() {
        let probe_source = include_str!("linux_handler_probe.rs");
        let production_source = probe_source
            .split("#[cfg(test)]")
            .next()
            .expect("probe source 应包含生产代码");
        let adapter_source = include_str!("linux_adapter.rs");
        let runtime_source = include_str!("runtime_facade.rs");

        let forbidden_probe_tokens = [
            ("crate::", "core"),
            ("crate::", "backend"),
            ("Backend", "Event"),
            ("Core", "Command"),
            ("BackendDriver", "Runner"),
            ("smithay", "::"),
            ("Display", "Handle"),
            ("Display", "<"),
            ("display_", "handle"),
            ("accept", "("),
            ("create_", "global("),
            (".create_", "global"),
            ("register_", "global("),
            (".register_", "global"),
            ("impl Global", "Dispatch"),
            ("GlobalDispatch", "<"),
            ("impl Dis", "patch"),
            ("delegate_", "compositor"),
            ("delegate_", "shm"),
            ("delegate_xdg_", "shell"),
            ("wl_", "surface"),
            ("xdg_", "toplevel"),
            ("xdg_wm_", "base"),
            ("impl Buffer", "Handler"),
            ("impl Shm", "Handler"),
            ("impl Compositor", "Handler"),
            ("impl XdgShell", "Handler"),
            ("d", "rm"),
            ("g", "bm"),
            ("lib", "input"),
            ("u", "dev"),
            ("x", "11"),
            ("vul", "kan"),
        ];
        for (left, right) in forbidden_probe_tokens {
            let token = format!("{left}{right}");
            assert!(
                !production_source.contains(&token),
                "production probe 不得包含受限入口 {token}"
            );
        }

        let probe_type = ["SmithayLinux", "InertHandlerProbe"].concat();
        let probe_report_type = ["SmithayLinux", "HandlerProbeReport"].concat();
        let probe_function = ["smithay_linux_", "handler_probe_report"].concat();
        let matrix_report_type = ["SmithayLinux", "HandlerRequirementMatrixReport"].concat();
        let matrix_function = ["smithay_linux_", "handler_requirement_matrix_report"].concat();
        for source in [adapter_source, runtime_source] {
            assert!(!source.contains(&probe_type));
            assert!(!source.contains(&probe_report_type));
            assert!(!source.contains(&probe_function));
            assert!(!source.contains(&matrix_report_type));
            assert!(!source.contains(&matrix_function));
        }
    }
}
