//! Linux-only nested socket probe 到核心 client lifecycle 的受控 flow proof。
//!
//! Phase 51G 只组合 Phase 51F 的纯数据 event producer 与 Phase 51E 的核心 bridge。
//! Probe 本身继续不知道 core；flow 取得 event 后才调用 bridge。这里不启动真实
//! socket accept loop，不创建 surface，不处理 xdg_toplevel，不启动 render，也不
//! 提升 `accepts_clients` capability。真实 callback、disconnect 和 surface identity
//! 留给后续阶段。

use crate::{
    core::{
        client::ClientId,
        runtime_bridge::{NestedClientSessionBridgeOutcome, NestedClientSessionCoreBridge},
        state::State,
    },
    smithay_backend::{
        client_session::NestedClientSessionId,
        nested_socket_probe::{NestedSocketAcceptProbe, NestedSocketAcceptProbeReport},
    },
};

/// 一次 probe-to-bridge flow 的结构化结果。
///
/// Report 同时保留 probe 证据和 bridge outcome，调用方无需绕过 seam 查询或修改
/// 核心 registry。能力字段固定保持保守值，不能解释为真实 compositor runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedSocketProbeBridgeFlowReport {
    /// Phase 51F probe 产出的原始纯数据报告和 event record。
    pub probe_report: NestedSocketAcceptProbeReport,

    /// Phase 51E bridge 对同一 event record 的结构化处理结果。
    pub bridge_outcome: NestedClientSessionBridgeOutcome,

    /// Flow 是否把 probe event 提交给 bridge；正常调用固定为 `true`。
    pub bridge_received_event: bool,

    /// 本次调用是否新注册了核心 client；duplicate flow 固定为 `false`。
    pub core_client_registered: bool,

    /// session 是否在 bridge 中存在 active mapping。
    pub mapping_stored: bool,

    /// 当前 session 映射到的核心 client ID。
    pub mapped_client: Option<ClientId>,

    /// 本次 connected 是否被识别为 duplicate 并阻止第二次注册。
    pub duplicate_connected_suppressed: bool,

    /// 是否支持真实 surface；Phase 51G 固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 xdg_toplevel 等 shell role；Phase 51G 固定为 `false`。
    pub shell_role_support: bool,

    /// 是否启动 render；Phase 51G 固定为 `false`。
    pub render_support: bool,

    /// 是否启动真实 socket accept loop；Phase 51G 固定为 `false`。
    pub runtime_accept_loop_started: bool,

    /// 是否启动真实 runtime dispatch；Phase 51G 固定为 `false`。
    pub runtime_dispatch_started: bool,

    /// 是否提升 adapter 的真实 client capability；Phase 51G 固定为 `false`。
    pub accepts_clients_capability: bool,
}

/// 组合 nested socket probe 与核心 session bridge 的 Linux-only flow。
///
/// Flow 拥有两个既有模块并维护它们的调用顺序：probe 先产出纯数据 record，bridge
/// 再消费 record。该类型不持有任何平台 client、display、surface 或输入对象。
#[derive(Debug, Clone)]
pub struct NestedSocketProbeBridgeFlow {
    probe: NestedSocketAcceptProbe,
    bridge: NestedClientSessionCoreBridge,
}

impl NestedSocketProbeBridgeFlow {
    /// 使用纯字符串 socket 名称创建空 flow。
    ///
    /// 构造过程不打开 socket，也不启动 runtime；名称只传给 Phase 51F probe 诊断。
    pub fn new(socket_name: impl Into<String>) -> Self {
        Self {
            probe: NestedSocketAcceptProbe::new(socket_name),
            bridge: NestedClientSessionCoreBridge::new(),
        }
    }

    /// 从 probe 产生 connected record，并通过既有 bridge 推进核心 client lifecycle。
    ///
    /// 所有核心变化都由 [`NestedClientSessionCoreBridge`] 内部的
    /// `BackendEvent -> CoreCommand -> State` seam 完成；flow 不直接修改 registry。
    pub fn observe_connected_session(
        &mut self,
        state: &mut State,
        session: NestedClientSessionId,
        label: Option<String>,
        diagnostic: Option<String>,
    ) -> NestedSocketProbeBridgeFlowReport {
        let probe_report = self
            .probe
            .observe_connected_session(session, label, diagnostic);

        // Probe 保持只产出 event；只有 flow 在 event 产生后显式调用 Phase 51E bridge。
        let bridge_outcome = self.bridge.handle_record(state, &probe_report.event);
        let mapped_client = self.bridge.lookup_client(session);
        let core_client_registered = matches!(
            &bridge_outcome,
            NestedClientSessionBridgeOutcome::Connected { .. }
        );
        let duplicate_connected_suppressed = matches!(
            &bridge_outcome,
            NestedClientSessionBridgeOutcome::DuplicateConnected { .. }
        );

        // Flow report 只描述 pure-data proof；所有真实 runtime 能力继续明确关闭。
        NestedSocketProbeBridgeFlowReport {
            surface_support: probe_report.surface_support,
            shell_role_support: probe_report.shell_role_support,
            render_support: probe_report.render_support,
            runtime_accept_loop_started: probe_report.runtime_accept_loop_started,
            runtime_dispatch_started: probe_report.runtime_dispatch_started,
            probe_report,
            bridge_outcome,
            bridge_received_event: true,
            core_client_registered,
            mapping_stored: mapped_client.is_some(),
            mapped_client,
            duplicate_connected_suppressed,
            accepts_clients_capability: false,
        }
    }

    /// 查询 flow 内部 bridge 当前保存的 session-to-client active mapping。
    pub fn lookup_client(&self, session: NestedClientSessionId) -> Option<ClientId> {
        self.bridge.lookup_client(session)
    }

    /// 返回 flow 内部 bridge 当前保存的 active mapping 数量。
    pub fn active_session_count(&self) -> usize {
        self.bridge.active_session_count()
    }
}

#[cfg(test)]
mod tests {
    use super::{NestedSocketProbeBridgeFlow, NestedSocketProbeBridgeFlowReport};
    use crate::{
        core::{
            backend_event::BackendEvent,
            client::ClientKind,
            command::{CommandResult, CoreCommand},
            runtime_bridge::NestedClientSessionBridgeOutcome,
            state::State,
        },
        smithay_backend::{
            client_session::{NestedClientSessionEventKind, NestedClientSessionId},
            linux_adapter::SmithayLinuxAdapterCapabilities,
        },
    };

    /// 验证 Linux-only flow 初始没有 session mapping。
    #[test]
    fn flow_can_be_constructed_without_active_mapping() {
        let flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let session = NestedClientSessionId::new(1).expect("非零 session ID 必须有效");

        assert_eq!(flow.active_session_count(), 0);
        assert_eq!(flow.lookup_client(session), None);
    }

    /// 验证 flow 从 Phase 51F probe 取得 connected event record。
    #[test]
    fn flow_obtains_connected_record_from_socket_probe() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(2).expect("非零 session ID 必须有效");

        let report = flow.observe_connected_session(
            &mut state,
            session,
            Some("nested-terminal".to_string()),
            Some("phase 51g flow proof".to_string()),
        );

        assert!(report.probe_report.produced_session_event);
        assert_eq!(
            report.probe_report.event.kind,
            NestedClientSessionEventKind::Connected
        );
        assert_eq!(report.probe_report.event.session, Some(session));
        assert_eq!(
            report.probe_report.event.label.as_deref(),
            Some("nested-terminal")
        );
    }

    /// 验证 flow 把 probe record 交给既有 Phase 51E bridge。
    #[test]
    fn flow_submits_probe_record_to_core_bridge() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(3).expect("非零 session ID 必须有效");

        let report = flow.observe_connected_session(&mut state, session, None, None);

        assert!(report.bridge_received_event);
        assert!(!report.probe_report.core_bridge_invoked);
        assert!(matches!(
            report.bridge_outcome,
            NestedClientSessionBridgeOutcome::Connected { session: actual, .. }
                if actual == session
        ));
    }

    /// 验证 connected event 通过 event/command/state seam 注册核心 client。
    #[test]
    fn connected_flow_registers_core_client_through_existing_seam() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(4).expect("非零 session ID 必须有效");

        let report = flow.observe_connected_session(
            &mut state,
            session,
            Some("nested-editor".to_string()),
            None,
        );

        let NestedClientSessionBridgeOutcome::Connected {
            client, runtime, ..
        } = &report.bridge_outcome
        else {
            panic!("首次 connected flow 必须注册核心 client");
        };
        let client_record = state
            .clients
            .get(*client)
            .expect("注册后必须存在 client record");

        assert!(report.core_client_registered);
        assert!(runtime.validation.is_clean());
        assert_eq!(
            runtime.event,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("nested-editor".to_string()),
            }
        );
        assert_eq!(
            runtime.command,
            CoreCommand::RegisterClient {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("nested-editor".to_string()),
            }
        );
        assert_eq!(
            runtime.result,
            CommandResult::ClientRegistered {
                client: *client,
                registered: true,
            }
        );
        assert!(client_record.alive);
        assert_eq!(client_record.kind, ClientKind::WaylandPlaceholder);
    }

    /// 验证 flow 和 report 都能查询 session 到核心 client 的 active mapping。
    #[test]
    fn connected_flow_exposes_session_to_client_mapping() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(5).expect("非零 session ID 必须有效");

        let report = flow.observe_connected_session(&mut state, session, None, None);

        assert!(report.mapping_stored);
        assert_eq!(flow.lookup_client(session), report.mapped_client);
        assert!(report.mapped_client.is_some());
        assert_eq!(flow.active_session_count(), 1);
    }

    /// 验证 duplicate connected 被 bridge 截断，不产生第二个核心 client。
    #[test]
    fn duplicate_connected_flow_does_not_register_second_client() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(6).expect("非零 session ID 必须有效");

        let first = flow.observe_connected_session(&mut state, session, None, None);
        let client_count = state.clients.records().len();
        let duplicate = flow.observe_connected_session(&mut state, session, None, None);

        assert!(first.core_client_registered);
        assert!(!duplicate.core_client_registered);
        assert!(duplicate.duplicate_connected_suppressed);
        assert_eq!(state.clients.records().len(), client_count);
        assert_eq!(flow.active_session_count(), 1);
        assert_eq!(duplicate.mapped_client, first.mapped_client);
        assert!(matches!(
            duplicate.bridge_outcome,
            NestedClientSessionBridgeOutcome::DuplicateConnected { session: actual, .. }
                if actual == session
        ));
    }

    /// 验证 flow report 明确关闭所有超出 Phase 51G 的 runtime 能力。
    #[test]
    fn flow_report_keeps_surface_shell_render_and_runtime_disabled() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");

        let report = flow.observe_connected_session(&mut state, session, None, None);

        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.runtime_dispatch_started);
        assert!(!report.accepts_clients_capability);
    }

    /// 验证 flow proof 不提升 Linux adapter 的真实 client capability。
    #[test]
    fn flow_does_not_change_linux_adapter_client_capability() {
        let capabilities = SmithayLinuxAdapterCapabilities::skeleton_only();

        assert!(!capabilities.accepts_clients);
    }

    /// 验证 flow report 是可克隆、可比较的纯数据诊断值。
    #[test]
    fn flow_report_supports_clone_compare_and_debug() {
        let mut flow = NestedSocketProbeBridgeFlow::new("wayland-phase-51g");
        let mut state = State::new();
        let session = NestedClientSessionId::new(8).expect("非零 session ID 必须有效");

        let report: NestedSocketProbeBridgeFlowReport =
            flow.observe_connected_session(&mut state, session, None, None);

        assert_eq!(report.clone(), report);
        assert!(format!("{report:?}").contains("NestedSocketProbeBridgeFlowReport"));
    }
}
