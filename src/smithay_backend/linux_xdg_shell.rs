//! Linux-only xdg-shell global 与 request-handler 编译边界。
//!
//! 本模块只在 Linux + `smithay-linux` 下可见。它把 Smithay 的真实
//! `XdgShellHandler` / `GlobalDispatch` / `Dispatch` trait 实现定位到 display 内部的
//! `LinuxXdgShellStateSkeleton`。Phase 52I 允许配对 display owner 显式调用
//! `XdgShellState::new`；构造时仍不自动初始化，也不把 global 初始化解释为 dispatch。

use std::collections::{BTreeSet, VecDeque};

use smithay::reexports::wayland_protocols::xdg::shell::server::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::XdgPositioner,
    xdg_surface::XdgSurface,
    xdg_toplevel::XdgToplevel,
    xdg_wm_base::XdgWmBase,
};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::{
    wl_buffer::WlBuffer, wl_callback::WlCallback, wl_surface::WlSurface,
};
use smithay::reexports::wayland_server::{Client, DataInit, Dispatch, DisplayHandle, Resource};
use smithay::utils::Serial;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    BufferAssignment, CompositorClientState, CompositorHandler, CompositorState, SurfaceAttributes,
    with_states,
};
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgPositionerUserData, XdgShellHandler,
    XdgShellState, XdgShellSurfaceUserData, XdgSurfaceUserData, XdgWmBaseUserData,
};
use smithay::wayland::shm::{ShmHandler, ShmState};

use super::client_insert::{NestedClientDataOwner, nested_session_for_inserted_client};
use super::client_session::NestedClientSessionId;
use super::linux_shm_render_admission::RuntimeShmRenderCommitToken;
use super::linux_toplevel_identity_registration::AdapterToplevelIdentityRegistrationError;
use super::linux_wl_compositor::{
    LinuxWlCompositorGlobalInitError, LinuxWlCompositorReadinessReport,
    build_linux_wl_compositor_readiness_report,
};
use super::linux_wl_surface_identity::{
    AdapterSurfaceCommitObservation, AdapterSurfaceIdentityMapping, LinuxWlSurfaceIdentityRegistry,
    SurfaceIdentityError,
};
use super::linux_xdg_lifecycle_observation::observe_toplevel_lifecycle;
use super::linux_xdg_toplevel_identity::LinuxXdgToplevelIdentityRegistry;
use super::wayland_display::SmithayWaylandState;
use super::xdg_lifecycle_observation::{
    XdgToplevelLifecycleObservationReport, XdgToplevelLifecycleSignal,
};
use super::xdg_toplevel_identity::XdgToplevelIdentityMapping;

/// 已通过严格边界检查、但尚未由 runtime renderer 消费的 SHM metadata。
///
/// 这是普通值，不包含 mmap 指针、slice、Wayland resource 或 Core identity。buffer
/// resource 始终由同一 Linux backend owner 保存，不能流入 Core。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValidatedShmBufferMetadata {
    /// 初始像素数据相对 pool 的字节偏移。
    pub offset: usize,
    /// 像素宽度。
    pub width: i32,
    /// 像素高度。
    pub height: i32,
    /// 每行字节数。
    pub stride: usize,
    /// 当前 R2 仅允许的 XRGB8888 format。
    pub format: smithay::reexports::wayland_server::protocol::wl_shm::Format,
    /// 访问时受 Smithay 保护的 pool 映射长度；仅作随后 import 前的一致性证据。
    pub mapped_len: usize,
}

/// SHM resource 在 handler/backend 边界被拒绝的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShmBufferResourceRejection {
    /// surface commit 没有新的 buffer attach。
    NoNewBuffer,
    /// buffer resource identity 已失效。
    DestroyedBuffer,
    /// buffer 不属于当前 wl_shm owner，或 Smithay 不能安全读取 metadata。
    ShmAccessUnavailable,
    /// 当前 MVP 只接受 XRGB8888。
    UnsupportedFormat,
    /// width/height/stride/offset 任一为负或尺寸为零。
    InvalidDimensions,
    /// checked arithmetic 溢出或最末像素超出受保护映射。
    InvalidByteRange,
}

/// runtime 已确认不能继续处理一条已入队 SHM resource 的原因。
///
/// 这与 handler 阶段的 [`ShmBufferResourceRejection`] 分开：前者发生在 resource 入队
/// 前，后者只允许 coordinator 在已经完成 token 精确匹配后写入。这样未知 admission
/// 可以继续 defer，而确定已经失效的 resource 不会永久堵住 FIFO。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShmBufferResourceDiscardReason {
    /// 对应的 Wayland buffer 已收到 destroy callback。
    Destroyed,
    /// source session 已不存在，不能猜测 Core client。
    UnknownSourceSession,
    /// adapter/Core identity 已确认失效，不能再安全 import。
    StaleAdmissionIdentity,
}

/// 已入队的 backend-only buffer resource。
///
/// 此类型明确是唯一 `WlBuffer` owner queue 的内部项；它不能被 Core、ledger 或纯数据
/// observation 直接引用。consumer 以后必须按 FIFO 消费，先验证 session/identity 再 import。
#[derive(Debug)]
pub(crate) struct PendingShmBufferResource {
    /// 与既有纯数据 commit observation 相同的 adapter surface identity。
    pub adapter_surface_id: super::surface_xdg_admission::AdapterSurfaceId,
    /// 与既有纯数据 commit observation 相同的单调 commit sequence。
    pub commit_sequence: u64,
    /// 真实 resource owner session；不是 CoreClientId，缺失时 consumer 必须 fail closed。
    pub source_session: Option<NestedClientSessionId>,
    /// 已检查、可供单次 renderer import 使用的 metadata。
    pub metadata: ValidatedShmBufferMetadata,
    /// 唯一保留的真实 Wayland resource，绝不进入 Core。
    pub buffer: WlBuffer,
    /// 与同一 commit 绑定的真实 frame callbacks。只有 import+draw+submit 成功且
    /// observation 有 damage 时，coordinator completion gate 才能逐个发送 done。
    pub frame_callbacks: Vec<WlCallback>,
}

/// `wl_buffer` 从 commit 到 runtime import 前的唯一 backend owner。
///
/// handler 只调用此 owner 建立 backend resource backlog；它不访问 ledger/Core，也不创建
/// texture 或 renderer。buffer destroy 会从 backlog 移除同一 resource，防止迟到 commit
/// 被 renderer 使用。任何拒绝都不入队，因此无需 Core rollback。
#[derive(Debug, Default)]
pub(crate) struct LinuxShmBufferResourceOwner {
    pending: VecDeque<PendingShmBufferResource>,
    discarded: Vec<ShmBufferResourceDiscardReason>,
    tombstones: BTreeSet<RuntimeShmRenderCommitToken>,
}

impl LinuxShmBufferResourceOwner {
    /// 返回尚未由 runtime consumer 取走的有效 buffer 数量。
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 从 commit 的当前 `NewBuffer` 建立一条严格校验后的 backend resource entry。
    fn observe_commit(
        &mut self,
        surface: &WlSurface,
        observation: &AdapterSurfaceCommitObservation,
        source_session: Option<NestedClientSessionId>,
    ) -> Result<(), ShmBufferResourceRejection> {
        let (buffer, frame_callbacks) = with_states(surface, |states| {
            let mut guard = states.cached_state.get::<SurfaceAttributes>();
            let current = guard.current();
            let buffer = match current.buffer.take() {
                Some(BufferAssignment::NewBuffer(buffer)) => Some(buffer),
                Some(BufferAssignment::Removed) | None => None,
            };
            let frame_callbacks = std::mem::take(&mut current.frame_callbacks);
            current.damage.clear();
            (buffer, frame_callbacks)
        });
        let buffer = buffer.ok_or(ShmBufferResourceRejection::NoNewBuffer)?;

        if buffer.id().is_null() {
            return Err(ShmBufferResourceRejection::DestroyedBuffer);
        }

        let metadata = validate_shm_buffer_metadata(&buffer)?;
        self.pending.push_back(PendingShmBufferResource {
            adapter_surface_id: observation.adapter_surface_id,
            commit_sequence: observation.commit_sequence,
            source_session,
            metadata,
            buffer,
            frame_callbacks,
        });
        Ok(())
    }

    /// 销毁 callback 的唯一 cleanup：移除已不再可安全 import 的 buffer resource。
    fn buffer_destroyed(&mut self, destroyed: &WlBuffer) {
        let destroyed_id = destroyed.id();
        let destroyed_tokens = self
            .pending
            .iter()
            .filter(|pending| pending.buffer.id() == destroyed_id)
            .map(|pending| RuntimeShmRenderCommitToken {
                adapter_surface: pending.adapter_surface_id,
                commit_sequence: pending.commit_sequence,
            })
            .collect::<Vec<_>>();
        self.pending
            .retain(|pending| pending.buffer.id() != destroyed_id);
        for token in destroyed_tokens {
            self.tombstones.insert(token);
            self.discarded
                .push(ShmBufferResourceDiscardReason::Destroyed);
        }
    }

    /// 返回真实 resource FIFO 队首 token，不暴露或转移 `WlBuffer`。
    pub(crate) fn front_token(&self) -> Option<RuntimeShmRenderCommitToken> {
        self.pending
            .front()
            .map(|pending| RuntimeShmRenderCommitToken {
                adapter_surface: pending.adapter_surface_id,
                commit_sequence: pending.commit_sequence,
            })
    }

    /// 判断 exact token 是否已经由 destroy/明确 cleanup owner 回收。
    pub(crate) fn is_tombstoned(&self, token: RuntimeShmRenderCommitToken) -> bool {
        self.tombstones.contains(&token)
    }

    /// 仅在 token 与队首精确一致时借阅 resource metadata，绝不转移 `WlBuffer`。
    ///
    /// coordinator 使用这条 seam 先验证 session、ledger 和 live Core window；未知
    /// admission 只会得到 defer，队首仍保留，禁止通过窥视跨过 A/B 的 FIFO 顺序。
    pub(crate) fn peek_front_matching(
        &self,
        adapter_surface_id: super::surface_xdg_admission::AdapterSurfaceId,
        commit_sequence: u64,
    ) -> Option<&PendingShmBufferResource> {
        self.pending.front().filter(|pending| {
            pending.adapter_surface_id == adapter_surface_id
                && pending.commit_sequence == commit_sequence
        })
    }

    /// 仅在既有纯数据 commit observation 与队首 resource 精确同一时消费。
    ///
    /// 普通 commit、无 buffer commit、无效 buffer commit 都不会生成 resource；因此禁止
    /// “每个 commit pop 一个 buffer”。不匹配时保留队首不动，调用方必须按 lifecycle FIFO
    /// 继续或明确拒绝，不能跨 surface/sequence 搜索、重排或猜配。
    pub(crate) fn take_front_matching(
        &mut self,
        adapter_surface_id: super::surface_xdg_admission::AdapterSurfaceId,
        commit_sequence: u64,
    ) -> Option<PendingShmBufferResource> {
        let matches = self.pending.front().is_some_and(|pending| {
            pending.adapter_surface_id == adapter_surface_id
                && pending.commit_sequence == commit_sequence
        });
        matches.then(|| self.pending.pop_front()).flatten()
    }

    /// 在 token 已精确匹配且 coordinator 已得出不可恢复结论后回收队首 resource。
    ///
    /// 不匹配时不做任何事，避免错 token 回收 A 后误放行 B；未完成 admission 不应调用
    /// 本方法，而应保留 resource 等待对应 lifecycle FIFO 前进。
    pub(crate) fn discard_front_matching(
        &mut self,
        adapter_surface_id: super::surface_xdg_admission::AdapterSurfaceId,
        commit_sequence: u64,
        reason: ShmBufferResourceDiscardReason,
    ) -> bool {
        let Some(_) = self.peek_front_matching(adapter_surface_id, commit_sequence) else {
            return false;
        };
        let _ = self.pending.pop_front();
        self.tombstones.insert(RuntimeShmRenderCommitToken {
            adapter_surface: adapter_surface_id,
            commit_sequence,
        });
        self.discarded.push(reason);
        true
    }

    /// 以纯 token 回收 resource FIFO 队首；只供 coordinator 处理“resource 比
    /// observation 更旧”的单侧 reconciliation，不能搜索或跳过队首。
    pub(crate) fn discard_front_token(
        &mut self,
        token: RuntimeShmRenderCommitToken,
        reason: ShmBufferResourceDiscardReason,
    ) -> bool {
        self.discard_front_matching(token.adapter_surface, token.commit_sequence, reason)
    }
}

/// 复制并检查 importer 所需的 SHM metadata，不构造指向 client memory 的 Rust slice。
fn validate_shm_buffer_metadata(
    buffer: &WlBuffer,
) -> Result<ValidatedShmBufferMetadata, ShmBufferResourceRejection> {
    smithay::wayland::shm::with_buffer_contents(buffer, |_, mapped_len, metadata| {
        use smithay::reexports::wayland_server::protocol::wl_shm::Format;

        if metadata.format != Format::Xrgb8888 {
            return Err(ShmBufferResourceRejection::UnsupportedFormat);
        }
        validate_xrgb8888_metadata_values(
            metadata.offset,
            metadata.width,
            metadata.height,
            metadata.stride,
            mapped_len,
        )
    })
    .map_err(|_| ShmBufferResourceRejection::ShmAccessUnavailable)?
}

/// 校验一个 XRGB8888 SHM metadata 快照的尺寸和字节范围。
///
/// 该纯函数与真实 `WlBuffer` 读取共用，确保受控测试覆盖的非法 metadata 条件正是 handler
/// 在资源进入 FIFO 前执行的条件；它不持有 buffer、映射指针或 Core identity。
fn validate_xrgb8888_metadata_values(
    offset: i32,
    width: i32,
    height: i32,
    stride: i32,
    mapped_len: usize,
) -> Result<ValidatedShmBufferMetadata, ShmBufferResourceRejection> {
    use smithay::reexports::wayland_server::protocol::wl_shm::Format;

    if offset < 0 || width <= 0 || height <= 0 || stride <= 0 {
        return Err(ShmBufferResourceRejection::InvalidDimensions);
    }

    let offset =
        usize::try_from(offset).map_err(|_| ShmBufferResourceRejection::InvalidDimensions)?;
    let stride =
        usize::try_from(stride).map_err(|_| ShmBufferResourceRejection::InvalidDimensions)?;
    let width_bytes = usize::try_from(width)
        .map_err(|_| ShmBufferResourceRejection::InvalidDimensions)?
        .checked_mul(4)
        .ok_or(ShmBufferResourceRejection::InvalidByteRange)?;
    let height =
        usize::try_from(height).map_err(|_| ShmBufferResourceRejection::InvalidDimensions)?;
    if stride < width_bytes {
        return Err(ShmBufferResourceRejection::InvalidDimensions);
    }
    let last_row_offset = height
        .checked_sub(1)
        .and_then(|rows| rows.checked_mul(stride))
        .ok_or(ShmBufferResourceRejection::InvalidByteRange)?;
    let required_len = offset
        .checked_add(last_row_offset)
        .and_then(|value| value.checked_add(width_bytes))
        .ok_or(ShmBufferResourceRejection::InvalidByteRange)?;
    if required_len > mapped_len {
        return Err(ShmBufferResourceRejection::InvalidByteRange);
    }

    Ok(ValidatedShmBufferMetadata {
        offset,
        width,
        height: i32::try_from(height).map_err(|_| ShmBufferResourceRejection::InvalidDimensions)?,
        stride,
        format: Format::Xrgb8888,
        mapped_len,
    })
}

/// Wayland display 内部持有的 Linux-only xdg-shell handler state。
///
/// 该类型把既有公开 `SmithayWaylandState` 与 `XdgShellState` 所有权组合起来。
/// 默认构造保持 `None`，只有配对 display owner 的显式调用才会初始化 global；
/// handler trait 可编译或 global 已初始化都不意味着 protocol dispatch 已启动。
#[derive(Debug, Default)]
pub struct LinuxXdgShellStateSkeleton {
    wayland_state: SmithayWaylandState,
    xdg_shell_state: Option<XdgShellState>,
    compositor_state: Option<CompositorState>,
    /// 唯一的 Smithay `wl_shm` global owner。它随配对 Display 销毁；不持有任何
    /// adapter/Core 映射或 renderer texture，具体 buffer identity 仍由后续 owner 管理。
    shm_state: Option<ShmState>,
    /// commit 后、runtime import 前的唯一 backend-only `WlBuffer` owner。它不属于
    /// surface identity observation FIFO，也不保存任何 Core/ledger mapping。
    shm_buffer_resource_owner: LinuxShmBufferResourceOwner,
    surface_identities: LinuxWlSurfaceIdentityRegistry,
    toplevel_identities: LinuxXdgToplevelIdentityRegistry,
    last_toplevel_identity_registration:
        Option<Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError>>,
    last_toplevel_lifecycle_observation: Option<XdgToplevelLifecycleObservationReport>,
    /// 真实 callback arrival order 的唯一 lifecycle backlog。admission 与 destroy 不得
    /// 分别放入两个 FIFO，否则跨对象 A(new) -> B(new) -> B(destroy) 会被 coordinator
    /// 重新配对而留下 ghost window。队列只保存 adapter-owned 纯数据 observation；
    /// handler 仍不接触 ledger 或 Core。
    pending_live_toplevel_lifecycle_observations: VecDeque<PendingLiveToplevelLifecycleObservation>,
    new_toplevel_callback_count: u64,
    last_new_toplevel_callback_observation_sequence: Option<u64>,
    /// 最近一次真实 `new_toplevel` callback 从其 `wl_surface` owner 解出的 adapter
    /// session。它只是 handler 的只读身份观察，不是 Core client ID，也不建立第二份
    /// 窗口状态；后续 lifecycle queue 必须携带该值并由 coordinator 统一消费。
    last_toplevel_callback_session: Option<NestedClientSessionId>,
}

/// Display owner 保存的待消费 live admission observation。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingLiveToplevelAdmissionObservation {
    /// `new_toplevel` callback 的单调 observation 序号。
    pub new_toplevel_callback_sequence: u64,
    /// 同一 callback 内产生的 adapter toplevel identity registration observation。
    pub adapter_toplevel_identity_registration:
        Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError>,
    /// callback 从真实 Wayland resource owner 解出的 adapter session。`None` 明确
    /// 表示 resource 已失活或并非由 `NestedClientDataOwner` 插入；后续 coordinator
    /// 必须拒绝/诊断，不能猜测 Core client identity。
    pub source_session: Option<NestedClientSessionId>,
}

/// Display owner 中按真实 handler callback 到达顺序保存的 toplevel lifecycle event。
///
/// 这是 admission 与 destroy observation 的唯一 backlog，既不保存 Smithay
/// `ToplevelSurface`，也不携带 ledger/Core 状态。runtime coordinator 必须按 pop_front
/// 的顺序决定由哪个 owner 消费，不能跨类型跳过前序 event。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingLiveToplevelLifecycleObservation {
    /// `new_toplevel` callback 对应的 admission candidate。
    Admission(PendingLiveToplevelAdmissionObservation),
    /// `toplevel_destroyed` callback 对应的纯数据 unmap observation。
    Destroyed(XdgToplevelLifecycleObservationReport),
}

/// Linux-only xdg-shell global 显式初始化的结构化错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgShellGlobalInitError {
    /// 当前 owner 已持有 `XdgShellState`；重复注册同一 global 被拒绝。
    AlreadyInitialized,
}

/// Linux-only `wl_shm` global 显式初始化错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxShmGlobalInitError {
    /// 同一 Display owner 已持有 `ShmState`；重复注册会造成第二个 global 真相，故拒绝。
    AlreadyInitialized,
}

/// `wl_shm` global 的最小 owner/readiness 事实。
///
/// 该报告只说明 server 注册了 Smithay 的 SHM 协议 global；不表示 client 已 bind、
/// 已创建 buffer、已导入 texture、已完成渲染或已发送 frame done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxShmGlobalInitReport {
    /// `ShmState::new` 已由该 Display owner 调用。
    pub(crate) shm_state_new_invoked: bool,
    /// owner 当前持有的 global 可供后续真实 client discovery。
    pub(crate) wl_shm_global_initialized: bool,
    /// `wl_buffer.destroy` callback 已由本 handler owner 接管。
    pub(crate) buffer_destroy_handler_wired: bool,
}

/// Phase 52I global owner 之后仍未满足的 runtime 前置条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgShellGlobalBlocker {
    /// 配对 display owner 尚未执行显式初始化。
    MissingExplicitInitialization,
    /// 尚无受控 xdg client/toplevel lifecycle harness。
    MissingControlledClientHarness,
    /// `new_toplevel` 尚无 identity registration owner。
    MissingNewToplevelRegistrationOwner,
    /// 尚无 dispatch 驱动的 callback observed proof。
    MissingDispatchDrivenCallbackProof,
}

/// Linux-only xdg-shell global owner 的精确初始化/readiness 报告。
///
/// Global 初始化只表示 owner 持有 `XdgShellState`。它不表示协议 dispatch、
/// callback、client harness、完整 runtime、render 或 input 已经可用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxXdgShellGlobalInitReport {
    /// Display 与 handler state 的配对 owner 是否存在。
    pub global_owner_available: bool,
    /// 是否已经成功调用 `XdgShellState::new`。
    pub xdg_shell_state_new_invoked: bool,
    /// xdg-shell global 是否已经由 Smithay 创建。
    pub xdg_shell_global_initialized: bool,
    /// 配对 handler state 是否持有创建出的 `XdgShellState`。
    pub xdg_shell_state_owned: bool,
    /// 是否存在受控 client/toplevel lifecycle harness。
    pub client_harness_available: bool,
    /// `new_toplevel` 是否已有 runtime identity registration owner。
    pub new_toplevel_registration_owner_available: bool,
    /// 是否已证明真实 callback invocation。
    pub callback_observed: bool,
    /// Global 初始化层是否调用 admission ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// Global 初始化层是否调用 core detach。
    pub core_detach_invoked: bool,
    /// 是否已启动 protocol request dispatch。
    pub protocol_dispatch_started: bool,
    /// 是否已有可用的真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 当前仍未满足的后续前置条件。
    pub blockers: Vec<LinuxXdgShellGlobalBlocker>,
}

impl LinuxXdgShellStateSkeleton {
    /// 创建未初始化 protocol global 的 Linux-only handler state。
    pub fn new() -> Self {
        Self {
            wayland_state: SmithayWaylandState::new(),
            xdg_shell_state: None,
            compositor_state: None,
            shm_state: None,
            shm_buffer_resource_owner: LinuxShmBufferResourceOwner::default(),
            surface_identities: LinuxWlSurfaceIdentityRegistry::new(),
            toplevel_identities: LinuxXdgToplevelIdentityRegistry::new(),
            last_toplevel_identity_registration: None,
            last_toplevel_lifecycle_observation: None,
            pending_live_toplevel_lifecycle_observations: VecDeque::new(),
            new_toplevel_callback_count: 0,
            last_new_toplevel_callback_observation_sequence: None,
            last_toplevel_callback_session: None,
        }
    }

    /// 返回既有 Wayland probe state 的只读视图。
    pub(crate) const fn wayland_state(&self) -> &SmithayWaylandState {
        &self.wayland_state
    }

    /// 返回 adapter-owned toplevel identity registry 的只读视图。
    ///
    /// Phase 52F 不从 protocol handler 调用该 registry；这里只明确未来 callback
    /// 所属的 state owner，mapping ownership 不等于 callback observed。
    pub(crate) const fn toplevel_identity_registry(&self) -> &LinuxXdgToplevelIdentityRegistry {
        &self.toplevel_identities
    }

    /// 返回 server handler 收到的 `new_surface` observation 次数。
    pub(crate) fn wl_surface_observation_count(&self) -> usize {
        self.surface_identities.observation_count()
    }

    /// 返回最近一次 server-side `new_surface` 建立的纯数据 mapping。
    pub(crate) fn last_wl_surface_identity_observation(
        &self,
    ) -> Option<Result<AdapterSurfaceIdentityMapping, SurfaceIdentityError>> {
        self.surface_identities.last_observation()
    }

    /// 返回 server handler 收到的 `wl_surface.commit` observation 次数。
    pub(crate) const fn wl_surface_commit_observation_count(&self) -> u64 {
        self.surface_identities.commit_observation_count()
    }

    /// 返回最近一次 `wl_surface.commit` 的 adapter-owned 纯数据 observation。
    pub(crate) fn last_wl_surface_commit_observation(
        &self,
    ) -> Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>> {
        self.surface_identities.last_commit_observation()
    }

    /// 消费下一条 `wl_surface.commit` adapter-owned observation。
    pub(crate) fn take_next_wl_surface_commit_observation(
        &mut self,
    ) -> Option<Result<AdapterSurfaceCommitObservation, SurfaceIdentityError>> {
        self.surface_identities.take_next_commit_observation()
    }

    /// 返回尚未由 runtime renderer consumer 取得的有效 SHM resource 数量。
    ///
    /// 该值只描述 backend backlog，绝不等价于 texture/import/render/present 或 Core window。
    pub(crate) fn pending_shm_buffer_resource_count(&self) -> usize {
        self.shm_buffer_resource_owner.pending_count()
    }

    /// 仅在 commit token 与队首精确匹配时由 coordinator/runtime consumer 取出 resource。
    ///
    /// resource 携带 adapter surface/commit/session metadata，但不携带 Core identity；消费方
    /// 必须在 import 前显式验证 session、ledger surface/toplevel/window 的存活性。
    pub(crate) fn take_shm_buffer_resource_for_commit(
        &mut self,
        observation: &AdapterSurfaceCommitObservation,
    ) -> Option<PendingShmBufferResource> {
        self.shm_buffer_resource_owner
            .take_front_matching(observation.adapter_surface_id, observation.commit_sequence)
    }

    /// 只读查看与 commit token 精确匹配的队首 resource。
    ///
    /// coordinator 必须先以这条 seam 验证 session、ledger 与 live Core window；返回值不
    /// 转移 `WlBuffer`，未知 admission 因而不会错误消费 FIFO 队首。
    pub(crate) fn peek_shm_buffer_resource_for_commit(
        &self,
        observation: &AdapterSurfaceCommitObservation,
    ) -> Option<&PendingShmBufferResource> {
        self.shm_buffer_resource_owner
            .peek_front_matching(observation.adapter_surface_id, observation.commit_sequence)
    }

    /// 返回 backend-only resource FIFO 队首 token，不读取或转移 buffer。
    pub(crate) fn shm_buffer_resource_front_token(&self) -> Option<RuntimeShmRenderCommitToken> {
        self.shm_buffer_resource_owner.front_token()
    }

    /// 判断 token 是否已经由 buffer destroy 或明确 cleanup 路径回收。
    pub(crate) fn shm_buffer_resource_is_tombstoned(
        &self,
        token: RuntimeShmRenderCommitToken,
    ) -> bool {
        self.shm_buffer_resource_owner.is_tombstoned(token)
    }

    /// 回收已由 coordinator 确认不可恢复的队首 resource。
    ///
    /// token 不一致时固定返回 `false` 并保留 FIFO；这不是对未知 admission 的快捷路径。
    pub(crate) fn discard_shm_buffer_resource_for_commit(
        &mut self,
        observation: &AdapterSurfaceCommitObservation,
        reason: ShmBufferResourceDiscardReason,
    ) -> bool {
        self.shm_buffer_resource_owner.discard_front_matching(
            observation.adapter_surface_id,
            observation.commit_sequence,
            reason,
        )
    }

    /// 按 resource 队首的纯 token 回收一条已确认 stale 的 resource。
    pub(crate) fn discard_shm_buffer_resource_token(
        &mut self,
        token: RuntimeShmRenderCommitToken,
        reason: ShmBufferResourceDiscardReason,
    ) -> bool {
        self.shm_buffer_resource_owner
            .discard_front_token(token, reason)
    }

    /// 返回最近一次 callback-like lifecycle identity lookup 报告。
    ///
    /// `Some` 只说明 handler 方法执行了 observation helper；报告中的
    /// `callback_observed` 仍需独立 runtime proof，不能由本 accessor 推导为 true。
    pub fn last_toplevel_lifecycle_observation(
        &self,
    ) -> Option<&XdgToplevelLifecycleObservationReport> {
        self.last_toplevel_lifecycle_observation.as_ref()
    }

    /// 消费最早到达的 live toplevel lifecycle observation。
    ///
    /// 这是 handler/display 的唯一生命周期 FIFO。调用方不得从两个按类型拆分的队列
    /// 分别取值；否则会破坏真实 callback 的跨对象因果顺序。
    pub(crate) fn take_next_live_toplevel_lifecycle_observation(
        &mut self,
    ) -> Option<PendingLiveToplevelLifecycleObservation> {
        self.pending_live_toplevel_lifecycle_observations
            .pop_front()
    }

    /// 返回统一 lifecycle FIFO 中尚未由 coordinator 消费的 event 数量。
    pub(crate) fn pending_live_toplevel_lifecycle_count(&self) -> usize {
        self.pending_live_toplevel_lifecycle_observations.len()
    }

    /// 仅当 admission 位于统一 lifecycle FIFO 的队首时才消费它。
    ///
    /// 该兼容 seam 不会越过前序 destroy event；专用 admission pump 因而不能重排
    /// lifecycle，只能在它确实是下一个 event 时前进。
    pub(crate) fn take_next_live_toplevel_admission_observation(
        &mut self,
    ) -> Option<PendingLiveToplevelAdmissionObservation> {
        match self.pending_live_toplevel_lifecycle_observations.front() {
            Some(PendingLiveToplevelLifecycleObservation::Admission(_)) => {
                let Some(PendingLiveToplevelLifecycleObservation::Admission(observation)) = self
                    .pending_live_toplevel_lifecycle_observations
                    .pop_front()
                else {
                    unreachable!("front 已确认 admission，pop_front 不能改变 queue head")
                };
                Some(observation)
            }
            Some(PendingLiveToplevelLifecycleObservation::Destroyed(_)) | None => None,
        }
    }

    /// 仅当 destroy 位于统一 lifecycle FIFO 的队首时才消费它。
    pub(crate) fn take_next_live_toplevel_unmap_observation(
        &mut self,
    ) -> Option<XdgToplevelLifecycleObservationReport> {
        match self.pending_live_toplevel_lifecycle_observations.front() {
            Some(PendingLiveToplevelLifecycleObservation::Destroyed(_)) => {
                let Some(PendingLiveToplevelLifecycleObservation::Destroyed(observation)) = self
                    .pending_live_toplevel_lifecycle_observations
                    .pop_front()
                else {
                    unreachable!("front 已确认 destroy，pop_front 不能改变 queue head")
                };
                Some(observation)
            }
            Some(PendingLiveToplevelLifecycleObservation::Admission(_)) | None => None,
        }
    }

    /// 返回 server handler 收到的 `new_toplevel` callback 次数。
    pub(crate) const fn new_toplevel_callback_observation_count(&self) -> u64 {
        self.new_toplevel_callback_count
    }

    /// 返回最近一次 `new_toplevel` callback 的纯数据观察序号。
    pub(crate) const fn last_new_toplevel_callback_observation_sequence(&self) -> Option<u64> {
        self.last_new_toplevel_callback_observation_sequence
    }

    /// 返回最近一次真实 `new_toplevel` callback 的 inserted-client adapter session。
    ///
    /// session 由 `NestedClientDataOwner` 随 Wayland client 生命周期持有；这里借
    /// Smithay/Wayland 的公开 `Resource::client()` 读取，不把 backend `ClientId`
    /// 数值强转为 session 或 Core ID。资源已失活、非本 owner 插入的 client 或缺失
    /// `ClientData` 时返回 `None`，调用方必须把它视为未完成映射而不是猜测身份。
    pub(crate) const fn last_toplevel_callback_session(&self) -> Option<NestedClientSessionId> {
        self.last_toplevel_callback_session
    }

    fn session_for_surface(surface: &WlSurface) -> Option<NestedClientSessionId> {
        surface
            .client()
            .as_ref()
            .and_then(nested_session_for_inserted_client)
    }

    /// 返回最近一次 `new_toplevel` callback 触发的 adapter identity registration。
    ///
    /// 该 observation 只包含纯数据 `AdapterToplevelId`/`AdapterSurfaceId` mapping。
    /// Handler 不保存 `ToplevelSurface`，也不把 `AdapterToplevelId` 解释成 core `WindowId`。
    pub(crate) fn last_adapter_toplevel_identity_registration_observation(
        &self,
    ) -> Option<Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError>> {
        self.last_toplevel_identity_registration
    }

    fn record_new_toplevel_callback_observation(&mut self) -> u64 {
        self.new_toplevel_callback_count += 1;
        let sequence = self.new_toplevel_callback_count;
        self.last_new_toplevel_callback_observation_sequence = Some(sequence);
        sequence
    }

    fn register_new_toplevel_identity(
        &mut self,
        surface: &ToplevelSurface,
    ) -> Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError> {
        // Phase 52T 只在 adapter 层登记 protocol identity。这里不持久化
        // `ToplevelSurface`，不调用 admission ledger/core，也不产生 render/input 能力。
        let result = (|| {
            let identity = LinuxXdgToplevelIdentityRegistry::key_for_toplevel(surface)?;
            let surface_mapping = self
                .surface_identities
                .observe_surface(surface.wl_surface())
                .map_err(AdapterToplevelIdentityRegistrationError::SurfaceIdentity)?;
            let adapter_surface = surface_mapping.adapter_surface_id;

            self.toplevel_identities
                .register(identity, adapter_surface)
                .map_err(Into::into)
        })();

        self.last_toplevel_identity_registration = Some(result);
        result
    }

    fn record_pending_live_toplevel_admission_observation(
        &mut self,
        new_toplevel_callback_sequence: u64,
        adapter_toplevel_identity_registration: Result<
            XdgToplevelIdentityMapping,
            AdapterToplevelIdentityRegistrationError,
        >,
        source_session: Option<NestedClientSessionId>,
    ) {
        self.pending_live_toplevel_lifecycle_observations.push_back(
            PendingLiveToplevelLifecycleObservation::Admission(
                PendingLiveToplevelAdmissionObservation {
                    new_toplevel_callback_sequence,
                    adapter_toplevel_identity_registration,
                    source_session,
                },
            ),
        );
    }

    /// 返回当前 owner 是否已经持有 xdg-shell global state。
    pub(crate) const fn is_xdg_shell_global_initialized(&self) -> bool {
        self.xdg_shell_state.is_some()
    }

    /// 使用与 handler state 配对的 display handle 显式初始化 xdg-shell global。
    ///
    /// 本方法保持 crate-private，外部调用方不能注入任意 `DisplayHandle`。公开入口由
    /// `SmithayWaylandDisplayProbe` 提供，并固定使用其自身 display 的 handle。
    pub(crate) fn initialize_xdg_shell_global(
        &mut self,
        display_handle: &DisplayHandle,
    ) -> Result<LinuxXdgShellGlobalInitReport, LinuxXdgShellGlobalInitError> {
        if self.xdg_shell_state.is_some() {
            return Err(LinuxXdgShellGlobalInitError::AlreadyInitialized);
        }

        // Smithay 0.7 的初始化是不可失败构造；先完成构造再写入 Option，避免留下
        // 对调用方可见的半初始化 owner state。
        let xdg_shell_state = XdgShellState::new::<LinuxXdgShellStateSkeleton>(display_handle);
        self.xdg_shell_state = Some(xdg_shell_state);

        Ok(self.xdg_shell_global_readiness_report())
    }

    /// 返回当前 global owner 的保守 readiness，不执行任何 mutation。
    pub(crate) fn xdg_shell_global_readiness_report(&self) -> LinuxXdgShellGlobalInitReport {
        let initialized = self.is_xdg_shell_global_initialized();
        let mut blockers = Vec::new();
        if !initialized {
            blockers.push(LinuxXdgShellGlobalBlocker::MissingExplicitInitialization);
        }
        blockers.extend([
            LinuxXdgShellGlobalBlocker::MissingControlledClientHarness,
            LinuxXdgShellGlobalBlocker::MissingDispatchDrivenCallbackProof,
        ]);

        LinuxXdgShellGlobalInitReport {
            global_owner_available: true,
            xdg_shell_state_new_invoked: initialized,
            xdg_shell_global_initialized: initialized,
            xdg_shell_state_owned: initialized,
            client_harness_available: false,
            new_toplevel_registration_owner_available: true,
            callback_observed: false,
            ledger_unmap_invoked: false,
            core_detach_invoked: false,
            protocol_dispatch_started: false,
            real_xdg_shell_runtime_available: false,
            render_support: false,
            input_support: false,
            blockers,
        }
    }

    /// 返回当前 owner 是否已经持有 `wl_compositor` global state。
    pub(crate) const fn is_wl_compositor_global_initialized(&self) -> bool {
        self.compositor_state.is_some()
    }

    /// 返回当前 owner 是否已经持有 `wl_shm` global state。
    pub(crate) const fn is_wl_shm_global_initialized(&self) -> bool {
        self.shm_state.is_some()
    }

    /// 在配对的 Display 上注册只允许 SHM 的 `wl_shm` global。
    ///
    /// Smithay 负责 pool/buffer 的协议层尺寸、stride、offset、format 与 map 安全检查。
    /// 这里不读取像素、不缓存 `WlBuffer`、不创建纹理、不改 Core；真实 import 前必须由
    /// 独立 buffer owner 把 resource 身份、session 与 surface 显式关联。
    pub(crate) fn initialize_wl_shm_global(
        &mut self,
        display_handle: &DisplayHandle,
    ) -> Result<LinuxShmGlobalInitReport, LinuxShmGlobalInitError> {
        if self.shm_state.is_some() {
            return Err(LinuxShmGlobalInitError::AlreadyInitialized);
        }

        let shm_state = ShmState::new::<LinuxXdgShellStateSkeleton>(display_handle, []);
        self.shm_state = Some(shm_state);
        Ok(self.wl_shm_global_init_report())
    }

    /// 返回 SHM global 的保守 owner/readiness 事实，不产生 mutation。
    pub(crate) fn wl_shm_global_init_report(&self) -> LinuxShmGlobalInitReport {
        let initialized = self.is_wl_shm_global_initialized();
        LinuxShmGlobalInitReport {
            shm_state_new_invoked: initialized,
            wl_shm_global_initialized: initialized,
            buffer_destroy_handler_wired: true,
        }
    }

    /// 使用与 handler state 配对的 display handle 显式初始化 `wl_compositor`。
    ///
    /// 真实 Smithay owner 只能存在于 Linux-only adapter 层。方法保持 crate-private，
    /// 由同时持有 display/state 的外层 owner 传入自己的 handle，避免错配 display。
    pub(crate) fn initialize_wl_compositor_global(
        &mut self,
        display_handle: &DisplayHandle,
    ) -> Result<LinuxWlCompositorReadinessReport, LinuxWlCompositorGlobalInitError> {
        if self.compositor_state.is_some() {
            return Err(LinuxWlCompositorGlobalInitError::AlreadyInitialized);
        }

        // 构造完成后再写入 Option；重复初始化会在 mutation 前结构化拒绝。
        let compositor_state = CompositorState::new::<LinuxXdgShellStateSkeleton>(display_handle);
        self.compositor_state = Some(compositor_state);

        Ok(self.wl_compositor_readiness_report())
    }

    /// 返回当前 `wl_compositor` owner readiness，不执行任何 mutation。
    pub(crate) fn wl_compositor_readiness_report(&self) -> LinuxWlCompositorReadinessReport {
        build_linux_wl_compositor_readiness_report(self.is_wl_compositor_global_initialized())
    }

    fn wl_compositor_state_mut(&mut self) -> &mut CompositorState {
        self.compositor_state
            .as_mut()
            .expect("wl_compositor global 必须先由配对 display owner 显式初始化")
    }

    /// 返回已初始化的 SHM protocol state；未初始化时 request dispatch 不能猜测 owner。
    fn wl_shm_state(&self) -> &ShmState {
        self.shm_state
            .as_ref()
            .expect("wl_shm global 尚未由配对 Display owner 显式初始化")
    }

    /// 返回已初始化的 xdg-shell helper state。
    ///
    /// Phase 52E 不调用本方法；它只满足 Smithay handler trait 的所有权形状。
    /// 若未来在未注册 global 时错误进入 request dispatch，应明确失败，不能把
    /// 缺失的 runtime 初始化静默解释为可用能力。
    fn xdg_shell_state_mut(&mut self) -> &mut XdgShellState {
        self.xdg_shell_state
            .as_mut()
            .expect("xdg-shell global 尚未初始化；Phase 52E 只有编译边界")
    }
}

/// Phase 52E 编译边界之后仍阻止真实 xdg-shell runtime 的结构化缺口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgShellCompileBlocker {
    /// 尚未调用 `XdgShellState::new` 注册 xdg-shell global。
    MissingGlobalInitialization,
    /// 真实 `ToplevelSurface` 尚未映射为纯数据 `AdapterToplevelId`。
    MissingAdapterToplevelIdentityMapping,
    /// toplevel lifecycle signal 尚未桥接到 admission ledger。
    MissingToplevelLifecycleBridge,
    /// Linux adapter 尚未取得 admission ledger 的明确调用所有权。
    MissingLedgerCallerOwnership,
    /// Smithay popup delegation 依赖 `SeatHandler`，本阶段禁止跨入 input/seat。
    MissingPopupSeatHandlerBoundary,
}

/// Linux-only xdg-shell 编译边界的保守能力报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxXdgShellCompileReport {
    /// Linux-only 模块是否存在。
    pub linux_xdg_shell_module_available: bool,
    /// `xdg_wm_base` global dispatch 的类型边界是否可编译。
    pub xdg_shell_global_compile_boundary_available: bool,
    /// xdg-shell request dispatch 的类型边界是否可编译。
    pub xdg_request_handler_compile_boundary_available: bool,
    /// `ToplevelSurface` lifecycle callback 是否已有未来 identity 挂接位置。
    pub xdg_toplevel_identity_hook_point_available: bool,
    /// 是否已观察到真实 xdg_toplevel unmap callback。
    pub xdg_unmap_callback_observed: bool,
    /// Linux 边界是否已调用 ledger unmap。
    pub ledger_unmap_invoked_from_linux_boundary: bool,
    /// 真实 xdg-shell runtime 是否可用。
    pub real_xdg_shell_runtime_available: bool,
    /// protocol dispatch 是否已经启动。
    pub protocol_dispatch_started: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 阻止 compile seam 被解释为真实 runtime 的剩余缺口。
    pub blockers: Vec<LinuxXdgShellCompileBlocker>,
}

/// 返回 Phase 52E Linux-only xdg-shell 编译边界报告。
///
/// callback、ledger、runtime、protocol dispatch、render 与 input 必须保持 false；
/// handler trait 编译成功只证明类型和所有权位置，不证明客户端请求已经发生。
pub fn linux_xdg_shell_readiness_report() -> LinuxXdgShellCompileReport {
    LinuxXdgShellCompileReport {
        linux_xdg_shell_module_available: true,
        xdg_shell_global_compile_boundary_available: true,
        xdg_request_handler_compile_boundary_available: true,
        xdg_toplevel_identity_hook_point_available: true,
        xdg_unmap_callback_observed: false,
        ledger_unmap_invoked_from_linux_boundary: false,
        real_xdg_shell_runtime_available: false,
        protocol_dispatch_started: false,
        render_support: false,
        input_support: false,
        blockers: vec![
            LinuxXdgShellCompileBlocker::MissingGlobalInitialization,
            LinuxXdgShellCompileBlocker::MissingAdapterToplevelIdentityMapping,
            LinuxXdgShellCompileBlocker::MissingToplevelLifecycleBridge,
            LinuxXdgShellCompileBlocker::MissingLedgerCallerOwnership,
            LinuxXdgShellCompileBlocker::MissingPopupSeatHandlerBoundary,
        ],
    }
}

impl XdgShellHandler for LinuxXdgShellStateSkeleton {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        self.xdg_shell_state_mut()
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Smithay handler callback 本身不传 `Client`，但底层 `WlSurface` 是公开
        // `Resource`，可反查其 owner。handler 只记录 adapter session 观察，绝不直接
        // 写 ledger/Core；若读取失败，后续 coordinator 必须拒绝该事件而非猜 ID。
        let source_session = Self::session_for_surface(surface.wl_surface());
        self.last_toplevel_callback_session = source_session;
        let callback_sequence = self.record_new_toplevel_callback_observation();
        let registration = self.register_new_toplevel_identity(&surface);
        self.record_pending_live_toplevel_admission_observation(
            callback_sequence,
            registration,
            source_session,
        );
        // xdg-shell 要求 client 在首次角色提交前取得并 ack configure。这里由 server
        // handler 唯一负责发送初始 configure；它既不触碰 ledger/Core，也不代表窗口
        // 已 map、已渲染或已有 frame completion。client 的 ack/无 buffer commit 仍经
        // 正常 protocol dispatch 回到本 Display owner。
        surface.send_configure();
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // popup 不在本阶段范围内；compile seam 不代表 popup runtime 支持。
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        // input/seat 不在本阶段范围内，request 不能越界启动 input 行为。
    }

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
        // 这里只满足 Smithay trait 的编译形状，不处理真实 popup request。
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        // Handler 只读取 adapter-owned registry 并保存 observation report。
        // Mapping 保持不变；本阶段不得调用 ledger/core，也不得把 wiring 当作
        // 已证明的真实 runtime callback observation。
        let report = observe_toplevel_lifecycle(
            &self.toplevel_identities,
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            &surface,
            None,
        );
        self.last_toplevel_lifecycle_observation = Some(report.clone());
        self.pending_live_toplevel_lifecycle_observations
            .push_back(PendingLiveToplevelLifecycleObservation::Destroyed(report));
    }
}

impl CompositorHandler for LinuxXdgShellStateSkeleton {
    fn compositor_state(&mut self) -> &mut CompositorState {
        self.wl_compositor_state_mut()
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        // Smithay 的 trait 要求返回与 Client 同生命周期的引用；现有 insertion seam
        // 保证所有 client 都安装 NestedClientDataOwner，而不是共享全局/fake state。
        client
            .get_data::<NestedClientDataOwner>()
            .map(NestedClientDataOwner::compositor_state)
            .expect("Wayland client 必须由 NestedClientDataOwner 插入")
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        // 真实 WlSurface 不能进入 core。先以 adapter-owned ObjectId key 去重，再分配
        // 纯数据 AdapterSurfaceId；该观察不赋 xdg role、不调用 ledger/core，也不
        // 表示 surface 已 commit 或可 render。
        let _ = self.surface_identities.observe_surface(surface);
    }

    fn commit(&mut self, surface: &WlSurface) {
        // 先保持既有 immutable pure-data observation FIFO；它是 coordinator 的 ordering
        // token，不能携带 WlBuffer/renderer。随后 backend-only resource owner 在同一 commit
        // sequence 内保留经严格 metadata 检查的 WlBuffer，仍不接触 ledger/Core/render。
        if let Ok(observation) = self.surface_identities.observe_surface_commit(surface) {
            let source_session = Self::session_for_surface(surface);
            let _ = self.shm_buffer_resource_owner.observe_commit(
                surface,
                &observation,
                source_session,
            );
        }
    }
}

impl BufferHandler for LinuxXdgShellStateSkeleton {
    fn buffer_destroyed(&mut self, buffer: &WlBuffer) {
        // wl_shm callback 是该 owner 唯一的资源销毁入口：删除尚未 import 的同一 buffer，
        // 令迟到/已毁 resource 无法跨入 coordinator。它不修改 ledger/Core，也不发送 callback。
        self.shm_buffer_resource_owner.buffer_destroyed(buffer);
    }
}

impl ShmHandler for LinuxXdgShellStateSkeleton {
    fn shm_state(&self) -> &ShmState {
        self.wl_shm_state()
    }
}

smithay::delegate_compositor!(LinuxXdgShellStateSkeleton);
smithay::delegate_shm!(LinuxXdgShellStateSkeleton);

// Smithay 的全量 delegate_xdg_shell! 会让 popup dispatch 要求 SeatHandler。
// 本阶段逐项生成 global 与非 popup request delegation，避免为编译证明伪造 input。
smithay::reexports::wayland_server::delegate_global_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgWmBase: ()
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgWmBase: XdgWmBaseUserData
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgPositioner: XdgPositionerUserData
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgSurface: XdgSurfaceUserData
] => XdgShellState);
smithay::reexports::wayland_server::delegate_dispatch!(LinuxXdgShellStateSkeleton: [
    XdgToplevel: XdgShellSurfaceUserData
] => XdgShellState);

impl Dispatch<XdgPopup, XdgShellSurfaceUserData> for LinuxXdgShellStateSkeleton {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &XdgPopup,
        _request: xdg_popup::Request,
        _data: &XdgShellSurfaceUserData,
        _display_handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // 该实现只关闭 Smithay 类型图中的 popup trait 缺口。真实 global 尚未注册，
        // 所以生产中不可达；若未来误启动，必须 fail closed，不能静默伪装 popup/input。
        panic!("Phase 52E 不处理真实 xdg_popup request；SeatHandler 尚未接入")
    }
}

#[cfg(test)]
mod tests {
    use crate::smithay_backend::{
        client_session::NestedClientSessionId,
        linux_toplevel_identity_registration::adapter_toplevel_identity_registration_report,
        test_support::assert_runtime_dir, wayland_display::SmithayWaylandDisplayProbe,
    };
    use smithay::reexports::wayland_protocols::xdg::shell::server::{
        xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
    };
    use smithay::reexports::wayland_server::protocol::{
        wl_compositor::WlCompositor, wl_surface::WlSurface,
    };
    use smithay::reexports::wayland_server::{Dispatch, GlobalDispatch};
    use smithay::wayland::compositor::{CompositorHandler, SurfaceUserData};
    use smithay::wayland::shell::xdg::{XdgShellHandler, XdgShellSurfaceUserData};

    use super::{
        LinuxXdgShellStateSkeleton, ShmBufferResourceRejection, validate_xrgb8888_metadata_values,
    };
    use smithay::reexports::wayland_server::protocol::wl_shm::Format;

    /// Red：R2 只接受严格的 32-bit XRGB SHM byte range；负/零尺寸、短 stride、映射
    /// 越界与 checked-arithmetic overflow 都必须在 resource 入队前被拒绝。
    #[test]
    fn xrgb_metadata_validator_rejects_invalid_dimensions_and_ranges() {
        assert_eq!(
            validate_xrgb8888_metadata_values(0, 0, 2, 8, 16),
            Err(ShmBufferResourceRejection::InvalidDimensions)
        );
        assert_eq!(
            validate_xrgb8888_metadata_values(0, 2, 2, 7, 16),
            Err(ShmBufferResourceRejection::InvalidDimensions)
        );
        assert_eq!(
            validate_xrgb8888_metadata_values(8, 2, 2, 8, 16),
            Err(ShmBufferResourceRejection::InvalidByteRange)
        );
        assert_eq!(
            validate_xrgb8888_metadata_values(0, i32::MAX, i32::MAX, i32::MAX, usize::MAX),
            Err(ShmBufferResourceRejection::InvalidDimensions)
        );
        let accepted = validate_xrgb8888_metadata_values(0, 2, 2, 8, 16)
            .expect("已知 2×2 XRGB byte range 必须通过");
        assert_eq!(accepted.format, Format::Xrgb8888);
        assert_eq!(
            (accepted.width, accepted.height, accepted.stride),
            (2, 2, 8)
        );
    }

    /// R2 Red：handler state 必须拥有独立于 Core 的 SHM resource queue，初始不得
    /// 伪造 pending buffer。后续真实 commit 才允许向该唯一 owner 入队。
    #[test]
    fn linux_xdg_shell_starts_with_no_pending_shm_buffer_resources() {
        let state = LinuxXdgShellStateSkeleton::new();
        assert_eq!(state.pending_shm_buffer_resource_count(), 0);
    }
    use super::{LinuxXdgShellCompileBlocker, linux_xdg_shell_readiness_report};

    /// 编译期证明 global、request handler 与 state owner 已连接。
    #[test]
    fn linux_xdg_shell_handler_traits_compile_for_wayland_state() {
        fn assert_handler<T: XdgShellHandler>() {}
        fn assert_global<T: GlobalDispatch<XdgWmBase, ()>>() {}
        fn assert_toplevel_dispatch<T: Dispatch<XdgToplevel, XdgShellSurfaceUserData>>() {}

        assert_handler::<LinuxXdgShellStateSkeleton>();
        assert_global::<LinuxXdgShellStateSkeleton>();
        assert_toplevel_dispatch::<LinuxXdgShellStateSkeleton>();
    }

    /// 编译期证明 compositor handler、global 与 surface dispatch 已连接。
    #[test]
    fn linux_wl_compositor_handler_traits_compile_for_wayland_state() {
        fn assert_handler<T: CompositorHandler>() {}
        fn assert_global<T: GlobalDispatch<WlCompositor, ()>>() {}
        fn assert_surface_dispatch<T: Dispatch<WlSurface, SurfaceUserData>>() {}

        assert_handler::<LinuxXdgShellStateSkeleton>();
        assert_global::<LinuxXdgShellStateSkeleton>();
        assert_surface_dispatch::<LinuxXdgShellStateSkeleton>();
    }

    /// Red：真实 inserted client 创建 `xdg_toplevel` 时，handler owner 必须能够从
    /// `ToplevelSurface -> WlSurface -> Resource::client() -> NestedClientDataOwner` 读取
    /// 原始 adapter session。它不触碰 Core，也不把 callback observation 外推为完整
    /// lifecycle；这里只锁定后续 queue 所需的身份读取边界。
    #[test]
    fn phase56q_real_toplevel_callback_reads_inserted_client_session() {
        assert_runtime_dir();
        let mut display = SmithayWaylandDisplayProbe::new().expect("测试 display 必须能够构造");
        display
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");
        display
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");

        let _registration = adapter_toplevel_identity_registration_report(&mut display)
            .expect("受控 client 必须真实创建 xdg_toplevel 并触发 callback");

        assert_eq!(
            display.last_toplevel_callback_session(),
            NestedClientSessionId::new(58),
            "callback session 必须保持 controlled harness 插入时分配的 identity"
        );
    }

    /// Red：handler 发布给 coordinator 的只读 admission observation 必须保留真实
    /// callback 解析出的 session，不能只保留 ObjectId 派生的 adapter identity。
    #[test]
    fn phase56q_live_admission_observation_keeps_real_client_session() {
        assert_runtime_dir();
        let mut display = SmithayWaylandDisplayProbe::new().expect("测试 display 必须能够构造");
        display
            .initialize_wl_compositor_global()
            .expect("测试 wl_compositor global 必须初始化");
        display
            .initialize_xdg_shell_global()
            .expect("测试 xdg-shell global 必须初始化");
        adapter_toplevel_identity_registration_report(&mut display)
            .expect("受控 client 必须触发真实 new_toplevel callback");

        let observation = display
            .take_next_live_toplevel_admission_observation()
            .expect("真实 callback admission 必须位于 lifecycle FIFO 队首");
        assert_eq!(
            observation.source_session,
            NestedClientSessionId::new(58),
            "Red 依据：callback queue 目前尚未携带真实 client session"
        );
    }

    /// 编译边界不得夸大 callback、runtime、dispatch、render 或 input。
    #[test]
    fn linux_xdg_shell_readiness_keeps_runtime_false() {
        let report = linux_xdg_shell_readiness_report();

        assert!(report.linux_xdg_shell_module_available);
        assert!(report.xdg_shell_global_compile_boundary_available);
        assert!(report.xdg_request_handler_compile_boundary_available);
        assert!(report.xdg_toplevel_identity_hook_point_available);
        assert!(!report.xdg_unmap_callback_observed);
        assert!(!report.ledger_unmap_invoked_from_linux_boundary);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert_eq!(
            report.blockers,
            vec![
                LinuxXdgShellCompileBlocker::MissingGlobalInitialization,
                LinuxXdgShellCompileBlocker::MissingAdapterToplevelIdentityMapping,
                LinuxXdgShellCompileBlocker::MissingToplevelLifecycleBridge,
                LinuxXdgShellCompileBlocker::MissingLedgerCallerOwnership,
                LinuxXdgShellCompileBlocker::MissingPopupSeatHandlerBoundary,
            ]
        );
    }
}
