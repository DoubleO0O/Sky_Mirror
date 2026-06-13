#![cfg(all(feature = "smithay-linux", target_os = "linux"))]

//! Smithay handler 边界的隔离类型形状探针。
//!
//! 本模块只记录 Phase 49C 的编译审计结果。它不实现 Smithay handler trait，
//! 不持有原生对象，也不进入 adapter、runtime 或核心状态。

/// Phase 49C handler compile probe 的完成范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxHandlerProbeKind {
    /// 只验证隔离 inert handler 类型和报告可以编译。
    TypeShapeOnly,

    /// trait 形状已审计，但因会建立可执行处理边界而被阻止。
    TraitShapeBlocked,

    /// 为未来阶段保留的隔离 trait 编译完成状态。
    TraitShapeCompiled,
}

/// 阻止 Phase 49C 扩展到真实 Smithay handler trait 的结构化原因。
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

/// Phase 49C 隔离 handler compile probe 的纯数据报告。
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

/// 返回 Phase 49C handler trait 审计的固定保守报告。
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

#[cfg(test)]
mod tests {
    use super::{
        SmithayLinuxHandlerProbeBlocker, SmithayLinuxHandlerProbeKind,
        SmithayLinuxInertHandlerProbe, smithay_linux_handler_probe_report,
    };
    use crate::smithay_backend::{
        linux_adapter::{
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
            ("Global", "Dispatch"),
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
        for source in [adapter_source, runtime_source] {
            assert!(!source.contains(&probe_type));
            assert!(!source.contains(&probe_report_type));
            assert!(!source.contains(&probe_function));
        }
    }
}
