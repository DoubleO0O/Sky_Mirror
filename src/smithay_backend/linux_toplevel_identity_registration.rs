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
    linux_xdg_toplevel_identity::{
        LinuxXdgToplevelIdentityOperationError, LinuxXdgToplevelIdentitySourceError,
    },
    surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId},
    wayland_display::SmithayWaylandDisplayProbe,
    xdg_toplevel_identity::XdgToplevelIdentityError,
};

const CONTROLLED_SESSION_ID: u64 = 58;
const CONTROLLED_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_PUMP_WAIT: Duration = Duration::from_millis(1);

/// Controlled adapter toplevel identity registration proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterToplevelIdentityRegistrationBlocker {
    /// Server 尚未显式初始化 xdg-shell global owner。
    MissingServerXdgShellGlobalOwner,
    /// Server 尚未显式初始化 `wl_compositor` owner。
    MissingServerWlCompositorOwner,
    /// Client 未能先 bind `wl_compositor`，因此禁止创建 surface。
    MissingClientWlCompositorBind,
    /// Client 未能先 bind `xdg_wm_base`，因此禁止创建 xdg object。
    MissingClientXdgWmBaseBind,
}

/// Controlled adapter toplevel identity registration proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterToplevelIdentityRegistrationOperation {
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

/// Controlled adapter toplevel identity registration proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterToplevelIdentityRegistrationError {
    /// 前置 capability 尚未满足。
    Blocked(AdapterToplevelIdentityRegistrationBlocker),
    /// 受控 session identity 无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败阶段。
        operation: AdapterToplevelIdentityRegistrationOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败阶段。
        operation: AdapterToplevelIdentityRegistrationOperation,
    },
    /// Server insertion 未产生预期 `NestedClientDataOwner` evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client 成功返回，但 server 没有观察到新 surface。
    MissingServerSurfaceObservation,
    /// Server 观察到了 surface，但 adapter surface identity 分配被结构化拒绝。
    SurfaceIdentity(SurfaceIdentityError),
    /// Client 成功返回，但 server 没有观察到 `new_toplevel` callback。
    MissingNewToplevelCallbackObservation,
    /// `new_toplevel` callback 没有产生 adapter toplevel identity registration observation。
    MissingToplevelIdentityRegistrationObservation,
    /// Server callback 无法从 Smithay toplevel object 提取或注册稳定 identity。
    ToplevelIdentity(LinuxXdgToplevelIdentityOperationError),
    /// Client proof thread 提前断开。
    ClientThreadDisconnected,
    /// Client proof thread panic。
    ClientThreadPanicked,
    /// 有界 proof 未在期限内完成。
    TimedOut,
}

impl From<LinuxXdgToplevelIdentitySourceError> for AdapterToplevelIdentityRegistrationError {
    fn from(source: LinuxXdgToplevelIdentitySourceError) -> Self {
        Self::ToplevelIdentity(LinuxXdgToplevelIdentityOperationError::Source(source))
    }
}

impl From<XdgToplevelIdentityError> for AdapterToplevelIdentityRegistrationError {
    fn from(source: XdgToplevelIdentityError) -> Self {
        Self::ToplevelIdentity(LinuxXdgToplevelIdentityOperationError::Mapping(source))
    }
}

impl From<LinuxXdgToplevelIdentityOperationError> for AdapterToplevelIdentityRegistrationError {
    fn from(source: LinuxXdgToplevelIdentityOperationError) -> Self {
        Self::ToplevelIdentity(source)
    }
}

/// Linux-only controlled adapter toplevel identity registration proof 报告。
///
/// 成功只表示受控 `get_toplevel` request 驱动到 `new_toplevel` callback，并且
/// handler 从真实 `ToplevelSurface` 提取 identity 后注册了 adapter-owned
/// `AdapterToplevelId`。`AdapterToplevelId` 不是 `WindowId`；本 proof 不调用
/// admission ledger/core，不生成 renderable window，也不代表真实 compositor runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterToplevelIdentityRegistrationReport {
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
    /// Adapter surface identity 是否可用。
    pub adapter_surface_identity_available: bool,
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
    /// 是否观察到 `new_toplevel` callback。
    pub new_toplevel_callback_observed: bool,
    /// Handler state 记录的 callback 次数。
    pub new_toplevel_callback_count: u64,
    /// 最近一次 callback 的纯数据观察序号。
    pub new_toplevel_callback_sequence: u64,
    /// 是否能从真实 `ToplevelSurface` 提取稳定 toplevel identity source。
    pub toplevel_identity_source_available: bool,
    /// 是否尝试注册 adapter toplevel identity。
    pub adapter_toplevel_identity_registration_attempted: bool,
    /// 是否成功注册 adapter toplevel identity。
    pub adapter_toplevel_identity_registered: bool,
    /// 是否分配 adapter-only toplevel ID；不是 core `WindowId`。
    pub adapter_toplevel_id_allocated: bool,
    /// 分配出的 adapter-only toplevel ID。
    pub adapter_toplevel_id: AdapterToplevelId,
    /// Toplevel registration 是否链接到了同一个 adapter surface identity。
    pub adapter_surface_id_linked: bool,
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
    pub blockers: Vec<AdapterToplevelIdentityRegistrationBlocker>,
}

#[derive(Debug, Default)]
struct AdapterToplevelIdentityRegistrationClientState;

impl Dispatch<WlRegistry, GlobalListContents> for AdapterToplevelIdentityRegistrationClientState {
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

wayland_client::delegate_noop!(AdapterToplevelIdentityRegistrationClientState: ignore WlCompositor);
wayland_client::delegate_noop!(AdapterToplevelIdentityRegistrationClientState: ignore WlSurface);
wayland_client::delegate_noop!(AdapterToplevelIdentityRegistrationClientState: ignore XdgWmBase);
wayland_client::delegate_noop!(AdapterToplevelIdentityRegistrationClientState: ignore XdgSurface);
wayland_client::delegate_noop!(AdapterToplevelIdentityRegistrationClientState: ignore XdgToplevel);

/// 在受控 endpoint 上证明 `new_toplevel` callback 注册 adapter-owned toplevel identity。
///
/// 本函数不连接系统 Wayland session socket，只使用内部 stream pair。它复用受控
/// `get_toplevel` path，但验收点是 adapter identity registration；不得调用
/// admission ledger、不得注册 core surface/window、不得分配窗口身份，也不得进入 render/input。
pub fn adapter_toplevel_identity_registration_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<AdapterToplevelIdentityRegistrationReport, AdapterToplevelIdentityRegistrationError> {
    if !server.is_xdg_shell_global_initialized() {
        return Err(AdapterToplevelIdentityRegistrationError::Blocked(
            AdapterToplevelIdentityRegistrationBlocker::MissingServerXdgShellGlobalOwner,
        ));
    }
    if !server.is_wl_compositor_global_initialized() {
        return Err(AdapterToplevelIdentityRegistrationError::Blocked(
            AdapterToplevelIdentityRegistrationBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let observations_before = server.wl_surface_observation_count();
    let callbacks_before = server.new_toplevel_callback_observation_count();
    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| AdapterToplevelIdentityRegistrationError::Io {
            operation: AdapterToplevelIdentityRegistrationOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(AdapterToplevelIdentityRegistrationError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| AdapterToplevelIdentityRegistrationError::Io {
            operation: AdapterToplevelIdentityRegistrationOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(AdapterToplevelIdentityRegistrationError::MissingNestedClientDataOwnerEvidence);
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_adapter_toplevel_identity_registration_client(client_stream);
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server.dispatch_clients_once().map_err(|error| {
            AdapterToplevelIdentityRegistrationError::Io {
                operation: AdapterToplevelIdentityRegistrationOperation::DispatchServerClients,
                kind: error.kind(),
            }
        })?;
        server.flush_clients_once().map_err(|error| {
            AdapterToplevelIdentityRegistrationError::Io {
                operation: AdapterToplevelIdentityRegistrationOperation::FlushServerClients,
                kind: error.kind(),
            }
        })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(AdapterToplevelIdentityRegistrationError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => {
                        Err(AdapterToplevelIdentityRegistrationError::ClientThreadDisconnected)
                    }
                    Err(_) => Err(AdapterToplevelIdentityRegistrationError::ClientThreadPanicked),
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| AdapterToplevelIdentityRegistrationError::ClientThreadPanicked)?;
    client_result?;

    if server.wl_surface_observation_count() <= observations_before {
        return Err(AdapterToplevelIdentityRegistrationError::MissingServerSurfaceObservation);
    }
    let surface_mapping = server
        .last_wl_surface_identity_observation()
        .ok_or(AdapterToplevelIdentityRegistrationError::MissingServerSurfaceObservation)?
        .map_err(AdapterToplevelIdentityRegistrationError::SurfaceIdentity)?;

    let callback_count = server.new_toplevel_callback_observation_count();
    if callback_count <= callbacks_before {
        return Err(
            AdapterToplevelIdentityRegistrationError::MissingNewToplevelCallbackObservation,
        );
    }
    let callback_sequence = server
        .last_new_toplevel_callback_observation_sequence()
        .ok_or(AdapterToplevelIdentityRegistrationError::MissingNewToplevelCallbackObservation)?;
    let registration = server
        .last_adapter_toplevel_identity_registration_observation()
        .ok_or(
            AdapterToplevelIdentityRegistrationError::MissingToplevelIdentityRegistrationObservation,
        )??;

    Ok(AdapterToplevelIdentityRegistrationReport {
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
        adapter_surface_identity_available: true,
        adapter_surface_id: surface_mapping.adapter_surface_id,
        surface_identity_key: surface_mapping.surface_identity_key,
        surface_identity_key_available: true,
        client_bound_xdg_wm_base: true,
        xdg_surface_create_attempted: true,
        xdg_surface_created: true,
        xdg_toplevel_create_attempted: true,
        xdg_toplevel_created: true,
        new_toplevel_callback_observed: true,
        new_toplevel_callback_count: callback_count,
        new_toplevel_callback_sequence: callback_sequence,
        toplevel_identity_source_available: true,
        adapter_toplevel_identity_registration_attempted: true,
        adapter_toplevel_identity_registered: true,
        adapter_toplevel_id_allocated: true,
        adapter_toplevel_id: registration.adapter_toplevel,
        adapter_surface_id_linked: registration.adapter_surface
            == surface_mapping.adapter_surface_id,
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

fn run_adapter_toplevel_identity_registration_client(
    client_stream: UnixStream,
) -> Result<(), AdapterToplevelIdentityRegistrationError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        AdapterToplevelIdentityRegistrationError::ClientProtocol {
            operation: AdapterToplevelIdentityRegistrationOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) = registry_queue_init::<
        AdapterToplevelIdentityRegistrationClientState,
    >(&connection)
    .map_err(
        |_| AdapterToplevelIdentityRegistrationError::ClientProtocol {
            operation: AdapterToplevelIdentityRegistrationOperation::InitializeRegistryQueue,
        },
    )?;
    let queue_handle = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| {
            AdapterToplevelIdentityRegistrationError::Blocked(
                AdapterToplevelIdentityRegistrationBlocker::MissingClientWlCompositorBind,
            )
        })?;
    let surface = compositor.create_surface(&queue_handle, ());
    let xdg_wm_base = globals
        .bind::<XdgWmBase, _, _>(&queue_handle, 1..=7, ())
        .map_err(|_| {
            AdapterToplevelIdentityRegistrationError::Blocked(
                AdapterToplevelIdentityRegistrationBlocker::MissingClientXdgWmBaseBind,
            )
        })?;

    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &queue_handle, ());
    let _xdg_toplevel = xdg_surface.get_toplevel(&queue_handle, ());
    let mut client_state = AdapterToplevelIdentityRegistrationClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        AdapterToplevelIdentityRegistrationError::ClientProtocol {
            operation: AdapterToplevelIdentityRegistrationOperation::CompleteNewToplevelRoundtrip,
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterToplevelIdentityRegistrationBlocker, AdapterToplevelIdentityRegistrationError,
        adapter_toplevel_identity_registration_report,
    };
    use crate::smithay_backend::{
        surface_xdg_admission::{AdapterSurfaceId, ProtocolObjectId},
        wayland_display::SmithayWaylandDisplayProbe,
        xdg_toplevel_identity::{AdapterOwnedToplevelIdentityRegistry, XdgToplevelIdentityError},
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    #[test]
    fn adapter_toplevel_identity_registration_requires_xdg_shell_global() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");

        assert_eq!(
            adapter_toplevel_identity_registration_report(&mut server),
            Err(AdapterToplevelIdentityRegistrationError::Blocked(
                AdapterToplevelIdentityRegistrationBlocker::MissingServerXdgShellGlobalOwner,
            ))
        );
    }

    #[test]
    fn adapter_toplevel_identity_registration_requires_wl_compositor_owner() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");

        assert_eq!(
            adapter_toplevel_identity_registration_report(&mut server),
            Err(AdapterToplevelIdentityRegistrationError::Blocked(
                AdapterToplevelIdentityRegistrationBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    #[test]
    fn adapter_toplevel_identity_registration_registers_adapter_identity() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = adapter_toplevel_identity_registration_report(&mut server)
            .expect("adapter toplevel identity registration proof 必须完成");

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
        assert!(report.adapter_surface_identity_available);
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
        assert!(report.new_toplevel_callback_observed);
        assert!(report.new_toplevel_callback_count > 0);
        assert_eq!(
            report.new_toplevel_callback_sequence,
            report.new_toplevel_callback_count
        );
        assert!(report.toplevel_identity_source_available);
        assert!(report.adapter_toplevel_identity_registration_attempted);
        assert!(report.adapter_toplevel_identity_registered);
        assert!(report.adapter_toplevel_id_allocated);
        assert_eq!(report.adapter_toplevel_id.value(), 1);
        assert!(report.adapter_surface_id_linked);
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
    fn adapter_toplevel_identity_registration_deduplicates_identity() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        let original = registry
            .register(11_u64, surface(41))
            .expect("首次注册必须成功");

        assert_eq!(
            registry.register(11_u64, surface(41)),
            Err(XdgToplevelIdentityError::DuplicateIdentity { existing: original })
        );
        assert_eq!(registry.lookup(&11), Ok(original));
        assert_eq!(registry.active_len(), 1);
    }

    #[test]
    fn adapter_toplevel_identity_registration_report_is_conservative() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        server
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");

        let report = adapter_toplevel_identity_registration_report(&mut server)
            .expect("adapter toplevel identity registration proof 必须完成");

        assert!(report.new_toplevel_callback_observed);
        assert!(report.adapter_toplevel_identity_registered);
        assert!(report.adapter_surface_id_linked);
        assert!(!report.ledger_admit_invoked);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_register_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.window_id_allocated);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.real_xdg_shell_runtime_available);
    }
}
