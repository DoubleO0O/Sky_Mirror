//! Phase 51H-R / 51I inserted-client 的 Linux-only 编译验证边界。
//!
//! 本模块定义真实 `ClientData` owner，并把已取得的 `UnixStream` 交给
//! `DisplayHandle::insert_client`。它故意不拥有 listening socket、不启动 accept loop，
//! 也不直接修改 core。只有 `insert_client` 成功后才记录 `Connected`；backend 后续
//! 调用 `ClientData::disconnected` 时，owner 只记录纯数据 `Disconnected` 事件。
//!
//! 该边界能在 Linux 编译和测试中验证锁定版本 API，但在真实 socket callback 与
//! Linux runtime 测试接入前，不代表项目已经能够接受 client。

use std::{
    collections::VecDeque,
    io,
    os::unix::net::UnixStream,
    sync::{Arc, Mutex, MutexGuard},
};

use smithay::reexports::wayland_server::{
    Client, DisplayHandle,
    backend::{ClientData, ClientId, DisconnectReason},
};

use super::client_session::{NestedClientSessionEvent, NestedClientSessionId};

/// `ClientData` callback 与 runtime coordinator 之间的线程安全纯数据队列。
///
/// 队列只保存 adapter session event，不保存 core `State`、Smithay surface 或 window。
/// mutex poison 时保留已有事件并继续工作，避免 callback 因诊断队列失效而 panic。
#[derive(Debug, Clone, Default)]
pub struct NestedClientCallbackEventQueue {
    events: Arc<Mutex<VecDeque<NestedClientSessionEvent>>>,
}

impl NestedClientCallbackEventQueue {
    /// 创建空 callback event 队列。
    pub fn new() -> Self {
        Self::default()
    }

    /// 取出当前全部事件，并保持原有顺序。
    pub fn drain(&self) -> Vec<NestedClientSessionEvent> {
        self.lock_events().drain(..).collect()
    }

    /// 只取出当前全部 `Connected`，并保留其他 callback event 的原有顺序。
    ///
    /// Phase 51I-C 只允许真实 accept 进入连接注册 seam；`Disconnected` 必须留给
    /// Phase 51J-A 的真实 callback 验证，不能被连接 flow 顺手提交到 core close。
    pub fn drain_connected(&self) -> Vec<NestedClientSessionEvent> {
        let mut events = self.lock_events();
        let mut connected = Vec::new();
        let mut deferred = VecDeque::new();

        while let Some(event) = events.pop_front() {
            match event {
                NestedClientSessionEvent::Connected { .. } => connected.push(event),
                NestedClientSessionEvent::Disconnected { .. } => deferred.push_back(event),
            }
        }

        *events = deferred;
        connected
    }

    /// 只取出当前全部 `Disconnected`，并保留 `Connected` 的原有顺序。
    ///
    /// disconnect coordinator 必须先把 callback 事实转换为 session event，再交给
    /// core bridge；这里不能直接访问 `State`，也不能吞掉尚待注册的连接事件。
    pub fn drain_disconnected(&self) -> Vec<NestedClientSessionEvent> {
        let mut events = self.lock_events();
        let mut disconnected = Vec::new();
        let mut connected = VecDeque::new();

        while let Some(event) = events.pop_front() {
            match event {
                NestedClientSessionEvent::Connected { .. } => connected.push_back(event),
                NestedClientSessionEvent::Disconnected { .. } => disconnected.push(event),
            }
        }

        *events = connected;
        disconnected
    }

    /// 返回当前尚未被 runtime coordinator 消费的事件数。
    pub fn len(&self) -> usize {
        self.lock_events().len()
    }

    /// 当前队列是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // callback 只追加纯数据 event；消费与 core bridge 由外层 coordinator 负责。
    fn push(&self, event: NestedClientSessionEvent) {
        self.lock_events().push_back(event);
    }

    // poison 只代表先前持锁线程 panic，不应让 Wayland callback 再次 panic 并丢失队列。
    fn lock_events(&self) -> MutexGuard<'_, VecDeque<NestedClientSessionEvent>> {
        match self.events.lock() {
            Ok(events) => events,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// 与单个 inserted Wayland client 同生命周期的 callback owner。
///
/// owner 保留 adapter 分配的 [`NestedClientSessionId`]，并只向纯数据队列发布
/// disconnect 事件。它不持有 `&mut State`，因此 callback 不能绕过既有 bridge。
#[derive(Debug)]
pub struct NestedClientDataOwner {
    session: NestedClientSessionId,
    events: NestedClientCallbackEventQueue,
}

impl NestedClientDataOwner {
    /// 为指定 adapter session 创建 callback owner。
    pub fn new(session: NestedClientSessionId, events: NestedClientCallbackEventQueue) -> Self {
        Self { session, events }
    }

    /// 返回该 Wayland client 对应的 adapter session identity。
    pub fn session(&self) -> NestedClientSessionId {
        self.session
    }

    // owner 只知道 adapter session，不保存或猜测 core ClientId。
    fn record_disconnected(&self) {
        self.events.push(NestedClientSessionEvent::Disconnected {
            session: self.session,
        });
    }
}

impl ClientData for NestedClientDataOwner {
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        self.record_disconnected();
    }
}

/// 已取得 stream 到 inserted Wayland client 的最小编译验证边界。
///
/// 调用方必须在更外层提供真实 accepted stream；本类型没有 listening socket 或
/// calloop source。成功插入后，返回的 [`Client`] 可通过
/// [`nested_session_for_inserted_client`] 找回 owner 中的 session identity。
#[derive(Debug)]
pub struct NestedClientInsertCompileBoundary {
    display_handle: DisplayHandle,
    events: NestedClientCallbackEventQueue,
}

impl NestedClientInsertCompileBoundary {
    /// 使用已有 `DisplayHandle` 创建 inserted-client 编译验证边界。
    pub fn new(display_handle: DisplayHandle) -> Self {
        Self {
            display_handle,
            events: NestedClientCallbackEventQueue::new(),
        }
    }

    /// 把调用方提供的 stream 插入 Wayland display。
    ///
    /// 只有 `DisplayHandle::insert_client` 返回成功后才记录 `Connected`。失败时错误
    /// 原样返回，队列不会生成 ghost connected event，也不会触碰 core registry。
    ///
    /// # Errors
    ///
    /// 当锁定版本的 Wayland backend 拒绝 stream 时，返回其 `io::Error`。
    pub fn insert_client(
        &mut self,
        stream: UnixStream,
        session: NestedClientSessionId,
    ) -> io::Result<Client> {
        let owner = Arc::new(NestedClientDataOwner::new(session, self.events.clone()));
        let result = self.display_handle.insert_client(stream, owner);

        record_connected_after_insert(result, &self.events, session)
    }

    /// 返回 callback event 队列的共享句柄，供 runtime coordinator 后续 drain。
    pub fn event_queue(&self) -> NestedClientCallbackEventQueue {
        self.events.clone()
    }
}

/// 从本边界插入的 Wayland client 中读取 adapter session identity。
///
/// 其他 `ClientData` 类型插入的 client 返回 `None`；调用方不得把 backend `ClientId`
/// 数值强转为 [`NestedClientSessionId`]。
pub fn nested_session_for_inserted_client(client: &Client) -> Option<NestedClientSessionId> {
    client
        .get_data::<NestedClientDataOwner>()
        .map(NestedClientDataOwner::session)
}

// `?` 必须先确认 insertion 成功，再发布 connected event，避免制造 ghost core 输入。
fn record_connected_after_insert(
    result: io::Result<Client>,
    events: &NestedClientCallbackEventQueue,
    session: NestedClientSessionId,
) -> io::Result<Client> {
    let client = result?;
    events.push(NestedClientSessionEvent::Connected { session });
    Ok(client)
}

/// Phase 51H-R / 51I B 路线的编译边界能力报告。
///
/// 前三个字段只说明 Linux-only 源码定义了对应 API 边界；其余真实 runtime 能力
/// 在 accept callback 与 Linux runtime 测试完成前必须保持 `false`。
#[must_use = "编译边界报告不能被当作真实 client accept 证据"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedClientInsertCompileProofReport {
    /// 当前组合资源是否已有可取得的 `DisplayHandle` ownership 路径。
    pub display_handle_available: bool,

    /// 锁定版本 API 是否存在 `DisplayHandle::insert_client` 调用边界。
    pub insert_client_api_available: bool,

    /// 是否定义了实现 `ClientData` 并保存 adapter session 的 owner。
    pub client_data_owner_defined: bool,

    /// 是否已有真实 listening socket callback 把 accepted stream 送入本边界。
    pub real_accept_loop_available: bool,

    /// 是否已由 Linux runtime 测试观察到真实 socket accept 后的 client insertion。
    pub real_client_insert_observed: bool,

    /// 是否已由 Linux runtime 证明 inserted client 到 session 的可消费映射。
    pub inserted_client_mapping_available: bool,

    /// 是否已把真实 connected event 送入既有 core bridge。
    pub connected_event_bridged_to_core: bool,

    /// 是否已具备项目级真实 client accept 能力。
    pub accepts_clients: bool,

    /// 是否支持真实 surface；本阶段固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；本阶段固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实渲染；本阶段固定为 `false`。
    pub render_support: bool,

    /// 是否已启动真实 Wayland protocol dispatch；本阶段固定为 `false`。
    pub protocol_dispatch_started: bool,
}

impl NestedClientInsertCompileProofReport {
    /// 判断真实 accept + insert runtime proof 是否已经完成。
    pub fn is_runtime_ready(&self) -> bool {
        self.display_handle_available
            && self.insert_client_api_available
            && self.client_data_owner_defined
            && self.real_accept_loop_available
            && self.real_client_insert_observed
            && self.inserted_client_mapping_available
            && self.connected_event_bridged_to_core
            && self.accepts_clients
    }
}

/// 返回 B 路线当前的保守编译验证报告。
#[must_use = "调用方必须区分 compile boundary 与 real runtime proof"]
pub fn nested_client_insert_compile_proof_report() -> NestedClientInsertCompileProofReport {
    NestedClientInsertCompileProofReport {
        display_handle_available: true,
        insert_client_api_available: true,
        client_data_owner_defined: true,
        real_accept_loop_available: false,
        real_client_insert_observed: false,
        inserted_client_mapping_available: false,
        connected_event_bridged_to_core: false,
        accepts_clients: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        protocol_dispatch_started: false,
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use smithay::reexports::wayland_server::{Display, backend::ClientData};

    use super::{
        NestedClientCallbackEventQueue, NestedClientDataOwner, NestedClientInsertCompileBoundary,
        nested_client_insert_compile_proof_report, nested_session_for_inserted_client,
        record_connected_after_insert,
    };
    use crate::smithay_backend::client_session::{NestedClientSessionEvent, NestedClientSessionId};

    fn session(value: u64) -> NestedClientSessionId {
        NestedClientSessionId::new(value).expect("测试 session ID 必须非零")
    }

    /// 验证 callback owner 满足锁定版本 `ClientData` 的线程安全 trait 边界。
    #[test]
    fn client_data_owner_is_send_sync_client_data() {
        fn assert_owner<T: ClientData + Send + Sync>() {}

        assert_owner::<NestedClientDataOwner>();
    }

    /// 验证 owner 保留 adapter session，并只产生受控 disconnected event。
    #[test]
    fn client_data_owner_preserves_session_for_disconnect_event() {
        // Arrange
        let session = session(51);
        let events = NestedClientCallbackEventQueue::new();
        let owner = NestedClientDataOwner::new(session, events.clone());

        // Act：直接调用内部记录 helper；这不冒充真实 backend callback。
        owner.record_disconnected();

        // Assert
        assert_eq!(owner.session(), session);
        assert_eq!(
            events.drain(),
            vec![NestedClientSessionEvent::Disconnected { session }]
        );
    }

    /// 验证 connected-only drain 不会提前消费 Phase 51J-A 的 disconnect callback。
    #[test]
    fn connected_only_drain_preserves_disconnected_events() {
        // Arrange
        let connected_session = session(54);
        let disconnected_session = session(55);
        let events = NestedClientCallbackEventQueue::new();
        events.push(NestedClientSessionEvent::Disconnected {
            session: disconnected_session,
        });
        events.push(NestedClientSessionEvent::Connected {
            session: connected_session,
        });

        // Act
        let connected = events.drain_connected();

        // Assert
        assert_eq!(
            connected,
            vec![NestedClientSessionEvent::Connected {
                session: connected_session
            }]
        );
        assert_eq!(
            events.drain(),
            vec![NestedClientSessionEvent::Disconnected {
                session: disconnected_session
            }]
        );
    }

    /// 验证 disconnected-only drain 不会消费仍待 connected coordinator 注册的事件。
    #[test]
    fn disconnected_only_drain_preserves_connected_events() {
        // Arrange
        let connected_session = session(56);
        let disconnected_session = session(57);
        let events = NestedClientCallbackEventQueue::new();
        events.push(NestedClientSessionEvent::Connected {
            session: connected_session,
        });
        events.push(NestedClientSessionEvent::Disconnected {
            session: disconnected_session,
        });

        // Act
        let disconnected = events.drain_disconnected();

        // Assert
        assert_eq!(
            disconnected,
            vec![NestedClientSessionEvent::Disconnected {
                session: disconnected_session
            }]
        );
        assert_eq!(
            events.drain(),
            vec![NestedClientSessionEvent::Connected {
                session: connected_session
            }]
        );
    }

    /// 验证 insertion error 不会发布 connected event 或制造后续 core 输入。
    #[test]
    fn failed_insert_result_does_not_publish_connected_event() {
        // Arrange
        let session = session(52);
        let events = NestedClientCallbackEventQueue::new();
        let failure = Err(io::Error::other("controlled insert failure"));

        // Act
        let result = record_connected_after_insert(failure, &events, session);

        // Assert
        assert!(result.is_err());
        assert!(events.is_empty());
    }

    /// 验证锁定版本 API 可插入 `UnixStream` 并从真实 ClientData 找回 session。
    #[test]
    fn insert_boundary_calls_display_handle_and_preserves_session_owner() {
        // Arrange
        let display = Display::<()>::new().expect("Wayland Display 必须能构造");
        let mut boundary = NestedClientInsertCompileBoundary::new(display.handle());
        let (server_stream, _client_stream) =
            std::os::unix::net::UnixStream::pair().expect("UnixStream pair 必须能构造");
        let session = session(53);

        // Act
        let client = boundary
            .insert_client(server_stream, session)
            .expect("锁定版本 insert_client API 必须接受 UnixStream 与 ClientData");

        // Assert
        assert_eq!(nested_session_for_inserted_client(&client), Some(session));
        assert_eq!(
            boundary.event_queue().drain(),
            vec![NestedClientSessionEvent::Connected { session }]
        );
    }

    /// 验证 B 路线只声明编译边界，不提升真实 accept 或 surface/render 能力。
    #[test]
    fn compile_proof_report_keeps_runtime_capabilities_false() {
        let report = nested_client_insert_compile_proof_report();

        assert!(report.display_handle_available);
        assert!(report.insert_client_api_available);
        assert!(report.client_data_owner_defined);
        assert!(!report.real_accept_loop_available);
        assert!(!report.real_client_insert_observed);
        assert!(!report.inserted_client_mapping_available);
        assert!(!report.connected_event_bridged_to_core);
        assert!(!report.accepts_clients);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.is_runtime_ready());
    }
}
