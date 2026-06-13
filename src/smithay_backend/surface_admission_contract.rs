//! Surface 接纳预检管线的稳定契约快照。
//!
//! 本模块只运行既有 mock 场景和管线，并把完整报告压缩为便于回归比较的
//! 快照。快照不会替代完整报告，也不会提交任何状态变化。

use crate::smithay_backend::{
    surface_admission_pipeline::{
        BackendSurfaceAdmissionPipelineReport, BackendSurfaceAdmissionPipelineStatus,
        SurfaceAdmissionPipelineRunner,
    },
    surface_trace::{BackendSurfaceTrace, BackendSurfaceTraceScenario},
};

/// Surface 接纳管线的紧凑契约快照。
///
/// 快照只保存稳定场景名、管线状态、阶段计数和来源失败位置，供 golden
/// 测试与开发者诊断使用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSurfaceAdmissionContractSnapshot {
    /// 稳定场景名称。
    pub scenario_name: &'static str,

    /// 管线整体状态。
    pub status: BackendSurfaceAdmissionPipelineStatus,

    /// Trace 结束时保留的 surface 记录数量。
    pub surface_count: usize,

    /// 候选意图数量。
    pub intent_count: usize,

    /// 接纳预检动作数量。
    pub action_count: usize,

    /// 接纳预检 warning 数量。
    pub warning_count: usize,

    /// 来源 trace 首个失败事件的下标。
    pub source_failed_at: Option<usize>,
}

impl BackendSurfaceAdmissionContractSnapshot {
    /// 从完整管线报告提取指定场景的稳定契约视图。
    pub fn from_report(
        scenario_name: &'static str,
        report: &BackendSurfaceAdmissionPipelineReport,
    ) -> Self {
        let summary = report.summary();

        Self {
            scenario_name,
            status: summary.status,
            surface_count: summary.surface_count,
            intent_count: summary.intent_count,
            action_count: summary.action_count,
            warning_count: summary.warning_count,
            source_failed_at: summary.source_failed_at,
        }
    }
}

/// Surface 接纳管线需要长期保持稳定的契约场景。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSurfaceAdmissionContractScenario {
    /// 空 trace。
    Empty,

    /// 创建后直接映射。
    SingleMap,

    /// 配置尺寸后映射。
    ConfigureThenMap,

    /// 映射、撤销映射后再次映射。
    MapUnmapRemap,

    /// 创建后销毁。
    Destroy,

    /// 销毁后尝试再次映射。
    InvalidDestroyedRemap,

    /// 多 surface 混合最终状态。
    MultiSurface,
}

impl BackendSurfaceAdmissionContractScenario {
    /// 按契约顺序列出全部 golden 场景。
    pub const ALL: [Self; 7] = [
        Self::Empty,
        Self::SingleMap,
        Self::ConfigureThenMap,
        Self::MapUnmapRemap,
        Self::Destroy,
        Self::InvalidDestroyedRemap,
        Self::MultiSurface,
    ];

    /// 返回用于日志与回归输出的稳定场景名称。
    pub const fn name(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::SingleMap => "SingleMap",
            Self::ConfigureThenMap => "ConfigureThenMap",
            Self::MapUnmapRemap => "MapUnmapRemap",
            Self::Destroy => "Destroy",
            Self::InvalidDestroyedRemap => "InvalidDestroyedRemap",
            Self::MultiSurface => "MultiSurface",
        }
    }
}

/// Surface 接纳管线的无状态契约 runner。
///
/// Runner 只调用现有管线入口，不复制生命周期、候选意图或接纳预检规则。
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceAdmissionContractRunner {
    pipeline_runner: SurfaceAdmissionPipelineRunner,
}

impl SurfaceAdmissionContractRunner {
    /// 创建使用默认纯数据管线的契约 runner。
    pub const fn new() -> Self {
        Self {
            pipeline_runner: SurfaceAdmissionPipelineRunner::new(),
        }
    }

    /// 运行指定契约场景并返回完整管线报告。
    pub fn report_for_scenario(
        &self,
        scenario: BackendSurfaceAdmissionContractScenario,
    ) -> BackendSurfaceAdmissionPipelineReport {
        match scenario {
            BackendSurfaceAdmissionContractScenario::Empty => {
                self.pipeline_runner.run_trace(&BackendSurfaceTrace::new())
            }
            BackendSurfaceAdmissionContractScenario::SingleMap => self
                .pipeline_runner
                .run_scenario(BackendSurfaceTraceScenario::SingleMap),
            BackendSurfaceAdmissionContractScenario::ConfigureThenMap => self
                .pipeline_runner
                .run_scenario(BackendSurfaceTraceScenario::ConfigureThenMap),
            BackendSurfaceAdmissionContractScenario::MapUnmapRemap => self
                .pipeline_runner
                .run_scenario(BackendSurfaceTraceScenario::MapUnmapRemap),
            BackendSurfaceAdmissionContractScenario::Destroy => self
                .pipeline_runner
                .run_scenario(BackendSurfaceTraceScenario::Destroy),
            BackendSurfaceAdmissionContractScenario::InvalidDestroyedRemap => self
                .pipeline_runner
                .run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap),
            BackendSurfaceAdmissionContractScenario::MultiSurface => self
                .pipeline_runner
                .run_scenario(BackendSurfaceTraceScenario::MultiSurface),
        }
    }

    /// 运行指定场景并生成紧凑契约快照。
    pub fn snapshot_for_scenario(
        &self,
        scenario: BackendSurfaceAdmissionContractScenario,
    ) -> BackendSurfaceAdmissionContractSnapshot {
        let report = self.report_for_scenario(scenario);

        BackendSurfaceAdmissionContractSnapshot::from_report(scenario.name(), &report)
    }

    /// 按固定契约顺序生成全部场景快照。
    pub fn snapshots_for_all_scenarios(&self) -> Vec<BackendSurfaceAdmissionContractSnapshot> {
        BackendSurfaceAdmissionContractScenario::ALL
            .into_iter()
            .map(|scenario| self.snapshot_for_scenario(scenario))
            .collect()
    }

    /// 生成稳定、可读的契约摘要文本。
    ///
    /// 文本只用于诊断和文档展示，结构化快照仍是回归断言的主要依据。
    pub fn contract_summary_text(&self) -> String {
        let mut text = String::from("Surface Admission Contract:\n");

        for snapshot in self.snapshots_for_all_scenarios() {
            text.push_str("* ");
            text.push_str(snapshot.scenario_name);
            text.push_str(": ");
            text.push_str(status_name(snapshot.status));
            text.push_str(", surfaces=");
            text.push_str(&snapshot.surface_count.to_string());
            text.push_str(", intents=");
            text.push_str(&snapshot.intent_count.to_string());
            text.push_str(", actions=");
            text.push_str(&snapshot.action_count.to_string());
            text.push_str(", warnings=");
            text.push_str(&snapshot.warning_count.to_string());

            if let Some(failed_at) = snapshot.source_failed_at {
                text.push_str(", source_failed_at=");
                text.push_str(&failed_at.to_string());
            }

            text.push('\n');
        }

        text
    }
}

/// 返回稳定的管线状态名称。
const fn status_name(status: BackendSurfaceAdmissionPipelineStatus) -> &'static str {
    match status {
        BackendSurfaceAdmissionPipelineStatus::Complete => "Complete",
        BackendSurfaceAdmissionPipelineStatus::SourceTraceFailed => "SourceTraceFailed",
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        BackendSurfaceAdmissionContractScenario, BackendSurfaceAdmissionContractSnapshot,
        SurfaceAdmissionContractRunner,
    };
    use crate::smithay_backend::{
        runtime_facade::BackendRuntimeCapabilities,
        surface_admission_pipeline::BackendSurfaceAdmissionPipelineStatus,
        surface_lifecycle::{BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceSize},
        surface_window_intent::BackendWindowCandidateIntent,
        window_admission_preview::BackendWindowAdmissionPreviewAction,
    };

    /// 返回生产代码部分，避免静态边界测试把自身断言字符串计入结果。
    fn production_source() -> String {
        let source = fs::read_to_string(file!()).expect("必须能读取当前模块源码");
        source
            .split("#[cfg(test)]")
            .next()
            .expect("模块必须包含测试边界")
            .to_string()
    }

    /// 递归检查 Rust 源码树是否包含指定文本。
    fn rust_tree_contains(path: &Path, needle: &str) -> bool {
        fs::read_dir(path)
            .expect("必须能读取源码目录")
            .filter_map(Result::ok)
            .any(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    rust_tree_contains(&path, needle)
                } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                    fs::read_to_string(path)
                        .map(|source| source.contains(needle))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
    }

    /// 生成指定场景的契约快照。
    fn snapshot(
        scenario: BackendSurfaceAdmissionContractScenario,
    ) -> BackendSurfaceAdmissionContractSnapshot {
        SurfaceAdmissionContractRunner::new().snapshot_for_scenario(scenario)
    }

    /// 验证 Empty 契约快照固定为全零成功状态。
    #[test]
    fn empty_contract_snapshot_matches_golden_rule() {
        assert_eq!(
            snapshot(BackendSurfaceAdmissionContractScenario::Empty),
            BackendSurfaceAdmissionContractSnapshot {
                scenario_name: "Empty",
                status: BackendSurfaceAdmissionPipelineStatus::Complete,
                surface_count: 0,
                intent_count: 0,
                action_count: 0,
                warning_count: 0,
                source_failed_at: None,
            }
        );
    }

    /// 验证 SingleMap 固定生成一个缺少尺寸的创建预检。
    #[test]
    fn single_map_contract_snapshot_matches_golden_rule() {
        assert_eq!(
            snapshot(BackendSurfaceAdmissionContractScenario::SingleMap),
            BackendSurfaceAdmissionContractSnapshot {
                scenario_name: "SingleMap",
                status: BackendSurfaceAdmissionPipelineStatus::Complete,
                surface_count: 1,
                intent_count: 1,
                action_count: 1,
                warning_count: 1,
                source_failed_at: None,
            }
        );
    }

    /// 验证 ConfigureThenMap 固定生成完整创建预检。
    #[test]
    fn configure_then_map_contract_snapshot_matches_golden_rule() {
        assert_eq!(
            snapshot(BackendSurfaceAdmissionContractScenario::ConfigureThenMap),
            BackendSurfaceAdmissionContractSnapshot {
                scenario_name: "ConfigureThenMap",
                status: BackendSurfaceAdmissionPipelineStatus::Complete,
                surface_count: 1,
                intent_count: 1,
                action_count: 1,
                warning_count: 0,
                source_failed_at: None,
            }
        );
    }

    /// 验证 ConfigureThenMap 的完整报告保留配置尺寸。
    #[test]
    fn configure_then_map_full_report_preserves_size() {
        let report = SurfaceAdmissionContractRunner::new()
            .report_for_scenario(BackendSurfaceAdmissionContractScenario::ConfigureThenMap);
        let expected_size = BackendSurfaceSize::new(1280, 720).expect("测试尺寸必须有效");

        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCreateWindow {
                size: Some(size),
                ..
            }] if *size == expected_size
        ));
    }

    /// 验证 MapUnmapRemap 最终仍生成创建预检。
    #[test]
    fn map_unmap_remap_finishes_with_create_preview() {
        let report = SurfaceAdmissionContractRunner::new()
            .report_for_scenario(BackendSurfaceAdmissionContractScenario::MapUnmapRemap);

        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCreateWindow { .. }]
        ));
    }

    /// 验证 Destroy 最终生成关闭意图和关闭预检。
    #[test]
    fn destroy_finishes_with_close_contract() {
        let report = SurfaceAdmissionContractRunner::new()
            .report_for_scenario(BackendSurfaceAdmissionContractScenario::Destroy);

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Close { .. }]
        ));
        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCloseWindow { .. }]
        ));
    }

    /// 验证非法销毁后重映射形成来源失败快照。
    #[test]
    fn invalid_destroyed_remap_snapshot_is_source_failed() {
        let snapshot = snapshot(BackendSurfaceAdmissionContractScenario::InvalidDestroyedRemap);

        assert_eq!(
            snapshot.status,
            BackendSurfaceAdmissionPipelineStatus::SourceTraceFailed
        );
    }

    /// 验证非法销毁后重映射保留失败事件下标。
    #[test]
    fn invalid_destroyed_remap_preserves_failed_at() {
        let snapshot = snapshot(BackendSurfaceAdmissionContractScenario::InvalidDestroyedRemap);

        assert_eq!(snapshot.source_failed_at, Some(2));
    }

    /// 验证非法销毁后重映射保留结构化来源错误。
    #[test]
    fn invalid_destroyed_remap_preserves_source_error() {
        let report = SurfaceAdmissionContractRunner::new()
            .report_for_scenario(BackendSurfaceAdmissionContractScenario::InvalidDestroyedRemap);

        assert_eq!(
            report.trace_report.error,
            Some(BackendSurfaceLifecycleError::AlreadyDestroyed {
                id: BackendSurfaceId::new(1)
            })
        );
    }

    /// 验证来源失败后仍为 tombstone 生成关闭意图和预检。
    #[test]
    fn invalid_destroyed_remap_keeps_close_contract() {
        let report = SurfaceAdmissionContractRunner::new()
            .report_for_scenario(BackendSurfaceAdmissionContractScenario::InvalidDestroyedRemap);

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Close { .. }]
        ));
        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCloseWindow { .. }]
        ));
    }

    /// 验证 MultiSurface 快照固定为三个有序结果。
    #[test]
    fn multi_surface_snapshot_matches_golden_rule() {
        assert_eq!(
            snapshot(BackendSurfaceAdmissionContractScenario::MultiSurface),
            BackendSurfaceAdmissionContractSnapshot {
                scenario_name: "MultiSurface",
                status: BackendSurfaceAdmissionPipelineStatus::Complete,
                surface_count: 3,
                intent_count: 3,
                action_count: 3,
                warning_count: 0,
                source_failed_at: None,
            }
        );
    }

    /// 验证 MultiSurface 动作按 surface ID 保持创建、隐藏、关闭顺序。
    #[test]
    fn multi_surface_action_order_is_stable() {
        let report = SurfaceAdmissionContractRunner::new()
            .report_for_scenario(BackendSurfaceAdmissionContractScenario::MultiSurface);
        let actions = report.preview_report.actions.as_slice();

        assert!(matches!(
            actions,
            [
                BackendWindowAdmissionPreviewAction::WouldCreateWindow { .. },
                BackendWindowAdmissionPreviewAction::WouldHideWindow { .. },
                BackendWindowAdmissionPreviewAction::WouldCloseWindow { .. }
            ]
        ));
        assert_eq!(
            actions
                .iter()
                .map(|action| action.surface_id().value())
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    /// 验证全部快照始终使用契约枚举定义的稳定顺序。
    #[test]
    fn snapshots_for_all_scenarios_have_stable_order() {
        let names: Vec<_> = SurfaceAdmissionContractRunner::new()
            .snapshots_for_all_scenarios()
            .into_iter()
            .map(|snapshot| snapshot.scenario_name)
            .collect();

        assert_eq!(
            names,
            vec![
                "Empty",
                "SingleMap",
                "ConfigureThenMap",
                "MapUnmapRemap",
                "Destroy",
                "InvalidDestroyedRemap",
                "MultiSurface",
            ]
        );
    }

    /// 验证每个契约场景都提供固定名称。
    #[test]
    fn contract_scenario_names_are_stable() {
        let names = BackendSurfaceAdmissionContractScenario::ALL
            .map(BackendSurfaceAdmissionContractScenario::name);

        assert_eq!(
            names,
            [
                "Empty",
                "SingleMap",
                "ConfigureThenMap",
                "MapUnmapRemap",
                "Destroy",
                "InvalidDestroyedRemap",
                "MultiSurface",
            ]
        );
    }

    /// 验证契约摘要中的场景顺序稳定。
    #[test]
    fn contract_summary_text_has_stable_order() {
        let text = SurfaceAdmissionContractRunner::new().contract_summary_text();
        let positions: Vec<_> = BackendSurfaceAdmissionContractScenario::ALL
            .into_iter()
            .map(|scenario| text.find(scenario.name()).expect("摘要必须包含全部场景"))
            .collect();

        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    /// 验证契约摘要包含固定 Empty 行。
    #[test]
    fn contract_summary_text_contains_empty_line() {
        let text = SurfaceAdmissionContractRunner::new().contract_summary_text();

        assert!(text.contains("* Empty: Complete, surfaces=0, intents=0, actions=0, warnings=0\n"));
    }

    /// 验证契约摘要包含固定 SingleMap 行。
    #[test]
    fn contract_summary_text_contains_single_map_line() {
        let text = SurfaceAdmissionContractRunner::new().contract_summary_text();

        assert!(
            text.contains("* SingleMap: Complete, surfaces=1, intents=1, actions=1, warnings=1\n")
        );
    }

    /// 验证契约摘要包含固定 ConfigureThenMap 行。
    #[test]
    fn contract_summary_text_contains_configure_then_map_line() {
        let text = SurfaceAdmissionContractRunner::new().contract_summary_text();

        assert!(text.contains(
            "* ConfigureThenMap: Complete, surfaces=1, intents=1, actions=1, warnings=0\n"
        ));
    }

    /// 验证契约摘要包含固定来源失败行。
    #[test]
    fn contract_summary_text_contains_invalid_destroyed_remap_line() {
        let text = SurfaceAdmissionContractRunner::new().contract_summary_text();

        assert!(text.contains(
            "* InvalidDestroyedRemap: SourceTraceFailed, surfaces=1, intents=1, actions=1, warnings=2, source_failed_at=2\n"
        ));
    }

    /// 验证生产代码不依赖核心模块。
    #[test]
    fn contract_production_code_does_not_depend_on_core() {
        assert!(!production_source().contains("crate::core"));
    }

    /// 验证生产代码不依赖通用后端模块。
    #[test]
    fn contract_production_code_does_not_depend_on_backend() {
        assert!(!production_source().contains("crate::backend"));
    }

    /// 验证生产代码不依赖系统后端 crate。
    #[test]
    fn contract_production_code_does_not_depend_on_system_backend() {
        assert!(!production_source().contains("smithay::"));
    }

    /// 验证生产代码不依赖平台专属 API。
    #[test]
    fn contract_production_code_does_not_depend_on_platform_api() {
        let source = production_source();

        for forbidden in ["std::os::unix", "libc::", "UnixStream", "XDG_RUNTIME_DIR"] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证生产代码不构造或提交核心事件路径类型。
    #[test]
    fn contract_production_code_does_not_use_event_submission_path() {
        let source = production_source();

        for forbidden in [
            "BackendEvent",
            "CoreCommand",
            "BackendDriverRunner",
            "handle_command",
            "dispatch_action",
        ] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证生产代码不引用核心放置模型类型。
    #[test]
    fn contract_production_code_does_not_use_core_placement_types() {
        let source = production_source();

        for forbidden in ["Workspace", "Slot", "WindowId"] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证生产逻辑不使用 panic、unwrap 或 expect。
    #[test]
    fn contract_production_code_has_no_panic_shortcuts() {
        let source = production_source();

        for forbidden in ["panic!", ".unwrap(", ".expect("] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证核心和通用后端源码没有反向依赖契约类型。
    #[test]
    fn core_and_backend_do_not_depend_on_contract_types() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        for directory in ["src/core", "src/backend"] {
            let path = manifest_dir.join(directory);
            assert!(!rust_tree_contains(&path, "surface_admission_contract"));
            assert!(!rust_tree_contains(
                &path,
                "BackendSurfaceAdmissionContract"
            ));
            assert!(!rust_tree_contains(&path, "SurfaceAdmissionContractRunner"));
        }
    }

    /// 验证契约快照保持可克隆和可直接比较。
    #[test]
    fn contract_snapshot_is_cloneable_and_comparable() {
        let snapshot = snapshot(BackendSurfaceAdmissionContractScenario::SingleMap);

        assert_eq!(snapshot.clone(), snapshot);
    }

    /// 验证纯数据 probe 不会宣称真实 surface 能力。
    #[test]
    fn real_surface_capability_remains_disabled() {
        assert!(!BackendRuntimeCapabilities::smithay_probe().supports_real_wayland_surfaces);
    }

    /// 验证纯数据 probe 不会宣称 GPU 渲染能力。
    #[test]
    fn gpu_rendering_capability_remains_disabled() {
        assert!(!BackendRuntimeCapabilities::smithay_probe().supports_gpu_rendering);
    }
}
