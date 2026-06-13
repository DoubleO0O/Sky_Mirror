//! Smithay 后端驱动接口探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不接真实 Smithay 回调、不接真实 client、不调用 `insert_client`，
//! 也不把 Wayland socket 插入 calloop。
//!
//! 该模块用于证明未来 Smithay backend 可以实现核心层的 `BackendDriver` trait，
//! 并且只能通过 `BackendEvent` 把外部事实交给核心状态。

use std::collections::VecDeque;

use crate::{
    core::{
        backend_driver::{BackendDriver, BackendDriverPoll},
        backend_event::BackendEvent,
        client::ClientId,
        surface::SurfaceId,
    },
    smithay_backend::action_event::{SmithayActionEventProbe, SmithayActionRequestDescriptor},
    smithay_backend::client_event::{
        SmithayClientConnectionDescriptor, SmithayClientConnectionProbe,
    },
    smithay_backend::diagnostic_event::{
        SmithayDiagnosticEventProbe, SmithayDiagnosticRequestDescriptor,
    },
    smithay_backend::output_event::{SmithayOutputEventProbe, SmithayOutputResizeDescriptor},
    smithay_backend::surface_event::{SmithaySurfaceCreationDescriptor, SmithaySurfaceEventProbe},
    smithay_backend::toplevel_event::{SmithayToplevelEventProbe, SmithayToplevelMapDescriptor},
};

/// Smithay 后端驱动探针当前模式。
///
/// 当前只允许 `ProbeOnly`，表示驱动只从内部队列产出纯数据 `BackendEvent`，
/// 不连接真实 Wayland client，也不启动真实 compositor。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayBackendDriverMode {
    /// 纯探针模式。
    ///
    /// 不插入 calloop，不接 client，也不调用 `DisplayHandle::insert_client`。
    ProbeOnly,
}

/// Smithay 后端驱动接口探针。
///
/// 该结构实现核心 `BackendDriver` trait，但当前只维护一个 `BackendEvent` 队列。
/// 未来真实 Smithay 回调应先转换为 `BackendEvent`，再由该类 driver 产出给
/// `BackendDriverRunner`。
///
/// 它不持有或直接修改核心 `State`，也不保存真实 Smithay client。
pub struct SmithayBackendDriverProbe {
    /// 当前驱动模式。
    mode: SmithayBackendDriverMode,

    /// 预设后端事件队列。
    ///
    /// 当前阶段用于测试 Smithay driver 到核心 `BackendDriverRunner` 的路径。
    pending_events: VecDeque<BackendEvent>,

    /// 是否已经请求关闭。
    ///
    /// 当前只影响 `poll_event` 返回 `ShutdownRequested`，不修改核心
    /// `State.running`。
    shutdown_requested: bool,
}

impl SmithayBackendDriverProbe {
    /// 创建纯探针模式的 Smithay 后端驱动。
    ///
    /// 当前不会创建 socket、display、client 或真实 Smithay 状态。
    pub fn new_probe_only() -> Self {
        Self {
            mode: SmithayBackendDriverMode::ProbeOnly,
            pending_events: VecDeque::new(),
            shutdown_requested: false,
        }
    }

    /// 使用预设事件创建探针驱动。
    ///
    /// 该方法用于测试多事件轮询顺序，不会直接执行事件或修改核心状态。
    pub fn with_events(events: impl IntoIterator<Item = BackendEvent>) -> Self {
        Self {
            mode: SmithayBackendDriverMode::ProbeOnly,
            pending_events: events.into_iter().collect(),
            shutdown_requested: false,
        }
    }

    /// 追加一条纯数据后端事件。
    ///
    /// 未来真实 Smithay 回调应复用相同边界，把回调转换为 `BackendEvent` 后入队，
    /// 而不是直接修改核心 `State`。
    pub fn push_event(&mut self, event: BackendEvent) {
        self.pending_events.push_back(event);
    }

    /// 模拟一个动作请求事件。
    ///
    /// 当前只把描述信息转换为 `BackendEvent::ActionRequested` 后入队，
    /// 不直接调用核心状态的动作分发入口，也不接真实输入设备。真正执行动作的
    /// 逻辑，要等 `run_once()` 后由核心处理。
    pub fn push_action_requested(&mut self, descriptor: SmithayActionRequestDescriptor) {
        self.push_event(SmithayActionEventProbe::action_requested_event(descriptor));
    }

    /// 模拟一个诊断请求事件。
    ///
    /// 当前只把描述信息转换为 `DebugRequested` 或 `ValidateRequested` 后入队，
    /// 不直接读取 `State`，也不直接生成诊断文本。真正生成文本的逻辑，要等
    /// `run_once()` 后由核心处理。
    pub fn push_diagnostic_requested(&mut self, descriptor: SmithayDiagnosticRequestDescriptor) {
        self.push_event(SmithayDiagnosticEventProbe::diagnostic_requested_event(
            descriptor,
        ));
    }

    /// 模拟一个完整诊断文本请求事件。
    ///
    /// 真正的诊断包文本会在 `run_once()` 后由核心生成。
    pub fn push_debug_requested(&mut self) {
        self.push_event(SmithayDiagnosticEventProbe::debug_requested_event());
    }

    /// 模拟一个状态验证请求事件。
    ///
    /// 真正的验证文本会在 `run_once()` 后由核心生成。
    pub fn push_validate_requested(&mut self) {
        self.push_event(SmithayDiagnosticEventProbe::validate_requested_event());
    }

    /// 模拟一个 Wayland client 连接事件。
    ///
    /// 当前只把描述信息转换为 `BackendEvent::ClientConnected` 后入队，
    /// 不接真实 client，也不调用 `insert_client`。
    pub fn push_client_connected(&mut self, descriptor: SmithayClientConnectionDescriptor) {
        self.push_event(SmithayClientConnectionProbe::client_connected_event(
            descriptor,
        ));
    }

    /// 模拟一个 Wayland client 断开事件。
    ///
    /// 当前只生成 `BackendEvent::ClientDisconnected` 并入队，真正的 surface 和
    /// window 级联关闭由核心纯数据状态处理。
    pub fn push_client_disconnected(&mut self, client: ClientId) {
        self.push_event(SmithayClientConnectionProbe::client_disconnected_event(
            client,
        ));
    }

    /// 模拟一个 Wayland surface 创建事件。
    ///
    /// 当前只把描述信息转换为 `BackendEvent::SurfaceCreated` 后入队，
    /// 不保存真实 `wl_surface`，也不注册 `wl_compositor`。
    pub fn push_surface_created(&mut self, descriptor: SmithaySurfaceCreationDescriptor) {
        self.push_event(SmithaySurfaceEventProbe::surface_created_event(descriptor));
    }

    /// 模拟一个 Wayland surface 关闭事件。
    ///
    /// 当前只把 surface ID 转换为 `BackendEvent::SurfaceClosed` 后入队，
    /// 不保存真实 `wl_surface`，也不直接修改核心 `State`。本阶段不接
    /// xdg-shell，也不注册 `wl_compositor`；真正关闭 surface 和绑定窗口仍由
    /// `run_once()` 后的核心链路处理。
    pub fn push_surface_closed(&mut self, surface: SurfaceId) {
        self.push_event(SmithaySurfaceEventProbe::surface_closed_event(surface));
    }

    /// 模拟一个输出尺寸变化事件。
    ///
    /// 当前只把描述信息转换为 `BackendEvent::OutputResized` 后入队，
    /// 不接真实 DRM 或 Winit 输出，也不直接修改核心 `State`。真正修改输出尺寸
    /// 并影响后续布局和渲染帧的逻辑，要等 `run_once()` 后由核心处理。
    pub fn push_output_resized(&mut self, descriptor: SmithayOutputResizeDescriptor) {
        self.push_event(SmithayOutputEventProbe::output_resized_event(descriptor));
    }

    /// 模拟一个 Wayland toplevel map 事件。
    ///
    /// 当前只把描述信息转换为 `BackendEvent::ToplevelMapped` 后入队，
    /// 不保存真实 `xdg_toplevel`，也不接 xdg-shell。
    pub fn push_toplevel_mapped(&mut self, descriptor: SmithayToplevelMapDescriptor) {
        self.push_event(SmithayToplevelEventProbe::toplevel_mapped_event(descriptor));
    }

    /// 请求驱动关闭。
    ///
    /// 当前只让下一次没有待处理事件的轮询返回 `ShutdownRequested`，
    /// 不修改核心 `State`。
    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    /// 返回当前驱动模式。
    pub fn mode(&self) -> SmithayBackendDriverMode {
        self.mode
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only(&self) -> bool {
        self.mode == SmithayBackendDriverMode::ProbeOnly
    }

    /// 返回尚未处理的事件数量。
    pub fn pending_event_count(&self) -> usize {
        self.pending_events.len()
    }

    /// 返回当前阶段说明。
    pub fn mode_description(&self) -> &'static str {
        "smithay-backend-driver-probe-only"
    }
}

impl BackendDriver for SmithayBackendDriverProbe {
    /// 轮询一条后端事件。
    ///
    /// 优先返回队列中的 `BackendEvent`；如果队列为空但已请求关闭，则返回
    /// `ShutdownRequested`；否则返回 `NoEvent`。该方法只产出事件，不调用
    /// 核心命令入口，也不直接修改 `State`。
    fn poll_event(&mut self) -> BackendDriverPoll {
        if let Some(event) = self.pending_events.pop_front() {
            BackendDriverPoll::Event(event)
        } else if self.shutdown_requested {
            BackendDriverPoll::ShutdownRequested
        } else {
            BackendDriverPoll::NoEvent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayBackendDriverMode, SmithayBackendDriverProbe};
    use crate::{
        core::{
            action::Action,
            backend_driver::{BackendDriver, BackendDriverPoll, BackendDriverRunner},
            backend_event::BackendEvent,
            client::ClientKind,
            command::CommandResult,
            state::State,
            surface::SurfaceRole,
            window::WindowKind,
        },
        smithay_backend::{
            action_event::SmithayActionRequestDescriptor,
            client_event::SmithayClientConnectionDescriptor,
            output_event::SmithayOutputResizeDescriptor,
            surface_event::SmithaySurfaceCreationDescriptor,
            toplevel_event::SmithayToplevelMapDescriptor,
        },
    };

    /// 验证 Smithay 后端驱动固定处于纯探针模式。
    #[test]
    fn smithay_backend_driver_probe_mode_is_probe_only() {
        let driver = SmithayBackendDriverProbe::new_probe_only();

        assert!(driver.is_probe_only());
        assert_eq!(driver.mode(), SmithayBackendDriverMode::ProbeOnly);
        assert_eq!(
            driver.mode_description(),
            "smithay-backend-driver-probe-only"
        );
    }

    /// 验证空事件队列会返回无事件，不产生任何核心状态修改。
    #[test]
    fn smithay_backend_driver_probe_without_events_returns_no_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        assert_eq!(driver.poll_event(), BackendDriverPoll::NoEvent);
    }

    /// 验证驱动会按队列顺序返回预设的纯数据后端事件。
    #[test]
    fn smithay_backend_driver_probe_returns_queued_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_event(BackendEvent::OutputResized {
            width: 1280,
            height: 720,
        });

        let poll = driver.poll_event();

        assert_eq!(
            poll,
            BackendDriverPoll::Event(BackendEvent::OutputResized {
                width: 1280,
                height: 720,
            })
        );
        assert_eq!(driver.pending_event_count(), 0);
    }

    /// 验证动作请求 helper 只把适配后的 ActionRequested 事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_action_requested_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_action_requested(SmithayActionRequestDescriptor::next_workspace());

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::ActionRequested(Action::NextWorkspace))
        );
    }

    /// 验证完整诊断请求 helper 只把 DebugRequested 事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_debug_requested_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_debug_requested();

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::DebugRequested)
        );
    }

    /// 验证状态验证请求 helper 只把 ValidateRequested 事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_validate_requested_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_validate_requested();

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::ValidateRequested)
        );
    }

    /// 验证 client 连接 helper 会把适配后的连接事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_client_connected_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_client_connected(
            SmithayClientConnectionDescriptor::with_client_id(7).with_name("app"),
        );

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::ClientConnected {
                client: Some(7),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("app".to_string()),
            })
        );
    }

    /// 验证 client 断开 helper 会把断开事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_client_disconnected_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_client_disconnected(7);

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::ClientDisconnected { client: 7 })
        );
    }

    /// 验证 surface 创建 helper 会把适配后的 SurfaceCreated 事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_surface_created_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_surface_created(SmithaySurfaceCreationDescriptor::for_client(
            42,
            7,
            SurfaceRole::XdgToplevel,
        ));

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::SurfaceCreated {
                surface: 42,
                client: Some(7),
                role: SurfaceRole::XdgToplevel,
            })
        );
    }

    /// 验证 driver helper 只把 SurfaceClosed 事件入队，不调用核心关闭方法。
    #[test]
    fn smithay_backend_driver_probe_push_surface_closed_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_surface_closed(42);

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::SurfaceClosed { surface: 42 })
        );
    }

    /// 验证输出尺寸 helper 只把适配后的 OutputResized 事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_output_resized_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_output_resized(SmithayOutputResizeDescriptor::new(2560, 1440));

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::OutputResized {
                width: 2560,
                height: 1440,
            })
        );
    }

    /// 验证 toplevel map helper 会把适配后的 ToplevelMapped 事件加入驱动队列。
    #[test]
    fn smithay_backend_driver_probe_push_toplevel_mapped_queues_event() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.push_toplevel_mapped(SmithayToplevelMapDescriptor::new(
            42,
            "Terminal",
            Some("foot".to_string()),
        ));

        assert_eq!(
            driver.poll_event(),
            BackendDriverPoll::Event(BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            })
        );
    }

    /// 验证关闭请求只通过轮询结果暴露，不直接修改核心状态。
    #[test]
    fn smithay_backend_driver_probe_shutdown_returns_shutdown_requested() {
        let mut driver = SmithayBackendDriverProbe::new_probe_only();

        driver.request_shutdown();

        assert_eq!(driver.poll_event(), BackendDriverPoll::ShutdownRequested);
    }

    /// 验证探针事件可以通过 BackendDriverRunner 进入统一核心状态链路。
    #[test]
    fn smithay_backend_driver_probe_works_with_backend_driver_runner() {
        let mut state = State::new();
        let mut driver = SmithayBackendDriverProbe::with_events([BackendEvent::OutputResized {
            width: 1440,
            height: 900,
        }]);

        let report = BackendDriverRunner::run_once(&mut state, &mut driver);

        // Runner 必须处理事件并保留事件执行后的有效状态。
        assert!(report.handled_event());
        assert!(report.is_valid());

        let output = state.compositor.current_output_size();

        // 输出变化证明驱动只产出事件，状态修改由既有 Runner 链路完成。
        assert_eq!(output.width, 1440);
        assert_eq!(output.height, 900);
    }

    /// 验证探针可以按顺序产出完整 client、surface 和 window 生命周期事件。
    #[test]
    fn smithay_backend_driver_probe_can_emit_client_lifecycle_events() {
        let mut state = State::new();
        let mut driver = SmithayBackendDriverProbe::with_events([
            BackendEvent::ClientConnected {
                client: Some(7),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("app".to_string()),
            },
            BackendEvent::SurfaceCreated {
                surface: 42,
                client: Some(7),
                role: SurfaceRole::XdgToplevel,
            },
            BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            },
            BackendEvent::ClientDisconnected { client: 7 },
        ]);

        let connected = BackendDriverRunner::run_once(&mut state, &mut driver);
        let created = BackendDriverRunner::run_once(&mut state, &mut driver);
        let mapped = BackendDriverRunner::run_once(&mut state, &mut driver);

        // 每个外部事实都必须经过 Runner，探针自身不直接修改 State。
        assert!(connected.is_valid());
        assert!(created.is_valid());
        assert!(mapped.is_valid());

        let Some(mapped_result) = mapped.runtime_result else {
            panic!("窗口 map 事件必须产生运行时结果");
        };
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped_result.result
        else {
            panic!("窗口 map 事件必须注册 surface 对应窗口");
        };
        assert!(bound);

        let disconnected = BackendDriverRunner::run_once(&mut state, &mut driver);

        // client 断开仍由现有核心级联规则处理，驱动只负责产出断开事件。
        assert!(disconnected.is_valid());
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
        assert_eq!(driver.pending_event_count(), 0);
    }
}
