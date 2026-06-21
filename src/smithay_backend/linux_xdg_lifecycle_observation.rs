//! Linux-only Smithay lifecycle signal 到 adapter identity lookup 的观察边界。
//!
//! 本模块接收真实 `ToplevelSurface` 引用，但只提取 key 并只读查询 registry。
//! 它不保存 protocol object，不删除 mapping，也不调用 admission ledger 或 core。

use smithay::wayland::shell::xdg::ToplevelSurface;

use super::linux_xdg_toplevel_identity::{
    LinuxXdgToplevelIdentityOperationError, LinuxXdgToplevelIdentityRegistry,
    LinuxXdgToplevelIdentitySourceError,
};
use super::surface_xdg_admission::AdapterSurfaceId;
use super::xdg_lifecycle_observation::{
    XdgToplevelLifecycleObservationError, XdgToplevelLifecycleObservationReport,
    XdgToplevelLifecycleSignal,
};

/// 从 callback-like ToplevelSurface 参数执行一次只读 identity lookup。
///
/// `expected_surface` 只用于核对 ownership；无论成功失败，本函数都不会调用
/// ledger/core，也不会 remove registry mapping。Helper 可编译不代表 runtime callback
/// 已真实发生，因此 report 的 `callback_observed` 保持 false。
pub fn observe_toplevel_lifecycle(
    registry: &LinuxXdgToplevelIdentityRegistry,
    signal: XdgToplevelLifecycleSignal,
    toplevel: &ToplevelSurface,
    expected_surface: Option<AdapterSurfaceId>,
) -> XdgToplevelLifecycleObservationReport {
    match registry.lookup_toplevel(toplevel) {
        Ok(mapping) => XdgToplevelLifecycleObservationReport::from_lookup(
            signal,
            Ok(mapping),
            expected_surface,
        ),
        Err(LinuxXdgToplevelIdentityOperationError::Mapping(source)) => {
            XdgToplevelLifecycleObservationReport::from_lookup(
                signal,
                Err(source),
                expected_surface,
            )
        }
        Err(LinuxXdgToplevelIdentityOperationError::Source(
            LinuxXdgToplevelIdentitySourceError::SmithayIdentityUnavailable,
        )) => XdgToplevelLifecycleObservationReport::identity_source_failed(
            signal,
            XdgToplevelLifecycleObservationError::IdentitySourceUnavailable,
        ),
        Err(LinuxXdgToplevelIdentityOperationError::Source(
            LinuxXdgToplevelIdentitySourceError::IdentitySourceNotStable,
        )) => XdgToplevelLifecycleObservationReport::identity_source_failed(
            signal,
            XdgToplevelLifecycleObservationError::IdentitySourceNotStable,
        ),
    }
}

/// Phase 52G observation boundary 后仍存在的 lifecycle/runtime blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgToplevelLifecycleBlocker {
    /// 尚未由运行中的 xdg-shell global 证明真实 callback invocation。
    MissingRealCallbackObservation,
    /// Production callback 尚无已注册 identity 的 runtime proof。
    MissingRegisteredIdentityRuntimeProof,
    /// Admission ledger 尚无明确 production caller owner。
    MissingLedgerCallerOwnership,
    /// xdg-shell global/runtime 尚未初始化。
    MissingXdgShellRuntime,
}

/// Linux callback-like lifecycle identity lookup 的保守 readiness report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxXdgToplevelLifecycleReadinessReport {
    /// Callback 参数到 registry lookup 的 helper 是否可用。
    pub callback_identity_lookup_available: bool,
    /// `toplevel_destroyed` handler hook 是否已连接 observation helper。
    pub lifecycle_signal_hook_available: bool,
    /// 是否已证明 production callback 实际执行 identity lookup。
    pub toplevel_identity_lookup_invoked: bool,
    /// 是否已由真实 callback resolve AdapterToplevelId。
    pub adapter_toplevel_id_resolved: bool,
    /// 是否已证明真实 runtime callback invocation。
    pub callback_observed: bool,
    /// 是否调用 ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// 是否调用 core detach。
    pub core_detach_invoked: bool,
    /// 真实 xdg-shell runtime 是否可用。
    pub real_xdg_shell_runtime_available: bool,
    /// protocol dispatch 是否启动。
    pub protocol_dispatch_started: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 尚未完成的 runtime/lifecycle 前置条件。
    pub blockers: Vec<LinuxXdgToplevelLifecycleBlocker>,
}

/// 返回 Phase 52G lifecycle observation readiness，不推导 runtime 能力。
pub fn linux_xdg_toplevel_lifecycle_readiness_report() -> LinuxXdgToplevelLifecycleReadinessReport {
    LinuxXdgToplevelLifecycleReadinessReport {
        callback_identity_lookup_available: true,
        lifecycle_signal_hook_available: true,
        toplevel_identity_lookup_invoked: false,
        adapter_toplevel_id_resolved: false,
        callback_observed: false,
        ledger_unmap_invoked: false,
        core_detach_invoked: false,
        real_xdg_shell_runtime_available: false,
        protocol_dispatch_started: false,
        render_support: false,
        input_support: false,
        blockers: vec![
            LinuxXdgToplevelLifecycleBlocker::MissingRealCallbackObservation,
            LinuxXdgToplevelLifecycleBlocker::MissingRegisteredIdentityRuntimeProof,
            LinuxXdgToplevelLifecycleBlocker::MissingLedgerCallerOwnership,
            LinuxXdgToplevelLifecycleBlocker::MissingXdgShellRuntime,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{LinuxXdgToplevelLifecycleBlocker, linux_xdg_toplevel_lifecycle_readiness_report};

    #[test]
    fn linux_xdg_lifecycle_observation_readiness_keeps_runtime_false() {
        let report = linux_xdg_toplevel_lifecycle_readiness_report();

        assert!(report.callback_identity_lookup_available);
        assert!(report.lifecycle_signal_hook_available);
        assert!(!report.toplevel_identity_lookup_invoked);
        assert!(!report.adapter_toplevel_id_resolved);
        assert!(!report.callback_observed);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert_eq!(
            report.blockers,
            vec![
                LinuxXdgToplevelLifecycleBlocker::MissingRealCallbackObservation,
                LinuxXdgToplevelLifecycleBlocker::MissingRegisteredIdentityRuntimeProof,
                LinuxXdgToplevelLifecycleBlocker::MissingLedgerCallerOwnership,
                LinuxXdgToplevelLifecycleBlocker::MissingXdgShellRuntime,
            ]
        );
    }
}
