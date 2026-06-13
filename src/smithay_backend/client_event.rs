//! Smithay client connection 事件适配探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不接真实 Wayland client，不保存 `UnixStream`，不调用 `insert_client`，
//! 也不把 socket 插入 calloop。
//!
//! 它只负责把未来 socket/client 连接描述转换为核心可理解的 `BackendEvent`。
//! client 连接只代表外部应用连接，不等于 surface 或 window。

use crate::core::{
    backend_event::BackendEvent,
    client::{ClientId, ClientKind},
};

/// Smithay client connection 适配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该模块只生成纯数据 `BackendEvent`，
/// 不处理或保存真实 Wayland client。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayClientConnectionMode {
    /// 纯探针模式。
    ///
    /// 不保存 `UnixStream`，不调用 `DisplayHandle::insert_client`。
    ProbeOnly,
}

/// client 连接描述信息。
///
/// 该结构不保存真实 socket 或 Smithay client，只保存未来可由 socket accept
/// 或 display 层提取出的最小诊断信息。真实 socket client 后续仍需要通过
/// Smithay/Wayland display 完成注册。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayClientConnectionDescriptor {
    /// 可选外部指定 client ID。
    ///
    /// `None` 表示由核心 `ClientRegistry` 自动分配。
    pub client: Option<ClientId>,

    /// 可选调试名称。
    ///
    /// 真实 Wayland client 不一定有稳定名称；这里仅作为调试 metadata。
    pub name: Option<String>,
}

impl SmithayClientConnectionDescriptor {
    /// 创建一个没有外部指定 ID 的 client 连接描述。
    pub fn anonymous() -> Self {
        Self {
            client: None,
            name: None,
        }
    }

    /// 创建一个带指定 ID 的 client 连接描述。
    pub fn with_client_id(client: ClientId) -> Self {
        Self {
            client: Some(client),
            name: None,
        }
    }

    /// 为描述信息添加调试名称。
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Smithay client connection 事件适配探针。
///
/// 该类型不持有状态，也不保存真实 client。它只把 client 连接描述转换成
/// `BackendEvent::ClientConnected`，不会直接修改核心 `State`。
pub struct SmithayClientConnectionProbe;

impl SmithayClientConnectionProbe {
    /// 返回当前适配器模式。
    pub fn mode() -> SmithayClientConnectionMode {
        SmithayClientConnectionMode::ProbeOnly
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only() -> bool {
        true
    }

    /// 把 client connection 描述转换成后端事件。
    ///
    /// 未来真实 Smithay socket accept 逻辑应先收集连接 metadata，再通过该路径
    /// 生成 `BackendEvent`，而不是直接修改核心 `State`。本方法不会调用
    /// `insert_client`，连接事件本身也不会创建 surface 或 window。
    pub fn client_connected_event(descriptor: SmithayClientConnectionDescriptor) -> BackendEvent {
        BackendEvent::ClientConnected {
            client: descriptor.client,
            kind: ClientKind::WaylandPlaceholder,
            name: descriptor.name,
        }
    }

    /// 生成 client 断开事件。
    ///
    /// 本方法只生成纯数据 `BackendEvent`，不接触真实 Wayland display；
    /// 后续生命周期级联仍由核心状态层处理。
    pub fn client_disconnected_event(client: ClientId) -> BackendEvent {
        BackendEvent::ClientDisconnected { client }
    }

    /// 返回当前阶段说明。
    pub fn mode_description() -> &'static str {
        "smithay-client-connection-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayClientConnectionDescriptor, SmithayClientConnectionMode,
        SmithayClientConnectionProbe,
    };
    use crate::core::{
        backend_event::BackendEvent,
        client::{ClientId, ClientKind},
    };

    /// 验证连接描述构造方法会正确保留可选 ID 和调试名称。
    #[test]
    fn client_connection_descriptor_builders_work() {
        let anonymous = SmithayClientConnectionDescriptor::anonymous();

        assert_eq!(anonymous.client, None);
        assert_eq!(anonymous.name, None);

        let identified = SmithayClientConnectionDescriptor::with_client_id(7).with_name("app");

        assert_eq!(identified.client, Some(7));
        assert_eq!(identified.name, Some("app".to_string()));
    }

    /// 验证连接适配器会生成 Wayland 占位 client 的纯数据连接事件。
    #[test]
    fn client_connection_probe_creates_connected_event() {
        let event = SmithayClientConnectionProbe::client_connected_event(
            SmithayClientConnectionDescriptor::with_client_id(7).with_name("app"),
        );

        assert_eq!(
            event,
            BackendEvent::ClientConnected {
                client: Some(7),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("app".to_string()),
            }
        );
        assert!(SmithayClientConnectionProbe::is_probe_only());
        assert_eq!(
            SmithayClientConnectionProbe::mode(),
            SmithayClientConnectionMode::ProbeOnly
        );
        assert_eq!(
            SmithayClientConnectionProbe::mode_description(),
            "smithay-client-connection-probe-only"
        );
    }

    /// 验证连接适配器会生成指定 client 的纯数据断开事件。
    #[test]
    fn client_connection_probe_creates_disconnected_event() {
        let client: ClientId = 7;
        let event = SmithayClientConnectionProbe::client_disconnected_event(client);

        assert_eq!(event, BackendEvent::ClientDisconnected { client: 7 });
    }
}
