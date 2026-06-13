//! renderer 边界层的纯数据规划与占位实现。
//!
//! RenderPlanner 把 SceneFrame 转换为未来真实 renderer 可消费的 RenderCommand。
//! MockRenderer 当前只打印命令，不连接 wgpu、OpenGL、Vulkan、Smithay renderer
//! 或真实 Wayland surface。

use crate::core::{
    layout::Rect,
    scene::SceneFrame,
    window::{WindowKind, WindowRegistry},
    workspace::WindowId,
};

/// 渲染命令中携带的窗口 metadata 快照。
///
/// 这是从 WindowRegistry 中复制出来的轻量调试信息，不持有 registry 引用，
/// 因此 RenderFrame 可以作为独立快照交给 renderer 消费。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderWindowMetadata {
    /// 窗口标题。
    pub title: String,

    /// 应用 ID。
    pub app_id: Option<String>,

    /// 窗口来源类型。
    pub kind: WindowKind,

    /// 窗口在 registry 中是否仍被标记为存活。
    pub alive: bool,
}

/// renderer 在一帧中需要执行的单条绘制命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderCommand {
    /// 绘制一个窗口。
    ///
    /// metadata 是从 WindowRegistry 合并进来的运行时快照。
    /// 如果 registry 中找不到对应 WindowId，则为 None，renderer 仍可以绘制占位窗口。
    DrawWindow {
        /// 需要绘制的稳定窗口 ID。
        window: WindowId,

        /// 从 WindowRegistry 复制出的窗口 metadata。
        metadata: Option<RenderWindowMetadata>,

        /// 窗口在当前输出上的矩形区域。
        rect: Rect,

        /// 该窗口是否是当前焦点窗口。
        focused: bool,

        /// 简化 z-index，focused window 通常更高。
        z_index: i32,
    },
}

/// renderer 消费的一帧完整绘制计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFrame {
    /// 当前帧对应的 workspace ID。
    pub workspace: u32,

    /// 当前帧需要执行的绘制命令列表。
    pub commands: Vec<RenderCommand>,
}

impl RenderFrame {
    /// 从 WindowRegistry 中复制窗口 metadata，并附加到每个 DrawWindow 命令。
    ///
    /// metadata 合并发生在 State 层调用该方法时，而不是 RenderPlanner 内部：
    /// RenderPlanner 只知道 SceneFrame，不能直接依赖全局 registry。
    pub fn attach_metadata(&mut self, registry: &WindowRegistry) {
        for command in &mut self.commands {
            match command {
                RenderCommand::DrawWindow {
                    window, metadata, ..
                } => {
                    // registry 缺失记录时 map 返回 None，renderer 仍可使用窗口 ID 和矩形。
                    *metadata = registry.get(*window).map(|record| RenderWindowMetadata {
                        title: record.title.clone(),
                        app_id: record.app_id.clone(),
                        kind: record.kind.clone(),
                        alive: record.alive,
                    });
                }
            }
        }
    }
}

/// 将 SceneFrame 转换为 RenderFrame 的无状态规划器。
pub struct RenderPlanner;

impl RenderPlanner {
    /// 将 SceneFrame 转换为 RenderFrame。
    ///
    /// 该函数只做 scene 到 render command 的纯数据转换，不访问 WindowRegistry。
    /// metadata 会保持为 None，随后由 State 层调用 `RenderFrame::attach_metadata()` 补齐。
    pub fn from_scene(scene: &SceneFrame) -> RenderFrame {
        // SceneNode 与 DrawWindow 命令一一对应，保留几何、焦点和层级信息。
        // SceneFrame 不知道 WindowRegistry，因此纯规划阶段暂时保留 metadata=None。
        let commands = scene
            .nodes
            .iter()
            .map(|node| RenderCommand::DrawWindow {
                window: node.window,
                metadata: None,
                rect: node.rect,
                focused: node.focused,
                z_index: node.z_index,
            })
            .collect();

        RenderFrame {
            workspace: scene.workspace,
            commands,
        }
    }
}

/// 当前阶段使用的日志型 renderer。
///
/// 该类型刻意不保存 compositor 状态，未来可以在 EventLoop 边界替换为真实 renderer。
pub struct MockRenderer;

impl MockRenderer {
    /// 创建无内部资源的 mock renderer。
    pub fn new() -> Self {
        Self
    }

    /// 输出 RenderFrame 内容，模拟提交一帧绘制命令。
    ///
    /// 本方法不会修改 frame，也不会进行任何真实 GPU 绘制。
    pub fn render(&mut self, frame: &RenderFrame) {
        println!(
            "[Render] workspace={}, commands={:?}",
            frame.workspace, frame.commands
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderCommand, RenderFrame, RenderPlanner};
    use crate::core::{
        layout::Rect,
        scene::{SceneFrame, SceneNode},
        window::{WindowKind, WindowRegistry},
    };

    /// 验证 RenderPlanner 保留 workspace，并严格维持 SceneNode 的原始顺序。
    #[test]
    fn render_planner_preserves_scene_node_order() {
        let first_rect = Rect {
            x: 640,
            y: 0,
            width: 640,
            height: 720,
        };
        let second_rect = Rect {
            x: 0,
            y: 0,
            width: 640,
            height: 720,
        };
        let scene = SceneFrame {
            workspace: 7,
            nodes: vec![
                SceneNode {
                    window: 2,
                    rect: first_rect,
                    workspace: 7,
                    slot: 1,
                    focused: true,
                    z_index: 10,
                },
                SceneNode {
                    window: 1,
                    rect: second_rect,
                    workspace: 7,
                    slot: 0,
                    focused: false,
                    z_index: 0,
                },
            ],
        };

        let frame = RenderPlanner::from_scene(&scene);

        // RenderFrame 必须继承 SceneFrame 的 workspace ID。
        assert_eq!(frame.workspace, 7);

        // 每个 SceneNode 必须生成且只生成一条绘制命令。
        assert_eq!(frame.commands.len(), 2);

        // 第一条命令必须对应 scene.nodes 中的第一个窗口，不能按层级再次排序。
        assert_eq!(
            frame.commands[0],
            RenderCommand::DrawWindow {
                window: 2,
                metadata: None,
                rect: first_rect,
                focused: true,
                z_index: 10,
            }
        );

        // 第二条命令必须对应 scene.nodes 中的第二个窗口。
        assert_eq!(
            frame.commands[1],
            RenderCommand::DrawWindow {
                window: 1,
                metadata: None,
                rect: second_rect,
                focused: false,
                z_index: 0,
            }
        );
    }

    /// 验证 attach_metadata 会为绘制命令附加完整的窗口 metadata 快照。
    #[test]
    fn render_frame_attach_metadata_enriches_draw_window_commands() {
        let mut registry = WindowRegistry::new();
        let window = registry.create_mock();
        let mut frame = RenderFrame {
            workspace: 0,
            commands: vec![RenderCommand::DrawWindow {
                window,
                metadata: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                focused: true,
                z_index: 10,
            }],
        };

        frame.attach_metadata(&registry);

        let RenderCommand::DrawWindow { metadata, .. } = &frame.commands[0];
        let metadata = metadata.as_ref().expect("已注册窗口必须附加 metadata");

        // metadata 标题必须来自 registry 中的 mock 窗口记录。
        assert_eq!(metadata.title, "Mock Window 1");

        // app_id 必须保持 registry 中的 mock 标识。
        assert_eq!(metadata.app_id.as_deref(), Some("sky-mirror.mock"));

        // 窗口来源类型必须完整复制到渲染快照。
        assert_eq!(metadata.kind, WindowKind::Mock);

        // 新建窗口在 registry 中处于存活状态。
        assert!(metadata.alive);
    }

    /// 验证 registry 缺少窗口记录时，attach_metadata 保持 None 且不会 panic。
    #[test]
    fn render_frame_attach_metadata_keeps_none_for_missing_registry_record() {
        let registry = WindowRegistry::new();
        let mut frame = RenderFrame {
            workspace: 0,
            commands: vec![RenderCommand::DrawWindow {
                window: 999,
                metadata: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                focused: false,
                z_index: 0,
            }],
        };

        frame.attach_metadata(&registry);

        let RenderCommand::DrawWindow { metadata, .. } = &frame.commands[0];

        // 缺失记录不能生成伪 metadata，renderer 应继续看到 None。
        assert!(metadata.is_none());
    }
}
