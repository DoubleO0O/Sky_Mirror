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
    /// `slot_window()` 只返回 active window。返回顺序与 slot 顺序一致。
    pub fn compute_workspace(workspace: &Workspace, output: OutputSize) -> Vec<WindowPlacement> {
        match workspace.layout {
            // Fullscreen 只显示 slot 0，并占满整个输出。
            LayoutMode::Fullscreen => {
                // slot 0 没有可见窗口时，没有任何内容需要布局。
                let Some(window) = workspace.slot_window(0) else {
                    return Vec::new();
                };

                vec![WindowPlacement {
                    window,
                    workspace: workspace.id,
                    slot: 0,
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
}

// 布局测试只验证纯几何计算，不启动 backend、EventLoop 或 renderer。
#[cfg(test)]
mod tests {
    use super::{LayoutEngine, OutputSize, Rect};
    use crate::core::workspace::{LayoutMode, Workspace};

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
