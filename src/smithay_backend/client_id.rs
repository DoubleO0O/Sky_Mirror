//! Smithay client ID 分配器探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不接真实 Wayland client，不保存 `UnixStream`，不调用 `insert_client`，
//! 也不把 socket 插入 calloop。
//!
//! 它只负责为未来 socket accept 事件生成核心纯数据层使用的 `ClientId`，
//! 并构造 `SmithayClientConnectionDescriptor`。

use crate::{
    core::client::ClientId, smithay_backend::client_event::SmithayClientConnectionDescriptor,
};

/// Smithay client ID 分配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该分配器只服务于纯数据探针流程，
/// 不代表真实 client 已经进入 Wayland Display。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayClientIdAllocatorMode {
    /// 纯探针模式。
    ///
    /// 不保存 `UnixStream`，不调用 `DisplayHandle::insert_client`。
    ProbeOnly,
}

/// Smithay client ID 分配器探针。
///
/// 该结构只维护一个递增计数器，用于模拟未来真实 socket accept 后为 client
/// 分配稳定 `ClientId` 的过程。
///
/// 分配出的 `ClientId` 只属于核心纯数据模型，不等于真实 Wayland Display
/// 内部 client，也不表示连接已经完成协议注册。
pub struct SmithayClientIdAllocatorProbe {
    /// 下一次分配的 client ID。
    next_client_id: ClientId,

    /// 当前分配器模式。
    mode: SmithayClientIdAllocatorMode,
}

impl SmithayClientIdAllocatorProbe {
    /// 创建默认 client ID 分配器。
    ///
    /// 默认从 1 开始分配，保持与核心 `ClientRegistry` 的初始习惯一致。
    pub fn new() -> Self {
        Self {
            next_client_id: 1,
            mode: SmithayClientIdAllocatorMode::ProbeOnly,
        }
    }

    /// 使用指定起点创建 client ID 分配器。
    ///
    /// 该方法主要用于测试或未来从外部状态恢复分配器计数。
    pub fn with_next_client_id(next_client_id: ClientId) -> Self {
        Self {
            next_client_id,
            mode: SmithayClientIdAllocatorMode::ProbeOnly,
        }
    }

    /// 分配下一个 client ID。
    ///
    /// 这里只生成核心纯数据 ID，不注册真实 Wayland client，也不调用
    /// `insert_client`。
    pub fn next_client_id(&mut self) -> ClientId {
        let client = self.next_client_id;
        self.next_client_id = self.next_client_id.saturating_add(1);
        client
    }

    /// 构造下一个 client connection descriptor。
    ///
    /// 未来真实 socket accept 可以先通过该方法得到描述信息，再转换为
    /// `BackendEvent::ClientConnected`。构造 descriptor 本身不会修改核心状态。
    pub fn next_descriptor(&mut self, name: Option<String>) -> SmithayClientConnectionDescriptor {
        let client = self.next_client_id();

        SmithayClientConnectionDescriptor {
            client: Some(client),
            name,
        }
    }

    /// 返回下一次将分配的 client ID。
    pub fn peek_next_client_id(&self) -> ClientId {
        self.next_client_id
    }

    /// 返回当前分配器模式。
    pub fn mode(&self) -> SmithayClientIdAllocatorMode {
        self.mode
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only(&self) -> bool {
        self.mode == SmithayClientIdAllocatorMode::ProbeOnly
    }

    /// 返回当前阶段说明。
    pub fn mode_description(&self) -> &'static str {
        "smithay-client-id-allocator-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayClientIdAllocatorMode, SmithayClientIdAllocatorProbe};

    /// 验证默认分配器从核心约定的 client ID 1 开始。
    #[test]
    fn smithay_client_id_allocator_starts_at_one() {
        let allocator = SmithayClientIdAllocatorProbe::new();

        assert_eq!(allocator.peek_next_client_id(), 1);
        assert!(allocator.is_probe_only());
        assert_eq!(allocator.mode(), SmithayClientIdAllocatorMode::ProbeOnly);
        assert_eq!(
            allocator.mode_description(),
            "smithay-client-id-allocator-probe-only"
        );
    }

    /// 验证分配器会按顺序产生递增且稳定的核心 client ID。
    #[test]
    fn smithay_client_id_allocator_allocates_incrementing_ids() {
        let mut allocator = SmithayClientIdAllocatorProbe::new();

        assert_eq!(allocator.next_client_id(), 1);
        assert_eq!(allocator.next_client_id(), 2);
        assert_eq!(allocator.peek_next_client_id(), 3);
    }

    /// 验证分配器会把新 ID 和名称写入纯数据连接描述。
    #[test]
    fn smithay_client_id_allocator_builds_descriptor() {
        let mut allocator = SmithayClientIdAllocatorProbe::with_next_client_id(7);

        let descriptor = allocator.next_descriptor(Some("app".to_string()));

        assert_eq!(descriptor.client, Some(7));
        assert_eq!(descriptor.name, Some("app".to_string()));
        assert_eq!(allocator.peek_next_client_id(), 8);
    }
}
