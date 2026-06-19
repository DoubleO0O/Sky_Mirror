//! 后端事件序列的纯数据回放测试器。
//!
//! 本模块是测试和调试工具，不是真实 backend，也不接入 Smithay 或 Wayland
//! 对象。它只按顺序组合现有的 `BackendEvent`、`CoreCommand`、`CommandResult`
//! 和 `ValidationReport`，用于在真实回调接入前验证完整状态链路。

use crate::core::{
    backend_event::{BackendEvent, BackendEventTranslator},
    command::{CommandResult, CoreCommand},
    state::State,
    validator::ValidationReport,
};

/// 单个后端事件回放步骤的完整记录。
///
/// 每个步骤保存原始 `BackendEvent`、翻译后的 `CoreCommand`、执行结果，
/// 以及执行后立刻生成的 `ValidationReport`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReplayStep {
    /// 原始后端事件。
    pub event: BackendEvent,

    /// 由 `BackendEventTranslator` 翻译出的核心命令。
    pub command: CoreCommand,

    /// `State` 执行命令后的结果。
    pub result: CommandResult,

    /// 命令执行后立即生成的状态验证报告。
    pub validation: ValidationReport,
}

/// 一次后端事件序列回放的完整报告。
///
/// 该报告不持有 `State` 引用，可以独立打印、测试或交给未来调试工具读取。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReplayReport {
    /// 每个事件对应的回放步骤。
    pub steps: Vec<BackendReplayStep>,
}

impl BackendReplayReport {
    /// 当前回放序列中的所有步骤是否都没有 Error。
    pub fn is_valid(&self) -> bool {
        self.steps.iter().all(|step| step.validation.is_valid())
    }

    /// 当前回放序列中的所有步骤是否完全没有 Warning 或 Error。
    pub fn is_clean(&self) -> bool {
        self.steps.iter().all(|step| step.validation.is_clean())
    }

    /// 将回放报告格式化为人类可读文本。
    ///
    /// 输出只读取已经保存的步骤快照，不重新执行命令、不验证状态，也不默认打印。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        // 头部汇总步骤数量和整体状态，便于快速判断整段回放是否可靠。
        output.push_str("Sky Mirror Backend Replay Report\n");
        output.push_str(&format!("steps: {}\n", self.steps.len()));
        output.push_str(&format!("valid: {}\n", self.is_valid()));
        output.push_str(&format!("clean: {}\n", self.is_clean()));

        // 每一步保留原始事实、翻译结果、执行结果和当时的验证快照。
        for (index, step) in self.steps.iter().enumerate() {
            output.push_str(&format!(
                "\nStep {index}:\nevent: {:?}\ncommand: {:?}\nresult: {:?}\nvalidation:\n{}",
                step.event,
                step.command,
                step.result,
                step.validation.pretty_print()
            ));
        }

        output
    }
}

/// 后端事件回放器。
///
/// `BackendEventReplayer` 不是真实 backend，也不接 Smithay。它只是把一组纯数据
/// `BackendEvent` 逐个翻译并提交给 `State`，用于验证未来真实回调接入前的核心
/// 状态链路。
pub struct BackendEventReplayer;

impl BackendEventReplayer {
    /// 回放一组后端事件。
    ///
    /// 每个事件都会按输入顺序执行：
    /// `BackendEvent -> CoreCommand -> State::handle_command_with_validation()`。
    /// 每一步执行后立即验证，但不会自动修复状态或打印报告。
    pub fn replay(
        state: &mut State,
        events: impl IntoIterator<Item = BackendEvent>,
    ) -> BackendReplayReport {
        let mut steps = Vec::new();

        for event in events {
            let command = BackendEventTranslator::translate(event.clone());

            // 每一步复用 State 的统一 seam，确保 runtime 与 replay 使用相同 post-command 时序。
            let (result, validation) = state.handle_command_with_validation(command.clone());

            steps.push(BackendReplayStep {
                event,
                command,
                result,
                validation,
            });
        }

        BackendReplayReport { steps }
    }
}

#[cfg(test)]
mod tests {
    use super::BackendEventReplayer;
    use crate::core::{
        backend_event::BackendEvent,
        client::ClientKind,
        command::{CommandResult, CoreCommand},
        state::State,
        surface::SurfaceRole,
        validator::ValidationIssueKind,
        window::WindowKind,
    };

    /// 验证空事件序列会生成有效且干净的空报告。
    #[test]
    fn backend_replay_accepts_empty_event_list() {
        let mut state = State::new();

        let report = BackendEventReplayer::replay(&mut state, Vec::new());

        // 空回放没有步骤，也没有任何验证错误或警告。
        assert!(report.steps.is_empty());
        assert!(report.is_valid());
        assert!(report.is_clean());
    }

    /// 验证 client connect/disconnect 可以通过 replay public seam 完成 clean 生命周期。
    #[test]
    fn backend_replay_runs_client_connect_disconnect_tracer() {
        let mut state = State::new();

        let report = BackendEventReplayer::replay(
            &mut state,
            vec![
                BackendEvent::ClientConnected {
                    client: Some(42),
                    kind: ClientKind::WaylandPlaceholder,
                    name: Some("replay-client".to_string()),
                },
                BackendEvent::ClientDisconnected { client: 42 },
            ],
        );

        assert_eq!(report.steps.len(), 2);

        // 每个 BackendEvent 都必须先翻译成 CoreCommand，再由 State seam 修改 registry。
        assert_eq!(
            report.steps[0].command,
            CoreCommand::RegisterClient {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("replay-client".to_string()),
            }
        );
        assert_eq!(
            report.steps[0].result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert_eq!(report.steps[1].command, CoreCommand::CloseClient(42));
        assert_eq!(
            report.steps[1].result,
            CommandResult::ClientClosed {
                client: 42,
                marked_dead: true,
                dead_surfaces: Vec::new(),
                closed_windows: Vec::new(),
                removed_from_workspace_count: 0,
                marked_window_dead_count: 0,
            }
        );

        // alive client 无 surface 以及随后保留 dead tombstone 都是合法状态。
        assert!(report.steps.iter().all(|step| step.validation.is_clean()));
        assert!(report.is_clean());
        assert!(!state.clients.is_alive(42));
        assert!(
            !state
                .debug_bundle()
                .snapshot
                .clients
                .iter()
                .find(|record| record.id == 42)
                .expect("replay 结束后必须保留 client tombstone")
                .alive
        );
    }

    /// 验证 surface 创建、map 和关闭事件可以按顺序完成完整生命周期。
    #[test]
    fn backend_replay_runs_surface_create_map_close_lifecycle() {
        let mut state = State::new();

        let report = BackendEventReplayer::replay(
            &mut state,
            vec![
                BackendEvent::SurfaceCreated {
                    surface: 42,
                    client: None,
                    role: SurfaceRole::XdgToplevel,
                },
                BackendEvent::ToplevelMapped {
                    surface: 42,
                    title: "Terminal".to_string(),
                    app_id: Some("foot".to_string()),
                    kind: WindowKind::WaylandPlaceholder,
                },
                BackendEvent::SurfaceClosed { surface: 42 },
            ],
        );

        // 三个输入事件必须严格对应三个回放步骤，并保持状态始终有效。
        assert_eq!(report.steps.len(), 3);
        assert!(report.is_valid());

        let CommandResult::WindowRegisteredForSurface { window, bound, .. } =
            &report.steps[1].result
        else {
            panic!("map 步骤必须返回 WindowRegisteredForSurface");
        };

        // map 必须成功建立绑定，关闭后 surface 与窗口都保留诊断记录但标记为 dead。
        assert!(*bound);
        assert!(!state.surfaces.get(42).expect("surface 记录必须保留").alive);
        assert!(!state.registry.is_alive(*window));

        // 完成关闭后，任何 workspace 都不得继续引用该窗口。
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(window))
        );
    }

    /// 验证输出尺寸事件会通过现有 Action 链路更新集中状态。
    #[test]
    fn backend_replay_applies_output_resize_event() {
        let mut state = State::new();

        let report = BackendEventReplayer::replay(
            &mut state,
            vec![BackendEvent::OutputResized {
                width: 1366,
                height: 768,
            }],
        );

        // 合法尺寸变化执行后必须保持核心状态有效。
        assert!(report.is_valid());

        let output = state.compositor.current_output_size();

        // 回放结果必须保留事件提供的宽高。
        assert_eq!(output.width, 1366);
        assert_eq!(output.height, 768);
    }

    /// 验证人类可读报告包含步骤四层信息和验证报告正文。
    #[test]
    fn backend_replay_pretty_print_contains_steps_and_validation() {
        let mut state = State::new();
        let report = BackendEventReplayer::replay(&mut state, vec![BackendEvent::DebugRequested]);

        let text = report.pretty_print();

        // 文本必须包含整体标题、步骤序号和每一步的完整边界信息。
        assert!(text.contains("Sky Mirror Backend Replay Report"));
        assert!(text.contains("steps:"));
        assert!(text.contains("Step 0"));
        assert!(text.contains("event:"));
        assert!(text.contains("command:"));
        assert!(text.contains("result:"));
        assert!(text.contains("validation:"));

        // 每一步保存的 ValidationReport 必须使用现有格式输出。
        assert!(text.contains("Sky Mirror Validation Report"));
    }

    /// 验证回放会记录命令执行后已经存在的状态不变量错误。
    #[test]
    fn backend_replay_records_validation_issue_after_invalid_event() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);

        // 测试直接构造损坏绑定，用于确认回放器记录错误而不是自动修复。
        assert!(state.surfaces.bind_window(surface, 999));

        let report =
            BackendEventReplayer::replay(&mut state, vec![BackendEvent::ValidateRequested]);

        // 单个验证请求必须产生一步报告，并保留状态无效结论。
        assert_eq!(report.steps.len(), 1);
        assert!(!report.is_valid());

        // 验证快照必须包含 surface 引用缺失窗口的稳定问题类型。
        assert!(
            report.steps[0]
                .validation
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesMissingWindow })
        );
    }
}
