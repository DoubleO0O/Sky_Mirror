//! Smithay 纯数据运行时探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。它组合纯数据
//! `BackendDriver`、client ID 分配器和 surface ID 分配器，并通过
//! `BackendDriverRunner` 进入核心状态。
//!
//! 它不构造 Display 或 socket，不依赖 Smithay crate，不接真实 client，也不直接
//! 修改核心 `State`。Linux 系统资源组合由 `linux_runtime` 模块负责。

use crate::{
    core::{
        action::Action,
        backend_driver::{BackendDriverRunReport, BackendDriverRunner},
        backend_event::BackendEvent,
        client::ClientId,
        state::State,
        surface::{SurfaceId, SurfaceRole},
    },
    smithay_backend::{
        action_event::SmithayActionRequestDescriptor,
        client_event::SmithayClientConnectionDescriptor,
        client_id::{SmithayClientIdAllocatorMode, SmithayClientIdAllocatorProbe},
        diagnostic_event::SmithayDiagnosticRequestDescriptor,
        driver::{SmithayBackendDriverMode, SmithayBackendDriverProbe},
        output_event::SmithayOutputResizeDescriptor,
        surface_event::SmithaySurfaceCreationDescriptor,
        surface_id::{SmithaySurfaceIdAllocatorMode, SmithaySurfaceIdAllocatorProbe},
        toplevel_event::SmithayToplevelMapDescriptor,
    },
};

#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
use crate::smithay_backend::bootstrap::{SmithayBootstrapMode, SmithayBootstrapProbe};

/// Smithay runtime 当前运行模式。
///
/// 当前只允许 `ProbeOnly`，表示 runtime 只组合纯数据 driver 与 ID 分配器，
/// 不进入真实 Wayland compositor 主循环。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayRuntimeMode {
    /// 纯探针模式。
    ///
    /// 不插入 calloop，不接 client，也不调用 `DisplayHandle::insert_client`。
    ProbeOnly,
}

/// Smithay 运行时组合探针。
///
/// 该结构只持有 backend driver probe 和两个核心 ID 分配器，不持有 Display、
/// listening socket 或其他系统资源。
///
/// 所有真实 Smithay 回调未来都必须先转换为 `BackendEvent`，再由 driver 产出并
/// 通过 `BackendDriverRunner` 进入核心。该结构不会直接修改 `State`。
pub struct SmithayRuntimeProbe {
    /// 旧 Linux runtime API 使用的 bootstrap 兼容资源。
    ///
    /// 该字段只在 `smithay-linux` 下存在。纯 `smithay-probe` 构建不会引用
    /// Smithay crate，也不会保存 Display 或 listening socket。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    compatibility_bootstrap: Option<SmithayBootstrapProbe>,

    /// Smithay 后端驱动接口探针。
    ///
    /// 当前只维护 `BackendEvent` 队列，不直接修改 `State`。
    driver: SmithayBackendDriverProbe,

    /// Smithay client ID 分配器探针。
    ///
    /// 当前只用于生成核心纯数据 `ClientId`，不注册真实 Wayland client。
    client_id_allocator: SmithayClientIdAllocatorProbe,

    /// Smithay surface ID 分配器探针。
    ///
    /// 当前只用于生成核心纯数据 `SurfaceId`，不注册真实 `wl_surface`。
    surface_id_allocator: SmithaySurfaceIdAllocatorProbe,

    /// 当前 runtime 模式。
    mode: SmithayRuntimeMode,
}

impl SmithayRuntimeProbe {
    /// 创建纯数据 Smithay runtime 探针。
    ///
    /// 该构造器不会读取环境变量，也不会构造 Display、socket 或真实 client。
    pub fn new_probe_only() -> Self {
        Self {
            #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
            compatibility_bootstrap: None,
            driver: SmithayBackendDriverProbe::new_probe_only(),
            client_id_allocator: SmithayClientIdAllocatorProbe::new(),
            surface_id_allocator: SmithaySurfaceIdAllocatorProbe::new(),
            mode: SmithayRuntimeMode::ProbeOnly,
        }
    }

    /// 兼容旧调用方式创建纯数据 runtime。
    ///
    /// `new_auto` 在纯 `smithay-probe` 中不构造真实 socket 或 Display。
    /// Linux feature 下保留旧行为并持有 bootstrap，以保证原有公共 API 可继续使用；
    /// 新代码仍应优先使用 `SmithayLinuxRuntimeProbe` 组合系统资源。
    pub fn new_auto() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
        {
            return Ok(Self::from_parts(
                SmithayBootstrapProbe::new_auto()?,
                SmithayBackendDriverProbe::new_probe_only(),
            ));
        }

        #[cfg(not(all(feature = "smithay-linux", target_os = "linux")))]
        Ok(Self::new_probe_only())
    }

    /// 使用指定 socket 名称创建旧式 Linux runtime。
    ///
    /// 该兼容入口只在 `smithay-linux` 下提供。它不会启动 compositor、接收 client
    /// 或绕过 `BackendDriverRunner`；新代码应优先使用 `SmithayLinuxRuntimeProbe`。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::from_parts(
            SmithayBootstrapProbe::with_socket_name(name)?,
            SmithayBackendDriverProbe::new_probe_only(),
        ))
    }

    /// 使用旧式 bootstrap 与 driver 构造 Linux runtime。
    ///
    /// 该方法保留原公共签名。bootstrap 只承担资源兼容，所有事件仍由传入的
    /// driver 产出，并通过 `BackendDriverRunner` 进入核心。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    pub fn from_parts(bootstrap: SmithayBootstrapProbe, driver: SmithayBackendDriverProbe) -> Self {
        Self {
            compatibility_bootstrap: Some(bootstrap),
            driver,
            client_id_allocator: SmithayClientIdAllocatorProbe::new(),
            surface_id_allocator: SmithaySurfaceIdAllocatorProbe::new(),
            mode: SmithayRuntimeMode::ProbeOnly,
        }
    }

    /// 向内部 driver 追加一条后端事件。
    ///
    /// 未来真实 Smithay callback 也应先转换为 `BackendEvent`，再进入 driver
    /// 队列，不能绕过核心运行时边界直接修改 `State`。
    pub fn push_event(&mut self, event: BackendEvent) {
        self.driver.push_event(event);
    }

    /// 推入一个动作请求事件。
    ///
    /// 当前只把 `ActionRequested` 事件推入内部 driver，不接真实输入设备，
    /// 也不直接调用核心状态的动作分发入口。真正执行动作的逻辑，要等
    /// `run_once()` 后由核心处理。
    pub fn push_action_requested(&mut self, descriptor: SmithayActionRequestDescriptor) {
        self.driver.push_action_requested(descriptor);
    }

    /// 直接推入一个核心动作。
    ///
    /// 该辅助方法只构造动作请求描述并入队，不直接修改核心 `State`。
    pub fn push_action(&mut self, action: Action) {
        self.push_action_requested(SmithayActionRequestDescriptor::new(action));
    }

    /// 推入一个诊断请求事件。
    ///
    /// 当前只把诊断请求推入内部 driver，不读取核心 `State`。
    /// 真正生成诊断文本的逻辑，要等 `run_once()` 后由核心处理。
    pub fn push_diagnostic_requested(&mut self, descriptor: SmithayDiagnosticRequestDescriptor) {
        self.driver.push_diagnostic_requested(descriptor);
    }

    /// 推入完整诊断文本请求事件。
    ///
    /// 该辅助方法只入队，不直接读取核心状态或生成文本。
    pub fn push_debug_requested(&mut self) {
        self.driver.push_debug_requested();
    }

    /// 推入状态验证请求事件。
    ///
    /// 该辅助方法只入队，不直接运行核心 Validator。
    pub fn push_validate_requested(&mut self) {
        self.driver.push_validate_requested();
    }

    /// 推入一个输出尺寸变化事件。
    ///
    /// 当前只把 `OutputResized` 事件推入内部 driver，不接真实 DRM、Winit 或
    /// udev，也不创建真实输出。真正修改核心输出尺寸并影响后续布局和渲染帧的
    /// 逻辑，要等 `run_once()` 后由核心处理。
    pub fn push_output_resized(&mut self, descriptor: SmithayOutputResizeDescriptor) {
        self.driver.push_output_resized(descriptor);
    }

    /// 使用宽高直接推入一个输出尺寸变化事件。
    ///
    /// 该辅助方法只构造纯数据描述并入队，不直接修改核心 `State`。
    pub fn push_output_resized_size(&mut self, width: u32, height: u32) {
        self.push_output_resized(SmithayOutputResizeDescriptor::new(width, height));
    }

    /// 模拟一个 Wayland client 连接。
    ///
    /// 当前只把连接描述转换为 `BackendEvent` 后推入内部 driver。
    /// 真实 Smithay client 后续仍必须通过 backend driver 边界进入核心。
    pub fn push_client_connected(&mut self, descriptor: SmithayClientConnectionDescriptor) {
        self.driver.push_client_connected(descriptor);
    }

    /// 模拟一个 Wayland client 断开。
    ///
    /// 当前只推入 `ClientDisconnected` 事件，surface 和 window 的级联关闭由
    /// 核心纯数据层处理。
    pub fn push_client_disconnected(&mut self, client: ClientId) {
        self.driver.push_client_disconnected(client);
    }

    /// 分配一个 client descriptor 并推入 `ClientConnected` 事件。
    ///
    /// 当前只模拟未来 socket accept 后的纯数据路径，不调用 `insert_client`。
    /// 该方法只向 driver 入队；真正注册 client 仍要等 `run_once` 处理事件。
    pub fn push_allocated_client_connected(&mut self, name: Option<String>) -> ClientId {
        let descriptor = self.client_id_allocator.next_descriptor(name);
        let client = descriptor
            .client
            .expect("分配器生成的 descriptor 必须包含 client ID");

        self.push_client_connected(descriptor);

        client
    }

    /// 返回下一次将分配的 client ID。
    pub fn peek_next_client_id(&self) -> ClientId {
        self.client_id_allocator.peek_next_client_id()
    }

    /// 返回 client ID 分配器模式。
    pub fn client_id_allocator_mode(&self) -> SmithayClientIdAllocatorMode {
        self.client_id_allocator.mode()
    }

    /// 推入一个 surface 创建事件。
    ///
    /// 当前只把描述转换为 `BackendEvent` 后推入内部 driver，
    /// 不保存或注册真实 `wl_surface`。
    pub fn push_surface_created(&mut self, descriptor: SmithaySurfaceCreationDescriptor) {
        self.driver.push_surface_created(descriptor);
    }

    /// 推入一个 surface 关闭事件。
    ///
    /// 当前只把 `SurfaceClosed` 事件推入内部 driver，不保存真实 `wl_surface`，
    /// 不接 xdg-shell，也不注册 `wl_compositor`。真正关闭 surface 和绑定窗口的
    /// 逻辑，要等 `run_once()` 后由核心处理。
    ///
    /// 该事件只关闭指定 surface；`ClientDisconnected` 才会关闭 client 拥有的
    /// 所有 surface 和窗口。
    pub fn push_surface_closed(&mut self, surface: SurfaceId) {
        self.driver.push_surface_closed(surface);
    }

    /// 分配一个 surface ID，并推入 `SurfaceCreated` 事件。
    ///
    /// 当前只模拟未来 `wl_surface` 创建后的纯数据路径，不注册真实
    /// `wl_surface`。该方法只负责分配和入队，不会直接修改核心 `State`，
    /// `SurfaceCreated` 也不等于窗口已经创建。
    pub fn push_allocated_surface_created(
        &mut self,
        client: Option<ClientId>,
        role: SurfaceRole,
    ) -> SurfaceId {
        let surface = self.surface_id_allocator.next_surface_id();

        self.push_surface_created(SmithaySurfaceCreationDescriptor::new(surface, client, role));

        surface
    }

    /// 返回下一次将分配的 surface ID。
    pub fn peek_next_surface_id(&self) -> SurfaceId {
        self.surface_id_allocator.peek_next_surface_id()
    }

    /// 返回 surface ID 分配器模式。
    pub fn surface_id_allocator_mode(&self) -> SmithaySurfaceIdAllocatorMode {
        self.surface_id_allocator.mode()
    }

    /// 推入一个 toplevel map 事件。
    ///
    /// 当前只把描述转换为 `BackendEvent` 后推入内部 driver。
    /// 真正创建窗口仍然发生在 `run_once()` 触发核心处理之后。
    pub fn push_toplevel_mapped(&mut self, descriptor: SmithayToplevelMapDescriptor) {
        self.driver.push_toplevel_mapped(descriptor);
    }

    /// 使用常用 metadata 推入一个 Wayland 占位 toplevel map 事件。
    ///
    /// 该 helper 只入队事件，不直接创建 `WindowRecord`，也不修改 workspace。
    /// `SurfaceCreated` 只注册 surface；本事件经核心处理后才会创建逻辑窗口。
    pub fn push_wayland_toplevel_mapped(
        &mut self,
        surface: SurfaceId,
        title: impl Into<String>,
        app_id: Option<String>,
    ) {
        self.push_toplevel_mapped(SmithayToplevelMapDescriptor::new(surface, title, app_id));
    }

    /// 请求 runtime driver 关闭。
    ///
    /// 当前只影响 driver 下一次无待处理事件时的轮询结果，不修改核心 `State`。
    pub fn request_shutdown(&mut self) {
        self.driver.request_shutdown();
    }

    /// 运行一轮 Smithay runtime 探针。
    ///
    /// 本方法只调用 `BackendDriverRunner`，不直接修改 `State`。
    pub fn run_once(&mut self, state: &mut State) -> BackendDriverRunReport {
        BackendDriverRunner::run_once(state, &mut self.driver)
    }

    /// 返回当前 runtime 模式。
    pub fn mode(&self) -> SmithayRuntimeMode {
        self.mode
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only(&self) -> bool {
        let resources_are_probe_only = {
            #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
            {
                self.compatibility_bootstrap
                    .as_ref()
                    .map_or(true, SmithayBootstrapProbe::is_probe_only)
            }

            #[cfg(not(all(feature = "smithay-linux", target_os = "linux")))]
            {
                true
            }
        };

        self.mode == SmithayRuntimeMode::ProbeOnly
            && self.driver.is_probe_only()
            && resources_are_probe_only
    }

    /// 返回旧式 Linux bootstrap 当前模式。
    ///
    /// 纯数据构造器没有 bootstrap，但其语义仍然是 `ProbeOnly`。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    pub fn bootstrap_mode(&self) -> SmithayBootstrapMode {
        self.compatibility_bootstrap
            .as_ref()
            .map_or(SmithayBootstrapMode::ProbeOnly, SmithayBootstrapProbe::mode)
    }

    /// 返回旧式 Linux runtime 持有的 socket 名称。
    ///
    /// 对通过 `new_probe_only` 创建的实例返回空字符串，明确表示没有系统资源。
    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]
    pub fn socket_name_string(&self) -> String {
        self.compatibility_bootstrap
            .as_ref()
            .map_or_else(String::new, SmithayBootstrapProbe::socket_name_string)
    }

    /// 返回 driver 模式。
    pub fn driver_mode(&self) -> SmithayBackendDriverMode {
        self.driver.mode()
    }

    /// 返回内部 driver 尚未处理的事件数量。
    pub fn pending_event_count(&self) -> usize {
        self.driver.pending_event_count()
    }

    /// 返回当前阶段说明。
    pub fn mode_description(&self) -> &'static str {
        "smithay-runtime-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayRuntimeMode, SmithayRuntimeProbe};
    use crate::{
        core::{
            action::Action, backend_driver::BackendDriverPoll, backend_event::BackendEvent,
            client::ClientKind, command::CommandResult, state::State, surface::SurfaceRole,
            window::WindowKind,
        },
        smithay_backend::{
            action_event::SmithayActionRequestDescriptor,
            client_event::SmithayClientConnectionDescriptor,
            client_id::SmithayClientIdAllocatorMode,
            diagnostic_event::SmithayDiagnosticRequestDescriptor, driver::SmithayBackendDriverMode,
            output_event::SmithayOutputResizeDescriptor, surface_id::SmithaySurfaceIdAllocatorMode,
        },
    };

    /// 验证 runtime 模式枚举当前只包含纯探针模式。
    #[test]
    fn smithay_runtime_mode_is_probe_only() {
        assert_eq!(SmithayRuntimeMode::ProbeOnly, SmithayRuntimeMode::ProbeOnly);
    }

    /// 验证兼容构造器能够完成当前 feature 对应的 runtime 构造。
    #[test]
    fn smithay_runtime_new_auto_returns_result_without_panic() {
        let runtime = SmithayRuntimeProbe::new_auto()
            .expect("runtime 兼容构造器必须完成当前 feature 对应的初始化");

        // 无论是否带 Linux 兼容资源，状态事件仍必须保持纯探针模式。
        assert!(runtime.is_probe_only());
        assert_eq!(runtime.mode(), SmithayRuntimeMode::ProbeOnly);
        assert_eq!(runtime.driver_mode(), SmithayBackendDriverMode::ProbeOnly);
        assert_eq!(
            runtime.client_id_allocator_mode(),
            SmithayClientIdAllocatorMode::ProbeOnly
        );
        assert_eq!(
            runtime.surface_id_allocator_mode(),
            SmithaySurfaceIdAllocatorMode::ProbeOnly
        );
        assert_eq!(runtime.mode_description(), "smithay-runtime-probe-only");
    }

    /// 验证动作请求会通过 driver 和 Runner 修改核心状态。
    #[test]
    fn smithay_runtime_action_requested_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();
        let before = state.compositor.current_workspace_id();

        runtime.push_action_requested(SmithayActionRequestDescriptor::next_workspace());

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let after = state.compositor.current_workspace_id();

        assert_ne!(before, after);
    }

    /// 验证动作请求在 `run_once()` 前只停留在 driver 队列。
    #[test]
    fn smithay_runtime_action_requested_does_not_run_before_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();
        let before = state.compositor.current_workspace_id();

        runtime.push_action(Action::NextWorkspace);

        // 入队不等于执行；Runner 消费事件前当前 workspace 必须保持不变。
        assert_eq!(state.compositor.current_workspace_id(), before);
        assert_eq!(runtime.pending_event_count(), 1);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert_ne!(state.compositor.current_workspace_id(), before);
    }

    /// 验证生成窗口动作只在 `run_once()` 后增加 workspace 引用窗口。
    #[test]
    fn smithay_runtime_action_spawn_window_creates_window_after_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();
        let before_count = state
            .debug_snapshot()
            .windows
            .iter()
            .filter(|window| window.referenced_by_workspace)
            .count();

        runtime.push_action(Action::SpawnWindow);

        let report = runtime.run_once(&mut state);
        let after_count = state
            .debug_snapshot()
            .windows
            .iter()
            .filter(|window| window.referenced_by_workspace)
            .count();

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(after_count > before_count);
    }

    /// 验证关闭焦点窗口动作会在 `run_once()` 后移除对应 workspace 引用。
    #[test]
    fn smithay_runtime_action_close_focused_window_closes_window_after_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();
        let focused_window = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");

        runtime.push_action(Action::CloseFocusedWindow);

        let report = runtime.run_once(&mut state);
        let snapshot = state.debug_snapshot();
        let closed_window = snapshot
            .windows
            .iter()
            .find(|window| window.id == focused_window)
            .expect("关闭后的窗口诊断记录必须继续保留");

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(!closed_window.alive);
        assert!(!closed_window.referenced_by_workspace);
    }

    /// 验证完整诊断请求会在 `run_once()` 后返回快照与验证文本。
    #[test]
    fn smithay_runtime_debug_requested_returns_debug_text() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_debug_requested();

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let runtime_result = report
            .runtime_result
            .expect("完整诊断请求必须产生运行时结果");
        let CommandResult::Text(text) = runtime_result.result else {
            panic!("完整诊断请求必须返回文本");
        };

        assert!(text.contains("Sky Mirror Debug Snapshot"));
        assert!(text.contains("Sky Mirror Validation Report"));
        assert!(text.contains("Workspaces:"));
        assert!(text.contains("Windows:"));
    }

    /// 验证状态验证请求会在 `run_once()` 后返回验证报告文本。
    #[test]
    fn smithay_runtime_validate_requested_returns_validation_text() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_validate_requested();

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let runtime_result = report
            .runtime_result
            .expect("状态验证请求必须产生运行时结果");
        let CommandResult::Text(text) = runtime_result.result else {
            panic!("状态验证请求必须返回文本");
        };

        assert!(text.contains("Sky Mirror Validation Report"));
    }

    /// 验证状态验证请求不会修复无效状态，只返回当前 Validator 报告。
    #[test]
    fn smithay_runtime_validate_requested_reports_invalid_state_text() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_output_resized_size(0, 900);

        let resize_report = runtime.run_once(&mut state);

        assert!(resize_report.handled_event());
        assert!(!resize_report.is_valid());

        runtime.push_validate_requested();

        let validate_report = runtime.run_once(&mut state);

        assert!(validate_report.handled_event());
        assert!(!validate_report.is_valid());

        let runtime_result = validate_report
            .runtime_result
            .expect("无效状态验证请求必须产生运行时结果");
        let CommandResult::Text(text) = runtime_result.result else {
            panic!("无效状态验证请求必须返回文本");
        };

        assert!(text.contains("Sky Mirror Validation Report"));
        assert!(text.contains("InvalidOutputSize"));
    }

    /// 验证诊断请求在 `run_once()` 前只停留在 driver 队列。
    #[test]
    fn smithay_runtime_diagnostic_requested_does_not_run_before_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_diagnostic_requested(SmithayDiagnosticRequestDescriptor::debug_text());

        // 入队不等于生成文本；Runner 消费事件前队列必须仍保留该请求。
        assert_eq!(runtime.pending_event_count(), 1);

        let report = runtime.run_once(&mut state);

        assert_eq!(runtime.pending_event_count(), 0);

        let runtime_result = report.runtime_result.expect("诊断请求必须在运行后产生结果");
        assert!(matches!(runtime_result.result, CommandResult::Text(_)));
    }

    /// 验证 runtime 会通过内部 driver 和 Runner 处理输出尺寸事件。
    #[test]
    fn smithay_runtime_runs_output_event_through_driver_runner() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_event(BackendEvent::OutputResized {
            width: 1600,
            height: 900,
        });

        let report = runtime.run_once(&mut state);

        // runtime 只能经由 Runner 处理事件，并保留核心验证结果。
        assert!(report.handled_event());
        assert!(report.is_valid());

        let output = state.compositor.current_output_size();

        assert_eq!(output.width, 1600);
        assert_eq!(output.height, 900);
        assert_eq!(runtime.pending_event_count(), 0);
    }

    /// 验证输出尺寸描述会通过 driver 和 Runner 修改核心输出尺寸。
    #[test]
    fn smithay_runtime_output_resized_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_output_resized(SmithayOutputResizeDescriptor::new(2560, 1440));

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let output = state.compositor.current_output_size();

        assert_eq!(output.width, 2560);
        assert_eq!(output.height, 1440);
    }

    /// 验证输出尺寸事件在 `run_once()` 前只停留在 driver 队列。
    #[test]
    fn smithay_runtime_output_resized_does_not_run_before_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();
        let original = state.compositor.current_output_size();

        runtime.push_output_resized_size(1600, 900);

        // 入队不等于执行；Runner 消费事件前核心输出尺寸必须保持不变。
        assert_eq!(state.compositor.current_output_size(), original);
        assert_eq!(runtime.pending_event_count(), 1);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let output = state.compositor.current_output_size();

        assert_eq!(output.width, 1600);
        assert_eq!(output.height, 900);
    }

    /// 验证零尺寸不会被 Smithay 探针过滤，而由核心 Validator 报告无效状态。
    #[test]
    fn smithay_runtime_output_resized_zero_size_reports_invalid_after_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_output_resized_size(0, 900);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(!report.is_valid());

        let output = state.compositor.current_output_size();

        assert_eq!(output.width, 0);
        assert_eq!(output.height, 900);
    }

    /// 验证 runtime 的 client 连接 helper 会通过 driver 和 Runner 注册核心记录。
    #[test]
    fn smithay_runtime_push_client_connected_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_client_connected(
            SmithayClientConnectionDescriptor::with_client_id(7).with_name("app"),
        );

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(state.clients.get(7).is_some());
        assert!(state.clients.is_alive(7));
    }

    /// 验证 runtime 分配的 client 只有经过 Runner 后才进入核心注册表。
    #[test]
    fn smithay_runtime_allocated_client_connected_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));

        assert_eq!(client, 1);
        assert_eq!(runtime.peek_next_client_id(), 2);

        // 分配和入队不等于核心注册；只有 Runner 消费事件后才产生 client 记录。
        assert!(state.clients.get(client).is_none());

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(state.clients.is_alive(client));
    }

    /// 验证 runtime 可以连续分配多个 client，并按队列顺序注册到核心。
    #[test]
    fn smithay_runtime_allocates_multiple_clients() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let first = runtime.push_allocated_client_connected(Some("first".to_string()));
        let second = runtime.push_allocated_client_connected(Some("second".to_string()));

        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(runtime.peek_next_client_id(), 3);
        assert_eq!(runtime.pending_event_count(), 2);

        // 两个 descriptor 此时都只在 driver 队列中，尚未修改核心 State。
        assert!(state.clients.get(first).is_none());
        assert!(state.clients.get(second).is_none());

        let first_report = runtime.run_once(&mut state);
        let second_report = runtime.run_once(&mut state);

        assert!(first_report.handled_event());
        assert!(second_report.handled_event());
        assert!(first_report.is_valid());
        assert!(second_report.is_valid());
        assert!(state.clients.is_alive(first));
        assert!(state.clients.is_alive(second));
        assert_eq!(runtime.pending_event_count(), 0);
    }

    /// 验证 runtime 分配的 surface 只有经过 Runner 后才进入核心注册表。
    #[test]
    fn smithay_runtime_allocated_surface_created_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));
        let client_report = runtime.run_once(&mut state);

        assert!(client_report.is_valid());
        assert!(state.clients.is_alive(client));

        let surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);

        assert_eq!(surface, 1);
        assert_eq!(runtime.peek_next_surface_id(), 2);

        // 分配和入队不等于核心注册，也不会在 SurfaceCreated 阶段创建窗口。
        assert!(state.surfaces.get(surface).is_none());

        let surface_report = runtime.run_once(&mut state);

        assert!(surface_report.handled_event());
        assert!(surface_report.is_valid());

        let record = state
            .surfaces
            .get(surface)
            .expect("surface 事件处理后必须存在核心记录");

        assert_eq!(record.client, Some(client));
        assert_eq!(record.window, None);
        assert!(record.alive);
    }

    /// 验证 runtime 可以连续分配多个 surface，并按队列顺序注册到核心。
    #[test]
    fn smithay_runtime_allocates_multiple_surfaces() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let first = runtime.push_allocated_surface_created(None, SurfaceRole::Unknown);
        let second = runtime.push_allocated_surface_created(None, SurfaceRole::XdgToplevel);

        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(runtime.peek_next_surface_id(), 3);
        assert_eq!(runtime.pending_event_count(), 2);

        // 两个描述此时都只在 driver 队列中，尚未修改核心 State。
        assert!(state.surfaces.get(first).is_none());
        assert!(state.surfaces.get(second).is_none());

        let first_report = runtime.run_once(&mut state);
        let second_report = runtime.run_once(&mut state);

        assert!(first_report.handled_event());
        assert!(second_report.handled_event());
        assert!(first_report.is_valid());
        assert!(second_report.is_valid());
        assert!(state.surfaces.is_alive(first));
        assert!(state.surfaces.is_alive(second));
        assert_eq!(runtime.pending_event_count(), 0);
    }

    /// 验证 runtime 的 toplevel map helper 会通过核心创建并管理逻辑窗口。
    #[test]
    fn smithay_runtime_toplevel_mapped_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));
        assert!(runtime.run_once(&mut state).is_valid());

        let surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);
        assert!(runtime.run_once(&mut state).is_valid());

        runtime.push_wayland_toplevel_mapped(surface, "Terminal", Some("foot".to_string()));

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let window = state
            .surfaces
            .get(surface)
            .and_then(|record| record.window)
            .expect("toplevel map 处理后 surface 必须绑定逻辑窗口");
        let record = state
            .registry
            .get(window)
            .expect("toplevel map 处理后窗口注册表必须包含对应记录");

        assert_eq!(record.kind, WindowKind::WaylandPlaceholder);
        assert_eq!(record.title, "Terminal");
        assert_eq!(record.app_id, Some("foot".to_string()));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .any(|workspace| workspace.window_ids().contains(&window))
        );
    }

    /// 验证 toplevel map 事件在 `run_once()` 前只停留在 driver 队列。
    #[test]
    fn smithay_runtime_toplevel_mapped_does_not_run_before_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let surface = runtime.push_allocated_surface_created(None, SurfaceRole::XdgToplevel);
        assert!(runtime.run_once(&mut state).is_valid());

        runtime.push_wayland_toplevel_mapped(surface, "Terminal", None);

        // 入队不等于执行；核心 surface 在 Runner 处理前仍然没有窗口绑定。
        assert_eq!(
            state
                .surfaces
                .get(surface)
                .expect("surface 创建事件处理后必须存在记录")
                .window,
            None
        );
        assert_eq!(runtime.pending_event_count(), 1);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(
            state
                .surfaces
                .get(surface)
                .expect("toplevel map 处理后必须保留 surface 记录")
                .window
                .is_some()
        );
    }

    /// 验证 SurfaceClosed 会通过核心关闭目标 surface、绑定窗口和 workspace 引用。
    #[test]
    fn smithay_runtime_surface_closed_runs_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));
        assert!(runtime.run_once(&mut state).is_valid());

        let surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);
        assert!(runtime.run_once(&mut state).is_valid());

        runtime.push_wayland_toplevel_mapped(surface, "Terminal", Some("foot".to_string()));
        assert!(runtime.run_once(&mut state).is_valid());

        let window = state
            .surfaces
            .get(surface)
            .and_then(|record| record.window)
            .expect("toplevel map 处理后 surface 必须绑定逻辑窗口");

        runtime.push_surface_closed(surface);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());

        let surface_record = state
            .surfaces
            .get(surface)
            .expect("关闭后必须保留 surface 诊断记录");
        assert!(!surface_record.alive);

        let window_record = state
            .registry
            .get(window)
            .expect("关闭后必须保留窗口诊断记录");
        assert!(!window_record.alive);

        let snapshot = state.debug_snapshot();
        let debug_window = snapshot
            .windows
            .iter()
            .find(|window_info| window_info.id == window)
            .expect("关闭后的窗口必须出现在调试快照中");

        assert!(!debug_window.referenced_by_workspace);
    }

    /// 验证 SurfaceClosed 在 `run_once()` 前只停留在 driver 队列。
    #[test]
    fn smithay_runtime_surface_closed_does_not_run_before_run_once() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let surface = runtime.push_allocated_surface_created(None, SurfaceRole::XdgToplevel);
        assert!(runtime.run_once(&mut state).is_valid());

        runtime.push_wayland_toplevel_mapped(surface, "Terminal", None);
        assert!(runtime.run_once(&mut state).is_valid());

        let window = state
            .surfaces
            .get(surface)
            .and_then(|record| record.window)
            .expect("toplevel map 处理后 surface 必须绑定逻辑窗口");

        runtime.push_surface_closed(surface);

        // 入队不等于执行；Runner 消费事件前两条核心记录必须仍然存活。
        assert!(state.surfaces.is_alive(surface));
        assert!(state.registry.is_alive(window));
        assert_eq!(runtime.pending_event_count(), 1);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(!state.surfaces.is_alive(surface));
        assert!(!state.registry.is_alive(window));
    }

    /// 验证 SurfaceClosed 只关闭目标 surface，不级联关闭同 client 的其他资源。
    #[test]
    fn smithay_runtime_surface_closed_only_closes_target_surface() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));
        assert!(runtime.run_once(&mut state).is_valid());

        let first_surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);
        let second_surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);
        assert!(runtime.run_once(&mut state).is_valid());
        assert!(runtime.run_once(&mut state).is_valid());

        runtime.push_wayland_toplevel_mapped(first_surface, "First", None);
        runtime.push_wayland_toplevel_mapped(second_surface, "Second", None);
        assert!(runtime.run_once(&mut state).is_valid());
        assert!(runtime.run_once(&mut state).is_valid());

        let first_window = state
            .surfaces
            .get(first_surface)
            .and_then(|record| record.window)
            .expect("第一个 surface 必须绑定逻辑窗口");
        let second_window = state
            .surfaces
            .get(second_surface)
            .and_then(|record| record.window)
            .expect("第二个 surface 必须绑定逻辑窗口");

        runtime.push_surface_closed(first_surface);

        let report = runtime.run_once(&mut state);

        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(!state.surfaces.is_alive(first_surface));
        assert!(!state.registry.is_alive(first_window));
        assert!(state.surfaces.is_alive(second_surface));
        assert!(state.registry.is_alive(second_window));
        assert!(state.clients.is_alive(client));
    }

    /// 验证 surface 创建、toplevel map 和 client 断开会复用核心完整级联。
    #[test]
    fn smithay_runtime_surface_created_toplevel_mapped_then_client_disconnects() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));
        assert!(runtime.run_once(&mut state).is_valid());

        let surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);
        assert!(runtime.run_once(&mut state).is_valid());

        runtime.push_wayland_toplevel_mapped(surface, "Terminal", Some("foot".to_string()));
        assert!(runtime.run_once(&mut state).is_valid());

        let window = state
            .surfaces
            .get(surface)
            .and_then(|record| record.window)
            .expect("toplevel map 处理后 surface 必须绑定逻辑窗口");

        runtime.push_client_disconnected(client);

        let disconnected = runtime.run_once(&mut state);

        assert!(disconnected.handled_event());
        assert!(disconnected.is_valid());
        assert!(!state.clients.is_alive(client));
        assert!(!state.surfaces.is_alive(surface));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );
    }

    /// 验证分配 surface、map 窗口和 client 断开会复用核心完整生命周期级联。
    #[test]
    fn smithay_runtime_surface_created_then_mapped_then_client_disconnects() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        let client = runtime.push_allocated_client_connected(Some("app".to_string()));
        let connected = runtime.run_once(&mut state);

        assert!(connected.handled_event());
        assert!(connected.is_valid());

        let surface =
            runtime.push_allocated_surface_created(Some(client), SurfaceRole::XdgToplevel);
        let created = runtime.run_once(&mut state);

        assert!(created.handled_event());
        assert!(created.is_valid());

        runtime.push_event(BackendEvent::ToplevelMapped {
            surface,
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });

        let mapped = runtime.run_once(&mut state);

        assert!(mapped.handled_event());
        assert!(mapped.is_valid());

        let Some(mapped_result) = mapped.runtime_result else {
            panic!("窗口 map 事件必须产生运行时结果");
        };
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped_result.result
        else {
            panic!("窗口 map 事件必须注册 surface 对应窗口");
        };
        assert!(bound);

        runtime.push_client_disconnected(client);

        let disconnected = runtime.run_once(&mut state);

        assert!(disconnected.handled_event());
        assert!(disconnected.is_valid());
        assert!(!state.clients.is_alive(client));
        assert!(!state.surfaces.is_alive(surface));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );
    }

    /// 验证 runtime 的 client helper 会复用核心完整断开级联。
    #[test]
    fn smithay_runtime_push_client_disconnected_cascades_through_core() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_client_connected(
            SmithayClientConnectionDescriptor::with_client_id(7).with_name("app"),
        );
        runtime.push_event(BackendEvent::SurfaceCreated {
            surface: 42,
            client: Some(7),
            role: SurfaceRole::XdgToplevel,
        });
        runtime.push_event(BackendEvent::ToplevelMapped {
            surface: 42,
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });
        runtime.push_client_disconnected(7);

        let first = runtime.run_once(&mut state);
        let second = runtime.run_once(&mut state);
        let third = runtime.run_once(&mut state);

        assert!(first.handled_event());
        assert!(second.handled_event());
        assert!(third.handled_event());
        assert!(first.is_valid());
        assert!(second.is_valid());
        assert!(third.is_valid());

        let Some(mapped_result) = third.runtime_result else {
            panic!("窗口 map 事件必须产生运行时结果");
        };
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped_result.result
        else {
            panic!("窗口 map 事件必须注册 surface 对应窗口");
        };
        assert!(bound);

        let fourth = runtime.run_once(&mut state);

        // helper 只负责生成事件，最终生命周期收束必须仍由核心级联逻辑完成。
        assert!(fourth.handled_event());
        assert!(fourth.is_valid());
        assert!(!state.clients.is_alive(7));
        assert!(!state.surfaces.is_alive(42));
        assert!(!state.registry.is_alive(window));
    }

    /// 验证 runtime 可以按顺序处理 client、surface 和 window 生命周期事件。
    #[test]
    fn smithay_runtime_processes_client_lifecycle_events() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.push_event(BackendEvent::ClientConnected {
            client: Some(7),
            kind: ClientKind::WaylandPlaceholder,
            name: Some("app".to_string()),
        });
        runtime.push_event(BackendEvent::SurfaceCreated {
            surface: 42,
            client: Some(7),
            role: SurfaceRole::XdgToplevel,
        });
        runtime.push_event(BackendEvent::ToplevelMapped {
            surface: 42,
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });
        runtime.push_event(BackendEvent::ClientDisconnected { client: 7 });

        let first = runtime.run_once(&mut state);
        let second = runtime.run_once(&mut state);
        let third = runtime.run_once(&mut state);

        // 前三步都必须由内部 driver 产出事件，并经 Runner 完成处理。
        assert!(first.handled_event());
        assert!(second.handled_event());
        assert!(third.handled_event());
        assert!(first.is_valid());
        assert!(second.is_valid());
        assert!(third.is_valid());

        let Some(mapped_result) = third.runtime_result else {
            panic!("窗口 map 事件必须产生运行时结果");
        };
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped_result.result
        else {
            panic!("窗口 map 事件必须注册 surface 对应窗口");
        };
        assert!(bound);

        let fourth = runtime.run_once(&mut state);

        // client 断开必须继续复用既有纯数据级联规则。
        assert!(fourth.handled_event());
        assert!(fourth.is_valid());
        assert!(!state.clients.is_alive(7));
        assert!(!state.surfaces.is_alive(42));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );
        assert_eq!(runtime.pending_event_count(), 0);
    }

    /// 验证 runtime 关闭请求只产生关闭轮询报告，不直接修改核心状态。
    #[test]
    fn smithay_runtime_shutdown_request_returns_shutdown_report() {
        let mut runtime = SmithayRuntimeProbe::new_probe_only();
        let mut state = State::new();

        runtime.request_shutdown();

        let report = runtime.run_once(&mut state);

        assert!(!report.handled_event());
        assert!(report.is_valid());
        assert!(matches!(report.poll, BackendDriverPoll::ShutdownRequested));
    }
}
