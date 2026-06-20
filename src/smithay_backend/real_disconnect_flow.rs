//! Phase 51J-A disconnect callback queue 到 core close 的 Linux-only 桥接边界。
//!
//! [`NestedClientDataOwner`](super::client_insert::NestedClientDataOwner) 的 callback
//! 只记录带 adapter session identity 的纯数据 `Disconnected` event。本模块把这些
//! event 记录化后交给既有 [`NestedClientSessionCoreBridge`]；它不直接访问 core
//! registry，也不把 controlled event 冒充真实 Wayland runtime callback。

use crate::{
    core::{
        client::ClientId as CoreClientId,
        runtime_bridge::{NestedClientSessionBridgeOutcome, NestedClientSessionCoreBridge},
        state::State,
    },
    smithay_backend::{
        client_disconnect::{
            NestedClientDisconnectCallbackReadinessReport,
            nested_client_disconnect_callback_readiness_report,
        },
        client_session::{
            NestedClientSessionEvent, NestedClientSessionEventLog, NestedClientSessionEventRecord,
        },
        real_accept_flow::NestedAcceptedClientMapping,
    },
};

/// 一轮 pending disconnect session event bridge 的结构化结果。
///
/// report 只保存纯数据 records、bridge outcomes 和保守 readiness 快照。即使
/// controlled test 证明 close seam 正确，真实 callback 与真实 core close 能力仍保持关闭。
#[must_use = "disconnect report 包含 unknown/duplicate 与 validation 结果，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRealDisconnectCallbackReport {
    /// 本轮 callback queue 产生的 disconnected records。
    pub disconnected_records: Vec<NestedClientSessionEventRecord>,

    /// records 经既有 session/core bridge 得到的结果。
    pub bridge_outcomes: Vec<NestedClientSessionBridgeOutcome>,

    /// known disconnect 后移除的 backend-client/session mapping 数量。
    pub removed_backend_mapping_count: usize,

    /// 有 core mutation 时表示 validation 是否全部 clean；否则为 `None`。
    pub all_observed_validations_clean: Option<bool>,

    /// 当前 Phase 51J-A B 路线的保守 capability 与 blockers。
    pub readiness: NestedClientDisconnectCallbackReadinessReport,
}

impl NestedRealDisconnectCallbackReport {
    /// 返回本轮记录化的 disconnected event 数量。
    pub fn disconnected_count(&self) -> usize {
        self.disconnected_records.len()
    }

    /// 返回本轮通过既有 core close seam 成功处理的核心 client IDs。
    pub fn closed_core_clients(&self) -> Vec<CoreClientId> {
        self.bridge_outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                NestedClientSessionBridgeOutcome::Disconnected { client, .. } => Some(*client),
                _ => None,
            })
            .collect()
    }
}

/// 把 callback queue 中的 disconnected events 记录化并提交给既有 core bridge。
///
/// callback 不能持有 `State`，所以必须先产生 session event；只有 coordinator 调用本
/// helper 时才跨越 bridge seam。duplicate 或 unknown session 会由 bridge 安全忽略，
/// 不会猜测 core client，也不会制造第二次 close。
pub(super) fn bridge_disconnected_events(
    events: Vec<NestedClientSessionEvent>,
    socket_name: &str,
    event_log: &mut NestedClientSessionEventLog,
    core_bridge: &mut NestedClientSessionCoreBridge,
    accepted_mapping: &mut NestedAcceptedClientMapping,
    state: &mut State,
) -> NestedRealDisconnectCallbackReport {
    let mut disconnected_records = Vec::with_capacity(events.len());
    let mut bridge_outcomes = Vec::with_capacity(events.len());
    let mut removed_backend_mapping_count = 0;

    for event in events {
        let NestedClientSessionEvent::Disconnected { session } = event else {
            // drain_disconnected 已保证形状；防御性忽略避免误把 Connected 当作 close。
            continue;
        };
        let record = event_log
            .record(
                NestedClientSessionEvent::Disconnected { session },
                Some(socket_name.to_string()),
                Some("disconnect callback queue event; real runtime callback proof pending".into()),
            )
            .clone();
        let outcome = core_bridge.handle_record(state, &record);

        if matches!(
            &outcome,
            NestedClientSessionBridgeOutcome::Disconnected { .. }
        ) && accepted_mapping.remove_session(session).is_some()
        {
            removed_backend_mapping_count += 1;
        }

        disconnected_records.push(record);
        bridge_outcomes.push(outcome);
    }

    NestedRealDisconnectCallbackReport {
        all_observed_validations_clean: observed_validations_clean(&bridge_outcomes),
        disconnected_records,
        bridge_outcomes,
        removed_backend_mapping_count,
        readiness: nested_client_disconnect_callback_readiness_report(),
    }
}

// 只汇总实际经过 BackendEvent -> CoreCommand -> State 的 outcome。
fn observed_validations_clean(outcomes: &[NestedClientSessionBridgeOutcome]) -> Option<bool> {
    let mut observed = false;
    let mut all_clean = true;

    for clean in outcomes.iter().filter_map(|outcome| match outcome {
        NestedClientSessionBridgeOutcome::Disconnected { runtime, .. } => {
            Some(runtime.validation.is_clean())
        }
        _ => None,
    }) {
        observed = true;
        all_clean &= clean;
    }

    observed.then_some(all_clean)
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream;

    use smithay::reexports::wayland_server::Display;

    use super::bridge_disconnected_events;
    use crate::{
        core::{
            backend_event::BackendEvent,
            command::CoreCommand,
            runtime_bridge::{NestedClientSessionBridgeOutcome, NestedClientSessionCoreBridge},
            state::State,
        },
        smithay_backend::{
            client_insert::NestedClientInsertCompileBoundary,
            client_session::{
                NestedClientSessionEvent, NestedClientSessionEventLog, NestedClientSessionId,
            },
            real_accept_flow::NestedAcceptedClientMapping,
        },
    };

    fn session(value: u64) -> NestedClientSessionId {
        NestedClientSessionId::new(value).expect("测试 session ID 必须非零")
    }

    fn connect_session(
        session: NestedClientSessionId,
        log: &mut NestedClientSessionEventLog,
        bridge: &mut NestedClientSessionCoreBridge,
        state: &mut State,
    ) {
        let connected = log
            .record(NestedClientSessionEvent::Connected { session }, None, None)
            .clone();
        let outcome = bridge.handle_record(state, &connected);
        assert!(matches!(
            outcome,
            NestedClientSessionBridgeOutcome::Connected { .. }
        ));
    }

    /// 验证 controlled disconnected event 只经既有 session/core bridge 关闭 known client。
    #[test]
    fn known_disconnected_session_closes_core_client() {
        let session = session(81);
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let mut mapping = NestedAcceptedClientMapping::new();
        let mut state = State::new();
        connect_session(session, &mut log, &mut bridge, &mut state);
        let client = bridge
            .lookup_client(session)
            .expect("连接后必须有 core mapping");

        let report = bridge_disconnected_events(
            vec![NestedClientSessionEvent::Disconnected { session }],
            "controlled-known-disconnect",
            &mut log,
            &mut bridge,
            &mut mapping,
            &mut state,
        );

        assert_eq!(report.disconnected_count(), 1);
        assert_eq!(report.closed_core_clients(), vec![client]);
        let NestedClientSessionBridgeOutcome::Disconnected { runtime, .. } =
            &report.bridge_outcomes[0]
        else {
            panic!("known session 必须生成 Disconnected outcome");
        };
        assert_eq!(runtime.event, BackendEvent::ClientDisconnected { client });
        assert_eq!(runtime.command, CoreCommand::CloseClient(client));
        assert_eq!(report.all_observed_validations_clean, Some(true));
        assert!(!state.clients.is_alive(client));
        assert!(state.validate().is_clean());
    }

    /// 验证 duplicate disconnect 在首次 close 后退化为 unknown，不重复修改 core。
    #[test]
    fn duplicate_disconnect_does_not_close_twice() {
        let session = session(82);
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let mut mapping = NestedAcceptedClientMapping::new();
        let mut state = State::new();
        connect_session(session, &mut log, &mut bridge, &mut state);

        let report = bridge_disconnected_events(
            vec![
                NestedClientSessionEvent::Disconnected { session },
                NestedClientSessionEvent::Disconnected { session },
            ],
            "controlled-duplicate-disconnect",
            &mut log,
            &mut bridge,
            &mut mapping,
            &mut state,
        );

        assert_eq!(report.closed_core_clients().len(), 1);
        assert!(matches!(
            report.bridge_outcomes[1],
            NestedClientSessionBridgeOutcome::UnknownDisconnected { session: actual }
                if actual == session
        ));
        assert!(state.validate().is_clean());
    }

    /// 验证未注册或 unknown session 不 panic、不伪造 core close。
    #[test]
    fn unknown_disconnect_is_safe() {
        let session = session(83);
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let mut mapping = NestedAcceptedClientMapping::new();
        let mut state = State::new();

        let report = bridge_disconnected_events(
            vec![NestedClientSessionEvent::Disconnected { session }],
            "controlled-unknown-disconnect",
            &mut log,
            &mut bridge,
            &mut mapping,
            &mut state,
        );

        assert_eq!(
            report.bridge_outcomes,
            vec![NestedClientSessionBridgeOutcome::UnknownDisconnected { session }]
        );
        assert!(report.closed_core_clients().is_empty());
        assert_eq!(report.all_observed_validations_clean, None);
        assert!(state.clients.records().is_empty());
        assert!(state.validate().is_clean());
    }

    /// 验证 known disconnect 会按 session 移除真实 backend client mapping。
    #[test]
    fn known_disconnect_removes_backend_client_mapping() {
        let session = session(84);
        let display = Display::<()>::new().expect("Wayland Display 必须能构造");
        let mut insert = NestedClientInsertCompileBoundary::new(display.handle());
        let (server_stream, _client_stream) =
            UnixStream::pair().expect("UnixStream pair 必须能构造");
        let backend_client = insert
            .insert_client(server_stream, session)
            .expect("测试 client 必须成功插入 Display");
        let mut mapping = NestedAcceptedClientMapping::new();
        assert_eq!(mapping.insert(backend_client.id(), session), None);

        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let mut state = State::new();
        connect_session(session, &mut log, &mut bridge, &mut state);

        let report = bridge_disconnected_events(
            vec![NestedClientSessionEvent::Disconnected { session }],
            "controlled-mapping-removal",
            &mut log,
            &mut bridge,
            &mut mapping,
            &mut state,
        );

        assert_eq!(report.removed_backend_mapping_count, 1);
        assert!(mapping.is_empty());
        assert_eq!(report.all_observed_validations_clean, Some(true));
    }

    /// 验证 controlled bridge 不能提升真实 callback 或真实 core close capability。
    #[test]
    fn real_disconnect_callback_boundary_keeps_runtime_capability_false() {
        let mut log = NestedClientSessionEventLog::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let mut mapping = NestedAcceptedClientMapping::new();
        let mut state = State::new();

        let report = bridge_disconnected_events(
            Vec::new(),
            "controlled-empty-disconnect",
            &mut log,
            &mut bridge,
            &mut mapping,
            &mut state,
        );

        assert!(!report.readiness.accepts_clients);
        assert!(!report.readiness.real_disconnect_callback_observed);
        assert!(!report.readiness.core_close_invoked_from_real_callback);
        assert!(!report.readiness.is_ready());
    }
}
