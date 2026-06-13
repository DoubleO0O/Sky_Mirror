//! compositor 的输入事件与临时模拟输入源。
//!
//! `InputEvent` 是设备输入与语义 `Action` 之间的一层稳定表示。
//! 当前 `InputSimulator` 使用 tick 周期模拟按键和输出 resize；未来接入真实
//! libinput 或 Smithay keyboard 后，应由真实设备事件生成同样的 `InputEvent`。

use crate::core::keybinding::{Key, KeyChord, KeybindingMap};

/// 输入层可以提交给 EventLoop 的事件。
///
/// 这些事件仍不直接修改状态；EventLoop 会把它们映射成 `Action`，
/// 再由 `State::dispatch_action()` 执行集中状态变更。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// 请求切换到下一个工作区。
    NextWorkspace,

    /// 请求切换到上一个工作区。
    PrevWorkspace,

    /// 请求切换到指定 workspace ID。
    ///
    /// 参数是目标 workspace 的稳定 ID，而不是 Vec 下标。
    SwitchWorkspace(u32),

    /// 请求创建新的逻辑窗口。
    SpawnWindow,

    /// 请求关闭当前焦点窗口。
    CloseFocusedWindow,

    /// 请求聚焦下一个已占用 slot。
    FocusNextSlot,

    /// 请求聚焦上一个已占用 slot。
    FocusPrevSlot,

    /// 请求直接聚焦指定 slot。
    ///
    /// 参数是固定 slot ID，当前合法范围为 0..=3。
    FocusSlot(u8),

    /// 请求切换当前 slot stack 的 active window。
    NextInStack,

    /// 请求循环当前 workspace 的布局模式。
    CycleLayout,

    /// 请求直接设置 Fullscreen 布局。
    SetLayoutFullscreen,

    /// 请求直接设置 Split 布局。
    SetLayoutSplit,

    /// 请求直接设置 Grid 布局。
    SetLayoutGrid,

    /// 模拟输出模式变化产生的尺寸更新。
    ///
    /// 该事件不是键盘快捷键，而是临时模拟 output resize。
    ResizeOutput {
        /// 新的逻辑输出宽度。
        width: u32,

        /// 新的逻辑输出高度。
        height: u32,
    },
}

/// 基于 tick 的临时输入源。
///
/// 它保留真实输入源应具备的 `poll -> Option<InputEvent>` 形状，但不连接任何设备。
/// 每次轮询最多返回一个事件，确保当前 EventLoop 可以按明确顺序逐个处理状态变化。
pub struct InputSimulator {
    /// 已经过的轮询次数，用于触发确定性的测试事件。
    tick: u64,
    /// 默认键位映射；键盘语义事件通过该表解析为 `InputEvent`。
    keybindings: KeybindingMap,
}

impl InputSimulator {
    /// 创建从 tick 0 开始、加载默认快捷键的模拟输入源。
    pub fn new() -> Self {
        Self {
            tick: 0,
            keybindings: KeybindingMap::default_bindings(),
        }
    }

    /// 模拟按下仅包含 Super 修饰键的组合键。
    ///
    /// KeybindingMap 只负责返回输入事件，不产生 Action，也不访问全局状态。
    /// 这里模拟的是“按下 Super + 指定按键”，用于验证真实键盘尚未接入时的映射链路。
    ///
    /// TODO: 后续接入真实 Smithay keyboard 后，应使用真实 key event 和 modifier 状态
    /// 构造 KeyChord，并替换当前固定使用 Super 修饰键的模拟入口。
    fn simulate_key(&self, key: Key) -> Option<InputEvent> {
        self.keybindings.resolve(KeyChord::super_key(key))
    }

    /// 推进一个 tick，并在命中测试周期时返回一个输入事件。
    ///
    /// resize 条件放在普通按键模拟之前，使 900/600 的公共倍数优先表达输出变化。
    /// 每个分支立即返回，因此一次 poll 不会产生多个事件。
    pub fn poll(&mut self) -> Option<InputEvent> {
        // 每次 EventLoop 轮询后推进计数器，为临时输入提供确定性时间基准。
        self.tick += 1;

        // 优先恢复到默认虚拟输出尺寸。
        //
        // ResizeOutput 不是键盘快捷键语义，因此不经过 KeybindingMap。
        // TODO: 后续接入真实 backend/output 后，应由真实输出模式变化事件替换该分支。
        if self.tick % 900 == 0 {
            return Some(InputEvent::ResizeOutput {
                width: 1920,
                height: 1080,
            });
        }

        // 模拟一次较小输出模式，验证布局和 RenderFrame 会使用 OutputState。
        //
        // 与上一个 resize 分支相同，这里直接产生 InputEvent，而不是伪装成键盘按键。
        // TODO: 真实 output resize 接入后删除该 tick 模拟，保留相同 InputEvent 边界。
        if self.tick % 600 == 0 {
            return Some(InputEvent::ResizeOutput {
                width: 1366,
                height: 768,
            });
        }

        // 通过 Super+Space 模拟布局循环。
        if self.tick % 300 == 0 {
            return self.simulate_key(Key::Space);
        }

        // 通过 Super+L 模拟向后一个 slot 移动焦点。
        if self.tick % 90 == 0 {
            return self.simulate_key(Key::L);
        }

        // 通过 Super+Q 模拟关闭当前焦点窗口。
        //
        // 240 是 120 的倍数，因此该分支必须位于 workspace 切换之前，
        // 否则每次命中关闭周期时都会先返回 Super+J，导致关闭事件不可达。
        if self.tick % 240 == 0 {
            return self.simulate_key(Key::Q);
        }

        // 通过 Super+J 模拟切换到下一个 workspace。
        if self.tick % 120 == 0 {
            return self.simulate_key(Key::J);
        }

        // 通过 Super+Tab 模拟 stack 内 active window 循环。
        if self.tick % 150 == 0 {
            return self.simulate_key(Key::Tab);
        }

        // 通过 Super+Enter 模拟创建窗口。
        if self.tick % 200 == 0 {
            return self.simulate_key(Key::Enter);
        }

        // 当前 tick 没有命中任何模拟输入，EventLoop 继续等待下一轮。
        //
        // TODO: 后续接入真实 Smithay keyboard 后，应由真实键盘事件替换该 tick 逻辑。
        None
    }
}
