//! Wayland client 的纯数据占位模型。
//!
//! 本模块不保存真实 Smithay client 对象，也不会把真实连接注册到 Wayland display。
//! 它只为未来 Wayland client 连接建立稳定的核心 ID 与 metadata 记录。
//!
//! `ClientId`、`SurfaceId`、`WindowId` 是不同层级：
//! `ClientId` 表示外部应用连接；`SurfaceId` 表示该 client 创建的 surface；
//! `WindowId` 表示 compositor 内部管理的逻辑窗口。socket 连接本身不等于
//! surface 或 window；ClientRegistry 本身不持有 surface 关系，归属绑定由
//! SurfaceRegistry 保存并由 State 统一协调生命周期。

/// Wayland client 的核心占位 ID。
///
/// `ClientId` 表示一个外部 Wayland client 连接，不等于 `SurfaceId`，
/// 也不等于 compositor 内部使用的 `WindowId`。
pub type ClientId = u64;

/// client 来源类型。
///
/// 当前只用于诊断和测试，不对应真实 Smithay client 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientKind {
    /// 纯测试或模拟 client。
    Mock,

    /// 未来通过 Wayland socket 接入的真实 client 占位。
    WaylandPlaceholder,
}

/// 单个 client 的纯数据记录。
///
/// `ClientRecord` 不保存真实 socket、连接流或 Smithay client，
/// 只记录核心可以稳定诊断的 metadata。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRecord {
    /// 稳定 client ID。
    pub id: ClientId,

    /// client 来源类型。
    pub kind: ClientKind,

    /// client 是否仍然存活。
    pub alive: bool,

    /// 可选显示名称。
    ///
    /// 真实 Wayland client 不一定有稳定名称；这里仅用于调试和测试。
    pub name: Option<String>,
}

/// client 注册表。
///
/// `ClientRegistry` 负责分配 `ClientId`，并记录 client 生命周期。
/// 未来真实 Smithay client 必须先通过后端边界转换为 `BackendEvent`，
/// 再由核心命令创建或关闭 `ClientRecord`。
#[derive(Debug, Clone)]
pub struct ClientRegistry {
    /// 下一次自动分配的 `ClientId`。
    next_id: ClientId,

    /// 当前已知 client 记录。
    clients: Vec<ClientRecord>,
}

impl ClientRegistry {
    /// 创建空 client 注册表。
    pub fn new() -> Self {
        Self {
            next_id: 1,
            clients: Vec::new(),
        }
    }

    /// 自动分配 ID 并注册 client。
    pub fn register_client(&mut self, kind: ClientKind, name: Option<String>) -> ClientId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);

        self.clients.push(ClientRecord {
            id,
            kind,
            alive: true,
            name,
        });

        id
    }

    /// 使用外部指定 ID 注册 client。
    ///
    /// 该方法为未来 Wayland display 已经持有稳定 client identity 的场景做准备。
    /// 如果指定 ID 已存在，则保留旧记录并返回 false。
    pub fn register_client_with_id(
        &mut self,
        client: ClientId,
        kind: ClientKind,
        name: Option<String>,
    ) -> bool {
        if self.get(client).is_some() {
            return false;
        }

        self.next_id = self.next_id.max(client.saturating_add(1));

        self.clients.push(ClientRecord {
            id: client,
            kind,
            alive: true,
            name,
        });

        true
    }

    /// 将 client 标记为 dead。
    ///
    /// Registry 层只修改 client 记录；需要级联关闭 surface 或 window 的调用方
    /// 必须通过 `State::close_client()`，避免局部注册表直接跨 seam 修改其他状态。
    pub fn mark_dead(&mut self, client: ClientId) -> bool {
        let Some(record) = self.clients.iter_mut().find(|record| record.id == client) else {
            return false;
        };

        record.alive = false;
        true
    }

    /// 判断 client 是否存在且仍然存活。
    pub fn is_alive(&self, client: ClientId) -> bool {
        self.get(client).map(|record| record.alive).unwrap_or(false)
    }

    /// 只读查找 client。
    pub fn get(&self, client: ClientId) -> Option<&ClientRecord> {
        self.clients.iter().find(|record| record.id == client)
    }

    /// 返回所有 client 记录。
    pub fn records(&self) -> &[ClientRecord] {
        &self.clients
    }

    /// 返回下一次将分配的 `ClientId`。
    pub fn next_id(&self) -> ClientId {
        self.next_id
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientKind, ClientRegistry};

    /// 验证自动注册 client 会分配 ID 并保存完整存活记录。
    #[test]
    fn client_registry_registers_client() {
        let mut registry = ClientRegistry::new();

        let client = registry.register_client(ClientKind::Mock, Some("测试 client".to_string()));
        let record = registry.get(client).expect("新 client 必须存在记录");

        // 首个 ClientId 必须从 1 开始，并推进下一次自动分配值。
        assert_eq!(client, 1);
        assert_eq!(registry.next_id(), 2);

        // 自动注册必须完整保留来源、名称和存活状态。
        assert_eq!(record.kind, ClientKind::Mock);
        assert_eq!(record.name.as_deref(), Some("测试 client"));
        assert!(record.alive);
        assert!(registry.is_alive(client));
    }

    /// 验证显式 ID 注册会保留外部 ID，并推进自动分配计数器。
    #[test]
    fn client_registry_register_client_with_id_uses_external_id() {
        let mut registry = ClientRegistry::new();

        assert!(registry.register_client_with_id(
            42,
            ClientKind::WaylandPlaceholder,
            Some("终端".to_string()),
        ));

        let record = registry.get(42).expect("显式 client ID 必须存在记录");

        // 显式 ID、来源和名称必须原样保留。
        assert_eq!(record.id, 42);
        assert_eq!(record.kind, ClientKind::WaylandPlaceholder);
        assert_eq!(record.name.as_deref(), Some("终端"));

        // 后续自动分配不能与外部指定 ID 冲突。
        assert!(registry.next_id() >= 43);
    }

    /// 验证重复显式 ID 会被拒绝，且不会覆盖已有记录。
    #[test]
    fn client_registry_rejects_duplicate_explicit_id() {
        let mut registry = ClientRegistry::new();

        assert!(registry.register_client_with_id(42, ClientKind::Mock, None));
        assert!(!registry.register_client_with_id(
            42,
            ClientKind::WaylandPlaceholder,
            Some("重复记录".to_string()),
        ));

        let record = registry.get(42).expect("首次注册记录必须继续存在");

        // 拒绝重复注册后，原始来源和名称必须保持不变。
        assert_eq!(record.kind, ClientKind::Mock);
        assert_eq!(record.name, None);
        assert_eq!(
            registry
                .records()
                .iter()
                .filter(|record| record.id == 42)
                .count(),
            1
        );
    }

    /// 验证关闭 client 只更新对应记录的存活状态。
    #[test]
    fn client_registry_mark_dead_updates_alive_flag() {
        let mut registry = ClientRegistry::new();
        let client = registry.register_client(ClientKind::Mock, None);

        assert!(registry.mark_dead(client));

        // 已关闭 client 必须保留诊断记录，但不再被视为存活。
        assert!(!registry.is_alive(client));
        assert!(!registry.get(client).expect("记录必须继续存在").alive);

        // 不存在的 ClientId 不得产生虚假成功结果。
        assert!(!registry.mark_dead(999));
    }
}
