//! Linux-only nested socket connection event probe。
//!
//! Phase 51F 只验证 socket 边界能够产出纯数据 session event。该模块不会调用
//! Phase 51E core bridge，也不会启动真实 accept loop、创建 surface、处理
//! xdg_toplevel 或启动 render。真实 Linux callback 到核心生命周期的受控流转留给
//! Phase 51G；因此本模块也不会改变 adapter 的 `accepts_clients` capability。

use crate::smithay_backend::client_session::{
    NestedClientSessionEvent, NestedClientSessionEventLog, NestedClientSessionEventRecord,
    NestedClientSessionId,
};

/// 阻止 Phase 51F probe 被解释为真实 client runtime 的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedSocketAcceptProbeBlocker {
    /// 本阶段没有启动真实 socket accept loop，只有显式的纯数据观察入口。
    RuntimeAcceptLoopNotStarted,
}

/// 一次 Linux-only socket connection probe 的保守报告。
///
/// 报告包含 Phase 51D 的 [`NestedClientSessionEventRecord`]，并显式关闭 core、
/// protocol object、shell role、render 和 runtime dispatch 能力。字段为纯数据，
/// 不携带真实 socket、display 或平台 client 对象。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedSocketAcceptProbeReport {
    /// probe 在当前 Linux feature 构建中可用。
    pub enabled: bool,

    /// probe 只在 Linux target 下编译。
    pub linux_only: bool,

    /// probe 要求启用 `smithay-linux` feature。
    pub requires_smithay_linux: bool,

    /// 观察来源的 socket 名称，仅作为纯字符串诊断。
    pub socket_name: String,

    /// 本次观察是否产出了 session event。
    pub produced_session_event: bool,

    /// Phase 51D 定义的纯数据 session event record。
    pub event: NestedClientSessionEventRecord,

    /// 本阶段是否调用了 Phase 51E core bridge；固定为 `false`。
    pub core_bridge_invoked: bool,

    /// 是否支持创建真实 surface；Phase 51F 固定为 `false`。
    pub surface_support: bool,

    /// 是否支持处理 xdg_toplevel 等 shell role；Phase 51F 固定为 `false`。
    pub shell_role_support: bool,

    /// 是否启动 render；Phase 51F 固定为 `false`。
    pub render_support: bool,

    /// 是否启动协议/runtime dispatch；Phase 51F 固定为 `false`。
    pub runtime_dispatch_started: bool,

    /// 是否启动真实 socket accept loop；Phase 51F 固定为 `false`。
    pub runtime_accept_loop_started: bool,

    /// 当前 probe 尚未成为真实 runtime 的结构化 blocker。
    pub blocker: NestedSocketAcceptProbeBlocker,
}

/// Linux-only nested socket connection 的纯数据 event producer。
///
/// Probe 只保存 socket 名称和 Phase 51D event log。真实 socket resource 仍由
/// `wayland_socket` 模块持有；真实 Smithay client callback、core bridge 复用和
/// disconnect 生命周期都留给 Phase 51G 或后续阶段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedSocketAcceptProbe {
    socket_name: String,
    events: NestedClientSessionEventLog,
}

impl NestedSocketAcceptProbe {
    /// 为一个已知 socket 名称创建纯数据 probe。
    ///
    /// 构造过程不会创建或打开 socket；调用方只提供现有 Linux socket 边界的诊断名。
    pub fn new(socket_name: impl Into<String>) -> Self {
        Self {
            socket_name: socket_name.into(),
            events: NestedClientSessionEventLog::new(),
        }
    }

    /// 记录 socket 边界观察到的一次连接，并产出 connected session event report。
    ///
    /// 该方法刻意不调用 Phase 51E bridge：Phase 51F 只证明 Linux-only socket
    /// seam 可以生成纯数据 event，不注册核心 client，也不创建协议对象。
    pub fn observe_connected_session(
        &mut self,
        session: NestedClientSessionId,
        label: Option<String>,
        diagnostic: Option<String>,
    ) -> NestedSocketAcceptProbeReport {
        let event = self
            .events
            .record(
                NestedClientSessionEvent::Connected { session },
                label,
                diagnostic,
            )
            .clone();

        // 所有真实能力字段保持 false，避免 probe report 被误读为 client runtime。
        NestedSocketAcceptProbeReport {
            enabled: true,
            linux_only: true,
            requires_smithay_linux: true,
            socket_name: self.socket_name.clone(),
            produced_session_event: true,
            event,
            core_bridge_invoked: false,
            surface_support: false,
            shell_role_support: false,
            render_support: false,
            runtime_dispatch_started: false,
            runtime_accept_loop_started: false,
            blocker: NestedSocketAcceptProbeBlocker::RuntimeAcceptLoopNotStarted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NestedSocketAcceptProbe, NestedSocketAcceptProbeBlocker, NestedSocketAcceptProbeReport,
    };
    use crate::smithay_backend::{
        client_session::{NestedClientSessionEventKind, NestedClientSessionId},
        linux_adapter::SmithayLinuxAdapterCapabilities,
    };

    /// 验证 report 明确声明该 producer 只属于 Linux feature 探针。
    #[test]
    fn report_declares_linux_only_probe_contract() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(1).expect("非零 session ID 必须有效");

        let report = probe.observe_connected_session(session, None, None);

        assert!(report.enabled);
        assert!(report.linux_only);
        assert!(report.requires_smithay_linux);
        assert_eq!(report.socket_name, "wayland-phase-51f");
    }

    /// 验证 socket 边界观察会产出 Phase 51D 的纯数据 event record。
    #[test]
    fn probe_produces_nested_client_session_event_record() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(2).expect("非零 session ID 必须有效");

        let report: NestedSocketAcceptProbeReport = probe.observe_connected_session(
            session,
            Some("nested-terminal".to_string()),
            Some("socket connection observed by probe".to_string()),
        );

        assert!(report.produced_session_event);
        assert_eq!(report.event.sequence, 1);
        assert_eq!(report.event.session, Some(session));
        assert_eq!(report.event.label.as_deref(), Some("nested-terminal"));
        assert_eq!(
            report.event.diagnostic.as_deref(),
            Some("socket connection observed by probe")
        );
    }

    /// 验证 probe 产出的生命周期类型是 connected，而不是拒绝或断开。
    #[test]
    fn produced_event_is_connected() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(3).expect("非零 session ID 必须有效");

        let report = probe.observe_connected_session(session, None, None);

        assert_eq!(report.event.kind, NestedClientSessionEventKind::Connected);
        assert_eq!(report.event.rejection_reason, None);
    }

    /// 验证同一 probe 的纯数据记录沿用单调递增顺序。
    #[test]
    fn probe_preserves_session_event_sequence() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let first = NestedClientSessionId::new(4).expect("非零 session ID 必须有效");
        let second = NestedClientSessionId::new(5).expect("非零 session ID 必须有效");

        let first_report = probe.observe_connected_session(first, None, None);
        let second_report = probe.observe_connected_session(second, None, None);

        assert_eq!(first_report.event.sequence, 1);
        assert_eq!(second_report.event.sequence, 2);
    }

    /// 验证 Phase 51F report 明确说明事件尚未进入核心生命周期 seam。
    #[test]
    fn probe_does_not_invoke_core_bridge() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(6).expect("非零 session ID 必须有效");

        let report = probe.observe_connected_session(session, None, None);

        assert!(!report.core_bridge_invoked);
    }

    /// 验证 probe 不创建协议对象、不启动输出链路或 runtime dispatch。
    #[test]
    fn probe_keeps_protocol_render_and_runtime_capabilities_disabled() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");

        let report = probe.observe_connected_session(session, None, None);

        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.runtime_dispatch_started);
        assert!(!report.runtime_accept_loop_started);
    }

    /// 验证新增 probe 不提升 Linux adapter 的真实 client capability。
    #[test]
    fn probe_does_not_change_real_client_capability() {
        let capabilities = SmithayLinuxAdapterCapabilities::skeleton_only();

        assert!(!capabilities.accepts_clients);
    }

    /// 验证 report 对未启动真实 runtime loop 给出结构化 blocker。
    #[test]
    fn report_exposes_runtime_accept_loop_blocker() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(8).expect("非零 session ID 必须有效");

        let report = probe.observe_connected_session(session, None, None);

        assert_eq!(
            report.blocker,
            NestedSocketAcceptProbeBlocker::RuntimeAcceptLoopNotStarted
        );
    }

    /// 验证 report 是可克隆、可比较并适合 diagnostics 的纯数据值。
    #[test]
    fn report_supports_clone_compare_and_debug() {
        let mut probe = NestedSocketAcceptProbe::new("wayland-phase-51f");
        let session = NestedClientSessionId::new(9).expect("非零 session ID 必须有效");

        let report = probe.observe_connected_session(session, None, None);

        assert_eq!(report.clone(), report);
        assert!(format!("{report:?}").contains("NestedSocketAcceptProbeReport"));
    }
}
