//! Smithay 动作请求事件适配探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不接真实键盘、指针、触摸、libinput，也不接 Smithay seat。
//!
//! 它只负责把未来输入层已经解析出的核心 `Action` 转换为
//! `BackendEvent::ActionRequested`。真正执行动作的逻辑，仍然发生在事件经过
//! `BackendDriverRunner` 进入核心状态之后。

use crate::core::{action::Action, backend_event::BackendEvent};

/// Smithay 动作事件适配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该模块只生成纯数据 `BackendEvent`，
/// 不处理真实键盘、指针或触摸输入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayActionEventMode {
    /// 纯探针模式。
    ///
    /// 不接 libinput，不接 Smithay seat，也不直接调用核心状态的动作分发入口。
    ProbeOnly,
}

/// 动作请求描述信息。
///
/// 该结构只保存核心 `Action`。未来真实输入层应先把键盘快捷键、鼠标按钮或手势
/// 转换成 `Action`，再通过该描述生成 `BackendEvent`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayActionRequestDescriptor {
    /// 要请求核心执行的动作。
    ///
    /// 创建描述不会执行动作；只有 `run_once()` 处理事件后核心才会执行它。
    pub action: Action,
}

impl SmithayActionRequestDescriptor {
    /// 创建一个动作请求描述。
    ///
    /// 本方法只保存纯数据 `Action`，不会执行该动作。
    pub fn new(action: Action) -> Self {
        Self { action }
    }

    /// 创建一个切换到下一个 workspace 的请求。
    pub fn next_workspace() -> Self {
        Self::new(Action::NextWorkspace)
    }

    /// 创建一个切换到上一个 workspace 的请求。
    pub fn prev_workspace() -> Self {
        Self::new(Action::PrevWorkspace)
    }

    /// 创建一个请求生成测试窗口的动作。
    pub fn spawn_window() -> Self {
        Self::new(Action::SpawnWindow)
    }

    /// 创建一个请求循环布局的动作。
    pub fn cycle_layout() -> Self {
        Self::new(Action::CycleLayout)
    }

    /// 创建一个关闭当前焦点窗口的动作。
    pub fn close_focused_window() -> Self {
        Self::new(Action::CloseFocusedWindow)
    }
}

/// Smithay 动作事件适配探针。
///
/// 该类型不持有状态，也不接真实输入设备。
/// 它只把动作请求描述转换成 `BackendEvent::ActionRequested`。
pub struct SmithayActionEventProbe;

impl SmithayActionEventProbe {
    /// 返回当前适配器模式。
    pub fn mode() -> SmithayActionEventMode {
        SmithayActionEventMode::ProbeOnly
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only() -> bool {
        true
    }

    /// 把动作请求描述转换成 `BackendEvent`。
    ///
    /// 未来真实 Smithay 键盘、指针或触摸回调应先解析成核心 `Action`，
    /// 再通过该路径生成 `BackendEvent`，而不是直接修改核心 `State` 或调用其
    /// 动作分发入口。真正执行动作要等事件经过 `run_once()` 后由核心完成。
    pub fn action_requested_event(descriptor: SmithayActionRequestDescriptor) -> BackendEvent {
        BackendEvent::ActionRequested(descriptor.action)
    }

    /// 返回当前阶段说明。
    pub fn mode_description() -> &'static str {
        "smithay-action-event-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayActionEventMode, SmithayActionEventProbe, SmithayActionRequestDescriptor};
    use crate::core::{action::Action, backend_event::BackendEvent};

    /// 验证动作描述器会原样保存核心动作。
    #[test]
    fn action_request_descriptor_builds_action() {
        let descriptor = SmithayActionRequestDescriptor::new(Action::NextWorkspace);

        assert_eq!(descriptor.action, Action::NextWorkspace);
    }

    /// 验证常用动作描述器辅助构造方法会生成对应核心动作。
    #[test]
    fn action_request_descriptor_convenience_builders_work() {
        assert_eq!(
            SmithayActionRequestDescriptor::next_workspace().action,
            Action::NextWorkspace
        );
        assert_eq!(
            SmithayActionRequestDescriptor::prev_workspace().action,
            Action::PrevWorkspace
        );
        assert_eq!(
            SmithayActionRequestDescriptor::spawn_window().action,
            Action::SpawnWindow
        );
        assert_eq!(
            SmithayActionRequestDescriptor::cycle_layout().action,
            Action::CycleLayout
        );
        assert_eq!(
            SmithayActionRequestDescriptor::close_focused_window().action,
            Action::CloseFocusedWindow
        );
    }

    /// 验证动作事件探针会生成完整的纯数据 ActionRequested 事件。
    #[test]
    fn action_event_probe_creates_action_requested_event() {
        let event = SmithayActionEventProbe::action_requested_event(
            SmithayActionRequestDescriptor::next_workspace(),
        );

        assert_eq!(event, BackendEvent::ActionRequested(Action::NextWorkspace));
    }

    /// 验证动作事件适配器固定保持纯探针模式。
    #[test]
    fn action_event_probe_reports_probe_mode() {
        assert!(SmithayActionEventProbe::is_probe_only());
        assert_eq!(
            SmithayActionEventProbe::mode(),
            SmithayActionEventMode::ProbeOnly
        );
        assert_eq!(
            SmithayActionEventProbe::mode_description(),
            "smithay-action-event-probe-only"
        );
    }
}
