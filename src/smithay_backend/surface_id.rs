//! Smithay surface ID 分配器探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不保存真实 `wl_surface`，不保存 Smithay surface，不注册
//! `wl_compositor`，也不接 xdg-shell。
//!
//! 它只负责为未来 surface 创建事件生成核心纯数据层使用的 `SurfaceId`。

use crate::core::surface::SurfaceId;

/// Smithay surface ID 分配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该分配器只服务于纯数据探针流程，
/// 不代表真实 `wl_surface` 已经进入 Wayland Display。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithaySurfaceIdAllocatorMode {
    /// 纯探针模式。
    ///
    /// 不保存真实 `wl_surface`，不注册 `wl_compositor`，不接 xdg-shell。
    ProbeOnly,
}

/// Smithay surface ID 分配器探针。
///
/// 该结构只维护一个递增计数器，用于模拟未来真实 Wayland client 创建 surface 后
/// 为其分配稳定 `SurfaceId` 的过程。
///
/// 分配出的 `SurfaceId` 只属于核心纯数据模型，不等于真实 `wl_surface` 对象。
pub struct SmithaySurfaceIdAllocatorProbe {
    /// 下一次分配的 surface ID。
    next_surface_id: SurfaceId,

    /// 当前分配器模式。
    mode: SmithaySurfaceIdAllocatorMode,
}

impl SmithaySurfaceIdAllocatorProbe {
    /// 创建默认 surface ID 分配器。
    ///
    /// 默认从 1 开始分配，保持与核心 `SurfaceRegistry` 的初始习惯一致。
    pub fn new() -> Self {
        Self {
            next_surface_id: 1,
            mode: SmithaySurfaceIdAllocatorMode::ProbeOnly,
        }
    }

    /// 使用指定起点创建 surface ID 分配器。
    ///
    /// 该方法主要用于测试或未来从外部状态恢复分配器计数。
    pub fn with_next_surface_id(next_surface_id: SurfaceId) -> Self {
        Self {
            next_surface_id,
            mode: SmithaySurfaceIdAllocatorMode::ProbeOnly,
        }
    }

    /// 分配下一个 surface ID。
    ///
    /// 这里只生成核心纯数据 ID，不注册真实 `wl_surface`。
    pub fn next_surface_id(&mut self) -> SurfaceId {
        let surface = self.next_surface_id;
        self.next_surface_id = self.next_surface_id.saturating_add(1);
        surface
    }

    /// 返回下一次将分配的 surface ID。
    pub fn peek_next_surface_id(&self) -> SurfaceId {
        self.next_surface_id
    }

    /// 返回当前分配器模式。
    pub fn mode(&self) -> SmithaySurfaceIdAllocatorMode {
        self.mode
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only(&self) -> bool {
        self.mode == SmithaySurfaceIdAllocatorMode::ProbeOnly
    }

    /// 返回当前阶段说明。
    pub fn mode_description(&self) -> &'static str {
        "smithay-surface-id-allocator-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithaySurfaceIdAllocatorMode, SmithaySurfaceIdAllocatorProbe};

    /// 验证默认分配器从核心约定的 surface ID 1 开始。
    #[test]
    fn smithay_surface_id_allocator_starts_at_one() {
        let allocator = SmithaySurfaceIdAllocatorProbe::new();

        assert_eq!(allocator.peek_next_surface_id(), 1);
        assert!(allocator.is_probe_only());
        assert_eq!(allocator.mode(), SmithaySurfaceIdAllocatorMode::ProbeOnly);
        assert_eq!(
            allocator.mode_description(),
            "smithay-surface-id-allocator-probe-only"
        );
    }

    /// 验证分配器会按顺序产生递增且稳定的核心 surface ID。
    #[test]
    fn smithay_surface_id_allocator_allocates_incrementing_ids() {
        let mut allocator = SmithaySurfaceIdAllocatorProbe::new();

        assert_eq!(allocator.next_surface_id(), 1);
        assert_eq!(allocator.next_surface_id(), 2);
        assert_eq!(allocator.peek_next_surface_id(), 3);
    }

    /// 验证分配器可以从指定 surface ID 起点继续分配。
    #[test]
    fn smithay_surface_id_allocator_builds_from_custom_start() {
        let mut allocator = SmithaySurfaceIdAllocatorProbe::with_next_surface_id(42);

        assert_eq!(allocator.next_surface_id(), 42);
        assert_eq!(allocator.peek_next_surface_id(), 43);
    }
}
