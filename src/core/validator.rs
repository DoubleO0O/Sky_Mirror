//! compositor 核心状态的只读一致性检查器。
//!
//! Validator 只检查 State 当前是否满足 workspace、slot、stack、focus、output
//! 和 WindowRegistry 的核心不变量。它不会修复错误，也不会修改任何运行状态。

use std::collections::HashMap;

use crate::core::{
    state::State,
    workspace::{SlotContent, WindowId},
};

/// 状态验证问题的严重程度。
///
/// 当前阶段只区分 Warning 和 Error：
/// Warning 表示状态可疑但不一定已经破坏运行；
/// Error 表示核心不变量已经被破坏。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// 可疑状态，通常用于提示未来可能需要清理。
    Warning,

    /// 严重状态错误，说明内部模型已经不一致。
    Error,
}

/// 状态验证问题类型。
///
/// 每个类型对应一个明确的不变量检查，便于测试和未来 debug UI 分类展示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationIssueKind {
    /// 当前 workspace ID 不存在于 workspace 列表中。
    MissingCurrentWorkspace,

    /// FocusState.workspace 与 CompositorState.current_workspace 不一致。
    FocusWorkspaceMismatch,

    /// FocusState.slot 超出固定 slot 范围。
    FocusSlotOutOfRange,

    /// FocusState.window 指向的窗口不在当前 workspace 中。
    FocusWindowNotInCurrentWorkspace,

    /// workspace 中引用了 registry 不存在的窗口。
    WorkspaceReferencesMissingRegistryWindow,

    /// workspace 中引用了 registry 中已经 dead 的窗口。
    WorkspaceReferencesDeadWindow,

    /// surface 绑定了 registry 中不存在的窗口。
    SurfaceReferencesMissingWindow,

    /// 存活 surface 绑定了 registry 中已经 dead 的窗口。
    SurfaceReferencesDeadWindow,

    /// surface 绑定了不存在的 client。
    SurfaceReferencesMissingClient,

    /// 存活 surface 绑定了已经 dead 的 client。
    SurfaceReferencesDeadClient,

    /// 同一个 WindowId 被多个 slot 或多个 workspace 重复引用。
    DuplicateWindowReference,

    /// Stack 的 active 索引越界。
    StackActiveIndexOutOfRange,

    /// Slot ID 与它在固定数组中的位置不一致。
    SlotIdMismatch,

    /// 输出尺寸为 0。
    InvalidOutputSize,

    /// registry 的 next_id 没有大于已经存在的最大 WindowId。
    RegistryNextIdNotGreaterThanKnownWindow,

    /// ClientRegistry 的 next_id 不大于已有最大 client ID。
    ClientRegistryNextIdNotGreaterThanKnownClient,
}

/// 单条状态验证问题。
///
/// 该结构只描述问题，不尝试修复问题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// 问题严重程度。
    pub severity: ValidationSeverity,

    /// 问题类型。
    pub kind: ValidationIssueKind,

    /// 人类可读的中文说明。
    pub message: String,
}

impl ValidationIssue {
    /// 创建一条 Error 级别问题。
    pub fn error(kind: ValidationIssueKind, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            kind,
            message: message.into(),
        }
    }

    /// 创建一条 Warning 级别问题。
    pub fn warning(kind: ValidationIssueKind, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            kind,
            message: message.into(),
        }
    }
}

/// 状态验证报告。
///
/// ValidationReport 是一次只读检查的结果快照，不持有 State 引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// 检查发现的所有问题。
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// 创建空报告。
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    /// 添加一条问题。
    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }

    /// 当前报告是否没有任何 Error。
    ///
    /// Warning 不会让该方法返回 false。
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }

    /// 当前报告是否完全没有问题。
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// 将验证报告格式化为人类可读文本。
    ///
    /// 输出保持问题收集顺序，不访问 State，也不会修改报告内容。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        // 头部先给出整体有效性和问题数量，便于日志中快速定位异常报告。
        output.push_str("Sky Mirror Validation Report\n");
        output.push_str(&format!("valid: {}\n", self.is_valid()));
        output.push_str(&format!("issues: {}\n", self.issues.len()));

        // 每条问题保留严重程度、稳定类型和中文说明，便于测试及未来界面分类。
        for issue in &self.issues {
            output.push_str(&format!(
                "- {:?} {:?}: {}\n",
                issue.severity, issue.kind, issue.message
            ));
        }

        output
    }
}

/// 只读状态一致性检查器。
///
/// StateValidator 不修复状态，不修改 State，只根据当前状态报告问题。
pub struct StateValidator;

impl StateValidator {
    /// 检查 State 是否满足核心不变量。
    ///
    /// 验证过程只读取集中状态和 registry，不触发焦点刷新、窗口分配或其他副作用。
    pub fn validate(state: &State) -> ValidationReport {
        let mut report = ValidationReport::new();
        let compositor = &state.compositor;

        // current_workspace 必须能解析到真实 workspace，否则后续焦点和布局都无从读取。
        let current_workspace = compositor
            .workspaces
            .iter()
            .find(|workspace| workspace.id == compositor.current_workspace);
        if current_workspace.is_none() {
            report.push(ValidationIssue::error(
                ValidationIssueKind::MissingCurrentWorkspace,
                format!(
                    "当前 workspace ID {} 不存在于 workspace 列表中",
                    compositor.current_workspace
                ),
            ));
        }

        // focus.workspace 必须跟随 current_workspace，否则焦点层级指向了其他工作区。
        if compositor.focus.workspace != compositor.current_workspace {
            report.push(ValidationIssue::error(
                ValidationIssueKind::FocusWorkspaceMismatch,
                format!(
                    "焦点 workspace {} 与当前 workspace {} 不一致",
                    compositor.focus.workspace, compositor.current_workspace
                ),
            ));
        }

        // 固定 slot 模型只有 0..=3，越界值无法被 Workspace 正确解析。
        if compositor.focus.slot >= 4 {
            report.push(ValidationIssue::error(
                ValidationIssueKind::FocusSlotOutOfRange,
                format!("焦点 slot {} 超出固定范围 0..=3", compositor.focus.slot),
            ));
        }

        // 零宽或零高输出无法生成有效窗口几何，应在进入布局前暴露为错误。
        let output_size = compositor.current_output_size();
        if output_size.width == 0 || output_size.height == 0 {
            report.push(ValidationIssue::error(
                ValidationIssueKind::InvalidOutputSize,
                format!(
                    "输出尺寸 {}x{} 包含零值",
                    output_size.width, output_size.height
                ),
            ));
        }

        let mut references = HashMap::<WindowId, Vec<String>>::new();

        for workspace in &compositor.workspaces {
            for (index, slot) in workspace.slots.iter().enumerate() {
                // slot ID 必须与固定数组位置一致，否则按 ID 查找和按索引布局会产生分歧。
                if slot.id != index as u8 {
                    report.push(ValidationIssue::error(
                        ValidationIssueKind::SlotIdMismatch,
                        format!(
                            "workspace {} 的数组位置 {} 使用了错误 slot ID {}",
                            workspace.id, index, slot.id
                        ),
                    ));
                }

                let referenced_windows = match &slot.content {
                    // Empty 不引用窗口，因此没有 registry 或重复位置需要检查。
                    SlotContent::Empty => Vec::new(),
                    // Single 只记录一个没有 stack 索引的窗口位置。
                    SlotContent::Single(window) => vec![(*window, None)],
                    SlotContent::Stack(stack) => {
                        // Stack 必须至少包含一个窗口，且 active 必须指向合法元素。
                        if stack.windows.is_empty() || stack.active >= stack.windows.len() {
                            report.push(ValidationIssue::error(
                                ValidationIssueKind::StackActiveIndexOutOfRange,
                                format!(
                                    "workspace {} slot {} 的 stack active={}，窗口数量={}",
                                    workspace.id,
                                    slot.id,
                                    stack.active,
                                    stack.windows.len()
                                ),
                            ));
                        }

                        // Stack 中每个窗口都要作为独立引用检查，而不只检查 active window。
                        stack
                            .windows
                            .iter()
                            .copied()
                            .enumerate()
                            .map(|(stack_index, window)| (window, Some(stack_index)))
                            .collect()
                    }
                };

                for (window, stack_index) in referenced_windows {
                    let location = match stack_index {
                        Some(stack_index) => format!(
                            "workspace={} slot={} stack_index={}",
                            workspace.id, slot.id, stack_index
                        ),
                        None => format!("workspace={} slot={}", workspace.id, slot.id),
                    };
                    references.entry(window).or_default().push(location.clone());

                    // 每个 workspace 窗口引用都必须具有 registry metadata。
                    let Some(record) = state.registry.get(window) else {
                        report.push(ValidationIssue::error(
                            ValidationIssueKind::WorkspaceReferencesMissingRegistryWindow,
                            format!(
                                "窗口 {} 在 {} 被引用，但 registry 中不存在",
                                window, location
                            ),
                        ));
                        continue;
                    };

                    // 可见 workspace 不应继续引用已标记 dead 的窗口。
                    if !record.alive {
                        report.push(ValidationIssue::error(
                            ValidationIssueKind::WorkspaceReferencesDeadWindow,
                            format!(
                                "窗口 {} 在 {} 被引用，但 registry 已标记为 dead",
                                window, location
                            ),
                        ));
                    }
                }
            }
        }

        // 同一 WindowId 只能属于一个位置，否则关闭、焦点和布局都会产生歧义。
        let mut duplicate_references: Vec<_> = references
            .iter()
            .filter(|(_, locations)| locations.len() > 1)
            .collect();
        duplicate_references.sort_by_key(|(window, _)| **window);
        for (window, locations) in duplicate_references {
            report.push(ValidationIssue::error(
                ValidationIssueKind::DuplicateWindowReference,
                format!("窗口 {} 被重复引用于 {}", window, locations.join(", ")),
            ));
        }

        // ClientId、SurfaceId 与 WindowId 是独立命名空间，分别检查显式归属和窗口绑定。
        for surface in state.surfaces.records() {
            if let Some(client) = surface.client {
                match state.clients.get(client) {
                    None => report.push(ValidationIssue::error(
                        ValidationIssueKind::SurfaceReferencesMissingClient,
                        format!(
                            "surface {} 绑定 client {}，但 client registry 中不存在",
                            surface.id, client
                        ),
                    )),
                    Some(record) if surface.alive && !record.alive => {
                        report.push(ValidationIssue::error(
                            ValidationIssueKind::SurfaceReferencesDeadClient,
                            format!(
                                "存活 surface {} 绑定了已经 dead 的 client {}",
                                surface.id, client
                            ),
                        ));
                    }
                    Some(_) => {}
                }
            }

            let Some(window) = surface.window else {
                // 未绑定 surface 可能尚未完成角色/map 流程，不属于错误状态。
                continue;
            };

            let Some(record) = state.registry.get(window) else {
                // 任何 surface 绑定都必须指向已注册的逻辑窗口。
                report.push(ValidationIssue::error(
                    ValidationIssueKind::SurfaceReferencesMissingWindow,
                    format!(
                        "surface {} 绑定窗口 {}，但 registry 中不存在该窗口",
                        surface.id, window
                    ),
                ));
                continue;
            };

            // dead surface 可以保留对 dead 窗口的历史绑定，只有 alive surface 才要求窗口存活。
            if surface.alive && !record.alive {
                report.push(ValidationIssue::error(
                    ValidationIssueKind::SurfaceReferencesDeadWindow,
                    format!(
                        "存活 surface {} 绑定了已经 dead 的窗口 {}",
                        surface.id, window
                    ),
                ));
            }
        }

        // focus.window 必须属于当前 workspace，避免焦点指向其他工作区或已移除窗口。
        if let (Some(workspace), Some(window)) = (current_workspace, compositor.focus.window) {
            if !workspace.window_ids().contains(&window) {
                report.push(ValidationIssue::error(
                    ValidationIssueKind::FocusWindowNotInCurrentWorkspace,
                    format!("焦点窗口 {} 不属于当前 workspace {}", window, workspace.id),
                ));
            }
        }

        // next_id 必须大于所有已知窗口 ID，否则下一次创建可能覆盖已有逻辑窗口。
        if let Some(max_window_id) = state
            .registry
            .records()
            .iter()
            .map(|record| record.id)
            .max()
        {
            if state.registry.next_id() <= max_window_id {
                report.push(ValidationIssue::warning(
                    ValidationIssueKind::RegistryNextIdNotGreaterThanKnownWindow,
                    format!(
                        "registry next_id={} 没有大于最大窗口 ID {}",
                        state.registry.next_id(),
                        max_window_id
                    ),
                ));
            }
        }

        // client next_id 必须大于所有已知 ClientId，避免未来连接注册覆盖已有记录。
        let max_client_id = state.clients.records().iter().map(|client| client.id).max();

        if let Some(max_client_id) = max_client_id {
            if state.clients.next_id() <= max_client_id {
                report.push(ValidationIssue::warning(
                    ValidationIssueKind::ClientRegistryNextIdNotGreaterThanKnownClient,
                    format!(
                        "client registry next_id={} 没有大于最大 client ID {}",
                        state.clients.next_id(),
                        max_client_id
                    ),
                ));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::{ValidationIssueKind, ValidationSeverity};
    use crate::core::{
        client::ClientKind,
        layout::OutputSize,
        state::State,
        surface::SurfaceRole,
        workspace::{SlotContent, Stack},
    };

    /// 验证默认集中状态满足全部核心不变量。
    #[test]
    fn validator_accepts_default_state() {
        let state = State::new();

        let report = state.validate();

        // 默认状态不得包含 Error。
        assert!(report.is_valid());

        // 默认状态也不应产生任何 Warning。
        assert!(report.is_clean());
    }

    /// 验证 current_workspace 无法解析时会报告明确错误。
    #[test]
    fn validator_reports_missing_current_workspace() {
        let mut state = State::new();
        state.compositor.current_workspace = 999;

        let report = state.validate();

        // 缺失当前 workspace 会破坏布局和焦点入口，因此报告必须无效。
        assert!(!report.is_valid());

        // 报告必须包含对应的稳定问题类型。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::MissingCurrentWorkspace })
        );
    }

    /// 验证 focus.workspace 与当前 workspace 不一致时会报告错误。
    #[test]
    fn validator_reports_focus_workspace_mismatch() {
        let mut state = State::new();
        state.compositor.focus.workspace = 1;

        let report = state.validate();

        // 焦点根层级不一致必须使报告无效。
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::FocusWorkspaceMismatch })
        );
    }

    /// 验证 focus.slot 超出固定四 slot 范围时会报告错误。
    #[test]
    fn validator_reports_focus_slot_out_of_range() {
        let mut state = State::new();
        state.compositor.focus.slot = 4;

        let report = state.validate();

        // slot 4 无法在固定数组中解析，必须报告越界。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::FocusSlotOutOfRange })
        );
    }

    /// 验证 workspace 引用 registry 不存在的窗口时会报告错误。
    #[test]
    fn validator_reports_missing_registry_window() {
        let mut state = State::new();
        state
            .compositor
            .current_workspace_mut()
            .expect("默认当前 workspace 必须存在")
            .assign_window(999);

        let report = state.validate();

        // 未注册窗口无法提供生命周期和 metadata，必须报告缺失。
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ValidationIssueKind::WorkspaceReferencesMissingRegistryWindow
        }));
    }

    /// 验证 workspace 仍引用 dead 窗口时会报告错误。
    #[test]
    fn validator_reports_dead_window_reference() {
        let mut state = State::new();
        let window = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");
        assert!(state.registry.mark_dead(window));

        let report = state.validate();

        // dead metadata 与可见 workspace 引用不能同时存在。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::WorkspaceReferencesDeadWindow })
        );
    }

    /// 验证同一个窗口出现在多个 slot 时会报告重复引用。
    #[test]
    fn validator_reports_duplicate_window_reference() {
        let mut state = State::new();
        let window = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");
        state.compositor.workspaces[0].slots[3].content = SlotContent::Single(window);

        let report = state.validate();

        // 一个 WindowId 只能有一个 workspace 位置。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::DuplicateWindowReference })
        );
    }

    /// 验证报告文本会展示总体状态和具体问题类型。
    #[test]
    fn validation_report_pretty_print_includes_issues() {
        let mut state = State::new();
        state.compositor.current_workspace = 999;

        let text = state.validate().pretty_print();

        // 报告必须具有稳定标题。
        assert!(text.contains("Sky Mirror Validation Report"));

        // Error 存在时总体状态必须显示为 false。
        assert!(text.contains("valid: false"));

        // 具体问题类型必须出现在文本中，便于日志检索。
        assert!(text.contains("MissingCurrentWorkspace"));
    }

    /// 验证零尺寸输出会被识别为无法布局的错误状态。
    #[test]
    fn validator_reports_invalid_output_size() {
        let mut state = State::new();
        state.compositor.resize_output(OutputSize {
            width: 0,
            height: 1080,
        });

        let report = state.validate();

        // 任一维度为零都必须报告 InvalidOutputSize。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == ValidationIssueKind::InvalidOutputSize)
        );
    }

    /// 验证 slot ID 与固定数组位置不一致时会报告错误。
    #[test]
    fn validator_reports_slot_id_mismatch() {
        let mut state = State::new();
        state.compositor.workspaces[0].slots[2].id = 9;

        let report = state.validate();

        // ID 与索引分歧会破坏按 ID 导航，因此必须报告错误。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == ValidationIssueKind::SlotIdMismatch)
        );
    }

    /// 验证 Stack active 索引越界时会报告错误。
    #[test]
    fn validator_reports_stack_active_index_out_of_range() {
        let mut state = State::new();
        let window = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");
        state.compositor.workspaces[0].slots[0].content = SlotContent::Stack(Stack {
            windows: vec![window],
            active: 1,
        });

        let report = state.validate();

        // active 等于窗口数量时已经越过最后一个合法索引。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::StackActiveIndexOutOfRange })
        );
    }

    /// 验证焦点窗口不属于当前 workspace 时会报告错误。
    #[test]
    fn validator_reports_focus_window_not_in_current_workspace() {
        let mut state = State::new();
        state.compositor.focus.window = Some(999);

        let report = state.validate();

        // 非当前 workspace 窗口不能继续作为焦点。
        assert!(
            report.issues.iter().any(|issue| {
                issue.kind == ValidationIssueKind::FocusWindowNotInCurrentWorkspace
            })
        );
    }

    /// 验证 next_id 未超过最大已知窗口 ID 时只产生 Warning。
    #[test]
    fn validator_warns_when_registry_next_id_can_conflict() {
        let mut state = State::new();
        state.registry.set_next_id(1);

        let report = state.validate();

        let issue = report
            .issues
            .iter()
            .find(|issue| {
                issue.kind == ValidationIssueKind::RegistryNextIdNotGreaterThanKnownWindow
            })
            .expect("next_id 冲突风险必须产生验证问题");

        // next_id 风险当前定义为 Warning，不应让报告整体无效。
        assert_eq!(issue.severity, ValidationSeverity::Warning);
        assert!(report.is_valid());

        // Warning 仍然属于问题，因此报告不能被视为完全 clean。
        assert!(!report.is_clean());
    }

    /// 验证 surface 绑定不存在的窗口时会报告错误。
    #[test]
    fn validator_reports_surface_references_missing_window() {
        let mut state = State::new();
        state
            .surfaces
            .register_for_window(999, SurfaceRole::XdgToplevel);

        let report = state.validate();

        // Surface 绑定必须指向 WindowRegistry 中已有的逻辑窗口。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesMissingWindow })
        );
    }

    /// 验证存活 surface 绑定 dead 窗口时会报告错误。
    #[test]
    fn validator_reports_alive_surface_references_dead_window() {
        let mut state = State::new();
        let window = state
            .compositor
            .focus
            .window
            .expect("默认状态必须包含焦点窗口");
        state
            .surfaces
            .register_for_window(window, SurfaceRole::XdgToplevel);
        assert!(state.registry.mark_dead(window));

        let report = state.validate();

        // alive surface 不能继续绑定 registry 中已经 dead 的窗口。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesDeadWindow })
        );
    }

    /// 验证存活但尚未绑定窗口的 surface 是合法生命周期中间状态。
    #[test]
    fn validator_accepts_unbound_alive_surface() {
        let mut state = State::new();
        state.register_surface(SurfaceRole::Unknown);

        let report = state.validate();

        // Wayland surface 在 map 前可以没有 WindowId，不应产生验证错误。
        assert!(report.is_valid());
    }

    /// 验证存活 client 尚未创建 surface 时仍然是合法中间状态。
    #[test]
    fn validator_accepts_alive_client_without_surface() {
        let mut state = State::new();
        let client = state.register_client(
            ClientKind::WaylandPlaceholder,
            Some("测试 client".to_string()),
        );

        let report = state.validate();

        // client socket 连接不等于 surface 或 window，未创建 surface 不应产生错误。
        assert!(state.clients.is_alive(client));
        assert!(state.surfaces.records().is_empty());
        assert!(report.is_valid());
        assert!(report.is_clean());
    }

    /// 验证存活 surface 归属于存活 client 时状态有效。
    #[test]
    fn validator_accepts_surface_owned_by_alive_client() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        state.register_surface_for_client(Some(client), SurfaceRole::XdgToplevel);

        let report = state.validate();

        // 合法归属不要求 surface 已经创建 WindowId。
        assert!(report.is_valid());
        assert!(report.is_clean());
    }

    /// 验证 surface 指向不存在的 client 时会报告错误。
    #[test]
    fn validator_reports_surface_references_missing_client() {
        let mut state = State::new();
        state.register_surface_for_client(Some(999), SurfaceRole::Unknown);

        let report = state.validate();

        // 显式 owner 必须能在 ClientRegistry 中解析。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesMissingClient })
        );
        assert!(!report.is_valid());
    }

    /// 验证存活 surface 指向 dead client 时会报告错误。
    #[test]
    fn validator_reports_alive_surface_references_dead_client() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        state.register_surface_for_client(Some(client), SurfaceRole::XdgToplevel);

        // 直接损坏 ClientRegistry，绕过正常 close_client 级联，用于验证错误检测。
        assert!(state.clients.mark_dead(client));

        let report = state.validate();

        // 存活 surface 指向 dead client 时，Validator 必须暴露悬空归属。
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesDeadClient })
        );
        assert!(!report.is_valid());
    }

    /// 验证 dead surface 可以保留对 dead client 的历史归属。
    #[test]
    fn validator_accepts_dead_surface_references_dead_client() {
        let mut state = State::new();
        let client = state.register_client(ClientKind::WaylandPlaceholder, None);
        let surface = state.register_surface_for_client(Some(client), SurfaceRole::XdgToplevel);
        assert!(state.surfaces.mark_dead(surface));
        assert!(state.close_client(client).marked_dead);

        let report = state.validate();

        // 两侧都结束生命周期后，归属关系只作为诊断历史保留，不应产生错误。
        assert!(report.is_valid());
        assert!(report.is_clean());
    }

    /// 验证正常 client 断开级联完成后满足全部状态不变量。
    #[test]
    fn validator_accepts_client_disconnect_cascade() {
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

        let result = state.close_client(client);
        let report = state.validate();

        // 正常级联必须同时结束三层生命周期，并清理 workspace 窗口引用。
        assert!(result.marked_dead);
        assert_eq!(result.dead_surfaces, vec![surface]);
        assert_eq!(result.closed_windows, vec![mapped.window]);
        assert!(!state.clients.is_alive(client));
        assert!(!state.surfaces.is_alive(surface));
        assert!(!state.registry.is_alive(mapped.window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&mapped.window))
        );

        // dead client、dead surface 和 dead window 的历史关系是合法诊断状态。
        assert!(report.is_valid());
        assert!(report.is_clean());
    }
}
