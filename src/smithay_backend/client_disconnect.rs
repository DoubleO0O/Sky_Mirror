//! Phase 51J 真实 client disconnect callback 的 Linux-only readiness 边界。
//!
//! 已接受的 Phase 51I-C-B 提供真实 accept/insertion 边界、`ClientData` owner 与
//! backend-client/session mapping；Phase 51J-A-B 再提供 disconnected event 到既有
//! core bridge 的受控 seam。Phase 51J-C 的 Linux test 已真实关闭 peer、dispatch
//! Display，并观察 callback 到 core close 的完整链路，因此只精确提升这两项证据。

/// 真实 disconnect callback runtime proof 的可诊断前置条件。
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

/// Phase 51J-C 的 disconnect callback runtime proof 报告。
///
/// 报告只保存 blocker 与布尔能力证据，不保存 Smithay client、Wayland object、
/// socket 或 core 状态引用。已由 accepted boundary 证明的结构事实可以为 `true`；
/// 真实 callback observation 与真实 core close 可由 Linux proof 标记为 `true`；项目级
/// accept、长期 protocol dispatch、surface 与 render 能力仍保持 `false`。
#[must_use = "runtime proof 报告必须被检查，不能把 callback proof 扩大为 compositor readiness"]
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

    /// 是否已存在 disconnected session event 到既有 core bridge 的受控 seam。
    pub disconnect_event_bridge_available: bool,

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
            && self.disconnect_event_bridge_available
            && self.protocol_dispatch_started
            && self.runtime_accept_loop_started
    }
}

/// 生成当前 Phase 51J-C 路线的精确 runtime proof 报告。
///
/// 该函数不会生成 disconnected record，也不会调用 `State`。真实 proof 已由 Linux
/// runtime test 取得；报告只保存证据快照，不能绕过 session event/bridge seam。
#[must_use = "调用方必须区分 callback proof 与尚未具备的项目级 runtime 能力"]
pub fn nested_client_disconnect_callback_readiness_report()
-> NestedClientDisconnectCallbackReadinessReport {
    NestedClientDisconnectCallbackReadinessReport {
        blockers: Vec::new(),
        accepts_clients: false,
        real_disconnect_callback_observed: true,
        core_close_invoked_from_real_callback: true,
        real_client_data_callback_owned: true,
        real_inserted_client_mapping_available: true,
        disconnect_event_bridge_available: true,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        protocol_dispatch_started: false,
        runtime_accept_loop_started: false,
    }
}

#[cfg(test)]
mod tests {
    use super::nested_client_disconnect_callback_readiness_report;

    /// 验证真实 callback source 与 Linux 完整链路 proof 已不再阻塞。
    #[test]
    fn disconnect_callback_runtime_proof_clears_remaining_blockers() {
        let report = nested_client_disconnect_callback_readiness_report();

        assert!(report.blockers.is_empty());
        assert!(!report.is_ready());
    }

    /// 验证 C 路线只提升真实 callback/close proof，不越级提升项目 runtime。
    #[test]
    fn disconnect_callback_runtime_proof_updates_only_proven_capabilities() {
        let report = nested_client_disconnect_callback_readiness_report();

        assert!(!report.accepts_clients);
        assert!(report.real_disconnect_callback_observed);
        assert!(report.core_close_invoked_from_real_callback);
        assert!(report.real_client_data_callback_owned);
        assert!(report.real_inserted_client_mapping_available);
        assert!(report.disconnect_event_bridge_available);
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
