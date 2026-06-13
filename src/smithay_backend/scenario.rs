//! Smithay runtime 场景回放探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 它不会启动真实 compositor，不接真实 client，不保存真实 `wl_surface` 或
//! `xdg_toplevel`，也不会注册任何 Wayland protocol global。
//!
//! 该模块只把前面各阶段的 runtime helper 串成可重复的端到端测试场景。
//! 每一步场景都必须先通过 runtime helper 入队，再通过 `run_once()` 进入核心状态。
//! 该边界用于验证 Smithay 纯数据探针组合后的生命周期行为是否稳定。

use crate::{
    core::{
        action::Action,
        backend_driver::BackendDriverRunReport,
        client::ClientId,
        command::CommandResult,
        state::State,
        surface::{SurfaceId, SurfaceRole},
        workspace::WindowId,
    },
    smithay_backend::{runtime::SmithayRuntimeProbe, toplevel_event::SmithayToplevelMapDescriptor},
};

/// Smithay runtime 场景回放器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该场景只驱动纯数据 runtime probe，
/// 不启动真实 compositor。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayRuntimeScenarioMode {
    /// 纯探针模式。
    ///
    /// 不接真实 client，不插入 calloop，不注册 protocol global。
    ProbeOnly,
}

/// 单个场景步骤的执行报告。
///
/// 每个步骤都对应一次 `runtime.run_once()`，因此可以看到该步是否处理事件、
/// 是否保持核心状态有效，以及核心返回了什么命令结果。
#[derive(Debug)]
pub struct SmithayRuntimeScenarioStepReport {
    /// 步骤名称。
    ///
    /// 使用静态字符串是为了让测试输出稳定。
    pub name: &'static str,

    /// 该步骤对应的 `BackendDriverRunner` 报告。
    pub report: BackendDriverRunReport,
}

impl SmithayRuntimeScenarioStepReport {
    /// 当前步骤是否实际处理了一个后端事件。
    pub fn handled_event(&self) -> bool {
        self.report.handled_event()
    }

    /// 当前步骤执行后核心状态是否有效。
    pub fn is_valid(&self) -> bool {
        self.report.is_valid()
    }

    /// 当前步骤是否返回了文本结果。
    ///
    /// 该方法只读取已经保存的命令结果，不重新执行事件或访问核心状态。
    pub fn text_result(&self) -> Option<&str> {
        let runtime_result = self.report.runtime_result.as_ref()?;

        match &runtime_result.result {
            CommandResult::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }
}

/// 场景回放整体报告。
///
/// 该报告保存所有步骤的执行结果，便于测试确认每一步都经过 runtime/driver 边界。
#[derive(Debug)]
pub struct SmithayRuntimeScenarioReport {
    /// 所有步骤报告。
    pub steps: Vec<SmithayRuntimeScenarioStepReport>,
}

impl SmithayRuntimeScenarioReport {
    /// 创建空场景报告。
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// 追加一个步骤报告。
    pub fn push(&mut self, step: SmithayRuntimeScenarioStepReport) {
        self.steps.push(step);
    }

    /// 场景是否每一步都处理了事件。
    pub fn all_handled(&self) -> bool {
        self.steps.iter().all(|step| step.handled_event())
    }

    /// 场景执行后每一步是否都保持状态有效。
    pub fn all_valid(&self) -> bool {
        self.steps.iter().all(|step| step.is_valid())
    }

    /// 返回步骤数量。
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// 当前报告是否为空。
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// 以稳定文本格式输出场景报告。
    ///
    /// 该方法只格式化已保存的步骤，不重新运行场景，也不打印到标准输出。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        output.push_str("Smithay Runtime Scenario Report\n");

        for step in &self.steps {
            output.push_str(&format!(
                "- {}: handled={} valid={}\n",
                step.name,
                step.handled_event(),
                step.is_valid()
            ));
        }

        output
    }
}

/// Smithay runtime 场景回放器。
///
/// 该结构同时持有一个 `SmithayRuntimeProbe` 和一个核心 `State`。
/// 它只通过 runtime helper 入队事件，再调用 `runtime.run_once()` 推进状态。
/// 这不是一个真实 compositor，只用于验证纯数据 Smithay 探针链路组合是否稳定。
pub struct SmithayRuntimeScenario {
    /// Smithay runtime 组合探针。
    ///
    /// 所有事件都必须先进入 runtime 内部 driver。
    runtime: SmithayRuntimeProbe,

    /// 核心状态。
    ///
    /// 场景可以在每步执行后读取该状态进行断言，但不会绕过 runtime 修改它。
    state: State,

    /// 场景执行报告。
    report: SmithayRuntimeScenarioReport,

    /// 当前场景模式。
    mode: SmithayRuntimeScenarioMode,
}

impl SmithayRuntimeScenario {
    /// 创建纯数据 runtime 场景。
    ///
    /// 该构造器不读取运行时目录，也不创建 Display 或 listening socket。
    pub fn new_probe_only() -> Self {
        Self {
            runtime: SmithayRuntimeProbe::new_probe_only(),
            state: State::new(),
            report: SmithayRuntimeScenarioReport::new(),
            mode: SmithayRuntimeScenarioMode::ProbeOnly,
        }
    }

    /// 兼容旧调用方式创建纯数据场景。
    ///
    /// `new_auto` 不再构造系统资源；Linux Display/socket 由 `linux_runtime` 负责。
    pub fn new_auto() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new_probe_only())
    }

    /// 使用已有 runtime 和 state 创建场景。
    ///
    /// 该方法主要用于测试。后续场景推进仍然只能通过 runtime helper 和
    /// `run_once()` 完成。
    pub fn from_parts(runtime: SmithayRuntimeProbe, state: State) -> Self {
        Self {
            runtime,
            state,
            report: SmithayRuntimeScenarioReport::new(),
            mode: SmithayRuntimeScenarioMode::ProbeOnly,
        }
    }

    /// 当前是否仍然只是纯探针场景。
    pub fn is_probe_only(&self) -> bool {
        self.mode == SmithayRuntimeScenarioMode::ProbeOnly && self.runtime.is_probe_only()
    }

    /// 返回当前场景模式。
    pub fn mode(&self) -> SmithayRuntimeScenarioMode {
        self.mode
    }

    /// 只读访问核心状态。
    ///
    /// 该方法只供测试断言使用，不能通过它绕过 runtime 修改状态。
    pub fn state(&self) -> &State {
        &self.state
    }

    /// 只读访问场景报告。
    pub fn report(&self) -> &SmithayRuntimeScenarioReport {
        &self.report
    }

    /// 消费场景并返回内部状态。
    ///
    /// 该方法用于测试完成后检查最终状态，不会额外执行任何场景步骤。
    pub fn into_state(self) -> State {
        self.state
    }

    /// 分配 client ID，推入 `ClientConnected`，并立即运行一步。
    ///
    /// 返回分配出的 `ClientId`。真正注册 client 只发生在 `run_step` 内部。
    pub fn connect_allocated_client(&mut self, name: Option<String>) -> ClientId {
        let client = self.runtime.push_allocated_client_connected(name);
        self.run_step("connect_allocated_client");
        client
    }

    /// 分配 surface ID，推入 `SurfaceCreated`，并立即运行一步。
    ///
    /// 返回分配出的 `SurfaceId`。`SurfaceCreated` 本身不会创建窗口。
    pub fn create_allocated_surface(
        &mut self,
        client: Option<ClientId>,
        role: SurfaceRole,
    ) -> SurfaceId {
        let surface = self.runtime.push_allocated_surface_created(client, role);
        self.run_step("create_allocated_surface");
        surface
    }

    /// 推入 `ToplevelMapped`，并立即运行一步。
    ///
    /// 返回该 surface 在核心处理完成后绑定的 `WindowId`。
    pub fn map_toplevel(
        &mut self,
        surface: SurfaceId,
        title: impl Into<String>,
        app_id: Option<String>,
    ) -> WindowId {
        self.runtime
            .push_toplevel_mapped(SmithayToplevelMapDescriptor::new(surface, title, app_id));

        self.run_step("map_toplevel");

        self.state
            .surfaces
            .get(surface)
            .and_then(|record| record.window)
            .expect("ToplevelMapped 处理后 surface 必须绑定 window")
    }

    /// 推入 `SurfaceClosed`，并立即运行一步。
    ///
    /// surface 和绑定窗口的实际生命周期变化只发生在该步骤进入核心之后。
    pub fn close_surface(&mut self, surface: SurfaceId) {
        self.runtime.push_surface_closed(surface);
        self.run_step("close_surface");
    }

    /// 推入 `ClientDisconnected`，并立即运行一步。
    ///
    /// client 拥有的 surface 和窗口级联关闭仍由核心命令语义完成。
    pub fn disconnect_client(&mut self, client: ClientId) {
        self.runtime.push_client_disconnected(client);
        self.run_step("disconnect_client");
    }

    /// 推入 `OutputResized`，并立即运行一步。
    ///
    /// 本方法只调用 runtime helper，不直接修改核心输出状态。
    pub fn resize_output(&mut self, width: u32, height: u32) {
        self.runtime.push_output_resized_size(width, height);
        self.run_step("resize_output");
    }

    /// 推入 `ActionRequested`，并立即运行一步。
    ///
    /// 动作仍然通过后端事件和核心命令边界执行。
    pub fn request_action(&mut self, action: Action) {
        self.runtime.push_action(action);
        self.run_step("request_action");
    }

    /// 推入 `DebugRequested`，并立即运行一步。
    ///
    /// 返回核心在 `run_once()` 后生成的完整诊断文本。
    pub fn request_debug_text(&mut self) -> String {
        self.runtime.push_debug_requested();
        let step = self.run_step("request_debug_text");

        step.text_result()
            .expect("DebugRequested 必须返回文本")
            .to_string()
    }

    /// 推入 `ValidateRequested`，并立即运行一步。
    ///
    /// 返回核心在 `run_once()` 后生成的状态验证文本。
    pub fn request_validation_text(&mut self) -> String {
        self.runtime.push_validate_requested();
        let step = self.run_step("request_validation_text");

        step.text_result()
            .expect("ValidateRequested 必须返回文本")
            .to_string()
    }

    /// 记录并返回一个步骤执行结果。
    ///
    /// 这是场景中唯一推进核心状态的位置；每次调用只运行 runtime 的一轮事件。
    fn run_step(&mut self, name: &'static str) -> &SmithayRuntimeScenarioStepReport {
        let report = self.runtime.run_once(&mut self.state);

        self.report
            .push(SmithayRuntimeScenarioStepReport { name, report });

        self.report
            .steps
            .last()
            .expect("刚刚插入的步骤报告必须存在")
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayRuntimeScenario, SmithayRuntimeScenarioMode};
    use crate::core::{action::Action, surface::SurfaceRole};

    /// 验证场景回放器及其内部 runtime 都保持纯探针模式。
    #[test]
    fn smithay_runtime_scenario_mode_is_probe_only() {
        let scenario = SmithayRuntimeScenario::new_probe_only();

        assert!(scenario.is_probe_only());
        assert_eq!(scenario.mode(), SmithayRuntimeScenarioMode::ProbeOnly);
    }

    /// 验证 client、surface 和 toplevel map 可以通过场景步骤建立完整绑定。
    #[test]
    fn smithay_runtime_scenario_maps_toplevel_window() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        let client = scenario.connect_allocated_client(Some("app".to_string()));
        let surface = scenario.create_allocated_surface(Some(client), SurfaceRole::XdgToplevel);
        let window = scenario.map_toplevel(surface, "Terminal", Some("foot".to_string()));

        assert!(scenario.state().clients.is_alive(client));

        let surface_record = scenario
            .state()
            .surfaces
            .get(surface)
            .expect("surface 必须存在");

        assert_eq!(surface_record.client, Some(client));
        assert_eq!(surface_record.window, Some(window));

        let window_record = scenario
            .state()
            .registry
            .get(window)
            .expect("window 必须存在");

        assert!(window_record.alive);
        assert_eq!(window_record.title, "Terminal");
        assert_eq!(window_record.app_id, Some("foot".to_string()));
        assert_eq!(scenario.report().len(), 3);
        assert!(scenario.report().all_handled());
        assert!(scenario.report().all_valid());
    }

    /// 验证关闭单个 surface 只结束目标 surface 和其绑定窗口。
    #[test]
    fn smithay_runtime_scenario_surface_close_only_closes_target_window() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        let client = scenario.connect_allocated_client(Some("app".to_string()));
        let first_surface =
            scenario.create_allocated_surface(Some(client), SurfaceRole::XdgToplevel);
        let second_surface =
            scenario.create_allocated_surface(Some(client), SurfaceRole::XdgToplevel);
        let first_window = scenario.map_toplevel(first_surface, "First", None);
        let second_window = scenario.map_toplevel(second_surface, "Second", None);

        scenario.close_surface(first_surface);

        assert!(!scenario.state().surfaces.is_alive(first_surface));
        assert!(!scenario.state().registry.is_alive(first_window));
        assert!(scenario.state().surfaces.is_alive(second_surface));
        assert!(scenario.state().registry.is_alive(second_window));
        assert!(scenario.state().clients.is_alive(client));
        assert!(scenario.report().all_handled());
        assert!(scenario.report().all_valid());
    }

    /// 验证 client 断开会由核心级联关闭其全部 surface 和窗口。
    #[test]
    fn smithay_runtime_scenario_client_disconnect_cascades_all_owned_surfaces() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        let client = scenario.connect_allocated_client(Some("app".to_string()));
        let first_surface =
            scenario.create_allocated_surface(Some(client), SurfaceRole::XdgToplevel);
        let second_surface =
            scenario.create_allocated_surface(Some(client), SurfaceRole::XdgToplevel);
        let first_window = scenario.map_toplevel(first_surface, "First", None);
        let second_window = scenario.map_toplevel(second_surface, "Second", None);

        scenario.disconnect_client(client);

        assert!(!scenario.state().clients.is_alive(client));
        assert!(!scenario.state().surfaces.is_alive(first_surface));
        assert!(!scenario.state().surfaces.is_alive(second_surface));
        assert!(!scenario.state().registry.is_alive(first_window));
        assert!(!scenario.state().registry.is_alive(second_window));
        assert!(
            scenario
                .state()
                .compositor
                .workspaces
                .iter()
                .all(|workspace| {
                    let windows = workspace.window_ids();
                    !windows.contains(&first_window) && !windows.contains(&second_window)
                })
        );
        assert!(scenario.report().all_handled());
        assert!(scenario.report().all_valid());
    }

    /// 验证输出尺寸和动作请求都通过场景步骤进入核心。
    #[test]
    fn smithay_runtime_scenario_resize_and_action_work() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        scenario.resize_output(1600, 900);

        let output = scenario.state().compositor.current_output_size();
        let previous_workspace = scenario.state().compositor.current_workspace;

        scenario.request_action(Action::NextWorkspace);

        assert_eq!(output.width, 1600);
        assert_eq!(output.height, 900);
        assert_ne!(
            scenario.state().compositor.current_workspace,
            previous_workspace
        );
        assert!(scenario.report().all_handled());
        assert!(scenario.report().all_valid());
    }

    /// 验证完整诊断和状态验证步骤都会返回核心生成的文本。
    #[test]
    fn smithay_runtime_scenario_diagnostics_return_text() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        let debug_text = scenario.request_debug_text();
        let validation_text = scenario.request_validation_text();

        assert!(debug_text.contains("Sky Mirror Debug Snapshot"));
        assert!(debug_text.contains("Workspaces"));
        assert!(debug_text.contains("Windows"));
        assert!(validation_text.contains("Validation Report"));
        assert!(scenario.report().all_handled());
        assert!(scenario.report().all_valid());
    }

    /// 验证非法输出尺寸会保留无效状态，并让整体报告保持无效。
    #[test]
    fn smithay_runtime_scenario_invalid_resize_makes_report_not_all_valid() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        scenario.resize_output(0, 900);

        let validation_text = scenario.request_validation_text();

        assert!(!scenario.report().all_valid());
        assert!(validation_text.contains("InvalidOutputSize"));
    }

    /// 验证场景报告文本包含稳定标题和已执行步骤名称。
    #[test]
    fn smithay_runtime_scenario_report_pretty_print_contains_steps() {
        let mut scenario = SmithayRuntimeScenario::new_probe_only();

        let client = scenario.connect_allocated_client(Some("app".to_string()));
        scenario.create_allocated_surface(Some(client), SurfaceRole::XdgToplevel);

        let text = scenario.report().pretty_print();

        assert!(text.contains("Smithay Runtime Scenario Report"));
        assert!(text.contains("connect_allocated_client"));
        assert!(text.contains("create_allocated_surface"));
        assert!(!scenario.report().is_empty());
    }
}
