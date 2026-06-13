//! Surface 生命周期到窗口接纳预检的纯数据集成管线。
//!
//! 本模块只编排已有的生命周期回放、候选意图规划和接纳预检，不重复实现
//! 任一阶段的规则，也不提交任何状态变化。
//!
//! Pipeline contract: `trace -> intent -> preview` 的每一步都生成独立报告。来源
//! trace 失败时，后两步只消费失败前保留的快照，不能把部分结果解释为接纳成功。

use crate::smithay_backend::{
    surface_lifecycle::{BackendSurfaceLifecycleEvent, BackendSurfaceRegistry},
    surface_trace::{
        BackendSurfaceMockAdapter, BackendSurfaceTrace, BackendSurfaceTraceReport,
        BackendSurfaceTraceScenario,
    },
    surface_window_intent::{BackendWindowCandidateIntentReport, SurfaceWindowIntentPlanner},
    window_admission_preview::{
        BackendWindowAdmissionPreviewReport, WindowAdmissionPreviewPlanner,
    },
};

/// Surface 接纳预检管线的整体状态。
///
/// Preview warning 不会改变该状态；只有来源 trace 失败才会形成失败状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSurfaceAdmissionPipelineStatus {
    /// 来源 trace 完整执行，后续纯数据阶段也已生成报告。
    Complete,

    /// 来源 trace 失败，但失败前的最终记录仍继续生成意图和预检报告。
    SourceTraceFailed,
}

/// Surface 接纳预检管线的完整报告。
///
/// 三个阶段报告全部保留，便于调用方检查状态快照、候选意图和预检动作。
/// `Complete` 仅表示纯数据管线完成，不表示真实 surface 或核心窗口已建立。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSurfaceAdmissionPipelineReport {
    /// 管线整体状态。
    pub status: BackendSurfaceAdmissionPipelineStatus,

    /// 生命周期 trace 执行报告。
    pub trace_report: BackendSurfaceTraceReport,

    /// 从最终 surface 记录规划出的候选意图报告。
    pub intent_report: BackendWindowCandidateIntentReport,

    /// 从候选意图生成的接纳预检报告。
    pub preview_report: BackendWindowAdmissionPreviewReport,
}

impl BackendSurfaceAdmissionPipelineReport {
    /// 创建用于快速诊断的计数摘要。
    pub fn summary(&self) -> BackendSurfaceAdmissionPipelineSummary {
        BackendSurfaceAdmissionPipelineSummary::from(self)
    }

    /// 判断来源 trace 是否完整成功。
    pub fn source_succeeded(&self) -> bool {
        self.status == BackendSurfaceAdmissionPipelineStatus::Complete
            && self.trace_report.is_success()
            && self.intent_report.source_succeeded()
            && self.preview_report.source_succeeded()
    }
}

/// Surface 接纳预检管线的轻量摘要。
///
/// 摘要只提供计数和失败位置，不替代完整阶段报告。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSurfaceAdmissionPipelineSummary {
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

impl From<&BackendSurfaceAdmissionPipelineReport> for BackendSurfaceAdmissionPipelineSummary {
    /// 从完整管线报告提取稳定计数。
    fn from(report: &BackendSurfaceAdmissionPipelineReport) -> Self {
        Self {
            status: report.status,
            surface_count: report.trace_report.final_records.len(),
            intent_count: report.intent_report.intents.len(),
            action_count: report.preview_report.actions.len(),
            warning_count: report.preview_report.warnings.len(),
            source_failed_at: report.trace_report.failed_at,
        }
    }
}

/// Surface 生命周期到窗口接纳预检的无状态集成 runner。
///
/// Runner 只组合既有公开 API。来源 trace 失败时仍会基于失败前的最终记录生成
/// 候选意图与预检报告，并通过整体状态明确标记失败。
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceAdmissionPipelineRunner {
    intent_planner: SurfaceWindowIntentPlanner,
    preview_planner: WindowAdmissionPreviewPlanner,
}

impl SurfaceAdmissionPipelineRunner {
    /// 创建使用默认纯数据规划器的管线 runner。
    pub const fn new() -> Self {
        Self {
            intent_planner: SurfaceWindowIntentPlanner::new(),
            preview_planner: WindowAdmissionPreviewPlanner::new(),
        }
    }

    /// 使用新建生命周期注册表运行一条 trace。
    pub fn run_trace(&self, trace: &BackendSurfaceTrace) -> BackendSurfaceAdmissionPipelineReport {
        let mut registry = BackendSurfaceRegistry::new();
        self.run_trace_with_registry(trace, &mut registry)
    }

    /// 把生命周期事件集合构造成 trace 后运行完整管线。
    pub fn run_events(
        &self,
        events: impl IntoIterator<Item = BackendSurfaceLifecycleEvent>,
    ) -> BackendSurfaceAdmissionPipelineReport {
        let trace = BackendSurfaceTrace::from_events(events);
        self.run_trace(&trace)
    }

    /// 使用默认 mock adapter 构造场景并运行完整管线。
    pub fn run_scenario(
        &self,
        scenario: BackendSurfaceTraceScenario,
    ) -> BackendSurfaceAdmissionPipelineReport {
        let mut adapter = BackendSurfaceMockAdapter::new();
        let trace = scenario.build(&mut adapter);
        self.run_trace(&trace)
    }

    /// 使用调用方提供的生命周期注册表运行一条 trace。
    ///
    /// 该入口允许测试和未来纯数据适配器累积状态。状态转换仍完全由 trace 与
    /// registry 的既有实现负责。
    pub fn run_trace_with_registry(
        &self,
        trace: &BackendSurfaceTrace,
        registry: &mut BackendSurfaceRegistry,
    ) -> BackendSurfaceAdmissionPipelineReport {
        let trace_report = trace.run(registry);
        let status = if trace_report.is_success() {
            BackendSurfaceAdmissionPipelineStatus::Complete
        } else {
            BackendSurfaceAdmissionPipelineStatus::SourceTraceFailed
        };
        let intent_report = self.intent_planner.intents_for_trace_report(&trace_report);
        let preview_report = self.preview_planner.preview_intent_report(&intent_report);

        BackendSurfaceAdmissionPipelineReport {
            status,
            trace_report,
            intent_report,
            preview_report,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        BackendSurfaceAdmissionPipelineReport, BackendSurfaceAdmissionPipelineStatus,
        BackendSurfaceAdmissionPipelineSummary, SurfaceAdmissionPipelineRunner,
    };
    use crate::smithay_backend::{
        surface_lifecycle::{
            BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleEvent,
            BackendSurfaceRegistry, BackendSurfaceSize,
        },
        surface_trace::{
            BackendSurfaceMockAdapter, BackendSurfaceTrace, BackendSurfaceTraceScenario,
        },
        surface_window_intent::BackendWindowCandidateIntent,
        window_admission_preview::{
            BackendWindowAdmissionPreviewAction, BackendWindowAdmissionPreviewWarning,
        },
    };

    /// 运行指定 mock 场景。
    fn run_scenario(
        scenario: BackendSurfaceTraceScenario,
    ) -> BackendSurfaceAdmissionPipelineReport {
        SurfaceAdmissionPipelineRunner::new().run_scenario(scenario)
    }

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

    /// 验证空 trace 可以完整运行。
    #[test]
    fn empty_trace_runs_successfully() {
        let report = SurfaceAdmissionPipelineRunner::new().run_trace(&BackendSurfaceTrace::new());

        assert!(report.source_succeeded());
        assert!(report.trace_report.is_success());
        assert!(report.intent_report.intents.is_empty());
        assert!(report.preview_report.actions.is_empty());
    }

    /// 验证空 trace 的管线状态为 Complete。
    #[test]
    fn empty_trace_status_is_complete() {
        let report = SurfaceAdmissionPipelineRunner::new().run_trace(&BackendSurfaceTrace::new());

        assert_eq!(
            report.status,
            BackendSurfaceAdmissionPipelineStatus::Complete
        );
    }

    /// 验证空 trace 的 summary 保持成功状态和全部零计数。
    #[test]
    fn empty_trace_summary_has_zero_counts() {
        let report = SurfaceAdmissionPipelineRunner::new().run_trace(&BackendSurfaceTrace::new());

        assert_eq!(
            report.summary(),
            BackendSurfaceAdmissionPipelineSummary {
                status: BackendSurfaceAdmissionPipelineStatus::Complete,
                surface_count: 0,
                intent_count: 0,
                action_count: 0,
                warning_count: 0,
                source_failed_at: None,
            }
        );
    }

    /// 验证 SingleMap 场景生成 Create 候选意图。
    #[test]
    fn single_map_builds_create_intent() {
        let report = run_scenario(BackendSurfaceTraceScenario::SingleMap);

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Create { .. }]
        ));
    }

    /// 验证 SingleMap 场景生成创建窗口预检动作。
    #[test]
    fn single_map_builds_would_create_action() {
        let report = run_scenario(BackendSurfaceTraceScenario::SingleMap);

        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCreateWindow { .. }]
        ));
    }

    /// 验证 ConfigureThenMap 场景把尺寸保留到意图和预检动作。
    #[test]
    fn configure_then_map_preserves_size() {
        let report = run_scenario(BackendSurfaceTraceScenario::ConfigureThenMap);
        let expected_size = BackendSurfaceSize {
            width: 1280,
            height: 720,
        };

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Create {
                size: Some(size),
                ..
            }] if *size == expected_size
        ));
        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCreateWindow {
                size: Some(size),
                ..
            }] if *size == expected_size
        ));
    }

    /// 验证 remap 场景最终仍生成 Create 意图。
    #[test]
    fn map_unmap_remap_finishes_with_create_intent() {
        let report = run_scenario(BackendSurfaceTraceScenario::MapUnmapRemap);

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Create { title, .. }]
                if title.as_deref() == Some("Remapped Mock Surface")
        ));
    }

    /// 验证 remap 场景最终仍生成创建预检动作。
    #[test]
    fn map_unmap_remap_finishes_with_create_action() {
        let report = run_scenario(BackendSurfaceTraceScenario::MapUnmapRemap);

        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCreateWindow { .. }]
        ));
    }

    /// 验证 Destroy 场景生成 Close 候选意图。
    #[test]
    fn destroy_builds_close_intent() {
        let report = run_scenario(BackendSurfaceTraceScenario::Destroy);

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Close { .. }]
        ));
    }

    /// 验证 Destroy 场景生成关闭窗口预检动作。
    #[test]
    fn destroy_builds_would_close_action() {
        let report = run_scenario(BackendSurfaceTraceScenario::Destroy);

        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCloseWindow { .. }]
        ));
    }

    /// 验证销毁后 remap 的管线状态为来源失败。
    #[test]
    fn invalid_destroyed_remap_status_is_source_failed() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert_eq!(
            report.status,
            BackendSurfaceAdmissionPipelineStatus::SourceTraceFailed
        );
        assert!(!report.source_succeeded());
    }

    /// 验证失败场景完整保留失败事件下标。
    #[test]
    fn invalid_destroyed_remap_preserves_failed_at() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert_eq!(report.trace_report.failed_at, Some(2));
        assert_eq!(report.intent_report.source_failed_at, Some(2));
        assert_eq!(report.preview_report.source_failed_at, Some(2));
    }

    /// 验证失败场景完整保留结构化来源错误。
    #[test]
    fn invalid_destroyed_remap_preserves_source_error() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);
        let expected = Some(BackendSurfaceLifecycleError::AlreadyDestroyed {
            id: BackendSurfaceId::new(1),
        });

        assert_eq!(report.trace_report.error, expected);
        assert_eq!(report.intent_report.source_error, expected);
        assert_eq!(report.preview_report.source_error, expected);
    }

    /// 验证失败场景仍根据最终 tombstone 生成 Close 意图。
    #[test]
    fn invalid_destroyed_remap_still_builds_close_intent() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Close { .. }]
        ));
    }

    /// 验证失败场景仍根据最终 tombstone 生成关闭预检动作。
    #[test]
    fn invalid_destroyed_remap_still_builds_would_close_action() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [BackendWindowAdmissionPreviewAction::WouldCloseWindow { .. }]
        ));
    }

    /// 验证失败场景生成来源下标和错误 warning。
    #[test]
    fn invalid_destroyed_remap_builds_source_warnings() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert!(
            report
                .preview_report
                .warnings
                .iter()
                .any(|warning| matches!(
                    warning,
                    BackendWindowAdmissionPreviewWarning::SourceTraceFailed { failed_at: 2 }
                ))
        );
        assert!(
            report
                .preview_report
                .warnings
                .iter()
                .any(|warning| matches!(
                    warning,
                    BackendWindowAdmissionPreviewWarning::SourceTraceError { .. }
                ))
        );
    }

    /// 验证缺失 metadata 和尺寸只形成 warning，不改变成功状态。
    #[test]
    fn missing_create_data_only_builds_warnings() {
        let id = BackendSurfaceId::new(7);
        let report = SurfaceAdmissionPipelineRunner::new().run_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: None,
                app_id: None,
            },
        ]);

        assert_eq!(
            report.status,
            BackendSurfaceAdmissionPipelineStatus::Complete
        );
        assert!(report.source_succeeded());
        assert_eq!(report.preview_report.actions.len(), 1);
        assert_eq!(
            report.preview_report.warnings,
            vec![
                BackendWindowAdmissionPreviewWarning::MissingTitle {
                    candidate_id: report.preview_report.actions[0].candidate_id(),
                },
                BackendWindowAdmissionPreviewWarning::MissingAppId {
                    candidate_id: report.preview_report.actions[0].candidate_id(),
                },
                BackendWindowAdmissionPreviewWarning::MissingSize {
                    candidate_id: report.preview_report.actions[0].candidate_id(),
                },
            ]
        );
    }

    /// 验证 preview warning 不会把完整 trace 标记为失败。
    #[test]
    fn preview_warnings_do_not_change_pipeline_status() {
        let report = run_scenario(BackendSurfaceTraceScenario::SingleMap);

        assert!(!report.preview_report.warnings.is_empty());
        assert_eq!(
            report.status,
            BackendSurfaceAdmissionPipelineStatus::Complete
        );
    }

    /// 验证事件入口和 trace 入口对同一序列生成相同报告。
    #[test]
    fn run_events_matches_run_trace() {
        let mut adapter = BackendSurfaceMockAdapter::new();
        let trace = adapter.single_surface_configure_then_map_trace();
        let runner = SurfaceAdmissionPipelineRunner::new();

        let from_trace = runner.run_trace(&trace);
        let from_events = runner.run_events(trace.events().iter().cloned());

        assert_eq!(from_events, from_trace);
    }

    /// 验证场景入口和显式场景 trace 生成相同报告。
    #[test]
    fn run_scenario_matches_built_trace() {
        let scenario = BackendSurfaceTraceScenario::MultiSurface;
        let runner = SurfaceAdmissionPipelineRunner::new();
        let mut adapter = BackendSurfaceMockAdapter::new();
        let trace = scenario.build(&mut adapter);

        assert_eq!(runner.run_scenario(scenario), runner.run_trace(&trace));
    }

    /// 验证外部 registry 会被实际使用并保留执行结果。
    #[test]
    fn run_trace_with_registry_updates_external_registry() {
        let id = BackendSurfaceId::new(40);
        let trace = BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("External Registry".to_string()),
                app_id: Some("sky-mirror.external".to_string()),
            },
        ]);
        let mut registry = BackendSurfaceRegistry::new();

        let report =
            SurfaceAdmissionPipelineRunner::new().run_trace_with_registry(&trace, &mut registry);

        assert!(registry.get_surface(id).is_some());
        assert_eq!(report.trace_report.final_records[0].id, id);
    }

    /// 验证外部 registry 的已有状态参与后续 trace。
    #[test]
    fn run_trace_with_registry_uses_existing_state() {
        let id = BackendSurfaceId::new(41);
        let mut registry = BackendSurfaceRegistry::new();
        registry
            .create_surface_with_id(id)
            .expect("测试 surface 必须创建成功");
        let trace = BackendSurfaceTrace::from_events([BackendSurfaceLifecycleEvent::Mapped {
            id,
            title: Some("Existing Surface".to_string()),
            app_id: None,
        }]);

        let report =
            SurfaceAdmissionPipelineRunner::new().run_trace_with_registry(&trace, &mut registry);

        assert_eq!(
            report.status,
            BackendSurfaceAdmissionPipelineStatus::Complete
        );
        assert!(matches!(
            report.intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Create { .. }]
        ));
    }

    /// 验证多 surface 场景按稳定 surface ID 顺序生成动作。
    #[test]
    fn multi_surface_actions_have_stable_order() {
        let report = run_scenario(BackendSurfaceTraceScenario::MultiSurface);
        let ids: Vec<_> = report
            .preview_report
            .actions
            .iter()
            .map(BackendWindowAdmissionPreviewAction::surface_id)
            .map(BackendSurfaceId::value)
            .collect();

        assert_eq!(ids, vec![1, 2, 3]);
        assert!(matches!(
            report.preview_report.actions.as_slice(),
            [
                BackendWindowAdmissionPreviewAction::WouldCreateWindow { .. },
                BackendWindowAdmissionPreviewAction::WouldHideWindow { .. },
                BackendWindowAdmissionPreviewAction::WouldCloseWindow { .. },
            ]
        ));
    }

    /// 验证 summary 的 surface 数量来自最终记录。
    #[test]
    fn summary_surface_count_matches_final_records() {
        let report = run_scenario(BackendSurfaceTraceScenario::MultiSurface);
        let summary = report.summary();

        assert_eq!(
            summary.surface_count,
            report.trace_report.final_records.len()
        );
        assert_eq!(summary.surface_count, 3);
    }

    /// 验证 summary 的意图数量来自意图报告。
    #[test]
    fn summary_intent_count_matches_intent_report() {
        let report = run_scenario(BackendSurfaceTraceScenario::MultiSurface);
        let summary = report.summary();

        assert_eq!(summary.intent_count, report.intent_report.intents.len());
        assert_eq!(summary.intent_count, 3);
    }

    /// 验证 summary 的动作数量来自预检报告。
    #[test]
    fn summary_action_count_matches_preview_report() {
        let report = run_scenario(BackendSurfaceTraceScenario::MultiSurface);
        let summary = report.summary();

        assert_eq!(summary.action_count, report.preview_report.actions.len());
        assert_eq!(summary.action_count, 3);
    }

    /// 验证 summary 的 warning 数量来自预检报告。
    #[test]
    fn summary_warning_count_matches_preview_report() {
        let report = run_scenario(BackendSurfaceTraceScenario::SingleMap);
        let summary = report.summary();

        assert_eq!(summary.warning_count, report.preview_report.warnings.len());
        assert_eq!(summary.warning_count, 1);
    }

    /// 验证失败 summary 保留整体状态和失败下标。
    #[test]
    fn failed_summary_preserves_status_and_failed_at() {
        let report = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert_eq!(
            report.summary(),
            BackendSurfaceAdmissionPipelineSummary {
                status: BackendSurfaceAdmissionPipelineStatus::SourceTraceFailed,
                surface_count: 1,
                intent_count: 1,
                action_count: 1,
                warning_count: 2,
                source_failed_at: Some(2),
            }
        );
    }

    /// 验证管线报告保持纯数据可克隆和可比较。
    #[test]
    fn pipeline_report_is_cloneable_and_comparable() {
        let report = run_scenario(BackendSurfaceTraceScenario::ConfigureThenMap);

        assert_eq!(report.clone(), report);
    }

    /// 验证管线状态可复制并直接比较。
    #[test]
    fn pipeline_status_is_copyable_and_comparable() {
        let status = BackendSurfaceAdmissionPipelineStatus::Complete;
        let copied = status;

        assert_eq!(copied, status);
    }

    /// 验证管线 summary 保持纯数据可复制和可比较。
    #[test]
    fn pipeline_summary_is_copyable_and_comparable() {
        let summary = run_scenario(BackendSurfaceTraceScenario::SingleMap).summary();
        let copied = summary;

        assert_eq!(copied, summary);
    }

    /// 验证生产代码不依赖核心模块。
    #[test]
    fn pipeline_production_code_does_not_depend_on_core() {
        assert!(!production_source().contains("crate::core"));
    }

    /// 验证生产代码不依赖通用后端模块。
    #[test]
    fn pipeline_production_code_does_not_depend_on_backend() {
        assert!(!production_source().contains("crate::backend"));
    }

    /// 验证生产代码不依赖 Smithay crate。
    #[test]
    fn pipeline_production_code_does_not_depend_on_smithay() {
        assert!(!production_source().contains("smithay::"));
    }

    /// 验证生产代码不依赖 Linux 或 Unix API。
    #[test]
    fn pipeline_production_code_does_not_depend_on_linux_api() {
        let source = production_source();

        for forbidden in ["std::os::unix", "libc::", "UnixStream", "XDG_RUNTIME_DIR"] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证生产代码不构造或提交状态事件路径类型。
    #[test]
    fn pipeline_production_code_does_not_use_event_submission_path() {
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
    fn pipeline_production_code_does_not_use_core_placement_types() {
        let source = production_source();

        for forbidden in ["Workspace", "Slot", "WindowId"] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证核心和通用后端源码没有反向依赖管线类型。
    #[test]
    fn core_and_backend_do_not_depend_on_pipeline_types() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        for directory in ["src/core", "src/backend"] {
            let path = manifest_dir.join(directory);
            assert!(!rust_tree_contains(&path, "surface_admission_pipeline"));
            assert!(!rust_tree_contains(
                &path,
                "BackendSurfaceAdmissionPipeline"
            ));
            assert!(!rust_tree_contains(&path, "SurfaceAdmissionPipelineRunner"));
        }
    }

    /// 验证生产逻辑不使用 panic、unwrap 或 expect。
    #[test]
    fn pipeline_production_code_has_no_panic_shortcuts() {
        let source = production_source();

        for forbidden in ["panic!", ".unwrap(", ".expect("] {
            assert!(!source.contains(forbidden));
        }
    }
}
