//! 外部系统进入 compositor 核心状态的统一命令边界。
//!
//! InputEvent 描述输入设备事件，Action 描述用户意图，CoreCommand 则描述
//! backend、Wayland protocol 或未来 Smithay 回调需要提交给核心状态的系统命令。
//! 外部模块应构造命令并交给 State，而不是直接修改 workspace、registry 或 focus。

use crate::core::{
    action::Action,
    client::{ClientId, ClientKind},
    state::{DetachWindowFromSurfaceError, DetachWindowFromSurfaceResult, State},
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::WindowId,
};

/// 外部系统进入核心状态的统一命令。
///
/// `Action` 表示用户意图，例如快捷键触发的切换 workspace 或关闭焦点窗口。
/// `CoreCommand` 表示更高一层的系统命令边界，未来 Smithay、Wayland protocol、
/// backend 都应该通过它进入核心 State，而不是直接修改内部状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreCommand {
    /// 执行一个已有用户 Action。
    ///
    /// 该变体用于复用当前已经稳定的 Action 分发链路。
    Action(Action),

    /// 注册一个 Wayland client 占位记录。
    ///
    /// `client = None` 时由 `ClientRegistry` 自动分配 ID；
    /// `client = Some(id)` 时注册外部系统指定的 client ID。
    RegisterClient {
        /// 可选的外部指定 client ID。
        client: Option<ClientId>,

        /// client 来源类型。
        kind: ClientKind,

        /// 可选调试名称。
        name: Option<String>,
    },

    /// 关闭指定 client 占位记录。
    ///
    /// 命令由 State 统一标记 client dead，并级联关闭归属 surface 与绑定 window；
    /// Command 层不直接操作 workspace、slot 或 stack。
    CloseClient(ClientId),

    /// 注册一个新逻辑窗口，并把它分配到当前 workspace。
    ///
    /// 当前仍然不接真实 surface，只注册 metadata 和 WindowId。
    RegisterWindow {
        /// 窗口标题。
        title: String,

        /// 应用 ID。
        app_id: Option<String>,

        /// 窗口来源类型。
        kind: WindowKind,
    },

    /// 注册一个尚未绑定窗口的 surface。
    ///
    /// `surface = None` 时由 SurfaceRegistry 自动分配 ID；
    /// `surface = Some(id)` 时注册外部系统指定的 surface ID。
    /// `client` 表示该 surface 属于哪个 Wayland client；None 表示暂未绑定。
    RegisterSurface {
        /// 可选的外部指定 surface ID。
        surface: Option<SurfaceId>,

        /// 可选的 surface 所属 client。
        ///
        /// ClientId 只表达连接归属，不等于 SurfaceId 或 WindowId。
        client: Option<ClientId>,

        /// surface 的协议角色。
        role: SurfaceRole,
    },

    /// 将已有 surface 绑定到已有 client。
    ///
    /// 该命令只建立 ClientId 到 SurfaceId 的归属关系，不创建 window，
    /// 也不接入真实 Smithay client。
    BindSurfaceToClient {
        /// 已存在的 surface ID。
        surface: SurfaceId,

        /// 已存在的 client ID。
        client: ClientId,
    },

    /// 将已有 surface 绑定到已有逻辑窗口。
    ///
    /// 该命令用于未来 xdg_toplevel role 已确认后，把 protocol 层 surface
    /// 关联到核心 WindowId。
    BindSurfaceToWindow {
        /// 已存在的 surface ID。
        surface: SurfaceId,

        /// 已存在的逻辑窗口 ID。
        window: WindowId,
    },

    /// 为已有 surface 注册一个新逻辑窗口，并建立绑定。
    ///
    /// 该命令模拟未来 xdg_toplevel map：surface 已经存在，map 时创建 WindowRecord，
    /// 再把 surface 与新 WindowId 绑定。本阶段仍不持有真实 Smithay surface。
    RegisterWindowForSurface {
        /// 已存在的 surface ID。
        surface: SurfaceId,

        /// 窗口标题。
        title: String,

        /// 应用 ID。
        app_id: Option<String>,

        /// 窗口来源类型。
        kind: WindowKind,
    },

    /// 将逻辑窗口从仍存活的 surface 上 detach。
    ///
    /// 该命令用于 core 纯数据 toplevel unmap：结束 WindowId 的 active 生命周期，
    /// 清除精确 `SurfaceId -> WindowId` link，但不得关闭 SurfaceId。它与
    /// terminal `CloseWindow` 不是同义命令。
    DetachWindowFromSurface {
        /// 应继续保持存活的 surface identity。
        surface: SurfaceId,

        /// 应被 detach 并结束 active 生命周期的 window identity。
        window: WindowId,
    },

    /// 关闭指定逻辑窗口。
    ///
    /// 与 toplevel detach 不同，这是 terminal close：它会结束窗口以及绑定
    /// surface 的生命周期。外部系统已经知道具体 WindowId，因此不依赖当前焦点。
    CloseWindow(WindowId),

    /// 关闭或销毁指定 surface。
    ///
    /// 与 CloseWindow 不同，CloseSurface 的输入来自 protocol/backend 层并使用
    /// SurfaceId；如果该 surface 绑定了窗口，再同步关闭对应 WindowId。
    CloseSurface(SurfaceId),

    /// 请求生成当前完整诊断文本。
    ///
    /// 该命令不修改 State，只返回 DebugBundle 的多行文本。
    DebugText,

    /// 请求只运行状态验证。
    ///
    /// 该命令不修改 State，只返回 ValidationReport 的多行文本。
    Validate,
}

/// CoreCommand 执行后的结果。
///
/// CommandResult 用于把状态修改结果或调试文本返回给外部调用方。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// 命令已经执行，但没有额外返回值。
    None,

    /// 注册 client 的结果。
    ClientRegistered {
        /// 被注册或请求注册的 client ID。
        client: ClientId,

        /// 是否实际完成注册。
        registered: bool,
    },

    /// 关闭 client 的结果。
    ClientClosed {
        /// 被关闭的 client ID。
        client: ClientId,

        /// 是否成功标记为 dead。
        marked_dead: bool,

        /// 本次被标记为 dead 的 surface 列表。
        dead_surfaces: Vec<SurfaceId>,

        /// 因 client 断开而同步关闭的 window 列表。
        closed_windows: Vec<WindowId>,

        /// 成功从 workspace 移除引用的 window 数量。
        removed_from_workspace_count: usize,

        /// 成功在 WindowRegistry 中标记为 dead 的 window 数量。
        marked_window_dead_count: usize,
    },

    /// 创建或注册了一个窗口。
    WindowRegistered(WindowId),

    /// 注册了一个 surface。
    SurfaceRegistered {
        /// 被注册或请求注册的 surface ID。
        surface: SurfaceId,

        /// 是否实际完成注册。
        registered: bool,
    },

    /// surface 绑定 client 的结果。
    SurfaceBoundToClient {
        /// 被绑定的 surface ID。
        surface: SurfaceId,

        /// 目标 client ID。
        client: ClientId,

        /// 是否绑定成功。
        bound: bool,
    },

    /// surface 绑定窗口的结果。
    SurfaceBound {
        /// 被绑定的 surface ID。
        surface: SurfaceId,

        /// 目标窗口 ID。
        window: WindowId,

        /// 是否绑定成功。
        bound: bool,
    },

    /// 已有 surface map 成新窗口的结果。
    WindowRegisteredForSurface {
        /// 被绑定的 surface ID。
        surface: SurfaceId,

        /// 新创建的窗口 ID。
        window: WindowId,

        /// surface 是否成功绑定到窗口。
        bound: bool,
    },

    /// Toplevel detach 的成功结果或结构化拒绝原因。
    ToplevelDetached {
        /// 请求 detach 的 surface identity。
        surface: SurfaceId,

        /// 请求 detach 的 window identity。
        window: WindowId,

        /// 成功时包含 workspace/window cleanup；拒绝时不修改 State。
        result: Result<DetachWindowFromSurfaceResult, DetachWindowFromSurfaceError>,
    },

    /// 指定窗口已关闭或已标记为 dead。
    WindowClosed {
        /// 被请求关闭的窗口 ID。
        window: WindowId,

        /// 是否从 workspace 中移除了可见引用。
        removed_from_workspace: bool,

        /// 是否在 registry 中成功标记为 dead。
        marked_dead: bool,

        /// 被标记为 dead 的 surface 数量。
        dead_surfaces: usize,
    },

    /// 关闭 surface 的结果。
    SurfaceClosed {
        /// 被关闭的 surface ID。
        surface: SurfaceId,

        /// surface 是否被标记为 dead。
        surface_marked_dead: bool,

        /// 如果该 surface 绑定了窗口，这里记录被同步关闭的窗口。
        closed_window: Option<WindowId>,

        /// 是否从 workspace 中移除了窗口引用。
        removed_from_workspace: bool,

        /// 是否在 registry 中标记窗口为 dead。
        marked_window_dead: bool,
    },

    /// 返回调试或验证文本。
    Text(String),
}

/// 命令处理器。
///
/// CommandHandler 本身不持有状态，只提供把 CoreCommand 应用到 State 的统一函数。
pub struct CommandHandler;

impl CommandHandler {
    /// 在 State 上执行一个 CoreCommand。
    ///
    /// 该方法是未来 Smithay、Wayland protocol 和 backend 进入核心状态的统一边界。
    pub fn handle(state: &mut State, command: CoreCommand) -> CommandResult {
        match command {
            // 用户意图继续复用既有 Action 分发链路，不复制状态修改逻辑。
            CoreCommand::Action(action) => {
                state.dispatch_action(action);
                CommandResult::None
            }
            // client 连接只创建纯数据记录，不保存真实连接或协议对象。
            CoreCommand::RegisterClient { client, kind, name } => match client {
                Some(client) => {
                    let registered = state.register_client_with_id(client, kind, name);
                    CommandResult::ClientRegistered { client, registered }
                }
                None => {
                    let client = state.register_client(kind, name);
                    CommandResult::ClientRegistered {
                        client,
                        registered: true,
                    }
                }
            },
            // client 断开通过 State 统一收束 client、surface、window 和 workspace 生命周期。
            CoreCommand::CloseClient(client) => {
                let result = state.close_client(client);
                CommandResult::ClientClosed {
                    client,
                    marked_dead: result.marked_dead,
                    dead_surfaces: result.dead_surfaces,
                    closed_windows: result.closed_windows,
                    removed_from_workspace_count: result.removed_from_workspace_count,
                    marked_window_dead_count: result.marked_window_dead_count,
                }
            }
            // 外部 map 事件提供 metadata，由 State 协调 registry 和 workspace。
            CoreCommand::RegisterWindow {
                title,
                app_id,
                kind,
            } => {
                let window = state.register_window(title, app_id, kind);
                CommandResult::WindowRegistered(window)
            }
            // new surface 阶段可以使用外部稳定 ID，也可以由 registry 自动分配。
            CoreCommand::RegisterSurface {
                surface,
                client,
                role,
            } => match surface {
                Some(surface) => {
                    let registered =
                        state.register_surface_with_id_for_client(surface, client, role);
                    CommandResult::SurfaceRegistered {
                        surface,
                        registered,
                    }
                }
                None => {
                    let surface = state.register_surface_for_client(client, role);
                    CommandResult::SurfaceRegistered {
                        surface,
                        registered: true,
                    }
                }
            },
            // 归属绑定只关联已有纯数据记录，不创建 surface、window 或真实 client。
            CoreCommand::BindSurfaceToClient { surface, client } => {
                let bound = state.bind_surface_to_client(surface, client);
                CommandResult::SurfaceBoundToClient {
                    surface,
                    client,
                    bound,
                }
            }
            // protocol 层已经知道两个 ID 时，通过 State 校验并建立绑定。
            CoreCommand::BindSurfaceToWindow { surface, window } => {
                let bound = state.bind_surface_to_window(surface, window);
                CommandResult::SurfaceBound {
                    surface,
                    window,
                    bound,
                }
            }
            // map 阶段创建窗口，但复用已有 surface，避免自动生成重复占位记录。
            CoreCommand::RegisterWindowForSurface {
                surface,
                title,
                app_id,
                kind,
            } => {
                let result = state.register_window_for_surface(surface, title, app_id, kind);
                CommandResult::WindowRegisteredForSurface {
                    surface,
                    window: result.window,
                    bound: result.bound,
                }
            }
            // Toplevel unmap 通过 State 统一验证 pair 并协调 link/workspace/focus/window。
            CoreCommand::DetachWindowFromSurface { surface, window } => {
                let result = state.detach_window_from_surface(surface, window);
                CommandResult::ToplevelDetached {
                    surface,
                    window,
                    result,
                }
            }
            // Terminal close 已知具体 WindowId，不依赖当前焦点，并保留原有 surface cascade。
            CoreCommand::CloseWindow(window) => {
                let result = state.close_window(window);
                CommandResult::WindowClosed {
                    window,
                    removed_from_workspace: result.removed_from_workspace,
                    marked_dead: result.marked_dead,
                    dead_surfaces: result.dead_surfaces,
                }
            }
            // surface destroy/unmap 先结束 surface 生命周期，再同步关闭绑定窗口。
            CoreCommand::CloseSurface(surface) => {
                let result = state.close_surface(surface);
                CommandResult::SurfaceClosed {
                    surface,
                    surface_marked_dead: result.surface_marked_dead,
                    closed_window: result.closed_window,
                    removed_from_workspace: result.removed_from_workspace,
                    marked_window_dead: result.marked_window_dead,
                }
            }
            // 诊断命令只读取 State，并把独立文本返回给调用方。
            CoreCommand::DebugText => CommandResult::Text(state.debug_bundle_text()),
            // 验证命令不执行修复，只返回当前 ValidationReport 文本。
            CoreCommand::Validate => CommandResult::Text(state.validate().pretty_print()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandResult, CoreCommand};
    use crate::core::{
        action::Action, client::ClientKind, state::State, surface::SurfaceRole, window::WindowKind,
    };

    /// 验证 Action 命令会复用现有 dispatch_action 状态链路。
    #[test]
    fn command_action_dispatches_existing_action() {
        let mut state = State::new();
        let before = state.compositor.current_workspace;

        let result = state.handle_command(CoreCommand::Action(Action::NextWorkspace));

        // Action 命令没有额外返回值。
        assert_eq!(result, CommandResult::None);

        // NextWorkspace 必须由现有 Action 分发逻辑完成切换。
        assert_ne!(state.compositor.current_workspace, before);
    }

    /// 验证注册 client 命令会自动分配稳定 ID。
    #[test]
    fn command_register_client_auto_allocates_id() {
        let mut state = State::new();

        let result = state.handle_command(CoreCommand::RegisterClient {
            client: None,
            kind: ClientKind::Mock,
            name: Some("测试 client".to_string()),
        });
        let CommandResult::ClientRegistered { client, registered } = result else {
            panic!("注册 client 命令必须返回 ClientRegistered");
        };

        // 自动注册路径必须报告成功，并创建对应存活记录。
        assert!(registered);
        assert_eq!(client, 1);
        assert!(state.clients.is_alive(client));
        assert_eq!(
            state
                .clients
                .get(client)
                .expect("client metadata 必须存在")
                .name
                .as_deref(),
            Some("测试 client")
        );
    }

    /// 验证注册 client 命令可以接受外部指定 ID。
    #[test]
    fn command_register_client_accepts_explicit_id() {
        let mut state = State::new();

        let result = state.handle_command(CoreCommand::RegisterClient {
            client: Some(42),
            kind: ClientKind::WaylandPlaceholder,
            name: Some("终端".to_string()),
        });

        // 命令结果必须保留外部 ID，并明确报告注册成功。
        assert_eq!(
            result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert_eq!(
            state.clients.get(42).expect("显式 client 必须存在").kind,
            ClientKind::WaylandPlaceholder
        );
    }

    /// 验证重复显式 ClientId 会返回失败且不覆盖已有记录。
    #[test]
    fn command_register_client_rejects_duplicate_explicit_id() {
        let mut state = State::new();

        let first = state.handle_command(CoreCommand::RegisterClient {
            client: Some(42),
            kind: ClientKind::Mock,
            name: None,
        });
        let second = state.handle_command(CoreCommand::RegisterClient {
            client: Some(42),
            kind: ClientKind::WaylandPlaceholder,
            name: Some("重复记录".to_string()),
        });

        // 首次注册成功，重复 ID 必须明确报告失败。
        assert_eq!(
            first,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert_eq!(
            second,
            CommandResult::ClientRegistered {
                client: 42,
                registered: false,
            }
        );

        // 失败的重复注册不得覆盖首次记录。
        assert_eq!(
            state.clients.get(42).expect("原 client 必须保留").kind,
            ClientKind::Mock
        );
    }

    /// 验证没有 surface 的 client 断开时只标记 client dead。
    #[test]
    fn command_close_client_without_surfaces_only_marks_client_dead() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);

        let result = state.handle_command(CoreCommand::CloseClient(client));

        // 没有 owned surface 时，级联列表和窗口计数必须保持为空。
        assert_eq!(
            result,
            CommandResult::ClientClosed {
                client,
                marked_dead: true,
                dead_surfaces: Vec::new(),
                closed_windows: Vec::new(),
                removed_from_workspace_count: 0,
                marked_window_dead_count: 0,
            }
        );
        assert!(!state.clients.is_alive(client));

        // 无 surface 的 client 关闭不应修改默认窗口或 workspace。
        assert_eq!(state.registry.records().len(), 3);
        assert_eq!(
            state
                .compositor
                .current_workspace()
                .expect("默认 workspace 必须存在")
                .window_ids()
                .len(),
            3
        );
    }

    /// 验证 client 断开会级联关闭 owned surface 和绑定窗口。
    #[test]
    fn command_close_client_cascades_to_owned_surface_and_window() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        let surface = state.register_surface_for_client(Some(client), SurfaceRole::XdgToplevel);
        let mapped = state.register_window_for_surface(
            surface,
            "Terminal",
            Some("foot".to_string()),
            WindowKind::WaylandPlaceholder,
        );
        assert!(mapped.bound);

        let result = state.handle_command(CoreCommand::CloseClient(client));

        // 级联结果必须完整列出本次关闭的 surface、window 和两类窗口计数。
        assert_eq!(
            result,
            CommandResult::ClientClosed {
                client,
                marked_dead: true,
                dead_surfaces: vec![surface],
                closed_windows: vec![mapped.window],
                removed_from_workspace_count: 1,
                marked_window_dead_count: 1,
            }
        );

        // records 必须继续存在，但 client、surface 和 window 都处于 dead 状态。
        assert!(!state.clients.is_alive(client));
        assert!(!state.surfaces.is_alive(surface));
        assert!(!state.registry.is_alive(mapped.window));
        assert!(state.surfaces.get(surface).is_some());
        assert!(state.registry.get(mapped.window).is_some());

        // workspace 不得继续引用被级联关闭的窗口。
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&mapped.window))
        );
    }

    /// 验证关闭不存在的 client 会返回空级联结果。
    #[test]
    fn command_close_missing_client_returns_empty_cascade() {
        let mut state = State::new();

        let result = state.handle_command(CoreCommand::CloseClient(999));

        // 缺失 client 不得误关闭任何悬空 surface 或窗口。
        assert_eq!(
            result,
            CommandResult::ClientClosed {
                client: 999,
                marked_dead: false,
                dead_surfaces: Vec::new(),
                closed_windows: Vec::new(),
                removed_from_workspace_count: 0,
                marked_window_dead_count: 0,
            }
        );
        assert_eq!(state.registry.records().len(), 3);
        assert!(state.surfaces.records().is_empty());
    }

    /// 验证注册窗口命令会创建 metadata 并分配到当前 workspace。
    #[test]
    fn command_register_window_creates_metadata_and_assigns_workspace() {
        let mut state = State::new();

        let result = state.handle_command(CoreCommand::RegisterWindow {
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });
        let CommandResult::WindowRegistered(window) = result else {
            panic!("注册窗口命令必须返回 WindowRegistered");
        };

        let record = state
            .registry
            .get(window)
            .expect("注册窗口必须创建 registry metadata");

        // metadata 必须完整保留外部命令提供的字段。
        assert_eq!(record.title, "Terminal");
        assert_eq!(record.app_id.as_deref(), Some("foot"));
        assert_eq!(record.kind, WindowKind::WaylandPlaceholder);

        // 新窗口必须被分配到当前 workspace。
        assert!(
            state
                .compositor
                .current_workspace()
                .expect("默认当前 workspace 必须存在")
                .window_ids()
                .contains(&window)
        );

        let surface = state
            .surfaces
            .records()
            .iter()
            .find(|surface| surface.window == Some(window))
            .expect("WaylandPlaceholder 必须创建 surface 占位绑定");

        // WaylandPlaceholder 当前固定模拟一个 XdgToplevel surface。
        assert_eq!(surface.role, SurfaceRole::XdgToplevel);
        assert!(surface.alive);

        // 命令执行后仍应满足全部核心不变量。
        assert!(state.validate().is_valid());
    }

    /// 验证关闭指定窗口命令会删除 workspace 引用并标记 registry 为 dead。
    #[test]
    fn command_close_window_removes_workspace_reference_and_marks_dead() {
        let mut state = State::new();
        let register = state.handle_command(CoreCommand::RegisterWindow {
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });
        let CommandResult::WindowRegistered(window) = register else {
            panic!("测试窗口必须注册成功");
        };

        let result = state.handle_command(CoreCommand::CloseWindow(window));
        let CommandResult::WindowClosed {
            window: closed,
            removed_from_workspace,
            marked_dead,
            dead_surfaces,
        } = result
        else {
            panic!("关闭窗口命令必须返回 WindowClosed");
        };

        // 返回结果必须对应原始请求窗口。
        assert_eq!(closed, window);

        // workspace 和 registry 两侧状态都必须成功更新。
        assert!(removed_from_workspace);
        assert!(marked_dead);
        assert_eq!(dead_surfaces, 1);
        assert!(!state.registry.is_alive(window));

        // 绑定到目标窗口的占位 surface 必须全部结束生命周期。
        assert!(
            state
                .surfaces
                .records()
                .iter()
                .filter(|surface| surface.window == Some(window))
                .all(|surface| !surface.alive)
        );

        // 所有 workspace 都不得继续引用已关闭窗口。
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );
    }

    /// 验证调试文本命令返回 State 当前完整诊断文本。
    #[test]
    fn command_debug_text_returns_debug_bundle_text() {
        let mut state = State::new();
        let expected = state.debug_bundle_text();

        let result = state.handle_command(CoreCommand::DebugText);

        // 命令结果必须与直接调用只读便捷入口完全一致。
        assert_eq!(result, CommandResult::Text(expected));
    }

    /// 验证状态验证命令返回当前 ValidationReport 文本。
    #[test]
    fn command_validate_returns_validation_text() {
        let mut state = State::new();
        let expected = state.validate().pretty_print();

        let result = state.handle_command(CoreCommand::Validate);

        // 验证命令只返回文本，不修改状态或自动修复。
        assert_eq!(result, CommandResult::Text(expected));
    }

    /// 验证注册 surface 命令会创建存活且未绑定窗口的记录。
    #[test]
    fn command_register_surface_creates_unbound_surface() {
        let mut state = State::new();

        let result = state.handle_command(CoreCommand::RegisterSurface {
            surface: None,
            client: None,
            role: SurfaceRole::Unknown,
        });
        let CommandResult::SurfaceRegistered {
            surface,
            registered,
        } = result
        else {
            panic!("注册 surface 命令必须返回 SurfaceRegistered");
        };
        let record = state
            .surfaces
            .get(surface)
            .expect("新注册 surface 必须存在记录");

        // 自动分配路径必须明确报告注册成功。
        assert!(registered);

        // new surface 阶段尚未 map，因此不得提前绑定 WindowId。
        assert_eq!(record.window, None);
        assert_eq!(record.role, SurfaceRole::Unknown);
        assert!(record.alive);
    }

    /// 验证注册 surface 命令可以接受 backend 提供的显式 ID。
    #[test]
    fn command_register_surface_accepts_explicit_id() {
        let mut state = State::new();

        let result = state.handle_command(CoreCommand::RegisterSurface {
            surface: Some(42),
            client: None,
            role: SurfaceRole::XdgToplevel,
        });

        // 命令结果必须返回同一个显式 ID，并报告实际完成注册。
        assert_eq!(
            result,
            CommandResult::SurfaceRegistered {
                surface: 42,
                registered: true,
            }
        );
        assert!(state.surfaces.get(42).is_some());
    }

    /// 验证重复显式 SurfaceId 会返回失败且不覆盖已有记录。
    #[test]
    fn command_register_surface_rejects_duplicate_explicit_id() {
        let mut state = State::new();

        let first = state.handle_command(CoreCommand::RegisterSurface {
            surface: Some(42),
            client: None,
            role: SurfaceRole::XdgToplevel,
        });
        let second = state.handle_command(CoreCommand::RegisterSurface {
            surface: Some(42),
            client: None,
            role: SurfaceRole::XdgPopup,
        });

        // 首次注册成功，重复 ID 必须明确报告 registered=false。
        assert_eq!(
            first,
            CommandResult::SurfaceRegistered {
                surface: 42,
                registered: true,
            }
        );
        assert_eq!(
            second,
            CommandResult::SurfaceRegistered {
                surface: 42,
                registered: false,
            }
        );

        // 重复注册不得修改首次写入的协议角色。
        assert_eq!(
            state.surfaces.get(42).expect("原 surface 必须保留").role,
            SurfaceRole::XdgToplevel
        );
    }

    /// 验证注册 surface 命令可以同时记录 client 归属。
    #[test]
    fn command_register_surface_can_attach_client() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);

        let result = state.handle_command(CoreCommand::RegisterSurface {
            surface: Some(42),
            client: Some(client),
            role: SurfaceRole::XdgToplevel,
        });

        // surface 注册结果必须成功，并保留命令提供的 ClientId。
        assert_eq!(
            result,
            CommandResult::SurfaceRegistered {
                surface: 42,
                registered: true,
            }
        );
        assert_eq!(state.surfaces.client_for_surface(42), Some(client));

        // 归属关系本身不得提前创建逻辑窗口。
        assert_eq!(state.surfaces.window_for_surface(42), None);
    }

    /// 验证绑定命令可以关联已有 surface 与已有 client。
    #[test]
    fn command_bind_surface_to_client_links_existing_records() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        let surface = state.register_surface(SurfaceRole::Unknown);

        let result = state.handle_command(CoreCommand::BindSurfaceToClient { surface, client });

        // 两侧记录存在时必须建立归属，并返回明确成功结果。
        assert_eq!(
            result,
            CommandResult::SurfaceBoundToClient {
                surface,
                client,
                bound: true,
            }
        );
        assert_eq!(state.surfaces.client_for_surface(surface), Some(client));
    }

    /// 验证绑定命令会拒绝不存在的 client。
    #[test]
    fn command_bind_surface_to_client_rejects_missing_client() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::Unknown);

        let result = state.handle_command(CoreCommand::BindSurfaceToClient {
            surface,
            client: 999,
        });

        // 公开命令入口不得建立指向缺失 client 的归属关系。
        assert_eq!(
            result,
            CommandResult::SurfaceBoundToClient {
                surface,
                client: 999,
                bound: false,
            }
        );
        assert_eq!(state.surfaces.client_for_surface(surface), None);
    }

    /// 验证 surface 绑定命令会关联已有 surface 与已有窗口。
    #[test]
    fn command_bind_surface_to_window_links_existing_records() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);
        let window = state.compositor.focus.window.expect("默认状态必须包含窗口");

        let result = state.handle_command(CoreCommand::BindSurfaceToWindow { surface, window });

        // 已存在的两个记录必须成功建立绑定。
        assert_eq!(
            result,
            CommandResult::SurfaceBound {
                surface,
                window,
                bound: true,
            }
        );
        assert_eq!(state.surfaces.window_for_surface(surface), Some(window));
    }

    /// 验证已有 surface map 成窗口时不会创建重复 surface。
    #[test]
    fn command_register_window_for_surface_creates_window_without_duplicate_surface() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);
        let surface_count_before = state.surfaces.records().len();

        let result = state.handle_command(CoreCommand::RegisterWindowForSurface {
            surface,
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });
        let CommandResult::WindowRegisteredForSurface {
            surface: mapped_surface,
            window,
            bound,
        } = result
        else {
            panic!("surface map 命令必须返回 WindowRegisteredForSurface");
        };

        // map 必须复用原 SurfaceId 并成功绑定新窗口。
        assert_eq!(mapped_surface, surface);
        assert!(bound);
        assert_eq!(state.surfaces.window_for_surface(surface), Some(window));

        // 已有 surface 路径不得额外创建第二条占位记录。
        assert_eq!(state.surfaces.records().len(), surface_count_before);

        let record = state
            .registry
            .get(window)
            .expect("map 必须创建窗口 metadata");
        assert_eq!(record.title, "Terminal");
        assert_eq!(record.app_id.as_deref(), Some("foot"));
        assert_eq!(record.kind, WindowKind::WaylandPlaceholder);

        // 新窗口必须进入当前 workspace。
        assert!(
            state
                .compositor
                .current_workspace()
                .expect("默认当前 workspace 必须存在")
                .window_ids()
                .contains(&window)
        );
    }

    /// 验证 detach command 清理 window/link，但保持 surface alive。
    #[test]
    fn command_detach_toplevel_keeps_surface_alive() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);
        let mapped = state.handle_command(CoreCommand::RegisterWindowForSurface {
            surface,
            title: "Detached".to_string(),
            app_id: None,
            kind: WindowKind::WaylandPlaceholder,
        });
        let CommandResult::WindowRegisteredForSurface { window, .. } = mapped else {
            panic!("测试 surface 必须成功 map 成窗口");
        };

        let result = state.handle_command(CoreCommand::DetachWindowFromSurface { surface, window });
        let CommandResult::ToplevelDetached {
            surface: detached_surface,
            window: detached_window,
            result: Ok(detached),
        } = result
        else {
            panic!("detach command 必须返回成功的 ToplevelDetached");
        };

        assert_eq!(detached_surface, surface);
        assert_eq!(detached_window, window);
        assert!(detached.removed_from_workspace);
        assert!(detached.marked_window_dead);
        assert!(state.surfaces.is_alive(surface));
        assert_eq!(state.surfaces.window_for_surface(surface), None);
        assert!(!state.registry.is_alive(window));
        assert!(state.validate().is_clean());
    }

    /// 验证关闭 surface 命令会结束 surface 并同步关闭绑定窗口。
    #[test]
    fn command_close_surface_marks_surface_dead_and_closes_bound_window() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);
        let mapped = state.handle_command(CoreCommand::RegisterWindowForSurface {
            surface,
            title: "Terminal".to_string(),
            app_id: Some("foot".to_string()),
            kind: WindowKind::WaylandPlaceholder,
        });
        let CommandResult::WindowRegisteredForSurface { window, .. } = mapped else {
            panic!("测试 surface 必须成功 map 成窗口");
        };

        let result = state.handle_command(CoreCommand::CloseSurface(surface));
        let CommandResult::SurfaceClosed {
            surface: closed_surface,
            surface_marked_dead,
            closed_window,
            removed_from_workspace,
            marked_window_dead,
        } = result
        else {
            panic!("关闭 surface 命令必须返回 SurfaceClosed");
        };

        // 返回结果必须完整描述 surface 和窗口两侧生命周期变化。
        assert_eq!(closed_surface, surface);
        assert!(surface_marked_dead);
        assert_eq!(closed_window, Some(window));
        assert!(removed_from_workspace);
        assert!(marked_window_dead);

        // surface 和窗口都必须标记为 dead，workspace 不得继续引用窗口。
        assert!(!state.surfaces.is_alive(surface));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );
    }
}
