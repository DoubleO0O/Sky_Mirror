//! Wayland Display 的 feature-gated 编译探针。
//!
//! 本模块只在 Linux 上启用 `smithay-linux` feature 时编译。当前阶段只验证
//! 可以构造 Wayland server `Display`、取得 `DisplayHandle`，并为 Linux runtime
//! proof 执行一次 client dispatch。这里不创建监听 socket；虽然内部 state 具备
//! xdg-shell trait 编译形状，但不注册任何 shell/compositor global，也不把 dispatch
//! 解释为完整 compositor 事件循环。
//!
//! 未来真实 Wayland callback 仍然必须先转换为 `BackendEvent`，再通过
//! `CoreRuntimeBridge` 进入核心状态，不能直接修改 workspace 或注册表。

use smithay::reexports::wayland_server::{Display, DisplayHandle};

use super::linux_xdg_shell::LinuxXdgShellStateSkeleton;

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
/// socket、不处理 shell request，也不注册 xdg-shell global。
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
    /// 该过程不会创建 socket、协议 global 或真实 surface。
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
    use super::SmithayWaylandDisplayProbe;

    /// 验证 Wayland Display 探针可以构造。
    #[test]
    fn wayland_display_probe_can_be_created() {
        let probe = SmithayWaylandDisplayProbe::new().expect("Wayland Display 探针必须能够构造");

        // 构造成功后仍必须明确处于纯探针模式。
        assert!(probe.is_probe_only());
        assert_eq!(probe.mode_description(), "wayland-display-probe-only");
    }

    /// 验证可以取得 `DisplayHandle`，但不注册任何 protocol global。
    #[test]
    fn wayland_display_probe_can_get_display_handle() {
        let probe = SmithayWaylandDisplayProbe::new().expect("Wayland Display 探针必须能够构造");

        // 这里只取得 handle 以验证类型边界，不使用它创建任何协议对象。
        let _handle = probe.display_handle();

        assert!(probe.is_probe_only());
    }
}
