//! 单个后端事件进入核心状态的运行时桥接层。
//!
//! 本模块为未来真实 Smithay 或 backend 回调提供单事件入口。它不持有状态，
//! 不引入 Wayland 类型，也不保存真实 surface 对象，只组合现有纯数据事件、
//! 命令、执行结果和验证报告。

use crate::core::{
    backend_event::{BackendEvent, BackendEventTranslator},
    command::{CommandResult, CoreCommand},
    state::State,
    validator::ValidationReport,
};

/// 单个后端事件处理后的完整结果。
///
/// `RuntimeEventResult` 是运行时版本的单步报告：它保存原始 `BackendEvent`、
/// 翻译出的 `CoreCommand`、命令执行结果，以及执行后立即生成的
/// `ValidationReport`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEventResult {
    /// 原始后端事件。
    pub event: BackendEvent,

    /// 翻译后的核心命令。
    pub command: CoreCommand,

    /// `State` 执行核心命令后的结果。
    pub result: CommandResult,

    /// 命令执行后的状态验证报告。
    pub validation: ValidationReport,
}

/// 单事件运行时桥接器。
///
/// `CoreRuntimeBridge` 是未来真实 Smithay 或 backend 回调进入核心状态的薄边界。
/// 它与 `BackendEventReplayer` 的区别是：回放器用于测试中一次处理多条事件，
/// 运行时桥接器用于每次处理一条后端事件。
///
/// 本阶段仍然只处理纯数据 `BackendEvent`，不持有真实 Smithay 或 Wayland 对象。
pub struct CoreRuntimeBridge;

impl CoreRuntimeBridge {
    /// 处理单个后端事件。
    ///
    /// 处理流程固定为：
    /// `BackendEvent -> BackendEventTranslator -> CoreCommand
    /// -> State::handle_command_with_validation()`。
    ///
    /// 该方法不会自动打印、不会自动修复状态，也不会保存 `State` 引用。
    pub fn handle_backend_event(state: &mut State, event: BackendEvent) -> RuntimeEventResult {
        let command = BackendEventTranslator::translate(event.clone());

        // State 统一保证 validation 在命令执行后生成；bridge 只附加 event/command 上下文。
        let (result, validation) = state.handle_command_with_validation(command.clone());

        RuntimeEventResult {
            event,
            command,
            result,
            validation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CoreRuntimeBridge;
    use crate::core::{
        action::Action,
        backend_event::BackendEvent,
        client::ClientKind,
        command::{CommandResult, CoreCommand},
        state::State,
        surface::SurfaceRole,
        validator::ValidationIssueKind,
        window::WindowKind,
    };

    /// 验证单个输出尺寸事件会通过现有 Action 链路更新集中状态。
    #[test]
    fn runtime_bridge_handles_output_resize_event() {
        let mut state = State::new();

        let result = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::OutputResized {
                width: 1600,
                height: 900,
            },
        );

        // 合法尺寸变化执行后必须保持核心状态有效。
        assert!(result.validation.is_valid());

        let output = state.compositor.current_output_size();

        // 运行时状态必须保留事件提供的输出宽高。
        assert_eq!(output.width, 1600);
        assert_eq!(output.height, 900);

        // 桥接器必须复用现有 ResizeOutput Action，不能直接修改 output。
        assert_eq!(
            result.command,
            CoreCommand::Action(Action::ResizeOutput {
                width: 1600,
                height: 900,
            })
        );
    }

    /// 验证 surface 创建、map 和关闭可以逐条通过运行时桥接器完成。
    #[test]
    fn runtime_bridge_handles_surface_lifecycle_step_by_step() {
        let mut state = State::new();

        let created = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceCreated {
                surface: 42,
                client: None,
                role: SurfaceRole::XdgToplevel,
            },
        );

        // 创建步骤必须注册外部指定 ID，并保持状态有效。
        assert!(created.validation.is_valid());
        assert_eq!(
            created.result,
            CommandResult::SurfaceRegistered {
                surface: 42,
                registered: true,
            }
        );

        let mapped = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            },
        );

        // map 步骤必须创建窗口并绑定已有 surface。
        assert!(mapped.validation.is_valid());
        let CommandResult::WindowRegisteredForSurface {
            surface,
            window,
            bound,
        } = &mapped.result
        else {
            panic!("map 步骤必须返回 WindowRegisteredForSurface");
        };
        assert_eq!(*surface, 42);
        assert!(*bound);
        let mapped_window = *window;

        let closed = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceClosed { surface: 42 },
        );

        // 关闭步骤必须同步结束 surface 和绑定窗口的生命周期。
        assert!(closed.validation.is_valid());
        let CommandResult::SurfaceClosed {
            surface,
            surface_marked_dead,
            closed_window,
            marked_window_dead,
            ..
        } = closed.result
        else {
            panic!("关闭步骤必须返回 SurfaceClosed");
        };
        assert_eq!(surface, 42);
        assert!(surface_marked_dead);
        assert_eq!(closed_window, Some(mapped_window));
        assert!(marked_window_dead);

        // 生命周期结束后诊断记录仍保留，但两侧都必须标记为 dead。
        assert!(!state.surfaces.get(42).expect("surface 记录必须保留").alive);
        assert!(!state.registry.is_alive(mapped_window));
    }

    /// 验证 client connection 通过 runtime public seam 自动分配 ID 并进入诊断快照。
    #[test]
    fn runtime_bridge_client_connected_auto_allocates_id_and_updates_diagnostics() {
        let mut state = State::new();

        let connected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("terminal".to_string()),
            },
        );

        // BackendEvent 只表达外部事实；bridge 必须先翻译为命令，不能直接修改 State。
        assert_eq!(
            connected.command,
            CoreCommand::RegisterClient {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("terminal".to_string()),
            }
        );

        let CommandResult::ClientRegistered { client, registered } = connected.result else {
            panic!("client connection 必须返回 ClientRegistered");
        };

        // generated ClientId 必须从 CommandResult 读取，未来 adapter 不能猜测 registry allocator。
        assert!(registered);
        let record = state
            .clients
            .get(client)
            .expect("自动分配的 client 必须存在 registry 记录");
        assert!(record.alive);
        assert_eq!(record.kind, ClientKind::WaylandPlaceholder);
        assert_eq!(record.name.as_deref(), Some("terminal"));

        // client socket connection 不等于 surface；alive client 暂无 surface 是合法中间状态。
        assert!(state.surfaces.records().is_empty());
        assert!(connected.validation.is_clean());

        let bundle = state.debug_bundle();
        let snapshot_record = bundle
            .snapshot
            .clients
            .iter()
            .find(|record| record.id == client)
            .expect("DebugBundle 必须包含自动注册的 client");

        // DebugBundle 必须同时反映 client metadata 与命令后的 clean validation。
        assert!(snapshot_record.alive);
        assert_eq!(snapshot_record.name.as_deref(), Some("terminal"));
        assert!(bundle.is_clean());
    }

    /// 验证 runtime public seam 接受 explicit ClientId，并推进后续自动分配边界。
    #[test]
    fn runtime_bridge_client_connected_accepts_explicit_id_without_allocator_conflict() {
        let mut state = State::new();

        let explicit = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("explicit-client".to_string()),
            },
        );

        // 翻译后的命令必须保留外部 ID 和 metadata，BackendEvent 本身不写 registry。
        assert_eq!(
            explicit.command,
            CoreCommand::RegisterClient {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("explicit-client".to_string()),
            }
        );
        assert_eq!(
            explicit.result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert!(state.clients.is_alive(42));
        assert!(explicit.validation.is_clean());

        let generated = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: None,
            },
        );
        let CommandResult::ClientRegistered {
            client: generated_client,
            registered,
        } = generated.result
        else {
            panic!("后续自动注册必须返回 ClientRegistered");
        };

        // 显式 ID 必须推进 allocator，后续 generated ClientId 不能覆盖现有 client 42。
        assert!(registered);
        assert_ne!(generated_client, 42);
        assert!(generated_client > 42);
        assert!(state.clients.is_alive(generated_client));
        assert!(generated.validation.is_clean());
    }

    /// 验证 runtime public seam 拒绝重复 explicit ClientId，且不覆盖原记录。
    #[test]
    fn runtime_bridge_client_connected_rejects_duplicate_explicit_id_cleanly() {
        let mut state = State::new();

        let first = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(42),
                kind: ClientKind::Mock,
                name: Some("original".to_string()),
            },
        );
        let duplicate = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("replacement".to_string()),
            },
        );

        assert_eq!(
            first.result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert_eq!(
            duplicate.result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: false,
            }
        );

        let record = state.clients.get(42).expect("首次 client 记录必须继续存在");

        // 重复外部 ID 不能覆盖首条 metadata，也不能制造第二条同 ID 记录。
        assert_eq!(record.kind, ClientKind::Mock);
        assert_eq!(record.name.as_deref(), Some("original"));
        assert_eq!(
            state
                .clients
                .records()
                .iter()
                .filter(|record| record.id == 42)
                .count(),
            1
        );

        // 重复连接是明确的 command result，不是结构损坏，validation 必须保持 clean。
        assert!(duplicate.validation.is_clean());
        assert!(state.debug_bundle().is_clean());
    }

    /// 验证没有 surface 的 client 断开后保留 tombstone，并保持状态 clean。
    #[test]
    fn runtime_bridge_client_disconnected_without_surfaces_keeps_clean_tombstone() {
        let mut state = State::new();

        let connected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("short-lived".to_string()),
            },
        );
        let CommandResult::ClientRegistered { client, registered } = connected.result else {
            panic!("client connection 必须返回 ClientRegistered");
        };
        assert!(registered);

        // alive client 尚未创建 surface 是合法中间状态，不应在断开前产生 validation issue。
        assert!(state.surfaces.records().is_empty());
        assert!(connected.validation.is_clean());

        let disconnected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientDisconnected { client },
        );

        assert_eq!(disconnected.command, CoreCommand::CloseClient(client));
        assert_eq!(
            disconnected.result,
            CommandResult::ClientClosed {
                client,
                marked_dead: true,
                dead_surfaces: Vec::new(),
                closed_windows: Vec::new(),
                removed_from_workspace_count: 0,
                marked_window_dead_count: 0,
            }
        );

        // 断开采用 tombstone 而不是物理删除，诊断层才能保留稳定 ClientId 与 metadata。
        let record = state
            .clients
            .get(client)
            .expect("断开后的 client tombstone 必须保留");
        assert!(!record.alive);
        assert!(!state.clients.is_alive(client));
        assert!(disconnected.validation.is_clean());

        let bundle = state.debug_bundle();
        let snapshot_record = bundle
            .snapshot
            .clients
            .iter()
            .find(|record| record.id == client)
            .expect("DebugBundle 必须保留 dead client tombstone");

        assert!(!snapshot_record.alive);
        assert_eq!(snapshot_record.name.as_deref(), Some("short-lived"));
        assert!(bundle.is_clean());
    }

    /// 验证 client 断开事件会通过运行时桥级联关闭 surface 和 window。
    #[test]
    fn runtime_bridge_client_disconnected_cascades_to_surface_and_window() {
        let mut state = State::new();

        let connected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(7),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("app".to_string()),
            },
        );
        assert!(connected.validation.is_valid());

        let created = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceCreated {
                surface: 42,
                client: Some(7),
                role: SurfaceRole::XdgToplevel,
            },
        );
        assert!(created.validation.is_valid());

        let mapped = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            },
        );
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped.result else {
            panic!("map 步骤必须返回 WindowRegisteredForSurface");
        };
        assert!(bound);

        let mapped_slot = state
            .compositor
            .current_workspace()
            .and_then(|workspace| {
                workspace
                    .slots
                    .iter()
                    .find(|slot| workspace.slot_window(slot.id) == Some(window))
                    .map(|slot| slot.id)
            })
            .expect("mapped window 必须属于当前 workspace 的一个 slot");
        let focused = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ActionRequested(Action::FocusSlot(mapped_slot)),
        );

        // 先经 public Action seam 聚焦目标窗口，才能封板 disconnect 后的 focus cleanup。
        assert_eq!(state.compositor.focus.window, Some(window));
        assert!(focused.validation.is_clean());

        let disconnected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientDisconnected { client: 7 },
        );
        assert_eq!(disconnected.command, CoreCommand::CloseClient(7));
        let CommandResult::ClientClosed {
            client,
            marked_dead,
            dead_surfaces,
            closed_windows,
            removed_from_workspace_count,
            marked_window_dead_count,
        } = disconnected.result
        else {
            panic!("client 断开必须返回 ClientClosed");
        };

        // 最后一步必须完整报告 client -> surface -> window 的纯数据级联。
        assert_eq!(client, 7);
        assert!(marked_dead);
        assert_eq!(dead_surfaces, vec![42]);
        assert_eq!(closed_windows, vec![window]);
        assert_eq!(removed_from_workspace_count, 1);
        assert_eq!(marked_window_dead_count, 1);

        // 级联后记录仍然存在，但三层生命周期都必须结束。
        assert!(!state.clients.is_alive(7));
        assert!(!state.surfaces.is_alive(42));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );

        // client disconnect 必须刷新焦点，不能让 live focus path 指向已关闭窗口。
        assert!(state.compositor.focus.slot < 4);
        assert_ne!(state.compositor.focus.window, Some(window));

        let bundle = state.debug_bundle();
        let client_snapshot = bundle
            .snapshot
            .clients
            .iter()
            .find(|record| record.id == 7)
            .expect("DebugBundle 必须保留 dead client");
        let surface_snapshot = bundle
            .snapshot
            .surfaces
            .iter()
            .find(|record| record.id == 42)
            .expect("DebugBundle 必须保留 dead surface");
        let window_snapshot = bundle
            .snapshot
            .windows
            .iter()
            .find(|record| record.id == window)
            .expect("DebugBundle 必须保留 dead window");

        // 级联采用 tombstone 保留历史 identity，同时从 workspace live path 清理窗口。
        assert!(!client_snapshot.alive);
        assert!(!surface_snapshot.alive);
        assert_eq!(surface_snapshot.client, Some(7));
        assert_eq!(surface_snapshot.window, Some(window));
        assert!(!window_snapshot.alive);
        assert!(!window_snapshot.referenced_by_workspace);

        // 运行时桥接完成级联后，状态必须重新满足全部不变量。
        assert!(disconnected.validation.is_clean());
        assert!(bundle.is_clean());
    }

    /// 验证桥接器会记录状态错误，但不会自动修复损坏的绑定。
    #[test]
    fn runtime_bridge_records_validation_issue_without_repairing_state() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);

        // 测试直接制造缺失窗口绑定，用于验证桥接器的只记录、不修复语义。
        assert!(state.surfaces.bind_window(surface, 999));

        let result =
            CoreRuntimeBridge::handle_backend_event(&mut state, BackendEvent::ValidateRequested);

        // 执行后验证必须报告无效状态及稳定问题类型。
        assert!(!result.validation.is_valid());
        assert!(
            result
                .validation
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesMissingWindow })
        );

        // 桥接器不得擅自清理或替换错误绑定。
        assert_eq!(
            state
                .surfaces
                .get(surface)
                .expect("损坏 surface 记录必须继续存在")
                .window,
            Some(999)
        );
    }

    /// 验证单步结果完整保存原始事件和翻译后的核心命令。
    #[test]
    fn runtime_bridge_result_stores_event_and_command() {
        let mut state = State::new();
        let event = BackendEvent::DebugRequested;

        let result = CoreRuntimeBridge::handle_backend_event(&mut state, event.clone());

        // 调用方必须能够从结果中追溯原始事实和实际执行命令。
        assert_eq!(result.event, event);
        assert_eq!(result.command, CoreCommand::DebugText);
    }
}
