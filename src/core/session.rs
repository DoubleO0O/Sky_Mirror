//! compositor 的 MVP session 序列化与纯数据转换层。
//!
//! session 只保存 workspace、slot、stack、focus 和 WindowRegistry 的纯数据。
//! backend、DRM 资源、renderer、OutputState 与真实 Wayland surface 都不会被序列化。
//! 加载时会把可变长度 JSON slot 列表规范化回 Workspace 固定的 `[Slot; 4]`。

use serde::{Deserialize, Serialize};

use crate::core::{
    focus::FocusState,
    workspace::{LayoutMode, SlotContent, Stack, WindowId, Workspace},
};

/// 可写入 JSON 的完整 session 根对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// 保存时处于活动状态的 workspace ID。
    pub current_workspace: u32,
    /// 保存时的显式焦点快照。
    pub focus: SessionFocus,
    /// 所有 workspace 的纯数据镜像。
    pub workspaces: Vec<SessionWorkspace>,
    /// WindowRegistry 下一次应分配的 ID。
    pub next_window_id: WindowId,
}

/// FocusState 的可序列化镜像。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SessionFocus {
    /// 焦点 workspace ID。
    pub workspace: u32,
    /// 焦点 slot ID。
    pub slot: u8,
    /// 焦点窗口；空 workspace 时为 None。
    pub window: Option<WindowId>,
}

/// Workspace 的可序列化镜像。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWorkspace {
    /// workspace 稳定 ID。
    pub id: u32,
    /// 可序列化布局模式。
    pub layout: SessionLayoutMode,
    /// JSON 中使用 Vec，加载时会重新规范化为固定四个 slot。
    pub slots: Vec<SessionSlot>,
}

/// LayoutMode 的 session 表示。
///
/// 独立类型避免给运行时 workspace 数据直接绑定 serde 细节。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SessionLayoutMode {
    /// 全屏布局。
    Fullscreen,
    /// 左右分屏布局。
    Split,
    /// 四宫格布局。
    Grid,
}

/// 单个 slot 的可序列化镜像。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSlot {
    /// slot 稳定 ID。
    pub id: u8,
    /// slot 中保存的窗口内容。
    pub content: SessionSlotContent,
}

/// SlotContent 的可序列化镜像。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionSlotContent {
    /// 空 slot。
    Empty,
    /// 单窗口 slot。
    Single(WindowId),
    /// 多窗口 stack 及其 active 索引。
    Stack {
        /// stack 中的全部窗口 ID。
        windows: Vec<WindowId>,
        /// 保存时的 active window 索引。
        active: usize,
    },
}

/// 将运行时 LayoutMode 转换为 session 表示。
pub fn layout_to_session(layout: LayoutMode) -> SessionLayoutMode {
    match layout {
        // 三个分支保持一一对应，不引入默认或降级行为。
        LayoutMode::Fullscreen => SessionLayoutMode::Fullscreen,
        LayoutMode::Split => SessionLayoutMode::Split,
        LayoutMode::Grid => SessionLayoutMode::Grid,
    }
}

/// 将 session 布局模式恢复为运行时 LayoutMode。
pub fn layout_from_session(layout: SessionLayoutMode) -> LayoutMode {
    match layout {
        // 反向转换同样保持枚举语义一一对应。
        SessionLayoutMode::Fullscreen => LayoutMode::Fullscreen,
        SessionLayoutMode::Split => LayoutMode::Split,
        SessionLayoutMode::Grid => LayoutMode::Grid,
    }
}

/// 把运行时焦点快照转换为可序列化焦点。
impl From<FocusState> for SessionFocus {
    fn from(focus: FocusState) -> Self {
        Self {
            workspace: focus.workspace,
            slot: focus.slot,
            window: focus.window,
        }
    }
}

/// 把 session 焦点恢复为运行时 FocusState。
///
/// 最终一致性仍由 `CompositorState::refresh_focus()` 校验和修正。
impl From<SessionFocus> for FocusState {
    fn from(focus: SessionFocus) -> Self {
        Self {
            workspace: focus.workspace,
            slot: focus.slot,
            window: focus.window,
        }
    }
}

/// 将一个运行时 Workspace 转换为纯数据 SessionWorkspace。
pub fn workspace_to_session(workspace: &Workspace) -> SessionWorkspace {
    // 固定数组按 slot 顺序转换为 JSON 友好的 Vec。
    let slots = workspace
        .slots
        .iter()
        .map(|slot| {
            // 每种运行时内容都保留足以恢复的纯数据。
            let content = match &slot.content {
                SlotContent::Empty => SessionSlotContent::Empty,
                SlotContent::Single(window) => SessionSlotContent::Single(*window),
                // Stack 同时保存窗口顺序和 active 索引。
                SlotContent::Stack(stack) => SessionSlotContent::Stack {
                    windows: stack.windows.clone(),
                    active: stack.active,
                },
            };

            SessionSlot {
                id: slot.id,
                content,
            }
        })
        .collect();

    SessionWorkspace {
        id: workspace.id,
        layout: layout_to_session(workspace.layout),
        slots,
    }
}

/// 将 SessionWorkspace 恢复为固定四 slot 的运行时 Workspace。
///
/// 缺失的 slot 保持 `Workspace::new()` 创建的 Empty 状态；
/// ID 大于 3 的额外 slot 被忽略，防止 JSON 数据破坏固定数组不变量。
pub fn workspace_from_session(session: &SessionWorkspace) -> Workspace {
    // 先建立完整合法的默认 Workspace，确保无论 session slots 数量如何都有四个 slot。
    let mut workspace = Workspace::new(session.id);
    workspace.layout = layout_from_session(session.layout);

    // 只接受固定模型支持的 slot ID 0..=3。
    for session_slot in session.slots.iter().filter(|slot| slot.id < 4) {
        // 将纯数据内容重新构造为运行时枚举。
        let content = match &session_slot.content {
            SessionSlotContent::Empty => SlotContent::Empty,
            SessionSlotContent::Single(window) => SlotContent::Single(*window),
            // active 索引原样恢复；读取 active window 时会进行安全取模。
            SessionSlotContent::Stack { windows, active } => SlotContent::Stack(Stack {
                windows: windows.clone(),
                active: *active,
            }),
        };

        // `id < 4` 已保证数组索引安全。
        workspace.slots[session_slot.id as usize].content = content;
    }

    workspace
}

#[cfg(test)]
mod tests {
    use super::{
        SessionLayoutMode, SessionSlot, SessionSlotContent, SessionWorkspace,
        workspace_from_session, workspace_to_session,
    };
    use crate::core::workspace::{SlotContent, Stack, Workspace};

    /// 验证 workspace 经过 session 纯数据往返转换后完整保留 stack 状态。
    #[test]
    fn workspace_session_round_trip_preserves_stack() {
        let mut workspace = Workspace::new(0);
        workspace.slots[0].content = SlotContent::Stack(Stack {
            windows: vec![10, 11],
            active: 1,
        });

        let session = workspace_to_session(&workspace);
        let restored = workspace_from_session(&session);

        let SlotContent::Stack(stack) = &restored.slots[0].content else {
            panic!("session 恢复后 slot 0 必须仍然是 Stack");
        };

        // stack 中的窗口顺序必须在序列化边界两侧保持一致。
        assert_eq!(stack.windows, vec![10, 11]);

        // active 索引必须原样恢复，不能重置为第一个窗口。
        assert_eq!(stack.active, 1);

        // 统一 slot 读取接口必须返回恢复后的 active window 11。
        assert_eq!(restored.slot_window(0), Some(11));
    }

    /// 验证 session 缺少 slot 时，恢复逻辑会补齐固定的四个 Empty slot。
    #[test]
    fn workspace_from_session_pads_missing_slots_with_empty() {
        let session = SessionWorkspace {
            id: 3,
            layout: SessionLayoutMode::Fullscreen,
            slots: vec![SessionSlot {
                id: 0,
                content: SessionSlotContent::Single(42),
            }],
        };

        let restored = workspace_from_session(&session);

        // 运行时 Workspace 必须始终维持固定四 slot 不变量。
        assert_eq!(restored.slots.len(), 4);

        // session 中存在的 slot 0 必须按原内容恢复。
        assert_eq!(restored.slot_window(0), Some(42));

        // session 未提供的 slot 1、2、3 必须由默认 Workspace 补齐为空。
        assert!(
            restored.slots[1..]
                .iter()
                .all(|slot| matches!(slot.content, SlotContent::Empty))
        );
    }
}
