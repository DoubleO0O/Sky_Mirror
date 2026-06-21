//! Linux-only xdg-shell global 与 request-handler 编译边界。
//!
//! 本模块只在 Linux + `smithay-linux` 下可见。它把 Smithay 的真实
//! `XdgShellHandler` / `GlobalDispatch` / `Dispatch` trait 实现定位到 display 内部的
//! `LinuxXdgShellStateSkeleton`，但不调用 `XdgShellState::new`，因此不会注册
//! global、不会收到真实 request，也不会触发 admission ledger 或核心状态变更。

use smithay::reexports::wayland_protocols::xdg::shell::server::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::XdgPositioner,
    xdg_surface::XdgSurface,
    xdg_toplevel::XdgToplevel,
    xdg_wm_base::XdgWmBase,
};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::{Client, DataInit, Dispatch, DisplayHandle};
use smithay::utils::Serial;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgPositionerUserData, XdgShellHandler,
    XdgShellState, XdgShellSurfaceUserData, XdgSurfaceUserData, XdgWmBaseUserData,
};

use super::wayland_display::SmithayWaylandState;

/// Wayland display 内部持有的 Linux-only xdg-shell handler state。
///
/// 该类型把既有公开 `SmithayWaylandState` 与未来 `XdgShellState` 所有权组合起来，
/// 避免修改旧 public struct 的字段形状。`xdg_shell_state` 当前保持 `None`，所以
/// handler trait 可编译并不意味着 xdg-shell global 已注册。
#[derive(Debug, Default)]
pub struct LinuxXdgShellStateSkeleton {
    wayland_state: SmithayWaylandState,
    xdg_shell_state: Option<XdgShellState>,
}

impl LinuxXdgShellStateSkeleton {
    /// 创建未初始化 protocol global 的 Linux-only handler state。
    pub fn new() -> Self {
        Self {
            wayland_state: SmithayWaylandState::new(),
            xdg_shell_state: None,
        }
    }

    /// 返回既有 Wayland probe state 的只读视图。
    pub(crate) const fn wayland_state(&self) -> &SmithayWaylandState {
        &self.wayland_state
    }

    /// 返回已初始化的 xdg-shell helper state。
    ///
    /// Phase 52E 不调用本方法；它只满足 Smithay handler trait 的所有权形状。
    /// 若未来在未注册 global 时错误进入 request dispatch，应明确失败，不能把
    /// 缺失的 runtime 初始化静默解释为可用能力。
    fn xdg_shell_state_mut(&mut self) -> &mut XdgShellState {
        self.xdg_shell_state
            .as_mut()
            .expect("xdg-shell global 尚未初始化；Phase 52E 只有编译边界")
    }
}

/// Phase 52E 编译边界之后仍阻止真实 xdg-shell runtime 的结构化缺口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgShellCompileBlocker {
    /// 尚未调用 `XdgShellState::new` 注册 xdg-shell global。
    MissingGlobalInitialization,
    /// 真实 `ToplevelSurface` 尚未映射为纯数据 `AdapterToplevelId`。
    MissingAdapterToplevelIdentityMapping,
    /// toplevel lifecycle signal 尚未桥接到 admission ledger。
    MissingToplevelLifecycleBridge,
    /// Linux adapter 尚未取得 admission ledger 的明确调用所有权。
    MissingLedgerCallerOwnership,
    /// Smithay popup delegation 依赖 `SeatHandler`，本阶段禁止跨入 input/seat。
    MissingPopupSeatHandlerBoundary,
}

/// Linux-only xdg-shell 编译边界的保守能力报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxXdgShellCompileReport {
    /// Linux-only 模块是否存在。
    pub linux_xdg_shell_module_available: bool,
    /// `xdg_wm_base` global dispatch 的类型边界是否可编译。
    pub xdg_shell_global_compile_boundary_available: bool,
    /// xdg-shell request dispatch 的类型边界是否可编译。
    pub xdg_request_handler_compile_boundary_available: bool,
    /// `ToplevelSurface` lifecycle callback 是否已有未来 identity 挂接位置。
    pub xdg_toplevel_identity_hook_point_available: bool,
    /// 是否已观察到真实 xdg_toplevel unmap callback。
    pub xdg_unmap_callback_observed: bool,
    /// Linux 边界是否已调用 ledger unmap。
    pub ledger_unmap_invoked_from_linux_boundary: bool,
    /// 真实 xdg-shell runtime 是否可用。
    pub real_xdg_shell_runtime_available: bool,
    /// protocol dispatch 是否已经启动。
    pub protocol_dispatch_started: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 阻止 compile seam 被解释为真实 runtime 的剩余缺口。
    pub blockers: Vec<LinuxXdgShellCompileBlocker>,
}

/// 返回 Phase 52E Linux-only xdg-shell 编译边界报告。
///
/// callback、ledger、runtime、protocol dispatch、render 与 input 必须保持 false；
/// handler trait 编译成功只证明类型和所有权位置，不证明客户端请求已经发生。
pub fn linux_xdg_shell_readiness_report() -> LinuxXdgShellCompileReport {
    LinuxXdgShellCompileReport {
        linux_xdg_shell_module_available: true,
        xdg_shell_global_compile_boundary_available: true,
        xdg_request_handler_compile_boundary_available: true,
        xdg_toplevel_identity_hook_point_available: true,
        xdg_unmap_callback_observed: false,
        ledger_unmap_invoked_from_linux_boundary: false,
        real_xdg_shell_runtime_available: false,
        protocol_dispatch_started: false,
        render_support: false,
        input_support: false,
        blockers: vec![
            LinuxXdgShellCompileBlocker::MissingGlobalInitialization,
            LinuxXdgShellCompileBlocker::MissingAdapterToplevelIdentityMapping,
            LinuxXdgShellCompileBlocker::MissingToplevelLifecycleBridge,
            LinuxXdgShellCompileBlocker::MissingLedgerCallerOwnership,
            LinuxXdgShellCompileBlocker::MissingPopupSeatHandlerBoundary,
        ],
    }
}

impl XdgShellHandler for LinuxXdgShellStateSkeleton {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        self.xdg_shell_state_mut()
    }

    fn new_toplevel(&mut self, _surface: ToplevelSurface) {
        // 真实 protocol object 不能进入 core；未来必须先转换为 AdapterToplevelId。
        // Phase 52E 不建立该映射，也不触发 admission 或 lifecycle mutation。
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // popup 不在本阶段范围内；compile seam 不代表 popup runtime 支持。
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        // input/seat 不在本阶段范围内，request 不能越界启动 input 行为。
    }

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
        // 这里只满足 Smithay trait 的编译形状，不处理真实 popup request。
    }

    fn toplevel_destroyed(&mut self, _surface: ToplevelSurface) {
        // 这是未来 identity hook 点，但当前没有 AdapterToplevelId 映射。
        // 因此不得调用 SurfaceXdgAdmissionLedger::unmap_toplevel，也不得声称
        // 已观察到真实 callback。
    }
}

// Smithay 的全量 delegate_xdg_shell! 会让 popup dispatch 要求 SeatHandler。
// 本阶段逐项生成 global 与非 popup request delegation，避免为编译证明伪造 input。
smithay::reexports::wayland_server::delegate_global_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgWmBase: ()
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgWmBase: XdgWmBaseUserData
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgPositioner: XdgPositionerUserData
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgSurface: XdgSurfaceUserData
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgToplevel: XdgShellSurfaceUserData
] => XdgShellState);

impl Dispatch<XdgPopup, XdgShellSurfaceUserData> for LinuxXdgShellStateSkeleton {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &XdgPopup,
        _request: xdg_popup::Request,
        _data: &XdgShellSurfaceUserData,
        _display_handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // 该实现只关闭 Smithay 类型图中的 popup trait 缺口。真实 global 尚未注册，
        // 所以生产中不可达；若未来误启动，必须 fail closed，不能静默伪装 popup/input。
        panic!("Phase 52E 不处理真实 xdg_popup request；SeatHandler 尚未接入")
    }
}

#[cfg(test)]
mod tests {
    use smithay::reexports::wayland_protocols::xdg::shell::server::{
        xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
    };
    use smithay::reexports::wayland_server::{Dispatch, GlobalDispatch};
    use smithay::wayland::shell::xdg::{XdgShellHandler, XdgShellSurfaceUserData};

    use super::LinuxXdgShellStateSkeleton;
    use super::{LinuxXdgShellCompileBlocker, linux_xdg_shell_readiness_report};

    /// 编译期证明 global、request handler 与 state owner 已连接。
    #[test]
    fn linux_xdg_shell_handler_traits_compile_for_wayland_state() {
        fn assert_handler<T: XdgShellHandler>() {}
        fn assert_global<T: GlobalDispatch<XdgWmBase, ()>>() {}
        fn assert_toplevel_dispatch<T: Dispatch<XdgToplevel, XdgShellSurfaceUserData>>() {}

        assert_handler::<LinuxXdgShellStateSkeleton>();
        assert_global::<LinuxXdgShellStateSkeleton>();
        assert_toplevel_dispatch::<LinuxXdgShellStateSkeleton>();
    }

    /// 编译边界不得夸大 callback、runtime、dispatch、render 或 input。
    #[test]
    fn linux_xdg_shell_readiness_keeps_runtime_false() {
        let report = linux_xdg_shell_readiness_report();

        assert!(report.linux_xdg_shell_module_available);
        assert!(report.xdg_shell_global_compile_boundary_available);
        assert!(report.xdg_request_handler_compile_boundary_available);
        assert!(report.xdg_toplevel_identity_hook_point_available);
        assert!(!report.xdg_unmap_callback_observed);
        assert!(!report.ledger_unmap_invoked_from_linux_boundary);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert_eq!(
            report.blockers,
            vec![
                LinuxXdgShellCompileBlocker::MissingGlobalInitialization,
                LinuxXdgShellCompileBlocker::MissingAdapterToplevelIdentityMapping,
                LinuxXdgShellCompileBlocker::MissingToplevelLifecycleBridge,
                LinuxXdgShellCompileBlocker::MissingLedgerCallerOwnership,
                LinuxXdgShellCompileBlocker::MissingPopupSeatHandlerBoundary,
            ]
        );
    }
}
