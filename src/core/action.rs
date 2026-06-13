//! compositor 的语义动作层。
//!
//! `Action` 表达“用户希望 compositor 做什么”，而不是具体键盘按键或设备事件。
//! 输入层先产生 `InputEvent`，EventLoop 再将其转换为 `Action`，最后统一交给
//! `State::dispatch_action()` 修改集中状态。

/// 所有可由输入触发的 compositor 语义动作。
///
/// 该枚举是输入意图与状态修改之间的稳定边界。新增真实键盘、触摸或 IPC 输入时，
/// 都应先转换为这里的动作，而不是绕过 `State` 直接修改 workspace 或 focus。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 循环切换到下一个工作区。
    NextWorkspace,

    /// 循环切换到上一个工作区。
    PrevWorkspace,

    /// 切换到指定稳定 ID 的工作区。
    ///
    /// 参数是目标 workspace 的稳定 ID，而不是 Vec 下标。
    SwitchWorkspace(u32),

    /// 创建一个新的逻辑窗口并分配到当前工作区。
    SpawnWindow,

    /// 关闭当前焦点窗口。
    ///
    /// 如果当前没有焦点窗口，该动作不会修改 workspace。
    CloseFocusedWindow,

    /// 将焦点移动到下一个包含可见窗口的 slot。
    FocusNextSlot,

    /// 将焦点移动到上一个包含可见窗口的 slot。
    FocusPrevSlot,

    /// 尝试直接聚焦指定 slot。
    ///
    /// 参数是固定 slot ID，当前合法范围为 0..=3。
    FocusSlot(u8),

    /// 在当前 slot 的 stack 中循环切换 active window。
    NextInStack,

    /// 按 Fullscreen、Split、Grid 的顺序循环布局模式。
    CycleLayout,

    /// 将当前工作区布局直接设为 Fullscreen。
    SetLayoutFullscreen,

    /// 将当前工作区布局直接设为 Split。
    SetLayoutSplit,

    /// 将当前工作区布局直接设为 Grid。
    SetLayoutGrid,

    /// 更新当前虚拟输出尺寸。
    ///
    /// 该动作模拟未来由真实 output hotplug 或 mode change 产生的尺寸变化。
    ResizeOutput {
        /// 新的逻辑输出宽度。
        width: u32,

        /// 新的逻辑输出高度。
        height: u32,
    },
}
