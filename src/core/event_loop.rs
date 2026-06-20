//! compositor 的 calloop 主事件循环。
//!
//! EventLoop 只负责调度：轮询输入、把 `InputEvent` 翻译为 `Action`、将 Action
//! 交给全局 `State`，并把状态生成的 `RenderFrame` 交给 renderer。
//! 它不直接修改 workspace、focus、stack、layout 或 output 状态。

use std::{io, time::Duration};

use calloop::EventLoop as CalloopEventLoop;

use crate::core::{
    action::Action,
    input::{InputEvent, InputSimulator},
    render::MockRenderer,
    state::State,
};

/// compositor 的系统主循环封装。
///
/// 业务状态由外部传入的 `State` 唯一持有；本结构只持有调度基础设施、
/// 临时输入源和当前 mock renderer。
pub struct EventLoop {
    /// calloop 事件循环实例。
    ///
    /// 泛型参数 `State` 表示事件源回调未来可以访问同一份集中状态。
    event_loop: CalloopEventLoop<'static, State>,
    /// 当前 MVP 使用的 tick-based 临时输入源。
    ///
    /// Boundary: 它只产生核心 `InputEvent`；系统输入接入时应替换事件来源，
    /// 而不是绕过既有 Action/State 分发链路。
    input: InputSimulator,
    /// 消费 RenderFrame 的占位 renderer。
    ///
    /// 当前只输出日志，不持有 GPU、surface 或 DRM 资源。
    renderer: MockRenderer,
}

impl EventLoop {
    /// 创建 calloop 主循环及其临时输入、渲染边界。
    ///
    /// 创建阶段不启动 backend，也不修改 compositor 状态；真正启动发生在 `run()`。
    pub fn new() -> io::Result<Self> {
        println!("[Core] EventLoop created");

        // calloop 创建失败时直接向上返回 I/O 错误，由程序入口决定终止策略。
        let event_loop = CalloopEventLoop::try_new()?;

        Ok(Self {
            event_loop,
            input: InputSimulator::new(),
            renderer: MockRenderer::new(),
        })
    }

    /// 启动 backend 并持续调度 compositor 事件。
    ///
    /// `state` 是系统唯一的全局可变状态。EventLoop 不保存其副本，
    /// 从而避免 workspace、focus 和 output 在多个所有者之间失去一致性。
    pub fn run(&mut self, state: &mut State) -> io::Result<()> {
        println!("[Core] EventLoop started");

        // backend 生命周期由 CompositorState 管理，EventLoop 只触发统一启动入口。
        state.compositor.start();

        // `running` 是集中状态中的退出控制位。
        // 当前 MVP 尚无真实退出事件，因此循环通常会持续运行。
        while state.compositor.running {
            // 以约 16ms 的等待上限调度 calloop。
            // 即使没有外部事件，也会定期返回以轮询临时 InputSimulator。
            self.event_loop.dispatch(Duration::from_millis(16), state)?;

            // 一次 poll 最多得到一个 InputEvent，保持状态变化顺序明确。
            if let Some(event) = self.input.poll() {
                // 输入层事件在这里被翻译为语义 Action。
                // 该 match 只做类型转换，不直接修改任何 compositor 状态。
                let action = match event {
                    // “下一个 workspace”输入保持相同语义交给 Action 层。
                    InputEvent::NextWorkspace => Action::NextWorkspace,
                    // “上一个 workspace”输入保持相同语义交给 Action 层。
                    InputEvent::PrevWorkspace => Action::PrevWorkspace,
                    // 指定 workspace 的稳定 ID 必须原样传递，边界检查由状态层完成。
                    InputEvent::SwitchWorkspace(id) => Action::SwitchWorkspace(id),
                    // 窗口创建只表达意图，WindowId 仍由 State 中的 registry 分配。
                    InputEvent::SpawnWindow => Action::SpawnWindow,
                    // 关闭窗口也只表达输入意图，真正移除 slot/stack 内容由 State 层完成。
                    InputEvent::CloseFocusedWindow => Action::CloseFocusedWindow,
                    // slot 导航输入不在事件循环中读取或修改 FocusState。
                    InputEvent::FocusNextSlot => Action::FocusNextSlot,
                    // 向前导航同样委托给 CompositorState 处理 wrap around。
                    InputEvent::FocusPrevSlot => Action::FocusPrevSlot,
                    // 指定 slot ID 原样传递，由状态层验证范围和占用状态。
                    InputEvent::FocusSlot(slot) => Action::FocusSlot(slot),
                    // stack 切换只产生动作，不在 EventLoop 中访问 SlotContent。
                    InputEvent::NextInStack => Action::NextInStack,
                    // 布局循环由 CompositorState 根据当前 LayoutMode 决定下一状态。
                    InputEvent::CycleLayout => Action::CycleLayout,
                    // 直接布局动作保持明确语义，状态层负责写入当前 workspace。
                    InputEvent::SetLayoutFullscreen => Action::SetLayoutFullscreen,
                    // Split 设置不在输入层接触 Workspace。
                    InputEvent::SetLayoutSplit => Action::SetLayoutSplit,
                    // Grid 设置不在输入层接触 Workspace。
                    InputEvent::SetLayoutGrid => Action::SetLayoutGrid,
                    // 输出尺寸变化仍转换为 Action，避免输入层直接写 OutputState。
                    InputEvent::ResizeOutput { width, height } => {
                        Action::ResizeOutput { width, height }
                    }
                };

                // 统一进入 Action 调度和渲染帧生成流程。
                self.handle_action(state, action);
            }
        }

        // 主循环正常结束时持久化纯数据 session。
        // backend、renderer 和真实 surface 均不会写入 session。
        state.save_session("sky_mirror_session.json")?;
        Ok(())
    }

    /// 分发一个 Action，并将最新状态规划成 RenderFrame。
    ///
    /// EventLoop 不直接接触 workspace/focus/output 的内部字段：
    /// 状态修改由 `dispatch_action` 完成，渲染输入由 CompositorState 只读生成。
    fn handle_action(&mut self, state: &mut State, action: Action) {
        // 所有业务状态变化必须先经过全局 State 的语义入口。
        state.dispatch_action(action);

        // EventLoop 通过 State 获取 RenderFrame，使窗口 metadata 可以在 State 层合并。
        // 该调用仍使用 CompositorState 当前输出尺寸，EventLoop 不访问 registry 或 workspace。
        let frame = state.current_render_frame_for_current_output();

        // MockRenderer 只消费规划结果并打印日志，不反向修改 compositor 状态。
        self.renderer.render(&frame);
    }
}
