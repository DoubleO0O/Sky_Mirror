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

use super::{
    client_insert::NestedClientInsertCompileBoundary,
    client_session::{NestedClientSessionEvent, NestedClientSessionId},
    wayland_display::SmithayWaylandDisplayProbe,
};

const CONTROLLED_SESSION_ID: u64 = 52;
const CONTROLLED_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_PUMP_WAIT: Duration = Duration::from_millis(1);

/// Controlled `wl_compositor` bind proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlCompositorBindBlocker {
    /// Server 尚未显式初始化 `wl_compositor` global owner。
    MissingServerWlCompositorOwner,
}

/// Controlled bind proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlCompositorBindOperation {
    /// 创建受控 Unix stream pair。
    CreateControlledEndpoint,
    /// 把 server endpoint 插入既有 display。
    InsertServerClient,
    /// 创建 wayland-client connection。
    CreateClientConnection,
    /// 创建 event queue 并完成初始 registry roundtrip。
    InitializeRegistryQueue,
    /// 只 bind `wl_compositor`。
    BindWlCompositor,
    /// bind 后执行同步 roundtrip。
    CompleteBindRoundtrip,
    /// 驱动 server request dispatch。
    DispatchServerClients,
    /// flush server events。
    FlushServerClients,
}

/// Controlled bind proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlCompositorBindError {
    /// 前置 capability 尚未满足。
    Blocked(ControlledWlCompositorBindBlocker),
    /// 受控 session 常量无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败发生的操作。
        operation: ControlledWlCompositorBindOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败发生的 client 操作。
        operation: ControlledWlCompositorBindOperation,
    },
    /// Server insertion 没有产生预期的 `NestedClientDataOwner` connected evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client proof thread 在发送结果前退出。
    ClientThreadDisconnected,
    /// Client proof thread 发生 panic。
    ClientThreadPanicked,
    /// 有界 server/client driver 未在期限内完成。
    TimedOut,
}

/// Linux-only controlled client `wl_compositor` bind proof 报告。
///
/// 报告只证明受控 endpoint 上的 connection、registry 与 compositor bind。它不
/// 表示创建了 `wl_surface`、进入 xdg lifecycle、触发 core admission，或已有
/// 可长期运行的 compositor。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledWlCompositorBindReport {
    /// Server-side `wl_compositor` owner 是否存在。
    pub server_wl_compositor_owner_available: bool,
    /// Server-side `wl_compositor` 是否已显式初始化。
    pub server_wl_compositor_initialized: bool,
    /// Inserted client 是否持有独立 `CompositorClientState`。
    pub per_client_compositor_state_available: bool,
    /// 是否创建了受控 Unix endpoint pair。
    pub controlled_endpoint_created: bool,
    /// Server endpoint 是否通过现有 insertion seam 插入。
    pub server_client_inserted: bool,
    /// 是否创建了 wayland-client connection。
    pub client_connection_created: bool,
    /// 是否创建了 client event queue。
    pub event_queue_created: bool,
    /// 是否开始 registry roundtrip。
    pub registry_roundtrip_started: bool,
    /// registry roundtrip 是否完成。
    pub registry_roundtrip_completed: bool,
    /// 是否尝试 bind registry 中的 `wl_compositor`。
    pub registry_bind_attempted: bool,
    /// Client 是否成功 bind `wl_compositor`。
    pub client_bound_wl_compositor: bool,
    /// Client 是否 bind `xdg_wm_base`；本阶段固定为 false。
    pub client_bound_xdg_wm_base: bool,
    /// 是否创建了 `wl_surface`；本阶段固定为 false。
    pub wl_surface_created: bool,
    /// 是否形成受控 client harness。
    pub client_harness_available: bool,
    /// 是否存在 `xdg_surface` lifecycle。
    pub xdg_surface_lifecycle_available: bool,
    /// 是否存在 `xdg_toplevel` lifecycle。
    pub xdg_toplevel_lifecycle_available: bool,
    /// 是否调用 admission ledger admit。
    pub ledger_admit_invoked: bool,
    /// 是否调用 admission ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// 是否调用 core register。
    pub core_register_invoked: bool,
    /// 是否调用 core detach。
    pub core_detach_invoked: bool,
    /// 是否驱动了本次受控 proof 所需的最小 protocol dispatch。
    pub protocol_dispatch_started: bool,
    /// 是否已有真实 compositor runtime；本阶段固定为 false。
    pub real_compositor_runtime_available: bool,
    /// 是否已有真实 xdg-shell runtime；本阶段固定为 false。
    pub real_xdg_shell_runtime_available: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 成功报告中没有剩余的 bind-proof blocker。
    pub blockers: Vec<ControlledWlCompositorBindBlocker>,
}

#[derive(Debug, Default)]
struct ControlledClientState;

impl Dispatch<WlRegistry, GlobalListContents> for ControlledClientState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &GlobalListContents,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // Registry 只用于发现 wl_compositor；动态 global 变化不属于本次有界 proof。
    }
}

wayland_client::delegate_noop!(ControlledClientState: ignore WlCompositor);

/// 在受控 Unix endpoint 上证明 client 可以 bind server 的 `wl_compositor`。
///
/// Phase 52N 才允许创建 `Connection`，且只能使用本函数内部的 stream pair；
/// 不读取真实系统 session socket。Registry 只 bind `wl_compositor`，不会 bind
/// xdg-shell global、创建 surface，或进入 ledger/core/render/input。
pub fn controlled_wl_compositor_bind_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledWlCompositorBindReport, ControlledWlCompositorBindError> {
    if !server.is_wl_compositor_global_initialized() {
        return Err(ControlledWlCompositorBindError::Blocked(
            ControlledWlCompositorBindBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| ControlledWlCompositorBindError::Io {
            operation: ControlledWlCompositorBindOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(ControlledWlCompositorBindError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| ControlledWlCompositorBindError::Io {
            operation: ControlledWlCompositorBindOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(ControlledWlCompositorBindError::MissingNestedClientDataOwnerEvidence);
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_controlled_client(client_stream);
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server
            .dispatch_clients_once()
            .map_err(|error| ControlledWlCompositorBindError::Io {
                operation: ControlledWlCompositorBindOperation::DispatchServerClients,
                kind: error.kind(),
            })?;
        server
            .flush_clients_once()
            .map_err(|error| ControlledWlCompositorBindError::Io {
                operation: ControlledWlCompositorBindOperation::FlushServerClients,
                kind: error.kind(),
            })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(ControlledWlCompositorBindError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => Err(ControlledWlCompositorBindError::ClientThreadDisconnected),
                    Err(_) => Err(ControlledWlCompositorBindError::ClientThreadPanicked),
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| ControlledWlCompositorBindError::ClientThreadPanicked)?;
    client_result?;

    Ok(ControlledWlCompositorBindReport {
        server_wl_compositor_owner_available: true,
        server_wl_compositor_initialized: true,
        per_client_compositor_state_available: true,
        controlled_endpoint_created: true,
        server_client_inserted: true,
        client_connection_created: true,
        event_queue_created: true,
        registry_roundtrip_started: true,
        registry_roundtrip_completed: true,
        registry_bind_attempted: true,
        client_bound_wl_compositor: true,
        client_bound_xdg_wm_base: false,
        wl_surface_created: false,
        client_harness_available: true,
        xdg_surface_lifecycle_available: false,
        xdg_toplevel_lifecycle_available: false,
        ledger_admit_invoked: false,
        ledger_unmap_invoked: false,
        core_register_invoked: false,
        core_detach_invoked: false,
        protocol_dispatch_started: true,
        real_compositor_runtime_available: false,
        real_xdg_shell_runtime_available: false,
        render_support: false,
        input_support: false,
        blockers: Vec::new(),
    })
}

fn run_controlled_client(client_stream: UnixStream) -> Result<(), ControlledWlCompositorBindError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        ControlledWlCompositorBindError::ClientProtocol {
            operation: ControlledWlCompositorBindOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) = registry_queue_init::<ControlledClientState>(&connection)
        .map_err(|_| ControlledWlCompositorBindError::ClientProtocol {
            operation: ControlledWlCompositorBindOperation::InitializeRegistryQueue,
        })?;
    let queue_handle = event_queue.handle();
    let _compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| ControlledWlCompositorBindError::ClientProtocol {
            operation: ControlledWlCompositorBindOperation::BindWlCompositor,
        })?;
    let mut client_state = ControlledClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        ControlledWlCompositorBindError::ClientProtocol {
            operation: ControlledWlCompositorBindOperation::CompleteBindRoundtrip,
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledWlCompositorBindBlocker, ControlledWlCompositorBindError,
        controlled_wl_compositor_bind_report,
    };
    use crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe;

    /// 未初始化 server global 时必须结构化拒绝，不能创建 client harness。
    #[test]
    fn controlled_wl_compositor_bind_requires_server_owner() {
        let mut server =
            SmithayWaylandDisplayProbe::new().expect("测试 Wayland display 必须可创建");

        let result = controlled_wl_compositor_bind_report(&mut server);

        assert_eq!(
            result,
            Err(ControlledWlCompositorBindError::Blocked(
                ControlledWlCompositorBindBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    /// 受控 endpoint 必须完成 registry roundtrip 并且只提升 wl_compositor bind 事实。
    #[test]
    fn controlled_wl_compositor_bind_binds_only_wl_compositor() {
        let mut server =
            SmithayWaylandDisplayProbe::new().expect("测试 Wayland display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 server 必须显式初始化 wl_compositor");

        let report = controlled_wl_compositor_bind_report(&mut server)
            .expect("受控 wl_compositor bind proof 必须完成");

        assert!(report.server_wl_compositor_owner_available);
        assert!(report.server_wl_compositor_initialized);
        assert!(report.per_client_compositor_state_available);
        assert!(report.controlled_endpoint_created);
        assert!(report.server_client_inserted);
        assert!(report.client_connection_created);
        assert!(report.event_queue_created);
        assert!(report.registry_roundtrip_started);
        assert!(report.registry_roundtrip_completed);
        assert!(report.registry_bind_attempted);
        assert!(report.client_bound_wl_compositor);
        assert!(report.client_harness_available);
        assert!(report.protocol_dispatch_started);
        assert!(!report.client_bound_xdg_wm_base);
        assert!(!report.wl_surface_created);
        assert!(!report.xdg_surface_lifecycle_available);
        assert!(!report.xdg_toplevel_lifecycle_available);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.blockers.is_empty());
    }
}
