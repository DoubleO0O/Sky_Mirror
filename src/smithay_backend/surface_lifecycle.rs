//! Surface 生命周期纯数据预备层。
//!
//! 本模块只描述后端 surface 从创建到销毁的状态变化，不保存真实
//! `wl_surface`，不依赖 Smithay，也不向核心 `State` 提交事件。
//! 未来 Linux 适配器应把真实协议回调转换为这里的纯数据事件，再由后续边界
//! 决定如何进入既有 `BackendEvent` 驱动路径。
//!
//! Platform boundary: 该模型是跨平台纯 Rust 契约；在任意主机上的测试通过都不
//! 替代 Linux 系统资源验收，也不证明真实 Wayland 协议时序成立。

use std::{collections::BTreeMap, fmt};

/// 后端局部使用的稳定 surface 标识。
///
/// 该标识不等于核心层的 `SurfaceId`，也不携带真实 Wayland 对象引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendSurfaceId(u64);

impl BackendSurfaceId {
    /// 从明确数值创建后端 surface 标识。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// 返回标识的原始数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Surface 最近一次已知的逻辑尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSurfaceSize {
    /// 逻辑宽度。
    pub width: u32,

    /// 逻辑高度。
    pub height: u32,
}

impl BackendSurfaceSize {
    /// 创建非零 surface 尺寸。
    pub fn new(width: u32, height: u32) -> Result<Self, BackendSurfaceLifecycleError> {
        let size = Self { width, height };
        size.validate()?;
        Ok(size)
    }

    /// 确认尺寸可以进入生命周期记录。
    fn validate(self) -> Result<(), BackendSurfaceLifecycleError> {
        if self.width == 0 || self.height == 0 {
            return Err(BackendSurfaceLifecycleError::InvalidSize {
                width: self.width,
                height: self.height,
            });
        }

        Ok(())
    }
}

/// 后端 surface 的纯数据生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSurfaceLifecycleState {
    /// 后端已经观察到 surface 创建。
    Created,

    /// Surface 已收到至少一次有效 configure。
    Configured,

    /// Surface 当前可映射到场景。
    Mapped,

    /// Surface 曾映射，但当前已撤销映射。
    Unmapped,

    /// Surface 已销毁，只保留 tombstone 供诊断和去重。
    Destroyed,
}

/// 一个 surface 的后端局部生命周期记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSurfaceRecord {
    /// 稳定后端标识。
    pub id: BackendSurfaceId,

    /// 当前生命周期状态。
    pub state: BackendSurfaceLifecycleState,

    /// 最近一次映射事件提供的标题。
    pub title: Option<String>,

    /// 最近一次映射事件提供的应用标识。
    pub app_id: Option<String>,

    /// 最近一次有效 configure 提供的尺寸。
    pub last_known_size: Option<BackendSurfaceSize>,
}

impl BackendSurfaceRecord {
    /// 创建初始 surface 记录。
    fn created(id: BackendSurfaceId) -> Self {
        Self {
            id,
            state: BackendSurfaceLifecycleState::Created,
            title: None,
            app_id: None,
            last_known_size: None,
        }
    }
}

/// 可由未来平台适配器产生的 surface 生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendSurfaceLifecycleEvent {
    /// 创建一个明确 ID 的 surface。
    Created {
        /// 后端 surface 标识。
        id: BackendSurfaceId,
    },

    /// 记录 configure；缺少尺寸时仍记录状态，但保留旧尺寸。
    Configured {
        /// 后端 surface 标识。
        id: BackendSurfaceId,

        /// 本次 configure 提供的可选尺寸。
        size: Option<BackendSurfaceSize>,
    },

    /// 映射 surface，并记录后端可见的窗口元数据。
    Mapped {
        /// 后端 surface 标识。
        id: BackendSurfaceId,

        /// 可选窗口标题。
        title: Option<String>,

        /// 可选应用标识。
        app_id: Option<String>,
    },

    /// 撤销 surface 映射。
    Unmapped {
        /// 后端 surface 标识。
        id: BackendSurfaceId,
    },

    /// 销毁 surface，并保留 tombstone。
    Destroyed {
        /// 后端 surface 标识。
        id: BackendSurfaceId,
    },
}

/// Surface 生命周期操作失败的结构化原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendSurfaceLifecycleError {
    /// 操作引用了尚未创建的 surface。
    UnknownSurface {
        /// 未知 surface 标识。
        id: BackendSurfaceId,
    },

    /// 显式创建使用了已经存在的 ID。
    AlreadyExists {
        /// 重复 surface 标识。
        id: BackendSurfaceId,
    },

    /// 当前状态不允许目标状态转换。
    InvalidTransition {
        /// 发生错误的 surface。
        id: BackendSurfaceId,

        /// 当前状态。
        from: BackendSurfaceLifecycleState,

        /// 请求进入的状态。
        to: BackendSurfaceLifecycleState,
    },

    /// Surface 已销毁，后续操作不能复活 tombstone。
    AlreadyDestroyed {
        /// 已销毁 surface 标识。
        id: BackendSurfaceId,
    },

    /// Surface 尺寸包含零宽或零高。
    InvalidSize {
        /// 无效宽度。
        width: u32,

        /// 无效高度。
        height: u32,
    },
}

impl fmt::Display for BackendSurfaceLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSurface { id } => {
                write!(formatter, "未知后端 surface: {}", id.value())
            }
            Self::AlreadyExists { id } => {
                write!(formatter, "后端 surface 已存在: {}", id.value())
            }
            Self::InvalidTransition { id, from, to } => write!(
                formatter,
                "后端 surface {} 不能从 {from:?} 转换到 {to:?}",
                id.value()
            ),
            Self::AlreadyDestroyed { id } => {
                write!(formatter, "后端 surface 已销毁: {}", id.value())
            }
            Self::InvalidSize { width, height } => {
                write!(formatter, "无效 surface 尺寸: {width}x{height}")
            }
        }
    }
}

impl std::error::Error for BackendSurfaceLifecycleError {}

/// 后端局部的 surface 生命周期注册表。
///
/// 注册表使用 `BTreeMap` 保持可审计的稳定顺序。销毁操作只改变记录状态，
/// 不删除条目，从而能够拒绝 ID 复用并保留生命周期诊断信息。
#[derive(Debug, Clone)]
pub struct BackendSurfaceRegistry {
    next_id: u64,
    surfaces: BTreeMap<BackendSurfaceId, BackendSurfaceRecord>,
}

impl BackendSurfaceRegistry {
    /// 创建空注册表，自动 ID 从 1 开始。
    pub fn new() -> Self {
        Self {
            next_id: 1,
            surfaces: BTreeMap::new(),
        }
    }

    /// 自动分配 ID 并创建 surface。
    pub fn create_surface(&mut self) -> Result<BackendSurfaceId, BackendSurfaceLifecycleError> {
        let id = BackendSurfaceId::new(self.next_id);
        self.create_surface_with_id(id)?;
        Ok(id)
    }

    /// 使用明确 ID 创建 surface。
    ///
    /// 显式 ID 会推进自动分配计数器，避免后续自动创建与既有记录冲突。
    pub fn create_surface_with_id(
        &mut self,
        id: BackendSurfaceId,
    ) -> Result<BackendSurfaceId, BackendSurfaceLifecycleError> {
        if self.surfaces.contains_key(&id) {
            return Err(BackendSurfaceLifecycleError::AlreadyExists { id });
        }

        self.surfaces.insert(id, BackendSurfaceRecord::created(id));
        self.next_id = self.next_id.max(id.value().saturating_add(1));
        Ok(id)
    }

    /// 记录一次 configure。
    ///
    /// `None` 表示平台回调没有提供尺寸，此时进入 `Configured`，但不会清除
    /// 已经记录的最近有效尺寸。
    pub fn configure_surface(
        &mut self,
        id: BackendSurfaceId,
        size: Option<BackendSurfaceSize>,
    ) -> Result<(), BackendSurfaceLifecycleError> {
        if let Some(size) = size {
            size.validate()?;
        }

        let record = self.record_for_update(id)?;
        Self::ensure_transition(record, BackendSurfaceLifecycleState::Configured)?;

        record.state = BackendSurfaceLifecycleState::Configured;
        if let Some(size) = size {
            record.last_known_size = Some(size);
        }
        Ok(())
    }

    /// 映射 surface 并更新可选标题与应用标识。
    ///
    /// 本预备模型明确允许 `Created -> Mapped`，用于表示后端暂时没有单独
    /// configure 事件的纯数据回放；这不代表真实协议层可以省略 configure。
    pub fn map_surface(
        &mut self,
        id: BackendSurfaceId,
        title: Option<String>,
        app_id: Option<String>,
    ) -> Result<(), BackendSurfaceLifecycleError> {
        let record = self.record_for_update(id)?;
        Self::ensure_transition(record, BackendSurfaceLifecycleState::Mapped)?;

        record.state = BackendSurfaceLifecycleState::Mapped;
        record.title = title;
        record.app_id = app_id;
        Ok(())
    }

    /// 撤销一个已映射 surface。
    pub fn unmap_surface(
        &mut self,
        id: BackendSurfaceId,
    ) -> Result<(), BackendSurfaceLifecycleError> {
        let record = self.record_for_update(id)?;
        Self::ensure_transition(record, BackendSurfaceLifecycleState::Unmapped)?;
        record.state = BackendSurfaceLifecycleState::Unmapped;
        Ok(())
    }

    /// 销毁 surface，并保留记录作为 tombstone。
    pub fn destroy_surface(
        &mut self,
        id: BackendSurfaceId,
    ) -> Result<(), BackendSurfaceLifecycleError> {
        let record = self.record_for_update(id)?;
        Self::ensure_transition(record, BackendSurfaceLifecycleState::Destroyed)?;
        record.state = BackendSurfaceLifecycleState::Destroyed;
        Ok(())
    }

    /// 按 ID 读取 surface 记录。
    pub fn get_surface(&self, id: BackendSurfaceId) -> Option<&BackendSurfaceRecord> {
        self.surfaces.get(&id)
    }

    /// 按 ID 稳定顺序列出全部记录，包括已销毁 tombstone。
    pub fn list_surfaces(&self) -> Vec<&BackendSurfaceRecord> {
        self.surfaces.values().collect()
    }

    /// 按 ID 稳定顺序列出当前已映射记录。
    pub fn mapped_surfaces(&self) -> Vec<&BackendSurfaceRecord> {
        self.surfaces
            .values()
            .filter(|record| record.state == BackendSurfaceLifecycleState::Mapped)
            .collect()
    }

    /// 应用一条纯数据生命周期事件。
    ///
    /// Transition invariant: 每个事件只经由对应的 registry 操作推进状态；非法
    /// 转换返回结构化错误，失败事件不会被伪装成已应用。
    pub fn apply_event(
        &mut self,
        event: BackendSurfaceLifecycleEvent,
    ) -> Result<(), BackendSurfaceLifecycleError> {
        match event {
            BackendSurfaceLifecycleEvent::Created { id } => {
                self.create_surface_with_id(id)?;
                Ok(())
            }
            BackendSurfaceLifecycleEvent::Configured { id, size } => {
                self.configure_surface(id, size)
            }
            BackendSurfaceLifecycleEvent::Mapped { id, title, app_id } => {
                self.map_surface(id, title, app_id)
            }
            BackendSurfaceLifecycleEvent::Unmapped { id } => self.unmap_surface(id),
            BackendSurfaceLifecycleEvent::Destroyed { id } => self.destroy_surface(id),
        }
    }

    /// 获取可变记录，并统一处理未知 ID。
    fn record_for_update(
        &mut self,
        id: BackendSurfaceId,
    ) -> Result<&mut BackendSurfaceRecord, BackendSurfaceLifecycleError> {
        self.surfaces
            .get_mut(&id)
            .ok_or(BackendSurfaceLifecycleError::UnknownSurface { id })
    }

    /// 验证状态转换，销毁后的操作使用更明确的错误类别。
    fn ensure_transition(
        record: &BackendSurfaceRecord,
        to: BackendSurfaceLifecycleState,
    ) -> Result<(), BackendSurfaceLifecycleError> {
        if record.state == BackendSurfaceLifecycleState::Destroyed {
            return Err(BackendSurfaceLifecycleError::AlreadyDestroyed { id: record.id });
        }

        let valid = matches!(
            (record.state, to),
            (
                BackendSurfaceLifecycleState::Created,
                BackendSurfaceLifecycleState::Configured
            ) | (
                BackendSurfaceLifecycleState::Created,
                BackendSurfaceLifecycleState::Mapped
            ) | (
                BackendSurfaceLifecycleState::Configured,
                BackendSurfaceLifecycleState::Configured
            ) | (
                BackendSurfaceLifecycleState::Configured,
                BackendSurfaceLifecycleState::Mapped
            ) | (
                BackendSurfaceLifecycleState::Mapped,
                BackendSurfaceLifecycleState::Unmapped
            ) | (
                BackendSurfaceLifecycleState::Unmapped,
                BackendSurfaceLifecycleState::Mapped
            ) | (_, BackendSurfaceLifecycleState::Destroyed)
        );

        if !valid {
            return Err(BackendSurfaceLifecycleError::InvalidTransition {
                id: record.id,
                from: record.state,
                to,
            });
        }

        Ok(())
    }
}

impl Default for BackendSurfaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        BackendSurfaceId, BackendSurfaceLifecycleError, BackendSurfaceLifecycleEvent,
        BackendSurfaceLifecycleState, BackendSurfaceRegistry, BackendSurfaceSize,
    };

    /// 创建测试使用的有效尺寸。
    fn size(width: u32, height: u32) -> BackendSurfaceSize {
        BackendSurfaceSize::new(width, height).expect("测试尺寸必须有效")
    }

    /// 验证 ID 支持稳定比较和原始值读取。
    #[test]
    fn surface_id_is_stable_and_ordered() {
        let first = BackendSurfaceId::new(1);
        let second = BackendSurfaceId::new(2);

        assert!(first < second);
        assert_eq!(first.value(), 1);
    }

    /// 验证默认注册表从 1 开始分配递增 ID。
    #[test]
    fn registry_allocates_incrementing_ids() {
        let mut registry = BackendSurfaceRegistry::new();

        assert_eq!(registry.create_surface(), Ok(BackendSurfaceId::new(1)));
        assert_eq!(registry.create_surface(), Ok(BackendSurfaceId::new(2)));
    }

    /// 验证显式大 ID 会推进自动分配计数器。
    #[test]
    fn explicit_id_advances_allocator() {
        let mut registry = BackendSurfaceRegistry::new();

        registry
            .create_surface_with_id(BackendSurfaceId::new(41))
            .expect("显式创建应成功");

        assert_eq!(registry.create_surface(), Ok(BackendSurfaceId::new(42)));
    }

    /// 验证新记录处于 Created 且没有元数据。
    #[test]
    fn created_surface_has_empty_metadata() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        let record = registry.get_surface(id).expect("记录应存在");

        assert_eq!(record.state, BackendSurfaceLifecycleState::Created);
        assert_eq!(record.title, None);
        assert_eq!(record.app_id, None);
        assert_eq!(record.last_known_size, None);
    }

    /// 验证重复显式 ID 返回 AlreadyExists。
    #[test]
    fn duplicate_explicit_id_is_rejected() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(7);
        registry.create_surface_with_id(id).expect("首次创建应成功");

        assert_eq!(
            registry.create_surface_with_id(id),
            Err(BackendSurfaceLifecycleError::AlreadyExists { id })
        );
    }

    /// 验证零宽尺寸被拒绝。
    #[test]
    fn zero_width_is_invalid() {
        assert_eq!(
            BackendSurfaceSize::new(0, 480),
            Err(BackendSurfaceLifecycleError::InvalidSize {
                width: 0,
                height: 480
            })
        );
    }

    /// 验证零高尺寸被拒绝。
    #[test]
    fn zero_height_is_invalid() {
        assert_eq!(
            BackendSurfaceSize::new(640, 0),
            Err(BackendSurfaceLifecycleError::InvalidSize {
                width: 640,
                height: 0
            })
        );
    }

    /// 验证公开字段绕过构造器时，configure 仍会拒绝无效尺寸。
    #[test]
    fn configure_revalidates_public_size_fields() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");

        assert_eq!(
            registry.configure_surface(
                id,
                Some(BackendSurfaceSize {
                    width: 0,
                    height: 1,
                }),
            ),
            Err(BackendSurfaceLifecycleError::InvalidSize {
                width: 0,
                height: 1
            })
        );
    }

    /// 验证 Created 可以进入 Configured 并记录尺寸。
    #[test]
    fn created_surface_can_be_configured() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        let configured_size = size(800, 600);

        registry
            .configure_surface(id, Some(configured_size))
            .expect("configure 应成功");

        let record = registry.get_surface(id).expect("记录应存在");
        assert_eq!(record.state, BackendSurfaceLifecycleState::Configured);
        assert_eq!(record.last_known_size, Some(configured_size));
    }

    /// 验证无尺寸 configure 不会清除最近有效尺寸。
    #[test]
    fn configure_without_size_preserves_last_known_size() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        let configured_size = size(1024, 768);
        registry
            .configure_surface(id, Some(configured_size))
            .expect("首次 configure 应成功");

        registry
            .configure_surface(id, None)
            .expect("无尺寸 configure 应成功");

        assert_eq!(
            registry
                .get_surface(id)
                .expect("记录应存在")
                .last_known_size,
            Some(configured_size)
        );
    }

    /// 验证 Configured 可以映射并记录标题和应用标识。
    #[test]
    fn configured_surface_can_be_mapped() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry
            .configure_surface(id, Some(size(1280, 720)))
            .expect("configure 应成功");

        registry
            .map_surface(
                id,
                Some("终端".to_string()),
                Some("org.example.Terminal".to_string()),
            )
            .expect("map 应成功");

        let record = registry.get_surface(id).expect("记录应存在");
        assert_eq!(record.state, BackendSurfaceLifecycleState::Mapped);
        assert_eq!(record.title.as_deref(), Some("终端"));
        assert_eq!(record.app_id.as_deref(), Some("org.example.Terminal"));
    }

    /// 验证预备模型明确允许 Created 直接进入 Mapped。
    #[test]
    fn created_surface_can_map_without_configure() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");

        registry
            .map_surface(id, None, None)
            .expect("直接 map 应成功");

        assert_eq!(
            registry.get_surface(id).expect("记录应存在").state,
            BackendSurfaceLifecycleState::Mapped
        );
    }

    /// 验证 Mapped 可以进入 Unmapped。
    #[test]
    fn mapped_surface_can_be_unmapped() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry.map_surface(id, None, None).expect("map 应成功");

        registry.unmap_surface(id).expect("unmap 应成功");

        assert_eq!(
            registry.get_surface(id).expect("记录应存在").state,
            BackendSurfaceLifecycleState::Unmapped
        );
    }

    /// 验证 Unmapped 可以重新进入 Mapped 并更新元数据。
    #[test]
    fn unmapped_surface_can_be_remapped() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry
            .map_surface(id, Some("旧标题".to_string()), None)
            .expect("首次 map 应成功");
        registry.unmap_surface(id).expect("unmap 应成功");

        registry
            .map_surface(id, Some("新标题".to_string()), None)
            .expect("重新 map 应成功");

        let record = registry.get_surface(id).expect("记录应存在");
        assert_eq!(record.state, BackendSurfaceLifecycleState::Mapped);
        assert_eq!(record.title.as_deref(), Some("新标题"));
    }

    /// 验证重复 map 会返回明确的无效转换。
    #[test]
    fn repeated_map_is_invalid() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry
            .map_surface(id, None, None)
            .expect("首次 map 应成功");

        assert_eq!(
            registry.map_surface(id, None, None),
            Err(BackendSurfaceLifecycleError::InvalidTransition {
                id,
                from: BackendSurfaceLifecycleState::Mapped,
                to: BackendSurfaceLifecycleState::Mapped,
            })
        );
    }

    /// 验证未映射记录不能直接 unmap。
    #[test]
    fn unmap_requires_mapped_state() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");

        assert_eq!(
            registry.unmap_surface(id),
            Err(BackendSurfaceLifecycleError::InvalidTransition {
                id,
                from: BackendSurfaceLifecycleState::Created,
                to: BackendSurfaceLifecycleState::Unmapped,
            })
        );
    }

    /// 验证任意未销毁状态都可以进入 Destroyed。
    #[test]
    fn any_live_state_can_be_destroyed() {
        let mut registry = BackendSurfaceRegistry::new();
        let created = registry.create_surface().expect("创建应成功");
        let configured = registry.create_surface().expect("创建应成功");
        let mapped = registry.create_surface().expect("创建应成功");
        let unmapped = registry.create_surface().expect("创建应成功");

        registry
            .configure_surface(configured, None)
            .expect("configure 应成功");
        registry
            .map_surface(mapped, None, None)
            .expect("map 应成功");
        registry
            .map_surface(unmapped, None, None)
            .expect("map 应成功");
        registry.unmap_surface(unmapped).expect("unmap 应成功");

        for id in [created, configured, mapped, unmapped] {
            registry.destroy_surface(id).expect("destroy 应成功");
            assert_eq!(
                registry.get_surface(id).expect("记录应存在").state,
                BackendSurfaceLifecycleState::Destroyed
            );
        }
    }

    /// 验证销毁记录保留为 tombstone。
    #[test]
    fn destroyed_surface_remains_as_tombstone() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry.destroy_surface(id).expect("destroy 应成功");

        assert_eq!(registry.list_surfaces().len(), 1);
        assert_eq!(
            registry.get_surface(id).expect("tombstone 应保留").state,
            BackendSurfaceLifecycleState::Destroyed
        );
    }

    /// 验证销毁后的 map 不会复活记录。
    #[test]
    fn destroyed_surface_cannot_be_mapped() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry.destroy_surface(id).expect("destroy 应成功");

        assert_eq!(
            registry.map_surface(id, None, None),
            Err(BackendSurfaceLifecycleError::AlreadyDestroyed { id })
        );
    }

    /// 验证销毁后的 configure 不会改变 tombstone。
    #[test]
    fn destroyed_surface_cannot_be_configured() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry.destroy_surface(id).expect("destroy 应成功");

        assert_eq!(
            registry.configure_surface(id, Some(size(800, 600))),
            Err(BackendSurfaceLifecycleError::AlreadyDestroyed { id })
        );
        assert_eq!(
            registry.get_surface(id).expect("tombstone 应保留").state,
            BackendSurfaceLifecycleState::Destroyed
        );
    }

    /// 验证销毁后的 unmap 不会改变 tombstone。
    #[test]
    fn destroyed_surface_cannot_be_unmapped() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry.destroy_surface(id).expect("destroy 应成功");

        assert_eq!(
            registry.unmap_surface(id),
            Err(BackendSurfaceLifecycleError::AlreadyDestroyed { id })
        );
        assert_eq!(
            registry.get_surface(id).expect("tombstone 应保留").state,
            BackendSurfaceLifecycleState::Destroyed
        );
    }

    /// 验证重复 destroy 返回 AlreadyDestroyed。
    #[test]
    fn repeated_destroy_is_rejected() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = registry.create_surface().expect("创建应成功");
        registry.destroy_surface(id).expect("首次 destroy 应成功");

        assert_eq!(
            registry.destroy_surface(id),
            Err(BackendSurfaceLifecycleError::AlreadyDestroyed { id })
        );
    }

    /// 验证未知 ID configure 返回 UnknownSurface。
    #[test]
    fn unknown_surface_configure_is_rejected() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(99);

        assert_eq!(
            registry.configure_surface(id, Some(size(800, 600))),
            Err(BackendSurfaceLifecycleError::UnknownSurface { id })
        );
    }

    /// 验证未知 ID map 返回 UnknownSurface。
    #[test]
    fn unknown_surface_map_is_rejected() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(99);

        assert_eq!(
            registry.map_surface(id, None, None),
            Err(BackendSurfaceLifecycleError::UnknownSurface { id })
        );
    }

    /// 验证未知 ID unmap 返回 UnknownSurface。
    #[test]
    fn unknown_surface_unmap_is_rejected() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(99);

        assert_eq!(
            registry.unmap_surface(id),
            Err(BackendSurfaceLifecycleError::UnknownSurface { id })
        );
    }

    /// 验证未知 ID destroy 返回 UnknownSurface。
    #[test]
    fn unknown_surface_destroy_is_rejected() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(99);

        assert_eq!(
            registry.destroy_surface(id),
            Err(BackendSurfaceLifecycleError::UnknownSurface { id })
        );
    }

    /// 验证全部记录按 ID 稳定排序。
    #[test]
    fn surface_listing_is_stably_ordered() {
        let mut registry = BackendSurfaceRegistry::new();
        for value in [30, 10, 20] {
            registry
                .create_surface_with_id(BackendSurfaceId::new(value))
                .expect("显式创建应成功");
        }

        let ids: Vec<_> = registry
            .list_surfaces()
            .into_iter()
            .map(|record| record.id.value())
            .collect();

        assert_eq!(ids, vec![10, 20, 30]);
    }

    /// 验证 mapped 查询只返回当前已映射记录。
    #[test]
    fn mapped_listing_filters_other_states() {
        let mut registry = BackendSurfaceRegistry::new();
        let mapped = registry.create_surface().expect("创建应成功");
        let unmapped = registry.create_surface().expect("创建应成功");
        let destroyed = registry.create_surface().expect("创建应成功");
        registry
            .map_surface(mapped, None, None)
            .expect("map 应成功");
        registry
            .map_surface(unmapped, None, None)
            .expect("map 应成功");
        registry.unmap_surface(unmapped).expect("unmap 应成功");
        registry.destroy_surface(destroyed).expect("destroy 应成功");

        let ids: Vec<_> = registry
            .mapped_surfaces()
            .into_iter()
            .map(|record| record.id)
            .collect();

        assert_eq!(ids, vec![mapped]);
    }

    /// 验证 apply_event 可以驱动完整生命周期。
    #[test]
    fn apply_event_drives_full_lifecycle() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(8);
        let events = [
            BackendSurfaceLifecycleEvent::Created { id },
            BackendSurfaceLifecycleEvent::Configured {
                id,
                size: Some(size(1920, 1080)),
            },
            BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("编辑器".to_string()),
                app_id: Some("dev.editor".to_string()),
            },
            BackendSurfaceLifecycleEvent::Unmapped { id },
            BackendSurfaceLifecycleEvent::Destroyed { id },
        ];

        for event in events {
            registry.apply_event(event).expect("事件应用应成功");
        }

        let record = registry.get_surface(id).expect("记录应存在");
        assert_eq!(record.state, BackendSurfaceLifecycleState::Destroyed);
        assert_eq!(record.last_known_size, Some(size(1920, 1080)));
        assert_eq!(record.title.as_deref(), Some("编辑器"));
    }

    /// 验证事件路径同样保留结构化错误。
    #[test]
    fn apply_event_preserves_structured_errors() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(5);

        assert_eq!(
            registry.apply_event(BackendSurfaceLifecycleEvent::Destroyed { id }),
            Err(BackendSurfaceLifecycleError::UnknownSurface { id })
        );
    }

    /// 验证 Created 事件可以单独创建记录。
    #[test]
    fn apply_created_event_creates_surface() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(12);

        registry
            .apply_event(BackendSurfaceLifecycleEvent::Created { id })
            .expect("Created 事件应成功");

        assert_eq!(
            registry.get_surface(id).expect("记录应存在").state,
            BackendSurfaceLifecycleState::Created
        );
    }

    /// 验证 Configured 事件可以单独推进状态。
    #[test]
    fn apply_configured_event_records_size() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(12);
        registry
            .apply_event(BackendSurfaceLifecycleEvent::Created { id })
            .expect("Created 事件应成功");

        registry
            .apply_event(BackendSurfaceLifecycleEvent::Configured {
                id,
                size: Some(size(640, 480)),
            })
            .expect("Configured 事件应成功");

        assert_eq!(
            registry.get_surface(id).expect("记录应存在").state,
            BackendSurfaceLifecycleState::Configured
        );
    }

    /// 验证 Mapped 事件可以单独记录元数据。
    #[test]
    fn apply_mapped_event_records_metadata() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(12);
        registry
            .apply_event(BackendSurfaceLifecycleEvent::Created { id })
            .expect("Created 事件应成功");

        registry
            .apply_event(BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: Some("浏览器".to_string()),
                app_id: Some("org.example.Browser".to_string()),
            })
            .expect("Mapped 事件应成功");

        let record = registry.get_surface(id).expect("记录应存在");
        assert_eq!(record.state, BackendSurfaceLifecycleState::Mapped);
        assert_eq!(record.title.as_deref(), Some("浏览器"));
    }

    /// 验证 Unmapped 事件可以单独推进状态。
    #[test]
    fn apply_unmapped_event_updates_state() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(12);
        registry
            .apply_event(BackendSurfaceLifecycleEvent::Created { id })
            .expect("Created 事件应成功");
        registry
            .apply_event(BackendSurfaceLifecycleEvent::Mapped {
                id,
                title: None,
                app_id: None,
            })
            .expect("Mapped 事件应成功");

        registry
            .apply_event(BackendSurfaceLifecycleEvent::Unmapped { id })
            .expect("Unmapped 事件应成功");

        assert_eq!(
            registry.get_surface(id).expect("记录应存在").state,
            BackendSurfaceLifecycleState::Unmapped
        );
    }

    /// 验证 Destroyed 事件可以单独生成 tombstone。
    #[test]
    fn apply_destroyed_event_keeps_tombstone() {
        let mut registry = BackendSurfaceRegistry::new();
        let id = BackendSurfaceId::new(12);
        registry
            .apply_event(BackendSurfaceLifecycleEvent::Created { id })
            .expect("Created 事件应成功");

        registry
            .apply_event(BackendSurfaceLifecycleEvent::Destroyed { id })
            .expect("Destroyed 事件应成功");

        assert_eq!(
            registry.get_surface(id).expect("tombstone 应保留").state,
            BackendSurfaceLifecycleState::Destroyed
        );
    }

    /// 验证错误文本包含 surface 或尺寸上下文。
    #[test]
    fn lifecycle_error_display_preserves_context() {
        let error = BackendSurfaceLifecycleError::InvalidSize {
            width: 0,
            height: 720,
        };

        assert_eq!(error.to_string(), "无效 surface 尺寸: 0x720");
    }

    /// 验证本模块没有直接引用 Smithay crate。
    #[test]
    fn lifecycle_model_has_no_smithay_crate_reference() {
        let source = include_str!("surface_lifecycle.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("生产代码片段应存在");

        assert!(!production_source.contains("smithay::"));
        assert!(!production_source.contains("use smithay"));
    }

    /// 验证 core 和 backend 源码没有反向依赖本模块类型。
    #[test]
    fn core_and_backend_do_not_depend_on_surface_lifecycle_model() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        for relative_dir in ["src/core", "src/backend"] {
            assert_directory_has_no_lifecycle_dependency(&manifest_dir.join(relative_dir));
        }
    }

    /// 递归检查 Rust 源文件中的生命周期模型反向依赖。
    fn assert_directory_has_no_lifecycle_dependency(directory: &Path) {
        for entry in fs::read_dir(directory).expect("源码目录应可读取") {
            let path = entry.expect("源码目录项应可读取").path();

            if path.is_dir() {
                assert_directory_has_no_lifecycle_dependency(&path);
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }

            let source = fs::read_to_string(&path).expect("Rust 源文件应可读取");
            let forbidden_references = [
                "smithay_backend::surface_lifecycle",
                "BackendSurfaceId",
                "BackendSurfaceSize",
                "BackendSurfaceLifecycleState",
                "BackendSurfaceLifecycleEvent",
                "BackendSurfaceLifecycleError",
                "BackendSurfaceRecord",
                "BackendSurfaceRegistry",
            ];
            assert!(
                forbidden_references
                    .iter()
                    .all(|reference| !source.contains(reference)),
                "{} 不应依赖 smithay_backend surface 生命周期模型",
                path.display()
            );
        }
    }
}
