//! 后端运行时只读门面。
//!
//! 本模块把 Smithay probe 与 Linux 资源探针转换为后端中立的结构化报告。
//! 它不持有核心 `State`，不提交 `BackendEvent`，也不会启动 compositor。

use std::{fmt, path::PathBuf};

#[cfg(any(test, all(feature = "smithay-linux", target_os = "linux")))]
use std::{ffi::OsString, path::Path};

use crate::smithay_backend::runtime::SmithayRuntimeProbe;

#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
use crate::smithay_backend::{
    bootstrap::SmithayBootstrapMode, linux_adapter::SmithayLinuxAdapterSkeleton,
    linux_runtime::SmithayLinuxRuntimeProbe,
};

/// 后端启动模式。
///
/// 当前阶段只有纯探针模式；即使 Linux Display 和 socket 已构造，也不会接收
/// client、注册协议 global 或进入真实 compositor 主循环。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendBootstrapMode {
    /// 仅验证运行时和资源边界，不启动真实 compositor。
    ProbeOnly,
}

/// 后端运行时当前具备的能力。
///
/// 字段只描述已经实现并可由当前实例提供的能力。纯数据 surface 生命周期边界
/// 与真实 Wayland surface 接入是两项独立能力，后者和 GPU 渲染必须保持为
/// `false`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendRuntimeCapabilities {
    /// 是否能够构造 Wayland Display。
    pub can_create_display: bool,

    /// 是否能够构造 Wayland listening socket。
    pub can_create_socket: bool,

    /// 是否使用 Linux `XDG_RUNTIME_DIR` 运行时目录。
    pub supports_linux_runtime_dir: bool,

    /// 是否支持纯数据 runtime probe。
    pub supports_mock_runtime: bool,

    /// 是否支持后端中立的 surface 生命周期纯数据边界。
    pub supports_surface_lifecycle_boundary: bool,

    /// 是否支持 surface 生命周期纯数据 trace harness。
    pub supports_surface_trace_harness: bool,

    /// 是否支持 surface 到窗口候选意图的纯数据规划。
    pub supports_surface_window_intent_planning: bool,

    /// 是否支持窗口候选意图到接纳动作的纯数据预检。
    pub supports_window_admission_preview: bool,

    /// 是否支持 surface 生命周期到接纳预检的纯数据集成管线。
    pub supports_surface_admission_pipeline_preview: bool,

    /// 是否已经接入真实 Wayland surface 生命周期。
    pub supports_real_wayland_surfaces: bool,

    /// 是否已经接入 GPU 渲染。
    pub supports_gpu_rendering: bool,
}

impl BackendRuntimeCapabilities {
    /// 返回跨平台纯数据 probe 的保守能力集合。
    pub const fn smithay_probe() -> Self {
        Self {
            can_create_display: false,
            can_create_socket: false,
            supports_linux_runtime_dir: false,
            supports_mock_runtime: true,
            supports_surface_lifecycle_boundary: true,
            supports_surface_trace_harness: true,
            supports_surface_window_intent_planning: true,
            supports_window_admission_preview: true,
            supports_surface_admission_pipeline_preview: true,
            supports_real_wayland_surfaces: false,
            supports_gpu_rendering: false,
        }
    }

    /// 返回 Linux 资源探针的保守能力集合。
    ///
    /// Display 和 socket 已可构造，但真实 surface 与 GPU 渲染仍未接入。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    pub const fn smithay_linux_probe() -> Self {
        Self {
            can_create_display: true,
            can_create_socket: true,
            supports_linux_runtime_dir: true,
            supports_mock_runtime: true,
            supports_surface_lifecycle_boundary: true,
            supports_surface_trace_harness: true,
            supports_surface_window_intent_planning: true,
            supports_window_admission_preview: true,
            supports_surface_admission_pipeline_preview: true,
            supports_real_wayland_surfaces: false,
            supports_gpu_rendering: false,
        }
    }
}

/// 后端运行时诊断。
///
/// 每个变体都保留机器可判断的类别和必要上下文；`Display` 只用于日志展示，
/// 调用方不需要解析字符串来理解诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendRuntimeDiagnostic {
    /// Linux runtime 缺少 `XDG_RUNTIME_DIR`。
    MissingRuntimeDir {
        /// 缺失的环境变量名称。
        variable: &'static str,
    },

    /// Linux runtime dir 存在配置值，但不是有效目录。
    InvalidRuntimeDir {
        /// 无效路径。
        path: PathBuf,
    },

    /// 后端已经选择或绑定 socket 名称。
    SocketNameSelected {
        /// Wayland socket 名称。
        socket_name: String,
    },

    /// 当前构建没有启用某个可选 feature。
    FeatureDisabled {
        /// 未启用的 Cargo feature。
        feature: &'static str,
    },

    /// 某项能力只允许在 Linux 上使用。
    LinuxOnly {
        /// Linux 专属 Cargo feature。
        feature: &'static str,
    },

    /// 当前实例回退到纯数据 probe。
    ProbeFallback {
        /// 回退后的后端名称。
        backend_name: &'static str,
    },

    /// 当前主机平台不支持请求的系统后端。
    UnsupportedPlatform {
        /// 当前 Rust 目标操作系统名称。
        platform: &'static str,
    },

    /// Linux adapter 仅提供 event pump skeleton 边界。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    AdapterEventPumpSkeleton {
        /// 是否提供 event pump 边界。
        has_event_pump_boundary: bool,

        /// 是否支持执行一次纯计数 skeleton tick。
        pumps_once: bool,

        /// 是否运行真实事件循环。
        runs_event_loop: bool,

        /// 是否接受客户端。
        accepts_clients: bool,

        /// 是否分发协议事件。
        dispatches_protocol_events: bool,

        /// 是否注册协议 global。
        registers_protocol_globals: bool,
    },

    /// Linux adapter 只提供 protocol global 计划，尚未执行注册。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    AdapterProtocolGlobalPlan {
        /// global 计划数量。
        planned_count: usize,

        /// 已注册 global 数量。
        registered_count: usize,

        /// 当前计划是否仍然只属于 skeleton。
        skeleton_only: bool,
    },

    /// Linux adapter 只建立了 protocol global registration skeleton ledger。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    AdapterProtocolGlobalRegistrationSkeleton {
        /// 是否已经尝试 registration skeleton。
        attempted: bool,

        /// 进入 registration skeleton 状态的 global 数量。
        skeleton_registered_count: usize,

        /// 真实注册的 global 数量。
        real_registered_count: usize,

        /// 当前 registration 状态是否仍然只属于 skeleton。
        skeleton_only: bool,
    },
}

impl fmt::Display for BackendRuntimeDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRuntimeDir { variable } => {
                write!(formatter, "缺少运行时目录环境变量 {variable}")
            }
            Self::InvalidRuntimeDir { path } => {
                write!(formatter, "运行时目录无效: {}", path.display())
            }
            Self::SocketNameSelected { socket_name } => {
                write!(formatter, "已选择 Wayland socket: {socket_name}")
            }
            Self::FeatureDisabled { feature } => {
                write!(formatter, "Cargo feature 未启用: {feature}")
            }
            Self::LinuxOnly { feature } => {
                write!(formatter, "Cargo feature 仅支持 Linux: {feature}")
            }
            Self::ProbeFallback { backend_name } => {
                write!(formatter, "使用纯数据运行时探针: {backend_name}")
            }
            Self::UnsupportedPlatform { platform } => {
                write!(formatter, "当前平台不支持 Linux 后端: {platform}")
            }
            #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
            Self::AdapterEventPumpSkeleton {
                has_event_pump_boundary,
                pumps_once,
                runs_event_loop,
                accepts_clients,
                dispatches_protocol_events,
                registers_protocol_globals,
            } => write!(
                formatter,
                "adapter event pump skeleton: boundary={has_event_pump_boundary}, \
                 pumps_once={pumps_once}, event_loop={runs_event_loop}, clients={accepts_clients}, \
                 protocol_events={dispatches_protocol_events}, globals={registers_protocol_globals}"
            ),
            #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
            Self::AdapterProtocolGlobalPlan {
                planned_count,
                registered_count,
                skeleton_only,
            } => write!(
                formatter,
                "adapter protocol global plan: planned={planned_count}, \
                 registered={registered_count}, skeleton_only={skeleton_only}"
            ),
            #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
            Self::AdapterProtocolGlobalRegistrationSkeleton {
                attempted,
                skeleton_registered_count,
                real_registered_count,
                skeleton_only,
            } => write!(
                formatter,
                "adapter protocol global registration skeleton: attempted={attempted}, \
                 skeleton_registered={skeleton_registered_count}, \
                 real_registered={real_registered_count}, skeleton_only={skeleton_only}"
            ),
        }
    }
}

/// 后端运行时门面报告。
///
/// 报告只描述启动资源和能力，不包含 workspace、focus、scene 或渲染帧，也不会
/// 修改任何核心状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendRuntimeReport {
    /// 后端实现名称。
    pub backend_name: &'static str,

    /// 当前启动模式。
    pub bootstrap_mode: BackendBootstrapMode,

    /// 已选择的 Wayland socket 名称；纯 probe 为 `None`。
    pub socket_name: Option<String>,

    /// 有效的 Linux runtime dir；纯 probe 或无效配置为 `None`。
    pub runtime_dir: Option<PathBuf>,

    /// 当前实例的保守能力集合。
    pub capabilities: BackendRuntimeCapabilities,

    /// 结构化启动诊断。
    pub diagnostics: Vec<BackendRuntimeDiagnostic>,
}

impl BackendRuntimeReport {
    /// 判断报告是否包含指定类别的诊断。
    pub fn has_diagnostic(&self, predicate: impl Fn(&BackendRuntimeDiagnostic) -> bool) -> bool {
        self.diagnostics.iter().any(predicate)
    }
}

impl From<&SmithayRuntimeProbe> for BackendRuntimeReport {
    /// 从兼容 runtime probe 创建后端中立报告。
    fn from(runtime: &SmithayRuntimeProbe) -> Self {
        #[cfg(not(all(feature = "smithay-linux", target_os = "linux")))]
        let _ = runtime;

        #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
        {
            let socket_name = runtime.socket_name_string();

            if !socket_name.is_empty() {
                return linux_resource_report(
                    "smithay-linux-compat",
                    map_bootstrap_mode(runtime.bootstrap_mode()),
                    socket_name,
                );
            }
        }

        smithay_probe_report()
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
impl From<&SmithayLinuxRuntimeProbe> for BackendRuntimeReport {
    /// 从 Linux Display/socket 组合探针创建后端中立报告。
    fn from(runtime: &SmithayLinuxRuntimeProbe) -> Self {
        linux_resource_report(
            "smithay-linux",
            map_bootstrap_mode(runtime.bootstrap_mode()),
            runtime.socket_name_string(),
        )
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
impl From<&SmithayLinuxAdapterSkeleton> for BackendRuntimeReport {
    /// 从 adapter event pump skeleton 创建保守的后端中立报告。
    fn from(adapter: &SmithayLinuxAdapterSkeleton) -> Self {
        let snapshot = adapter.snapshot();
        let capabilities = snapshot.capabilities;
        let mut report = linux_resource_report(
            "smithay-linux-adapter-skeleton",
            BackendBootstrapMode::ProbeOnly,
            snapshot.socket_name,
        );
        report
            .diagnostics
            .push(BackendRuntimeDiagnostic::AdapterEventPumpSkeleton {
                has_event_pump_boundary: capabilities.has_event_pump_boundary,
                pumps_once: capabilities.pumps_once,
                runs_event_loop: capabilities.runs_event_loop,
                accepts_clients: capabilities.accepts_clients,
                dispatches_protocol_events: capabilities.dispatches_protocol_events,
                registers_protocol_globals: capabilities.registers_protocol_globals,
            });
        report
            .diagnostics
            .push(BackendRuntimeDiagnostic::AdapterProtocolGlobalPlan {
                planned_count: snapshot.global_plan.planned_count,
                registered_count: snapshot.global_plan.registered_count,
                skeleton_only: snapshot.global_plan.skeleton_only,
            });
        let registration = snapshot.global_registration_report.as_ref();
        report.diagnostics.push(
            BackendRuntimeDiagnostic::AdapterProtocolGlobalRegistrationSkeleton {
                attempted: registration.is_some(),
                skeleton_registered_count: registration
                    .map_or(0, |report| report.skeleton_registered_count),
                real_registered_count: registration
                    .map_or(0, |report| report.real_registered_count),
                skeleton_only: registration.map_or(true, |report| report.skeleton_only),
            },
        );

        report
    }
}

/// 创建跨平台纯数据 probe 报告。
fn smithay_probe_report() -> BackendRuntimeReport {
    let mut diagnostics = vec![BackendRuntimeDiagnostic::ProbeFallback {
        backend_name: "smithay-probe",
    }];

    if !cfg!(feature = "smithay-linux") {
        diagnostics.push(BackendRuntimeDiagnostic::FeatureDisabled {
            feature: "smithay-linux",
        });
    }

    if !cfg!(target_os = "linux") {
        diagnostics.push(BackendRuntimeDiagnostic::LinuxOnly {
            feature: "smithay-linux",
        });
        diagnostics.push(BackendRuntimeDiagnostic::UnsupportedPlatform {
            platform: std::env::consts::OS,
        });
    }

    BackendRuntimeReport {
        backend_name: "smithay-probe",
        bootstrap_mode: BackendBootstrapMode::ProbeOnly,
        socket_name: None,
        runtime_dir: None,
        capabilities: BackendRuntimeCapabilities::smithay_probe(),
        diagnostics,
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
/// 创建已持有 Linux Display/socket 资源的报告。
fn linux_resource_report(
    backend_name: &'static str,
    bootstrap_mode: BackendBootstrapMode,
    socket_name: String,
) -> BackendRuntimeReport {
    let (runtime_dir, mut diagnostics) = inspect_runtime_dir(std::env::var_os("XDG_RUNTIME_DIR"));

    diagnostics.push(BackendRuntimeDiagnostic::SocketNameSelected {
        socket_name: socket_name.clone(),
    });

    BackendRuntimeReport {
        backend_name,
        bootstrap_mode,
        socket_name: Some(socket_name),
        runtime_dir,
        capabilities: BackendRuntimeCapabilities::smithay_linux_probe(),
        diagnostics,
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
/// 把 Smithay bootstrap 模式映射为后端中立模式。
fn map_bootstrap_mode(mode: SmithayBootstrapMode) -> BackendBootstrapMode {
    match mode {
        SmithayBootstrapMode::ProbeOnly => BackendBootstrapMode::ProbeOnly,
    }
}

/// 检查 Linux runtime dir，但不因环境缺失或路径无效而 panic。
#[cfg(any(test, all(feature = "smithay-linux", target_os = "linux")))]
fn inspect_runtime_dir(
    runtime_dir: Option<OsString>,
) -> (Option<PathBuf>, Vec<BackendRuntimeDiagnostic>) {
    let Some(runtime_dir) = runtime_dir else {
        return (
            None,
            vec![BackendRuntimeDiagnostic::MissingRuntimeDir {
                variable: "XDG_RUNTIME_DIR",
            }],
        );
    };

    let runtime_dir = PathBuf::from(runtime_dir);

    if !Path::new(&runtime_dir).is_dir() {
        return (
            None,
            vec![BackendRuntimeDiagnostic::InvalidRuntimeDir { path: runtime_dir }],
        );
    }

    (Some(runtime_dir), Vec::new())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        BackendBootstrapMode, BackendRuntimeCapabilities, BackendRuntimeDiagnostic,
        BackendRuntimeReport, inspect_runtime_dir,
    };
    use crate::smithay_backend::runtime::SmithayRuntimeProbe;

    /// 验证纯数据 runtime 可以生成后端中立报告。
    #[test]
    fn smithay_probe_builds_runtime_facade_report() {
        let runtime = SmithayRuntimeProbe::new_probe_only();
        let report = BackendRuntimeReport::from(&runtime);

        assert_eq!(report.backend_name, "smithay-probe");
        assert_eq!(report.bootstrap_mode, BackendBootstrapMode::ProbeOnly);
        assert_eq!(report.socket_name, None);
        assert_eq!(report.runtime_dir, None);
        assert_eq!(
            report.capabilities,
            BackendRuntimeCapabilities::smithay_probe()
        );
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::ProbeFallback { .. }
        )));
    }

    /// 验证纯 probe 不会宣称尚未实现的系统能力。
    #[test]
    fn smithay_probe_capabilities_are_conservative() {
        let runtime = SmithayRuntimeProbe::new_probe_only();
        let capabilities = BackendRuntimeReport::from(&runtime).capabilities;

        assert!(!capabilities.can_create_display);
        assert!(!capabilities.can_create_socket);
        assert!(!capabilities.supports_linux_runtime_dir);
        assert!(capabilities.supports_mock_runtime);
        assert!(capabilities.supports_surface_lifecycle_boundary);
        assert!(capabilities.supports_surface_trace_harness);
        assert!(capabilities.supports_surface_window_intent_planning);
        assert!(capabilities.supports_window_admission_preview);
        assert!(capabilities.supports_surface_admission_pipeline_preview);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
    }

    /// 验证不可用 Linux 后端会形成结构化诊断，而不是触发 panic。
    #[test]
    fn unavailable_linux_runtime_becomes_structured_diagnostics() {
        let runtime = SmithayRuntimeProbe::new_probe_only();
        let report = BackendRuntimeReport::from(&runtime);

        #[cfg(not(feature = "smithay-linux"))]
        {
            assert!(report.has_diagnostic(|diagnostic| matches!(
                diagnostic,
                BackendRuntimeDiagnostic::FeatureDisabled {
                    feature: "smithay-linux"
                }
            )));
        }

        #[cfg(not(target_os = "linux"))]
        {
            assert!(report.has_diagnostic(|diagnostic| matches!(
                diagnostic,
                BackendRuntimeDiagnostic::LinuxOnly {
                    feature: "smithay-linux"
                }
            )));
            assert!(report.has_diagnostic(|diagnostic| matches!(
                diagnostic,
                BackendRuntimeDiagnostic::UnsupportedPlatform { .. }
            )));
        }
    }

    /// 验证缺失 runtime dir 会返回明确诊断。
    #[test]
    fn missing_runtime_dir_becomes_diagnostic() {
        let (runtime_dir, diagnostics) = inspect_runtime_dir(None);

        assert_eq!(runtime_dir, None);
        assert_eq!(
            diagnostics,
            vec![BackendRuntimeDiagnostic::MissingRuntimeDir {
                variable: "XDG_RUNTIME_DIR"
            }]
        );
    }

    /// 验证无效 runtime dir 会返回路径诊断。
    #[test]
    fn invalid_runtime_dir_becomes_diagnostic() {
        let invalid_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let (runtime_dir, diagnostics) =
            inspect_runtime_dir(Some(invalid_path.clone().into_os_string()));

        assert_eq!(runtime_dir, None);
        assert_eq!(
            diagnostics,
            vec![BackendRuntimeDiagnostic::InvalidRuntimeDir { path: invalid_path }]
        );
    }

    /// 验证结构化诊断可以安全转换为日志文本。
    #[test]
    fn runtime_diagnostic_display_preserves_context() {
        let diagnostic = BackendRuntimeDiagnostic::SocketNameSelected {
            socket_name: "wayland-test".to_string(),
        };

        assert_eq!(
            diagnostic.to_string(),
            "已选择 Wayland socket: wayland-test"
        );
    }

    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    mod linux_tests {
        use super::{
            BackendBootstrapMode, BackendRuntimeDiagnostic, BackendRuntimeReport,
            SmithayRuntimeProbe,
        };
        use crate::smithay_backend::{
            linux_runtime::SmithayLinuxRuntimeProbe,
            test_support::{assert_runtime_dir, unique_socket_name},
        };

        /// 验证旧 runtime Linux API 配置的 socket 会进入门面报告。
        #[test]
        fn legacy_smithay_runtime_builds_linux_facade_report() {
            assert_runtime_dir();

            let socket_name = unique_socket_name("facade-legacy");
            let runtime = SmithayRuntimeProbe::with_socket_name(&socket_name)
                .expect("旧 runtime API 必须真实构造指定 socket");
            let report = BackendRuntimeReport::from(&runtime);

            assert_eq!(report.backend_name, "smithay-linux-compat");
            assert_eq!(report.bootstrap_mode, BackendBootstrapMode::ProbeOnly);
            assert_eq!(report.socket_name.as_deref(), Some(socket_name.as_str()));
            assert!(report.runtime_dir.is_some());
            assert!(report.capabilities.can_create_display);
            assert!(report.capabilities.can_create_socket);
            assert!(report.capabilities.supports_surface_lifecycle_boundary);
            assert!(report.capabilities.supports_surface_trace_harness);
            assert!(report.capabilities.supports_surface_window_intent_planning);
            assert!(report.capabilities.supports_window_admission_preview);
            assert!(
                report
                    .capabilities
                    .supports_surface_admission_pipeline_preview
            );
            assert!(!report.capabilities.supports_real_wayland_surfaces);
            assert!(!report.capabilities.supports_gpu_rendering);
            assert!(report.has_diagnostic(|diagnostic| matches!(
                diagnostic,
                BackendRuntimeDiagnostic::SocketNameSelected {
                    socket_name: selected
                } if selected == &socket_name
            )));
        }

        /// 验证 Linux 资源组合探针可以生成相同语义的门面报告。
        #[test]
        fn linux_runtime_builds_facade_report() {
            assert_runtime_dir();

            let socket_name = unique_socket_name("facade-linux");
            let runtime = SmithayLinuxRuntimeProbe::with_socket_name(&socket_name)
                .expect("Linux runtime 必须真实构造 Display 和 socket");
            let report = BackendRuntimeReport::from(&runtime);

            assert_eq!(report.backend_name, "smithay-linux");
            assert_eq!(report.bootstrap_mode, BackendBootstrapMode::ProbeOnly);
            assert_eq!(report.socket_name.as_deref(), Some(socket_name.as_str()));
            assert!(report.runtime_dir.is_some());
            assert!(report.capabilities.supports_linux_runtime_dir);
            assert!(report.capabilities.supports_mock_runtime);
            assert!(report.capabilities.supports_surface_lifecycle_boundary);
            assert!(report.capabilities.supports_surface_trace_harness);
            assert!(report.capabilities.supports_surface_window_intent_planning);
            assert!(report.capabilities.supports_window_admission_preview);
            assert!(
                report
                    .capabilities
                    .supports_surface_admission_pipeline_preview
            );
            assert!(!report.capabilities.supports_real_wayland_surfaces);
            assert!(!report.capabilities.supports_gpu_rendering);
        }
    }
}
