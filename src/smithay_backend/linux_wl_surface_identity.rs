//! Linux-only controlled `wl_surface` creation 与 adapter identity proof。
//!
//! 真实 Wayland object 只在本模块和配对 handler state 内出现。对外 report 只包含
//! 纯数据 identity/capability；本模块不 bind xdg-shell、不调用 admission ledger/core，
//! 也不进入 render/input。

use std::{
    collections::{HashMap, VecDeque},
    io,
    os::unix::net::UnixStream,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use smithay::{
    reexports::wayland_server::{Resource, backend::ObjectId},
    wayland::compositor::{self, BufferAssignment, Damage, SurfaceAttributes},
};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_callback::WlCallback, wl_compositor::WlCompositor, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};

use super::{
    client_insert::NestedClientInsertCompileBoundary,
    client_session::{NestedClientSessionEvent, NestedClientSessionId},
    surface_xdg_admission::{AdapterSurfaceId, ProtocolObjectId},
    wayland_display::SmithayWaylandDisplayProbe,
};

const CONTROLLED_SESSION_ID: u64 = 53;
const CONTROLLED_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_PUMP_WAIT: Duration = Duration::from_millis(1);

/// Adapter 层分配的纯数据 surface identity key。
///
/// 该 key 不保存 `WlSurface`/`ObjectId`，也不是 core `SurfaceId`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceIdentityKey(u64);

impl SurfaceIdentityKey {
    /// 返回 adapter 内部单调分配的非零数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Server surface object 与 adapter 纯数据 identity 的只读映射结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterSurfaceIdentityMapping {
    /// Adapter-only surface identity；不得作为 core `SurfaceId` 使用。
    pub adapter_surface_id: AdapterSurfaceId,
    /// Adapter-only 稳定 key。
    pub surface_identity_key: SurfaceIdentityKey,
}

/// Server surface commit callback 的 adapter-owned 纯数据 observation。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterSurfaceCommitObservation {
    /// Adapter-only surface identity；不得作为 core `SurfaceId` 使用。
    pub adapter_surface_id: AdapterSurfaceId,
    /// Adapter-only 稳定 key。
    pub surface_identity_key: SurfaceIdentityKey,
    /// 本 registry 内观察到的单调 commit 序号。
    pub commit_sequence: u64,
    /// 本次 commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,
    /// 本次 commit 是否携带真实 buffer presence evidence。
    pub buffer_present: bool,
    /// 本次 commit 是否携带 `attach(NULL)` / buffer removal evidence。
    pub buffer_removed: bool,
    /// 本次 commit 是否已可作为 renderable buffer；Phase 54D 固定为 false。
    pub renderable_buffer: bool,
    /// 本次 commit 是否携带 damage / damage_buffer evidence。
    pub damage_observed: bool,
    /// 本次 commit 中 surface-coordinate damage rectangle 数量。
    pub surface_damage_rects: usize,
    /// 本次 commit 中 buffer-coordinate damage rectangle 数量。
    pub buffer_damage_rects: usize,
    /// 本次 commit 是否携带 frame callback request evidence。
    pub frame_callback_observed: bool,
    /// 本次 commit 中 frame callback request 数量。
    pub frame_callback_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct AdapterSurfaceCommitBufferEvidence {
    buffer_attach_observed: bool,
    buffer_present: bool,
    buffer_removed: bool,
    renderable_buffer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct AdapterSurfaceCommitDamageEvidence {
    damage_observed: bool,
    surface_damage_rects: usize,
    buffer_damage_rects: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct AdapterSurfaceCommitFrameCallbackEvidence {
    frame_callback_observed: bool,
    frame_callback_count: usize,
}

/// 从 server object 建立 adapter identity 时的结构化错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceIdentityError {
    /// Smithay resource 没有可用 object identity。
    SmithayIdentityUnavailable,
    /// Commit callback 到达时 adapter 尚未观察过对应 `new_surface` identity。
    AdapterSurfaceIdentityMissing,
    /// Adapter 的单调 identity 空间耗尽。
    AdapterIdentityExhausted,
}

/// 真实 server `ObjectId` 到纯数据 identity 的 adapter-owned registry。
///
/// `ObjectId` 仅作为 adapter 内部 key；公开 mapping 不泄漏 Smithay 类型。重复观察
/// 同一 surface 返回原 mapping，不会重复分配 identity。
#[derive(Debug)]
pub(crate) struct LinuxWlSurfaceIdentityRegistry {
    next_identity: u64,
    mappings: HashMap<ObjectId, AdapterSurfaceIdentityMapping>,
    observation_count: usize,
    last_observation: Option<Result<AdapterSurfaceIdentityMapping, SurfaceIdentityError>>,
    commit_observation_count: u64,
    last_commit_observation: Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>>,
    pending_commit_observations:
        VecDeque<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>>,
}

impl LinuxWlSurfaceIdentityRegistry {
    pub(crate) fn new() -> Self {
        Self {
            next_identity: 1,
            mappings: HashMap::new(),
            observation_count: 0,
            last_observation: None,
            commit_observation_count: 0,
            last_commit_observation: None,
            pending_commit_observations: VecDeque::new(),
        }
    }

    pub(crate) fn observe_surface(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) -> Result<AdapterSurfaceIdentityMapping, SurfaceIdentityError> {
        self.observation_count = self.observation_count.saturating_add(1);
        let object_id = surface.id();
        if object_id.is_null() {
            let error = SurfaceIdentityError::SmithayIdentityUnavailable;
            self.last_observation = Some(Err(error));
            return Err(error);
        }
        if let Some(mapping) = self.mappings.get(&object_id).copied() {
            self.last_observation = Some(Ok(mapping));
            return Ok(mapping);
        }

        let value = self.next_identity;
        let protocol_object_id =
            ProtocolObjectId::new(value).ok_or(SurfaceIdentityError::AdapterIdentityExhausted)?;
        self.next_identity = value
            .checked_add(1)
            .ok_or(SurfaceIdentityError::AdapterIdentityExhausted)?;
        let mapping = AdapterSurfaceIdentityMapping {
            adapter_surface_id: AdapterSurfaceId::new(protocol_object_id),
            surface_identity_key: SurfaceIdentityKey(value),
        };
        self.mappings.insert(object_id, mapping);
        self.last_observation = Some(Ok(mapping));
        Ok(mapping)
    }

    pub(crate) fn observe_surface_commit(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) -> Result<AdapterSurfaceCommitObservation, SurfaceIdentityError> {
        self.commit_observation_count = self.commit_observation_count.saturating_add(1);
        let commit_sequence = self.commit_observation_count;
        let object_id = surface.id();
        if object_id.is_null() {
            let error = SurfaceIdentityError::SmithayIdentityUnavailable;
            let result = Err(error);
            self.last_commit_observation = Some(result);
            self.pending_commit_observations.push_back(result);
            return Err(error);
        }

        let Some(mapping) = self.mappings.get(&object_id).copied() else {
            let error = SurfaceIdentityError::AdapterSurfaceIdentityMissing;
            let result = Err(error);
            self.last_commit_observation = Some(result);
            self.pending_commit_observations.push_back(result);
            return Err(error);
        };
        let buffer_evidence = observe_surface_commit_buffer_evidence(surface);
        let damage_evidence = observe_surface_commit_damage_evidence(surface);
        let frame_callback_evidence = observe_surface_commit_frame_callback_evidence(surface);
        let observation = AdapterSurfaceCommitObservation {
            adapter_surface_id: mapping.adapter_surface_id,
            surface_identity_key: mapping.surface_identity_key,
            commit_sequence,
            buffer_attach_observed: buffer_evidence.buffer_attach_observed,
            buffer_present: buffer_evidence.buffer_present,
            buffer_removed: buffer_evidence.buffer_removed,
            renderable_buffer: buffer_evidence.renderable_buffer,
            damage_observed: damage_evidence.damage_observed,
            surface_damage_rects: damage_evidence.surface_damage_rects,
            buffer_damage_rects: damage_evidence.buffer_damage_rects,
            frame_callback_observed: frame_callback_evidence.frame_callback_observed,
            frame_callback_count: frame_callback_evidence.frame_callback_count,
        };
        let result = Ok(observation);
        self.last_commit_observation = Some(result);
        self.pending_commit_observations.push_back(result);
        Ok(observation)
    }

    pub(crate) fn observation_count(&self) -> usize {
        self.observation_count
    }

    pub(crate) fn last_observation(
        &self,
    ) -> Option<Result<AdapterSurfaceIdentityMapping, SurfaceIdentityError>> {
        self.last_observation
    }

    pub(crate) const fn commit_observation_count(&self) -> u64 {
        self.commit_observation_count
    }

    pub(crate) fn last_commit_observation(
        &self,
    ) -> Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>> {
        self.last_commit_observation
    }

    pub(crate) fn take_next_commit_observation(
        &mut self,
    ) -> Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>> {
        self.pending_commit_observations.pop_front()
    }
}

fn observe_surface_commit_buffer_evidence(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
) -> AdapterSurfaceCommitBufferEvidence {
    compositor::with_states(surface, |states| {
        let mut guard = states.cached_state.get::<SurfaceAttributes>();
        match guard.current().buffer.as_ref() {
            Some(BufferAssignment::NewBuffer(_)) => AdapterSurfaceCommitBufferEvidence {
                buffer_attach_observed: true,
                buffer_present: true,
                buffer_removed: false,
                renderable_buffer: false,
            },
            Some(BufferAssignment::Removed) => AdapterSurfaceCommitBufferEvidence {
                buffer_attach_observed: true,
                buffer_present: false,
                buffer_removed: true,
                renderable_buffer: false,
            },
            None => AdapterSurfaceCommitBufferEvidence::default(),
        }
    })
}

fn observe_surface_commit_damage_evidence(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
) -> AdapterSurfaceCommitDamageEvidence {
    compositor::with_states(surface, |states| {
        let mut guard = states.cached_state.get::<SurfaceAttributes>();
        let mut evidence = AdapterSurfaceCommitDamageEvidence::default();
        for damage in &guard.current().damage {
            match damage {
                Damage::Surface(_) => {
                    evidence.surface_damage_rects = evidence.surface_damage_rects.saturating_add(1);
                }
                Damage::Buffer(_) => {
                    evidence.buffer_damage_rects = evidence.buffer_damage_rects.saturating_add(1);
                }
            }
        }
        evidence.damage_observed =
            evidence.surface_damage_rects > 0 || evidence.buffer_damage_rects > 0;
        evidence
    })
}

fn observe_surface_commit_frame_callback_evidence(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
) -> AdapterSurfaceCommitFrameCallbackEvidence {
    compositor::with_states(surface, |states| {
        let mut guard = states.cached_state.get::<SurfaceAttributes>();
        let frame_callback_count = guard.current().frame_callbacks.len();
        AdapterSurfaceCommitFrameCallbackEvidence {
            frame_callback_observed: frame_callback_count > 0,
            frame_callback_count,
        }
    })
}

impl Default for LinuxWlSurfaceIdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Controlled `wl_surface` proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlSurfaceCreationBlocker {
    /// Server 尚未显式初始化 `wl_compositor` owner。
    MissingServerWlCompositorOwner,
    /// Client 未能先 bind `wl_compositor`，因此禁止创建 surface。
    MissingClientWlCompositorBind,
}

/// Controlled surface proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlSurfaceCreationOperation {
    /// 创建受控 Unix endpoint pair。
    CreateControlledEndpoint,
    /// 插入 server endpoint。
    InsertServerClient,
    /// 创建 client connection。
    CreateClientConnection,
    /// 创建 registry/event queue。
    InitializeRegistryQueue,
    /// 完成 surface request roundtrip。
    CompleteSurfaceRoundtrip,
    /// 驱动 server request dispatch。
    DispatchServerClients,
    /// flush server events。
    FlushServerClients,
}

/// Controlled surface creation proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlSurfaceCreationError {
    /// 前置 capability 尚未满足。
    Blocked(ControlledWlSurfaceCreationBlocker),
    /// 受控 session identity 无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败阶段。
        operation: ControlledWlSurfaceCreationOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败阶段。
        operation: ControlledWlSurfaceCreationOperation,
    },
    /// Server insertion 未产生预期 owner evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client 成功返回，但 server 没有观察到新 surface。
    MissingServerSurfaceObservation,
    /// Server 观察到了 surface，但 adapter identity 分配被结构化拒绝。
    SurfaceIdentity(SurfaceIdentityError),
    /// Client proof thread 提前断开。
    ClientThreadDisconnected,
    /// Client proof thread panic。
    ClientThreadPanicked,
    /// 有界 proof 超时。
    TimedOut,
}

/// Linux-only controlled `wl_surface` creation proof 报告。
///
/// `wl_surface_created` 只表示受控 request 被 server 观察；不表示 xdg role、core
/// registration、commit/renderability 或完整 compositor runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledWlSurfaceCreationReport {
    /// Server-side `wl_compositor` owner 是否存在。
    pub server_wl_compositor_owner_available: bool,
    /// Server-side `wl_compositor` 是否显式初始化。
    pub server_wl_compositor_initialized: bool,
    /// Inserted client 是否持有独立 compositor state。
    pub per_client_compositor_state_available: bool,
    /// 是否创建受控 endpoint pair。
    pub controlled_endpoint_created: bool,
    /// Server endpoint 是否通过现有 insertion seam 插入。
    pub server_client_inserted: bool,
    /// 是否创建 wayland-client connection。
    pub client_connection_created: bool,
    /// 是否创建 client event queue。
    pub event_queue_created: bool,
    /// Registry discovery roundtrip 是否完成。
    pub registry_roundtrip_completed: bool,
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
    /// 分配出的 adapter-only surface ID；不是 core `SurfaceId`。
    pub adapter_surface_id: AdapterSurfaceId,
    /// 分配出的 adapter-only identity key。
    pub surface_identity_key: SurfaceIdentityKey,
    /// Adapter identity key 是否可用。
    pub surface_identity_key_available: bool,
    /// Client 是否 bind xdg-shell；本阶段固定为 false。
    pub client_bound_xdg_wm_base: bool,
    /// 是否创建 xdg surface；本阶段固定为 false。
    pub xdg_surface_created: bool,
    /// 是否创建 xdg toplevel；本阶段固定为 false。
    pub xdg_toplevel_created: bool,
    /// 是否已有 xdg surface lifecycle。
    pub xdg_surface_lifecycle_available: bool,
    /// 是否已有 xdg toplevel lifecycle。
    pub xdg_toplevel_lifecycle_available: bool,
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
    pub blockers: Vec<ControlledWlSurfaceCreationBlocker>,
}

/// Controlled `wl_surface.commit` proof 的结构化 blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlSurfaceCommitBlocker {
    /// Server 尚未显式初始化 `wl_compositor` owner。
    MissingServerWlCompositorOwner,
    /// Client 未能先 bind `wl_compositor`，因此禁止创建和 commit surface。
    MissingClientWlCompositorBind,
}

/// Controlled commit proof 中可定位的操作阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlSurfaceCommitOperation {
    /// 创建受控 Unix endpoint pair。
    CreateControlledEndpoint,
    /// 插入 server endpoint。
    InsertServerClient,
    /// 创建 client connection。
    CreateClientConnection,
    /// 创建 registry/event queue。
    InitializeRegistryQueue,
    /// 完成 commit request roundtrip。
    CompleteCommitRoundtrip,
    /// 驱动 server request dispatch。
    DispatchServerClients,
    /// flush server events。
    FlushServerClients,
}

/// Controlled `wl_surface.commit` proof 的纯数据错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledWlSurfaceCommitError {
    /// 前置 capability 尚未满足。
    Blocked(ControlledWlSurfaceCommitBlocker),
    /// 受控 session identity 无效。
    InvalidControlledSessionIdentity,
    /// I/O 操作失败。
    Io {
        /// 失败阶段。
        operation: ControlledWlSurfaceCommitOperation,
        /// 标准 I/O 错误类别。
        kind: io::ErrorKind,
    },
    /// wayland-client 操作失败。
    ClientProtocol {
        /// 失败阶段。
        operation: ControlledWlSurfaceCommitOperation,
    },
    /// Server insertion 未产生预期 owner evidence。
    MissingNestedClientDataOwnerEvidence,
    /// Client 成功返回，但 server 没有观察到新 surface。
    MissingServerSurfaceObservation,
    /// Client 成功返回，但 server 没有观察到 commit。
    MissingServerSurfaceCommitObservation,
    /// Server 观察到了 surface/commit，但 adapter identity lookup 被结构化拒绝。
    SurfaceIdentity(SurfaceIdentityError),
    /// Client proof thread 提前断开。
    ClientThreadDisconnected,
    /// Client proof thread panic。
    ClientThreadPanicked,
    /// 有界 proof 超时。
    TimedOut,
}

/// Linux-only controlled `wl_surface.commit` proof 报告。
///
/// `wl_surface_committed` 只表示 server handler 观察到 commit callback，并把它解析为
/// adapter-owned surface identity。它不表示 buffer attached、damage、frame callback、
/// core mutation、renderable surface，或完整 compositor runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledWlSurfaceCommitReport {
    /// Server-side `wl_compositor` owner 是否存在。
    pub server_wl_compositor_owner_available: bool,
    /// Server-side `wl_compositor` 是否显式初始化。
    pub server_wl_compositor_initialized: bool,
    /// Inserted client 是否持有独立 compositor state。
    pub per_client_compositor_state_available: bool,
    /// 是否创建受控 endpoint pair。
    pub controlled_endpoint_created: bool,
    /// Server endpoint 是否通过现有 insertion seam 插入。
    pub server_client_inserted: bool,
    /// 是否创建 wayland-client connection。
    pub client_connection_created: bool,
    /// 是否创建 client event queue。
    pub event_queue_created: bool,
    /// Registry discovery roundtrip 是否完成。
    pub registry_roundtrip_completed: bool,
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
    /// Client 是否调用 `wl_surface.commit`。
    pub wl_surface_commit_attempted: bool,
    /// Server `commit` handler 是否观察到 callback。
    pub server_surface_commit_observed: bool,
    /// Commit observation 是否解析到 adapter-owned surface identity。
    pub adapter_surface_commit_observation_available: bool,
    /// 分配出的 adapter-only surface ID；不是 core `SurfaceId`。
    pub adapter_surface_id: AdapterSurfaceId,
    /// 分配出的 adapter-only identity key。
    pub surface_identity_key: SurfaceIdentityKey,
    /// Commit observation 中的 adapter-only surface ID。
    pub committed_adapter_surface_id: AdapterSurfaceId,
    /// Commit observation 中的 adapter-only identity key。
    pub committed_surface_identity_key: SurfaceIdentityKey,
    /// Commit observation 序号。
    pub commit_sequence: u64,
    /// 本次 commit 是否携带 buffer attach/remove evidence。
    pub buffer_attach_observed: bool,
    /// 本次 commit 是否携带真实 buffer presence evidence。
    pub buffer_present: bool,
    /// 本次 commit 是否携带 `attach(NULL)` / buffer removal evidence。
    pub buffer_removed: bool,
    /// 本次 commit 是否已可作为 renderable buffer；Phase 54D 固定为 false。
    pub renderable_buffer: bool,
    /// 本次 commit 是否携带 damage / damage_buffer evidence。
    pub damage_observed: bool,
    /// 本次 commit 中 surface-coordinate damage rectangle 数量。
    pub surface_damage_rects: usize,
    /// 本次 commit 中 buffer-coordinate damage rectangle 数量。
    pub buffer_damage_rects: usize,
    /// 本次 commit 是否携带 frame callback request evidence。
    pub frame_callback_observed: bool,
    /// 本次 commit 中 frame callback request 数量。
    pub frame_callback_count: usize,
    /// 是否 attach 了 buffer；本阶段固定为 false。
    pub buffer_attached: bool,
    /// 是否提交 damage；本阶段固定为 false。
    pub damage_submitted: bool,
    /// 是否请求或发送 frame callback；本阶段固定为 false。
    pub frame_callback_requested: bool,
    /// Client 是否 bind xdg-shell；本阶段固定为 false。
    pub client_bound_xdg_wm_base: bool,
    /// 是否创建 xdg surface；本阶段固定为 false。
    pub xdg_surface_created: bool,
    /// 是否创建 xdg toplevel；本阶段固定为 false。
    pub xdg_toplevel_created: bool,
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
    pub blockers: Vec<ControlledWlSurfaceCommitBlocker>,
}

#[derive(Debug, Default)]
struct ControlledSurfaceClientState;

impl Dispatch<WlRegistry, GlobalListContents> for ControlledSurfaceClientState {
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

wayland_client::delegate_noop!(ControlledSurfaceClientState: ignore WlCompositor);
wayland_client::delegate_noop!(ControlledSurfaceClientState: ignore WlSurface);
wayland_client::delegate_noop!(ControlledSurfaceClientState: ignore WlCallback);

/// 证明 controlled client 创建 `wl_surface` 且 server adapter 分配纯数据 identity。
///
/// 本函数只使用内部 stream pair，不连接系统 Wayland session socket。Surface creation
/// 不等于 xdg surface/window，不得调用 ledger/core，也不提供 render/input 能力。
pub fn controlled_wl_surface_creation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledWlSurfaceCreationReport, ControlledWlSurfaceCreationError> {
    if !server.is_wl_compositor_global_initialized() {
        return Err(ControlledWlSurfaceCreationError::Blocked(
            ControlledWlSurfaceCreationBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let observations_before = server.wl_surface_observation_count();
    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| ControlledWlSurfaceCreationError::Io {
            operation: ControlledWlSurfaceCreationOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(ControlledWlSurfaceCreationError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| ControlledWlSurfaceCreationError::Io {
            operation: ControlledWlSurfaceCreationOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(ControlledWlSurfaceCreationError::MissingNestedClientDataOwnerEvidence);
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_controlled_surface_client(client_stream);
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server
            .dispatch_clients_once()
            .map_err(|error| ControlledWlSurfaceCreationError::Io {
                operation: ControlledWlSurfaceCreationOperation::DispatchServerClients,
                kind: error.kind(),
            })?;
        server
            .flush_clients_once()
            .map_err(|error| ControlledWlSurfaceCreationError::Io {
                operation: ControlledWlSurfaceCreationOperation::FlushServerClients,
                kind: error.kind(),
            })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(ControlledWlSurfaceCreationError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => Err(ControlledWlSurfaceCreationError::ClientThreadDisconnected),
                    Err(_) => Err(ControlledWlSurfaceCreationError::ClientThreadPanicked),
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| ControlledWlSurfaceCreationError::ClientThreadPanicked)?;
    client_result?;

    if server.wl_surface_observation_count() <= observations_before {
        return Err(ControlledWlSurfaceCreationError::MissingServerSurfaceObservation);
    }
    let mapping = server
        .last_wl_surface_identity_observation()
        .ok_or(ControlledWlSurfaceCreationError::MissingServerSurfaceObservation)?
        .map_err(ControlledWlSurfaceCreationError::SurfaceIdentity)?;

    Ok(ControlledWlSurfaceCreationReport {
        server_wl_compositor_owner_available: true,
        server_wl_compositor_initialized: true,
        per_client_compositor_state_available: true,
        controlled_endpoint_created: true,
        server_client_inserted: true,
        client_connection_created: true,
        event_queue_created: true,
        registry_roundtrip_completed: true,
        client_bound_wl_compositor: true,
        wl_surface_create_attempted: true,
        wl_surface_created: true,
        server_surface_observed: true,
        adapter_surface_identity_allocated: true,
        adapter_surface_id: mapping.adapter_surface_id,
        surface_identity_key: mapping.surface_identity_key,
        surface_identity_key_available: true,
        client_bound_xdg_wm_base: false,
        xdg_surface_created: false,
        xdg_toplevel_created: false,
        xdg_surface_lifecycle_available: false,
        xdg_toplevel_lifecycle_available: false,
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

/// 证明 controlled client 的 `wl_surface.commit` 可被 server handler 观察为纯数据 identity。
///
/// 本函数只使用内部 stream pair。Commit observation 不等于 buffer attach、damage、
/// frame callback、renderable surface、ledger/core mutation 或完整 compositor runtime。
pub fn controlled_wl_surface_commit_observation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledWlSurfaceCommitReport, ControlledWlSurfaceCommitError> {
    controlled_wl_surface_commit_observation_report_with_options(server, false, false, false)
}

/// 证明 controlled client 的 `attach(NULL) + commit` 只产生纯数据 buffer removal evidence。
///
/// `attach(NULL)` 不是 buffer import，也不代表 renderable buffer；本 proof 不创建 shm/dmabuf、
/// 不实现 BufferHandler、不处理 damage/frame/render/input/core。
pub fn controlled_wl_surface_null_attach_commit_observation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledWlSurfaceCommitReport, ControlledWlSurfaceCommitError> {
    controlled_wl_surface_commit_observation_report_with_options(server, true, false, false)
}

/// 证明 controlled client 的 `damage_buffer + commit` 只产生纯数据 damage evidence。
///
/// damage observation 不是 render damage submission；本 proof 不 import buffer、不发 frame
/// callback、不调用 renderer/input/core，也不把 surface 标记为 renderable。
pub fn controlled_wl_surface_damage_commit_observation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledWlSurfaceCommitReport, ControlledWlSurfaceCommitError> {
    controlled_wl_surface_commit_observation_report_with_options(server, false, true, false)
}

/// 证明 controlled client 的 `frame + commit` 只产生纯数据 frame callback evidence。
///
/// frame callback observation 不发送 callback done、不调度 frame、不调用 renderer/input/core，
/// 也不把 runtime 标记为具备 frame callback capability。
pub fn controlled_wl_surface_frame_callback_commit_observation_report(
    server: &mut SmithayWaylandDisplayProbe,
) -> Result<ControlledWlSurfaceCommitReport, ControlledWlSurfaceCommitError> {
    controlled_wl_surface_commit_observation_report_with_options(server, false, false, true)
}

fn controlled_wl_surface_commit_observation_report_with_options(
    server: &mut SmithayWaylandDisplayProbe,
    attach_null_buffer: bool,
    damage_buffer: bool,
    request_frame_callback: bool,
) -> Result<ControlledWlSurfaceCommitReport, ControlledWlSurfaceCommitError> {
    if !server.is_wl_compositor_global_initialized() {
        return Err(ControlledWlSurfaceCommitError::Blocked(
            ControlledWlSurfaceCommitBlocker::MissingServerWlCompositorOwner,
        ));
    }

    let observations_before = server.wl_surface_observation_count();
    let commits_before = server.wl_surface_commit_observation_count();
    let (server_stream, client_stream) =
        UnixStream::pair().map_err(|error| ControlledWlSurfaceCommitError::Io {
            operation: ControlledWlSurfaceCommitOperation::CreateControlledEndpoint,
            kind: error.kind(),
        })?;
    let session = NestedClientSessionId::new(CONTROLLED_SESSION_ID)
        .ok_or(ControlledWlSurfaceCommitError::InvalidControlledSessionIdentity)?;
    let mut insertion = NestedClientInsertCompileBoundary::new(server.display_handle());
    let _server_client = insertion
        .insert_client(server_stream, session)
        .map_err(|error| ControlledWlSurfaceCommitError::Io {
            operation: ControlledWlSurfaceCommitOperation::InsertServerClient,
            kind: error.kind(),
        })?;
    if insertion.event_queue().drain_connected()
        != vec![NestedClientSessionEvent::Connected { session }]
    {
        return Err(ControlledWlSurfaceCommitError::MissingNestedClientDataOwnerEvidence);
    }

    let (result_sender, result_receiver) = mpsc::channel();
    let client_thread = thread::spawn(move || {
        let result = run_controlled_surface_commit_client(
            client_stream,
            attach_null_buffer,
            damage_buffer,
            request_frame_callback,
        );
        let _ = result_sender.send(result);
    });

    let deadline = Instant::now() + CONTROLLED_PROOF_TIMEOUT;
    let client_result = loop {
        server
            .dispatch_clients_once()
            .map_err(|error| ControlledWlSurfaceCommitError::Io {
                operation: ControlledWlSurfaceCommitOperation::DispatchServerClients,
                kind: error.kind(),
            })?;
        server
            .flush_clients_once()
            .map_err(|error| ControlledWlSurfaceCommitError::Io {
                operation: ControlledWlSurfaceCommitOperation::FlushServerClients,
                kind: error.kind(),
            })?;

        match result_receiver.recv_timeout(SERVER_PUMP_WAIT) {
            Ok(result) => break result,
            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(RecvTimeoutError::Timeout) => {
                return Err(ControlledWlSurfaceCommitError::TimedOut);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return match client_thread.join() {
                    Ok(()) => Err(ControlledWlSurfaceCommitError::ClientThreadDisconnected),
                    Err(_) => Err(ControlledWlSurfaceCommitError::ClientThreadPanicked),
                };
            }
        }
    };

    client_thread
        .join()
        .map_err(|_| ControlledWlSurfaceCommitError::ClientThreadPanicked)?;
    client_result?;

    if server.wl_surface_observation_count() <= observations_before {
        return Err(ControlledWlSurfaceCommitError::MissingServerSurfaceObservation);
    }
    let mapping = server
        .last_wl_surface_identity_observation()
        .ok_or(ControlledWlSurfaceCommitError::MissingServerSurfaceObservation)?
        .map_err(ControlledWlSurfaceCommitError::SurfaceIdentity)?;
    if server.wl_surface_commit_observation_count() <= commits_before {
        return Err(ControlledWlSurfaceCommitError::MissingServerSurfaceCommitObservation);
    }
    let commit = server
        .last_wl_surface_commit_observation()
        .ok_or(ControlledWlSurfaceCommitError::MissingServerSurfaceCommitObservation)?
        .map_err(ControlledWlSurfaceCommitError::SurfaceIdentity)?;

    Ok(ControlledWlSurfaceCommitReport {
        server_wl_compositor_owner_available: true,
        server_wl_compositor_initialized: true,
        per_client_compositor_state_available: true,
        controlled_endpoint_created: true,
        server_client_inserted: true,
        client_connection_created: true,
        event_queue_created: true,
        registry_roundtrip_completed: true,
        client_bound_wl_compositor: true,
        wl_surface_create_attempted: true,
        wl_surface_created: true,
        server_surface_observed: true,
        adapter_surface_identity_allocated: true,
        wl_surface_commit_attempted: true,
        server_surface_commit_observed: true,
        adapter_surface_commit_observation_available: true,
        adapter_surface_id: mapping.adapter_surface_id,
        surface_identity_key: mapping.surface_identity_key,
        committed_adapter_surface_id: commit.adapter_surface_id,
        committed_surface_identity_key: commit.surface_identity_key,
        commit_sequence: commit.commit_sequence,
        buffer_attach_observed: commit.buffer_attach_observed,
        buffer_present: commit.buffer_present,
        buffer_removed: commit.buffer_removed,
        renderable_buffer: commit.renderable_buffer,
        damage_observed: commit.damage_observed,
        surface_damage_rects: commit.surface_damage_rects,
        buffer_damage_rects: commit.buffer_damage_rects,
        frame_callback_observed: commit.frame_callback_observed,
        frame_callback_count: commit.frame_callback_count,
        buffer_attached: false,
        damage_submitted: false,
        frame_callback_requested: false,
        client_bound_xdg_wm_base: false,
        xdg_surface_created: false,
        xdg_toplevel_created: false,
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

fn run_controlled_surface_client(
    client_stream: UnixStream,
) -> Result<(), ControlledWlSurfaceCreationError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        ControlledWlSurfaceCreationError::ClientProtocol {
            operation: ControlledWlSurfaceCreationOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) =
        registry_queue_init::<ControlledSurfaceClientState>(&connection).map_err(|_| {
            ControlledWlSurfaceCreationError::ClientProtocol {
                operation: ControlledWlSurfaceCreationOperation::InitializeRegistryQueue,
            }
        })?;
    let queue_handle = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| {
            ControlledWlSurfaceCreationError::Blocked(
                ControlledWlSurfaceCreationBlocker::MissingClientWlCompositorBind,
            )
        })?;
    // 52O 首次允许 controlled create_surface；保留 proxy 直到 roundtrip 完成，
    // 但不 commit、不赋 xdg role，也不把真实 object 交给 core。
    let _surface = compositor.create_surface(&queue_handle, ());
    let mut client_state = ControlledSurfaceClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        ControlledWlSurfaceCreationError::ClientProtocol {
            operation: ControlledWlSurfaceCreationOperation::CompleteSurfaceRoundtrip,
        }
    })?;

    Ok(())
}

fn run_controlled_surface_commit_client(
    client_stream: UnixStream,
    attach_null_buffer: bool,
    damage_buffer: bool,
    request_frame_callback: bool,
) -> Result<(), ControlledWlSurfaceCommitError> {
    let connection = Connection::from_socket(client_stream).map_err(|_| {
        ControlledWlSurfaceCommitError::ClientProtocol {
            operation: ControlledWlSurfaceCommitOperation::CreateClientConnection,
        }
    })?;
    let (globals, mut event_queue) =
        registry_queue_init::<ControlledSurfaceClientState>(&connection).map_err(|_| {
            ControlledWlSurfaceCommitError::ClientProtocol {
                operation: ControlledWlSurfaceCommitOperation::InitializeRegistryQueue,
            }
        })?;
    let queue_handle = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
        .map_err(|_| {
            ControlledWlSurfaceCommitError::Blocked(
                ControlledWlSurfaceCommitBlocker::MissingClientWlCompositorBind,
            )
        })?;
    let surface = compositor.create_surface(&queue_handle, ());
    if attach_null_buffer {
        surface.attach(None, 0, 0);
    }
    if damage_buffer {
        surface.damage_buffer(0, 0, 32, 24);
    }
    if request_frame_callback {
        let _callback = surface.frame(&queue_handle, ());
    }
    surface.commit();
    let mut client_state = ControlledSurfaceClientState;
    event_queue.roundtrip(&mut client_state).map_err(|_| {
        ControlledWlSurfaceCommitError::ClientProtocol {
            operation: ControlledWlSurfaceCommitOperation::CompleteCommitRoundtrip,
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledWlSurfaceCommitBlocker, ControlledWlSurfaceCommitError,
        ControlledWlSurfaceCreationBlocker, ControlledWlSurfaceCreationError,
        controlled_wl_surface_commit_observation_report, controlled_wl_surface_creation_report,
        controlled_wl_surface_damage_commit_observation_report,
        controlled_wl_surface_frame_callback_commit_observation_report,
        controlled_wl_surface_null_attach_commit_observation_report,
    };
    use crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe;

    #[test]
    fn controlled_wl_surface_creation_requires_server_owner() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");

        assert_eq!(
            controlled_wl_surface_creation_report(&mut server),
            Err(ControlledWlSurfaceCreationError::Blocked(
                ControlledWlSurfaceCreationBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    #[test]
    fn controlled_wl_surface_creation_observes_server_surface_boundary() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let report = controlled_wl_surface_creation_report(&mut server)
            .expect("controlled surface proof 必须完成");

        assert!(report.server_wl_compositor_owner_available);
        assert!(report.server_wl_compositor_initialized);
        assert!(report.per_client_compositor_state_available);
        assert!(report.controlled_endpoint_created);
        assert!(report.server_client_inserted);
        assert!(report.client_connection_created);
        assert!(report.event_queue_created);
        assert!(report.registry_roundtrip_completed);
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
        assert!(!report.client_bound_xdg_wm_base);
        assert!(!report.xdg_surface_created);
        assert!(!report.xdg_toplevel_created);
        assert!(!report.xdg_surface_lifecycle_available);
        assert!(!report.xdg_toplevel_lifecycle_available);
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
    fn controlled_wl_surface_creation_allocates_distinct_adapter_identities() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let first = controlled_wl_surface_creation_report(&mut server)
            .expect("首个 controlled surface proof 必须完成");
        let second = controlled_wl_surface_creation_report(&mut server)
            .expect("第二个 controlled surface proof 必须完成");

        assert_ne!(first.adapter_surface_id, second.adapter_surface_id);
        assert_ne!(first.surface_identity_key, second.surface_identity_key);
    }

    #[test]
    fn controlled_wl_surface_commit_requires_server_owner() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");

        assert_eq!(
            controlled_wl_surface_commit_observation_report(&mut server),
            Err(ControlledWlSurfaceCommitError::Blocked(
                ControlledWlSurfaceCommitBlocker::MissingServerWlCompositorOwner,
            ))
        );
    }

    #[test]
    fn controlled_wl_surface_commit_observes_adapter_surface_identity() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let report = controlled_wl_surface_commit_observation_report(&mut server)
            .expect("controlled surface commit proof 必须完成");

        assert!(report.server_wl_compositor_owner_available);
        assert!(report.server_wl_compositor_initialized);
        assert!(report.per_client_compositor_state_available);
        assert!(report.controlled_endpoint_created);
        assert!(report.server_client_inserted);
        assert!(report.client_connection_created);
        assert!(report.event_queue_created);
        assert!(report.registry_roundtrip_completed);
        assert!(report.client_bound_wl_compositor);
        assert!(report.wl_surface_create_attempted);
        assert!(report.wl_surface_created);
        assert!(report.server_surface_observed);
        assert!(report.adapter_surface_identity_allocated);
        assert!(report.wl_surface_commit_attempted);
        assert!(report.server_surface_commit_observed);
        assert!(report.adapter_surface_commit_observation_available);
        assert_eq!(
            report.adapter_surface_id,
            report.committed_adapter_surface_id
        );
        assert_eq!(
            report.surface_identity_key,
            report.committed_surface_identity_key
        );
        assert_eq!(report.commit_sequence, 1);
        assert!(!report.buffer_attach_observed);
        assert!(!report.buffer_present);
        assert!(!report.buffer_removed);
        assert!(!report.renderable_buffer);
        assert!(!report.damage_observed);
        assert_eq!(report.surface_damage_rects, 0);
        assert_eq!(report.buffer_damage_rects, 0);
        assert!(!report.frame_callback_observed);
        assert_eq!(report.frame_callback_count, 0);
        assert!(!report.buffer_attached);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_requested);
        assert!(!report.client_bound_xdg_wm_base);
        assert!(!report.xdg_surface_created);
        assert!(!report.xdg_toplevel_created);
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
    fn controlled_wl_surface_null_attach_commit_records_buffer_removal_evidence() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let report = controlled_wl_surface_null_attach_commit_observation_report(&mut server)
            .expect("controlled null attach commit proof 必须完成");

        assert!(report.server_surface_commit_observed);
        assert!(report.adapter_surface_commit_observation_available);
        assert!(report.buffer_attach_observed);
        assert!(!report.buffer_present);
        assert!(report.buffer_removed);
        assert!(!report.renderable_buffer);
        assert!(!report.damage_observed);
        assert_eq!(report.surface_damage_rects, 0);
        assert_eq!(report.buffer_damage_rects, 0);
        assert!(!report.frame_callback_observed);
        assert_eq!(report.frame_callback_count, 0);
        assert!(!report.buffer_attached);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_requested);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn controlled_wl_surface_damage_commit_records_buffer_damage_evidence() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let report = controlled_wl_surface_damage_commit_observation_report(&mut server)
            .expect("controlled damage commit proof 必须完成");

        assert!(report.server_surface_commit_observed);
        assert!(report.adapter_surface_commit_observation_available);
        assert!(!report.buffer_attach_observed);
        assert!(!report.buffer_present);
        assert!(!report.buffer_removed);
        assert!(!report.renderable_buffer);
        assert!(report.damage_observed);
        assert_eq!(report.surface_damage_rects, 0);
        assert_eq!(report.buffer_damage_rects, 1);
        assert!(!report.frame_callback_observed);
        assert_eq!(report.frame_callback_count, 0);
        assert!(!report.buffer_attached);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_requested);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn controlled_wl_surface_frame_callback_commit_records_request_evidence() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let report = controlled_wl_surface_frame_callback_commit_observation_report(&mut server)
            .expect("controlled frame callback commit proof 必须完成");

        assert!(report.server_surface_commit_observed);
        assert!(report.adapter_surface_commit_observation_available);
        assert!(!report.buffer_attach_observed);
        assert!(!report.buffer_present);
        assert!(!report.buffer_removed);
        assert!(!report.renderable_buffer);
        assert!(!report.damage_observed);
        assert_eq!(report.surface_damage_rects, 0);
        assert_eq!(report.buffer_damage_rects, 0);
        assert!(report.frame_callback_observed);
        assert_eq!(report.frame_callback_count, 1);
        assert!(!report.buffer_attached);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_requested);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.real_compositor_runtime_available);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn controlled_wl_surface_commit_observations_are_fifo_backlogged() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let first = controlled_wl_surface_commit_observation_report(&mut server)
            .expect("首个 controlled surface commit proof 必须完成");
        let second = controlled_wl_surface_commit_observation_report(&mut server)
            .expect("第二个 controlled surface commit proof 必须完成");

        let first_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("首个 commit observation 必须进入 FIFO")
            .expect("首个 commit observation 必须成功解析");
        let second_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("第二个 commit observation 必须进入 FIFO")
            .expect("第二个 commit observation 必须成功解析");

        assert_eq!(first_pending.commit_sequence, 1);
        assert_eq!(first_pending.adapter_surface_id, first.adapter_surface_id);
        assert_eq!(
            first_pending.surface_identity_key,
            first.surface_identity_key
        );
        assert_eq!(first_pending.buffer_removed, false);
        assert!(!first_pending.damage_observed);
        assert_eq!(first_pending.surface_damage_rects, 0);
        assert_eq!(first_pending.buffer_damage_rects, 0);
        assert!(!first_pending.frame_callback_observed);
        assert_eq!(first_pending.frame_callback_count, 0);
        assert_eq!(second_pending.commit_sequence, 2);
        assert_eq!(second_pending.adapter_surface_id, second.adapter_surface_id);
        assert_eq!(
            second_pending.surface_identity_key,
            second.surface_identity_key
        );
        assert_eq!(second_pending.buffer_removed, false);
        assert!(!second_pending.damage_observed);
        assert_eq!(second_pending.surface_damage_rects, 0);
        assert_eq!(second_pending.buffer_damage_rects, 0);
        assert!(!second_pending.frame_callback_observed);
        assert_eq!(second_pending.frame_callback_count, 0);
        assert_ne!(
            first_pending.adapter_surface_id,
            second_pending.adapter_surface_id
        );
        assert_eq!(server.take_next_wl_surface_commit_observation(), None);
    }

    #[test]
    fn controlled_wl_surface_commit_buffer_evidence_is_fifo_not_latest_snapshot() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let first = controlled_wl_surface_null_attach_commit_observation_report(&mut server)
            .expect("首个 null attach commit proof 必须完成");
        let second = controlled_wl_surface_commit_observation_report(&mut server)
            .expect("第二个 plain commit proof 必须完成");

        let first_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("首个 commit observation 必须进入 FIFO")
            .expect("首个 commit observation 必须成功解析");
        let second_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("第二个 commit observation 必须进入 FIFO")
            .expect("第二个 commit observation 必须成功解析");

        assert_eq!(first_pending.commit_sequence, first.commit_sequence);
        assert!(first_pending.buffer_attach_observed);
        assert!(!first_pending.buffer_present);
        assert_eq!(first_pending.buffer_removed, true);
        assert!(!first_pending.renderable_buffer);
        assert!(!first_pending.damage_observed);
        assert_eq!(first_pending.surface_damage_rects, 0);
        assert_eq!(first_pending.buffer_damage_rects, 0);
        assert!(!first_pending.frame_callback_observed);
        assert_eq!(first_pending.frame_callback_count, 0);
        assert_eq!(second_pending.commit_sequence, second.commit_sequence);
        assert!(!second_pending.buffer_attach_observed);
        assert!(!second_pending.buffer_present);
        assert_eq!(second_pending.buffer_removed, false);
        assert!(!second_pending.renderable_buffer);
        assert!(!second_pending.damage_observed);
        assert_eq!(second_pending.surface_damage_rects, 0);
        assert_eq!(second_pending.buffer_damage_rects, 0);
        assert!(!second_pending.frame_callback_observed);
        assert_eq!(second_pending.frame_callback_count, 0);
        assert_eq!(server.take_next_wl_surface_commit_observation(), None);
    }

    #[test]
    fn controlled_wl_surface_commit_damage_evidence_is_fifo_not_latest_snapshot() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let first = controlled_wl_surface_damage_commit_observation_report(&mut server)
            .expect("首个 damage commit proof 必须完成");
        let second = controlled_wl_surface_commit_observation_report(&mut server)
            .expect("第二个 plain commit proof 必须完成");

        let first_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("首个 commit observation 必须进入 FIFO")
            .expect("首个 commit observation 必须成功解析");
        let second_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("第二个 commit observation 必须进入 FIFO")
            .expect("第二个 commit observation 必须成功解析");

        assert_eq!(first_pending.commit_sequence, first.commit_sequence);
        assert!(first_pending.damage_observed);
        assert_eq!(first_pending.surface_damage_rects, 0);
        assert_eq!(first_pending.buffer_damage_rects, 1);
        assert!(!first_pending.frame_callback_observed);
        assert_eq!(first_pending.frame_callback_count, 0);
        assert!(!first_pending.renderable_buffer);
        assert_eq!(second_pending.commit_sequence, second.commit_sequence);
        assert!(!second_pending.damage_observed);
        assert_eq!(second_pending.surface_damage_rects, 0);
        assert_eq!(second_pending.buffer_damage_rects, 0);
        assert!(!second_pending.frame_callback_observed);
        assert_eq!(second_pending.frame_callback_count, 0);
        assert!(!second_pending.renderable_buffer);
        assert_eq!(server.take_next_wl_surface_commit_observation(), None);
    }

    #[test]
    fn controlled_wl_surface_commit_frame_callback_evidence_is_fifo_not_latest_snapshot() {
        let mut server = SmithayWaylandDisplayProbe::new().expect("测试 display 必须可创建");
        server
            .initialize_wl_compositor_global()
            .expect("测试 compositor owner 必须初始化");

        let first = controlled_wl_surface_frame_callback_commit_observation_report(&mut server)
            .expect("首个 frame callback commit proof 必须完成");
        let second = controlled_wl_surface_commit_observation_report(&mut server)
            .expect("第二个 plain commit proof 必须完成");

        let first_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("首个 commit observation 必须进入 FIFO")
            .expect("首个 commit observation 必须成功解析");
        let second_pending = server
            .take_next_wl_surface_commit_observation()
            .expect("第二个 commit observation 必须进入 FIFO")
            .expect("第二个 commit observation 必须成功解析");

        assert_eq!(first_pending.commit_sequence, first.commit_sequence);
        assert!(first_pending.frame_callback_observed);
        assert_eq!(first_pending.frame_callback_count, 1);
        assert!(!first_pending.renderable_buffer);
        assert_eq!(second_pending.commit_sequence, second.commit_sequence);
        assert!(!second_pending.frame_callback_observed);
        assert_eq!(second_pending.frame_callback_count, 0);
        assert!(!second_pending.renderable_buffer);
        assert_eq!(server.take_next_wl_surface_commit_observation(), None);
    }
}
