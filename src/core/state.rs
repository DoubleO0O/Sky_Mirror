//! compositor 的集中式全局状态与唯一状态修改入口。
//!
//! `State` 是事件循环持有的根状态，包含 compositor-owned 状态和 WindowRegistry。
//! `CompositorState` 统一持有 backend、workspace、focus、output 与运行标志；
//! 输入必须先转换为 Action，再通过 `State::dispatch_action()` 修改这些状态。
//!
//! 从状态到绘制的路径保持只读：
//! Workspace -> LayoutEngine -> SceneBuilder -> RenderPlanner -> RenderFrame。
//! backend 不参与 session 序列化，OutputState 也由未来真实输出发现流程负责。

use std::{fs, io};

use crate::{
    backend::drm::DrmBackend,
    core::{
        action::Action,
        client::{ClientId, ClientKind, ClientRegistry},
        command::{CommandHandler, CommandResult, CoreCommand},
        diagnostics::DebugBundle,
        focus::FocusState,
        inspector::{Inspector, SystemDebugSnapshot},
        layout::{LayoutEngine, OutputSize, WindowPlacement},
        output::OutputState,
        render::{RenderFrame, RenderPlanner},
        scene::{SceneBuilder, SceneFrame},
        session::{SessionState, workspace_from_session, workspace_to_session},
        surface::{SurfaceId, SurfaceRegistry, SurfaceRole},
        validator::{StateValidator, ValidationReport},
        window::{WindowKind, WindowRegistry},
        workspace::{LayoutMode, WindowId, Workspace},
    },
};

/// compositor 自身拥有的集中状态。
///
/// 所有 workspace、focus、output 和 backend 生命周期都聚合在该结构中，
/// 避免多个模块分别持有可变副本而产生不一致。
pub struct CompositorState {
    /// 当前 backend 实例。
    ///
    /// backend 是运行时资源，不会进入 session JSON。
    pub backend: DrmBackend,
    /// compositor 管理的全部 workspace。
    pub workspaces: Vec<Workspace>,
    /// 当前活动 workspace 的稳定 ID。
    pub current_workspace: u32,
    /// 当前 workspace/slot/window 的显式焦点状态。
    pub focus: FocusState,
    /// 当前输出尺寸。
    ///
    /// 该状态影响布局和渲染帧，但不会写入 session。
    pub output: OutputState,
    /// 主事件循环是否继续运行。
    pub running: bool,
}

impl CompositorState {
    /// 创建默认 compositor 状态。
    ///
    /// 初始化三个固定 workspace、空焦点和 1920x1080 虚拟输出；
    /// backend 只被构造，尚未执行 `init()`。
    pub fn new() -> Self {
        Self {
            backend: DrmBackend::new(),
            workspaces: vec![Workspace::new(0), Workspace::new(1), Workspace::new(2)],
            current_workspace: 0,
            focus: FocusState::new(),
            output: OutputState::default_virtual(),
            running: false,
        }
    }

    /// 初始化 backend 并将 compositor 标记为运行状态。
    ///
    /// EventLoop 根据 `running` 控制 dispatch 循环，该方法是统一的启动边界。
    pub fn start(&mut self) {
        // backend 初始化必须发生在运行阶段，而不是纯状态构造阶段。
        self.backend.init();

        // 标记 compositor 已进入主循环。
        // TODO: 后续增加退出 Action 时，应通过统一状态方法把该字段设回 false。
        self.running = true;
    }

    /// 可变获取当前 workspace。
    ///
    /// 根据稳定 ID 查找而不是直接把 ID 当作 Vec 下标，因此 workspace 重排后仍安全。
    /// 当前 ID 无效时返回 None，不会 panic。
    pub fn current_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|workspace| workspace.id == self.current_workspace)
    }

    /// 只读获取当前 workspace。
    ///
    /// layout、scene 和 render frame 计算通过该入口读取状态，不会修改 workspace。
    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == self.current_workspace)
    }

    /// 返回当前 workspace 的稳定 ID。
    pub fn current_workspace_id(&self) -> u32 {
        self.current_workspace
    }

    /// 返回当前 OutputState 中的逻辑输出尺寸。
    pub fn current_output_size(&self) -> OutputSize {
        self.output.size()
    }

    /// 更新当前输出尺寸。
    ///
    /// 该操作只修改 OutputState，不切换 workspace、不改变 focus，也不触发 session restore。
    pub fn resize_output(&mut self, size: OutputSize) {
        self.output.resize(size);
    }

    /// 为当前 workspace 计算窗口 placement。
    ///
    /// 如果 current_workspace ID 无效，则返回空列表；计算过程不修改状态。
    pub fn current_layout(&self, output: OutputSize) -> Vec<WindowPlacement> {
        self.current_workspace()
            .map(|workspace| LayoutEngine::compute_workspace(workspace, output))
            .unwrap_or_default()
    }

    /// 为当前 workspace 构建 SceneFrame。
    ///
    /// LayoutEngine 提供可见窗口与几何位置，FocusState 提供 focused window，
    /// SceneBuilder 将二者合并并计算简化 z-index。
    pub fn current_scene(&self, output: OutputSize) -> SceneFrame {
        // placement 是当前 workspace 的只读布局快照。
        let placements = self.current_layout(output);

        SceneBuilder::build(self.current_workspace, self.focus.window, placements)
    }

    /// 使用明确输出尺寸构建 RenderFrame。
    ///
    /// 该方法保留给测试或未来多输出调用方；它不会读取或修改 OutputState。
    pub fn current_render_frame(&self, output: OutputSize) -> RenderFrame {
        // 先构建场景快照，再规划为 renderer 边界使用的命令列表。
        let scene = self.current_scene(output);

        RenderPlanner::from_scene(&scene)
    }

    /// 使用当前 OutputState 尺寸构建 RenderFrame。
    ///
    /// EventLoop 调用该方法，因此无需知道或硬编码具体输出大小。
    pub fn current_render_frame_for_current_output(&self) -> RenderFrame {
        self.current_render_frame(self.current_output_size())
    }

    /// 将窗口分配到当前 workspace，并在成功后刷新焦点。
    ///
    /// Workspace 自己维护固定 slot 与 stack 不变量；CompositorState 只负责在容器
    /// 变化后重新推导 focus.window，确保焦点指向 Single 或 Stack 的 active window。
    pub fn assign_window_to_current_workspace(&mut self, window: WindowId) -> bool {
        // 可变 workspace 借用限制在该表达式中，结束后才能再次可变借用 self 刷新焦点。
        let assigned = if let Some(workspace) = self.current_workspace_mut() {
            workspace.assign_window(window);
            true
        } else {
            false
        };

        // 只有实际完成分配后才刷新，避免无效 current_workspace 改写原焦点。
        if assigned {
            self.refresh_focus();
        }

        assigned
    }

    /// 关闭当前焦点窗口，并修复 workspace 容器与焦点状态。
    ///
    /// 输入来自 `FocusState.window`。删除只发生在当前 workspace 中：
    /// Workspace 负责维护 Empty、Single 与 Stack 的结构不变量，删除结束后本方法
    /// 调用 `refresh_focus()`，确保焦点重新指向当前 slot 的 active window 或下一个
    /// occupied slot。
    pub fn close_focused_window(&mut self) -> bool {
        // 先复制焦点 WindowId，避免后续可变借用 workspace 时同时借用 focus。
        let Some(window) = self.focus.window else {
            println!("[Compositor] No focused window to close");
            return false;
        };

        // 将 workspace 的可变借用限制在该表达式内，借用结束后才能刷新整个状态。
        let removed = self
            .current_workspace_mut()
            .map(|workspace| workspace.remove_window(window))
            .unwrap_or(false);

        if removed {
            println!("[Compositor] Closed focused window {}", window);

            // 删除可能使 slot 变空或使 Stack active window 改变，必须统一重新推导焦点。
            self.refresh_focus();
            true
        } else {
            println!("[Compositor] Focused window {window} not found in current workspace");

            // 焦点指向不存在的窗口说明状态可能不一致，即使删除失败也需要修复。
            self.refresh_focus();
            false
        }
    }

    /// 从所有 workspace 中移除指定窗口，并在必要时刷新焦点。
    ///
    /// 该方法用于外部 client unmap / destroy 场景：外部系统给出具体 WindowId，
    /// compositor 删除所有可见引用，但不会操作 WindowRegistry。
    pub fn remove_window(&mut self, window: WindowId) -> bool {
        let mut removed = false;
        let mut removed_from_current_workspace = false;

        // 遍历全部 workspace，确保异常重复引用也不会残留在其他工作区。
        for workspace in &mut self.workspaces {
            if workspace.remove_window(window) {
                removed = true;

                // 当前 workspace 内容变化后需要重新推导焦点窗口。
                if workspace.id == self.current_workspace {
                    removed_from_current_workspace = true;
                }
            }
        }

        // 只有实际删除引用后才刷新，避免无效命令改变原焦点状态。
        if removed && (removed_from_current_workspace || self.focus.window == Some(window)) {
            self.refresh_focus();
        }

        removed
    }

    /// 切换到指定 workspace ID。
    ///
    /// 成功时同步更新 current_workspace、重置 FocusState 并重新解析焦点；
    /// 目标不存在时保持全部状态不变并返回 false。
    pub fn switch_workspace(&mut self, id: u32) -> bool {
        // 先验证稳定 ID 存在，避免保存一个无法解析的 current_workspace。
        if self.workspaces.iter().any(|workspace| workspace.id == id) {
            self.current_workspace = id;
            println!("[Compositor] Switched to workspace {}", id);

            // workspace 变化后旧 slot/window 组合失效，先重置再统一刷新。
            self.focus.set_workspace(id);
            self.refresh_focus();
            return true;
        }

        // 无效 ID 不 panic，也不修改原 workspace/focus。
        println!("[Compositor] Workspace {} not found", id);
        false
    }

    /// 循环切换到下一个 workspace。
    ///
    /// 使用 Vec 中的顺序导航，并在末尾 wrap 到第一个 workspace。
    pub fn next_workspace(&mut self) {
        let len = self.workspaces.len();

        // 空 workspace 列表没有可导航目标，直接保持状态。
        if len == 0 {
            return;
        }

        // 当前 ID 无效时从第一个 workspace 恢复，避免 panic。
        let next = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == self.current_workspace)
            .map(|index| (index + 1) % len)
            .unwrap_or(0);
        let id = self.workspaces[next].id;
        self.switch_workspace(id);
    }

    /// 循环切换到上一个 workspace。
    ///
    /// 位于第一个 workspace 时 wrap 到最后一个 workspace。
    pub fn prev_workspace(&mut self) {
        let len = self.workspaces.len();

        // 防御性处理 session 恢复产生的空列表。
        if len == 0 {
            return;
        }

        // 找到当前位置并安全计算前一个索引；无效 ID 回退到索引 0。
        let prev = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == self.current_workspace)
            .map(|index| if index == 0 { len - 1 } else { index - 1 })
            .unwrap_or(0);
        let id = self.workspaces[prev].id;
        self.switch_workspace(id);
    }

    /// 根据当前 workspace 和 slot 内容重新建立焦点一致性。
    ///
    /// 该方法是 focus.window 的统一推导入口：
    /// 先尝试当前 slot，再寻找下一个 occupied slot，最后回退到 slot 0 / None。
    /// Stack 的焦点窗口始终通过 `Workspace::slot_window()` 解析 active window。
    pub fn refresh_focus(&mut self) {
        // 如果恢复数据或外部状态导致 workspace ID 不一致，先重置焦点层级。
        if self.focus.workspace != self.current_workspace {
            self.focus.set_workspace(self.current_workspace);
        }

        // 复制 slot 值，避免只读借用 current workspace 时同时借用 self.focus。
        let current_slot = self.focus.slot;

        // 在只读 workspace 借用范围内计算最终 slot/window，随后统一写回 FocusState。
        let (slot, window) = if let Some(workspace) = self.current_workspace() {
            // 当前 slot 有窗口时保持 slot，并读取 Single 或 Stack active window。
            if let Some(window) = workspace.slot_window(current_slot) {
                (current_slot, Some(window))
            // 当前 slot 为空时，循环查找下一个包含窗口的 slot。
            } else if let Some(slot) = workspace.next_occupied_slot(current_slot) {
                (slot, workspace.slot_window(slot))
            } else {
                // 整个 workspace 为空时使用确定的无焦点状态。
                (0, None)
            }
        } else {
            // current_workspace ID 无效时同样回退到安全空焦点。
            (0, None)
        };

        // set_slot 会先清除旧 window，再写入刚刚推导出的最终窗口。
        self.focus.set_slot(slot);
        self.focus.set_window(window);

        println!(
            "[Focus] workspace={}, slot={}, window={:?}",
            self.focus.workspace, self.focus.slot, self.focus.window
        );
    }

    /// 聚焦当前 workspace 中下一个包含窗口的 slot。
    ///
    /// Workspace 负责 wrap around 搜索；找到目标后再调用 refresh_focus，
    /// 以确保 focus.window 与目标 slot 的 active window 一致。
    pub fn focus_next_slot(&mut self) {
        // 先复制当前 slot，避免后续只读借用 workspace 时借用冲突。
        let current_slot = self.focus.slot;
        let next_slot = self
            .current_workspace()
            .and_then(|workspace| workspace.next_occupied_slot(current_slot));

        // 没有任何 occupied slot 时保持原焦点，不制造无效 slot。
        if let Some(slot) = next_slot {
            self.focus.set_slot(slot);
            self.refresh_focus();
        }
    }

    /// 聚焦当前 workspace 中上一个包含窗口的 slot。
    ///
    /// 行为与 `focus_next_slot()` 对称，并支持从 slot 0 wrap 到末尾。
    pub fn focus_prev_slot(&mut self) {
        let current_slot = self.focus.slot;
        let prev_slot = self
            .current_workspace()
            .and_then(|workspace| workspace.prev_occupied_slot(current_slot));

        // 只有找到可见窗口时才更新 FocusState。
        if let Some(slot) = prev_slot {
            self.focus.set_slot(slot);
            self.refresh_focus();
        }
    }

    /// 尝试直接聚焦指定 slot。
    ///
    /// 只接受固定范围 0..4 且包含 active window 的 slot；
    /// 无效或空 slot 只记录日志，不破坏现有焦点。
    pub fn focus_slot(&mut self, slot: u8) {
        // 固定 slot 模型只允许 0、1、2、3。
        let window = if slot < 4 {
            self.current_workspace()
                .and_then(|workspace| workspace.slot_window(slot))
        } else {
            None
        };

        // 存在可见窗口时才切换 slot，并通过统一刷新写入 window。
        if window.is_some() {
            self.focus.set_slot(slot);
            self.refresh_focus();
        } else {
            println!("[Focus] Slot {} is empty", slot);
        }
    }

    /// 在当前焦点 slot 的 stack 中切换 active window。
    ///
    /// stack 状态只存在于 Workspace；CompositorState 在切换后仅同步 focus.window。
    pub fn next_in_stack(&mut self) {
        // 先复制 slot，避免可变借用 workspace 时读取 self.focus。
        let slot = self.focus.slot;

        // Workspace 负责区分 Empty、Single 和 Stack，并返回最终可见窗口。
        let window = self
            .current_workspace_mut()
            .and_then(|workspace| workspace.next_in_stack(slot));

        // 切换成功后只更新 window，workspace 与 slot 保持不变。
        if let Some(window) = window {
            self.focus.set_window(Some(window));
            println!(
                "[Focus] workspace={}, slot={}, window={:?}",
                self.focus.workspace, self.focus.slot, self.focus.window
            );
        } else {
            println!("[Compositor] No stack/window in focused slot {}", slot);
        }
    }

    /// 按 Fullscreen -> Split -> Grid -> Fullscreen 循环当前布局。
    pub fn cycle_layout(&mut self) {
        // 只读取当前 workspace 的布局值，不在借用期间修改 self。
        let layout = match self.current_workspace().map(|workspace| workspace.layout) {
            // Fullscreen 的下一个模式固定为左右 Split。
            Some(LayoutMode::Fullscreen) => LayoutMode::Split,
            // Split 的下一个模式固定为四宫格 Grid。
            Some(LayoutMode::Split) => LayoutMode::Grid,
            // Grid 到达循环末尾后回到 Fullscreen。
            Some(LayoutMode::Grid) => LayoutMode::Fullscreen,
            // 当前 workspace 不存在时无法切换布局，保持原状态。
            None => return,
        };

        // 复用统一设置入口，确保日志和 focus 刷新行为一致。
        self.set_current_layout(layout);
    }

    /// 设置当前 workspace 的布局模式。
    ///
    /// 只修改 workspace.layout；固定 slot、stack 内容和窗口 ID 均保持不变。
    /// 设置后刷新 focus，确保布局变化后焦点快照仍与当前可见窗口一致。
    pub fn set_current_layout(&mut self, layout: LayoutMode) {
        // current workspace 无效时安全返回，不修改其他状态。
        let Some(workspace) = self.current_workspace_mut() else {
            return;
        };

        // 可变借用仅用于写入布局并复制日志所需 ID。
        workspace.layout = layout;
        let workspace_id = workspace.id;

        println!(
            "[Layout] Workspace {} layout set to {:?}",
            workspace_id, layout
        );
        self.refresh_focus();
    }

    /// 用 session 中的纯数据替换可恢复状态。
    ///
    /// backend、output 和 running 不在参数中，因此不会被 session 覆盖或重建。
    /// 替换完成后统一刷新 focus，修正可能不再指向 active window 的恢复数据。
    pub fn replace_session_state(
        &mut self,
        current_workspace: u32,
        focus: FocusState,
        workspaces: Vec<Workspace>,
    ) {
        // session 中的活动 workspace ID 替换对应的纯数据状态。
        self.current_workspace = current_workspace;

        // 焦点先按 session 快照恢复，随后由 refresh_focus 做一致性校验。
        self.focus = focus;

        // workspace 集合整体替换，backend、output 和 running 保持当前运行期实例。
        self.workspaces = workspaces;

        // 最后统一修正无效 slot、空窗口或 stack active window 对应关系。
        self.refresh_focus();
    }
}

/// EventLoop 使用的全局根状态。
///
/// `compositor` 包含 compositor-owned 运行状态，`registry` 独立负责 WindowId 分配。
/// 该分层确保 registry 只有一个生命周期，不会在 CompositorState 初始化时临时创建。
pub struct State {
    /// workspace、focus、output、backend 等集中状态。
    pub compositor: CompositorState,
    /// 全局唯一窗口 ID 注册表。
    pub registry: WindowRegistry,
    /// 未来 Wayland/Smithay surface 与 WindowId 的占位绑定注册表。
    pub surfaces: SurfaceRegistry,
    /// Wayland client 的纯数据占位注册表。
    pub clients: ClientRegistry,
}

/// 关闭指定窗口后的状态修改结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseWindowResult {
    /// 是否从任意 workspace 中移除了窗口引用。
    pub removed_from_workspace: bool,

    /// 是否在 registry 中成功标记为 dead。
    pub marked_dead: bool,

    /// 被标记为 dead 的 surface 数量。
    pub dead_surfaces: usize,
}

/// 关闭 client 后的纯数据级联结果。
///
/// 该结果不代表真实 Wayland client 已经被 display 移除，只表示核心占位模型
/// 已经完成 client -> surface -> window 的生命周期收束。所有记录都会继续保留，
/// 并通过 alive=false 供后续诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseClientResult {
    /// client 是否成功标记为 dead。
    pub marked_dead: bool,

    /// 本次被标记为 dead 的 surface 列表。
    pub dead_surfaces: Vec<SurfaceId>,

    /// 因 client 断开而同步关闭的 window 列表。
    ///
    /// 即使多个 surface 指向同一窗口，每个 WindowId 也最多出现一次。
    pub closed_windows: Vec<WindowId>,

    /// 成功从 workspace 移除引用的 window 数量。
    pub removed_from_workspace_count: usize,

    /// 成功在 WindowRegistry 中标记为 dead 的 window 数量。
    pub marked_window_dead_count: usize,
}

/// 已有 surface 注册为窗口后的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterWindowForSurfaceResult {
    /// 新创建的窗口 ID。
    pub window: WindowId,

    /// surface 是否成功绑定到该窗口。
    pub bound: bool,
}

/// 关闭 surface 后的状态修改结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseSurfaceResult {
    /// surface 是否成功标记为 dead。
    pub surface_marked_dead: bool,

    /// 如果 surface 绑定了窗口，这里记录被同步关闭的窗口。
    pub closed_window: Option<WindowId>,

    /// 如果同步关闭了窗口，是否从 workspace 移除了窗口引用。
    pub removed_from_workspace: bool,

    /// 如果同步关闭了窗口，是否在 registry 中标记窗口为 dead。
    pub marked_window_dead: bool,
}

impl State {
    /// 创建默认全局状态并生成三个测试窗口。
    ///
    /// 测试窗口用于当前无真实 Wayland client 阶段验证布局和焦点；
    /// 程序入口随后可用 session 数据替换这些默认纯数据状态。
    pub fn new() -> Self {
        let mut state = Self {
            compositor: CompositorState::new(),
            registry: WindowRegistry::new(),
            surfaces: SurfaceRegistry::new(),
            clients: ClientRegistry::new(),
        };

        // 通过统一 spawn 入口创建窗口，确保 registry 与 workspace 分配保持同步。
        for _ in 0..3 {
            state.spawn_window();
        }

        state
    }

    /// 注册一个自动分配 ID 的 client 占位记录。
    ///
    /// 该方法不接真实 Smithay client，也不会把真实连接注册到 Wayland display。
    /// client socket 连接不等于 surface 或 window，本阶段只创建独立 metadata。
    pub fn register_client(&mut self, kind: ClientKind, name: Option<String>) -> ClientId {
        self.clients.register_client(kind, name)
    }

    /// 使用指定 ID 注册 client 占位记录。
    ///
    /// 该方法用于未来 backend 或 Wayland display 已经持有稳定 client identity
    /// 的场景；当前仍然只写入纯数据注册表。
    pub fn register_client_with_id(
        &mut self,
        client: ClientId,
        kind: ClientKind,
        name: Option<String>,
    ) -> bool {
        self.clients.register_client_with_id(client, kind, name)
    }

    /// 关闭 client 占位记录，并级联关闭它拥有的 surface 和 window。
    ///
    /// Wayland client 断开后，它创建的 surface 不再有效；已经映射成逻辑窗口的
    /// surface 还必须同步从 workspace 移除窗口，并把窗口 metadata 标记为 dead。
    /// 本方法只修改纯数据 registry，不接触真实 Smithay client，也不会删除记录。
    pub fn close_client(&mut self, client: ClientId) -> CloseClientResult {
        let marked_dead = self.clients.mark_dead(client);

        // 不存在的 client 不触发基于悬空 owner 的级联，返回明确空结果。
        if !marked_dead {
            return CloseClientResult {
                marked_dead: false,
                dead_surfaces: Vec::new(),
                closed_windows: Vec::new(),
                removed_from_workspace_count: 0,
                marked_window_dead_count: 0,
            };
        }

        // 必须在修改 surface 生命周期前收集窗口；同一窗口在 registry 内部已去重。
        let windows = self.surfaces.windows_for_client(client);
        let dead_surfaces = self.surfaces.mark_dead_for_client(client);
        let mut closed_windows = Vec::new();
        let mut removed_from_workspace_count = 0;
        let mut marked_window_dead_count = 0;

        for window in windows {
            // 防御性去重保证异常数据下每个窗口在单次级联中也只关闭一次。
            if closed_windows.contains(&window) {
                continue;
            }

            let result = self.close_window(window);

            if result.removed_from_workspace {
                removed_from_workspace_count += 1;
            }

            if result.marked_dead {
                marked_window_dead_count += 1;
            }

            closed_windows.push(window);
        }

        CloseClientResult {
            marked_dead,
            dead_surfaces,
            closed_windows,
            removed_from_workspace_count,
            marked_window_dead_count,
        }
    }

    /// 创建窗口 ID，并把窗口分配到当前 workspace。
    ///
    /// WindowRegistry 是唯一 ID 来源；分配失败时仍保留已创建 ID，
    /// 但记录日志以暴露无效 current_workspace 状态。
    pub fn spawn_window(&mut self) -> WindowId {
        // 当前无真实 client，先创建带默认 metadata 的 mock 窗口记录。
        let window = self.registry.create_mock();
        if !self.compositor.assign_window_to_current_workspace(window) {
            println!("[State] No current workspace for window {}", window);
        }
        return window;
    }

    /// 使用指定 metadata 注册窗口，并分配到当前 workspace。
    ///
    /// 该方法为未来真实 Wayland surface map 事件准备。
    /// 当前只创建逻辑窗口记录，不持有真实 surface。
    pub fn register_window(
        &mut self,
        title: impl Into<String>,
        app_id: Option<String>,
        kind: WindowKind,
    ) -> WindowId {
        self.register_window_internal(title, app_id, kind, true)
    }

    /// 使用指定 metadata 注册窗口，并可选择是否自动创建 surface 占位记录。
    ///
    /// 普通 RegisterWindow 保留自动创建行为；已有 surface map 成窗口时关闭自动创建，
    /// 避免同一个 xdg_toplevel 被重复表示为两条 surface 记录。
    fn register_window_internal(
        &mut self,
        title: impl Into<String>,
        app_id: Option<String>,
        kind: WindowKind,
        create_surface_placeholder: bool,
    ) -> WindowId {
        // metadata 移入 WindowRegistry 前先记录是否需要自动创建 surface。
        let needs_surface = create_surface_placeholder && kind == WindowKind::WaylandPlaceholder;

        // WindowRegistry 仍然是 WindowId 和 metadata 的唯一创建入口。
        let window = self.registry.create_with_metadata(title, app_id, kind);

        // workspace 分配继续由 CompositorState 维护 slot、stack 和焦点不变量。
        if !self.compositor.assign_window_to_current_workspace(window) {
            println!("[State] No current workspace for registered window {window}");
        }

        // WaylandPlaceholder 模拟未来 xdg_toplevel map，并建立纯数据 surface 绑定。
        if needs_surface {
            self.surfaces
                .register_for_window(window, SurfaceRole::XdgToplevel);
        }

        return window;
    }

    /// 注册一个尚未绑定窗口的 surface 占位记录。
    ///
    /// 该方法为未来 Smithay new surface 事件准备，只创建纯数据 SurfaceRecord，
    /// 不创建 WindowRecord，也不持有真实 Smithay surface。
    pub fn register_surface(&mut self, role: SurfaceRole) -> SurfaceId {
        self.surfaces.register_surface(role)
    }

    /// 注册一个可选绑定 client 的 surface。
    ///
    /// 该方法仍然只创建纯数据占位记录，不接真实 Smithay surface 或 client。
    /// ClientId 表示连接归属，SurfaceId 表示协议对象，WindowId 表示逻辑窗口。
    pub fn register_surface_for_client(
        &mut self,
        client: Option<ClientId>,
        role: SurfaceRole,
    ) -> SurfaceId {
        self.surfaces.register_surface_for_client(client, role)
    }

    /// 使用指定 ID 注册一个 surface 占位记录。
    ///
    /// 该方法用于未来 backend 已经持有稳定 surface identity 的场景。
    /// 重复 ID 不会覆盖已有记录，并通过 false 报告注册失败。
    pub fn register_surface_with_id(&mut self, surface: SurfaceId, role: SurfaceRole) -> bool {
        self.surfaces.register_surface_with_id(surface, role)
    }

    /// 使用指定 ID 注册一个可选绑定 client 的 surface。
    ///
    /// 如果 client 不存在，本方法仍允许创建纯数据记录，Validator 会报告问题。
    /// 这样后端事件顺序异常时不会丢失原始 surface 事实。
    pub fn register_surface_with_id_for_client(
        &mut self,
        surface: SurfaceId,
        client: Option<ClientId>,
        role: SurfaceRole,
    ) -> bool {
        self.surfaces
            .register_surface_with_id_for_client(surface, client, role)
    }

    /// 将已有 surface 绑定到已有 client。
    ///
    /// 该方法只建立纯数据归属关系，不创建 window。client 不存在时返回 false，
    /// 避免通过公开命令入口制造无效引用。
    pub fn bind_surface_to_client(&mut self, surface: SurfaceId, client: ClientId) -> bool {
        if self.clients.get(client).is_none() {
            return false;
        }

        self.surfaces.bind_client(surface, client)
    }

    /// 将已有 surface 绑定到已有窗口。
    ///
    /// 该方法只建立 SurfaceId 到 WindowId 的纯数据关系，不修改 workspace。
    /// surface 或窗口不存在时返回 false，不自动创建缺失记录。
    pub fn bind_surface_to_window(&mut self, surface: SurfaceId, window: WindowId) -> bool {
        if self.registry.get(window).is_none() {
            return false;
        }

        self.surfaces.bind_window(surface, window)
    }

    /// 为已有 surface 创建逻辑窗口，并建立绑定。
    ///
    /// 该方法模拟未来 xdg_toplevel map：surface 已经由 protocol/backend 发现，
    /// map 时创建 WindowRecord，再绑定到 WindowId，并分配到当前 workspace。
    /// surface 不存在时仍创建窗口，但通过 bound=false 明确报告绑定失败。
    pub fn register_window_for_surface(
        &mut self,
        surface: SurfaceId,
        title: impl Into<String>,
        app_id: Option<String>,
        kind: WindowKind,
    ) -> RegisterWindowForSurfaceResult {
        // 已有 surface 路径关闭自动占位创建，避免生成第二条重复 surface。
        let window = self.register_window_internal(title, app_id, kind, false);
        let bound = self.surfaces.bind_window(surface, window);

        RegisterWindowForSurfaceResult { window, bound }
    }

    /// 关闭当前焦点窗口，并同步 WindowRegistry 生命周期 metadata。
    ///
    /// CompositorState 负责从 workspace 的 Slot/Stack 中移除窗口并刷新焦点；
    /// State 负责把同一个 WindowId 在 registry 中标记为 dead。
    pub fn close_focused_window(&mut self) -> bool {
        // 在 CompositorState 修改焦点前保存原 WindowId，供 registry 同步生命周期。
        let focused = self.compositor.focus.window;
        let closed = self.compositor.close_focused_window();

        // 只有 workspace 确认删除成功后，才把对应 metadata 标记为 dead。
        if closed {
            if let Some(window) = focused {
                self.registry.mark_dead(window);

                // 焦点关闭同样必须结束该窗口绑定的 surface 生命周期。
                self.surfaces.mark_dead_for_window(window);
            }
        }

        return closed;
    }

    /// 关闭指定窗口，并同步更新 workspace 与 registry。
    ///
    /// 该方法用于未来 Wayland client unmap / destroy 事件，不依赖当前焦点，
    /// 因此可以关闭任意已知 WindowId。
    pub fn close_window(&mut self, window: WindowId) -> CloseWindowResult {
        // CompositorState 负责移除全部 workspace 引用并按需刷新焦点。
        let removed_from_workspace = self.compositor.remove_window(window);

        // Registry 独立记录逻辑窗口生命周期，不反向修改 workspace。
        let marked_dead = self.registry.mark_dead(window);

        // SurfaceRegistry 同步结束所有绑定到该 WindowId 的存活 surface。
        let dead_surfaces = self.surfaces.mark_dead_for_window(window);

        CloseWindowResult {
            removed_from_workspace,
            marked_dead,
            dead_surfaces,
        }
    }

    /// 关闭指定 surface，并在它绑定窗口时同步关闭窗口。
    ///
    /// 该方法用于未来 Wayland surface destroy / unmap 事件。
    /// CloseSurface 的输入是 SurfaceId；CloseWindow 的输入是 WindowId。
    /// 本阶段只修改纯数据生命周期，不持有或销毁真实 Smithay surface。
    pub fn close_surface(&mut self, surface: SurfaceId) -> CloseSurfaceResult {
        // 必须在标记 surface dead 前保存绑定关系，供后续同步关闭逻辑使用。
        let closed_window = self.surfaces.window_for_surface(surface);
        let surface_marked_dead = self.surfaces.mark_dead(surface);
        let mut removed_from_workspace = false;
        let mut marked_window_dead = false;

        if let Some(window) = closed_window {
            // close_window 会同时标记同一窗口绑定的其他 surface 为 dead。
            let result = self.close_window(window);
            removed_from_workspace = result.removed_from_workspace;
            marked_window_dead = result.marked_dead;
        }

        CloseSurfaceResult {
            surface_marked_dead,
            closed_window,
            removed_from_workspace,
            marked_window_dead,
        }
    }

    /// 为当前输出构建 RenderFrame，并附加 WindowRegistry metadata。
    ///
    /// CompositorState 只负责生成不含 registry 信息的纯 RenderFrame；
    /// State 同时拥有 compositor 与 registry，因此在这里把 WindowId metadata
    /// 合并进独立渲染快照，避免布局、场景或 compositor 状态依赖窗口注册表。
    pub fn current_render_frame_for_current_output(&self) -> RenderFrame {
        // 先通过纯 compositor 状态生成基础渲染命令。
        let mut frame = self.compositor.current_render_frame_for_current_output();

        // metadata 只复制进本帧快照，不让 renderer 持有 registry 引用。
        frame.attach_metadata(&self.registry);
        return frame;
    }

    /// 生成当前 compositor 状态的调试快照。
    ///
    /// 该方法只读访问 State，并委托 Inspector 合并 workspace、focus、output
    /// 和 registry 信息，不影响渲染、session 或事件循环。
    pub fn debug_snapshot(&self) -> SystemDebugSnapshot {
        Inspector::snapshot(self)
    }

    /// 只读验证当前 compositor 状态是否满足核心不变量。
    ///
    /// 该方法不会修复状态，也不会修改 State，只返回独立验证报告。
    pub fn validate(&self) -> ValidationReport {
        StateValidator::validate(self)
    }

    /// 生成当前 compositor 状态的完整诊断包。
    ///
    /// 诊断包同时包含只读调试快照和只读验证报告。
    /// 该方法不会修改 State，也不会触发自动修复或日志输出。
    pub fn debug_bundle(&self) -> DebugBundle {
        DebugBundle::from_state(self)
    }

    /// 生成当前 compositor 状态的完整诊断文本。
    ///
    /// 该方法只是 `debug_bundle().pretty_print()` 的便捷封装。
    /// 它不会修改 State，也不会触发自动修复或日志输出。
    pub fn debug_bundle_text(&self) -> String {
        self.debug_bundle().pretty_print()
    }

    /// 处理外部系统命令。
    ///
    /// 这是未来 Smithay、Wayland protocol 和 backend 进入核心状态的统一入口。
    /// 方法本身委托 CommandHandler，避免 State 中直接堆积命令分发细节。
    pub fn handle_command(&mut self, command: CoreCommand) -> CommandResult {
        CommandHandler::handle(self, command)
    }

    /// 暴露 workspace 切换的薄封装。
    ///
    /// 实际状态修改仍由 CompositorState 完成。
    pub fn switch_workspace(&mut self, id: u32) {
        self.compositor.switch_workspace(id);
    }

    /// 将当前可恢复纯数据写入 pretty JSON。
    ///
    /// backend、output、renderer 和真实 surface 不包含在 SessionState 中。
    pub fn save_session(&self, path: &str) -> io::Result<()> {
        // 从运行时状态构建独立的可序列化镜像。
        let session = SessionState {
            current_workspace: self.compositor.current_workspace,
            focus: self.compositor.focus.into(),
            workspaces: self
                .compositor
                .workspaces
                .iter()
                .map(workspace_to_session)
                .collect(),
            next_window_id: self.registry.next_id(),
        };

        // serde_json 错误转换为 io::Error，使调用方使用统一 I/O 错误接口。
        let json = serde_json::to_string_pretty(&session)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        // 只有序列化成功后才覆盖目标文件。
        fs::write(path, json)?;
        println!("[Session] Saved session to {}", path);
        Ok(())
    }

    /// 从 JSON 文件恢复 workspace、focus、stack 和 WindowRegistry。
    ///
    /// 读取或反序列化失败时返回错误，调用方可以继续使用当前默认状态。
    /// backend 与 OutputState 保持现有实例，不参与恢复。
    pub fn load_session(&mut self, path: &str) -> io::Result<()> {
        // 先完整读取文件，避免部分内容直接修改运行中状态。
        let json = fs::read_to_string(path)?;

        // 只有 JSON 完整反序列化成功后才开始构建 workspace。
        let session: SessionState = serde_json::from_str(&json)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        // 每个 SessionWorkspace 都规范化回固定 `[Slot; 4]`。
        let workspaces = session
            .workspaces
            .iter()
            .map(workspace_from_session)
            .collect();

        // 替换可恢复 compositor 数据，并由内部 refresh_focus 修正一致性。
        self.compositor.replace_session_state(
            session.current_workspace,
            session.focus.into(),
            workspaces,
        );

        // session 已经成功读取并反序列化，workspace 也已经替换为 session 内容。
        // 此时 State::new() 默认创建的 mock metadata 不再可信，必须清空 registry，
        // 再根据恢复后的 workspace 引用重建窗口 metadata。
        self.registry.reset();

        // WindowRecord 当前不进入 session。
        // 因此恢复 session 后，需要根据 workspace 中实际引用的 WindowId 补齐 mock metadata。
        for workspace in &self.compositor.workspaces {
            for window in workspace.window_ids() {
                self.registry.ensure_mock(window);
            }
        }

        // next_id 同时尊重 session 保存的计数器，以及 ensure_mock
        // 根据最大窗口 ID 推导出的计数器。
        let next_id = session.next_window_id.max(self.registry.next_id());
        self.registry.set_next_id(next_id);

        println!("[Session] Loaded session from {}", path);
        Ok(())
    }

    /// Action 到集中状态修改的唯一语义分发入口。
    ///
    /// EventLoop、输入源和 KeybindingMap 都不直接修改 workspace/focus/output；
    /// 它们只能产生 Action，并由该 match 委托给对应状态方法。
    pub fn dispatch_action(&mut self, action: Action) {
        match action {
            // 下一个 workspace 的顺序与 wrap around 由 CompositorState 维护。
            Action::NextWorkspace => self.compositor.next_workspace(),
            // 上一个 workspace 的顺序与 wrap around 由 CompositorState 维护。
            Action::PrevWorkspace => self.compositor.prev_workspace(),
            // 指定 ID 的存在性检查由 switch_workspace 负责，失败时不修改状态。
            Action::SwitchWorkspace(id) => {
                self.compositor.switch_workspace(id);
            }
            // 窗口创建需要同时访问 registry 和 compositor，因此由根 State 协调。
            Action::SpawnWindow => {
                self.spawn_window();
            }
            // CompositorState 删除 Slot/Stack 引用，State 同步 registry metadata。
            Action::CloseFocusedWindow => {
                self.close_focused_window();
            }
            // 下一个 occupied slot 的查找和焦点刷新都封装在 CompositorState 中。
            Action::FocusNextSlot => self.compositor.focus_next_slot(),
            // 上一个 occupied slot 的查找同样由 CompositorState 维护。
            Action::FocusPrevSlot => self.compositor.focus_prev_slot(),
            // 指定 slot 的范围和窗口占用状态由 focus_slot 校验。
            Action::FocusSlot(slot) => self.compositor.focus_slot(slot),
            // stack active window 切换后由 CompositorState 同步 focus.window。
            Action::NextInStack => self.compositor.next_in_stack(),
            // 循环布局只修改当前 workspace.layout，并保持 slot 内容不变。
            Action::CycleLayout => self.compositor.cycle_layout(),
            // 显式 Fullscreen 设置复用统一布局入口和焦点刷新流程。
            Action::SetLayoutFullscreen => {
                self.compositor.set_current_layout(LayoutMode::Fullscreen)
            }
            // 显式 Split 设置复用统一布局入口和焦点刷新流程。
            Action::SetLayoutSplit => self.compositor.set_current_layout(LayoutMode::Split),
            // 显式 Grid 设置复用统一布局入口和焦点刷新流程。
            Action::SetLayoutGrid => self.compositor.set_current_layout(LayoutMode::Grid),
            // 输出变化构造纯数据 OutputSize，再交给 OutputState 更新。
            Action::ResizeOutput { width, height } => {
                self.compositor.resize_output(OutputSize { width, height });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{CompositorState, State};
    use crate::core::render::RenderCommand;
    use crate::core::session::{
        SessionFocus, SessionLayoutMode, SessionSlot, SessionSlotContent, SessionState,
        SessionWorkspace,
    };

    /// 验证关闭焦点窗口后，CompositorState 会刷新到下一个已占用 slot。
    #[test]
    fn close_focused_window_refreshes_focus() {
        let mut compositor = CompositorState::new();
        compositor.assign_window_to_current_workspace(1);
        compositor.assign_window_to_current_workspace(2);

        // 初始焦点必须落在第一个窗口所在的 slot 0。
        assert_eq!(compositor.focus.slot, 0);
        assert_eq!(compositor.focus.window, Some(1));

        // 关闭当前焦点窗口必须成功。
        assert!(compositor.close_focused_window());

        // 被关闭窗口必须从原 slot 中移除。
        assert_eq!(
            compositor
                .current_workspace()
                .and_then(|workspace| workspace.slot_window(0)),
            None
        );

        // slot 0 变空后，焦点必须移动到下一个 occupied slot 1 的窗口 2。
        assert_eq!(compositor.focus.slot, 1);
        assert_eq!(compositor.focus.window, Some(2));
    }

    /// 验证通过 State 关闭焦点窗口时，会同步标记 registry metadata 为 dead。
    #[test]
    fn close_focused_window_marks_registry_record_dead() {
        let mut state = State::new();
        let focused = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");

        // 默认创建的焦点窗口必须先处于存活状态。
        assert!(state.registry.is_alive(focused));

        // 关闭必须通过 State 协调 Workspace 和 WindowRegistry 两侧状态。
        assert!(state.close_focused_window());

        // workspace 删除成功后，同一 WindowId 的 metadata 必须标记为 dead。
        assert!(!state.registry.is_alive(focused));
    }

    /// 验证成功加载 session 后会重建 registry，不保留默认 mock metadata。
    #[test]
    fn load_session_rebuilds_registry_without_default_mock_residue() {
        // 使用进程 ID 构造测试专用临时路径，避免访问真实 session 文件。
        let path = std::env::temp_dir().join(format!(
            "sky_mirror_test_session_registry_reset_{}.json",
            std::process::id()
        ));
        let session = SessionState {
            current_workspace: 0,
            focus: SessionFocus {
                workspace: 0,
                slot: 0,
                window: Some(42),
            },
            workspaces: vec![SessionWorkspace {
                id: 0,
                layout: SessionLayoutMode::Fullscreen,
                slots: vec![SessionSlot {
                    id: 0,
                    content: SessionSlotContent::Single(42),
                }],
            }],
            next_window_id: 2,
        };
        let json = serde_json::to_string_pretty(&session).expect("测试 session 必须能够序列化");
        fs::write(&path, json).expect("测试 session 文件必须能够写入");

        let mut state = State::new();

        // State::new 创建的默认窗口 1 必须先存在，才能验证加载后确实被清理。
        assert!(state.registry.get(1).is_some());

        let load_result = state.load_session(path.to_str().expect("临时路径必须是有效 UTF-8"));

        // load_session 完成文件读取后立即清理测试文件，不污染真实运行环境。
        fs::remove_file(&path).expect("测试 session 文件必须能够删除");
        load_result.expect("测试 session 必须能够成功加载");

        // registry 重建后不得残留 State::new 创建的默认窗口 metadata。
        assert!(state.registry.get(1).is_none());

        let restored = state
            .registry
            .get(42)
            .expect("session workspace 引用必须补齐 metadata");

        // 恢复窗口必须使用 ensure_mock 生成的明确标题。
        assert_eq!(restored.title, "Restored Window 42");

        // next_id 必须高于恢复窗口 ID，避免后续创建发生冲突。
        assert!(state.registry.next_id() >= 43);
    }

    /// 验证 State 生成的渲染帧包含 WindowRegistry metadata。
    #[test]
    fn state_render_frame_contains_window_metadata() {
        let state = State::new();

        let frame = state.current_render_frame_for_current_output();
        let first = frame
            .commands
            .first()
            .expect("默认状态必须生成至少一条绘制命令");
        let RenderCommand::DrawWindow { metadata, .. } = first;
        let metadata = metadata.as_ref().expect("State 必须附加窗口 metadata");

        // 默认窗口标题必须来自 WindowRegistry 创建的 mock 记录。
        assert!(metadata.title.starts_with("Mock Window "));

        // 默认 mock 窗口在初始状态下必须处于存活状态。
        assert!(metadata.alive);
    }
}
