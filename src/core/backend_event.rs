//! 后端或 protocol 事件到核心命令的纯数据适配层。
//!
//! `BackendEvent` 描述外部世界已经发生的事实，`CoreCommand` 描述核心 `State`
//! 可以执行的命令。未来 Smithay 回调应先转换成不含真实协议对象的
//! `BackendEvent`，再由本模块翻译为 `CoreCommand`。
//!
//! 本阶段不接入 Smithay，不保存 Wayland surface，也不直接修改任何状态。

use crate::core::{
    action::Action,
    client::{ClientId, ClientKind},
    command::CoreCommand,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::WindowId,
};

/// 后端或 protocol 层产生的外部事件。
///
/// `BackendEvent` 表示外部世界发生了什么，例如 surface 被创建、toplevel 被
/// map、surface 被销毁或输出尺寸发生变化。
///
/// 它不同于 `CoreCommand`：`BackendEvent` 是输入事实，`CoreCommand` 是核心
/// `State` 可以执行的命令。本阶段只使用纯数据模拟未来回调，不持有任何真实
/// Smithay 类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    /// 后端发现一个新的 Wayland client 连接。
    ///
    /// 当前只是纯数据事件，不携带真实连接流，也不会把 client 注册到 Wayland display。
    ClientConnected {
        /// 可选外部指定 client ID。
        client: Option<ClientId>,

        /// client 来源类型。
        kind: ClientKind,

        /// 可选调试名称。
        name: Option<String>,
    },

    /// 后端发现一个 Wayland client 断开。
    ///
    /// 该事件只表达外部断开事实；翻译后的 CloseClient 命令由 State 统一级联
    /// 收束 client、surface、window、workspace 和 focus 的纯数据生命周期。
    ClientDisconnected {
        /// 断开的 client ID。
        client: ClientId,
    },

    /// 后端发现了一个新的 surface，但它尚未 map 成窗口。
    SurfaceCreated {
        /// 外部系统分配或模拟出来的 surface ID。
        surface: SurfaceId,

        /// 可选的 surface 所属 client。
        ///
        /// None 表示当前后端事件还无法确定 client 归属。
        client: Option<ClientId>,

        /// surface 的协议角色。
        role: SurfaceRole,
    },

    /// 后端确认已有 surface 属于某个 client。
    ///
    /// 该事件只建立纯数据归属关系，不创建 WindowId，也不接入真实 Smithay client。
    SurfaceAssignedToClient {
        /// 已存在的 surface ID。
        surface: SurfaceId,

        /// 已存在的 client ID。
        client: ClientId,
    },

    /// 已存在 surface 被 map 成一个普通窗口。
    ToplevelMapped {
        /// 已存在的 surface ID。
        surface: SurfaceId,

        /// 窗口标题。
        title: String,

        /// 应用 ID。
        app_id: Option<String>,

        /// 窗口来源类型。
        kind: WindowKind,
    },

    /// 已映射 toplevel 被 unmap，但底层 surface 仍保持存活。
    ///
    /// 该纯数据事实只携带已知 ID，不代表真实 XDG callback 或 protocol dispatch
    /// 已经接入；core 会验证精确 link 后 detach WindowId。
    ToplevelUnmapped {
        /// 仍应保持存活的 surface identity。
        surface: SurfaceId,

        /// 需要从 surface detach 并结束 active 生命周期的窗口 identity。
        window: WindowId,
    },

    /// 已存在 surface 与一个已存在窗口建立绑定。
    SurfaceBoundToWindow {
        /// 已存在的 surface ID。
        surface: SurfaceId,

        /// 已存在的逻辑窗口 ID。
        window: WindowId,
    },

    /// surface 被 terminal close 或 destroy。
    SurfaceClosed {
        /// 被关闭的 surface ID。
        surface: SurfaceId,
    },

    /// 后端请求 terminal close 指定窗口及其绑定 surface。
    WindowClosed {
        /// 被关闭的逻辑窗口 ID。
        window: WindowId,
    },

    /// 后端报告输出尺寸变化。
    OutputResized {
        /// 新输出宽度。
        width: u32,

        /// 新输出高度。
        height: u32,
    },

    /// 后端产生了一个已经解析好的用户 Action。
    ///
    /// 该事件用于未来真实输入层接入后，复用现有 Action 语义。
    ActionRequested(
        /// 后端请求核心执行的用户意图。
        Action,
    ),

    /// 后端请求当前完整诊断文本。
    DebugRequested,

    /// 后端请求只运行状态验证。
    ValidateRequested,
}

/// 后端事件翻译器。
///
/// `BackendEventTranslator` 不持有状态，也不直接修改 `State`。未来 Smithay
/// 回调应先构造 `BackendEvent`，再通过该翻译器生成核心可以处理的
/// `CoreCommand`。
pub struct BackendEventTranslator;

impl BackendEventTranslator {
    /// 将后端事件转换为核心命令。
    ///
    /// 转换过程是纯函数，不读取 `State`，不修改 `State`，也不保存真实
    /// Wayland 或 Smithay 对象。
    pub fn translate(event: BackendEvent) -> CoreCommand {
        match event {
            BackendEvent::ClientConnected { client, kind, name } => {
                CoreCommand::RegisterClient { client, kind, name }
            }
            BackendEvent::ClientDisconnected { client } => CoreCommand::CloseClient(client),
            BackendEvent::SurfaceCreated {
                surface,
                client,
                role,
            } => CoreCommand::RegisterSurface {
                surface: Some(surface),
                client,
                role,
            },
            BackendEvent::SurfaceAssignedToClient { surface, client } => {
                CoreCommand::BindSurfaceToClient { surface, client }
            }
            BackendEvent::ToplevelMapped {
                surface,
                title,
                app_id,
                kind,
            } => CoreCommand::RegisterWindowForSurface {
                surface,
                title,
                app_id,
                kind,
            },
            BackendEvent::ToplevelUnmapped { surface, window } => {
                CoreCommand::DetachWindowFromSurface { surface, window }
            }
            BackendEvent::SurfaceBoundToWindow { surface, window } => {
                CoreCommand::BindSurfaceToWindow { surface, window }
            }
            BackendEvent::SurfaceClosed { surface } => CoreCommand::CloseSurface(surface),
            BackendEvent::WindowClosed { window } => CoreCommand::CloseWindow(window),
            BackendEvent::OutputResized { width, height } => {
                CoreCommand::Action(Action::ResizeOutput { width, height })
            }
            BackendEvent::ActionRequested(action) => CoreCommand::Action(action),
            BackendEvent::DebugRequested => CoreCommand::DebugText,
            BackendEvent::ValidateRequested => CoreCommand::Validate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendEvent, BackendEventTranslator};
    use crate::core::{
        action::Action, client::ClientKind, command::CoreCommand, surface::SurfaceRole,
        window::WindowKind,
    };

    /// 验证 client 连接事实会完整转换为注册 client 命令。
    #[test]
    fn backend_event_client_connected_translates_to_register_client() {
        let command = BackendEventTranslator::translate(BackendEvent::ClientConnected {
            client: Some(42),
            kind: ClientKind::WaylandPlaceholder,
            name: Some("终端".to_string()),
        });

        // 翻译必须保留外部 ID、来源和调试名称。
        assert_eq!(
            command,
            CoreCommand::RegisterClient {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("终端".to_string()),
            }
        );
    }

    /// 验证 client 断开事实会转换为关闭 client 命令。
    #[test]
    fn backend_event_client_disconnected_translates_to_close_client() {
        let command =
            BackendEventTranslator::translate(BackendEvent::ClientDisconnected { client: 42 });

        // client 生命周期事件必须保留 ClientId，不能误用 SurfaceId 或 WindowId 命令。
        assert_eq!(command, CoreCommand::CloseClient(42));
    }

    /// 验证 surface 创建事实会保留 backend 提供的稳定 ID。
    #[test]
    fn backend_event_surface_created_translates_to_register_surface_with_id() {
        let command = BackendEventTranslator::translate(BackendEvent::SurfaceCreated {
            surface: 42,
            client: None,
            role: SurfaceRole::XdgToplevel,
        });

        // 翻译结果必须使用显式 ID 注册路径，不能丢弃 backend identity。
        assert_eq!(
            command,
            CoreCommand::RegisterSurface {
                surface: Some(42),
                client: None,
                role: SurfaceRole::XdgToplevel,
            }
        );
    }

    /// 验证带 client 的 surface 创建事件会保留归属关系。
    #[test]
    fn backend_event_surface_created_with_client_translates_to_register_surface_with_client() {
        let command = BackendEventTranslator::translate(BackendEvent::SurfaceCreated {
            surface: 42,
            client: Some(7),
            role: SurfaceRole::XdgToplevel,
        });

        // 注册命令必须完整保留 SurfaceId、ClientId 和协议角色。
        assert_eq!(
            command,
            CoreCommand::RegisterSurface {
                surface: Some(42),
                client: Some(7),
                role: SurfaceRole::XdgToplevel,
            }
        );
    }

    /// 验证后端确认 surface 归属时会转换为绑定 client 命令。
    #[test]
    fn backend_event_surface_assigned_to_client_translates_to_bind_surface_to_client() {
        let command = BackendEventTranslator::translate(BackendEvent::SurfaceAssignedToClient {
            surface: 42,
            client: 7,
        });

        // 归属事件不得创建窗口，只能生成明确的 client 绑定命令。
        assert_eq!(
            command,
            CoreCommand::BindSurfaceToClient {
                surface: 42,
                client: 7,
            }
        );
    }

    /// 验证 toplevel map 事件会完整保留 surface 与窗口 metadata。
    #[test]
    fn backend_event_toplevel_mapped_translates_to_register_window_for_surface() {
        let command = BackendEventTranslator::translate(BackendEvent::ToplevelMapped {
            surface: 42,
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });

        // map 事件必须进入已有 surface 创建窗口的命令路径。
        assert_eq!(
            command,
            CoreCommand::RegisterWindowForSurface {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            }
        );
    }

    /// 验证 surface 关闭事实会转换为按 SurfaceId 关闭的核心命令。
    #[test]
    fn backend_event_surface_closed_translates_to_close_surface() {
        let command =
            BackendEventTranslator::translate(BackendEvent::SurfaceClosed { surface: 42 });

        // surface 生命周期事件不能被错误翻译为按 WindowId 关闭。
        assert_eq!(command, CoreCommand::CloseSurface(42));
    }

    /// 验证 toplevel unmap 事实会保留精确 surface/window pair。
    #[test]
    fn backend_event_toplevel_unmapped_translates_to_detach_window_from_surface() {
        let command = BackendEventTranslator::translate(BackendEvent::ToplevelUnmapped {
            surface: 42,
            window: 7,
        });

        assert_eq!(
            command,
            CoreCommand::DetachWindowFromSurface {
                surface: 42,
                window: 7,
            }
        );
    }

    /// 验证窗口关闭事实会转换为按 WindowId 清理的核心命令。
    #[test]
    fn backend_event_window_closed_translates_to_close_window() {
        let command = BackendEventTranslator::translate(BackendEvent::WindowClosed { window: 42 });

        // 翻译器只保留纯数据 identity，不得直接读取或修改 workspace、focus 或 registry。
        assert_eq!(command, CoreCommand::CloseWindow(42));
    }

    /// 验证输出尺寸变化会复用现有 ResizeOutput Action 语义。
    #[test]
    fn backend_event_output_resized_translates_to_resize_action() {
        let command = BackendEventTranslator::translate(BackendEvent::OutputResized {
            width: 1366,
            height: 768,
        });

        // 输出事件必须通过 CoreCommand::Action 进入既有状态修改链路。
        assert_eq!(
            command,
            CoreCommand::Action(Action::ResizeOutput {
                width: 1366,
                height: 768,
            })
        );
    }

    /// 验证诊断请求会转换为只读调试文本命令。
    #[test]
    fn backend_event_debug_requested_translates_to_debug_text() {
        let command = BackendEventTranslator::translate(BackendEvent::DebugRequested);

        // 诊断请求不得直接读取 State，翻译器只生成对应命令。
        assert_eq!(command, CoreCommand::DebugText);
    }
}
