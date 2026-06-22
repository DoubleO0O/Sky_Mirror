//! Linux-only controlled `xdg_wm_base` bind proof。
//!
//! 本模块只在受控 Unix endpoint 上证明 client 可以 bind 已初始化的 xdg-shell
//! global。它不创建 `wl_surface`、`xdg_surface` 或 `xdg_toplevel`，不触发 lifecycle、
//! admission ledger/core，也不提供 render/input 能力。

use std::{
    io,
    os::unix::net::UnixStream,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_compositor::WlCompositor, wl_registry::WlRegistry},
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

use super::{
    client_insert::NestedClientInsertCompileBoundary,
    client_session::{NestedClientSessionEvent, NestedClientSessionId},
    wayland_display::SmithayWaylandDisplayProbe,
};

const CONTROLLED_SESSION_ID: u64 = 54;
const CONTROLLED_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_PUMP_WAIT: Duration = Duration::from_millis(1);

/// Controlled `xdg_wm_base` bind proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledXdgWmBaseBindBlocker {
    /// Server 尚未显式初始化 xdg-shell global owner。
    MissingServerXdgShellGlobalOwner,
    /// Server 尚未显式初始化 `wl_compositor` owner。
    MissingServerWlCompositorOwner,
}

/// Controlled xdg bind proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledXdgWmBaseBindOperation {
    /// 创建受控 Unix endpoint pair。
    CreateControlledEndpoint,
    /// 把 server endpoint 插入既有 display。
    InsertServerClient,
    /// 创建 wayland-client connection。
    CreateClientConnection,
    /// 创建 event queue 并完成 registry discovery。
    InitializeRegistryQueue,
    /// Bind `wl_compositor`。
    BindWlCompositor,
    /// Bind `xdg_wm_base`。
    BindXdgWmBase,
    /// Bind 后执行同步 roundtrip。
    CompleteBindRoundtrip,
    /// 驱动 server request dispatch。
    DispatchServerClients,
    /// Flush server events。
    FlushServerClients,
}

/// Controlled `xdg_wm_base` bind proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledXdgWmBaseBindError {
    /// 前置 capability 尚未满足。
    Blocked(ControlledXdgWmBaseBindBlocker),
    /// 受控 session identity 无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败阶段。
        operation: ControlledXdgWmBaseBindOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败阶段。
        operation: ControlledXdgWmBaseBindOperation,
    },
    /// Server insertion 未产生预期 `NestedClientDataOwner` evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client proof thread 提前断开。
    ClientThreadDisconnected,
    /// Client proof thread panic。
    ClientThreadPanicked,
    /// 有界 proof 未在期限内完成。
    TimedOut,
}

/// Linux-only controlled `xdg_wm_base` bind proof 报告。
///
/// 成功只表示受控 client 绑定了 global。它不表示创建了 xdg surface/toplevel、
/// 触发 lifecycle 或拥有真实 xdg/compositor runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledXdgWmBaseBindReport {
    /// Server-side xdg-shell global owner 是否存在。
    pub server_xdg_shell_global_owner_available: bool,
    /// Server-side xdg-shell global 是否已显式初始化。
    pub server_xdg_shell_global_initialized: bool,
    /// Server-side `wl_compositor` owner 是否存在。
    pub server_wl_compositor_owner_available: bool,
    /// Server-side `wl_compositor` 是否已显式初始化。
    pub server_wl_compositor_initialized: bool,
    /// 是否创建受控 Unix endpoint pair。
    pub controlled_endpoint_created: bool,
    /// Server endpoint 是否通过既有 insertion seam 插入。
    pub server_client_inserted: bool,
    /// 是否创建 wayland-client connection。
    pub client_connection_created: bool,
    /// 是否创建 client event queue。
    pub event_queue_created: bool,
    /// Registry discovery/bind roundtrip 是否完成。
    pub registry_roundtrip_completed: bool,
    /// 是否尝试 bind registry globals。
    pub registry_bind_attempted: bool,
    /// Client 是否成功 bind `wl_compositor`。
    pub client_bound_wl_compositor: bool,
    /// Client 是否成功 bind `xdg_wm_base`。
    pub client_bound_xdg_wm_base: bool,
    /// 是否创建 `wl_surface`；本阶段固定为 false。
    pub wl_surface_created: bool,
    /// 是否分配 adapter surface identity；本阶段固定为 false。
    pub adapter_surface_identity_allocated: bool,
    /// 是否尝试创建 xdg surface；本阶段固定为 false。
    pub xdg_surface_create_attempted: bool,
    /// 是否创建 xdg surface；本阶段固定为 false。
    pub xdg_surface_created: bool,
    /// 是否尝试创建 xdg toplevel；本阶段固定为 false。
    pub xdg_toplevel_create_attempted: bool,
    /// 是否创建 xdg toplevel；本阶段固定为 false。
    pub xdg_toplevel_created: bool,
    /// 是否已有 xdg surface lifecycle。
    pub xdg_surface_lifecycle_available: bool,
    /// 是否已有 xdg toplevel lifecycle。
    pub xdg_toplevel_lifecycle_available: bool,
    /// 是否观察到 `new_toplevel` callback。
    pub new_toplevel_callback_observed: bool,
    /// 是否调用 admission ledger admit。
    pub ledger_admit_invoked: bool,
    /// 是否调用 admission ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// 是否调用 core register。
    pub core_register_invoked: bool,
    /// 是否调用 core detach。
    pub core_detach_invoked: bool,
    /// 是否分配 core `WindowId`。
    pub window_id_allocated: bool,
    /// 是否运行本 proof 的有界 protocol dispatch。
    pub protocol_dispatch_started: bool,
    /// Render 是否可用。
    pub render_support: bool,
    /// Input 是否可用。
    pub input_support: bool,
    /// 是否已有真实 compositor runtime。
    pub real_compositor_runtime_available: bool,
    /// 是否已有真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// 成功报告中未解决的 blockers。
    pub blockers: Vec<ControlledXdgWmBaseBindBlocker>,
}

#[derive(Debug, Default)]
struct ControlledXdgWmBaseClientState;

impl Dispatch<WlRegistry, GlobalListContents> for ControlledXdgWmBaseClientState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &GlobalListContents,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(ControlledXdgWmBaseClientState: ignore WlCompositor);
wayland_client::delegate_noop!(ControlledXdgWmBaseClientState: ignore XdgWmBase);

/// 在受控 endpoint 上证明 client 可以 bind `xdg_wm_base`。
///
/// 本函数不连接系统 Wayland session socket。52P 只允许 global bind：不得调用
/// `get_xdg_surface`/`get_toplevel`，不得触发 lifecycle、ledger/core 或 render/input。
pub fn controlled_xdg_wm_base_bind_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledXdgWmBaseBindReport, ControlledXdgWmBaseBindError> {
    if !server.is_xdg_shell_global_initialized() {
        return Err(ControlledXdgWmBaseBindError::Blocked(
            ControlledXdgWmBaseBindBlocker::MissingServerXdgShellGlobalOwner,
        ));
    }
    if !server.is_wl_compositor_global_initialized() {
        return Err(ControlledXdgWmBaseBindError::Blocked(
            ControlledXdgWmBaseBindBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| ControlledXdgWmBaseBindError::Io {
            operation: ControlledXdgWmBaseBindOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(ControlledXdgWmBaseBindError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| ControlledXdgWmBaseBindError::Io {
            operation: ControlledXdgWmBaseBindOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(ControlledXdgWmBaseBindError::MissingNestedClientDataOwnerEvidence);
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_controlled_xdg_wm_base_client(client_stream);
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server
            .dispatch_clients_once()
            .map_err(|error| ControlledXdgWmBaseBindError::Io {
                operation: ControlledXdgWmBaseBindOperation::DispatchServerClients,
                kind: error.kind(),
            })?;
        server
            .flush_clients_once()
            .map_err(|error| ControlledXdgWmBaseBindError::Io {
                operation: ControlledXdgWmBaseBindOperation::FlushServerClients,
                kind: error.kind(),
            })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(ControlledXdgWmBaseBindError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => Err(ControlledXdgWmBaseBindError::ClientThreadDisconnected),
                    Err(_) => Err(ControlledXdgWmBaseBindError::ClientThreadPanicked),
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| ControlledXdgWmBaseBindError::ClientThreadPanicked)?;
    client_result?;

    Ok(ControlledXdgWmBaseBindReport {
        server_xdg_shell_global_owner_available: true,
        server_xdg_shell_global_initialized: true,
        server_wl_compositor_owner_available: true,
        server_wl_compositor_initialized: true,
        controlled_endpoint_created: true,
        server_client_inserted: true,
        client_connection_created: true,
        event_queue_created: true,
        registry_roundtrip_completed: true,
        registry_bind_attempted: true,
        client_bound_wl_compositor: true,
        client_bound_xdg_wm_base: true,
        wl_surface_created: false,
        adapter_surface_identity_allocated: false,
        xdg_surface_create_attempted: false,
        xdg_surface_created: false,
        xdg_toplevel_create_attempted: false,
        xdg_toplevel_created: false,
        xdg_surface_lifecycle_available: false,
        xdg_toplevel_lifecycle_available: false,
        new_toplevel_callback_observed: false,
        ledger_admit_invoked: false,
        ledger_unmap_invoked: false,
        core_register_invoked: false,
        core_detach_invoked: false,
        window_id_allocated: false,
        protocol_dispatch_started: true,
        render_support: false,
        input_support: false,
        real_compositor_runtime_available: false,
        real_xdg_shell_runtime_available: false,
        blockers: Vec::new(),
    })
}

fn run_controlled_xdg_wm_base_client(
    client_stream: UnixStream,
) -> Result<(), ControlledXdgWmBaseBindError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        ControlledXdgWmBaseBindError::ClientProtocol {
            operation: ControlledXdgWmBaseBindOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) =
        registry_queue_init::<ControlledXdgWmBaseClientState>(&connection).map_err(|_| {
            ControlledXdgWmBaseBindError::ClientProtocol {
                operation: ControlledXdgWmBaseBindOperation::InitializeRegistryQueue,
            }
        })?;
    let queue_handle = event_queue.handle();
    let _compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| ControlledXdgWmBaseBindError::ClientProtocol {
            operation: ControlledXdgWmBaseBindOperation::BindWlCompositor,
        })?;
    let _xdg_wm_base = globals
        .bind::<XdgWmBase, _, _>(&queue_handle, 1..=7, ())
        .map_err(|_| ControlledXdgWmBaseBindError::ClientProtocol {
            operation: ControlledXdgWmBaseBindOperation::BindXdgWmBase,
        })?;

    // Bind 不会自动创建 xdg_surface/toplevel。本阶段也不主动发送 server ping，
    // 因而不把 ping/pong 冒充为已完成；这里只完成 bind request 的同步 roundtrip。
    let mut client_state = ControlledXdgWmBaseClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        ControlledXdgWmBaseBindError::ClientProtocol {
            operation: ControlledXdgWmBaseBindOperation::CompleteBindRoundtrip,
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledXdgWmBaseBindBlocker, ControlledXdgWmBaseBindError,
        controlled_xdg_wm_base_bind_report,
    };
    use crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe;

    #[test]
    fn controlled_xdg_wm_base_bind_requires_xdg_shell_global() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");

        assert_eq!(
            controlled_xdg_wm_base_bind_report(&mut server),
            Err(ControlledXdgWmBaseBindError::Blocked(
                ControlledXdgWmBaseBindBlocker::MissingServerXdgShellGlobalOwner,
            ))
        );
    }

    #[test]
    fn controlled_xdg_wm_base_bind_requires_wl_compositor_owner() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");

        assert_eq!(
            controlled_xdg_wm_base_bind_report(&mut server),
            Err(ControlledXdgWmBaseBindError::Blocked(
                ControlledXdgWmBaseBindBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    #[test]
    fn controlled_xdg_wm_base_bind_binds_global_without_shell_objects() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = controlled_xdg_wm_base_bind_report(&mut server)
            .expect("controlled xdg_wm_base bind proof 必须完成");

        assert!(report.server_xdg_shell_global_owner_available);
        assert!(report.server_xdg_shell_global_initialized);
        assert!(report.server_wl_compositor_owner_available);
        assert!(report.server_wl_compositor_initialized);
        assert!(report.controlled_endpoint_created);
        assert!(report.server_client_inserted);
        assert!(report.client_connection_created);
        assert!(report.event_queue_created);
        assert!(report.registry_roundtrip_completed);
        assert!(report.registry_bind_attempted);
        assert!(report.client_bound_wl_compositor);
        assert!(report.client_bound_xdg_wm_base);
        assert!(!report.wl_surface_created);
        assert!(!report.adapter_surface_identity_allocated);
        assert!(!report.xdg_surface_create_attempted);
        assert!(!report.xdg_surface_created);
        assert!(!report.xdg_toplevel_create_attempted);
        assert!(!report.xdg_toplevel_created);
        assert!(!report.xdg_surface_lifecycle_available);
        assert!(!report.xdg_toplevel_lifecycle_available);
        assert!(!report.new_toplevel_callback_observed);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.window_id_allocated);
        assert!(report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(report.blockers.is_empty());
    }
}
