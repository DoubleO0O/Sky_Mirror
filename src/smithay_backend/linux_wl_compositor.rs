/// Linux-only `wl_compositor` global 显式初始化的结构化错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxWlCompositorGlobalInitError {
    /// 当前 display state 已持有 `CompositorState`，拒绝重复注册 global。
    AlreadyInitialized,
}

/// Phase 52M-B owner seam 之后仍未满足的 runtime 前置条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxWlCompositorGlobalBlocker {
    /// 配对 display owner 尚未执行显式初始化。
    MissingExplicitInitialization,
    /// 尚未创建受控 Wayland client connection。
    MissingClientConnection,
    /// 尚未执行 client registry bind。
    MissingRegistryBind,
    /// 尚未证明 client 已 bind `wl_compositor`。
    MissingClientBindProof,
    /// 尚无受控 client lifecycle harness。
    MissingControlledClientHarness,
    /// 尚未建立 `wl_surface` adapter identity registry。
    MissingSurfaceIdentityRegistry,
    /// 尚未建立 `xdg_surface` lifecycle。
    MissingXdgSurfaceLifecycle,
    /// 尚未建立 `xdg_toplevel` lifecycle。
    MissingXdgToplevelLifecycle,
    /// 尚未启动并证明真实 protocol dispatch runtime。
    MissingProtocolDispatchProof,
}

/// Linux-only `wl_compositor` state owner seam 的精确 readiness 报告。
///
/// 报告只描述 server-side owner 与 dispatch wiring。即使 global 初始化成功，
/// 也不表示 client 已 bind、surface 已创建、protocol dispatch 已启动，或 core
/// admission、render、input 已可用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxWlCompositorReadinessReport {
    /// Display 与 handler state 的配对 owner 是否存在。
    pub global_owner_available: bool,
    /// 是否已成功调用 `CompositorState::new`。
    pub compositor_state_new_invoked: bool,
    /// `wl_compositor` global 是否已由 Smithay 创建。
    pub wl_compositor_global_initialized: bool,
    /// 配对 display state 是否持有创建出的 `CompositorState`。
    pub wl_compositor_state_owned: bool,
    /// 最小 `CompositorHandler` 是否已实现。
    pub compositor_handler_available: bool,
    /// `delegate_compositor!` server dispatch wiring 是否已接入。
    pub delegate_compositor_wired: bool,
    /// 每个 inserted client 是否由自己的 `CompositorClientState` owner 持有状态。
    pub per_client_compositor_state_available: bool,
    /// server-side `wl_surface` handler owner boundary 是否可用。
    pub wl_surface_owner_boundary_available: bool,
    /// 是否已有 adapter-owned `wl_surface` identity registry。
    pub wl_surface_identity_registry_available: bool,
    /// 本阶段是否创建了 Wayland client connection。
    pub client_connection_created: bool,
    /// 本阶段是否创建了 client event queue。
    pub event_queue_created: bool,
    /// 本阶段是否尝试 registry bind。
    pub registry_bind_attempted: bool,
    /// 是否已证明 client bind `wl_compositor`。
    pub client_bound_wl_compositor: bool,
    /// 是否已有受控 client harness。
    pub client_harness_available: bool,
    /// 是否已有 `xdg_surface` lifecycle。
    pub xdg_surface_lifecycle_available: bool,
    /// 是否已有 `xdg_toplevel` lifecycle。
    pub xdg_toplevel_lifecycle_available: bool,
    /// `new_toplevel` 是否已有 identity registration owner。
    pub new_toplevel_identity_registration_owner_available: bool,
    /// 本阶段是否调用 admission ledger admit。
    pub ledger_admit_invoked: bool,
    /// 本阶段是否调用 admission ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// 本阶段是否调用 core register。
    pub core_register_invoked: bool,
    /// 本阶段是否调用 core detach。
    pub core_detach_invoked: bool,
    /// 是否已启动 protocol request dispatch。
    pub protocol_dispatch_started: bool,
    /// 是否已有可用的真实 compositor runtime。
    pub real_compositor_runtime_available: bool,
    /// 是否已有可用的真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 当前仍未满足的后续前置条件。
    pub blockers: Vec<LinuxWlCompositorGlobalBlocker>,
}

/// 返回尚未显式初始化的 Phase 52M-B Linux owner readiness。
///
/// 该纯数据查询不创建 global、client、event queue 或 protocol object。
pub fn linux_wl_compositor_readiness_report() -> LinuxWlCompositorReadinessReport {
    build_linux_wl_compositor_readiness_report(false)
}

pub(crate) fn build_linux_wl_compositor_readiness_report(
    initialized: bool,
) -> LinuxWlCompositorReadinessReport {
    let mut blockers = Vec::new();
    if !initialized {
        blockers.push(LinuxWlCompositorGlobalBlocker::MissingExplicitInitialization);
    }
    blockers.extend([
        LinuxWlCompositorGlobalBlocker::MissingClientConnection,
        LinuxWlCompositorGlobalBlocker::MissingRegistryBind,
        LinuxWlCompositorGlobalBlocker::MissingClientBindProof,
        LinuxWlCompositorGlobalBlocker::MissingControlledClientHarness,
        LinuxWlCompositorGlobalBlocker::MissingSurfaceIdentityRegistry,
        LinuxWlCompositorGlobalBlocker::MissingXdgSurfaceLifecycle,
        LinuxWlCompositorGlobalBlocker::MissingXdgToplevelLifecycle,
        LinuxWlCompositorGlobalBlocker::MissingProtocolDispatchProof,
    ]);

    LinuxWlCompositorReadinessReport {
        global_owner_available: true,
        compositor_state_new_invoked: initialized,
        wl_compositor_global_initialized: initialized,
        wl_compositor_state_owned: initialized,
        compositor_handler_available: true,
        delegate_compositor_wired: true,
        per_client_compositor_state_available: true,
        wl_surface_owner_boundary_available: initialized,
        wl_surface_identity_registry_available: false,
        client_connection_created: false,
        event_queue_created: false,
        registry_bind_attempted: false,
        client_bound_wl_compositor: false,
        client_harness_available: false,
        xdg_surface_lifecycle_available: false,
        xdg_toplevel_lifecycle_available: false,
        new_toplevel_identity_registration_owner_available: false,
        ledger_admit_invoked: false,
        ledger_unmap_invoked: false,
        core_register_invoked: false,
        core_detach_invoked: false,
        protocol_dispatch_started: false,
        real_compositor_runtime_available: false,
        real_xdg_shell_runtime_available: false,
        render_support: false,
        input_support: false,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LinuxWlCompositorGlobalBlocker, build_linux_wl_compositor_readiness_report,
        linux_wl_compositor_readiness_report,
    };

    /// 未显式初始化时，owner wiring 可用但 global/state owner 仍为 false。
    #[test]
    fn linux_wl_compositor_global_initialization_is_explicit() {
        let report = linux_wl_compositor_readiness_report();

        assert!(report.global_owner_available);
        assert!(report.compositor_handler_available);
        assert!(report.delegate_compositor_wired);
        assert!(report.per_client_compositor_state_available);
        assert!(!report.compositor_state_new_invoked);
        assert!(!report.wl_compositor_global_initialized);
        assert!(!report.wl_compositor_state_owned);
        assert!(!report.wl_surface_owner_boundary_available);
        assert_eq!(
            report.blockers.first(),
            Some(&LinuxWlCompositorGlobalBlocker::MissingExplicitInitialization)
        );
    }

    /// 初始化后的 owner 报告只提升 server owner 事实，runtime 字段保持 false。
    #[test]
    fn linux_wl_compositor_report_keeps_client_runtime_false() {
        let report = build_linux_wl_compositor_readiness_report(true);

        assert!(report.compositor_state_new_invoked);
        assert!(report.wl_compositor_global_initialized);
        assert!(report.wl_compositor_state_owned);
        assert!(report.wl_surface_owner_boundary_available);
        assert!(!report.wl_surface_identity_registry_available);
        assert!(!report.client_connection_created);
        assert!(!report.event_queue_created);
        assert!(!report.registry_bind_attempted);
        assert!(!report.client_bound_wl_compositor);
        assert!(!report.client_harness_available);
        assert!(!report.xdg_surface_lifecycle_available);
        assert!(!report.xdg_toplevel_lifecycle_available);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(
            !report
                .blockers
                .contains(&LinuxWlCompositorGlobalBlocker::MissingExplicitInitialization)
        );
    }
}
