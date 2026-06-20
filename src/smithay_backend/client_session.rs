//! Nested client session 的跨平台纯数据边界。
//!
//! 本模块只描述 adapter 观察到的 session 身份及其与核心 client 身份的关系。
//! 它不持有平台后端对象，也不负责把 session 事件提交到核心状态机。
//! Phase 51D 只在这条 seam 产生并记录纯数据事件；Phase 51E 才会消费事件，
//! 协调核心 registry、session mapping 和 disconnect bridge。

use std::{
    collections::{HashMap, hash_map::Entry},
    num::NonZeroU64,
};

use crate::core::client::ClientId;

/// adapter 层观察到的一次 nested client session 身份。
///
/// 该 ID 只在 adapter session 生命周期内有效，不能等同于核心层的 `ClientId`，
/// 也不能等同于未来平台后端内部的 client 句柄。私有的 `NonZeroU64` 字段既阻止
/// 调用方直接构造，也避免用零同时表达“尚未分配”和“有效 session”。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NestedClientSessionId(NonZeroU64);

impl NestedClientSessionId {
    /// 从明确的非零数值创建 adapter session ID。
    ///
    /// 零表示调用方尚未分配有效身份，因此返回 `None`，不会把无效占位值带入映射。
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// 返回 adapter session ID 的底层非零数值，供日志和跨 callback 关联使用。
    pub const fn value(self) -> u64 {
        self.0.get()
    }
}

/// adapter 拒绝 nested client session 的纯数据原因。
///
/// 当前只表达“本阶段不支持”这一保守事实，不提升任何真实 client capability。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedClientSessionRejectionReason {
    /// 当前 adapter/runtime 尚不支持处理该 session。
    Unsupported,
}

/// adapter 层观察到的 nested client session 生命周期事件。
///
/// 事件只携带 adapter session ID，不携带核心 `ClientId`，也不自动转换为核心事件。
/// 后续 coordinator 必须先完成显式映射，才能安全处理断开事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedClientSessionEvent {
    /// adapter 观察到一个新的 session。
    Connected {
        /// 本次连接在 adapter 层的唯一身份。
        session: NestedClientSessionId,
    },

    /// adapter 观察到一个既有 session 断开。
    Disconnected {
        /// 本次断开引用的 adapter session 身份。
        session: NestedClientSessionId,
    },
}

/// event record 中稳定、可筛选的 session 事件类别。
///
/// 该枚举与 Phase 51C 的 lifecycle event 分离，因此增加拒绝诊断不会改变旧 enum
/// 的 variants，也不会破坏已有 exhaustive match。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedClientSessionEventKind {
    /// adapter 观察到 session 连接。
    Connected,

    /// adapter 观察到 session 断开。
    Disconnected,

    /// adapter 拒绝或暂不支持 session。
    Rejected,
}

/// 一条带稳定顺序和可选诊断信息的 session 事件记录。
///
/// Record 只包含 ID、枚举、整数与字符串等纯数据，adapter 可以安全地把它交给
/// 后续协调层。Phase 51D 不在这里读取或修改核心状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedClientSessionEventRecord {
    /// adapter 观察事件的单调递增顺序，从 1 开始。
    pub sequence: u64,

    /// 本条记录的稳定事件类别。
    pub kind: NestedClientSessionEventKind,

    /// adapter session 身份；拒绝发生在分配 ID 之前时允许为空。
    pub session: Option<NestedClientSessionId>,

    /// 拒绝事件的结构化原因；连接和断开事件固定为空。
    pub rejection_reason: Option<NestedClientSessionRejectionReason>,

    /// adapter 可选提供的 session/client 显示标签，仅用于诊断。
    pub label: Option<String>,

    /// adapter 可选提供的诊断文本，不用于驱动核心状态转换。
    pub diagnostic: Option<String>,
}

/// 按 adapter 观察顺序保存 session 事件的纯数据 recorder。
///
/// Log 是 Phase 51D 的 producer seam：调用方只提交纯数据事件，log 统一分配
/// sequence 并保留顺序。它不持有核心 registry 或 mapping；Phase 51E 才会在
/// 独立 coordinator 中消费这些记录并处理核心生命周期。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedClientSessionEventLog {
    next_sequence: u64,
    records: Vec<NestedClientSessionEventRecord>,
}

impl NestedClientSessionEventLog {
    /// 创建空 event log，首条记录的 sequence 为 1。
    pub fn new() -> Self {
        Self {
            next_sequence: 1,
            records: Vec::new(),
        }
    }

    /// 记录一条纯数据 session 事件，并返回刚写入的记录。
    ///
    /// 本方法只追加本地诊断记录，不提交核心事件、不注册 client，也不修改
    /// Phase 51C 的 session-to-client mapping。
    pub fn record(
        &mut self,
        event: NestedClientSessionEvent,
        label: Option<String>,
        diagnostic: Option<String>,
    ) -> &NestedClientSessionEventRecord {
        let (kind, session) = match event {
            NestedClientSessionEvent::Connected { session } => {
                (NestedClientSessionEventKind::Connected, session)
            }
            NestedClientSessionEvent::Disconnected { session } => {
                (NestedClientSessionEventKind::Disconnected, session)
            }
        };

        self.push_record(kind, Some(session), None, label, diagnostic)
    }

    /// 记录一条纯数据拒绝事件，并返回刚写入的记录。
    ///
    /// 拒绝发生在 adapter session ID 分配前时，`session` 可以为 `None`。
    /// 本方法只记录 unsupported 事实，不改变 Linux client capability。
    pub fn record_rejected(
        &mut self,
        session: Option<NestedClientSessionId>,
        reason: NestedClientSessionRejectionReason,
        label: Option<String>,
        diagnostic: Option<String>,
    ) -> &NestedClientSessionEventRecord {
        self.push_record(
            NestedClientSessionEventKind::Rejected,
            session,
            Some(reason),
            label,
            diagnostic,
        )
    }

    /// 在唯一位置分配 sequence 并追加标准化 event record。
    fn push_record(
        &mut self,
        kind: NestedClientSessionEventKind,
        session: Option<NestedClientSessionId>,
        rejection_reason: Option<NestedClientSessionRejectionReason>,
        label: Option<String>,
        diagnostic: Option<String>,
    ) -> &NestedClientSessionEventRecord {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);

        // Vec 只在末尾追加，确保记录顺序与 adapter 观察顺序完全一致。
        let record_index = self.records.len();
        self.records.push(NestedClientSessionEventRecord {
            sequence,
            kind,
            session,
            rejection_reason,
            label,
            diagnostic,
        });

        // record_index 在 push 前取自 len，因此必然指向刚追加的记录。
        &self.records[record_index]
    }

    /// 返回按 sequence 排列的全部 event records。
    pub fn records(&self) -> &[NestedClientSessionEventRecord] {
        &self.records
    }

    /// 返回当前已记录的 session 事件数量。
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 判断当前是否尚未记录任何 session 事件。
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl Default for NestedClientSessionEventLog {
    fn default() -> Self {
        Self::new()
    }
}

/// adapter session 映射操作的结构化失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedClientSessionError {
    /// 目标 session 已经绑定核心 client，不能被第二次覆盖。
    AlreadyBound {
        /// 发生重复绑定的 adapter session。
        session: NestedClientSessionId,
    },
}

/// 当前活跃 adapter session 到核心 `ClientId` 的映射容器。
///
/// 两种 ID 通过显式键值关系关联，禁止用数值强转把 session 当成核心 client。
/// 容器只保存纯数据 ID；真实平台 client 的所有权和生命周期必须留在后续
/// Linux-only adapter 中，不能进入这个跨平台边界。
#[derive(Debug, Clone, Default)]
pub struct NestedClientSessionRegistry {
    clients: HashMap<NestedClientSessionId, ClientId>,
}

impl NestedClientSessionRegistry {
    /// 创建不含任何活跃 session 映射的 registry。
    pub fn new() -> Self {
        Self::default()
    }

    /// 将 adapter session 绑定到一个核心 `ClientId`。
    ///
    /// # Errors
    ///
    /// session 已经存在映射时返回 [`NestedClientSessionError::AlreadyBound`]。
    pub fn bind(
        &mut self,
        session: NestedClientSessionId,
        client: ClientId,
    ) -> Result<(), NestedClientSessionError> {
        match self.clients.entry(session) {
            // 已存在的 session 必须保留首次映射，避免重复连接制造第二个核心 client。
            Entry::Occupied(_) => Err(NestedClientSessionError::AlreadyBound { session }),
            // 只有首次观察到的 session 才能写入 active mapping。
            Entry::Vacant(entry) => {
                entry.insert(client);
                Ok(())
            }
        }
    }

    /// 查询 adapter session 当前绑定的核心 `ClientId`。
    ///
    /// 未知或已经移除的 session 返回 `None`，调用方不得据此猜测或伪造核心 ID。
    pub fn lookup(&self, session: NestedClientSessionId) -> Option<ClientId> {
        self.clients.get(&session).copied()
    }

    /// 移除 adapter session 的 active mapping，并返回原核心 `ClientId`。
    ///
    /// 未知 session 返回 `None`，因此 disconnect 调用方无法凭空构造核心 client。
    /// 移除后允许重新 bind 同一数值；本容器不负责保存历史 tombstone。
    pub fn remove(&mut self, session: NestedClientSessionId) -> Option<ClientId> {
        self.clients.remove(&session)
    }

    /// 判断 adapter session 当前是否存在活跃映射。
    pub fn contains_session(&self, session: NestedClientSessionId) -> bool {
        self.clients.contains_key(&session)
    }

    /// 返回当前活跃 session 映射数量。
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// 判断当前是否没有任何活跃 session 映射。
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NestedClientSessionError, NestedClientSessionEvent, NestedClientSessionEventKind,
        NestedClientSessionEventLog, NestedClientSessionEventRecord, NestedClientSessionId,
        NestedClientSessionRegistry, NestedClientSessionRejectionReason,
    };
    use crate::core::client::ClientId;

    /// 验证 session ID 会保留调用方明确提供的非零身份值。
    #[test]
    fn session_id_preserves_explicit_nonzero_value() {
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");

        assert_eq!(session.value(), 7);
    }

    /// 验证零值不能被误当成已经分配的 adapter session 身份。
    #[test]
    fn session_id_rejects_zero() {
        assert_eq!(NestedClientSessionId::new(0), None);
    }

    /// 验证连接和断开事件具有稳定的值语义与可读调试输出。
    #[test]
    fn session_events_support_clone_compare_and_debug() {
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let connected = NestedClientSessionEvent::Connected { session };
        let disconnected = NestedClientSessionEvent::Disconnected { session };

        assert_eq!(connected.clone(), connected);
        assert_ne!(connected, disconnected);
        assert!(format!("{connected:?}").contains("Connected"));
        assert!(format!("{disconnected:?}").contains("Disconnected"));
    }

    /// 验证纯数据 log 可以记录连接事件及 adapter 可见的诊断字段。
    #[test]
    fn event_log_records_connected_event() {
        let mut log = NestedClientSessionEventLog::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let event = NestedClientSessionEvent::Connected { session };

        let record = log.record(
            event,
            Some("nested-terminal".to_string()),
            Some("session connected".to_string()),
        );

        assert_eq!(record.sequence, 1);
        assert_eq!(record.kind, NestedClientSessionEventKind::Connected);
        assert_eq!(record.session, Some(session));
        assert_eq!(record.rejection_reason, None);
        assert_eq!(record.label.as_deref(), Some("nested-terminal"));
        assert_eq!(record.diagnostic.as_deref(), Some("session connected"));
    }

    /// 验证纯数据 log 可以记录断开事件，而不触发任何核心状态变更。
    #[test]
    fn event_log_records_disconnected_event() {
        let mut log = NestedClientSessionEventLog::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let event = NestedClientSessionEvent::Disconnected { session };

        let record = log.record(event, None, Some("peer closed".to_string()));

        assert_eq!(record.sequence, 1);
        assert_eq!(record.kind, NestedClientSessionEventKind::Disconnected);
        assert_eq!(record.session, Some(session));
        assert_eq!(record.rejection_reason, None);
        assert_eq!(record.label, None);
        assert_eq!(record.diagnostic.as_deref(), Some("peer closed"));
    }

    /// 验证 log 按观察顺序保存事件，并分配单调递增的 sequence。
    #[test]
    fn event_log_preserves_order_and_monotonic_sequence() {
        let mut log = NestedClientSessionEventLog::new();
        let first_session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let second_session = NestedClientSessionId::new(8).expect("非零 session ID 必须有效");
        let first = NestedClientSessionEvent::Connected {
            session: first_session,
        };
        let second = NestedClientSessionEvent::Disconnected {
            session: second_session,
        };

        log.record(first, None, None);
        log.record(second, None, None);

        assert_eq!(log.len(), 2);
        assert!(!log.is_empty());
        assert_eq!(log.records()[0].sequence, 1);
        assert_eq!(
            log.records()[0].kind,
            NestedClientSessionEventKind::Connected
        );
        assert_eq!(log.records()[0].session, Some(first_session));
        assert_eq!(log.records()[1].sequence, 2);
        assert_eq!(
            log.records()[1].kind,
            NestedClientSessionEventKind::Disconnected
        );
        assert_eq!(log.records()[1].session, Some(second_session));
    }

    /// 验证 event record 是可克隆、可比较的纯数据诊断值。
    #[test]
    fn event_record_supports_clone_compare_and_debug() {
        let mut log = NestedClientSessionEventLog::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let record: NestedClientSessionEventRecord = log
            .record(
                NestedClientSessionEvent::Connected { session },
                Some("nested-client".to_string()),
                None,
            )
            .clone();

        assert_eq!(record.clone(), record);
        assert!(format!("{record:?}").contains("NestedClientSessionEventRecord"));
    }

    /// 验证 adapter 可以记录尚未取得 session ID 的 unsupported 拒绝结果。
    #[test]
    fn event_log_records_rejected_session() {
        let mut log = NestedClientSessionEventLog::new();
        let reason = NestedClientSessionRejectionReason::Unsupported;

        let record = log.record_rejected(
            None,
            reason,
            Some("nested-client".to_string()),
            Some("client sessions remain unsupported".to_string()),
        );

        assert_eq!(record.sequence, 1);
        assert_eq!(record.kind, NestedClientSessionEventKind::Rejected);
        assert_eq!(record.session, None);
        assert_eq!(record.rejection_reason, Some(reason));
        assert_eq!(record.label.as_deref(), Some("nested-client"));
        assert_eq!(
            record.diagnostic.as_deref(),
            Some("client sessions remain unsupported")
        );
    }

    /// 验证新 registry 不包含任何 adapter session 映射。
    #[test]
    fn new_registry_is_empty() {
        let registry = NestedClientSessionRegistry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    /// 验证首次 bind 会保存 session 到核心 client 的显式映射。
    #[test]
    fn bind_and_lookup_known_session_returns_core_client() {
        let mut registry = NestedClientSessionRegistry::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let client: ClientId = 42;

        assert_eq!(registry.bind(session, client), Ok(()));
        assert_eq!(registry.lookup(session), Some(client));
        assert!(registry.contains_session(session));
        assert_eq!(registry.len(), 1);
    }

    /// 验证未知 session 查询不会制造不存在的核心 client 身份。
    #[test]
    fn lookup_unknown_session_returns_none() {
        let registry = NestedClientSessionRegistry::new();
        let unknown = NestedClientSessionId::new(99).expect("非零 session ID 必须有效");

        assert_eq!(registry.lookup(unknown), None);
        assert!(!registry.contains_session(unknown));
    }

    /// 验证重复 bind 被拒绝，且首次建立的核心 client 映射不会被覆盖。
    #[test]
    fn duplicate_bind_is_rejected_without_replacing_original_client() {
        let mut registry = NestedClientSessionRegistry::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");

        assert_eq!(registry.bind(session, 42), Ok(()));
        assert_eq!(
            registry.bind(session, 84),
            Err(NestedClientSessionError::AlreadyBound { session })
        );
        assert_eq!(registry.lookup(session), Some(42));
        assert_eq!(registry.len(), 1);
    }

    /// 验证移除已知 session 会返回核心 client，并清除 active mapping。
    #[test]
    fn remove_known_session_returns_client_and_clears_mapping() {
        let mut registry = NestedClientSessionRegistry::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        registry.bind(session, 42).expect("首次 bind 必须成功");

        assert_eq!(registry.remove(session), Some(42));
        assert_eq!(registry.lookup(session), None);
        assert!(!registry.contains_session(session));
        assert!(registry.is_empty());
    }

    /// 验证未知断开只返回 None，不移除其他映射或伪造核心 client。
    #[test]
    fn remove_unknown_session_returns_none_without_disturbing_known_mapping() {
        let mut registry = NestedClientSessionRegistry::new();
        let known = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let unknown = NestedClientSessionId::new(99).expect("非零 session ID 必须有效");
        registry.bind(known, 42).expect("首次 bind 必须成功");

        assert_eq!(registry.remove(unknown), None);
        assert_eq!(registry.lookup(known), Some(42));
        assert_eq!(registry.len(), 1);
    }

    /// 验证 remove 后允许同一 session ID 建立新的 active mapping。
    ///
    /// registry 不保存历史 tombstone；历史 client 生命周期仍由核心层负责记录。
    #[test]
    fn removed_session_can_be_bound_again() {
        let mut registry = NestedClientSessionRegistry::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        registry.bind(session, 42).expect("首次 bind 必须成功");
        assert_eq!(registry.remove(session), Some(42));

        assert_eq!(registry.bind(session, 84), Ok(()));
        assert_eq!(registry.lookup(session), Some(84));
    }

    /// 验证 production source 没有越过纯数据边界引入平台协议或连接操作。
    #[test]
    fn production_source_remains_platform_object_free() {
        let source = include_str!("client_session.rs");
        let forbidden_tokens = [
            ["smithay", "::"].concat(),
            ["wayland", "_server"].concat(),
            ["Display", "Handle"].concat(),
            ["Listening", "SocketSource"].concat(),
            ["Unix", "Stream"].concat(),
            ["Wl", "Surface"].concat(),
            ["xdg", "_toplevel"].concat(),
            ["insert", "_client"].concat(),
            ["delegate", "_"].concat(),
            ["impl ", "Dispatch"].concat(),
            ["impl ", "GlobalDispatch"].concat(),
            ["ac", "cept"].concat(),
            ["Backend", "Event"].concat(),
            ["Core", "Command"].concat(),
        ];

        for forbidden in forbidden_tokens {
            assert!(
                !source.contains(&forbidden),
                "client session 纯数据边界包含禁止 token: {forbidden}"
            );
        }
    }
}
