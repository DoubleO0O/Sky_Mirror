//! Surface 到窗口候选意图的纯数据规划层。
//!
//! 本模块只把 surface 最终记录规划为后端中立的窗口候选意图。它不会构造
//! `BackendEvent` 或核心命令，不会调用 driver，也不会修改任何核心状态。
//! 这些意图只是未来接入边界的候选数据，不表示窗口已经进入 compositor。

use crate::smithay_backend::{
    surface_lifecycle::{
        BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleState,
        BackendSurfaceRecord, BackendSurfaceSize,
    },
    surface_trace::BackendSurfaceTraceReport,
};

/// 从 surface 稳定派生的后端窗口候选标识。
///
/// 当前使用相同数值建立一对一映射，但类型保持独立，避免把 surface ID
/// 错当成真实核心窗口 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendWindowCandidateId(u64);

impl BackendWindowCandidateId {
    /// 从 surface ID 稳定派生候选窗口 ID。
    pub const fn from_surface(surface_id: BackendSurfaceId) -> Self {
        Self(surface_id.value())
    }

    /// 返回候选标识的原始数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Surface 最终状态对应的窗口候选意图。
///
/// 所有变体都是纯数据，不包含核心窗口类型、真实 surface 或系统资源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendWindowCandidateIntent {
    /// 为已映射 surface 创建候选窗口。
    Create {
        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 可选窗口标题。
        title: Option<String>,

        /// 可选应用标识。
        app_id: Option<String>,

        /// 最近一次已知尺寸。
        size: Option<BackendSurfaceSize>,
    },

    /// 更新候选窗口的标题和应用标识。
    ///
    /// 当前最终状态规划不会单独生成该变体；它为未来增量规划保留明确的数据形状。
    UpdateMetadata {
        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 可选窗口标题。
        title: Option<String>,

        /// 可选应用标识。
        app_id: Option<String>,
    },

    /// 更新已配置 surface 对应候选窗口的尺寸。
    UpdateSize {
        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,

        /// 已验证的 surface 尺寸。
        size: BackendSurfaceSize,
    },

    /// 隐藏已取消映射的候选窗口。
    Hide {
        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,
    },

    /// 关闭已销毁 surface 对应的候选窗口。
    Close {
        /// 来源 surface。
        surface_id: BackendSurfaceId,

        /// 稳定候选窗口标识。
        candidate_id: BackendWindowCandidateId,
    },
}

impl BackendWindowCandidateIntent {
    /// 返回意图对应的 surface ID。
    pub const fn surface_id(&self) -> BackendSurfaceId {
        match self {
            Self::Create { surface_id, .. }
            | Self::UpdateMetadata { surface_id, .. }
            | Self::UpdateSize { surface_id, .. }
            | Self::Hide { surface_id, .. }
            | Self::Close { surface_id, .. } => *surface_id,
        }
    }

    /// 返回意图对应的候选窗口 ID。
    pub const fn candidate_id(&self) -> BackendWindowCandidateId {
        match self {
            Self::Create { candidate_id, .. }
            | Self::UpdateMetadata { candidate_id, .. }
            | Self::UpdateSize { candidate_id, .. }
            | Self::Hide { candidate_id, .. }
            | Self::Close { candidate_id, .. } => *candidate_id,
        }
    }
}

/// 从 surface trace 最终状态生成的窗口候选意图报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendWindowCandidateIntentReport {
    /// 按 surface ID 稳定排序的候选意图。
    pub intents: Vec<BackendWindowCandidateIntent>,

    /// 来源 trace 首个失败事件的下标。
    pub source_failed_at: Option<usize>,

    /// 来源 trace 的结构化失败原因。
    pub source_error: Option<BackendSurfaceLifecycleError>,
}

impl BackendWindowCandidateIntentReport {
    /// 判断来源 trace 是否完整成功。
    pub fn source_succeeded(&self) -> bool {
        self.source_failed_at.is_none() && self.source_error.is_none()
    }
}

/// Surface 最终记录到窗口候选意图的无状态规划器。
///
/// Planner 只读取纯数据快照，不重放 trace，不调用核心模块，也不提交任何状态变化。
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceWindowIntentPlanner;

impl SurfaceWindowIntentPlanner {
    /// 创建无状态候选意图规划器。
    pub const fn new() -> Self {
        Self
    }

    /// 为 surface 稳定派生候选窗口 ID。
    pub const fn candidate_id_for_surface(
        &self,
        surface_id: BackendSurfaceId,
    ) -> BackendWindowCandidateId {
        BackendWindowCandidateId::from_surface(surface_id)
    }

    /// 根据单个 surface 最终记录规划当前状态意图。
    ///
    /// `Created` 和没有尺寸的 `Configured` 不产生意图，其余状态最多产生一个意图。
    pub fn intents_for_record(
        &self,
        record: &BackendSurfaceRecord,
    ) -> Vec<BackendWindowCandidateIntent> {
        let surface_id = record.id;
        let candidate_id = self.candidate_id_for_surface(surface_id);

        let intent = match record.state {
            BackendSurfaceLifecycleState::Created => None,
            BackendSurfaceLifecycleState::Configured => {
                record
                    .last_known_size
                    .map(|size| BackendWindowCandidateIntent::UpdateSize {
                        surface_id,
                        candidate_id,
                        size,
                    })
            }
            BackendSurfaceLifecycleState::Mapped => Some(BackendWindowCandidateIntent::Create {
                surface_id,
                candidate_id,
                title: record.title.clone(),
                app_id: record.app_id.clone(),
                size: record.last_known_size,
            }),
            BackendSurfaceLifecycleState::Unmapped => Some(BackendWindowCandidateIntent::Hide {
                surface_id,
                candidate_id,
            }),
            BackendSurfaceLifecycleState::Destroyed => Some(BackendWindowCandidateIntent::Close {
                surface_id,
                candidate_id,
            }),
        };

        intent.into_iter().collect()
    }

    /// 根据多个 surface 最终记录生成稳定顺序的意图。
    ///
    /// 输入记录不会被修改；规划器只排序借用并克隆意图所需的 metadata。
    pub fn intents_for_records(
        &self,
        records: &[BackendSurfaceRecord],
    ) -> Vec<BackendWindowCandidateIntent> {
        let mut ordered_records: Vec<_> = records.iter().collect();
        ordered_records.sort_by_key(|record| record.id);

        ordered_records
            .into_iter()
            .flat_map(|record| self.intents_for_record(record))
            .collect()
    }

    /// 根据 trace 最终快照生成候选意图报告。
    ///
    /// 本方法不会重放事件。即使来源 trace 失败，也会规划失败点之前
    /// `final_records` 的当前意图，并原样保留失败下标与错误。
    pub fn intents_for_trace_report(
        &self,
        report: &BackendSurfaceTraceReport,
    ) -> BackendWindowCandidateIntentReport {
        BackendWindowCandidateIntentReport {
            intents: self.intents_for_records(&report.final_records),
            source_failed_at: report.failed_at,
            source_error: report.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        BackendWindowCandidateId, BackendWindowCandidateIntent, BackendWindowCandidateIntentReport,
        SurfaceWindowIntentPlanner,
    };
    use crate::smithay_backend::{
        surface_lifecycle::{
            BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleState,
            BackendSurfaceRecord, BackendSurfaceRegistry, BackendSurfaceSize,
        },
        surface_trace::{BackendSurfaceMockAdapter, BackendSurfaceTraceScenario},
    };

    /// 创建指定状态的测试记录。
    fn record(id: u64, state: BackendSurfaceLifecycleState) -> BackendSurfaceRecord {
        BackendSurfaceRecord {
            id: BackendSurfaceId::new(id),
            state,
            title: None,
            app_id: None,
            last_known_size: None,
        }
    }

    /// 创建有效测试尺寸。
    fn size(width: u32, height: u32) -> BackendSurfaceSize {
        BackendSurfaceSize::new(width, height).expect("测试尺寸必须有效")
    }

    /// 运行 mock trace 并返回 trace report。
    fn trace_report(
        scenario: BackendSurfaceTraceScenario,
    ) -> crate::smithay_backend::surface_trace::BackendSurfaceTraceReport {
        let mut adapter = BackendSurfaceMockAdapter::new();
        let trace = scenario.build(&mut adapter);
        let mut registry = BackendSurfaceRegistry::new();

        trace.run(&mut registry)
    }

    /// 验证候选 ID 可以从 surface ID 稳定派生。
    #[test]
    fn candidate_id_is_stably_derived_from_surface() {
        let planner = SurfaceWindowIntentPlanner::new();
        let surface_id = BackendSurfaceId::new(42);

        assert_eq!(
            planner.candidate_id_for_surface(surface_id),
            BackendWindowCandidateId::from_surface(surface_id)
        );
        assert_eq!(
            planner.candidate_id_for_surface(surface_id).value(),
            surface_id.value()
        );
    }

    /// 验证同一 surface 多次规划得到相同候选 ID。
    #[test]
    fn same_surface_always_has_same_candidate_id() {
        let planner = SurfaceWindowIntentPlanner::new();
        let surface_id = BackendSurfaceId::new(7);

        assert_eq!(
            planner.candidate_id_for_surface(surface_id),
            planner.candidate_id_for_surface(surface_id)
        );
    }

    /// 验证不同 surface 产生不同候选 ID。
    #[test]
    fn different_surfaces_have_different_candidate_ids() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert_ne!(
            planner.candidate_id_for_surface(BackendSurfaceId::new(1)),
            planner.candidate_id_for_surface(BackendSurfaceId::new(2))
        );
    }

    /// 验证候选 ID 可以按派生 surface ID 稳定排序。
    #[test]
    fn candidate_ids_are_stably_ordered() {
        let mut ids = [
            BackendWindowCandidateId::from_surface(BackendSurfaceId::new(30)),
            BackendWindowCandidateId::from_surface(BackendSurfaceId::new(10)),
            BackendWindowCandidateId::from_surface(BackendSurfaceId::new(20)),
        ];

        ids.sort();

        assert_eq!(ids.map(BackendWindowCandidateId::value), [10, 20, 30]);
    }

    /// 验证 Created 最终状态不生成窗口候选意图。
    #[test]
    fn created_record_produces_no_intent() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert!(
            planner
                .intents_for_record(&record(1, BackendSurfaceLifecycleState::Created))
                .is_empty()
        );
    }

    /// 验证 Configured 有尺寸时生成 UpdateSize。
    #[test]
    fn configured_record_with_size_produces_update_size() {
        let planner = SurfaceWindowIntentPlanner::new();
        let mut configured = record(1, BackendSurfaceLifecycleState::Configured);
        configured.last_known_size = Some(size(800, 600));

        assert_eq!(
            planner.intents_for_record(&configured),
            vec![BackendWindowCandidateIntent::UpdateSize {
                surface_id: BackendSurfaceId::new(1),
                candidate_id: BackendWindowCandidateId::from_surface(BackendSurfaceId::new(1)),
                size: size(800, 600),
            }]
        );
    }

    /// 验证 Configured 没有尺寸时不生成意图。
    #[test]
    fn configured_record_without_size_produces_no_intent() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert!(
            planner
                .intents_for_record(&record(1, BackendSurfaceLifecycleState::Configured))
                .is_empty()
        );
    }

    /// 验证 Mapped 最终状态生成 Create。
    #[test]
    fn mapped_record_produces_create_intent() {
        let planner = SurfaceWindowIntentPlanner::new();
        let intents = planner.intents_for_record(&record(1, BackendSurfaceLifecycleState::Mapped));

        assert!(matches!(
            intents.as_slice(),
            [BackendWindowCandidateIntent::Create {
                surface_id,
                candidate_id,
                ..
            }] if *surface_id == BackendSurfaceId::new(1)
                && *candidate_id
                    == BackendWindowCandidateId::from_surface(BackendSurfaceId::new(1))
        ));
    }

    /// 验证 Mapped Create 携带标题。
    #[test]
    fn mapped_record_preserves_title() {
        let planner = SurfaceWindowIntentPlanner::new();
        let mut mapped = record(1, BackendSurfaceLifecycleState::Mapped);
        mapped.title = Some("候选窗口".to_string());

        assert!(matches!(
            planner.intents_for_record(&mapped).as_slice(),
            [BackendWindowCandidateIntent::Create {
                title: Some(title),
                ..
            }] if title == "候选窗口"
        ));
    }

    /// 验证 Mapped Create 携带应用标识。
    #[test]
    fn mapped_record_preserves_app_id() {
        let planner = SurfaceWindowIntentPlanner::new();
        let mut mapped = record(1, BackendSurfaceLifecycleState::Mapped);
        mapped.app_id = Some("sky-mirror.intent".to_string());

        assert!(matches!(
            planner.intents_for_record(&mapped).as_slice(),
            [BackendWindowCandidateIntent::Create {
                app_id: Some(app_id),
                ..
            }] if app_id == "sky-mirror.intent"
        ));
    }

    /// 验证 Mapped Create 携带最近尺寸。
    #[test]
    fn mapped_record_preserves_size() {
        let planner = SurfaceWindowIntentPlanner::new();
        let mut mapped = record(1, BackendSurfaceLifecycleState::Mapped);
        mapped.last_known_size = Some(size(1280, 720));

        assert!(matches!(
            planner.intents_for_record(&mapped).as_slice(),
            [BackendWindowCandidateIntent::Create {
                size: Some(intent_size),
                ..
            }] if *intent_size == size(1280, 720)
        ));
    }

    /// 验证 Mapped 缺少标题和应用标识仍可生成 Create。
    #[test]
    fn mapped_record_allows_missing_metadata() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert!(matches!(
            planner
                .intents_for_record(&record(1, BackendSurfaceLifecycleState::Mapped))
                .as_slice(),
            [BackendWindowCandidateIntent::Create {
                title: None,
                app_id: None,
                ..
            }]
        ));
    }

    /// 验证 Mapped 缺少尺寸仍可生成 Create。
    #[test]
    fn mapped_record_allows_missing_size() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert!(matches!(
            planner
                .intents_for_record(&record(1, BackendSurfaceLifecycleState::Mapped))
                .as_slice(),
            [BackendWindowCandidateIntent::Create { size: None, .. }]
        ));
    }

    /// 验证 Unmapped 最终状态生成 Hide。
    #[test]
    fn unmapped_record_produces_hide_intent() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert!(matches!(
            planner
                .intents_for_record(&record(1, BackendSurfaceLifecycleState::Unmapped))
                .as_slice(),
            [BackendWindowCandidateIntent::Hide { .. }]
        ));
    }

    /// 验证 Destroyed 最终状态生成 Close。
    #[test]
    fn destroyed_record_produces_close_intent() {
        let planner = SurfaceWindowIntentPlanner::new();

        assert!(matches!(
            planner
                .intents_for_record(&record(1, BackendSurfaceLifecycleState::Destroyed))
                .as_slice(),
            [BackendWindowCandidateIntent::Close { .. }]
        ));
    }

    /// 验证多记录意图按 surface ID 稳定排序。
    #[test]
    fn multiple_records_produce_stably_ordered_intents() {
        let planner = SurfaceWindowIntentPlanner::new();
        let records = vec![
            record(30, BackendSurfaceLifecycleState::Destroyed),
            record(10, BackendSurfaceLifecycleState::Mapped),
            record(20, BackendSurfaceLifecycleState::Unmapped),
        ];
        let ids: Vec<_> = planner
            .intents_for_records(&records)
            .iter()
            .map(BackendWindowCandidateIntent::surface_id)
            .collect();

        assert_eq!(
            ids,
            vec![
                BackendSurfaceId::new(10),
                BackendSurfaceId::new(20),
                BackendSurfaceId::new(30),
            ]
        );
    }

    /// 验证多记录规划不会修改原始记录及其顺序。
    #[test]
    fn planning_records_does_not_modify_source_records() {
        let planner = SurfaceWindowIntentPlanner::new();
        let records = vec![
            record(2, BackendSurfaceLifecycleState::Destroyed),
            record(1, BackendSurfaceLifecycleState::Mapped),
        ];
        let original = records.clone();

        let _intents = planner.intents_for_records(&records);

        assert_eq!(records, original);
    }

    /// 验证成功 trace report 可生成最终状态意图。
    #[test]
    fn successful_trace_report_produces_intents() {
        let planner = SurfaceWindowIntentPlanner::new();
        let trace_report = trace_report(BackendSurfaceTraceScenario::SingleMap);

        let intent_report = planner.intents_for_trace_report(&trace_report);

        assert!(intent_report.source_succeeded());
        assert_eq!(intent_report.source_failed_at, None);
        assert_eq!(intent_report.source_error, None);
        assert!(matches!(
            intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Create { .. }]
        ));
    }

    /// 验证失败 trace report 保留失败事件下标。
    #[test]
    fn failed_trace_report_preserves_failed_at() {
        let planner = SurfaceWindowIntentPlanner::new();
        let trace_report = trace_report(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        let intent_report = planner.intents_for_trace_report(&trace_report);

        assert_eq!(intent_report.source_failed_at, Some(2));
    }

    /// 验证失败 trace report 保留结构化错误。
    #[test]
    fn failed_trace_report_preserves_source_error() {
        let planner = SurfaceWindowIntentPlanner::new();
        let trace_report = trace_report(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        let intent_report = planner.intents_for_trace_report(&trace_report);

        assert_eq!(
            intent_report.source_error,
            Some(BackendSurfaceLifecycleError::AlreadyDestroyed {
                id: BackendSurfaceId::new(1)
            })
        );
    }

    /// 验证失败 trace 仍根据 final_records 生成 Close。
    #[test]
    fn failed_trace_report_still_plans_final_records() {
        let planner = SurfaceWindowIntentPlanner::new();
        let trace_report = trace_report(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        let intent_report = planner.intents_for_trace_report(&trace_report);

        assert!(matches!(
            intent_report.intents.as_slice(),
            [BackendWindowCandidateIntent::Close { surface_id, .. }]
                if *surface_id == BackendSurfaceId::new(1)
        ));
    }

    /// 验证失败 trace 的意图报告不会假装来源成功。
    #[test]
    fn invalid_destroyed_remap_intent_report_is_not_successful() {
        let planner = SurfaceWindowIntentPlanner::new();
        let trace_report = trace_report(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        let intent_report = planner.intents_for_trace_report(&trace_report);

        assert!(!intent_report.source_succeeded());
        assert!(intent_report.source_failed_at.is_some());
        assert!(intent_report.source_error.is_some());
    }

    /// 验证意图辅助方法返回稳定的 surface 和候选 ID。
    #[test]
    fn intent_accessors_preserve_ids() {
        let surface_id = BackendSurfaceId::new(9);
        let candidate_id = BackendWindowCandidateId::from_surface(surface_id);
        let intent = BackendWindowCandidateIntent::UpdateMetadata {
            surface_id,
            candidate_id,
            title: None,
            app_id: None,
        };

        assert_eq!(intent.surface_id(), surface_id);
        assert_eq!(intent.candidate_id(), candidate_id);
    }

    /// 验证报告可以直接构造和比较。
    #[test]
    fn intent_report_is_pure_comparable_data() {
        let report = BackendWindowCandidateIntentReport {
            intents: Vec::new(),
            source_failed_at: None,
            source_error: None,
        };

        assert_eq!(report.clone(), report);
    }

    /// 验证 planner 生产代码不依赖核心模块。
    #[test]
    fn planner_has_no_core_dependency() {
        assert!(!production_source().contains("crate::core"));
    }

    /// 验证 planner 生产代码不依赖 backend 抽象层。
    #[test]
    fn planner_has_no_backend_dependency() {
        assert!(!production_source().contains("crate::backend"));
    }

    /// 验证 planner 生产代码不依赖 Smithay crate。
    #[test]
    fn planner_has_no_smithay_dependency() {
        let source = production_source();

        assert!(!source.contains("smithay::"));
        assert!(!source.contains("use smithay"));
    }

    /// 验证 planner 生产代码不依赖 Linux 或真实 Wayland API。
    #[test]
    fn planner_has_no_linux_or_wayland_dependency() {
        let source = production_source();

        for forbidden in [
            "target_os",
            "std::os",
            "UnixStream",
            "libc::",
            "nix::",
            "wl_surface",
            "xdg_toplevel",
        ] {
            assert!(
                !source.contains(forbidden),
                "生产代码不应包含 Linux 或 Wayland API: {forbidden}"
            );
        }
    }

    /// 验证 core 和 backend 不反向依赖窗口候选意图类型。
    #[test]
    fn core_and_backend_do_not_depend_on_window_intents() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        for relative_dir in ["src/core", "src/backend"] {
            assert_directory_has_no_intent_dependency(&manifest_dir.join(relative_dir));
        }
    }

    /// 返回测试模块之前的生产代码。
    fn production_source() -> &'static str {
        include_str!("surface_window_intent.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("生产代码片段应存在")
    }

    /// 递归检查 Rust 源文件中的候选意图反向依赖。
    fn assert_directory_has_no_intent_dependency(directory: &Path) {
        for entry in fs::read_dir(directory).expect("源码目录应可读取") {
            let path = entry.expect("源码目录项应可读取").path();

            if path.is_dir() {
                assert_directory_has_no_intent_dependency(&path);
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }

            let source = fs::read_to_string(&path).expect("Rust 源文件应可读取");
            let forbidden_references = [
                "smithay_backend::surface_window_intent",
                "BackendWindowCandidateId",
                "BackendWindowCandidateIntent",
                "BackendWindowCandidateIntentReport",
                "SurfaceWindowIntentPlanner",
            ];
            assert!(
                forbidden_references
                    .iter()
                    .all(|reference| !source.contains(reference)),
                "{} 不应依赖 smithay_backend surface window intent",
                path.display()
            );
        }
    }
}
