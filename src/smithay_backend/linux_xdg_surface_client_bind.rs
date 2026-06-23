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
    protocol::{wl_compositor::WlCompositor, wl_registry::WlRegistry, wl_surface::WlSurface},
};
use wayland_protocols::xdg::shell::client::{xdg_surface::XdgSurface, xdg_wm_base::XdgWmBase};

use super::{
    client_insert::NestedClientInsertCompileBoundary,
    client_session::{NestedClientSessionEvent, NestedClientSessionId},
    linux_wl_surface_identity::{SurfaceIdentityError, SurfaceIdentityKey},
    surface_xdg_admission::AdapterSurfaceId,
    wayland_display::SmithayWaylandDisplayProbe,
};

const CONTROLLED_SESSION_ID: u64 = 55;
const CONTROLLED_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_PUMP_WAIT: Duration = Duration::from_millis(1);

/// Controlled `xdg_surface` creation proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledXdgSurfaceCreationBlocker {
    /// Server 尚未显式初始化 xdg-shell global owner。
    MissingServerXdgShellGlobalOwner,
    /// Server 尚未显式初始化 `wl_compositor` owner。
    MissingServerWlCompositorOwner,
    /// Client 未能先 bind `wl_compositor`，因此禁止创建 surface。
    MissingClientWlCompositorBind,
    /// Client 未能先 bind `xdg_wm_base`，因此禁止创建 xdg surface。
    MissingClientXdgWmBaseBind,
}

/// Controlled xdg_surface proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledXdgSurfaceCreationOperation {
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
    /// 创建 `wl_surface`。
    CreateWlSurface,
    /// Bind `xdg_wm_base`。
    BindXdgWmBase,
    /// 调用 `xdg_wm_base.get_xdg_surface`。
    CreateXdgSurface,
    /// 创建 xdg surface 后执行同步 roundtrip。
    CompleteXdgSurfaceRoundtrip,
    /// 驱动 server request dispatch。
    DispatchServerClients,
    /// Flush server events。
    FlushServerClients,
}

/// Controlled `xdg_surface` creation proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledXdgSurfaceCreationError {
    /// 前置 capability 尚未满足。
    Blocked(ControlledXdgSurfaceCreationBlocker),
    /// 受控 session identity 无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败阶段。
        operation: ControlledXdgSurfaceCreationOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败阶段。
        operation: ControlledXdgSurfaceCreationOperation,
    },
    /// Server insertion 未产生预期 `NestedClientDataOwner` evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client 成功返回，但 server 没有观察到新 surface。
    MissingServerSurfaceObservation,
    /// Server 观察到了 surface，但 adapter identity 分配被结构化拒绝。
    SurfaceIdentity(SurfaceIdentityError),
    /// Client proof thread 提前断开。
    ClientThreadDisconnected,
    /// Client proof thread panic。
    ClientThreadPanicked,
    /// 有界 proof 未在期限内完成。
    TimedOut,
}

/// Linux-only controlled `xdg_surface` creation proof 报告。
///
/// 成功只表示受控 client 为已创建的 `wl_surface` 创建了 `xdg_surface` role object。
/// 它不表示创建了 xdg toplevel、观察到 `new_toplevel` callback、触发 ledger/core、
/// 分配窗口身份，或具备 render/input/真实 runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledXdgSurfaceCreationReport {
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
    /// Client 是否尝试 create_surface request。
    pub wl_surface_create_attempted: bool,
    /// Controlled client 是否创建 `wl_surface`。
    pub wl_surface_created: bool,
    /// Server `new_surface` handler 是否观察到该 boundary。
    pub server_surface_observed: bool,
    /// Adapter 是否为 server object 分配纯数据 identity。
    pub adapter_surface_identity_allocated: bool,
    /// 分配出的 adapter-only surface ID；不是 core surface identity。
    pub adapter_surface_id: AdapterSurfaceId,
    /// 分配出的 adapter-only identity key。
    pub surface_identity_key: SurfaceIdentityKey,
    /// Adapter identity key 是否可用。
    pub surface_identity_key_available: bool,
    /// Client 是否成功 bind `xdg_wm_base`。
    pub client_bound_xdg_wm_base: bool,
    /// 是否尝试创建 xdg surface。
    pub xdg_surface_create_attempted: bool,
    /// 是否创建 xdg surface。
    pub xdg_surface_created: bool,
    /// 是否尝试创建 xdg toplevel；本阶段固定为 false。
    pub xdg_toplevel_create_attempted: bool,
    /// 是否创建 xdg toplevel；本阶段固定为 false。
    pub xdg_toplevel_created: bool,
    /// 是否观察到 `new_toplevel` callback；本阶段固定为 false。
    pub new_toplevel_callback_observed: bool,
    /// 是否调用 admission ledger admit；本阶段固定为 false。
    pub ledger_admit_invoked: bool,
    /// 是否调用 admission ledger unmap；本阶段固定为 false。
    pub ledger_unmap_invoked: bool,
    /// 是否调用 core register；本阶段固定为 false。
    pub core_register_invoked: bool,
    /// 是否调用 core detach；本阶段固定为 false。
    pub core_detach_invoked: bool,
    /// 是否分配 core window identity；本阶段固定为 false。
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
    pub blockers: Vec<ControlledXdgSurfaceCreationBlocker>,
}

#[derive(Debug, Default)]
struct ControlledXdgSurfaceClientState;

impl Dispatch<WlRegistry, GlobalListContents> for ControlledXdgSurfaceClientState {
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

wayland_client::delegate_noop!(ControlledXdgSurfaceClientState: ignore WlCompositor);
wayland_client::delegate_noop!(ControlledXdgSurfaceClientState: ignore WlSurface);
wayland_client::delegate_noop!(ControlledXdgSurfaceClientState: ignore XdgWmBase);
wayland_client::delegate_noop!(ControlledXdgSurfaceClientState: ignore XdgSurface);

/// 在受控 endpoint 上证明 client 可以为 `wl_surface` 创建 `xdg_surface`。
///
/// 本函数不连接系统 Wayland session socket，只使用内部 stream pair。52Q 只允许
/// controlled `xdg_surface` creation：`xdg_surface` 不等于 xdg toplevel/window，
/// 因此不得调用 `get_toplevel`、不得触发 `new_toplevel` callback、不得调用 admission
/// ledger、不得注册 core surface/window，也不得进入 render/input。
pub fn controlled_xdg_surface_creation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledXdgSurfaceCreationReport, ControlledXdgSurfaceCreationError> {
    if !server.is_xdg_shell_global_initialized() {
        return Err(ControlledXdgSurfaceCreationError::Blocked(
            ControlledXdgSurfaceCreationBlocker::MissingServerXdgShellGlobalOwner,
        ));
    }
    if !server.is_wl_compositor_global_initialized() {
        return Err(ControlledXdgSurfaceCreationError::Blocked(
            ControlledXdgSurfaceCreationBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let observations_before = server.wl_surface_observation_count();
    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| ControlledXdgSurfaceCreationError::Io {
            operation: ControlledXdgSurfaceCreationOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(ControlledXdgSurfaceCreationError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| ControlledXdgSurfaceCreationError::Io {
            operation: ControlledXdgSurfaceCreationOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(ControlledXdgSurfaceCreationError::MissingNestedClientDataOwnerEvidence);
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_controlled_xdg_surface_client(client_stream);
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server
            .dispatch_clients_once()
            .map_err(|error| ControlledXdgSurfaceCreationError::Io {
                operation: ControlledXdgSurfaceCreationOperation::DispatchServerClients,
                kind: error.kind(),
            })?;
        server
            .flush_clients_once()
            .map_err(|error| ControlledXdgSurfaceCreationError::Io {
                operation: ControlledXdgSurfaceCreationOperation::FlushServerClients,
                kind: error.kind(),
            })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(ControlledXdgSurfaceCreationError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => Err(ControlledXdgSurfaceCreationError::ClientThreadDisconnected),
                    Err(_) => Err(ControlledXdgSurfaceCreationError::ClientThreadPanicked),
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| ControlledXdgSurfaceCreationError::ClientThreadPanicked)?;
    client_result?;

    if server.wl_surface_observation_count() <= observations_before {
        return Err(ControlledXdgSurfaceCreationError::MissingServerSurfaceObservation);
    }
    let mapping = server
        .last_wl_surface_identity_observation()
        .ok_or(ControlledXdgSurfaceCreationError::MissingServerSurfaceObservation)?
        .map_err(ControlledXdgSurfaceCreationError::SurfaceIdentity)?;

    Ok(ControlledXdgSurfaceCreationReport {
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
        wl_surface_create_attempted: true,
        wl_surface_created: true,
        server_surface_observed: true,
        adapter_surface_identity_allocated: true,
        adapter_surface_id: mapping.adapter_surface_id,
        surface_identity_key: mapping.surface_identity_key,
        surface_identity_key_available: true,
        client_bound_xdg_wm_base: true,
        xdg_surface_create_attempted: true,
        xdg_surface_created: true,
        xdg_toplevel_create_attempted: false,
        xdg_toplevel_created: false,
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

fn run_controlled_xdg_surface_client(
    client_stream: UnixStream,
) -> Result<(), ControlledXdgSurfaceCreationError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        ControlledXdgSurfaceCreationError::ClientProtocol {
            operation: ControlledXdgSurfaceCreationOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) =
        registry_queue_init::<ControlledXdgSurfaceClientState>(&connection).map_err(|_| {
            ControlledXdgSurfaceCreationError::ClientProtocol {
                operation: ControlledXdgSurfaceCreationOperation::InitializeRegistryQueue,
            }
        })?;
    let queue_handle = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| {
            ControlledXdgSurfaceCreationError::Blocked(
                ControlledXdgSurfaceCreationBlocker::MissingClientWlCompositorBind,
            )
        })?;
    let surface = compositor.create_surface(&queue_handle, ());
    let xdg_wm_base = globals
        .bind::<XdgWmBase, _, _>(&queue_handle, 1..=7, ())
        .map_err(|_| {
            ControlledXdgSurfaceCreationError::Blocked(
                ControlledXdgSurfaceCreationBlocker::MissingClientXdgWmBaseBind,
            )
        })?;

    // 52Q 首次允许 controlled get_xdg_surface。保留 proxy 到 roundtrip 结束，
    // 但不调用 get_toplevel，不 commit，不把 role object 交给 ledger 或 core。
    let _xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &queue_handle, ());
    let mut client_state = ControlledXdgSurfaceClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        ControlledXdgSurfaceCreationError::ClientProtocol {
            operation: ControlledXdgSurfaceCreationOperation::CompleteXdgSurfaceRoundtrip,
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledXdgSurfaceCreationBlocker, ControlledXdgSurfaceCreationError,
        controlled_xdg_surface_creation_report,
    };
    use crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe;

    #[test]
    fn controlled_xdg_surface_creation_requires_xdg_shell_global() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");

        assert_eq!(
            controlled_xdg_surface_creation_report(&mut server),
            Err(ControlledXdgSurfaceCreationError::Blocked(
                ControlledXdgSurfaceCreationBlocker::MissingServerXdgShellGlobalOwner,
            ))
        );
    }

    #[test]
    fn controlled_xdg_surface_creation_requires_wl_compositor_owner() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");

        assert_eq!(
            controlled_xdg_surface_creation_report(&mut server),
            Err(ControlledXdgSurfaceCreationError::Blocked(
                ControlledXdgSurfaceCreationBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    #[test]
    fn controlled_xdg_surface_creation_creates_xdg_surface_without_toplevel() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = controlled_xdg_surface_creation_report(&mut server)
            .expect("controlled xdg_surface creation proof 必须完成");

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
        assert!(report.wl_surface_create_attempted);
        assert!(report.wl_surface_created);
        assert!(report.server_surface_observed);
        assert!(report.adapter_surface_identity_allocated);
        assert!(report.surface_identity_key_available);
        assert_eq!(
            report.adapter_surface_id.value(),
            report.surface_identity_key.value()
        );
        assert!(report.client_bound_xdg_wm_base);
        assert!(report.xdg_surface_create_attempted);
        assert!(report.xdg_surface_created);
        assert!(!report.xdg_toplevel_create_attempted);
        assert!(!report.xdg_toplevel_created);
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

    #[test]
    fn controlled_xdg_surface_creation_report_is_conservative() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = controlled_xdg_surface_creation_report(&mut server)
            .expect("controlled xdg_surface creation proof 必须完成");

        assert!(report.xdg_surface_created);
        assert!(!report.xdg_toplevel_created);
        assert!(!report.new_toplevel_callback_observed);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
    }
}
