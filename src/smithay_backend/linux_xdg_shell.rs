//! Linux-only xdg-shell global 与 request-handler 编译边界。
//!
//! 本模块只在 Linux + `smithay-linux` 下可见。它把 Smithay 的真实
//! `XdgShellHandler` / `GlobalDispatch` / `Dispatch` trait 实现定位到 display 内部的
//! `LinuxXdgShellStateSkeleton`。Phase 52I 允许配对 display owner 显式调用
//! `XdgShellState::new`；构造时仍不自动初始化，也不把 global 初始化解释为 dispatch。

use std::collections::VecDeque;

use smithay::reexports::wayland_protocols::xdg::shell::server::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::XdgPositioner,
    xdg_surface::XdgSurface,
    xdg_toplevel::XdgToplevel,
    xdg_wm_base::XdgWmBase,
};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, DataInit, Dispatch, DisplayHandle};
use smithay::utils::Serial;
use smithay::wayland::compositor::{CompositorClientState, CompositorHandler, CompositorState};
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgPositionerUserData, XdgShellHandler,
    XdgShellState, XdgShellSurfaceUserData, XdgSurfaceUserData, XdgWmBaseUserData,
};

use super::client_insert::NestedClientDataOwner;
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
    surface_identities: LinuxWlSurfaceIdentityRegistry,
    toplevel_identities: LinuxXdgToplevelIdentityRegistry,
    last_toplevel_identity_registration:
        Option<Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError>>,
    last_toplevel_lifecycle_observation: Option<XdgToplevelLifecycleObservationReport>,
    pending_live_toplevel_unmap_observations: VecDeque<XdgToplevelLifecycleObservationReport>,
    new_toplevel_callback_count: u64,
    last_new_toplevel_callback_observation_sequence: Option<u64>,
    pending_live_toplevel_admission_observations: VecDeque<PendingLiveToplevelAdmissionObservation>,
}

/// Display owner 保存的待消费 live admission observation。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingLiveToplevelAdmissionObservation {
    /// `new_toplevel` callback 的单调 observation 序号。
    pub new_toplevel_callback_sequence: u64,
    /// 同一 callback 内产生的 adapter toplevel identity registration observation。
    pub adapter_toplevel_identity_registration:
        Result<XdgToplevelIdentityMapping, AdapterToplevelIdentityRegistrationError>,
}

/// Linux-only xdg-shell global 显式初始化的结构化错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgShellGlobalInitError {
    /// 当前 owner 已持有 `XdgShellState`；重复注册同一 global 被拒绝。
    AlreadyInitialized,
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
            surface_identities: LinuxWlSurfaceIdentityRegistry::new(),
            toplevel_identities: LinuxXdgToplevelIdentityRegistry::new(),
            last_toplevel_identity_registration: None,
            last_toplevel_lifecycle_observation: None,
            pending_live_toplevel_unmap_observations: VecDeque::new(),
            new_toplevel_callback_count: 0,
            last_new_toplevel_callback_observation_sequence: None,
            pending_live_toplevel_admission_observations: VecDeque::new(),
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

    /// 返回最近一次 callback-like lifecycle identity lookup 报告。
    ///
    /// `Some` 只说明 handler 方法执行了 observation helper；报告中的
    /// `callback_observed` 仍需独立 runtime proof，不能由本 accessor 推导为 true。
    pub fn last_toplevel_lifecycle_observation(
        &self,
    ) -> Option<&XdgToplevelLifecycleObservationReport> {
        self.last_toplevel_lifecycle_observation.as_ref()
    }

    /// 消费下一条 `toplevel_destroyed` lifecycle observation。
    ///
    /// Handler 只把真实 callback 解析为纯数据 observation；是否触发 ledger/core unmap
    /// 由同时拥有 runtime ledger 与 `State` 的上层 owner 决定。
    pub(crate) fn take_next_live_toplevel_unmap_observation(
        &mut self,
    ) -> Option<XdgToplevelLifecycleObservationReport> {
        self.pending_live_toplevel_unmap_observations.pop_front()
    }

    /// 返回 server handler 收到的 `new_toplevel` callback 次数。
    pub(crate) const fn new_toplevel_callback_observation_count(&self) -> u64 {
        self.new_toplevel_callback_count
    }

    /// 返回最近一次 `new_toplevel` callback 的纯数据观察序号。
    pub(crate) const fn last_new_toplevel_callback_observation_sequence(&self) -> Option<u64> {
        self.last_new_toplevel_callback_observation_sequence
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

    pub(crate) fn take_next_live_toplevel_admission_observation(
        &mut self,
    ) -> Option<PendingLiveToplevelAdmissionObservation> {
        self.pending_live_toplevel_admission_observations
            .pop_front()
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
    ) {
        self.pending_live_toplevel_admission_observations.push_back(
            PendingLiveToplevelAdmissionObservation {
                new_toplevel_callback_sequence,
                adapter_toplevel_identity_registration,
            },
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
        let callback_sequence = self.record_new_toplevel_callback_observation();
        let registration = self.register_new_toplevel_identity(&surface);
        self.record_pending_live_toplevel_admission_observation(callback_sequence, registration);
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
        self.pending_live_toplevel_unmap_observations
            .push_back(report);
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
        // commit observation 只记录 adapter-owned surface identity；不检查 buffer/damage，
        // 不发 frame callback，不创建 xdg lifecycle，也不触发 admission ledger/core。
        let _ = self.surface_identities.observe_surface_commit(surface);
    }
}

smithay::delegate_compositor!(LinuxXdgShellStateSkeleton);

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
    use smithay::reexports::wayland_protocols::xdg::shell::server::{
        xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
    };
    use smithay::reexports::wayland_server::protocol::{
        wl_compositor::WlCompositor, wl_surface::WlSurface,
    };
    use smithay::reexports::wayland_server::{Dispatch, GlobalDispatch};
    use smithay::wayland::compositor::{CompositorHandler, SurfaceUserData};
    use smithay::wayland::shell::xdg::{XdgShellHandler, XdgShellSurfaceUserData};

    use super::LinuxXdgShellStateSkeleton;
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
