//! 渲染前的纯数据场景图。
//!
//! SceneBuilder 将 LayoutEngine 产生的几何 placement 与 FocusState 推导出的
//! focused window 合并为 SceneFrame。该过程不访问 renderer，也不修改任何状态。

use crate::core::{
    layout::{Rect, WindowPlacement},
    workspace::WindowId,
};

/// 当前帧中的一个可见窗口节点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneNode {
    /// 节点对应的逻辑窗口 ID。
    pub window: WindowId,
    /// 节点在输出中的目标矩形。
    pub rect: Rect,
    /// 节点所属 workspace ID。
    pub workspace: u32,
    /// 节点来源的 slot ID。
    pub slot: u8,
    /// 该窗口是否等于 FocusState 中的当前窗口。
    pub focused: bool,
    /// 简化后的绘制层级。
    ///
    /// 当前 MVP 使用 0 表示普通窗口，10 表示焦点窗口。
    pub z_index: i32,
}

/// 当前 workspace 的完整可见场景快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneFrame {
    /// 该帧描述的 workspace ID。
    pub workspace: u32,
    /// 已按 z-index 从低到高排序的可见节点。
    pub nodes: Vec<SceneNode>,
}

/// 无状态的 SceneFrame 构建器。
pub struct SceneBuilder;

impl SceneBuilder {
    /// 将布局结果和当前焦点合并成场景帧。
    ///
    /// 输入 placements 按值传入并转换为新节点，不会回写 LayoutEngine 或 compositor 状态。
    pub fn build(
        workspace: u32,
        focused_window: Option<WindowId>,
        placements: Vec<WindowPlacement>,
    ) -> SceneFrame {
        // 每个 placement 一一转换为 SceneNode，并根据窗口 ID 推导焦点标记。
        let mut nodes: Vec<SceneNode> = placements
            .into_iter()
            .map(|placement| {
                // FocusState 使用 Option，因此空焦点不会误标记任何节点。
                let focused = focused_window == Some(placement.window);

                SceneNode {
                    window: placement.window,
                    rect: placement.rect,
                    workspace: placement.workspace,
                    slot: placement.slot,
                    focused,
                    z_index: if focused { 10 } else { 0 },
                }
            })
            .collect();

        // 普通窗口先绘制，焦点窗口后绘制，从而为后续 renderer 提供简单覆盖顺序。
        nodes.sort_by_key(|node| node.z_index);

        SceneFrame { workspace, nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::SceneBuilder;
    use crate::core::layout::{Rect, WindowPlacement};

    /// 验证焦点窗口具有更高层级，并在升序排序后位于普通窗口之后。
    #[test]
    fn focused_node_has_higher_z_index_and_is_sorted_last() {
        let placements = vec![
            WindowPlacement {
                window: 1,
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 720,
                },
                workspace: 0,
                slot: 0,
            },
            WindowPlacement {
                window: 2,
                rect: Rect {
                    x: 640,
                    y: 0,
                    width: 640,
                    height: 720,
                },
                workspace: 0,
                slot: 1,
            },
        ];

        let frame = SceneBuilder::build(0, Some(2), placements);

        // 两个 placement 必须一一转换为两个 SceneNode。
        assert_eq!(frame.nodes.len(), 2);

        // 普通窗口必须保持非焦点状态和基础层级 0。
        assert_eq!(frame.nodes[0].window, 1);
        assert!(!frame.nodes[0].focused);
        assert_eq!(frame.nodes[0].z_index, 0);

        // 焦点窗口必须标记为 focused，并使用更高层级 10。
        assert_eq!(frame.nodes[1].window, 2);
        assert!(frame.nodes[1].focused);
        assert_eq!(frame.nodes[1].z_index, 10);

        // 节点按 z-index 升序排列，因此焦点窗口必须位于最后。
        assert!(frame.nodes[0].z_index < frame.nodes[1].z_index);
    }
}
