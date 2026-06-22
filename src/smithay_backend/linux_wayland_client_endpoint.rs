//! Linux-only Wayland client dependency and compile/import seam.
//!
//! 本模块只证明 `wayland-client` 与 xdg-shell client 类型在 `smithay-linux` 下可引用。
//! 它不创建 connection、event queue，不 bind global，也不进入 ledger/core。

use wayland_client::Connection;
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_registry::WlRegistry, wl_surface::WlSurface,
};
use wayland_protocols::xdg::shell::client::{
    xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
};

/// Phase 52L compile seam 之后仍未具备的 runtime 前置条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaylandClientEndpointCompileBlocker {
    /// 尚未定义 runtime connection owner。
    MissingRuntimeConnectionOwner,
    /// 尚未定义 client event queue owner。
    MissingEventQueueOwner,
    /// 尚未定义 registry discovery 与 global bind driver。
    MissingRegistryBindDriver,
    /// 尚未实现 `wl_compositor` global 与 `wl_surface` owner。
    MissingWlCompositorSurfaceOwner,
    /// 尚未实现受控 client/toplevel lifecycle harness。
    MissingControlledLifecycleHarness,
    /// `new_toplevel` 尚无 production identity registration owner。
    MissingNewToplevelIdentityRegistrationOwner,
}

/// Linux-only Wayland client 类型 import 的保守 readiness report。
///
/// `true` 字段只说明依赖和类型可编译；不代表创建了 client、绑定了 global，
/// 更不代表 protocol dispatch、xdg-shell runtime 或 compositor 已可用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxWaylandClientEndpointCompileReport {
    /// `wayland-client` optional dependency 已通过 `smithay-linux` 可见。
    pub wayland_client_dependency_available: bool,
    /// `wayland-protocols` 的 client feature 已通过 `smithay-linux` 可见。
    pub wayland_protocols_client_feature_available: bool,
    /// Linux-only client imports 已通过 Rust 类型检查。
    pub linux_client_imports_compile: bool,
    /// `xdg_wm_base` client proxy 类型可引用。
    pub xdg_wm_base_client_type_available: bool,
    /// `xdg_surface` client proxy 类型可引用。
    pub xdg_surface_client_type_available: bool,
    /// `xdg_toplevel` client proxy 类型可引用。
    pub xdg_toplevel_client_type_available: bool,
    /// 本阶段没有创建 client connection。
    pub runtime_connection_created: bool,
    /// 本阶段没有创建 client event queue。
    pub event_queue_created: bool,
    /// 本阶段没有尝试 registry/global bind。
    pub registry_bind_attempted: bool,
    /// 本阶段没有 controlled client harness。
    pub client_harness_available: bool,
    /// 本阶段没有尝试 client bind。
    pub client_bind_attempted: bool,
    /// 本阶段没有证明 client 已绑定 xdg-shell global。
    pub client_bound_xdg_shell_global: bool,
    /// 本阶段没有 controlled toplevel lifecycle。
    pub controlled_toplevel_lifecycle_available: bool,
    /// `new_toplevel` 尚无 production identity registration owner。
    pub new_toplevel_identity_registration_owner_available: bool,
    /// 本阶段没有观察到真实 runtime callback。
    pub callback_observed: bool,
    /// 本阶段没有调用 admission ledger。
    pub ledger_admit_invoked: bool,
    /// 本阶段没有调用 removal ledger。
    pub ledger_unmap_invoked: bool,
    /// 本阶段没有触发 core register mutation。
    pub core_register_invoked: bool,
    /// 本阶段没有触发 core detach mutation。
    pub core_detach_invoked: bool,
    /// dependency/import seam 不会启动 protocol dispatch。
    pub protocol_dispatch_started: bool,
    /// dependency/import seam 不代表真实 xdg-shell runtime 可用。
    pub real_xdg_shell_runtime_available: bool,
    /// 本阶段没有 render 支持。
    pub render_support: bool,
    /// 本阶段没有 input 支持。
    pub input_support: bool,
    /// 阻止 compile seam 被提升为 runtime capability 的显式前置条件。
    pub blockers: Vec<WaylandClientEndpointCompileBlocker>,
}

/// 在不构造实例的前提下要求编译器解析 client-side 类型。
fn require_client_type<T>() {
    let _ = std::any::type_name::<T>();
}

/// 返回 Linux-only Wayland client dependency/import readiness。
///
/// 这里刻意只引用类型。Phase 52L 不创建 `Connection`、不创建 event queue、
/// 不 bind registry/xdg global，也不实现 `wl_compositor`/`wl_surface` owner。
pub fn linux_wayland_client_endpoint_compile_report() -> LinuxWaylandClientEndpointCompileReport {
    // 真实 client object 只能留在 Linux adapter；core 只接受纯数据 identity/command。
    require_client_type::<Connection>();
    require_client_type::<WlRegistry>();
    require_client_type::<WlCompositor>();
    require_client_type::<WlSurface>();
    require_client_type::<XdgWmBase>();
    require_client_type::<XdgSurface>();
    require_client_type::<XdgToplevel>();

    LinuxWaylandClientEndpointCompileReport {
        wayland_client_dependency_available: true,
        wayland_protocols_client_feature_available: true,
        linux_client_imports_compile: true,
        xdg_wm_base_client_type_available: true,
        xdg_surface_client_type_available: true,
        xdg_toplevel_client_type_available: true,
        runtime_connection_created: false,
        event_queue_created: false,
        registry_bind_attempted: false,
        client_harness_available: false,
        client_bind_attempted: false,
        client_bound_xdg_shell_global: false,
        controlled_toplevel_lifecycle_available: false,
        new_toplevel_identity_registration_owner_available: false,
        callback_observed: false,
        ledger_admit_invoked: false,
        ledger_unmap_invoked: false,
        core_register_invoked: false,
        core_detach_invoked: false,
        protocol_dispatch_started: false,
        real_xdg_shell_runtime_available: false,
        render_support: false,
        input_support: false,
        blockers: vec![
            WaylandClientEndpointCompileBlocker::MissingRuntimeConnectionOwner,
            WaylandClientEndpointCompileBlocker::MissingEventQueueOwner,
            WaylandClientEndpointCompileBlocker::MissingRegistryBindDriver,
            WaylandClientEndpointCompileBlocker::MissingWlCompositorSurfaceOwner,
            WaylandClientEndpointCompileBlocker::MissingControlledLifecycleHarness,
            WaylandClientEndpointCompileBlocker::MissingNewToplevelIdentityRegistrationOwner,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        WaylandClientEndpointCompileBlocker, linux_wayland_client_endpoint_compile_report,
    };

    #[test]
    fn linux_wayland_client_compile_seam_imports_client_xdg_types() {
        let report = linux_wayland_client_endpoint_compile_report();

        assert!(report.wayland_client_dependency_available);
        assert!(report.wayland_protocols_client_feature_available);
        assert!(report.linux_client_imports_compile);
        assert!(report.xdg_wm_base_client_type_available);
        assert!(report.xdg_surface_client_type_available);
        assert!(report.xdg_toplevel_client_type_available);
    }

    #[test]
    fn linux_wayland_client_compile_seam_keeps_harness_false() {
        let report = linux_wayland_client_endpoint_compile_report();

        assert!(!report.runtime_connection_created);
        assert!(!report.event_queue_created);
        assert!(!report.registry_bind_attempted);
        assert!(!report.client_harness_available);
        assert!(!report.client_bind_attempted);
        assert!(!report.client_bound_xdg_shell_global);
        assert!(!report.controlled_toplevel_lifecycle_available);
        assert!(!report.new_toplevel_identity_registration_owner_available);
        assert!(!report.callback_observed);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert_eq!(
            report.blockers,
            vec![
                WaylandClientEndpointCompileBlocker::MissingRuntimeConnectionOwner,
                WaylandClientEndpointCompileBlocker::MissingEventQueueOwner,
                WaylandClientEndpointCompileBlocker::MissingRegistryBindDriver,
                WaylandClientEndpointCompileBlocker::MissingWlCompositorSurfaceOwner,
                WaylandClientEndpointCompileBlocker::MissingControlledLifecycleHarness,
                WaylandClientEndpointCompileBlocker::MissingNewToplevelIdentityRegistrationOwner,
            ]
        );
    }
}
