//! compositor 的统一调试诊断包。
//!
//! diagnostics 模块只负责组合 Inspector 与 Validator 的输出。
//! Inspector 负责生成状态快照，Validator 负责检查不变量；
//! DebugBundle 把二者合并成一个可打印、可测试、未来可通过 IPC 暴露的只读诊断结果。
//! 该组合职责独立于 inspector 和 validator，避免任一模块反向承担对方的职责。

use crate::core::{
    inspector::{Inspector, SystemDebugSnapshot},
    state::State,
    validator::{StateValidator, ValidationReport},
};

/// compositor 当前状态的一次完整诊断结果。
///
/// DebugBundle 是只读数据，不持有 State 引用，也不会修改 workspace、focus、registry、
/// output 或 session。它把“当前状态是什么”和“当前状态是否有效”放在同一个结构中，
/// 方便测试、日志和未来 debug IPC 直接消费。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugBundle {
    /// 当前 compositor 状态快照。
    pub snapshot: SystemDebugSnapshot,

    /// 当前 compositor 状态验证报告。
    pub validation: ValidationReport,
}

impl DebugBundle {
    /// 从 State 生成完整诊断包。
    ///
    /// 该方法只读访问 State：先通过 Inspector 生成快照，再通过 StateValidator
    /// 生成验证报告。两个结果都不持有 State 引用，因此可以安全打印或传递。
    pub fn from_state(state: &State) -> Self {
        Self {
            snapshot: Inspector::snapshot(state),
            validation: StateValidator::validate(state),
        }
    }

    /// 当前诊断包是否没有 Error 级别问题。
    ///
    /// Warning 不会让该方法返回 false。
    pub fn is_valid(&self) -> bool {
        self.validation.is_valid()
    }

    /// 当前诊断包是否完全没有 Warning 或 Error。
    pub fn is_clean(&self) -> bool {
        self.validation.is_clean()
    }

    /// 将完整诊断包格式化为人类可读文本。
    ///
    /// 输出先展示状态快照，再展示验证报告，方便先看当前状态，
    /// 再检查是否存在不变量错误。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        // 快照部分回答当前状态是什么。
        output.push_str(&self.snapshot.pretty_print());

        // 用空行分隔两个调试区段，避免日志内容粘连。
        output.push('\n');

        // 验证部分回答当前状态是否满足核心不变量。
        output.push_str(&self.validation.pretty_print());

        output
    }
}

#[cfg(test)]
mod tests {
    use super::DebugBundle;
    use crate::core::state::State;

    /// 验证默认状态生成的诊断包有效且完全没有问题。
    #[test]
    fn debug_bundle_accepts_default_state() {
        let state = State::new();

        let bundle = state.debug_bundle();

        // 默认状态不包含 Error，因此诊断包必须有效。
        assert!(bundle.is_valid());

        // 默认状态也不包含 Warning，因此诊断包必须完全干净。
        assert!(bundle.is_clean());

        // 诊断快照中的当前 workspace 必须与 State 保持一致。
        assert_eq!(
            bundle.snapshot.current_workspace,
            state.compositor.current_workspace
        );

        // 干净诊断包不应包含任何验证问题。
        assert!(bundle.validation.issues.is_empty());
    }

    /// 验证诊断文本同时包含状态快照与验证报告。
    #[test]
    fn debug_bundle_pretty_print_contains_snapshot_and_validation() {
        let state = State::new();

        let text = state.debug_bundle().pretty_print();

        // 文本前半部分必须包含 Inspector 的快照标题和主体区段。
        assert!(text.contains("Sky Mirror Debug Snapshot"));
        assert!(text.contains("Workspaces:"));
        assert!(text.contains("Windows:"));

        // 文本后半部分必须包含 Validator 的报告标题和干净状态。
        assert!(text.contains("Sky Mirror Validation Report"));
        assert!(text.contains("valid: true"));
        assert!(text.contains("issues: 0"));
    }

    /// 验证状态损坏时诊断包无效，但仍完整保留状态快照。
    #[test]
    fn debug_bundle_reports_invalid_state_while_preserving_snapshot() {
        let mut state = State::new();
        state.compositor.current_workspace = 999;

        let bundle = state.debug_bundle();
        let text = bundle.pretty_print();

        // 缺失当前 workspace 是 Error，诊断包必须无效。
        assert!(!bundle.is_valid());

        // 即使状态无效，快照仍必须忠实记录损坏后的 current_workspace。
        assert!(text.contains("current_workspace: 999"));

        // 验证报告必须包含稳定问题类型和无效标志。
        assert!(text.contains("MissingCurrentWorkspace"));
        assert!(text.contains("valid: false"));
    }

    /// 验证 State helper 与直接构建 DebugBundle 得到相同结果。
    #[test]
    fn state_debug_bundle_matches_direct_debug_bundle() {
        let state = State::new();

        let from_state = state.debug_bundle();
        let direct = DebugBundle::from_state(&state);

        // State helper 只做委托，不得改变快照或验证报告内容。
        assert_eq!(from_state, direct);
    }

    /// 验证 State 的文本便捷入口与 DebugBundle 的格式化结果完全一致。
    #[test]
    fn state_debug_bundle_text_matches_bundle_pretty_print() {
        let state = State::new();

        // 该入口不得引入新语义，只能复用 DebugBundle::pretty_print()。
        assert_eq!(
            state.debug_bundle_text(),
            state.debug_bundle().pretty_print()
        );
    }
}
