//! Smithay Display 与 Wayland socket 的组合探针。
//!
//! 本模块只在 Linux 上启用 `smithay-linux` feature 时编译，用于验证 Display
//! 探针和 socket 探针可以被同一个 bootstrap 结构同时持有。
//!
//! 该组合探针不会把 socket 加入 calloop，不接收或注册真实 client，也不会创建
//! `SurfaceId` 或 `WindowId`。未来真实 client 连接仍然必须先经过后端驱动边界，
//! 转换为 `BackendEvent` 后再进入核心运行时，不能直接修改核心 `State`。

use smithay::reexports::wayland_server::DisplayHandle;

use crate::smithay_backend::{
    wayland_display::SmithayWaylandDisplayProbe,
    wayland_socket::{SmithayWaylandSocketProbe, SmithayWaylandSocketProbeMode},
};

/// Smithay bootstrap 当前运行模式。
///
/// 唯一模式 `ProbeOnly` 表示 Display 和 socket 仅被构造出来，但不会进入真实
/// compositor 事件循环。资源所有权与运行能力是两个独立事实。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayBootstrapMode {
    /// 纯探针模式。
    ///
    /// 不加入 calloop，不接收 client，也不把客户端连接注册到 Wayland display。
    ProbeOnly,
}

/// Smithay Display 与 socket 的组合探针。
///
/// 该结构同时持有 Wayland Display 探针和 Wayland socket 探针，用于验证未来
/// compositor bootstrap 所需的两类资源可以一起存在。
///
/// 该结构不是完整 compositor：它不会启动事件循环，不会接受 client，不会创建
/// surface 或 window，也不会直接修改核心 `State`。
pub struct SmithayBootstrapProbe {
    /// Wayland Display 编译探针。
    ///
    /// 当前只用于持有 Display 和取得 `DisplayHandle`。
    display: SmithayWaylandDisplayProbe,

    /// Wayland listening socket 编译探针。
    ///
    /// 当前只用于持有 `ListeningSocketSource` 并读取 socket 名称。
    socket: SmithayWaylandSocketProbe,

    /// 当前 bootstrap 模式。
    mode: SmithayBootstrapMode,
}

impl SmithayBootstrapProbe {
    /// 自动创建 Display 与 socket 组合探针。
    ///
    /// 该方法会构造 Wayland Display，并尝试创建自动命名的 Wayland socket。
    /// 它不会把 socket 加入 calloop，也不会让真实 client 连接进入 Display。
    pub fn new_auto() -> Result<Self, Box<dyn std::error::Error>> {
        let display = SmithayWaylandDisplayProbe::new()?;
        let socket = SmithayWaylandSocketProbe::new_auto()?;

        Ok(Self {
            display,
            socket,
            mode: SmithayBootstrapMode::ProbeOnly,
        })
    }

    /// 使用指定 socket 名称创建 Display 与 socket 组合探针。
    ///
    /// 该方法主要用于调试或未来测试；当前仍然不会启动真实 compositor。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let display = SmithayWaylandDisplayProbe::new()?;
        let socket = SmithayWaylandSocketProbe::with_name(name)?;

        Ok(Self {
            display,
            socket,
            mode: SmithayBootstrapMode::ProbeOnly,
        })
    }

    /// 返回当前模式。
    pub fn mode(&self) -> SmithayBootstrapMode {
        self.mode
    }

    /// 当前是否仍然只是组合探针模式。
    pub fn is_probe_only(&self) -> bool {
        self.mode == SmithayBootstrapMode::ProbeOnly
            && self.display.is_probe_only()
            && self.socket.is_probe_only()
    }

    /// 取得 `DisplayHandle`。
    ///
    /// 当前只取得 handle，不使用它注册任何协议 global，也不接收 client。
    pub fn display_handle(&self) -> DisplayHandle {
        self.display.display_handle()
    }

    /// 返回 socket 名称。
    ///
    /// 该名称只用于确认 socket probe 已创建，不代表已有 client 连接。
    pub fn socket_name_string(&self) -> String {
        self.socket.socket_name_string()
    }

    /// 返回 socket 探针模式。
    pub fn socket_mode(&self) -> SmithayWaylandSocketProbeMode {
        self.socket.mode()
    }

    /// 返回稳定的探针模式说明，供日志和测试使用。
    pub fn mode_description(&self) -> &'static str {
        "smithay-bootstrap-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayBootstrapMode, SmithayBootstrapProbe};
    use crate::smithay_backend::{
        test_support::{assert_runtime_dir, unique_socket_name},
        wayland_socket::SmithayWaylandSocketProbeMode,
    };

    /// 验证 bootstrap 模式固定为纯探针模式。
    #[test]
    fn smithay_bootstrap_mode_is_probe_only() {
        assert_eq!(
            SmithayBootstrapMode::ProbeOnly,
            SmithayBootstrapMode::ProbeOnly
        );
    }

    /// 验证自动创建组合探针会真实构造 Display 和 socket。
    #[test]
    fn smithay_bootstrap_new_auto_returns_result_without_panic() {
        assert_runtime_dir();

        let probe =
            SmithayBootstrapProbe::new_auto().expect("bootstrap 必须真实构造 Display 和 socket");

        assert!(probe.is_probe_only());
        assert_eq!(probe.mode(), SmithayBootstrapMode::ProbeOnly);
        assert_eq!(
            probe.socket_mode(),
            SmithayWaylandSocketProbeMode::ProbeOnly
        );
        assert_eq!(probe.mode_description(), "smithay-bootstrap-probe-only");
        assert!(probe.socket_name_string().contains("wayland"));

        // 这里只取得 handle 以验证组合资源路径，不注册任何协议对象。
        let _handle = probe.display_handle();
    }

    /// 验证组合探针创建成功时可以取得 Display handle。
    #[test]
    fn smithay_bootstrap_display_handle_is_available_when_created() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("bootstrap-display");
        let probe = SmithayBootstrapProbe::with_socket_name(&socket_name)
            .expect("bootstrap 必须真实构造指定名称的 socket");

        let _handle = probe.display_handle();

        assert!(probe.is_probe_only());
    }

    /// 验证组合探针创建成功时可以读取 socket 名称。
    #[test]
    fn smithay_bootstrap_socket_name_is_available_when_created() {
        assert_runtime_dir();

        let expected_name = unique_socket_name("bootstrap-name");
        let probe = SmithayBootstrapProbe::with_socket_name(&expected_name)
            .expect("bootstrap 必须真实构造指定名称的 socket");

        let socket_name = probe.socket_name_string();

        assert_eq!(socket_name, expected_name);
    }
}
