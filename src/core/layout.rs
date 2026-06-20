//! 纯数据布局计算引擎。
//!
//! 本模块把 `Workspace + LayoutMode + OutputSize` 转换为窗口矩形列表。
//! 计算过程只读取 workspace，不修改 slot、stack、focus 或其他全局状态。
//! 所有窗口都通过 `Workspace::slot_window()` 获取，因此 Stack 只会布局 active window。

use crate::core::workspace::{LayoutMode, WindowId, Workspace};

/// 输出坐标系中的矩形区域。
///
/// 位置使用有符号整数，为未来支持负坐标输出排列保留空间；
/// 尺寸使用无符号整数，避免表达无效的负宽高。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// 矩形左上角的水平坐标。
    pub x: i32,
    /// 矩形左上角的垂直坐标。
    pub y: i32,
    /// 矩形宽度。
    pub width: u32,
    /// 矩形高度。
    pub height: u32,
}

/// 当前布局计算使用的输出尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputSize {
    /// 输出逻辑宽度。
    pub width: u32,
    /// 输出逻辑高度。
    pub height: u32,
}

/// 一个可见窗口在某个 workspace 中的布局结果。
///
/// 该结构只描述几何位置，不包含焦点、z-index 或真实 surface。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPlacement {
    /// 应显示的逻辑窗口 ID。
    pub window: WindowId,
    /// placement 所属的 workspace ID。
    pub workspace: u32,
    /// placement 来源的固定 slot ID。
    pub slot: u8,
    /// 窗口在输出坐标系中的目标矩形。
    pub rect: Rect,
}

/// 无内部状态的布局计算入口。
pub struct LayoutEngine;

impl LayoutEngine {
    /// 根据 workspace 当前布局模式计算所有可见窗口的位置。
    ///
    /// Empty slot 不产生 placement；Single 返回其窗口；Stack 通过
    /// `slot_window()` 只返回 active window。该兼容入口不接收焦点信息，
    /// 会按固定 slot 顺序选择 Fullscreen 窗口；现有调用方无需修改。
    pub fn compute_workspace(workspace: &Workspace, output: OutputSize) -> Vec<WindowPlacement> {
        // 旧 interface 保持可用，并把新增焦点语义集中交给同一个实现维护。
        // None 表示调用方没有提供 focused slot，Fullscreen 会回退到第一个 occupied slot。
        Self::compute_workspace_with_focus(workspace, None, output)
    }

    /// 根据 workspace、可选 focused slot 和输出尺寸计算可见窗口的位置。
    ///
    /// Fullscreen 优先显示 `focus_slot` 中的 active window；focused slot 为空、
    /// 无效或未提供时，按固定 slot 顺序回退到第一个 occupied slot。Split 与 Grid
    /// 保持原有几何规则，不因 focused slot 改变 placement 数量或顺序。
    pub fn compute_workspace_with_focus(
        workspace: &Workspace,
        focus_slot: Option<u8>,
        output: OutputSize,
    ) -> Vec<WindowPlacement> {
        match workspace.layout {
            // Fullscreen 只显示 resolved slot 的 active window，并占满整个输出。
            // slot 选择统一封装在 helper 中，避免焦点与 fallback 规则散落到调用方。
            LayoutMode::Fullscreen => {
                let Some((slot, window)) = Self::fullscreen_window(workspace, focus_slot) else {
                    // 空 workspace 没有 occupied slot，返回空 placement 且不制造占位窗口。
                    return Vec::new();
                };

                vec![WindowPlacement {
                    window,
                    workspace: workspace.id,
                    slot,
                    rect: Rect {
                        x: 0,
                        y: 0,
                        width: output.width,
                        height: output.height,
                    },
                }]
            }
            // Split 最多显示 slot 0 和 slot 1，分别占据左右半屏。
            LayoutMode::Split => {
                // 整数除法得到左半宽度；奇数宽度的余数由右半区域吸收。
                let half_w = output.width / 2;
                let rects = [
                    // slot 0：左侧区域。
                    Rect {
                        x: 0,
                        y: 0,
                        width: half_w,
                        height: output.height,
                    },
                    // slot 1：右侧区域，使用减法保留全部剩余像素。
                    Rect {
                        x: half_w as i32,
                        y: 0,
                        width: output.width - half_w,
                        height: output.height,
                    },
                ];

                // 只为实际包含 active window 的 slot 创建 placement。
                (0..2)
                    .filter_map(|slot| {
                        workspace.slot_window(slot).map(|window| WindowPlacement {
                            window,
                            workspace: workspace.id,
                            slot,
                            rect: rects[slot as usize],
                        })
                    })
                    .collect()
            }
            // Grid 最多显示四个 slot，按左上、右上、左下、右下排列。
            LayoutMode::Grid => {
                // 奇数宽高的余数分别由右侧列与底部行吸收。
                let half_w = output.width / 2;
                let half_h = output.height / 2;
                let rects = [
                    // slot 0：左上。
                    Rect {
                        x: 0,
                        y: 0,
                        width: half_w,
                        height: half_h,
                    },
                    // slot 1：右上。
                    Rect {
                        x: half_w as i32,
                        y: 0,
                        width: output.width - half_w,
                        height: half_h,
                    },
                    // slot 2：左下。
                    Rect {
                        x: 0,
                        y: half_h as i32,
                        width: half_w,
                        height: output.height - half_h,
                    },
                    // slot 3：右下。
                    Rect {
                        x: half_w as i32,
                        y: half_h as i32,
                        width: output.width - half_w,
                        height: output.height - half_h,
                    },
                ];

                // 固定遍历 slot 0..3；空 slot 被 filter_map 安全忽略。
                (0..4)
                    .filter_map(|slot| {
                        workspace.slot_window(slot).map(|window| WindowPlacement {
                            window,
                            workspace: workspace.id,
                            slot,
                            rect: rects[slot as usize],
                        })
                    })
                    .collect()
            }
        }
    }

    /// 解析 Fullscreen 应显示的 slot 与 active window。
    ///
    /// 先尝试调用方提供的 focused slot；如果它为空或无效，再按 workspace 固定
    /// slot 顺序寻找第一个 occupied slot。所有窗口都经 `slot_window()` 读取，
    /// 因此 Stack 只会返回 active window，隐藏的 stack 成员不会泄漏到布局结果。
    fn fullscreen_window(workspace: &Workspace, focus_slot: Option<u8>) -> Option<(u8, WindowId)> {
        if let Some(slot) = focus_slot {
            if let Some(window) = workspace.slot_window(slot) {
                // focused slot 可见时立即返回，保证 keyboard focus 与 Fullscreen 画面一致。
                return Some((slot, window));
            }
        }

        // focused slot 为空、无效或未提供时，使用固定 slot 顺序提供确定性 fallback。
        // `slot_window()` 同时处理 Empty、Single 和 Stack active window 三种内容。
        workspace.slots.iter().find_map(|slot| {
            workspace
                .slot_window(slot.id)
                .map(|window| (slot.id, window))
        })
    }
}

// 布局测试只验证纯几何计算，不启动 backend、EventLoop 或 renderer。
#[cfg(test)]
mod tests {
    use super::{LayoutEngine, OutputSize, Rect, WindowPlacement};
    use crate::core::workspace::{LayoutMode, Workspace};

    /// 返回布局测试统一使用的输出尺寸。
    ///
    /// 集中测试尺寸可以让各个 focused slot 用例只表达焦点语义，避免重复的几何噪音。
    fn test_output() -> OutputSize {
        OutputSize {
            width: 1920,
            height: 1080,
        }
    }

    /// 创建四个固定 slot 都已占用的 Fullscreen workspace。
    ///
    /// 窗口 ID 与 slot ID 保持稳定对应，便于断言 focused slot 最终选择的窗口。
    fn fullscreen_workspace_with_four_windows() -> Workspace {
        let mut workspace = Workspace::new(0);
        for window in 1..=4 {
            workspace.assign_window(window);
        }
        workspace.layout = LayoutMode::Fullscreen;
        workspace
    }

    /// 断言 Fullscreen 只生成一个覆盖完整输出的 placement。
    ///
    /// 该 helper 同时检查 slot、窗口和几何，确保 focus-aware 选择不会破坏全屏尺寸语义。
    fn assert_fullscreen_placement(
        placements: &[WindowPlacement],
        expected_slot: u8,
        expected_window: u64,
    ) {
        // Fullscreen 无论选中哪个 slot，都只能暴露一个可见窗口。
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].slot, expected_slot);
        assert_eq!(placements[0].window, expected_window);
        assert_eq!(
            placements[0].rect,
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }
        );
    }

    /// 验证 Fullscreen 只布局 slot 0，并让其覆盖完整输出。
    #[test]
    fn fullscreen_only_places_slot_zero() {
        let mut workspace = Workspace::new(0);
        workspace.assign_window(1);
        workspace.assign_window(2);
        workspace.layout = LayoutMode::Fullscreen;
        let output = OutputSize {
            width: 1920,
            height: 1080,
        };

        let placements = LayoutEngine::compute_workspace(&workspace, output);

        // Fullscreen 模式即使 slot 1 有窗口，也只能生成一个 placement。
        assert_eq!(placements.len(), 1);

        // 唯一 placement 必须来自 slot 0 的窗口。
        assert_eq!(placements[0].window, 1);
        assert_eq!(placements[0].slot, 0);

        // Fullscreen 矩形必须完整覆盖输出尺寸。
        assert_eq!(
            placements[0].rect,
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }
        );
    }

    /// 验证 focus-aware Fullscreen 在聚焦 slot 0 时显示 slot 0。
    #[test]
    fn fullscreen_with_focused_slot_zero_places_slot_zero() {
        let workspace = fullscreen_workspace_with_four_windows();

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(0), test_output());

        // focused slot 0 中的窗口 1 必须成为唯一可见窗口。
        assert_fullscreen_placement(&placements, 0, 1);
    }

    /// 验证 focus-aware Fullscreen 在聚焦 slot 1 时显示 slot 1。
    #[test]
    fn fullscreen_with_focused_slot_one_places_slot_one() {
        let workspace = fullscreen_workspace_with_four_windows();

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(1), test_output());

        // focused slot 1 中的窗口 2 必须成为唯一可见窗口。
        assert_fullscreen_placement(&placements, 1, 2);
    }

    /// 验证 focus-aware Fullscreen 在聚焦 slot 2 时显示 slot 2。
    #[test]
    fn fullscreen_with_focused_slot_two_places_slot_two() {
        let workspace = fullscreen_workspace_with_four_windows();

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(2), test_output());

        // focused slot 2 中的窗口 3 必须成为唯一可见窗口。
        assert_fullscreen_placement(&placements, 2, 3);
    }

    /// 验证 focus-aware Fullscreen 在聚焦 slot 3 时显示 slot 3。
    #[test]
    fn fullscreen_with_focused_slot_three_places_slot_three() {
        let workspace = fullscreen_workspace_with_four_windows();

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(3), test_output());

        // focused slot 3 中的窗口 4 必须成为唯一可见窗口。
        assert_fullscreen_placement(&placements, 3, 4);
    }

    /// 验证 focused slot 为空时回退到第一个 occupied slot。
    #[test]
    fn fullscreen_with_empty_focused_slot_falls_back_to_first_occupied_slot() {
        let mut workspace = fullscreen_workspace_with_four_windows();
        workspace.remove_window(1);

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(0), test_output());

        // slot 0 已空，fallback 必须选择按固定顺序遇到的第一个 occupied slot 1。
        assert_fullscreen_placement(&placements, 1, 2);
    }

    /// 验证空 workspace 在 Fullscreen 下安全返回空 placement。
    #[test]
    fn fullscreen_with_empty_workspace_returns_no_placement() {
        let workspace = Workspace::new(0);

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(3), test_output());

        // 没有任何 active window 时不能制造占位窗口，也不能发生 panic。
        assert!(placements.is_empty());
    }

    /// 验证 focused slot 为 Stack 时只显示其 active window。
    #[test]
    fn fullscreen_with_focused_stack_places_only_active_window() {
        let mut workspace = Workspace::new(0);
        for window in 1..=5 {
            workspace.assign_window(window);
        }
        workspace.layout = LayoutMode::Fullscreen;

        let placements =
            LayoutEngine::compute_workspace_with_focus(&workspace, Some(0), test_output());

        // 第五个窗口进入 slot 0 stack 后成为 active，旧窗口 1 必须保持隐藏。
        assert_fullscreen_placement(&placements, 0, 5);
        assert!(placements.iter().all(|placement| placement.window != 1));
    }

    /// 验证 Split 在奇数宽度下把余数分配给右侧区域。
    #[test]
    fn split_assigns_odd_width_remainder_to_right_side() {
        let mut workspace = Workspace::new(0);
        workspace.assign_window(1);
        workspace.assign_window(2);
        workspace.layout = LayoutMode::Split;
        let output = OutputSize {
            width: 1367,
            height: 768,
        };

        let placements = LayoutEngine::compute_workspace(&workspace, output);

        // 两个已占用 slot 必须分别生成左右两个 placement。
        assert_eq!(placements.len(), 2);

        // 左侧使用整数除法结果 683。
        assert_eq!(placements[0].rect.width, 683);

        // 右侧从 x=683 开始，并吸收剩余像素得到宽度 684。
        assert_eq!(placements[1].rect.x, 683);
        assert_eq!(placements[1].rect.width, 684);
    }

    /// 验证 Grid 在奇数宽高下由右列和下行吸收余数。
    #[test]
    fn grid_assigns_odd_remainders_to_right_and_bottom() {
        let mut workspace = Workspace::new(0);
        for window in 1..=4 {
            workspace.assign_window(window);
        }
        workspace.layout = LayoutMode::Grid;
        let output = OutputSize {
            width: 1367,
            height: 769,
        };

        let placements = LayoutEngine::compute_workspace(&workspace, output);

        // 四个已占用 slot 必须生成完整 2x2 网格。
        assert_eq!(placements.len(), 4);

        // 左列宽度为 683，右列吸收一个余数像素后宽度为 684。
        assert_eq!(placements[0].rect.width, 683);
        assert_eq!(placements[2].rect.width, 683);
        assert_eq!(placements[1].rect.width, 684);
        assert_eq!(placements[3].rect.width, 684);

        // 上行高度为 384，下行吸收一个余数像素后高度为 385。
        assert_eq!(placements[0].rect.height, 384);
        assert_eq!(placements[1].rect.height, 384);
        assert_eq!(placements[2].rect.height, 385);
        assert_eq!(placements[3].rect.height, 385);
    }

    /// 验证 slot 形成 stack 后，布局只包含 active window。
    #[test]
    fn layout_only_places_active_stack_window() {
        let mut workspace = Workspace::new(0);
        for window in 1..=5 {
            workspace.assign_window(window);
        }
        workspace.layout = LayoutMode::Fullscreen;

        let placements = LayoutEngine::compute_workspace(
            &workspace,
            OutputSize {
                width: 1920,
                height: 1080,
            },
        );

        // Fullscreen 下 slot 0 只应产生一个 active window placement。
        assert_eq!(placements.len(), 1);

        // 第五个窗口加入 stack 后成为 active，因此必须被布局。
        assert_eq!(placements[0].window, 5);

        // 被 stack 遮挡的旧窗口 1 不得出现在任何 placement 中。
        assert!(placements.iter().all(|placement| placement.window != 1));
    }
}
