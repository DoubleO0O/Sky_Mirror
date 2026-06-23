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
use wayland_protocols::xdg::shell::client::{
    xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
};

use super::{
    client_insert::NestedClientInsertCompileBoundary,
    client_session::{NestedClientSessionEvent, NestedClientSessionId},
    linux_wl_surface_identity::{SurfaceIdentityError, SurfaceIdentityKey},
    surface_xdg_admission::AdapterSurfaceId,
    wayland_display::SmithayWaylandDisplayProbe,
};

const CONTROLLED_SESSION_ID: u64 = 57;
const CONTROLLED_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_PUMP_WAIT: Duration = Duration::from_millis(1);

/// Controlled `new_toplevel` callback observation proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledNewToplevelCallbackObservationBlocker {
    /// Server 尚未显式初始化 xdg-shell global owner。
    MissingServerXdgShellGlobalOwner,
    /// Server 尚未显式初始化 `wl_compositor` owner。
    MissingServerWlCompositorOwner,
    /// Client 未能先 bind `wl_compositor`，因此禁止创建 surface。
    MissingClientWlCompositorBind,
    /// Client 未能先 bind `xdg_wm_base`，因此禁止创建 xdg object。
    MissingClientXdgWmBaseBind,
}

/// Controlled callback observation proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledNewToplevelCallbackObservationOperation {
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
    /// 调用 `xdg_surface.get_toplevel`。
    CreateXdgToplevel,
    /// 创建 xdg toplevel 后执行同步 roundtrip。
    CompleteNewToplevelRoundtrip,
    /// 驱动 server request dispatch。
    DispatchServerClients,
    /// Flush server events。
    FlushServerClients,
}

/// Controlled `new_toplevel` callback observation proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledNewToplevelCallbackObservationError {
    /// 前置 capability 尚未满足。
    Blocked(ControlledNewToplevelCallbackObservationBlocker),
    /// 受控 session identity 无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败阶段。
        operation: ControlledNewToplevelCallbackObservationOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败阶段。
        operation: ControlledNewToplevelCallbackObservationOperation,
    },
    /// Server insertion 未产生预期 `NestedClientDataOwner` evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client 成功返回，但 server 没有观察到新 surface。
    MissingServerSurfaceObservation,
    /// Server 观察到了 surface，但 adapter identity 分配被结构化拒绝。
    SurfaceIdentity(SurfaceIdentityError),
    /// Client 成功返回，但 server 没有观察到 `new_toplevel` callback。
    MissingNewToplevelCallbackObservation,
    /// Client proof thread 提前断开。
    ClientThreadDisconnected,
    /// Client proof thread panic。
    ClientThreadPanicked,
    /// 有界 proof 未在期限内完成。
    TimedOut,
}

/// Linux-only controlled `new_toplevel` callback observation proof 报告。
///
/// 成功只表示受控 client 的 `get_toplevel` request 驱动到了 server handler，并且
/// handler 记录了 callback 观察次数。它不表示 adapter toplevel identity 已注册，
/// 不触发 admission ledger 或 core mutation，也不表示 render/input/真实 runtime 可用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledNewToplevelCallbackObservationReport {
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
    /// Adapter 是否为 server object 分配纯数据 surface identity。
    pub adapter_surface_identity_allocated: bool,
    /// 分配出的 adapter-only surface ID；不是 core surface identity。
    pub adapter_surface_id: AdapterSurfaceId,
    /// 分配出的 adapter-only surface identity key。
    pub surface_identity_key: SurfaceIdentityKey,
    /// Adapter surface identity key 是否可用。
    pub surface_identity_key_available: bool,
    /// Client 是否成功 bind `xdg_wm_base`。
    pub client_bound_xdg_wm_base: bool,
    /// 是否尝试创建 xdg surface。
    pub xdg_surface_create_attempted: bool,
    /// 是否创建 xdg surface。
    pub xdg_surface_created: bool,
    /// 是否尝试创建 xdg toplevel protocol object。
    pub xdg_toplevel_create_attempted: bool,
    /// 是否创建 xdg toplevel protocol object。
    pub xdg_toplevel_created: bool,
    /// 本 proof 是否预期观察到 `new_toplevel` callback。
    pub new_toplevel_callback_expected: bool,
    /// 是否观察到 `new_toplevel` callback。
    pub new_toplevel_callback_observed: bool,
    /// Handler state 记录的 callback 次数。
    pub new_toplevel_callback_count: u64,
    /// 最近一次 callback 的纯数据观察序号。
    pub new_toplevel_callback_sequence: u64,
    /// 是否注册 adapter toplevel identity；本阶段固定为 false。
    pub adapter_toplevel_identity_registered: bool,
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
    pub blockers: Vec<ControlledNewToplevelCallbackObservationBlocker>,
}

#[derive(Debug, Default)]
struct ControlledNewToplevelCallbackClientState;

impl Dispatch<WlRegistry, GlobalListContents> for ControlledNewToplevelCallbackClientState {
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

wayland_client::delegate_noop!(ControlledNewToplevelCallbackClientState: ignore WlCompositor);
wayland_client::delegate_noop!(ControlledNewToplevelCallbackClientState: ignore WlSurface);
wayland_client::delegate_noop!(ControlledNewToplevelCallbackClientState: ignore XdgWmBase);
wayland_client::delegate_noop!(ControlledNewToplevelCallbackClientState: ignore XdgSurface);
wayland_client::delegate_noop!(ControlledNewToplevelCallbackClientState: ignore XdgToplevel);

/// 在受控 endpoint 上证明 `get_toplevel` request 会触发 server handler callback。
///
/// 本函数不连接系统 Wayland session socket，只使用内部 stream pair。52S 只允许
/// controlled callback observation：不得调用 admission ledger、不得注册 core
/// surface/window、不得分配窗口身份，也不得进入 render/input。
pub fn controlled_new_toplevel_callback_observation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<
    ControlledNewToplevelCallbackObservationReport,
    ControlledNewToplevelCallbackObservationError,
> {
    if !server.is_xdg_shell_global_initialized() {
        return Err(ControlledNewToplevelCallbackObservationError::Blocked(
            ControlledNewToplevelCallbackObservationBlocker::MissingServerXdgShellGlobalOwner,
        ));
    }
    if !server.is_wl_compositor_global_initialized() {
        return Err(ControlledNewToplevelCallbackObservationError::Blocked(
            ControlledNewToplevelCallbackObservationBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let observations_before = server.wl_surface_observation_count();
    let callbacks_before = server.new_toplevel_callback_observation_count();
    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| ControlledNewToplevelCallbackObservationError::Io {
            operation: ControlledNewToplevelCallbackObservationOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(ControlledNewToplevelCallbackObservationError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| ControlledNewToplevelCallbackObservationError::Io {
            operation: ControlledNewToplevelCallbackObservationOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(
            ControlledNewToplevelCallbackObservationError::MissingNestedClientDataOwnerEvidence,
        );
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_controlled_new_toplevel_callback_client(client_stream);
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server.dispatch_clients_once().map_err(|error| {
            ControlledNewToplevelCallbackObservationError::Io {
                operation: ControlledNewToplevelCallbackObservationOperation::DispatchServerClients,
                kind: error.kind(),
            }
        })?;
        server.flush_clients_once().map_err(|error| {
            ControlledNewToplevelCallbackObservationError::Io {
                operation: ControlledNewToplevelCallbackObservationOperation::FlushServerClients,
                kind: error.kind(),
            }
        })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(ControlledNewToplevelCallbackObservationError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => {
                        Err(ControlledNewToplevelCallbackObservationError::ClientThreadDisconnected)
                    }
                    Err(_) => {
                        Err(ControlledNewToplevelCallbackObservationError::ClientThreadPanicked)
                    }
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| ControlledNewToplevelCallbackObservationError::ClientThreadPanicked)?;
    client_result?;

    if server.wl_surface_observation_count() <= observations_before {
        return Err(ControlledNewToplevelCallbackObservationError::MissingServerSurfaceObservation);
    }
    let mapping = server
        .last_wl_surface_identity_observation()
        .ok_or(ControlledNewToplevelCallbackObservationError::MissingServerSurfaceObservation)?
        .map_err(ControlledNewToplevelCallbackObservationError::SurfaceIdentity)?;

    let callback_count = server.new_toplevel_callback_observation_count();
    if callback_count <= callbacks_before {
        return Err(
            ControlledNewToplevelCallbackObservationError::MissingNewToplevelCallbackObservation,
        );
    }
    let callback_sequence = server
        .last_new_toplevel_callback_observation_sequence()
        .ok_or(
            ControlledNewToplevelCallbackObservationError::MissingNewToplevelCallbackObservation,
        )?;

    Ok(ControlledNewToplevelCallbackObservationReport {
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
        xdg_toplevel_create_attempted: true,
        xdg_toplevel_created: true,
        new_toplevel_callback_expected: true,
        new_toplevel_callback_observed: true,
        new_toplevel_callback_count: callback_count,
        new_toplevel_callback_sequence: callback_sequence,
        adapter_toplevel_identity_registered: false,
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

fn run_controlled_new_toplevel_callback_client(
    client_stream: UnixStream,
) -> Result<(), ControlledNewToplevelCallbackObservationError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        ControlledNewToplevelCallbackObservationError::ClientProtocol {
            operation: ControlledNewToplevelCallbackObservationOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) =
        registry_queue_init::<ControlledNewToplevelCallbackClientState>(&connection).map_err(
            |_| ControlledNewToplevelCallbackObservationError::ClientProtocol {
                operation:
                    ControlledNewToplevelCallbackObservationOperation::InitializeRegistryQueue,
            },
        )?;
    let queue_handle = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| {
            ControlledNewToplevelCallbackObservationError::Blocked(
                ControlledNewToplevelCallbackObservationBlocker::MissingClientWlCompositorBind,
            )
        })?;
    let surface = compositor.create_surface(&queue_handle, ());
    let xdg_wm_base = globals
        .bind::<XdgWmBase, _, _>(&queue_handle, 1..=7, ())
        .map_err(|_| {
            ControlledNewToplevelCallbackObservationError::Blocked(
                ControlledNewToplevelCallbackObservationBlocker::MissingClientXdgWmBaseBind,
            )
        })?;

    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &queue_handle, ());
    let _xdg_toplevel = xdg_surface.get_toplevel(&queue_handle, ());
    let mut client_state = ControlledNewToplevelCallbackClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        ControlledNewToplevelCallbackObservationError::ClientProtocol {
            operation:
                ControlledNewToplevelCallbackObservationOperation::CompleteNewToplevelRoundtrip,
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledNewToplevelCallbackObservationBlocker,
        ControlledNewToplevelCallbackObservationError,
        controlled_new_toplevel_callback_observation_report,
    };
    use crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe;

    #[test]
    fn controlled_new_toplevel_callback_observation_requires_xdg_shell_global() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");

        assert_eq!(
            controlled_new_toplevel_callback_observation_report(&mut server),
            Err(ControlledNewToplevelCallbackObservationError::Blocked(
                ControlledNewToplevelCallbackObservationBlocker::MissingServerXdgShellGlobalOwner,
            ))
        );
    }

    #[test]
    fn controlled_new_toplevel_callback_observation_requires_wl_compositor_owner() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");

        assert_eq!(
            controlled_new_toplevel_callback_observation_report(&mut server),
            Err(ControlledNewToplevelCallbackObservationError::Blocked(
                ControlledNewToplevelCallbackObservationBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    #[test]
    fn controlled_new_toplevel_callback_observation_records_callback_without_admission() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = controlled_new_toplevel_callback_observation_report(&mut server)
            .expect("controlled new_toplevel callback observation proof 必须完成");

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
        assert!(report.xdg_toplevel_create_attempted);
        assert!(report.xdg_toplevel_created);
        assert!(report.new_toplevel_callback_expected);
        assert!(report.new_toplevel_callback_observed);
        assert!(report.new_toplevel_callback_count > 0);
        assert_eq!(
            report.new_toplevel_callback_sequence,
            report.new_toplevel_callback_count
        );
        assert!(!report.adapter_toplevel_identity_registered);
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
    fn controlled_new_toplevel_callback_observation_report_is_conservative() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = controlled_new_toplevel_callback_observation_report(&mut server)
            .expect("controlled new_toplevel callback observation proof 必须完成");

        assert!(report.xdg_surface_created);
        assert!(report.xdg_toplevel_created);
        assert!(report.new_toplevel_callback_observed);
        assert!(report.new_toplevel_callback_count > 0);
        assert!(!report.adapter_toplevel_identity_registered);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.window_id_allocated);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
    }
}
