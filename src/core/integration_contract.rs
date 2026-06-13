//! 未来真实 backend 或 Smithay 接入核心状态时必须遵守的边界契约。
//!
//! 本模块只用纯数据描述允许入口、禁止直接访问的内部区域和只读入口。
//! 它不持有或修改 `State`，不接入 Smithay，也不引入任何 Wayland 类型。
//! 后续真实 backend 应通过这里描述的稳定入口进入核心，而不是直接修改内部字段。

/// 外部系统允许使用的核心入口。
///
/// 这些入口构成未来 Smithay、Wayland 和 backend 接入核心状态的稳定边界。
/// 本枚举只描述架构契约，不执行命令，也不修改 `State`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreEntryPoint {
    /// 后端事件翻译入口。
    ///
    /// 外部事实应先表达为 `BackendEvent`，再通过 `BackendEventTranslator`
    /// 转换为 `CoreCommand`。
    BackendEventTranslator,

    /// 核心命令处理入口。
    ///
    /// 所有真实后端命令最终应通过 `State::handle_command()` 修改核心状态。
    StateHandleCommand,

    /// 用户意图分发入口。
    ///
    /// 输入系统可以继续通过 `Action` 进入 `State::dispatch_action()`。
    StateDispatchAction,

    /// 后端事件回放入口。
    ///
    /// 该入口会修改传入的测试状态，只用于测试和调试，不是真实 backend。
    BackendEventReplayer,

    /// 只读诊断入口。
    ///
    /// 外部调试工具可以通过 `State::debug_bundle()` 或
    /// `State::debug_bundle_text()` 读取状态。
    DebugBundle,

    /// 只读验证入口。
    ///
    /// 外部调试工具可以通过 `State::validate()` 检查核心不变量。
    Validator,
}

/// 核心接入契约报告。
///
/// 该报告只描述当前核心边界，不持有 `State` 引用，也不会修改任何状态。
/// 未来 Smithay 接入代码可以读取该报告确认允许入口，但不能借此绕过命令边界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreContractReport {
    /// 允许外部系统使用的入口列表。
    pub allowed_entry_points: Vec<CoreEntryPoint>,

    /// 明确禁止真实 backend 直接操作的内部区域。
    pub forbidden_direct_access: Vec<&'static str>,

    /// 需要保持只读语义的诊断入口。
    pub readonly_entry_points: Vec<CoreEntryPoint>,
}

impl CoreContractReport {
    /// 判断报告是否包含指定入口。
    pub fn allows(&self, entry: CoreEntryPoint) -> bool {
        self.allowed_entry_points.contains(&entry)
    }

    /// 判断指定入口是否必须保持只读。
    pub fn is_readonly(&self, entry: CoreEntryPoint) -> bool {
        self.readonly_entry_points.contains(&entry)
    }

    /// 输出人类可读的核心接入契约说明。
    ///
    /// 该方法只格式化报告已有数据，不读取 `State`，也不会默认打印文本。
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        output.push_str("Sky Mirror Core Integration Contract\n");
        output.push_str("Allowed entry points:\n");
        for entry in &self.allowed_entry_points {
            output.push_str(&format!("- {entry:?}\n"));
        }

        output.push_str("\nForbidden direct access:\n");
        for path in &self.forbidden_direct_access {
            output.push_str(&format!("- {path}\n"));
        }

        output.push_str("\nReadonly entry points:\n");
        for entry in &self.readonly_entry_points {
            output.push_str(&format!("- {entry:?}\n"));
        }

        output
    }
}

/// 核心接入契约描述器。
///
/// 该类型不持有状态，只返回当前架构规定的接入边界。
/// 本阶段不接 Smithay 或 Wayland，只冻结未来真实 backend 必须遵守的核心入口。
pub struct CoreIntegrationContract;

impl CoreIntegrationContract {
    /// 返回当前核心接入契约。
    ///
    /// 未来接入 Smithay 时，应优先遵守这里列出的入口和禁止访问区域。
    /// 返回值是独立纯数据报告，不读取或修改 `State`。
    pub fn describe() -> CoreContractReport {
        CoreContractReport {
            allowed_entry_points: vec![
                CoreEntryPoint::BackendEventTranslator,
                CoreEntryPoint::StateHandleCommand,
                CoreEntryPoint::StateDispatchAction,
                CoreEntryPoint::BackendEventReplayer,
                CoreEntryPoint::DebugBundle,
                CoreEntryPoint::Validator,
            ],
            forbidden_direct_access: vec![
                "State.compositor.workspaces",
                "State.registry",
                "State.surfaces",
                "CompositorState.focus",
                "CompositorState.output",
            ],
            readonly_entry_points: vec![CoreEntryPoint::DebugBundle, CoreEntryPoint::Validator],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreEntryPoint, CoreIntegrationContract};

    /// 验证契约包含未来 backend 接入所需的全部稳定入口。
    #[test]
    fn integration_contract_contains_required_entry_points() {
        let report = CoreIntegrationContract::describe();

        // 外部事实必须能够通过后端事件翻译器进入核心命令边界。
        assert!(report.allows(CoreEntryPoint::BackendEventTranslator));

        // 核心命令和用户意图必须保留各自明确的 State 入口。
        assert!(report.allows(CoreEntryPoint::StateHandleCommand));
        assert!(report.allows(CoreEntryPoint::StateDispatchAction));

        // 测试回放、诊断和验证入口必须继续作为受支持边界。
        assert!(report.allows(CoreEntryPoint::BackendEventReplayer));
        assert!(report.allows(CoreEntryPoint::DebugBundle));
        assert!(report.allows(CoreEntryPoint::Validator));
    }

    /// 验证诊断与验证入口标记为只读，而命令处理入口不是只读。
    #[test]
    fn integration_contract_marks_diagnostics_readonly() {
        let report = CoreIntegrationContract::describe();

        // 诊断和验证只能观察状态，不得产生自动修复或业务修改。
        assert!(report.is_readonly(CoreEntryPoint::DebugBundle));
        assert!(report.is_readonly(CoreEntryPoint::Validator));

        // 命令处理入口负责执行状态变化，因此不能错误标记为只读。
        assert!(!report.is_readonly(CoreEntryPoint::StateHandleCommand));

        // 回放器会修改传入的测试状态，因此也不属于只读入口。
        assert!(!report.is_readonly(CoreEntryPoint::BackendEventReplayer));
    }

    /// 验证契约明确列出真实 backend 禁止直接访问的核心内部区域。
    #[test]
    fn integration_contract_lists_forbidden_direct_access() {
        let report = CoreIntegrationContract::describe();

        // workspace、窗口注册表和 surface 注册表必须由核心状态入口协调修改。
        assert!(
            report
                .forbidden_direct_access
                .contains(&"State.compositor.workspaces")
        );
        assert!(report.forbidden_direct_access.contains(&"State.registry"));
        assert!(report.forbidden_direct_access.contains(&"State.surfaces"));

        // 焦点和输出同样不能由未来 backend 绕过 Action 或 CoreCommand 直接修改。
        assert!(
            report
                .forbidden_direct_access
                .contains(&"CompositorState.focus")
        );
        assert!(
            report
                .forbidden_direct_access
                .contains(&"CompositorState.output")
        );
    }

    /// 验证契约文本包含入口、禁止访问区域和只读入口三个关键部分。
    #[test]
    fn integration_contract_pretty_print_contains_key_sections() {
        let report = CoreIntegrationContract::describe();
        let text = report.pretty_print();

        // 文本必须包含稳定标题和三类契约信息。
        assert!(text.contains("Sky Mirror Core Integration Contract"));
        assert!(text.contains("Allowed entry points:"));
        assert!(text.contains("Forbidden direct access:"));
        assert!(text.contains("Readonly entry points:"));

        // 关键命令入口和后端事件翻译入口必须在人类可读文本中明确出现。
        assert!(text.contains("StateHandleCommand"));
        assert!(text.contains("BackendEventTranslator"));
    }
}
