//! compositor 的显式焦点状态。
//!
//! 焦点同时记录 workspace、slot 与当前 active window，便于导航、scene 构建和
//! session restore 使用同一份状态。焦点一致性由 `CompositorState::refresh_focus`
//! 统一维护，本模块只提供最小的数据结构和局部更新操作。

use crate::core::workspace::WindowId;

/// 当前 compositor 焦点位置。
///
/// 三层标识显式保存，避免仅依赖容器下标反推焦点，并允许空工作区或空 slot
/// 合法地以 `window: None` 表示“当前没有可聚焦窗口”。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusState {
    /// 当前焦点所属的 workspace ID。
    pub workspace: u32,
    /// 当前焦点指向的固定 slot ID。
    pub slot: u8,
    /// 当前 slot 中实际获得焦点的 active window。
    ///
    /// 空 slot、空 workspace 或无效恢复状态会暂时使用 `None`。
    pub window: Option<WindowId>,
}

impl FocusState {
    /// 创建指向 workspace 0、slot 0 且尚无窗口的初始焦点。
    pub fn new() -> Self {
        Self {
            workspace: 0,
            slot: 0,
            window: None,
        }
    }

    /// 切换焦点所属工作区，并重置 slot 与 window。
    ///
    /// workspace 切换后不能沿用旧工作区的 slot/window 组合，因此先回到 slot 0，
    /// 再由 `CompositorState::refresh_focus()` 查找该工作区真正可见的窗口。
    pub fn set_workspace(&mut self, workspace: u32) {
        // workspace ID 是焦点层级的根，必须先替换为新的稳定 ID。
        self.workspace = workspace;

        // 新 workspace 不继承旧 workspace 的 slot 位置，从确定的 slot 0 开始解析。
        self.slot = 0;

        // 旧 window 必然属于旧 workspace，必须清空后再由 refresh_focus 推导。
        self.window = None;
    }

    /// 更新当前 slot，并清除旧窗口焦点。
    ///
    /// slot 变化后，旧 `window` 很可能不属于新 slot，因此必须先清空，
    /// 再由统一刷新流程根据 Single 或 Stack 的 active window 重新填充。
    pub fn set_slot(&mut self, slot: u8) {
        // 先记录目标 slot，后续 refresh_focus 会从该位置解析 active window。
        self.slot = slot;

        // 不允许旧 slot 的窗口 ID 暂时冒充新 slot 的焦点窗口。
        self.window = None;
    }

    /// 设置当前 slot 最终解析出的焦点窗口。
    pub fn set_window(&mut self, window: Option<WindowId>) {
        // 这里只写入已由 CompositorState 校验过的最终结果，不自行遍历 workspace。
        self.window = window;
    }
}
