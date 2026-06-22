//! Wayland Display 的 feature-gated 编译探针。
//!
//! 本模块只在 Linux 上启用 `smithay-linux` feature 时编译。当前阶段只验证
//! 可以构造 Wayland server `Display`、取得 `DisplayHandle`，并为 Linux runtime
//! proof 执行一次 client dispatch。这里不创建监听 socket；虽然内部 state 具备
//! xdg-shell trait 编译形状，并可由 owner 显式注册 xdg-shell global；构造函数仍不
//! 自动注册。Global 初始化与 client dispatch 分离，也不解释为完整 compositor loop。
//!
//! 未来真实 Wayland callback 仍然必须先转换为 `BackendEvent`，再通过
//! `CoreRuntimeBridge` 进入核心状态，不能直接修改 workspace 或注册表。

use smithay::reexports::wayland_server::{Display, DisplayHandle};

use super::linux_xdg_shell::{
    LinuxXdgShellGlobalInitError, LinuxXdgShellGlobalInitReport, LinuxXdgShellStateSkeleton,
};

/// Smithay 和 Wayland server 侧的最小状态占位。
///
/// 真实 Smithay compositor 后续会把 protocol state、seat、output 和 shell state
/// 等放入类似结构中。当前阶段只用它作为 `Display<State>` 的泛型参数，证明
/// display 类型边界可以编译。
#[derive(Debug, Default)]
pub struct SmithayWaylandState {
    /// 当前是否只是 display probe。
    ///
    /// 该字段没有业务含义，只用于测试确认本阶段没有启动真实 compositor。
    pub probe_only: bool,
}

impl SmithayWaylandState {
    /// 创建最小 Wayland state 占位。
    ///
    /// 当前不会创建任何 protocol global，也不会接入核心 `State`。
    pub fn new() -> Self {
        Self { probe_only: true }
    }
}

/// Wayland Display 编译探针。
///
/// 该结构持有一个 `wayland_server::Display<LinuxXdgShellStateSkeleton>`，用于确认
/// feature-gated Wayland server 与 xdg-shell handler owner 的类型可以组合。它不创建
/// socket，也不在构造时注册 xdg-shell global。显式初始化不会创建 client harness、
/// 启动 dispatch，或证明 callback/runtime 可用。
pub struct SmithayWaylandDisplayProbe {
    /// Wayland server display。
    ///
    /// 当前只用于编译和构造验证，不参与主事件循环。
    display: Display<LinuxXdgShellStateSkeleton>,

    /// 与 display 配套的最小状态占位。
    ///
    /// 真实 compositor 后续会在这里保存 Smithay protocol helper state。
    state: LinuxXdgShellStateSkeleton,
}

impl SmithayWaylandDisplayProbe {
    /// 创建 Wayland Display 编译探针。
    ///
    /// 如果 Wayland display 初始化失败，直接把错误向上传递给调用方。
    /// 该过程不会创建 socket、协议 global 或真实 surface；global 必须显式初始化。
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let display = Display::<LinuxXdgShellStateSkeleton>::new()?;
        let state = LinuxXdgShellStateSkeleton::new();

        Ok(Self { display, state })
    }

    /// 取得 `DisplayHandle`。
    ///
    /// 该方法只验证 handle 路径可用，不用 handle 注册任何 protocol global。
    pub fn display_handle(&self) -> DisplayHandle {
        self.display.handle()
    }

    /// 使用当前 display 自己的 handle 显式初始化 xdg-shell global。
    ///
    /// 公开入口位于同时持有 display 与 handler state 的 owner 上，因此外部不能把
    /// 不匹配的 `DisplayHandle` 注入 state。该操作不 dispatch client request，
    /// 不创建 client harness，也不触发 callback、ledger 或 core mutation。
    pub fn initialize_xdg_shell_global(
        &mut self,
    ) -> Result<LinuxXdgShellGlobalInitReport, LinuxXdgShellGlobalInitError> {
        let display_handle = self.display.handle();
        self.state.initialize_xdg_shell_global(&display_handle)
    }

    /// 返回当前 owner 是否已持有 `XdgShellState`。
    pub fn is_xdg_shell_global_initialized(&self) -> bool {
        self.state.is_xdg_shell_global_initialized()
    }

    /// 返回当前 xdg-shell global owner readiness；查询不会产生 mutation。
    pub fn xdg_shell_global_readiness_report(&self) -> LinuxXdgShellGlobalInitReport {
        self.state.xdg_shell_global_readiness_report()
    }

    /// 执行一次 Wayland backend client dispatch，并返回处理的 request 数量。
    ///
    /// 该 Linux-only seam 只用于让 backend 观察真实 peer EOF，从而触发其持有的
    /// `ClientData::disconnected`。callback 仍只能写 adapter session event，不能访问
    /// core；本方法也不注册 protocol global 或启动长期 compositor loop。
    pub(crate) fn dispatch_clients_once(&mut self) -> std::io::Result<usize> {
        self.display.dispatch_clients(&mut self.state)
    }

    /// 返回当前是否仍然只是 probe 模式。
    pub fn is_probe_only(&self) -> bool {
        self.state.wayland_state().probe_only
    }

    /// 返回当前模式说明。
    ///
    /// 该文本用于测试和日志确认当前阶段不会启动真实 compositor。
    pub fn mode_description(&self) -> &'static str {
        "wayland-display-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use crate::smithay_backend::linux_xdg_shell::{
        LinuxXdgShellGlobalBlocker, LinuxXdgShellGlobalInitError,
    };

    use super::SmithayWaylandDisplayProbe;

    /// 验证 Wayland Display 探针可以构造。
    #[test]
    fn wayland_display_probe_can_be_created() {
        let probe = SmithayWaylandDisplayProbe::new().expect("Wayland Display 探针必须能够构造");

        // 构造成功后仍必须明确处于纯探针模式。
        assert!(probe.is_probe_only());
        assert_eq!(probe.mode_description(), "wayland-display-probe-only");
        assert!(!probe.is_xdg_shell_global_initialized());
        let report = probe.xdg_shell_global_readiness_report();
        assert!(!report.xdg_shell_state_new_invoked);
        assert!(!report.xdg_shell_global_initialized);
        assert!(!report.xdg_shell_state_owned);
        assert_eq!(
            report.blockers,
            vec![
                LinuxXdgShellGlobalBlocker::MissingExplicitInitialization,
                LinuxXdgShellGlobalBlocker::MissingControlledClientHarness,
                LinuxXdgShellGlobalBlocker::MissingNewToplevelRegistrationOwner,
                LinuxXdgShellGlobalBlocker::MissingDispatchDrivenCallbackProof,
            ]
        );
    }

    /// 验证可以取得 `DisplayHandle`，但不注册任何 protocol global。
    #[test]
    fn wayland_display_probe_can_get_display_handle() {
        let probe = SmithayWaylandDisplayProbe::new().expect("Wayland Display 探针必须能够构造");

        // 这里只取得 handle 以验证类型边界，不使用它创建任何协议对象。
        let _handle = probe.display_handle();

        assert!(probe.is_probe_only());
    }

    /// 验证显式初始化后，配对的 display owner 持有真实 `XdgShellState`。
    #[test]
    fn linux_xdg_shell_global_owner_initializes_explicitly() {
        let mut probe =
            SmithayWaylandDisplayProbe::new().expect("Wayland Display 探针必须能够构造");

        let report = probe
            .initialize_xdg_shell_global()
            .expect("首次显式初始化必须成功");

        assert!(probe.is_xdg_shell_global_initialized());
        assert!(report.global_owner_available);
        assert!(report.xdg_shell_state_new_invoked);
        assert!(report.xdg_shell_global_initialized);
        assert!(report.xdg_shell_state_owned);
        assert!(!report.client_harness_available);
        assert!(!report.new_toplevel_registration_owner_available);
        assert!(!report.callback_observed);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert_eq!(
            report.blockers,
            vec![
                LinuxXdgShellGlobalBlocker::MissingControlledClientHarness,
                LinuxXdgShellGlobalBlocker::MissingNewToplevelRegistrationOwner,
                LinuxXdgShellGlobalBlocker::MissingDispatchDrivenCallbackProof,
            ]
        );
    }

    /// 验证重复初始化结构化拒绝，并保留首次创建的 state。
    #[test]
    fn linux_xdg_shell_global_init_rejects_duplicate_initialization() {
        let mut probe =
            SmithayWaylandDisplayProbe::new().expect("Wayland Display 探针必须能够构造");
        probe
            .initialize_xdg_shell_global()
            .expect("首次显式初始化必须成功");

        let duplicate = probe.initialize_xdg_shell_global();

        assert_eq!(
            duplicate,
            Err(LinuxXdgShellGlobalInitError::AlreadyInitialized)
        );
        assert!(probe.is_xdg_shell_global_initialized());
        let report = probe.xdg_shell_global_readiness_report();
        assert!(report.xdg_shell_state_new_invoked);
        assert!(report.xdg_shell_global_initialized);
        assert!(report.xdg_shell_state_owned);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.callback_observed);
    }
}
