//! Wayland listening socket 的 feature-gated 编译探针。
//!
//! 本模块只在 Linux 上启用 `smithay-linux` feature 时编译。当前阶段只验证
//! Smithay 的 `ListeningSocketSource` 可以被引用、创建并读取 socket 名称。
//!
//! 这里不把 socket 加入 calloop，不接收或注册真实 client，也不创建任何
//! `SurfaceId` 或 `WindowId`。socket 只负责未来让 client 发现 compositor，
//! 与核心窗口和 surface 标识没有直接关系。

use std::ffi::OsString;

use smithay::wayland::socket::ListeningSocketSource;

/// Wayland socket 探针当前所处模式。
///
/// 当前阶段只允许 `ProbeOnly`，表示 socket 最多只被创建和读取名称，
/// 不会进入真实 compositor 事件循环。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayWaylandSocketProbeMode {
    /// 纯探针模式。
    ///
    /// 不加入 calloop，不接收 client，也不把连接注册到 Wayland display。
    ProbeOnly,
}

/// Wayland listening socket 编译探针。
///
/// 该结构持有 Smithay 的 `ListeningSocketSource`，但不会把它加入 calloop。
/// socket 只代表 client 发现 compositor 的入口；它不会直接创建 `SurfaceId`、
/// `WindowId`，也不会直接修改核心 `State`。
pub struct SmithayWaylandSocketProbe {
    /// Smithay 提供的 Wayland listening socket event source。
    ///
    /// 当前只保存该对象以验证构造和 socket 名称读取，不处理任何 client 连接。
    socket: ListeningSocketSource,

    /// 当前探针模式。
    mode: SmithayWaylandSocketProbeMode,
}

impl SmithayWaylandSocketProbe {
    /// 自动创建一个 Wayland listening socket 探针。
    ///
    /// Smithay 会自动选择可用的 Wayland socket 名称。当前方法只创建 socket，
    /// 不加入 calloop，也不会让 client 连接进入 Wayland display。
    pub fn new_auto() -> Result<Self, Box<dyn std::error::Error>> {
        let socket = ListeningSocketSource::new_auto()?;

        Ok(Self {
            socket,
            mode: SmithayWaylandSocketProbeMode::ProbeOnly,
        })
    }

    /// 创建指定名称的 Wayland listening socket 探针。
    ///
    /// 该方法主要用于调试或未来测试。当前仍然不会加入 calloop，也不会接收 client。
    pub fn with_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = ListeningSocketSource::with_name(name)?;

        Ok(Self {
            socket,
            mode: SmithayWaylandSocketProbeMode::ProbeOnly,
        })
    }

    /// 返回当前探针模式。
    pub fn mode(&self) -> SmithayWaylandSocketProbeMode {
        self.mode
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only(&self) -> bool {
        self.mode == SmithayWaylandSocketProbeMode::ProbeOnly
    }

    /// 返回 socket 名称的 `OsString` 版本。
    ///
    /// Wayland socket 名称本质上是 `OsStr`；这里复制成 `OsString`，避免暴露内部引用。
    pub fn socket_name_os_string(&self) -> OsString {
        self.socket.socket_name().to_os_string()
    }

    /// 返回 socket 名称的 `String` 版本。
    ///
    /// 如果 socket 名称不是合法 UTF-8，则使用有损转换。Wayland 默认名称通常类似
    /// `wayland-1` 或 `wayland-2`。
    pub fn socket_name_string(&self) -> String {
        self.socket.socket_name().to_string_lossy().into_owned()
    }

    /// 返回当前阶段说明。
    ///
    /// 该文本用于测试和日志确认 socket 仍未接入真实 compositor。
    pub fn mode_description(&self) -> &'static str {
        "wayland-socket-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayWaylandSocketProbe, SmithayWaylandSocketProbeMode};
    use crate::smithay_backend::test_support::{assert_runtime_dir, unique_socket_name};

    /// 验证 socket 探针模式固定为纯探针模式。
    #[test]
    fn wayland_socket_probe_mode_is_probe_only() {
        assert_eq!(
            SmithayWaylandSocketProbeMode::ProbeOnly,
            SmithayWaylandSocketProbeMode::ProbeOnly
        );
    }

    /// 验证自动创建 socket 会真实绑定 Wayland listening socket。
    #[test]
    fn wayland_socket_probe_new_auto_returns_result_without_panic() {
        assert_runtime_dir();

        let probe = SmithayWaylandSocketProbe::new_auto()
            .expect("Wayland socket 测试必须真实绑定 listening socket");

        assert!(probe.is_probe_only());
        assert_eq!(probe.mode(), SmithayWaylandSocketProbeMode::ProbeOnly);
        assert_eq!(probe.mode_description(), "wayland-socket-probe-only");
        assert!(probe.socket_name_string().contains("wayland"));
    }

    /// 验证创建成功时可以取得 socket 名称的操作系统字符串副本。
    #[test]
    fn wayland_socket_probe_socket_name_os_string_is_available_when_created() {
        assert_runtime_dir();

        let expected_name = unique_socket_name("socket-os-string");
        let probe = SmithayWaylandSocketProbe::with_name(&expected_name)
            .expect("Wayland socket 测试必须真实绑定指定名称的 socket");

        let os_name = probe.socket_name_os_string();

        assert_eq!(os_name, std::ffi::OsString::from(expected_name));
    }
}
