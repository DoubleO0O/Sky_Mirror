//! Phase 51J 真实 client disconnect callback 的 Linux-only readiness 边界。
//!
//! 当前仓库尚未具备经 Linux runtime 测试证明的真实 accept loop、accepted-stream
//! insertion、callback ownership 或真实 inserted-client mapping，因此本模块只枚举
//! 前置缺口并返回保守报告。Phase 51H-R / 51I 的 compile boundary 不等同于这些
//! runtime 事实；本模块不创建 fake disconnected event，不调用 core，也不提升能力位。

/// 真实 disconnect callback 接入前仍缺失的独立前置条件。
///
/// 每个 blocker 都对应一项必须由真实 Linux runtime 代码和测试证明的事实，避免用
/// 单一 `NotReady` 状态掩盖 accept、client ownership 或 callback source 的具体缺口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedClientDisconnectCallbackBlocker {
    /// 尚未启动真实 Wayland socket accept loop。
    MissingRealAcceptLoop,

    /// 尚未由真实 accept callback 使用 accepted stream 调用并验证 client insertion。
    MissingDisplayHandleInsertClient,

    /// 尚无经真实 inserted-client 生命周期验证的 `ClientData` callback ownership。
    MissingClientDataOwner,

    /// 尚无真实 inserted client 到 [`NestedClientSessionId`](super::client_session::NestedClientSessionId) 的 adapter mapping。
    MissingRealClientSessionMapping,

    /// 尚无真实 Wayland backend disconnect/error 通知作为事件来源。
    MissingDisconnectCallbackSource,

    /// 尚无 Linux runtime 测试证明 callback 到 session event 再到 core close 的完整链路。
    MissingLinuxRuntimeProof,
}

/// Phase 51J B 路线的 disconnect callback readiness 报告。
///
/// 报告只保存 blocker 与布尔能力证据，不保存 Smithay client、Wayland object、
/// socket 或 core 状态引用。所有能力字段在当前阶段都故意保持 `false`；这表示
/// 前置实现尚未存在，而不是遗漏了某个赋值。
#[must_use = "readiness 报告必须被检查，不能忽略尚未满足的真实 runtime 前置条件"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedClientDisconnectCallbackReadinessReport {
    /// 当前仍阻止真实 disconnect callback 接入的全部已知条件。
    pub blockers: Vec<NestedClientDisconnectCallbackBlocker>,

    /// 是否已有真实 socket accept 与 client insertion 能力。
    pub accepts_clients: bool,

    /// 是否已经由真实 `ClientData` disconnect/error callback 观察到断开。
    pub real_disconnect_callback_observed: bool,

    /// 是否已经由真实 callback 触发既有 bridge 并关闭 core client。
    pub core_close_invoked_from_real_callback: bool,

    /// 是否已经存在真实 `ClientData` owner。
    pub real_client_data_callback_owned: bool,

    /// 是否已经保存真实 inserted client 到 adapter session 的映射。
    pub real_inserted_client_mapping_available: bool,

    /// 是否支持真实 surface；Phase 51J readiness 固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；Phase 51J readiness 固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实渲染；Phase 51J readiness 固定为 `false`。
    pub render_support: bool,

    /// 是否已启动真实 Wayland protocol dispatch。
    pub protocol_dispatch_started: bool,

    /// 是否已启动真实 socket accept loop。
    pub runtime_accept_loop_started: bool,
}

impl NestedClientDisconnectCallbackReadinessReport {
    /// 判断真实 disconnect callback 前置条件是否全部满足。
    ///
    /// surface、shell role 和 render 不属于 client disconnect 的必要条件，因此不参与
    /// readiness 判定；真实 accept、mapping、callback、dispatch 和 core close 证据缺一不可。
    pub fn is_ready(&self) -> bool {
        self.blockers.is_empty()
            && self.accepts_clients
            && self.real_disconnect_callback_observed
            && self.core_close_invoked_from_real_callback
            && self.real_client_data_callback_owned
            && self.real_inserted_client_mapping_available
            && self.protocol_dispatch_started
            && self.runtime_accept_loop_started
    }
}

/// 生成当前 Phase 51J B 路线的保守 readiness 报告。
///
/// 该函数不会生成 disconnected record，也不会调用 `State`。未来真实 callback
/// 必须先在 adapter/runtime 层形成纯数据 session event，再复用既有 bridge；
/// readiness 报告本身不能绕过这条 seam 去修改 core registry。
#[must_use = "调用方必须检查 blockers，readiness 报告不代表真实 callback 已完成"]
pub fn nested_client_disconnect_callback_readiness_report()
-> NestedClientDisconnectCallbackReadinessReport {
    NestedClientDisconnectCallbackReadinessReport {
        blockers: vec![
            NestedClientDisconnectCallbackBlocker::MissingRealAcceptLoop,
            NestedClientDisconnectCallbackBlocker::MissingDisplayHandleInsertClient,
            NestedClientDisconnectCallbackBlocker::MissingClientDataOwner,
            NestedClientDisconnectCallbackBlocker::MissingRealClientSessionMapping,
            NestedClientDisconnectCallbackBlocker::MissingDisconnectCallbackSource,
            NestedClientDisconnectCallbackBlocker::MissingLinuxRuntimeProof,
        ],
        accepts_clients: false,
        real_disconnect_callback_observed: false,
        core_close_invoked_from_real_callback: false,
        real_client_data_callback_owned: false,
        real_inserted_client_mapping_available: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        protocol_dispatch_started: false,
        runtime_accept_loop_started: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NestedClientDisconnectCallbackBlocker, nested_client_disconnect_callback_readiness_report,
    };

    /// 验证报告逐项暴露全部真实 runtime 前置缺口。
    #[test]
    fn disconnect_callback_readiness_reports_all_missing_preconditions() {
        // Arrange 与 Act：当前 readiness 不接受调用方伪造的运行时证据。
        let report = nested_client_disconnect_callback_readiness_report();

        // Assert：blocker 顺序从 socket/client owner 到完整 Linux proof，便于稳定诊断。
        assert_eq!(
            report.blockers,
            vec![
                NestedClientDisconnectCallbackBlocker::MissingRealAcceptLoop,
                NestedClientDisconnectCallbackBlocker::MissingDisplayHandleInsertClient,
                NestedClientDisconnectCallbackBlocker::MissingClientDataOwner,
                NestedClientDisconnectCallbackBlocker::MissingRealClientSessionMapping,
                NestedClientDisconnectCallbackBlocker::MissingDisconnectCallbackSource,
                NestedClientDisconnectCallbackBlocker::MissingLinuxRuntimeProof,
            ]
        );
        assert!(!report.is_ready());
    }

    /// 验证 B 路线不会把 controlled record 或 readiness 文案冒充真实能力。
    #[test]
    fn disconnect_callback_readiness_keeps_runtime_capabilities_false() {
        // Arrange 与 Act：生成当前唯一受支持的保守报告。
        let report = nested_client_disconnect_callback_readiness_report();

        // Assert：所有真实 runtime 与越级 surface/render 能力必须保持关闭。
        assert!(!report.accepts_clients);
        assert!(!report.real_disconnect_callback_observed);
        assert!(!report.core_close_invoked_from_real_callback);
        assert!(!report.real_client_data_callback_owned);
        assert!(!report.real_inserted_client_mapping_available);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.runtime_accept_loop_started);
    }

    /// 验证 readiness 报告支持稳定比较、克隆与调试输出。
    #[test]
    fn disconnect_callback_readiness_report_supports_value_semantics() {
        let report = nested_client_disconnect_callback_readiness_report();

        // 纯数据报告必须可作为 Linux CI 断言和诊断证据传递。
        assert_eq!(report.clone(), report);
        assert!(format!("{report:?}").contains("NestedClientDisconnectCallbackReadinessReport"));
    }
}
