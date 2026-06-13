//! 窗口候选意图到核心接纳动作的纯数据预检层。
//!
//! 本模块只描述未来接纳候选窗口时可能发生的动作，并为缺失信息生成结构化
//! warning。它不会提交事件、分配核心资源或修改任何核心状态。

use std::fmt;

use crate::smithay_backend::{
    surface_lifecycle::{BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceSize},
    surface_window_intent::{
        BackendWindowCandidateId, BackendWindowCandidateIntent, BackendWindowCandidateIntentReport,
    },
};

/// 候选窗口通过预检后可能触发的接纳动作。
///
/// 所有变体都只是未来行为的纯数据描述，不表示动作已经执行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendWindowAdmissionPreviewAction {
    /// 未来可能创建候选窗口。
    WouldCreateWindow {
        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 可选窗口标题。
        title: Option<String>,

        /// 可选应用标识。
        app_id: Option<String>,

        /// 最近一次已知尺寸。
        size: Option<BackendSurfaceSize>,
    },

    /// 未来可能更新候选窗口尺寸。
    WouldUpdateSize {
        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 新的逻辑尺寸。
        size: BackendSurfaceSize,
    },

    /// 未来可能隐藏候选窗口。
    WouldHideWindow {
        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 来源 surface。
        surface_id: BackendSurfaceId,
    },

    /// 未来可能关闭候选窗口。
    WouldCloseWindow {
        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 来源 surface。
        surface_id: BackendSurfaceId,
    },

    /// 当前接纳预检不会单独应用 metadata-only 更新。
    WouldIgnoreMetadataOnly {
        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 可选窗口标题。
        title: Option<String>,

        /// 可选应用标识。
        app_id: Option<String>,
    },
}

impl BackendWindowAdmissionPreviewAction {
    /// 返回动作对应的候选窗口标识。
    pub const fn candidate_id(&self) -> BackendWindowCandidateId {
        match self {
            Self::WouldCreateWindow { candidate_id, .. }
            | Self::WouldUpdateSize { candidate_id, .. }
            | Self::WouldHideWindow { candidate_id, .. }
            | Self::WouldCloseWindow { candidate_id, .. }
            | Self::WouldIgnoreMetadataOnly { candidate_id, .. } => *candidate_id,
        }
    }

    /// 返回动作对应的 surface 标识。
    pub const fn surface_id(&self) -> BackendSurfaceId {
        match self {
            Self::WouldCreateWindow { surface_id, .. }
            | Self::WouldUpdateSize { surface_id, .. }
            | Self::WouldHideWindow { surface_id, .. }
            | Self::WouldCloseWindow { surface_id, .. }
            | Self::WouldIgnoreMetadataOnly { surface_id, .. } => *surface_id,
        }
    }
}

/// 窗口接纳预检产生的结构化 warning。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendWindowAdmissionPreviewWarning {
    /// 创建候选窗口时缺少标题。
    MissingTitle {
        /// 缺少标题的候选窗口。
        candidate_id: BackendWindowCandidateId,
    },

    /// 创建候选窗口时缺少应用标识。
    MissingAppId {
        /// 缺少应用标识的候选窗口。
        candidate_id: BackendWindowCandidateId,
    },

    /// 创建候选窗口时缺少最近尺寸。
    MissingSize {
        /// 缺少尺寸的候选窗口。
        candidate_id: BackendWindowCandidateId,
    },

    /// 来源 trace 在指定事件下标失败。
    SourceTraceFailed {
        /// 首个失败事件下标。
        failed_at: usize,
    },

    /// 来源 trace 保留的结构化错误。
    SourceTraceError {
        /// 生命周期错误。
        error: BackendSurfaceLifecycleError,
    },

    /// Metadata-only 更新当前不会进入接纳动作。
    MetadataOnlyIgnored {
        /// 被忽略更新的候选窗口。
        candidate_id: BackendWindowCandidateId,
    },
}

impl fmt::Display for BackendWindowAdmissionPreviewWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTitle { candidate_id } => {
                write!(formatter, "候选窗口 {} 缺少标题", candidate_id.value())
            }
            Self::MissingAppId { candidate_id } => {
                write!(formatter, "候选窗口 {} 缺少应用标识", candidate_id.value())
            }
            Self::MissingSize { candidate_id } => {
                write!(formatter, "候选窗口 {} 缺少尺寸", candidate_id.value())
            }
            Self::SourceTraceFailed { failed_at } => {
                write!(formatter, "来源 surface trace 在事件 {failed_at} 失败")
            }
            Self::SourceTraceError { error } => {
                write!(formatter, "来源 surface trace 错误: {error}")
            }
            Self::MetadataOnlyIgnored { candidate_id } => write!(
                formatter,
                "候选窗口 {} 的 metadata-only 更新当前被忽略",
                candidate_id.value()
            ),
        }
    }
}

/// 窗口候选意图的接纳预检报告。
///
/// 报告保留来源失败信息，并按输入顺序记录动作和 warning。来源失败不会阻止
/// 对失败点之前已经生成的候选意图进行预检。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendWindowAdmissionPreviewReport {
    /// 按候选意图输入顺序生成的预检动作。
    pub actions: Vec<BackendWindowAdmissionPreviewAction>,

    /// 按候选意图输入顺序生成的 warning，来源失败 warning 最后追加。
    pub warnings: Vec<BackendWindowAdmissionPreviewWarning>,

    /// 来源 trace 首个失败事件的下标。
    pub source_failed_at: Option<usize>,

    /// 来源 trace 的结构化失败原因。
    pub source_error: Option<BackendSurfaceLifecycleError>,
}

impl BackendWindowAdmissionPreviewReport {
    /// 判断来源 trace 是否完整成功。
    pub fn source_succeeded(&self) -> bool {
        self.source_failed_at.is_none() && self.source_error.is_none()
    }
}

/// 候选窗口接纳预检的无状态规划器。
///
/// 规划器只翻译纯数据意图，不提交任何状态变化。
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowAdmissionPreviewPlanner;

impl WindowAdmissionPreviewPlanner {
    /// 创建无状态预检规划器。
    pub const fn new() -> Self {
        Self
    }

    /// 预检单个窗口候选意图。
    pub fn preview_intent(
        &self,
        intent: &BackendWindowCandidateIntent,
    ) -> BackendWindowAdmissionPreviewReport {
        self.preview_intents(std::slice::from_ref(intent))
    }

    /// 按输入顺序预检多个窗口候选意图。
    pub fn preview_intents(
        &self,
        intents: &[BackendWindowCandidateIntent],
    ) -> BackendWindowAdmissionPreviewReport {
        let mut actions = Vec::with_capacity(intents.len());
        let mut warnings = Vec::new();

        for intent in intents {
            match intent {
                BackendWindowCandidateIntent::Create {
                    surface_id,
                    candidate_id,
                    title,
                    app_id,
                    size,
                } => {
                    actions.push(BackendWindowAdmissionPreviewAction::WouldCreateWindow {
                        candidate_id: *candidate_id,
                        surface_id: *surface_id,
                        title: title.clone(),
                        app_id: app_id.clone(),
                        size: *size,
                    });

                    if title.is_none() {
                        warnings.push(BackendWindowAdmissionPreviewWarning::MissingTitle {
                            candidate_id: *candidate_id,
                        });
                    }
                    if app_id.is_none() {
                        warnings.push(BackendWindowAdmissionPreviewWarning::MissingAppId {
                            candidate_id: *candidate_id,
                        });
                    }
                    if size.is_none() {
                        warnings.push(BackendWindowAdmissionPreviewWarning::MissingSize {
                            candidate_id: *candidate_id,
                        });
                    }
                }
                BackendWindowCandidateIntent::UpdateMetadata {
                    surface_id,
                    candidate_id,
                    title,
                    app_id,
                } => {
                    actions.push(
                        BackendWindowAdmissionPreviewAction::WouldIgnoreMetadataOnly {
                            candidate_id: *candidate_id,
                            surface_id: *surface_id,
                            title: title.clone(),
                            app_id: app_id.clone(),
                        },
                    );
                    warnings.push(BackendWindowAdmissionPreviewWarning::MetadataOnlyIgnored {
                        candidate_id: *candidate_id,
                    });
                }
                BackendWindowCandidateIntent::UpdateSize {
                    surface_id,
                    candidate_id,
                    size,
                } => {
                    actions.push(BackendWindowAdmissionPreviewAction::WouldUpdateSize {
                        candidate_id: *candidate_id,
                        surface_id: *surface_id,
                        size: *size,
                    });
                }
                BackendWindowCandidateIntent::Hide {
                    surface_id,
                    candidate_id,
                } => {
                    actions.push(BackendWindowAdmissionPreviewAction::WouldHideWindow {
                        candidate_id: *candidate_id,
                        surface_id: *surface_id,
                    });
                }
                BackendWindowCandidateIntent::Close {
                    surface_id,
                    candidate_id,
                } => {
                    actions.push(BackendWindowAdmissionPreviewAction::WouldCloseWindow {
                        candidate_id: *candidate_id,
                        surface_id: *surface_id,
                    });
                }
            }
        }

        BackendWindowAdmissionPreviewReport {
            actions,
            warnings,
            source_failed_at: None,
            source_error: None,
        }
    }

    /// 预检候选意图报告，并原样保留来源失败信息。
    ///
    /// 来源失败 warning 固定追加在各意图 warning 之后，使顺序可预测且不会
    /// 打乱逐意图诊断。
    pub fn preview_intent_report(
        &self,
        report: &BackendWindowCandidateIntentReport,
    ) -> BackendWindowAdmissionPreviewReport {
        let mut preview = self.preview_intents(&report.intents);

        if let Some(failed_at) = report.source_failed_at {
            preview
                .warnings
                .push(BackendWindowAdmissionPreviewWarning::SourceTraceFailed { failed_at });
        }
        if let Some(error) = &report.source_error {
            preview
                .warnings
                .push(BackendWindowAdmissionPreviewWarning::SourceTraceError {
                    error: error.clone(),
                });
        }

        preview.source_failed_at = report.source_failed_at;
        preview.source_error = report.source_error.clone();
        preview
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        BackendWindowAdmissionPreviewAction, BackendWindowAdmissionPreviewReport,
        BackendWindowAdmissionPreviewWarning, WindowAdmissionPreviewPlanner,
    };
    use crate::smithay_backend::{
        surface_lifecycle::{BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceSize},
        surface_window_intent::{
            BackendWindowCandidateId, BackendWindowCandidateIntent,
            BackendWindowCandidateIntentReport,
        },
    };

    /// 创建测试用 surface 标识。
    fn surface_id(value: u64) -> BackendSurfaceId {
        BackendSurfaceId::new(value)
    }

    /// 创建与 surface 稳定对应的候选标识。
    fn candidate_id(value: u64) -> BackendWindowCandidateId {
        BackendWindowCandidateId::from_surface(surface_id(value))
    }

    /// 创建测试用非零尺寸。
    fn size(width: u32, height: u32) -> BackendSurfaceSize {
        BackendSurfaceSize::new(width, height).expect("测试尺寸必须有效")
    }

    /// 创建 metadata 和尺寸完整的 Create 意图。
    fn complete_create(value: u64) -> BackendWindowCandidateIntent {
        BackendWindowCandidateIntent::Create {
            surface_id: surface_id(value),
            candidate_id: candidate_id(value),
            title: Some(format!("窗口 {value}")),
            app_id: Some(format!("app.{value}")),
            size: Some(size(1280, 720)),
        }
    }

    /// 创建保留来源错误的候选意图报告。
    fn failed_intent_report() -> BackendWindowCandidateIntentReport {
        BackendWindowCandidateIntentReport {
            intents: vec![complete_create(7)],
            source_failed_at: Some(3),
            source_error: Some(BackendSurfaceLifecycleError::AlreadyDestroyed {
                id: surface_id(8),
            }),
        }
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

    /// 递归检查目录中的 Rust 文件是否包含指定文本。
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

    /// 验证 Create 意图生成创建预检动作。
    #[test]
    fn create_intent_builds_would_create_action() {
        let intent = complete_create(1);
        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.actions,
            vec![BackendWindowAdmissionPreviewAction::WouldCreateWindow {
                candidate_id: candidate_id(1),
                surface_id: surface_id(1),
                title: Some("窗口 1".to_string()),
                app_id: Some("app.1".to_string()),
                size: Some(size(1280, 720)),
            }]
        );
    }

    /// 验证 Create 缺少标题时生成明确 warning。
    #[test]
    fn create_without_title_builds_missing_title_warning() {
        let mut intent = complete_create(1);
        if let BackendWindowCandidateIntent::Create { title, .. } = &mut intent {
            *title = None;
        }

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.warnings,
            vec![BackendWindowAdmissionPreviewWarning::MissingTitle {
                candidate_id: candidate_id(1)
            }]
        );
    }

    /// 验证 Create 缺少应用标识时生成明确 warning。
    #[test]
    fn create_without_app_id_builds_missing_app_id_warning() {
        let mut intent = complete_create(1);
        if let BackendWindowCandidateIntent::Create { app_id, .. } = &mut intent {
            *app_id = None;
        }

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.warnings,
            vec![BackendWindowAdmissionPreviewWarning::MissingAppId {
                candidate_id: candidate_id(1)
            }]
        );
    }

    /// 验证 Create 缺少尺寸时生成明确 warning。
    #[test]
    fn create_without_size_builds_missing_size_warning() {
        let mut intent = complete_create(1);
        if let BackendWindowCandidateIntent::Create { size, .. } = &mut intent {
            *size = None;
        }

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.warnings,
            vec![BackendWindowAdmissionPreviewWarning::MissingSize {
                candidate_id: candidate_id(1)
            }]
        );
    }

    /// 验证完整 Create 不产生缺失信息 warning。
    #[test]
    fn complete_create_has_no_missing_warning() {
        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&complete_create(1));

        assert!(report.warnings.is_empty());
    }

    /// 验证 UpdateSize 意图生成尺寸更新预检动作。
    #[test]
    fn update_size_intent_builds_would_update_size_action() {
        let intent = BackendWindowCandidateIntent::UpdateSize {
            surface_id: surface_id(2),
            candidate_id: candidate_id(2),
            size: size(900, 600),
        };

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.actions,
            vec![BackendWindowAdmissionPreviewAction::WouldUpdateSize {
                candidate_id: candidate_id(2),
                surface_id: surface_id(2),
                size: size(900, 600),
            }]
        );
    }

    /// 验证 Hide 意图生成隐藏预检动作。
    #[test]
    fn hide_intent_builds_would_hide_action() {
        let intent = BackendWindowCandidateIntent::Hide {
            surface_id: surface_id(3),
            candidate_id: candidate_id(3),
        };

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.actions,
            vec![BackendWindowAdmissionPreviewAction::WouldHideWindow {
                candidate_id: candidate_id(3),
                surface_id: surface_id(3),
            }]
        );
    }

    /// 验证 Close 意图生成关闭预检动作。
    #[test]
    fn close_intent_builds_would_close_action() {
        let intent = BackendWindowCandidateIntent::Close {
            surface_id: surface_id(4),
            candidate_id: candidate_id(4),
        };

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.actions,
            vec![BackendWindowAdmissionPreviewAction::WouldCloseWindow {
                candidate_id: candidate_id(4),
                surface_id: surface_id(4),
            }]
        );
    }

    /// 验证 UpdateMetadata 生成 metadata-only 忽略动作。
    #[test]
    fn metadata_intent_builds_ignore_action() {
        let intent = BackendWindowCandidateIntent::UpdateMetadata {
            surface_id: surface_id(5),
            candidate_id: candidate_id(5),
            title: Some("新标题".to_string()),
            app_id: Some("app.new".to_string()),
        };

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.actions,
            vec![
                BackendWindowAdmissionPreviewAction::WouldIgnoreMetadataOnly {
                    candidate_id: candidate_id(5),
                    surface_id: surface_id(5),
                    title: Some("新标题".to_string()),
                    app_id: Some("app.new".to_string()),
                }
            ]
        );
    }

    /// 验证 UpdateMetadata 同时生成结构化忽略 warning。
    #[test]
    fn metadata_intent_builds_ignored_warning() {
        let intent = BackendWindowCandidateIntent::UpdateMetadata {
            surface_id: surface_id(5),
            candidate_id: candidate_id(5),
            title: None,
            app_id: None,
        };

        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&intent);

        assert_eq!(
            report.warnings,
            vec![BackendWindowAdmissionPreviewWarning::MetadataOnlyIgnored {
                candidate_id: candidate_id(5)
            }]
        );
    }

    /// 验证多个意图的动作顺序保持输入顺序。
    #[test]
    fn multiple_intents_preserve_action_order() {
        let intents = vec![
            BackendWindowCandidateIntent::Hide {
                surface_id: surface_id(3),
                candidate_id: candidate_id(3),
            },
            complete_create(1),
            BackendWindowCandidateIntent::Close {
                surface_id: surface_id(4),
                candidate_id: candidate_id(4),
            },
        ];

        let report = WindowAdmissionPreviewPlanner::new().preview_intents(&intents);
        let ids: Vec<_> = report
            .actions
            .iter()
            .map(BackendWindowAdmissionPreviewAction::candidate_id)
            .collect();

        assert_eq!(ids, vec![candidate_id(3), candidate_id(1), candidate_id(4)]);
    }

    /// 验证 warning 按意图顺序和固定字段顺序生成。
    #[test]
    fn warnings_have_stable_intent_and_field_order() {
        let intents = vec![
            BackendWindowCandidateIntent::Create {
                surface_id: surface_id(1),
                candidate_id: candidate_id(1),
                title: None,
                app_id: Some("app.1".to_string()),
                size: None,
            },
            BackendWindowCandidateIntent::Create {
                surface_id: surface_id(2),
                candidate_id: candidate_id(2),
                title: Some("窗口 2".to_string()),
                app_id: None,
                size: Some(size(640, 480)),
            },
        ];

        let report = WindowAdmissionPreviewPlanner::new().preview_intents(&intents);

        assert_eq!(
            report.warnings,
            vec![
                BackendWindowAdmissionPreviewWarning::MissingTitle {
                    candidate_id: candidate_id(1)
                },
                BackendWindowAdmissionPreviewWarning::MissingSize {
                    candidate_id: candidate_id(1)
                },
                BackendWindowAdmissionPreviewWarning::MissingAppId {
                    candidate_id: candidate_id(2)
                },
            ]
        );
    }

    /// 验证失败来源下标原样进入预检报告。
    #[test]
    fn failed_report_preserves_source_failed_at() {
        let report =
            WindowAdmissionPreviewPlanner::new().preview_intent_report(&failed_intent_report());

        assert_eq!(report.source_failed_at, Some(3));
    }

    /// 验证失败来源错误原样进入预检报告。
    #[test]
    fn failed_report_preserves_source_error() {
        let report =
            WindowAdmissionPreviewPlanner::new().preview_intent_report(&failed_intent_report());

        assert_eq!(
            report.source_error,
            Some(BackendSurfaceLifecycleError::AlreadyDestroyed { id: surface_id(8) })
        );
    }

    /// 验证失败来源下标生成结构化 warning。
    #[test]
    fn failed_report_builds_source_failed_warning() {
        let report =
            WindowAdmissionPreviewPlanner::new().preview_intent_report(&failed_intent_report());

        assert!(
            report
                .warnings
                .contains(&BackendWindowAdmissionPreviewWarning::SourceTraceFailed {
                    failed_at: 3
                })
        );
    }

    /// 验证失败来源错误生成结构化 warning。
    #[test]
    fn failed_report_builds_source_error_warning() {
        let error = BackendSurfaceLifecycleError::AlreadyDestroyed { id: surface_id(8) };
        let report =
            WindowAdmissionPreviewPlanner::new().preview_intent_report(&failed_intent_report());

        assert!(
            report
                .warnings
                .contains(&BackendWindowAdmissionPreviewWarning::SourceTraceError { error })
        );
    }

    /// 验证来源失败时仍预检失败点之前已有的候选意图。
    #[test]
    fn failed_report_still_builds_actions() {
        let report =
            WindowAdmissionPreviewPlanner::new().preview_intent_report(&failed_intent_report());

        assert_eq!(report.actions.len(), 1);
        assert!(matches!(
            report.actions[0],
            BackendWindowAdmissionPreviewAction::WouldCreateWindow { .. }
        ));
    }

    /// 验证空意图输入生成空预检报告。
    #[test]
    fn empty_intents_build_empty_report() {
        let report = WindowAdmissionPreviewPlanner::new().preview_intents(&[]);

        assert_eq!(
            report,
            BackendWindowAdmissionPreviewReport {
                actions: Vec::new(),
                warnings: Vec::new(),
                source_failed_at: None,
                source_error: None,
            }
        );
    }

    /// 验证无失败信息的预检报告明确标记来源成功。
    #[test]
    fn successful_preview_reports_source_success() {
        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&complete_create(1));

        assert!(report.source_succeeded());
    }

    /// 验证任一来源失败信息都会阻止来源成功标记。
    #[test]
    fn partial_source_failure_is_not_reported_as_success() {
        let intent_report = BackendWindowCandidateIntentReport {
            intents: Vec::new(),
            source_failed_at: Some(0),
            source_error: None,
        };
        let report = WindowAdmissionPreviewPlanner::new().preview_intent_report(&intent_report);

        assert!(!report.source_succeeded());
    }

    /// 验证 warning 文本保留候选标识上下文。
    #[test]
    fn warning_display_preserves_context() {
        let warning = BackendWindowAdmissionPreviewWarning::MissingTitle {
            candidate_id: candidate_id(12),
        };

        assert_eq!(warning.to_string(), "候选窗口 12 缺少标题");
    }

    /// 验证动作访问器返回对应候选和 surface 标识。
    #[test]
    fn action_accessors_preserve_ids() {
        let action = BackendWindowAdmissionPreviewAction::WouldHideWindow {
            candidate_id: candidate_id(9),
            surface_id: surface_id(9),
        };

        assert_eq!(action.candidate_id(), candidate_id(9));
        assert_eq!(action.surface_id(), surface_id(9));
    }

    /// 验证生产代码不依赖核心模块。
    #[test]
    fn preview_production_code_does_not_depend_on_core() {
        assert!(!production_source().contains("crate::core"));
    }

    /// 验证生产代码不依赖通用后端模块。
    #[test]
    fn preview_production_code_does_not_depend_on_backend() {
        assert!(!production_source().contains("crate::backend"));
    }

    /// 验证生产代码不依赖 Smithay crate。
    #[test]
    fn preview_production_code_does_not_depend_on_smithay() {
        assert!(!production_source().contains("smithay::"));
    }

    /// 验证生产代码不依赖 Linux 或 Unix API。
    #[test]
    fn preview_production_code_does_not_depend_on_linux_api() {
        let source = production_source();

        for forbidden in ["std::os::unix", "libc::", "UnixStream", "XDG_RUNTIME_DIR"] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证生产代码不构造或提交核心事件路径类型。
    #[test]
    fn preview_production_code_does_not_use_event_submission_path() {
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
    fn preview_production_code_does_not_use_core_placement_types() {
        let source = production_source();

        for forbidden in ["Workspace", "Slot", "WindowId"] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证核心和通用后端源码没有反向依赖预检类型。
    #[test]
    fn core_and_backend_do_not_depend_on_preview_types() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        for directory in ["src/core", "src/backend"] {
            let path = manifest_dir.join(directory);
            assert!(!rust_tree_contains(&path, "window_admission_preview"));
            assert!(!rust_tree_contains(&path, "BackendWindowAdmissionPreview"));
            assert!(!rust_tree_contains(&path, "WindowAdmissionPreviewPlanner"));
        }
    }

    /// 验证生产逻辑不使用 panic、unwrap 或 expect。
    #[test]
    fn preview_production_code_has_no_panic_shortcuts() {
        let source = production_source();

        for forbidden in ["panic!", ".unwrap(", ".expect("] {
            assert!(!source.contains(forbidden));
        }
    }

    /// 验证报告和诊断保持可克隆、可直接比较。
    #[test]
    fn preview_report_is_cloneable_and_comparable() {
        let report = WindowAdmissionPreviewPlanner::new().preview_intent(&complete_create(1));

        assert_eq!(report.clone(), report);
    }
}
