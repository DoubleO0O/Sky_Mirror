//! Linux Smithay 系统资源与纯数据 runtime 的组合探针。
//!
//! 本模块只在 Linux 上启用 `smithay-linux` feature 时编译。它负责持有真实
//! Wayland Display 和 listening socket 探针，但所有状态变化仍交给纯数据
//! `SmithayRuntimeProbe`，并继续通过 `BackendDriverRunner` 进入核心。
//!
//! Boundary: 该组合类型可以驱动预置的纯数据事件，但不从 Wayland socket 接收
//! client，也不把 Display 事件转换为协议回调。

use crate::{
    core::{backend_driver::BackendDriverRunReport, backend_event::BackendEvent, state::State},
    smithay_backend::{
        bootstrap::{SmithayBootstrapMode, SmithayBootstrapProbe},
        runtime::SmithayRuntimeProbe,
    },
};

/// Linux Smithay 资源组合探针。
///
/// 该结构只组合 bootstrap 与纯数据 runtime。bootstrap 不接收真实 client，
/// runtime 不持有系统资源，也不会绕过后端驱动边界直接修改 `State`。
pub struct SmithayLinuxRuntimeProbe {
    /// Linux Wayland Display 与 listening socket 组合探针。
    bootstrap: SmithayBootstrapProbe,

    /// 跨平台纯数据 runtime。
    runtime: SmithayRuntimeProbe,
}

impl SmithayLinuxRuntimeProbe {
    /// 自动创建 Linux Display/socket 和纯数据 runtime。
    ///
    /// 当前只构造资源，不把 socket 插入 calloop，也不注册真实 client。
    pub fn new_auto() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            bootstrap: SmithayBootstrapProbe::new_auto()?,
            runtime: SmithayRuntimeProbe::new_probe_only(),
        })
    }

    /// 使用指定 socket 名称创建 Linux 资源组合探针。
    ///
    /// 该方法不会启动 compositor，也不会接收或注册真实 client。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            bootstrap: SmithayBootstrapProbe::with_socket_name(name)?,
            runtime: SmithayRuntimeProbe::new_probe_only(),
        })
    }

    /// 使用已有 bootstrap 与纯数据 runtime 创建组合探针。
    ///
    /// 该入口用于测试资源和事件队列的独立准备，不改变事件处理边界。
    pub fn from_parts(bootstrap: SmithayBootstrapProbe, runtime: SmithayRuntimeProbe) -> Self {
        Self { bootstrap, runtime }
    }

    /// 向纯数据 runtime 推入一条后端事件。
    pub fn push_event(&mut self, event: BackendEvent) {
        self.runtime.push_event(event);
    }

    /// 运行一轮纯数据 runtime。
    ///
    /// 本方法只转发到 `SmithayRuntimeProbe::run_once`，不会直接调用核心命令入口。
    pub fn run_once(&mut self, state: &mut State) -> BackendDriverRunReport {
        self.runtime.run_once(state)
    }

    /// 返回 bootstrap 当前模式。
    pub fn bootstrap_mode(&self) -> SmithayBootstrapMode {
        self.bootstrap.mode()
    }

    /// 返回 listening socket 名称。
    pub fn socket_name_string(&self) -> String {
        self.bootstrap.socket_name_string()
    }

    /// 只读访问内部纯数据 runtime。
    pub fn runtime(&self) -> &SmithayRuntimeProbe {
        &self.runtime
    }

    /// 可变访问内部纯数据 runtime。
    ///
    /// 该访问只暴露纯数据队列 helper，不暴露 bootstrap 或系统资源。状态仍在
    /// `run_once` 时通过 `BackendDriverRunner` 统一推进。
    pub fn runtime_mut(&mut self) -> &mut SmithayRuntimeProbe {
        &mut self.runtime
    }

    /// 当前是否仍处于资源探针与纯数据探针组合模式。
    pub fn is_probe_only(&self) -> bool {
        self.bootstrap.is_probe_only() && self.runtime.is_probe_only()
    }
}

#[cfg(test)]
mod tests {
    use super::SmithayLinuxRuntimeProbe;
    use crate::{
        core::{backend_event::BackendEvent, state::State},
        smithay_backend::{
            bootstrap::SmithayBootstrapMode,
            driver::SmithayBackendDriverProbe,
            runtime::SmithayRuntimeProbe,
            test_support::{assert_runtime_dir, unique_socket_name},
        },
    };

    /// 验证 Linux runtime 创建后仍保持探针模式。
    #[test]
    fn smithay_linux_runtime_is_probe_only_when_created() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("linux-runtime-mode");
        let runtime = SmithayLinuxRuntimeProbe::with_socket_name(&socket_name)
            .expect("Linux runtime 必须真实构造 Display 和指定名称的 socket");

        assert!(runtime.is_probe_only());
        assert_eq!(runtime.bootstrap_mode(), SmithayBootstrapMode::ProbeOnly);
        assert_eq!(runtime.socket_name_string(), socket_name);
    }

    /// 验证 Linux 资源外壳仍通过纯数据 runtime 推进核心事件。
    #[test]
    fn smithay_linux_runtime_runs_event_through_probe_runtime() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("linux-runtime-event");
        let mut runtime = SmithayLinuxRuntimeProbe::with_socket_name(&socket_name)
            .expect("Linux runtime 必须真实构造资源后才能验证事件链");
        let mut state = State::new();

        runtime.push_event(BackendEvent::OutputResized {
            width: 1440,
            height: 900,
        });

        let report = runtime.run_once(&mut state);
        let output = state.compositor.current_output_size();

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert_eq!(output.width, 1440);
        assert_eq!(output.height, 900);
    }

    /// 验证旧 `SmithayRuntimeProbe` Linux 构造和查询 API 仍可工作。
    #[test]
    fn smithay_runtime_legacy_linux_api_remains_available() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("legacy-runtime");
        let runtime = SmithayRuntimeProbe::with_socket_name(&socket_name)
            .expect("旧 runtime API 必须真实构造指定名称的 socket");

        assert!(runtime.is_probe_only());
        assert_eq!(runtime.bootstrap_mode(), SmithayBootstrapMode::ProbeOnly);
        assert_eq!(runtime.socket_name_string(), socket_name);

        let bootstrap_name = unique_socket_name("legacy-from-parts");
        let bootstrap = crate::smithay_backend::bootstrap::SmithayBootstrapProbe::with_socket_name(
            &bootstrap_name,
        )
        .expect("旧 from_parts 测试必须真实构造 bootstrap");
        let runtime =
            SmithayRuntimeProbe::from_parts(bootstrap, SmithayBackendDriverProbe::new_probe_only());

        assert_eq!(runtime.socket_name_string(), bootstrap_name);
    }
}
