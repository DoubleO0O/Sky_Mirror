#![cfg(all(feature = "smithay-linux", target_os = "linux"))]

//! Linux Smithay adapter 的受控结构骨架。
//!
//! 本模块只定义资源所有权、能力边界和生命周期转换。它不启动调度循环、不接受
//! 客户端，也不注册协议对象。底层 Display 与 listening socket 继续由
//! `SmithayBootstrapProbe` 封装，本模块不暴露其内部系统类型。

use std::{error::Error, fmt};

use crate::smithay_backend::bootstrap::SmithayBootstrapProbe;

/// Smithay Linux adapter skeleton 的生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterLifecycle {
    /// 资源已准备，但尚未启动任何真实 compositor 行为。
    Prepared,

    /// 已请求关闭，等待完成受控收尾。
    ShutdownRequested,

    /// skeleton 生命周期已经结束。
    Stopped,
}

/// Smithay Linux adapter skeleton 当前具备的保守能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterCapabilities {
    /// 是否持有 Wayland Display 资源。
    pub holds_wayland_display: bool,

    /// 是否持有 Wayland listening socket。
    pub holds_listening_socket: bool,

    /// 是否提供显式 adapter 生命周期边界。
    pub has_adapter_lifecycle_boundary: bool,

    /// 是否运行调度循环。
    pub runs_event_loop: bool,

    /// 是否接受真实客户端连接。
    pub accepts_clients: bool,

    /// 是否注册协议对象。
    pub registers_protocol_globals: bool,

    /// 是否接入真实 Wayland surface。
    pub supports_real_wayland_surfaces: bool,

    /// 是否接入 GPU 渲染。
    pub supports_gpu_rendering: bool,

    /// 当前实现是否仍然仅为结构骨架。
    pub is_skeleton_only: bool,
}

impl SmithayLinuxAdapterCapabilities {
    /// 返回 Phase 48A adapter skeleton 的固定保守能力集合。
    pub const fn skeleton_only() -> Self {
        Self {
            holds_wayland_display: true,
            holds_listening_socket: true,
            has_adapter_lifecycle_boundary: true,
            runs_event_loop: false,
            accepts_clients: false,
            registers_protocol_globals: false,
            supports_real_wayland_surfaces: false,
            supports_gpu_rendering: false,
            is_skeleton_only: true,
        }
    }
}

/// Smithay Linux adapter skeleton 支持的生命周期操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterOperation {
    /// 请求进入关闭流程。
    RequestShutdown,

    /// 完成已经请求的关闭流程。
    FinishShutdown,
}

/// Smithay Linux adapter skeleton 的结构化错误。
#[derive(Debug)]
pub enum SmithayLinuxAdapterError {
    /// Display 或 listening socket 初始化失败。
    ResourceInitialization {
        /// 保留可跨线程传递的底层错误信息。
        source: Box<dyn Error + Send + Sync>,
    },

    /// 请求的生命周期操作不适用于当前状态。
    InvalidLifecycleTransition {
        /// 收到操作时的生命周期状态。
        from: SmithayLinuxAdapterLifecycle,

        /// 被拒绝的生命周期操作。
        operation: SmithayLinuxAdapterOperation,
    },
}

impl fmt::Display for SmithayLinuxAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceInitialization { source } => {
                write!(formatter, "Smithay Linux adapter 资源初始化失败: {source}")
            }
            Self::InvalidLifecycleTransition { from, operation } => write!(
                formatter,
                "Smithay Linux adapter 生命周期转换无效: state={from:?}, operation={operation:?}"
            ),
        }
    }
}

impl Error for SmithayLinuxAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResourceInitialization { source } => Some(source.as_ref()),
            Self::InvalidLifecycleTransition { .. } => None,
        }
    }
}

/// 真实 Smithay adapter 的 Phase 48A 结构骨架。
///
/// 该结构只持有已经封装的 Linux bootstrap 资源和显式生命周期状态。它不提供
/// bootstrap 可变访问，不暴露系统 handle，也没有启动真实 compositor 的入口。
pub struct SmithayLinuxAdapterSkeleton {
    /// 已封装的 Display 与 listening socket 资源。
    bootstrap: SmithayBootstrapProbe,

    /// 当前 adapter skeleton 生命周期。
    lifecycle: SmithayLinuxAdapterLifecycle,
}

impl SmithayLinuxAdapterSkeleton {
    /// 使用自动选择的 socket 名称构造 adapter skeleton。
    pub fn new_auto() -> Result<Self, SmithayLinuxAdapterError> {
        let bootstrap = SmithayBootstrapProbe::new_auto().map_err(resource_initialization_error)?;

        Ok(Self::from_bootstrap(bootstrap))
    }

    /// 使用指定 socket 名称构造 adapter skeleton。
    pub fn with_socket_name(
        socket_name: impl Into<String>,
    ) -> Result<Self, SmithayLinuxAdapterError> {
        let socket_name = socket_name.into();
        let bootstrap = SmithayBootstrapProbe::with_socket_name(&socket_name)
            .map_err(resource_initialization_error)?;

        Ok(Self::from_bootstrap(bootstrap))
    }

    /// 使用已构造的 bootstrap 资源创建 adapter skeleton。
    pub fn from_bootstrap(bootstrap: SmithayBootstrapProbe) -> Self {
        Self {
            bootstrap,
            lifecycle: SmithayLinuxAdapterLifecycle::Prepared,
        }
    }

    /// 返回当前生命周期状态。
    pub fn lifecycle(&self) -> SmithayLinuxAdapterLifecycle {
        self.lifecycle
    }

    /// 返回固定的保守能力集合。
    pub fn capabilities(&self) -> SmithayLinuxAdapterCapabilities {
        SmithayLinuxAdapterCapabilities::skeleton_only()
    }

    /// 返回 bootstrap 已绑定的 listening socket 名称。
    pub fn socket_name_string(&self) -> String {
        self.bootstrap.socket_name_string()
    }

    /// 从 `Prepared` 转换到 `ShutdownRequested`。
    pub fn request_shutdown(&mut self) -> Result<(), SmithayLinuxAdapterError> {
        self.transition(
            SmithayLinuxAdapterLifecycle::Prepared,
            SmithayLinuxAdapterLifecycle::ShutdownRequested,
            SmithayLinuxAdapterOperation::RequestShutdown,
        )
    }

    /// 从 `ShutdownRequested` 转换到 `Stopped`。
    pub fn finish_shutdown(&mut self) -> Result<(), SmithayLinuxAdapterError> {
        self.transition(
            SmithayLinuxAdapterLifecycle::ShutdownRequested,
            SmithayLinuxAdapterLifecycle::Stopped,
            SmithayLinuxAdapterOperation::FinishShutdown,
        )
    }

    /// 当前实例是否仍严格保持 Phase 48A skeleton 边界。
    pub fn is_skeleton_only(&self) -> bool {
        self.bootstrap.is_probe_only() && self.capabilities().is_skeleton_only
    }

    fn transition(
        &mut self,
        expected: SmithayLinuxAdapterLifecycle,
        next: SmithayLinuxAdapterLifecycle,
        operation: SmithayLinuxAdapterOperation,
    ) -> Result<(), SmithayLinuxAdapterError> {
        if self.lifecycle != expected {
            return Err(SmithayLinuxAdapterError::InvalidLifecycleTransition {
                from: self.lifecycle,
                operation,
            });
        }

        self.lifecycle = next;

        Ok(())
    }
}

fn resource_initialization_error(source: Box<dyn Error>) -> SmithayLinuxAdapterError {
    SmithayLinuxAdapterError::ResourceInitialization {
        source: Box::new(std::io::Error::other(source.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayLinuxAdapterError, SmithayLinuxAdapterLifecycle, SmithayLinuxAdapterOperation,
        SmithayLinuxAdapterSkeleton,
    };
    use crate::smithay_backend::{
        runtime_facade::{BackendBootstrapMode, BackendRuntimeReport},
        test_support::{assert_runtime_dir, unique_socket_name},
    };

    #[test]
    fn adapter_skeleton_constructs_with_requested_socket() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-skeleton");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name.clone())
            .expect("adapter skeleton 必须持有指定名称的 bootstrap socket");

        assert_eq!(adapter.lifecycle(), SmithayLinuxAdapterLifecycle::Prepared);
        assert_eq!(adapter.socket_name_string(), socket_name);
        assert!(adapter.is_skeleton_only());
    }

    #[test]
    fn adapter_skeleton_capabilities_remain_conservative() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-capabilities");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        let capabilities = adapter.capabilities();

        assert!(capabilities.holds_wayland_display);
        assert!(capabilities.holds_listening_socket);
        assert!(capabilities.has_adapter_lifecycle_boundary);
        assert!(!capabilities.runs_event_loop);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
        assert!(capabilities.is_skeleton_only);
    }

    #[test]
    fn adapter_skeleton_follows_shutdown_lifecycle() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-lifecycle");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter
            .request_shutdown()
            .expect("Prepared 必须允许请求关闭");
        assert_eq!(
            adapter.lifecycle(),
            SmithayLinuxAdapterLifecycle::ShutdownRequested
        );

        adapter
            .finish_shutdown()
            .expect("ShutdownRequested 必须允许完成关闭");
        assert_eq!(adapter.lifecycle(), SmithayLinuxAdapterLifecycle::Stopped);
        assert!(adapter.is_skeleton_only());
    }

    #[test]
    fn repeated_shutdown_request_returns_structured_error() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-repeat-shutdown");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter.request_shutdown().expect("首次请求关闭必须成功");

        let error = adapter
            .request_shutdown()
            .expect_err("重复请求关闭必须返回错误");

        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidLifecycleTransition {
                from: SmithayLinuxAdapterLifecycle::ShutdownRequested,
                operation: SmithayLinuxAdapterOperation::RequestShutdown,
            }
        ));
    }

    #[test]
    fn finish_before_shutdown_request_returns_structured_error() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-finish-early");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        let error = adapter
            .finish_shutdown()
            .expect_err("Prepared 不得直接完成关闭");

        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidLifecycleTransition {
                from: SmithayLinuxAdapterLifecycle::Prepared,
                operation: SmithayLinuxAdapterOperation::FinishShutdown,
            }
        ));
    }

    #[test]
    fn adapter_skeleton_builds_conservative_runtime_report() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-report");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name.clone())
            .expect("adapter skeleton 必须能够构造");
        let report = BackendRuntimeReport::from(&adapter);

        assert_eq!(report.backend_name, "smithay-linux-adapter-skeleton");
        assert_eq!(report.bootstrap_mode, BackendBootstrapMode::ProbeOnly);
        assert_eq!(report.socket_name.as_deref(), Some(socket_name.as_str()));
        assert!(report.capabilities.can_create_display);
        assert!(report.capabilities.can_create_socket);
        assert!(!report.capabilities.supports_real_wayland_surfaces);
        assert!(!report.capabilities.supports_gpu_rendering);
    }

    #[test]
    fn adapter_production_code_keeps_system_and_core_boundaries() {
        let source = include_str!("linux_adapter.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        for forbidden in [
            "crate::core",
            "crate::backend",
            "BackendEvent",
            "smithay::",
            "DisplayHandle",
            "wayland_server::Display",
            "GlobalDispatch",
            "delegate_",
            "calloop",
            "wl_surface",
            "xdg_toplevel",
            "drm",
            "gbm",
            "libinput",
            "udev",
            "x11",
            "vulkan",
        ] {
            assert!(
                !production.contains(forbidden),
                "adapter skeleton 生产代码不得引用边界外入口: {forbidden}"
            );
        }
    }
}
