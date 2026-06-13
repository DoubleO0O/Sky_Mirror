//! 纯数据键位映射层。
//!
//! 本模块把抽象 `KeyChord` 解析为 `InputEvent`，不依赖真实 keycode、xkbcommon、
//! Smithay keyboard，也不产生 Action 或修改 State。未来真实键盘事件只需先转换为
//! 这里的 Key/Modifiers，即可复用相同绑定表。

use std::collections::HashMap;

use crate::core::input::InputEvent;

/// 当前 compositor 关心的抽象按键集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// 字母 H。
    H,
    /// 字母 J。
    J,
    /// 字母 K。
    K,
    /// 字母 L。
    L,
    /// 字母 Q。
    Q,
    /// Tab 键。
    Tab,
    /// 空格键。
    Space,
    /// Enter 键。
    Enter,
    /// 数字键；默认绑定只使用 1..=4。
    Num(u8),
}

/// 与按键同时按下的修饰键集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Super/Logo 键是否按下。
    pub super_key: bool,
    /// Ctrl 键是否按下。
    pub ctrl: bool,
    /// Alt 键是否按下。
    pub alt: bool,
    /// Shift 键是否按下。
    pub shift: bool,
}

/// 一个完整的“修饰键 + 主键”组合。
///
/// 该类型实现 Hash，因此可直接作为 HashMap 的稳定键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    /// 组合键的修饰键状态。
    pub modifiers: Modifiers,
    /// 组合键的主键。
    pub key: Key,
}

impl KeyChord {
    /// 使用明确修饰键和主键创建组合键。
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    /// 创建仅按下 Super 修饰键的组合键。
    pub fn super_key(key: Key) -> Self {
        Self {
            modifiers: Modifiers {
                super_key: true,
                ctrl: false,
                alt: false,
                shift: false,
            },
            key,
        }
    }
}

/// 从 KeyChord 到 InputEvent 的可修改映射表。
///
/// 映射结果停留在输入层，只负责 `KeyChord -> InputEvent`。
/// 它不会直接生成 Action，也不会读取或修改 State；后续语义转换仍由 EventLoop 完成。
pub struct KeybindingMap {
    /// 所有已注册组合键及其对应输入事件。
    bindings: HashMap<KeyChord, InputEvent>,
}

impl KeybindingMap {
    /// 创建 compositor 当前使用的默认快捷键集合。
    ///
    /// 默认绑定覆盖 slot/workspace 导航、stack 切换、布局循环、窗口创建、
    /// 当前窗口关闭和 workspace 1..4 的直接选择。
    pub fn default_bindings() -> Self {
        let mut map = Self {
            bindings: HashMap::new(),
        };

        // Super+H/L：在 occupied slot 之间向前/向后移动。
        map.insert(KeyChord::super_key(Key::H), InputEvent::FocusPrevSlot);
        map.insert(KeyChord::super_key(Key::L), InputEvent::FocusNextSlot);

        // Super+J/K：在 workspace 列表中循环导航。
        map.insert(KeyChord::super_key(Key::J), InputEvent::NextWorkspace);
        map.insert(KeyChord::super_key(Key::K), InputEvent::PrevWorkspace);

        // Super+Tab：切换当前 stack 的 active window。
        map.insert(KeyChord::super_key(Key::Tab), InputEvent::NextInStack);

        // Super+Space：循环布局模式。
        map.insert(KeyChord::super_key(Key::Space), InputEvent::CycleLayout);

        // Super+Enter：创建测试逻辑窗口。
        map.insert(KeyChord::super_key(Key::Enter), InputEvent::SpawnWindow);

        // Super+Q：关闭当前焦点窗口。
        map.insert(KeyChord::super_key(Key::Q), InputEvent::CloseFocusedWindow);

        // 数字键使用面向用户的 1..4，映射到内部 workspace ID 0..3。
        map.insert(
            KeyChord::super_key(Key::Num(1)),
            InputEvent::SwitchWorkspace(0),
        );
        map.insert(
            KeyChord::super_key(Key::Num(2)),
            InputEvent::SwitchWorkspace(1),
        );
        map.insert(
            KeyChord::super_key(Key::Num(3)),
            InputEvent::SwitchWorkspace(2),
        );
        map.insert(
            KeyChord::super_key(Key::Num(4)),
            InputEvent::SwitchWorkspace(3),
        );

        map
    }

    /// 解析一个组合键。
    ///
    /// 已绑定时复制返回 InputEvent，未知组合键返回 None，不会 panic。
    pub fn resolve(&self, chord: KeyChord) -> Option<InputEvent> {
        self.bindings.get(&chord).copied()
    }

    /// 插入或覆盖一个快捷键绑定。
    ///
    /// 该入口为未来用户配置保留，但仍只允许绑定到 InputEvent。
    pub fn insert(&mut self, chord: KeyChord, event: InputEvent) {
        self.bindings.insert(chord, event);
    }
}

// 键位映射测试只操作纯数据，不启动 EventLoop、backend 或 session I/O。
#[cfg(test)]
mod tests {
    use super::{Key, KeyChord, KeybindingMap, Modifiers};
    use crate::core::input::InputEvent;

    /// 验证默认 H/L 绑定分别产生前后 slot 导航事件。
    #[test]
    fn resolves_default_slot_bindings() {
        let bindings = KeybindingMap::default_bindings();

        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::H)),
            Some(InputEvent::FocusPrevSlot)
        );
        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::L)),
            Some(InputEvent::FocusNextSlot)
        );
    }

    /// 验证面向用户的数字 1 映射到内部 workspace ID 0。
    #[test]
    fn resolves_workspace_number_binding() {
        let bindings = KeybindingMap::default_bindings();

        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::Num(1))),
            Some(InputEvent::SwitchWorkspace(0))
        );
    }

    /// 验证未注册的修饰键组合安全返回 None。
    #[test]
    fn unknown_chord_returns_none() {
        let bindings = KeybindingMap::default_bindings();
        let chord = KeyChord::new(
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
            Key::H,
        );

        assert_eq!(bindings.resolve(chord), None);
    }

    /// 验证 stack、布局、窗口创建和第四个 workspace 的补充默认绑定。
    #[test]
    fn resolves_additional_default_bindings() {
        let bindings = KeybindingMap::default_bindings();

        // Super+Tab 必须映射为切换当前 stack 的 active window。
        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::Tab)),
            Some(InputEvent::NextInStack)
        );

        // Super+Space 必须映射为循环当前 workspace 的布局模式。
        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::Space)),
            Some(InputEvent::CycleLayout)
        );

        // Super+Enter 必须映射为创建逻辑窗口。
        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::Enter)),
            Some(InputEvent::SpawnWindow)
        );

        // Super+Q 必须映射为关闭当前焦点窗口。
        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::Q)),
            Some(InputEvent::CloseFocusedWindow)
        );

        // 面向用户的 Super+4 必须映射到内部 workspace ID 3。
        assert_eq!(
            bindings.resolve(KeyChord::super_key(Key::Num(4))),
            Some(InputEvent::SwitchWorkspace(3))
        );
    }
}
