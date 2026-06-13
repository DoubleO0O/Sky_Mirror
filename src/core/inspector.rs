//! compositor 内部状态的只读调试快照。
//!
//! Inspector 只负责把 State、Workspace、FocusState、OutputState、ClientRegistry
//! 与 WindowRegistry 的数据合并成独立快照，不修改任何运行状态。它不属于
//! renderer、layout 或 session：renderer 只消费绘制命令，layout 只计算几何，
//! session 只负责持久化。

use std::collections::HashMap;

use crate::core::{
    client::{ClientId, ClientKind},
    focus::FocusState,
    layout::OutputSize,
    state::State,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::{LayoutMode, SlotContent, WindowId},
};

/// 调试快照中 slot 的内容类型。
///
/// 该类型只用于描述当前状态，不参与真实布局或渲染。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotDebugKind {
    /// 空 slot。
    Empty,

    /// 只包含单个窗口的 slot。
    Single,

    /// 包含多个窗口的 stack slot。
    Stack,
}

/// 单个 slot 的调试信息。
///
/// SlotDebugInfo 是 SlotContent 的只读快照，用于检查固定 slot 模型是否正确。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotDebugInfo {
    /// slot 的固定 ID，当前合法范围为 0..=3。
    pub id: u8,

    /// 当前 slot 的内容类型。
    pub kind: SlotDebugKind,

    /// 当前 slot 内所有窗口 ID。
    ///
    /// Empty 时为空；Single 时包含一个窗口；Stack 时包含 stack 中所有窗口。
    pub windows: Vec<WindowId>,

    /// 当前 slot 对外可见的 active window。
    pub active_window: Option<WindowId>,

    /// 如果该 slot 是 Stack，则记录 active 索引。
    ///
    /// Empty 和 Single 使用 None。
    pub active_index: Option<usize>,
}

/// 单个 workspace 的调试信息。
///
/// 该结构用于查看 workspace 的 layout、slot 内容和焦点状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDebugInfo {
    /// workspace 的稳定 ID。
    pub id: u32,

    /// workspace 当前布局模式。
    pub layout: LayoutMode,

    /// 当前 workspace 是否是活动 workspace。
    pub active: bool,

    /// 当前 workspace 是否包含焦点窗口。
    pub contains_focus: bool,

    /// workspace 中 4 个固定 slot 的调试信息。
    pub slots: Vec<SlotDebugInfo>,
}

/// 单个窗口的调试信息。
///
/// 该结构把 WindowRegistry metadata 与 workspace 引用关系合并到一起。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowDebugInfo {
    /// 稳定逻辑窗口 ID。
    pub id: WindowId,

    /// 窗口标题。
    pub title: String,

    /// 应用 ID。
    pub app_id: Option<String>,

    /// 窗口来源类型。
    pub kind: WindowKind,

    /// registry 中记录的生命周期状态。
    pub alive: bool,

    /// 该窗口是否是当前焦点窗口。
    pub focused: bool,

    /// 该窗口当前是否仍被某个 workspace 引用。
    ///
    /// 已关闭窗口可能仍在 registry 中保留 metadata，但不再可见。
    pub referenced_by_workspace: bool,

    /// 窗口所在 workspace。
    ///
    /// 如果 registry 中存在 metadata，但 workspace 已不再引用它，则为 None。
    pub workspace: Option<u32>,

    /// 窗口所在 slot。
    pub slot: Option<u8>,

    /// 窗口在 stack 中的位置。
    ///
    /// Single 或未被引用时为 None。
    pub stack_index: Option<usize>,

    /// 窗口是否是所在 slot 当前对外可见的 active window。
    ///
    /// Single 窗口始终为 true；Stack 只有 active window 为 true。
    pub active_in_slot: bool,
}

/// 单个 client 的调试信息。
///
/// 该结构只复制 `ClientRegistry` 的纯数据字段，不保存真实 Wayland client。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDebugInfo {
    /// client ID。
    pub id: ClientId,

    /// client 来源类型。
    pub kind: ClientKind,

    /// client 是否仍然存活。
    pub alive: bool,

    /// 可选调试名称。
    pub name: Option<String>,
}

/// 单个 surface 的调试信息。
///
/// 该结构是 SurfaceRegistry 记录的独立只读副本，不持有真实 surface 或 registry 引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceDebugInfo {
    /// 稳定 surface ID。
    pub id: SurfaceId,

    /// 该 surface 所属的 client。
    ///
    /// None 表示尚未记录归属；ClientId 与 SurfaceId、WindowId 属于不同层级。
    pub client: Option<ClientId>,

    /// 该 surface 当前绑定到的逻辑窗口。
    pub window: Option<WindowId>,

    /// surface 的协议角色。
    pub role: SurfaceRole,

    /// surface 是否仍然存活。
    pub alive: bool,
}

/// compositor 当前状态的完整调试快照。
///
/// 该快照是只读数据，不持有 State、WindowRegistry 或 Workspace 的引用，
/// 因此可以安全打印、测试或未来通过 IPC 暴露。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemDebugSnapshot {
    /// 当前活动 workspace ID。
    pub current_workspace: u32,

    /// 当前焦点状态。
    pub focus: FocusState,

    /// 当前输出尺寸。
    pub output_size: OutputSize,

    /// 所有 workspace 的调试快照。
    pub workspaces: Vec<WorkspaceDebugInfo>,

    /// 所有 registry 窗口的调试快照。
    pub windows: Vec<WindowDebugInfo>,

    /// 所有 client 占位记录的调试快照。
    pub clients: Vec<ClientDebugInfo>,

    /// 所有 surface 占位记录的调试快照。
    pub surfaces: Vec<SurfaceDebugInfo>,
}

impl SystemDebugSnapshot {
    /// 将调试快照格式化为人类可读的多行文本。
    ///
    /// 该方法只读取快照自身，不访问 State，也不修改 compositor 状态。
    /// 输出主要用于测试、日志或未来 IPC debug 命令。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        // 头部集中展示当前 workspace、焦点和输出尺寸，便于快速确认全局状态。
        output.push_str("Sky Mirror Debug Snapshot\n");
        output.push_str(&format!("current_workspace: {}\n", self.current_workspace));
        output.push_str(&format!(
            "focus: workspace={} slot={} window={:?}\n",
            self.focus.workspace, self.focus.slot, self.focus.window
        ));
        output.push_str(&format!(
            "output: {}x{}\n\n",
            self.output_size.width, self.output_size.height
        ));

        // Workspace 和 slot 按快照中的稳定顺序输出，并保留 stack 的 active 索引。
        output.push_str("Workspaces:\n");
        for workspace in &self.workspaces {
            output.push_str(&format!(
                "- workspace {} active={} contains_focus={} layout={:?}\n",
                workspace.id, workspace.active, workspace.contains_focus, workspace.layout
            ));

            for slot in &workspace.slots {
                output.push_str(&format!(
                    "  slot {} {:?} active={:?} active_index={:?} windows={:?}\n",
                    slot.id, slot.kind, slot.active_window, slot.active_index, slot.windows
                ));
            }
        }

        // Window metadata 同样保持 registry 快照顺序，完整展示生命周期和位置关系。
        output.push_str("\nWindows:\n");
        for window in &self.windows {
            output.push_str(&format!(
                "- window {} title={:?} app_id={:?} kind={:?} alive={} focused={} referenced={} workspace={:?} slot={:?} stack_index={:?} active_in_slot={}\n",
                window.id,
                window.title,
                window.app_id,
                window.kind,
                window.alive,
                window.focused,
                window.referenced_by_workspace,
                window.workspace,
                window.slot,
                window.stack_index,
                window.active_in_slot
            ));
        }

        // Client 占位记录按 ClientRegistry 顺序输出，不推导不存在的 surface 关系。
        output.push_str("\nClients:\n");
        for client in &self.clients {
            output.push_str(&format!(
                "- client {} kind={:?} alive={} name={:?}\n",
                client.id, client.kind, client.alive, client.name
            ));
        }

        // Surface 占位记录按 SurfaceRegistry 顺序输出，展示角色、生命周期和窗口绑定。
        output.push_str("\nSurfaces:\n");
        for surface in &self.surfaces {
            output.push_str(&format!(
                "- surface {} role={:?} alive={} client={:?} window={:?}\n",
                surface.id, surface.role, surface.alive, surface.client, surface.window
            ));
        }

        output
    }
}

/// 窗口在 workspace 固定结构中的位置。
///
/// 该私有结构只在构建快照期间使用，用于把 registry metadata 与 slot 引用关联起来。
#[derive(Debug, Clone)]
struct WindowLocation {
    /// 窗口所在 workspace ID。
    workspace: u32,

    /// 窗口所在 slot ID。
    slot: u8,

    /// 窗口在 stack 中的位置；Single 使用 None。
    stack_index: Option<usize>,

    /// 窗口是否是该 slot 当前 active window。
    active_in_slot: bool,
}

/// 只读状态检查器。
///
/// Inspector 不修改 State，不参与渲染，也不参与 session save/load。
/// 它只把 State、CompositorState、Workspace 和 WindowRegistry 中的状态整理成调试快照。
pub struct Inspector;

impl Inspector {
    /// 生成当前系统状态的调试快照。
    ///
    /// 该方法只读取 State，不修改 workspace、focus、registry 或 output。
    pub fn snapshot(state: &State) -> SystemDebugSnapshot {
        let focused_window = state.compositor.focus.window;
        let mut locations = HashMap::<WindowId, WindowLocation>::new();

        // 先遍历 workspace，生成 slot 快照并建立 WindowId 到位置的关联。
        let workspaces = state
            .compositor
            .workspaces
            .iter()
            .map(|workspace| {
                let mut contains_focus = false;
                let slots = workspace
                    .slots
                    .iter()
                    .map(|slot| match &slot.content {
                        // Empty 不产生窗口引用或 active 状态。
                        SlotContent::Empty => SlotDebugInfo {
                            id: slot.id,
                            kind: SlotDebugKind::Empty,
                            windows: Vec::new(),
                            active_window: None,
                            active_index: None,
                        },
                        // Single 同时是该 slot 唯一窗口和 active window。
                        SlotContent::Single(window) => {
                            if focused_window == Some(*window) {
                                contains_focus = true;
                            }

                            // 异常重复引用时保留首次发现的位置，便于快照稳定复现问题。
                            locations.entry(*window).or_insert(WindowLocation {
                                workspace: workspace.id,
                                slot: slot.id,
                                stack_index: None,
                                active_in_slot: true,
                            });

                            SlotDebugInfo {
                                id: slot.id,
                                kind: SlotDebugKind::Single,
                                windows: vec![*window],
                                active_window: Some(*window),
                                active_index: None,
                            }
                        }
                        // Stack 快照保留全部窗口、原始 active 索引和安全解析后的 active 窗口。
                        SlotContent::Stack(stack) => {
                            let active_window = stack.active_window();

                            for (index, window) in stack.windows.iter().copied().enumerate() {
                                if focused_window == Some(window) {
                                    contains_focus = true;
                                }

                                locations.entry(window).or_insert(WindowLocation {
                                    workspace: workspace.id,
                                    slot: slot.id,
                                    stack_index: Some(index),
                                    active_in_slot: active_window == Some(window),
                                });
                            }

                            SlotDebugInfo {
                                id: slot.id,
                                kind: SlotDebugKind::Stack,
                                windows: stack.windows.clone(),
                                active_window,
                                active_index: Some(stack.active),
                            }
                        }
                    })
                    .collect();

                WorkspaceDebugInfo {
                    id: workspace.id,
                    layout: workspace.layout,
                    active: workspace.id == state.compositor.current_workspace,
                    contains_focus,
                    slots,
                }
            })
            .collect();

        // Registry 是 metadata 的权威来源；location map 只补充 workspace 引用关系。
        let windows = state
            .registry
            .records()
            .iter()
            .map(|record| {
                let location = locations.get(&record.id);

                WindowDebugInfo {
                    id: record.id,
                    title: record.title.clone(),
                    app_id: record.app_id.clone(),
                    kind: record.kind.clone(),
                    alive: record.alive,
                    focused: focused_window == Some(record.id),
                    referenced_by_workspace: location.is_some(),
                    workspace: location.map(|location| location.workspace),
                    slot: location.map(|location| location.slot),
                    stack_index: location.and_then(|location| location.stack_index),
                    active_in_slot: location.is_some_and(|location| location.active_in_slot),
                }
            })
            .collect();

        // ClientRegistry 是 client 生命周期 metadata 的权威来源，快照只复制字段。
        let clients = state
            .clients
            .records()
            .iter()
            .map(|client| ClientDebugInfo {
                id: client.id,
                kind: client.kind,
                alive: client.alive,
                name: client.name.clone(),
            })
            .collect();

        // SurfaceRegistry 是 surface 占位绑定的权威来源，快照只复制纯数据字段。
        let surfaces = state
            .surfaces
            .records()
            .iter()
            .map(|surface| SurfaceDebugInfo {
                id: surface.id,
                client: surface.client,
                window: surface.window,
                role: surface.role,
                alive: surface.alive,
            })
            .collect();

        SystemDebugSnapshot {
            current_workspace: state.compositor.current_workspace,
            focus: state.compositor.focus,
            output_size: state.compositor.current_output_size(),
            workspaces,
            windows,
            clients,
            surfaces,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Inspector;
    use crate::core::{client::ClientKind, state::State, surface::SurfaceRole};

    /// 验证 Inspector 快照包含全部 workspace 及固定四 slot 结构。
    #[test]
    fn inspector_snapshot_includes_workspace_slots() {
        let state = State::new();

        let snapshot = Inspector::snapshot(&state);

        // 快照活动 workspace 必须与当前 compositor 状态一致。
        assert_eq!(
            snapshot.current_workspace,
            state.compositor.current_workspace
        );

        // 快照必须覆盖 compositor 管理的全部 workspace。
        assert_eq!(snapshot.workspaces.len(), state.compositor.workspaces.len());

        let current = snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.id == snapshot.current_workspace)
            .expect("快照必须包含当前 workspace");

        // 固定 slot 模型在调试快照中也必须保持四个 slot。
        assert_eq!(current.slots.len(), 4);
    }

    /// 验证 Inspector 会把 registry metadata 与 workspace 位置合并。
    #[test]
    fn inspector_snapshot_merges_registry_metadata_with_workspace_location() {
        let state = State::new();

        let snapshot = state.debug_snapshot();
        let window = snapshot.windows.first().expect("默认状态必须包含窗口");

        // 窗口标题必须来自 WindowRegistry 的 mock metadata。
        assert!(window.title.starts_with("Mock Window "));

        // 默认窗口必须存活且仍被 workspace 引用。
        assert!(window.alive);
        assert!(window.referenced_by_workspace);

        // 默认窗口创建在 workspace 0，并且必须具有明确 slot。
        assert_eq!(window.workspace, Some(0));
        assert!(window.slot.is_some());

        // focused 标记必须与当前 FocusState.window 保持一致。
        assert_eq!(
            window.focused,
            state.compositor.focus.window == Some(window.id)
        );
    }

    /// 验证关闭窗口后 Inspector 仍保留 dead metadata，但不再报告 workspace 引用。
    #[test]
    fn inspector_snapshot_keeps_dead_unreferenced_window_metadata() {
        let mut state = State::new();
        let closed = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");

        // 通过 State 关闭窗口，确保 workspace 引用和 registry 生命周期同步更新。
        assert!(state.close_focused_window());

        let snapshot = state.debug_snapshot();
        let window = snapshot
            .windows
            .iter()
            .find(|window| window.id == closed)
            .expect("关闭窗口的 metadata 必须继续保留");

        // registry 中的关闭窗口必须标记为 dead。
        assert!(!window.alive);

        // workspace 删除引用后，位置字段必须全部清空。
        assert!(!window.referenced_by_workspace);
        assert_eq!(window.workspace, None);
        assert_eq!(window.slot, None);

        // 已关闭窗口不得继续被报告为当前焦点。
        assert!(!window.focused);
    }

    /// 验证 Inspector 快照会包含 SurfaceRegistry 中的占位记录。
    #[test]
    fn inspector_snapshot_includes_surface_records() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::Unknown);

        let snapshot = state.debug_snapshot();
        let record = snapshot
            .surfaces
            .iter()
            .find(|record| record.id == surface)
            .expect("快照必须包含已注册 surface");

        // 未 map surface 必须保留角色、存活状态和空窗口绑定。
        assert_eq!(record.client, None);
        assert_eq!(record.window, None);
        assert_eq!(record.role, SurfaceRole::Unknown);
        assert!(record.alive);
    }

    /// 验证 Inspector 快照会保留 surface 的 client owner。
    #[test]
    fn inspector_snapshot_includes_surface_client_owner() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        let surface = state.register_surface_for_client(Some(client), SurfaceRole::XdgToplevel);

        let snapshot = state.debug_snapshot();
        let record = snapshot
            .surfaces
            .iter()
            .find(|record| record.id == surface)
            .expect("快照必须包含带归属的 surface");

        // 快照必须保留 ClientId，同时保持尚未 map 的空 WindowId 状态。
        assert_eq!(record.client, Some(client));
        assert_eq!(record.window, None);

        let text = snapshot.pretty_print();

        // 人类可读输出必须同时展示 surface ID 和 client owner。
        assert!(text.contains(&format!(
            "- surface {} role=XdgToplevel alive=true client=Some({}) window=None",
            surface, client
        )));
    }

    /// 验证 Inspector 快照会包含 ClientRegistry 中的 client 占位记录。
    #[test]
    fn inspector_snapshot_includes_client_records() {
        let mut state = State::new();
        let client = state.register_client(
            ClientKind::WaylandPlaceholder,
            Some("alacritty".to_string()),
        );

        let snapshot = state.debug_snapshot();
        let record = snapshot
            .clients
            .iter()
            .find(|record| record.id == client)
            .expect("快照必须包含已注册 client");

        // 快照必须完整保留 ClientRegistry 中的来源、名称和生命周期状态。
        assert_eq!(record.kind, ClientKind::WaylandPlaceholder);
        assert_eq!(record.name.as_deref(), Some("alacritty"));
        assert!(record.alive);
    }

    /// 验证 client 断开级联后快照保留三层 dead metadata 与历史绑定。
    #[test]
    fn inspector_snapshot_shows_client_surface_window_after_disconnect_cascade() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        let surface = state.register_surface_for_client(Some(client), SurfaceRole::XdgToplevel);
        let mapped = state.register_window_for_surface(
            surface,
            "Terminal",
            Some("foot".to_string()),
            crate::core::window::WindowKind::WaylandPlaceholder,
        );
        assert!(mapped.bound);

        let cascade = state.close_client(client);
        assert!(cascade.marked_dead);

        let snapshot = state.debug_snapshot();
        let client_record = snapshot
            .clients
            .iter()
            .find(|record| record.id == client)
            .expect("快照必须保留 dead client");
        let surface_record = snapshot
            .surfaces
            .iter()
            .find(|record| record.id == surface)
            .expect("快照必须保留 dead surface");
        let window_record = snapshot
            .windows
            .iter()
            .find(|record| record.id == mapped.window)
            .expect("快照必须保留 dead window");

        // ClientRegistry 记录保留，但生命周期必须结束。
        assert!(!client_record.alive);

        // Surface 记录保留历史 owner 和窗口绑定，同时标记为 dead。
        assert!(!surface_record.alive);
        assert_eq!(surface_record.client, Some(client));
        assert_eq!(surface_record.window, Some(mapped.window));

        // Window metadata 保留，但不得继续被 workspace 引用。
        assert!(!window_record.alive);
        assert!(!window_record.referenced_by_workspace);
    }

    /// 验证 pretty print 包含识别快照所需的核心区段和全局状态。
    #[test]
    fn pretty_print_contains_core_sections() {
        let state = State::new();

        let text = state.debug_snapshot().pretty_print();

        // 标题必须明确标识这是 Sky Mirror 调试快照。
        assert!(text.contains("Sky Mirror Debug Snapshot"));

        // 默认活动 workspace 必须显示为 0。
        assert!(text.contains("current_workspace: 0"));

        // 焦点摘要必须包含默认 workspace。
        assert!(text.contains("focus: workspace=0"));

        // 默认虚拟输出尺寸必须完整显示。
        assert!(text.contains("output: 1920x1080"));

        // Workspace 与 Window 两个主体区段必须同时存在。
        assert!(text.contains("Workspaces:"));
        assert!(text.contains("Windows:"));

        // Client 占位模型必须有独立区段，即使默认状态暂时没有 client。
        assert!(text.contains("Clients:"));

        // Surface 占位绑定必须有独立区段，即使默认状态暂时没有 surface。
        assert!(text.contains("Surfaces:"));
    }

    /// 验证 pretty print 会输出 workspace 和固定 slot 的关键状态。
    #[test]
    fn pretty_print_includes_workspace_and_slot_lines() {
        let state = State::new();

        let text = state.debug_snapshot().pretty_print();

        // 默认 workspace 0 必须有独立摘要行。
        assert!(text.contains("- workspace 0"));

        // 固定 slot 0 必须出现在 workspace 详情中。
        assert!(text.contains("slot 0"));

        // slot 行必须同时包含 active window 和完整窗口列表。
        assert!(text.contains("active="));
        assert!(text.contains("windows="));
    }

    /// 验证 pretty print 会展示窗口 metadata、生命周期和引用状态。
    #[test]
    fn pretty_print_includes_window_metadata() {
        let state = State::new();

        let text = state.debug_snapshot().pretty_print();

        // 默认窗口标题和 app_id 必须来自 WindowRegistry metadata。
        assert!(text.contains("Mock Window 1"));
        assert!(text.contains("sky-mirror.mock"));

        // 默认焦点窗口必须显示为存活、聚焦且仍被 workspace 引用。
        assert!(text.contains("alive=true"));
        assert!(text.contains("focused=true"));
        assert!(text.contains("referenced=true"));
    }

    /// 验证 pretty print 会把已关闭窗口显示为 dead 且不再被 workspace 引用。
    #[test]
    fn pretty_print_shows_closed_window_as_dead_and_unreferenced() {
        let mut state = State::new();
        let closed = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");

        // 通过 State 关闭窗口，确保 workspace 和 registry 两侧状态同步更新。
        assert!(state.close_focused_window());

        let text = state.debug_snapshot().pretty_print();
        let closed_prefix = format!("- window {}", closed);
        let closed_line = text
            .lines()
            .find(|line| line.starts_with(&closed_prefix))
            .expect("pretty print 必须保留已关闭窗口 metadata");

        // 目标窗口的输出行必须明确显示生命周期已结束。
        assert!(closed_line.contains("alive=false"));

        // 目标窗口从 workspace 删除后必须显示为未引用。
        assert!(closed_line.contains("referenced=false"));
    }
}
