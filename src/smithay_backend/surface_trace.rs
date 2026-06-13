//! Surface 生命周期事件轨迹与 mock adapter。
//!
//! 这是可在 macOS 验证的纯 Rust 测试辅助层。它只生成并回放
//! `BackendSurfaceLifecycleEvent`，不保存真实 surface，不依赖系统后端，也不会
//! 把事件提交到核心状态。未来平台 adapter 应复用同一事件形状，而不是复制
//! `BackendSurfaceRegistry` 的状态转换规则。
//!
//! Contract: trace 中的事件顺序是测试输入，不声明 Wayland request、configure
//! 或 commit 的真实协议顺序已经得到验证。

use crate::smithay_backend::surface_lifecycle::{
    BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleEvent,
    BackendSurfaceRecord, BackendSurfaceRegistry, BackendSurfaceSize,
};

/// 一组按顺序应用的 surface 生命周期纯数据事件。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackendSurfaceTrace {
    events: Vec<BackendSurfaceLifecycleEvent>,
}

impl BackendSurfaceTrace {
    /// 创建空事件轨迹。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从事件迭代器创建轨迹。
    pub fn from_events(events: impl IntoIterator<Item = BackendSurfaceLifecycleEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }

    /// 追加一条生命周期事件。
    pub fn push(&mut self, event: BackendSurfaceLifecycleEvent) {
        self.events.push(event);
    }

    /// 追加多条生命周期事件。
    pub fn extend(&mut self, events: impl IntoIterator<Item = BackendSurfaceLifecycleEvent>) {
        self.events.extend(events);
    }

    /// 只读访问轨迹中的事件。
    pub fn events(&self) -> &[BackendSurfaceLifecycleEvent] {
        &self.events
    }

    /// 返回事件数量。
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 判断轨迹是否为空。
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 消耗轨迹并返回内部事件。
    pub fn into_events(self) -> Vec<BackendSurfaceLifecycleEvent> {
        self.events
    }

    /// 通过统一 runner 将轨迹应用到生命周期注册表。
    pub fn run(&self, registry: &mut BackendSurfaceRegistry) -> BackendSurfaceTraceReport {
        BackendSurfaceTraceRunner::run(self, registry)
    }
}

/// Surface 事件轨迹执行报告。
///
/// `applied` 只统计成功事件。出现错误时，`failed_at` 是从零开始的事件下标，
/// runner 会立即停止，并保留失败前已经产生的注册表状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSurfaceTraceReport {
    /// 成功应用的事件数量。
    pub applied: usize,

    /// 首个失败事件的下标。
    pub failed_at: Option<usize>,

    /// 首个失败事件产生的结构化错误。
    pub error: Option<BackendSurfaceLifecycleError>,

    /// 执行结束时按 surface ID 稳定排序的记录快照。
    pub final_records: Vec<BackendSurfaceRecord>,
}

impl BackendSurfaceTraceReport {
    /// 判断轨迹是否完整执行成功。
    pub fn is_success(&self) -> bool {
        self.failed_at.is_none() && self.error.is_none()
    }
}

/// Surface 生命周期事件轨迹 runner。
///
/// Runner 不实现任何状态转换，只按顺序调用
/// `BackendSurfaceRegistry::apply_event`，确保 mock 场景和未来 adapter 共用同一套
/// 纯数据规则。首个错误后的事件不会执行。
pub struct BackendSurfaceTraceRunner;

impl BackendSurfaceTraceRunner {
    /// 执行轨迹，遇到首个错误后停止。
    pub fn run(
        trace: &BackendSurfaceTrace,
        registry: &mut BackendSurfaceRegistry,
    ) -> BackendSurfaceTraceReport {
        let mut applied = 0;

        for (index, event) in trace.events().iter().cloned().enumerate() {
            if let Err(error) = registry.apply_event(event) {
                return BackendSurfaceTraceReport {
                    applied,
                    failed_at: Some(index),
                    error: Some(error),
                    final_records: snapshot_records(registry),
                };
            }

            applied += 1;
        }

        BackendSurfaceTraceReport {
            applied,
            failed_at: None,
            error: None,
            final_records: snapshot_records(registry),
        }
    }
}

/// 可重复构造的 surface mock 场景。
///
/// 每个变体只描述纯数据事件序列，不代表真实协议时序已经验证。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSurfaceTraceScenario {
    /// 创建后直接映射一个 surface。
    SingleMap,

    /// 创建、配置尺寸后映射一个 surface。
    ConfigureThenMap,

    /// 映射、撤销映射后再次映射。
    MapUnmapRemap,

    /// 创建后销毁并保留 tombstone。
    Destroy,

    /// 销毁后尝试再次映射，用于验证失败报告。
    InvalidDestroyedRemap,

    /// 构造多个 surface 和不同最终状态。
    MultiSurface,
}

impl BackendSurfaceTraceScenario {
    /// 使用 mock adapter 为当前场景生成稳定事件轨迹。
    pub fn build(self, adapter: &mut BackendSurfaceMockAdapter) -> BackendSurfaceTrace {
        adapter.trace_for(self)
    }
}

/// 只生成生命周期事件的纯数据 mock adapter。
///
/// Adapter 不持有注册表，也不直接改变状态。ID 从 1 开始稳定递增，便于测试比较。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSurfaceMockAdapter {
    next_id: u64,
}

impl BackendSurfaceMockAdapter {
    /// 创建从 surface ID 1 开始的 mock adapter。
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    /// 使用指定起始 ID 创建 mock adapter。
    pub fn with_next_id(next_id: BackendSurfaceId) -> Self {
        Self {
            next_id: next_id.value(),
        }
    }

    /// 返回下一次场景分配的 surface ID。
    pub const fn peek_next_id(&self) -> BackendSurfaceId {
        BackendSurfaceId::new(self.next_id)
    }

    /// 生成创建后直接映射的单 surface 轨迹。
    pub fn single_surface_map_trace(&mut self) -> BackendSurfaceTrace {
        let id = self.allocate_id();

        BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("Mock Surface".to_string()),
                app_id: Some("sky-mirror.mock".to_string()),
            },
        ])
    }

    /// 生成创建、配置、映射的单 surface 轨迹。
    pub fn single_surface_configure_then_map_trace(&mut self) -> BackendSurfaceTrace {
        let id = self.allocate_id();

        BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Configured {
                id,
                size: Some(BackendSurfaceSize {
                    width: 1280,
                    height: 720,
                }),
            },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("Configured Mock Surface".to_string()),
                app_id: Some("sky-mirror.mock.configured".to_string()),
            },
        ])
    }

    /// 生成 map、unmap、remap 轨迹。
    pub fn map_unmap_remap_trace(&mut self) -> BackendSurfaceTrace {
        let id = self.allocate_id();

        BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("Initial Mock Surface".to_string()),
                app_id: None,
            },
            BackendSurfaceLifecycleEvent::Unmapped { id },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("Remapped Mock Surface".to_string()),
                app_id: None,
            },
        ])
    }

    /// 生成创建后销毁的轨迹。
    pub fn destroy_trace(&mut self) -> BackendSurfaceTrace {
        let id = self.allocate_id();

        BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Destroyed { id },
        ])
    }

    /// 生成销毁后再次映射的预期失败轨迹。
    pub fn invalid_destroyed_remap_trace(&mut self) -> BackendSurfaceTrace {
        let id = self.allocate_id();

        BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Destroyed { id },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("Invalid Remap".to_string()),
                app_id: None,
            },
        ])
    }

    /// 生成包含 mapped、unmapped 和 destroyed 记录的多 surface 轨迹。
    pub fn multi_surface_trace(&mut self) -> BackendSurfaceTrace {
        let first = self.allocate_id();
        let second = self.allocate_id();
        let third = self.allocate_id();

        BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id: third },
            BackendSurfaceLifecycleEvent::Destroyed { id: third },
            BackendSurfaceLifecycleEvent::Created { id: first },
            BackendSurfaceLifecycleEvent::Configured {
                id: first,
                size: Some(BackendSurfaceSize {
                    width: 1920,
                    height: 1080,
                }),
            },
            BackendSurfaceLifecycleEvent::Mapped {
                id: first,
                title: Some("Primary Mock Surface".to_string()),
                app_id: Some("sky-mirror.mock.primary".to_string()),
            },
            BackendSurfaceLifecycleEvent::Created { id: second },
            BackendSurfaceLifecycleEvent::Mapped {
                id: second,
                title: Some("Secondary Mock Surface".to_string()),
                app_id: Some("sky-mirror.mock.secondary".to_string()),
            },
            BackendSurfaceLifecycleEvent::Unmapped { id: second },
        ])
    }

    /// 根据场景枚举生成对应轨迹。
    pub fn trace_for(&mut self, scenario: BackendSurfaceTraceScenario) -> BackendSurfaceTrace {
        match scenario {
            BackendSurfaceTraceScenario::SingleMap => self.single_surface_map_trace(),
            BackendSurfaceTraceScenario::ConfigureThenMap => {
                self.single_surface_configure_then_map_trace()
            }
            BackendSurfaceTraceScenario::MapUnmapRemap => self.map_unmap_remap_trace(),
            BackendSurfaceTraceScenario::Destroy => self.destroy_trace(),
            BackendSurfaceTraceScenario::InvalidDestroyedRemap => {
                self.invalid_destroyed_remap_trace()
            }
            BackendSurfaceTraceScenario::MultiSurface => self.multi_surface_trace(),
        }
    }

    /// 分配稳定递增的后端 surface ID。
    fn allocate_id(&mut self) -> BackendSurfaceId {
        let id = BackendSurfaceId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}

impl Default for BackendSurfaceMockAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// 克隆注册表的稳定顺序快照。
fn snapshot_records(registry: &BackendSurfaceRegistry) -> Vec<BackendSurfaceRecord> {
    registry.list_surfaces().into_iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        BackendSurfaceMockAdapter, BackendSurfaceTrace, BackendSurfaceTraceRunner,
        BackendSurfaceTraceScenario,
    };
    use crate::smithay_backend::surface_lifecycle::{
        BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleEvent,
        BackendSurfaceLifecycleState, BackendSurfaceRegistry,
    };

    /// 运行测试场景并返回报告与注册表。
    fn run_scenario(
        scenario: BackendSurfaceTraceScenario,
    ) -> (super::BackendSurfaceTraceReport, BackendSurfaceRegistry) {
        let mut adapter = BackendSurfaceMockAdapter::new();
        let trace = scenario.build(&mut adapter);
        let mut registry = BackendSurfaceRegistry::new();
        let report = trace.run(&mut registry);

        (report, registry)
    }

    /// 验证空轨迹成功运行且不产生记录。
    #[test]
    fn empty_trace_runs_without_events() {
        let trace = BackendSurfaceTrace::new();
        let mut registry = BackendSurfaceRegistry::new();
        let report = BackendSurfaceTraceRunner::run(&trace, &mut registry);

        assert!(trace.is_empty());
        assert_eq!(trace.len(), 0);
        assert!(report.is_success());
        assert_eq!(report.applied, 0);
        assert_eq!(report.failed_at, None);
        assert_eq!(report.error, None);
        assert!(report.final_records.is_empty());
    }

    /// 验证 trace 的 push、extend 和只读事件访问保持顺序。
    #[test]
    fn trace_builders_preserve_event_order() {
        let first = BackendSurfaceId::new(1);
        let second = BackendSurfaceId::new(2);
        let mut trace = BackendSurfaceTrace::new();
        trace.push(BackendSurfaceLifecycleEvent::Created { id: first });
        trace.extend([
            BackendSurfaceLifecycleEvent::Destroyed { id: first },
            BackendSurfaceLifecycleEvent::Created { id: second },
        ]);

        assert_eq!(trace.len(), 3);
        assert_eq!(
            trace.events(),
            &[
                BackendSurfaceLifecycleEvent::Created { id: first },
                BackendSurfaceLifecycleEvent::Destroyed { id: first },
                BackendSurfaceLifecycleEvent::Created { id: second },
            ]
        );
    }

    /// 验证 into_events 返回可稳定比较的原始事件。
    #[test]
    fn trace_into_events_returns_owned_events() {
        let id = BackendSurfaceId::new(8);
        let trace = BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Destroyed { id },
        ]);

        assert_eq!(
            trace.into_events(),
            vec![
                BackendSurfaceLifecycleEvent::Created { id },
                BackendSurfaceLifecycleEvent::Destroyed { id },
            ]
        );
    }

    /// 验证单 surface Created -> Mapped 轨迹成功。
    #[test]
    fn single_surface_map_trace_succeeds() {
        let (report, registry) = run_scenario(BackendSurfaceTraceScenario::SingleMap);

        assert!(report.is_success());
        assert_eq!(report.applied, 2);
        assert_eq!(
            registry
                .get_surface(BackendSurfaceId::new(1))
                .expect("surface 应存在")
                .state,
            BackendSurfaceLifecycleState::Mapped
        );
    }

    /// 验证 Created -> Configured -> Mapped 轨迹记录尺寸。
    #[test]
    fn configure_then_map_trace_succeeds() {
        let (report, registry) = run_scenario(BackendSurfaceTraceScenario::ConfigureThenMap);
        let record = registry
            .get_surface(BackendSurfaceId::new(1))
            .expect("surface 应存在");

        assert!(report.is_success());
        assert_eq!(report.applied, 3);
        assert_eq!(record.state, BackendSurfaceLifecycleState::Mapped);
        assert_eq!(record.last_known_size.expect("尺寸应存在").width, 1280);
    }

    /// 验证 map、unmap、remap 轨迹最终重新映射。
    #[test]
    fn map_unmap_remap_trace_succeeds() {
        let (report, registry) = run_scenario(BackendSurfaceTraceScenario::MapUnmapRemap);
        let record = registry
            .get_surface(BackendSurfaceId::new(1))
            .expect("surface 应存在");

        assert!(report.is_success());
        assert_eq!(report.applied, 4);
        assert_eq!(record.state, BackendSurfaceLifecycleState::Mapped);
        assert_eq!(record.title.as_deref(), Some("Remapped Mock Surface"));
    }

    /// 验证 destroy 轨迹保留 tombstone。
    #[test]
    fn destroy_trace_keeps_tombstone() {
        let (report, registry) = run_scenario(BackendSurfaceTraceScenario::Destroy);

        assert!(report.is_success());
        assert_eq!(report.applied, 2);
        assert_eq!(
            registry
                .get_surface(BackendSurfaceId::new(1))
                .expect("tombstone 应存在")
                .state,
            BackendSurfaceLifecycleState::Destroyed
        );
    }

    /// 验证销毁后 remap 返回结构化失败。
    #[test]
    fn invalid_destroyed_remap_trace_fails() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert!(!report.is_success());
        assert_eq!(
            report.error,
            Some(BackendSurfaceLifecycleError::AlreadyDestroyed {
                id: BackendSurfaceId::new(1)
            })
        );
    }

    /// 验证失败下标指向从零开始的第三个事件。
    #[test]
    fn failed_trace_reports_correct_index() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert_eq!(report.failed_at, Some(2));
    }

    /// 验证失败报告只统计前两个成功事件。
    #[test]
    fn failed_trace_reports_applied_count() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert_eq!(report.applied, 2);
    }

    /// 验证失败后不会应用后续事件。
    #[test]
    fn failed_trace_stops_before_following_events() {
        let first = BackendSurfaceId::new(1);
        let trailing = BackendSurfaceId::new(2);
        let trace = BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id: first },
            BackendSurfaceLifecycleEvent::Destroyed { id: first },
            BackendSurfaceLifecycleEvent::Mapped {
                id: first,
                title: None,
                app_id: None,
            },
            BackendSurfaceLifecycleEvent::Created { id: trailing },
        ]);
        let mut registry = BackendSurfaceRegistry::new();

        let report = trace.run(&mut registry);

        assert_eq!(report.failed_at, Some(2));
        assert_eq!(report.applied, 2);
        assert!(registry.get_surface(trailing).is_none());
        assert_eq!(report.final_records.len(), 1);
        assert_eq!(report.final_records[0].id, first);
        assert_eq!(
            report.final_records[0].state,
            BackendSurfaceLifecycleState::Destroyed
        );
    }

    /// 验证多 surface 报告按 ID 稳定排序。
    #[test]
    fn multi_surface_trace_has_stable_record_order() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::MultiSurface);
        let ids: Vec<_> = report
            .final_records
            .iter()
            .map(|record| record.id.value())
            .collect();

        assert!(report.is_success());
        assert_eq!(ids, vec![1, 2, 3]);
    }

    /// 验证多 surface 场景最终只把一个记录保留为 Mapped。
    #[test]
    fn multi_surface_trace_filters_mapped_surfaces() {
        let (_, registry) = run_scenario(BackendSurfaceTraceScenario::MultiSurface);
        let mapped_ids: Vec<_> = registry
            .mapped_surfaces()
            .into_iter()
            .map(|record| record.id.value())
            .collect();

        assert_eq!(mapped_ids, vec![1]);
    }

    /// 验证 mock adapter 生成稳定递增的 ID。
    #[test]
    fn mock_adapter_generates_stable_ids() {
        let mut adapter = BackendSurfaceMockAdapter::new();
        let first = adapter.single_surface_map_trace();
        let second = adapter.destroy_trace();

        assert!(matches!(
            first.events().first(),
            Some(BackendSurfaceLifecycleEvent::Created { id })
                if *id == BackendSurfaceId::new(1)
        ));
        assert!(matches!(
            second.events().first(),
            Some(BackendSurfaceLifecycleEvent::Created { id })
                if *id == BackendSurfaceId::new(2)
        ));
        assert_eq!(adapter.peek_next_id(), BackendSurfaceId::new(3));
    }

    /// 验证 mock adapter 可以从指定 ID 开始。
    #[test]
    fn mock_adapter_supports_custom_start_id() {
        let mut adapter = BackendSurfaceMockAdapter::with_next_id(BackendSurfaceId::new(40));
        let trace = adapter.single_surface_map_trace();

        assert!(matches!(
            trace.events().first(),
            Some(BackendSurfaceLifecycleEvent::Created { id })
                if *id == BackendSurfaceId::new(40)
        ));
        assert_eq!(adapter.peek_next_id(), BackendSurfaceId::new(41));
    }

    /// 验证 SingleMap 场景可生成并运行。
    #[test]
    fn single_map_scenario_builds_and_runs() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::SingleMap);

        assert!(report.is_success());
    }

    /// 验证 ConfigureThenMap 场景可生成并运行。
    #[test]
    fn configure_then_map_scenario_builds_and_runs() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::ConfigureThenMap);

        assert!(report.is_success());
    }

    /// 验证 MapUnmapRemap 场景可生成并运行。
    #[test]
    fn map_unmap_remap_scenario_builds_and_runs() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::MapUnmapRemap);

        assert!(report.is_success());
    }

    /// 验证 Destroy 场景可生成并运行。
    #[test]
    fn destroy_scenario_builds_and_runs() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::Destroy);

        assert!(report.is_success());
    }

    /// 验证 InvalidDestroyedRemap 场景产生预期失败。
    #[test]
    fn invalid_destroyed_remap_scenario_builds_expected_failure() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::InvalidDestroyedRemap);

        assert_eq!(report.failed_at, Some(2));
        assert!(matches!(
            report.error,
            Some(BackendSurfaceLifecycleError::AlreadyDestroyed { .. })
        ));
    }

    /// 验证 MultiSurface 场景可生成并运行。
    #[test]
    fn multi_surface_scenario_builds_and_runs() {
        let (report, _) = run_scenario(BackendSurfaceTraceScenario::MultiSurface);

        assert!(report.is_success());
        assert_eq!(report.applied, 8);
        assert_eq!(report.final_records.len(), 3);
    }

    /// 验证 runner 确实遵守 registry 的重复 ID 错误。
    #[test]
    fn trace_runner_reuses_registry_error_rules() {
        let id = BackendSurfaceId::new(5);
        let trace = BackendSurfaceTrace::from_events([
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Created { id },
        ]);
        let mut registry = BackendSurfaceRegistry::new();

        let report = trace.run(&mut registry);

        assert_eq!(report.applied, 1);
        assert_eq!(report.failed_at, Some(1));
        assert_eq!(
            report.error,
            Some(BackendSurfaceLifecycleError::AlreadyExists { id })
        );
    }

    /// 验证本模块生产代码没有直接引用 Smithay crate。
    #[test]
    fn surface_trace_has_no_smithay_crate_reference() {
        let production_source = production_source();

        assert!(!production_source.contains("smithay::"));
        assert!(!production_source.contains("use smithay"));
    }

    /// 验证本模块生产代码没有 Linux 专属 API。
    #[test]
    fn surface_trace_has_no_linux_api_reference() {
        let production_source = production_source();

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
                !production_source.contains(forbidden),
                "生产代码不应包含 Linux 或 Wayland API: {forbidden}"
            );
        }
    }

    /// 验证 core 和 backend 不反向依赖 surface trace 类型。
    #[test]
    fn core_and_backend_do_not_depend_on_surface_trace() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        for relative_dir in ["src/core", "src/backend"] {
            assert_directory_has_no_trace_dependency(&manifest_dir.join(relative_dir));
        }
    }

    /// 读取测试模块之前的生产代码。
    fn production_source() -> &'static str {
        include_str!("surface_trace.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("生产代码片段应存在")
    }

    /// 递归检查 Rust 源文件中的 trace 反向依赖。
    fn assert_directory_has_no_trace_dependency(directory: &Path) {
        for entry in fs::read_dir(directory).expect("源码目录应可读取") {
            let path = entry.expect("源码目录项应可读取").path();

            if path.is_dir() {
                assert_directory_has_no_trace_dependency(&path);
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }

            let source = fs::read_to_string(&path).expect("Rust 源文件应可读取");
            let forbidden_references = [
                "smithay_backend::surface_trace",
                "BackendSurfaceTrace",
                "BackendSurfaceTraceReport",
                "BackendSurfaceTraceRunner",
                "BackendSurfaceMockAdapter",
                "BackendSurfaceTraceScenario",
            ];
            assert!(
                forbidden_references
                    .iter()
                    .all(|reference| !source.contains(reference)),
                "{} 不应依赖 smithay_backend surface trace",
                path.display()
            );
        }
    }
}
