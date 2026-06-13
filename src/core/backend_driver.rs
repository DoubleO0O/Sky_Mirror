//! 后端驱动到核心运行时桥接层的纯数据抽象接口。
//!
//! 本模块为未来真实 Smithay 或其他 backend 定义事件来源边界。驱动只能产出
//! `BackendEvent`，不能直接持有或修改 `State`；状态变化必须由
//! `CoreRuntimeBridge` 统一处理。本阶段不接 Smithay，不引入 Wayland 类型，
//! 也不保存真实 surface 对象。

use crate::core::{
    backend_event::BackendEvent,
    runtime_bridge::{CoreRuntimeBridge, RuntimeEventResult},
    state::State,
};

/// 后端驱动一次轮询的结果。
///
/// `BackendDriverPoll` 只描述后端本轮是否产生事件，不包含 `State` 引用，
/// 也不直接执行任何核心状态修改。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendDriverPoll {
    /// 本轮没有新的后端事件。
    NoEvent,

    /// 本轮产生了一条后端事件。
    Event(
        /// 需要交给核心运行时桥接器处理的纯数据事件。
        BackendEvent,
    ),

    /// 后端请求结束运行。
    ///
    /// 当前阶段只返回给调用方，不直接修改 `State.running`。
    ShutdownRequested,
}

/// 后端驱动抽象接口。
///
/// 未来真实 Smithay 或其他 backend 可以实现该 trait，把外部回调或底层事件
/// 转换为纯数据 `BackendEvent`。驱动不允许直接修改 `State`，所有状态修改
/// 必须经由 `CoreRuntimeBridge` 或统一核心命令边界。
pub trait BackendDriver {
    /// 轮询后端事件。
    ///
    /// 当前接口只抽象每次最多返回一个事件的简单模型，便于未来接入真实事件循环。
    fn poll_event(&mut self) -> BackendDriverPoll;
}

/// 后端驱动单轮运行报告。
///
/// 一次 `run_once` 最多处理一个 `BackendEvent`，因此报告中最多包含一个
/// `RuntimeEventResult`。报告不持有 `State` 或 driver 引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendDriverRunReport {
    /// 后端 poll 返回的原始结果。
    pub poll: BackendDriverPoll,

    /// 如果本轮产生事件并被桥接处理，这里保存处理结果。
    pub runtime_result: Option<RuntimeEventResult>,
}

impl BackendDriverRunReport {
    /// 本轮是否处理了一个事件。
    pub fn handled_event(&self) -> bool {
        self.runtime_result.is_some()
    }

    /// 本轮处理后状态是否有效。
    ///
    /// 没有事件时视为有效；有事件时读取 `RuntimeEventResult` 中的验证报告。
    pub fn is_valid(&self) -> bool {
        self.runtime_result
            .as_ref()
            .map(|result| result.validation.is_valid())
            .unwrap_or(true)
    }

    /// 将本轮运行报告格式化为人类可读文本。
    ///
    /// 该方法只读取已经保存的结果，不重新 poll、不修改状态，也不默认打印。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        output.push_str("Sky Mirror Backend Driver Run Report\n");
        output.push_str(&format!("poll: {:?}\n", self.poll));
        output.push_str(&format!("handled_event: {}\n", self.handled_event()));
        output.push_str(&format!("valid: {}\n", self.is_valid()));

        // 只有实际处理事件时，才追加运行时桥接器保存的完整单步信息。
        if let Some(runtime_result) = &self.runtime_result {
            output.push_str(&format!("event: {:?}\n", runtime_result.event));
            output.push_str(&format!("command: {:?}\n", runtime_result.command));
            output.push_str(&format!("result: {:?}\n", runtime_result.result));
            output.push_str("validation:\n");
            output.push_str(&runtime_result.validation.pretty_print());
        }

        output
    }
}

/// 后端驱动运行器。
///
/// `BackendDriverRunner` 负责把 `BackendDriver` 产生的 `BackendEvent` 交给
/// `CoreRuntimeBridge`。它不拥有 `State`，也不保存 driver，只提供单轮执行函数。
pub struct BackendDriverRunner;

impl BackendDriverRunner {
    /// 运行后端驱动的一轮 poll。
    ///
    /// 如果 driver 返回 `Event`，则通过 `CoreRuntimeBridge` 处理；如果没有事件
    /// 或请求关闭，则不修改 `State`。该方法不会默认打印报告。
    pub fn run_once<D: BackendDriver>(state: &mut State, driver: &mut D) -> BackendDriverRunReport {
        let poll = driver.poll_event();

        // 驱动只提供事件来源，所有事件状态修改统一交给单事件运行时桥接器。
        let runtime_result = match &poll {
            BackendDriverPoll::Event(event) => Some(CoreRuntimeBridge::handle_backend_event(
                state,
                event.clone(),
            )),
            BackendDriverPoll::NoEvent | BackendDriverPoll::ShutdownRequested => None,
        };

        BackendDriverRunReport {
            poll,
            runtime_result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendDriver, BackendDriverPoll, BackendDriverRunner};
    use crate::core::{
        backend_event::BackendEvent, command::CommandResult, state::State, surface::SurfaceRole,
        window::WindowKind,
    };

    /// 测试用后端驱动。
    ///
    /// 该 mock 只按顺序弹出预设 poll 结果，用于验证 `BackendDriverRunner`，
    /// 不访问或修改 `State`。
    struct MockBackendDriver {
        /// 尚未被轮询的预设结果。
        polls: Vec<BackendDriverPoll>,
    }

    impl MockBackendDriver {
        /// 使用预设轮询序列创建测试驱动。
        fn new(polls: Vec<BackendDriverPoll>) -> Self {
            Self { polls }
        }
    }

    impl BackendDriver for MockBackendDriver {
        /// 返回下一条预设结果，序列耗尽后返回无事件。
        fn poll_event(&mut self) -> BackendDriverPoll {
            if self.polls.is_empty() {
                BackendDriverPoll::NoEvent
            } else {
                self.polls.remove(0)
            }
        }
    }

    /// 验证无事件轮询不会修改核心状态。
    #[test]
    fn backend_driver_runner_no_event_does_not_modify_state() {
        let mut state = State::new();
        let before = state.debug_bundle_text();
        let mut driver = MockBackendDriver::new(vec![BackendDriverPoll::NoEvent]);

        let report = BackendDriverRunner::run_once(&mut state, &mut driver);

        // 无事件时不得生成运行时结果，但本轮仍然视为有效。
        assert!(!report.handled_event());
        assert!(report.is_valid());

        // 状态诊断文本必须保持完全一致，证明 runner 没有产生副作用。
        assert_eq!(state.debug_bundle_text(), before);
    }

    /// 验证后端事件会通过运行时桥接器进入核心状态。
    #[test]
    fn backend_driver_runner_event_goes_through_runtime_bridge() {
        let mut state = State::new();
        let mut driver = MockBackendDriver::new(vec![BackendDriverPoll::Event(
            BackendEvent::OutputResized {
                width: 1280,
                height: 720,
            },
        )]);

        let report = BackendDriverRunner::run_once(&mut state, &mut driver);

        // Event 必须产生运行时结果，并保留即时验证结论。
        assert!(report.handled_event());
        assert!(report.is_valid());
        assert!(report.runtime_result.is_some());

        let output = state.compositor.current_output_size();

        // 尺寸变化证明事件经过 CoreRuntimeBridge 和现有 Action 链路执行。
        assert_eq!(output.width, 1280);
        assert_eq!(output.height, 720);
    }

    /// 验证关闭请求只作为 poll 结果返回，不修改核心运行状态。
    #[test]
    fn backend_driver_runner_shutdown_requested_does_not_mutate_state() {
        let mut state = State::new();
        let before = state.debug_bundle_text();
        let mut driver = MockBackendDriver::new(vec![BackendDriverPoll::ShutdownRequested]);

        let report = BackendDriverRunner::run_once(&mut state, &mut driver);

        // 当前阶段关闭请求不进入核心命令，也不生成运行时结果。
        assert!(!report.handled_event());
        assert!(report.is_valid());

        // runner 不得擅自修改 State.running 或其他集中状态。
        assert_eq!(state.debug_bundle_text(), before);
    }

    /// 验证多轮轮询可以逐步完成 surface 创建、map 和关闭生命周期。
    #[test]
    fn backend_driver_runner_processes_surface_lifecycle_across_multiple_runs() {
        let mut state = State::new();
        let mut driver = MockBackendDriver::new(vec![
            BackendDriverPoll::Event(BackendEvent::SurfaceCreated {
                surface: 42,
                client: None,
                role: SurfaceRole::XdgToplevel,
            }),
            BackendDriverPoll::Event(BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            }),
            BackendDriverPoll::Event(BackendEvent::SurfaceClosed { surface: 42 }),
        ]);

        let first = BackendDriverRunner::run_once(&mut state, &mut driver);
        let second = BackendDriverRunner::run_once(&mut state, &mut driver);
        let third = BackendDriverRunner::run_once(&mut state, &mut driver);

        // 每轮必须只处理一条事件，并在处理后保持状态有效。
        assert!(first.handled_event());
        assert!(second.handled_event());
        assert!(third.handled_event());
        assert!(first.is_valid());
        assert!(second.is_valid());
        assert!(third.is_valid());

        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = &second
            .runtime_result
            .as_ref()
            .expect("map 轮必须包含运行时结果")
            .result
        else {
            panic!("map 轮必须返回 WindowRegisteredForSurface");
        };
        assert!(*bound);
        let mapped_window = *window;

        // 生命周期结束后 surface 和窗口记录都必须存在但标记为 dead。
        assert!(!state.surfaces.get(42).expect("surface 记录必须保留").alive);
        assert!(!state.registry.is_alive(mapped_window));
    }

    /// 验证单轮报告文本包含 poll、处理状态和运行时结果字段。
    #[test]
    fn backend_driver_run_report_pretty_print_contains_fields() {
        let mut state = State::new();
        let mut driver =
            MockBackendDriver::new(vec![BackendDriverPoll::Event(BackendEvent::DebugRequested)]);

        let report = BackendDriverRunner::run_once(&mut state, &mut driver);
        let text = report.pretty_print();

        // 报告头部必须包含 poll、是否处理事件和有效性。
        assert!(text.contains("Sky Mirror Backend Driver Run Report"));
        assert!(text.contains("poll:"));
        assert!(text.contains("handled_event:"));
        assert!(text.contains("valid:"));

        // 有运行时结果时必须包含完整单事件桥接信息和验证报告正文。
        assert!(text.contains("event:"));
        assert!(text.contains("command:"));
        assert!(text.contains("result:"));
        assert!(text.contains("validation:"));
        assert!(text.contains("Sky Mirror Validation Report"));
    }
}
