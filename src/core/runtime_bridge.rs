//! 单个后端事件进入核心状态的运行时桥接层。
//!
//! 本模块为未来真实 Smithay 或 backend 回调提供单事件入口。`CoreRuntimeBridge`
//! 不持有 `State` 引用；nested session coordinator 只持有纯数据 active mapping。
//! 本模块不引入 Wayland 类型，也不保存真实 surface/client 对象，只组合现有纯数据
//! 事件、命令、执行结果和验证报告。

use crate::{
    core::{
        backend_event::{BackendEvent, BackendEventTranslator},
        client::{ClientId, ClientKind},
        command::{CommandResult, CoreCommand},
        state::State,
        validator::ValidationReport,
    },
    smithay_backend::client_session::{
        NestedClientSessionEventKind, NestedClientSessionEventRecord, NestedClientSessionId,
        NestedClientSessionRegistry, NestedClientSessionRejectionReason,
    },
};

/// 单个后端事件处理后的完整结果。
///
/// `RuntimeEventResult` 是运行时版本的单步报告：它保存原始 `BackendEvent`、
/// 翻译出的 `CoreCommand`、命令执行结果，以及执行后立即生成的
/// `ValidationReport`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEventResult {
    /// 原始后端事件。
    pub event: BackendEvent,

    /// 翻译后的核心命令。
    pub command: CoreCommand,

    /// `State` 执行核心命令后的结果。
    pub result: CommandResult,

    /// 命令执行后的状态验证报告。
    pub validation: ValidationReport,
}

/// 单事件运行时桥接器。
///
/// `CoreRuntimeBridge` 是未来真实 Smithay 或 backend 回调进入核心状态的薄边界。
/// 它与 `BackendEventReplayer` 的区别是：回放器用于测试中一次处理多条事件，
/// 运行时桥接器用于每次处理一条后端事件。
///
/// 本阶段仍然只处理纯数据 `BackendEvent`，不持有真实 Smithay 或 Wayland 对象。
pub struct CoreRuntimeBridge;

impl CoreRuntimeBridge {
    /// 处理单个后端事件。
    ///
    /// 处理流程固定为：
    /// `BackendEvent -> BackendEventTranslator -> CoreCommand
    /// -> State::handle_command_with_validation()`。
    ///
    /// 该方法不会自动打印、不会自动修复状态，也不会保存 `State` 引用。
    pub fn handle_backend_event(state: &mut State, event: BackendEvent) -> RuntimeEventResult {
        let command = BackendEventTranslator::translate(event.clone());

        // State 统一保证 validation 在命令执行后生成；bridge 只附加 event/command 上下文。
        let (result, validation) = state.handle_command_with_validation(command.clone());

        RuntimeEventResult {
            event,
            command,
            result,
            validation,
        }
    }
}

/// nested session record 进入核心生命周期 seam 后的结构化结果。
///
/// 只有 `Connected` 和 `Disconnected` 携带 [`RuntimeEventResult`]，用于证明状态
/// 变化确实经过 `BackendEvent -> CoreCommand -> State`。其余 variants 不提交核心事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedClientSessionBridgeOutcome {
    /// session 已注册核心 client，并保存 active mapping。
    Connected {
        /// adapter session 身份。
        session: NestedClientSessionId,

        /// 核心分配的 client ID。
        client: ClientId,

        /// 完整的核心运行时处理结果。
        runtime: RuntimeEventResult,
    },

    /// session 已通过核心关闭路径结束生命周期。
    Disconnected {
        /// adapter session 身份。
        session: NestedClientSessionId,

        /// 被关闭的核心 client ID。
        client: ClientId,

        /// 完整的核心运行时处理结果。
        runtime: RuntimeEventResult,
    },

    /// adapter 已拒绝 session；核心状态保持不变。
    Rejected {
        /// 拒绝发生前已经存在时保留 session 身份。
        session: Option<NestedClientSessionId>,

        /// adapter 提供的结构化拒绝原因。
        reason: NestedClientSessionRejectionReason,
    },

    /// session 已有 active mapping，重复连接未提交核心注册事件。
    DuplicateConnected {
        /// 重复连接引用的 adapter session。
        session: NestedClientSessionId,

        /// 首次连接已经绑定的核心 client。
        client: ClientId,
    },

    /// 断开事件引用未知 session，未提交伪造的核心关闭事件。
    UnknownDisconnected {
        /// 未知 adapter session。
        session: NestedClientSessionId,
    },

    /// record 字段组合不符合对应 event kind 的纯数据契约。
    InvalidRecord {
        /// 无效 record 的观察顺序，便于诊断来源。
        sequence: u64,

        /// 无效 record 声明的事件类别。
        kind: NestedClientSessionEventKind,
    },

    /// 核心注册路径未返回成功的 `ClientRegistered` 结果。
    RegistrationFailed {
        /// 注册失败的 adapter session。
        session: NestedClientSessionId,

        /// 核心返回的完整结果，供调用方诊断而不是 panic。
        runtime: RuntimeEventResult,
    },
}

/// Phase 51E 的 nested session 到核心 client 生命周期协调器。
///
/// Bridge 只持有纯数据 active mapping，不持有真实 socket、平台 display 或 client
/// 对象。它不会直接修改核心 registry；所有状态变化都委托给 [`CoreRuntimeBridge`]。
/// 真实平台 callback 仍属于后续 Linux-only 阶段。
#[derive(Debug, Clone, Default)]
pub struct NestedClientSessionCoreBridge {
    sessions: NestedClientSessionRegistry,
}

impl NestedClientSessionCoreBridge {
    /// 创建没有 active session mapping 的 bridge。
    pub fn new() -> Self {
        Self::default()
    }

    /// 处理一条 Phase 51D 纯数据 session event record。
    ///
    /// Connected/Disconnected 通过既有 client lifecycle seam 修改核心状态；
    /// Rejected、duplicate 和 unknown 输入只返回结构化结果，不提交核心事件。
    pub fn handle_record(
        &mut self,
        state: &mut State,
        record: &NestedClientSessionEventRecord,
    ) -> NestedClientSessionBridgeOutcome {
        match record.kind {
            NestedClientSessionEventKind::Connected => self.handle_connected(state, record),
            NestedClientSessionEventKind::Disconnected => self.handle_disconnected(state, record),
            NestedClientSessionEventKind::Rejected => self.handle_rejected(record),
        }
    }

    /// 查询 session 当前绑定的核心 client ID。
    pub fn lookup_client(&self, session: NestedClientSessionId) -> Option<ClientId> {
        self.sessions.lookup(session)
    }

    /// 返回当前 active session mapping 数量。
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 处理 connected record，并在核心注册成功后写入 active mapping。
    fn handle_connected(
        &mut self,
        state: &mut State,
        record: &NestedClientSessionEventRecord,
    ) -> NestedClientSessionBridgeOutcome {
        let Some(session) = record.session else {
            return NestedClientSessionBridgeOutcome::InvalidRecord {
                sequence: record.sequence,
                kind: record.kind,
            };
        };

        if record.rejection_reason.is_some() {
            return NestedClientSessionBridgeOutcome::InvalidRecord {
                sequence: record.sequence,
                kind: record.kind,
            };
        }

        // 重复 session 必须在提交核心事件前被截断，避免创建第二个核心 client。
        if let Some(client) = self.sessions.lookup(session) {
            return NestedClientSessionBridgeOutcome::DuplicateConnected { session, client };
        }

        let runtime = CoreRuntimeBridge::handle_backend_event(
            state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: record.label.clone(),
            },
        );

        let CommandResult::ClientRegistered { client, registered } = &runtime.result else {
            return NestedClientSessionBridgeOutcome::RegistrationFailed { session, runtime };
        };
        let client = *client;

        if !registered {
            return NestedClientSessionBridgeOutcome::RegistrationFailed { session, runtime };
        }

        // sessions 是 bridge 私有状态；前置 lookup 后 bind 在同一 &mut self 调用中完成。
        if self.sessions.bind(session, client).is_err() {
            return NestedClientSessionBridgeOutcome::RegistrationFailed { session, runtime };
        }

        NestedClientSessionBridgeOutcome::Connected {
            session,
            client,
            runtime,
        }
    }

    /// 处理 disconnected record；只有 known session 才能生成核心关闭事件。
    fn handle_disconnected(
        &mut self,
        state: &mut State,
        record: &NestedClientSessionEventRecord,
    ) -> NestedClientSessionBridgeOutcome {
        let Some(session) = record.session else {
            return NestedClientSessionBridgeOutcome::InvalidRecord {
                sequence: record.sequence,
                kind: record.kind,
            };
        };

        if record.rejection_reason.is_some() {
            return NestedClientSessionBridgeOutcome::InvalidRecord {
                sequence: record.sequence,
                kind: record.kind,
            };
        }

        // 未知 session 没有可信 ClientId，必须明确忽略，不能猜测或伪造关闭目标。
        let Some(client) = self.sessions.lookup(session) else {
            return NestedClientSessionBridgeOutcome::UnknownDisconnected { session };
        };

        let runtime = CoreRuntimeBridge::handle_backend_event(
            state,
            BackendEvent::ClientDisconnected { client },
        );

        // Active mapping 随 adapter session 结束而移除；core 继续保留 dead tombstone。
        self.sessions.remove(session);

        NestedClientSessionBridgeOutcome::Disconnected {
            session,
            client,
            runtime,
        }
    }

    /// 处理 rejected record，不向核心提交任何 client lifecycle event。
    fn handle_rejected(
        &self,
        record: &NestedClientSessionEventRecord,
    ) -> NestedClientSessionBridgeOutcome {
        let Some(reason) = record.rejection_reason else {
            return NestedClientSessionBridgeOutcome::InvalidRecord {
                sequence: record.sequence,
                kind: record.kind,
            };
        };

        // Rejected 表示 adapter 未建立 session，不得注册核心 client 或写入 mapping。
        NestedClientSessionBridgeOutcome::Rejected {
            session: record.session,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CoreRuntimeBridge, NestedClientSessionBridgeOutcome, NestedClientSessionCoreBridge,
    };
    use crate::{
        core::{
            action::Action,
            backend_event::BackendEvent,
            client::ClientKind,
            command::{CommandResult, CoreCommand},
            state::State,
            surface::SurfaceRole,
            validator::ValidationIssueKind,
            window::WindowKind,
        },
        smithay_backend::client_session::{
            NestedClientSessionEvent, NestedClientSessionEventKind, NestedClientSessionEventLog,
            NestedClientSessionEventRecord, NestedClientSessionId,
            NestedClientSessionRejectionReason,
        },
    };

    /// 验证 connected session record 通过既有 event/command/state seam 注册核心 client。
    #[test]
    fn nested_session_bridge_connected_registers_client_and_saves_mapping() {
        let mut state = State::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();
        let record = log
            .record(
                NestedClientSessionEvent::Connected { session },
                Some("nested-terminal".to_string()),
                None,
            )
            .clone();

        let outcome = bridge.handle_record(&mut state, &record);

        let NestedClientSessionBridgeOutcome::Connected {
            session: outcome_session,
            client,
            runtime,
        } = outcome
        else {
            panic!("connected record 必须生成 Connected outcome");
        };
        assert_eq!(outcome_session, session);
        assert_eq!(bridge.lookup_client(session), Some(client));
        assert!(state.clients.is_alive(client));
        assert_eq!(
            runtime.event,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("nested-terminal".to_string()),
            }
        );
        assert_eq!(
            runtime.command,
            CoreCommand::RegisterClient {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("nested-terminal".to_string()),
            }
        );
        assert_eq!(
            runtime.result,
            CommandResult::ClientRegistered {
                client,
                registered: true,
            }
        );
        assert!(runtime.validation.is_clean());
    }

    /// 验证 disconnected session 通过既有关闭命令生成核心 tombstone 并移除 active mapping。
    #[test]
    fn nested_session_bridge_disconnected_closes_client_through_core_path() {
        let mut state = State::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();
        let connected = log
            .record(NestedClientSessionEvent::Connected { session }, None, None)
            .clone();
        let _ = bridge.handle_record(&mut state, &connected);
        let client = bridge
            .lookup_client(session)
            .expect("连接后必须保存 mapping");
        let disconnected = log
            .record(
                NestedClientSessionEvent::Disconnected { session },
                None,
                Some("peer closed".to_string()),
            )
            .clone();

        let outcome = bridge.handle_record(&mut state, &disconnected);

        let NestedClientSessionBridgeOutcome::Disconnected {
            session: outcome_session,
            client: outcome_client,
            runtime,
        } = outcome
        else {
            panic!("known disconnected record 必须生成 Disconnected outcome");
        };
        assert_eq!(outcome_session, session);
        assert_eq!(outcome_client, client);
        assert_eq!(runtime.event, BackendEvent::ClientDisconnected { client });
        assert_eq!(runtime.command, CoreCommand::CloseClient(client));
        assert!(matches!(
            runtime.result,
            CommandResult::ClientClosed {
                client: closed_client,
                marked_dead: true,
                ..
            } if closed_client == client
        ));
        assert!(runtime.validation.is_clean());
        assert_eq!(bridge.lookup_client(session), None);
        assert_eq!(bridge.active_session_count(), 0);
        assert!(!state.clients.is_alive(client));
        assert!(state.clients.get(client).is_some());
    }

    /// 验证 controlled disconnected record 会复用既有 seam 完成完整级联清理。
    ///
    /// 该测试只封板纯数据 record 行为，不代表真实 Smithay `ClientData` callback 已触发。
    #[test]
    fn controlled_disconnected_record_closes_bound_core_client() {
        let mut state = State::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let session = NestedClientSessionId::new(51).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();

        // Arrange：先经 session bridge 注册核心 client，再经 public runtime seam 创建
        // 该 client 拥有的 surface/window，禁止测试直接写任一 registry。
        let connected = log
            .record(
                NestedClientSessionEvent::Connected { session },
                Some("controlled-phase-51j".to_string()),
                None,
            )
            .clone();
        let _ = bridge.handle_record(&mut state, &connected);
        let client = bridge
            .lookup_client(session)
            .expect("controlled connect 后必须存在 session mapping");

        let created = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceCreated {
                surface: 510,
                client: Some(client),
                role: SurfaceRole::XdgToplevel,
            },
        );
        assert!(created.validation.is_clean());
        let mapped = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ToplevelMapped {
                surface: 510,
                title: "controlled-window".to_string(),
                app_id: Some("sky-mirror.phase-51j".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            },
        );
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped.result else {
            panic!("controlled map 必须返回 WindowRegisteredForSurface");
        };
        assert!(bound);

        // Act：用 controlled record 驱动既有 bridge；这里不是 runtime callback。
        let disconnected = log
            .record(
                NestedClientSessionEvent::Disconnected { session },
                None,
                Some("controlled disconnected record".to_string()),
            )
            .clone();
        let outcome = bridge.handle_record(&mut state, &disconnected);

        // Assert：event/command/state 路径、mapping remove、tombstone 与级联清理必须同时成立。
        let NestedClientSessionBridgeOutcome::Disconnected {
            client: outcome_client,
            runtime,
            ..
        } = outcome
        else {
            panic!("known controlled disconnected record 必须生成 Disconnected outcome");
        };
        assert_eq!(outcome_client, client);
        assert_eq!(runtime.event, BackendEvent::ClientDisconnected { client });
        assert_eq!(runtime.command, CoreCommand::CloseClient(client));
        assert!(runtime.validation.is_clean());
        assert_eq!(bridge.lookup_client(session), None);
        assert_eq!(bridge.active_session_count(), 0);

        // core 保留诊断 tombstone，但 client、surface 与 window 都必须结束生命周期。
        let client_record = state
            .clients
            .get(client)
            .expect("disconnect 后必须保留 client tombstone");
        assert!(!client_record.alive);
        assert!(!state.surfaces.is_alive(510));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );
        assert!(state.debug_bundle().is_clean());
    }

    /// 验证 duplicate controlled disconnect 不会重复关闭或制造验证错误。
    #[test]
    fn duplicate_disconnected_session_does_not_close_twice() {
        let mut state = State::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let session = NestedClientSessionId::new(52).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();

        // Arrange：首次连接建立唯一 mapping；disconnect record 随后会移除它。
        let connected = log
            .record(NestedClientSessionEvent::Connected { session }, None, None)
            .clone();
        let _ = bridge.handle_record(&mut state, &connected);
        let client = bridge
            .lookup_client(session)
            .expect("controlled connect 后必须存在 mapping");
        let disconnected = log
            .record(
                NestedClientSessionEvent::Disconnected { session },
                None,
                None,
            )
            .clone();
        let first = bridge.handle_record(&mut state, &disconnected);
        assert!(matches!(
            first,
            NestedClientSessionBridgeOutcome::Disconnected {
                client: closed_client,
                ..
            } if closed_client == client
        ));
        let client_count_after_first = state.clients.records().len();

        // Act：同一 controlled record 重放时，session 已不再有可信 mapping。
        let duplicate = bridge.handle_record(&mut state, &disconnected);

        // Assert：第二次必须退化为 unknown，不产生第二次 core close 或新记录。
        assert_eq!(
            duplicate,
            NestedClientSessionBridgeOutcome::UnknownDisconnected { session }
        );
        assert_eq!(state.clients.records().len(), client_count_after_first);
        assert_eq!(bridge.lookup_client(session), None);
        assert!(!state.clients.is_alive(client));
        assert!(state.clients.get(client).is_some());
        assert!(state.validate().is_clean());
    }

    /// 验证缺少 session 的 invalid disconnected record 返回结构化结果且不 panic。
    #[test]
    fn invalid_disconnected_record_does_not_panic_or_mutate_core() {
        let mut state = State::new();
        let initial_client_count = state.clients.records().len();
        let mut bridge = NestedClientSessionCoreBridge::new();

        // Arrange：手工构造字段不完整的纯数据 record，模拟 adapter 边界输入错误。
        let invalid = NestedClientSessionEventRecord {
            sequence: 51,
            kind: NestedClientSessionEventKind::Disconnected,
            session: None,
            rejection_reason: None,
            label: None,
            diagnostic: Some("missing session".to_string()),
        };

        // Act：现有 bridge 必须返回 outcome，而不是 panic 或猜测核心 ClientId。
        let outcome = bridge.handle_record(&mut state, &invalid);

        // Assert：无效输入不产生 mapping、client mutation 或 validation issue。
        assert_eq!(
            outcome,
            NestedClientSessionBridgeOutcome::InvalidRecord {
                sequence: 51,
                kind: NestedClientSessionEventKind::Disconnected,
            }
        );
        assert_eq!(bridge.active_session_count(), 0);
        assert_eq!(state.clients.records().len(), initial_client_count);
        assert!(state.validate().is_clean());
    }

    /// 验证 rejected record 只返回拒绝结果，不注册核心 client 或 mapping。
    #[test]
    fn nested_session_bridge_rejected_does_not_register_client() {
        let mut state = State::new();
        let initial_client_count = state.clients.records().len();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let mut log = NestedClientSessionEventLog::new();
        let reason = NestedClientSessionRejectionReason::Unsupported;
        let rejected = log
            .record_rejected(
                None,
                reason,
                Some("unsupported-client".to_string()),
                Some("runtime unavailable".to_string()),
            )
            .clone();

        let outcome = bridge.handle_record(&mut state, &rejected);

        assert_eq!(
            outcome,
            NestedClientSessionBridgeOutcome::Rejected {
                session: None,
                reason,
            }
        );
        assert_eq!(state.clients.records().len(), initial_client_count);
        assert_eq!(bridge.active_session_count(), 0);
        assert!(state.validate().is_clean());
    }

    /// 验证 unknown disconnect 返回明确结果，不 panic、不提交虚假核心关闭命令。
    #[test]
    fn nested_session_bridge_unknown_disconnect_is_ignored() {
        let mut state = State::new();
        let initial_client_count = state.clients.records().len();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let session = NestedClientSessionId::new(99).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();
        let disconnected = log
            .record(
                NestedClientSessionEvent::Disconnected { session },
                None,
                None,
            )
            .clone();

        let outcome = bridge.handle_record(&mut state, &disconnected);

        assert_eq!(
            outcome,
            NestedClientSessionBridgeOutcome::UnknownDisconnected { session }
        );
        assert_eq!(state.clients.records().len(), initial_client_count);
        assert_eq!(bridge.active_session_count(), 0);
        assert!(state.validate().is_clean());
    }

    /// 验证不同 session 分别映射到不同核心 client，互不覆盖。
    #[test]
    fn nested_session_bridge_keeps_multiple_session_mappings_distinct() {
        let mut state = State::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let first_session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let second_session = NestedClientSessionId::new(8).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();
        let first = log
            .record(
                NestedClientSessionEvent::Connected {
                    session: first_session,
                },
                Some("first".to_string()),
                None,
            )
            .clone();
        let second = log
            .record(
                NestedClientSessionEvent::Connected {
                    session: second_session,
                },
                Some("second".to_string()),
                None,
            )
            .clone();

        let _ = bridge.handle_record(&mut state, &first);
        let _ = bridge.handle_record(&mut state, &second);

        let first_client = bridge
            .lookup_client(first_session)
            .expect("first session 必须存在 mapping");
        let second_client = bridge
            .lookup_client(second_session)
            .expect("second session 必须存在 mapping");
        assert_ne!(first_client, second_client);
        assert!(state.clients.is_alive(first_client));
        assert!(state.clients.is_alive(second_client));
        assert_eq!(bridge.active_session_count(), 2);
        assert!(state.validate().is_clean());
    }

    /// 验证 duplicate connected session 不产生第二个核心 client。
    #[test]
    fn nested_session_bridge_duplicate_connected_returns_existing_mapping() {
        let mut state = State::new();
        let mut bridge = NestedClientSessionCoreBridge::new();
        let session = NestedClientSessionId::new(7).expect("非零 session ID 必须有效");
        let mut log = NestedClientSessionEventLog::new();
        let first = log
            .record(NestedClientSessionEvent::Connected { session }, None, None)
            .clone();
        let duplicate = log
            .record(
                NestedClientSessionEvent::Connected { session },
                Some("duplicate".to_string()),
                None,
            )
            .clone();
        let _ = bridge.handle_record(&mut state, &first);
        let client = bridge
            .lookup_client(session)
            .expect("首次连接必须保存 mapping");
        let client_count = state.clients.records().len();

        let outcome = bridge.handle_record(&mut state, &duplicate);

        assert_eq!(
            outcome,
            NestedClientSessionBridgeOutcome::DuplicateConnected { session, client }
        );
        assert_eq!(state.clients.records().len(), client_count);
        assert_eq!(bridge.lookup_client(session), Some(client));
        assert_eq!(bridge.active_session_count(), 1);
        assert!(state.validate().is_clean());
    }

    /// 验证单个输出尺寸事件会通过现有 Action 链路更新集中状态。
    #[test]
    fn runtime_bridge_handles_output_resize_event() {
        let mut state = State::new();

        let result = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::OutputResized {
                width: 1600,
                height: 900,
            },
        );

        // 合法尺寸变化执行后必须保持核心状态有效。
        assert!(result.validation.is_valid());

        let output = state.compositor.current_output_size();

        // 运行时状态必须保留事件提供的输出宽高。
        assert_eq!(output.width, 1600);
        assert_eq!(output.height, 900);

        // 桥接器必须复用现有 ResizeOutput Action，不能直接修改 output。
        assert_eq!(
            result.command,
            CoreCommand::Action(Action::ResizeOutput {
                width: 1600,
                height: 900,
            })
        );
    }

    /// 验证 surface 创建、map 和关闭可以逐条通过运行时桥接器完成。
    #[test]
    fn runtime_bridge_handles_surface_lifecycle_step_by_step() {
        let mut state = State::new();

        let created = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceCreated {
                surface: 42,
                client: None,
                role: SurfaceRole::XdgToplevel,
            },
        );

        // 创建步骤必须注册外部指定 ID，并保持状态有效。
        assert!(created.validation.is_valid());
        assert_eq!(
            created.result,
            CommandResult::SurfaceRegistered {
                surface: 42,
                registered: true,
            }
        );

        let mapped = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            },
        );

        // map 步骤必须创建窗口并绑定已有 surface。
        assert!(mapped.validation.is_valid());
        let CommandResult::WindowRegisteredForSurface {
            surface,
            window,
            bound,
        } = &mapped.result
        else {
            panic!("map 步骤必须返回 WindowRegisteredForSurface");
        };
        assert_eq!(*surface, 42);
        assert!(*bound);
        let mapped_window = *window;

        let closed = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceClosed { surface: 42 },
        );

        // 关闭步骤必须同步结束 surface 和绑定窗口的生命周期。
        assert!(closed.validation.is_valid());
        let CommandResult::SurfaceClosed {
            surface,
            surface_marked_dead,
            closed_window,
            marked_window_dead,
            ..
        } = closed.result
        else {
            panic!("关闭步骤必须返回 SurfaceClosed");
        };
        assert_eq!(surface, 42);
        assert!(surface_marked_dead);
        assert_eq!(closed_window, Some(mapped_window));
        assert!(marked_window_dead);

        // 生命周期结束后诊断记录仍保留，但两侧都必须标记为 dead。
        assert!(!state.surfaces.get(42).expect("surface 记录必须保留").alive);
        assert!(!state.registry.is_alive(mapped_window));
    }

    /// 验证 client connection 通过 runtime public seam 自动分配 ID 并进入诊断快照。
    #[test]
    fn runtime_bridge_client_connected_auto_allocates_id_and_updates_diagnostics() {
        let mut state = State::new();

        let connected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("terminal".to_string()),
            },
        );

        // BackendEvent 只表达外部事实；bridge 必须先翻译为命令，不能直接修改 State。
        assert_eq!(
            connected.command,
            CoreCommand::RegisterClient {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("terminal".to_string()),
            }
        );

        let CommandResult::ClientRegistered { client, registered } = connected.result else {
            panic!("client connection 必须返回 ClientRegistered");
        };

        // generated ClientId 必须从 CommandResult 读取，未来 adapter 不能猜测 registry allocator。
        assert!(registered);
        let record = state
            .clients
            .get(client)
            .expect("自动分配的 client 必须存在 registry 记录");
        assert!(record.alive);
        assert_eq!(record.kind, ClientKind::WaylandPlaceholder);
        assert_eq!(record.name.as_deref(), Some("terminal"));

        // client socket connection 不等于 surface；alive client 暂无 surface 是合法中间状态。
        assert!(state.surfaces.records().is_empty());
        assert!(connected.validation.is_clean());

        let bundle = state.debug_bundle();
        let snapshot_record = bundle
            .snapshot
            .clients
            .iter()
            .find(|record| record.id == client)
            .expect("DebugBundle 必须包含自动注册的 client");

        // DebugBundle 必须同时反映 client metadata 与命令后的 clean validation。
        assert!(snapshot_record.alive);
        assert_eq!(snapshot_record.name.as_deref(), Some("terminal"));
        assert!(bundle.is_clean());
    }

    /// 验证 runtime public seam 接受 explicit ClientId，并推进后续自动分配边界。
    #[test]
    fn runtime_bridge_client_connected_accepts_explicit_id_without_allocator_conflict() {
        let mut state = State::new();

        let explicit = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("explicit-client".to_string()),
            },
        );

        // 翻译后的命令必须保留外部 ID 和 metadata，BackendEvent 本身不写 registry。
        assert_eq!(
            explicit.command,
            CoreCommand::RegisterClient {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("explicit-client".to_string()),
            }
        );
        assert_eq!(
            explicit.result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert!(state.clients.is_alive(42));
        assert!(explicit.validation.is_clean());

        let generated = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: None,
            },
        );
        let CommandResult::ClientRegistered {
            client: generated_client,
            registered,
        } = generated.result
        else {
            panic!("后续自动注册必须返回 ClientRegistered");
        };

        // 显式 ID 必须推进 allocator，后续 generated ClientId 不能覆盖现有 client 42。
        assert!(registered);
        assert_ne!(generated_client, 42);
        assert!(generated_client > 42);
        assert!(state.clients.is_alive(generated_client));
        assert!(generated.validation.is_clean());
    }

    /// 验证 runtime public seam 拒绝重复 explicit ClientId，且不覆盖原记录。
    #[test]
    fn runtime_bridge_client_connected_rejects_duplicate_explicit_id_cleanly() {
        let mut state = State::new();

        let first = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(42),
                kind: ClientKind::Mock,
                name: Some("original".to_string()),
            },
        );
        let duplicate = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(42),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("replacement".to_string()),
            },
        );

        assert_eq!(
            first.result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: true,
            }
        );
        assert_eq!(
            duplicate.result,
            CommandResult::ClientRegistered {
                client: 42,
                registered: false,
            }
        );

        let record = state.clients.get(42).expect("首次 client 记录必须继续存在");

        // 重复外部 ID 不能覆盖首条 metadata，也不能制造第二条同 ID 记录。
        assert_eq!(record.kind, ClientKind::Mock);
        assert_eq!(record.name.as_deref(), Some("original"));
        assert_eq!(
            state
                .clients
                .records()
                .iter()
                .filter(|record| record.id == 42)
                .count(),
            1
        );

        // 重复连接是明确的 command result，不是结构损坏，validation 必须保持 clean。
        assert!(duplicate.validation.is_clean());
        assert!(state.debug_bundle().is_clean());
    }

    /// 验证没有 surface 的 client 断开后保留 tombstone，并保持状态 clean。
    #[test]
    fn runtime_bridge_client_disconnected_without_surfaces_keeps_clean_tombstone() {
        let mut state = State::new();

        let connected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: None,
                kind: ClientKind::WaylandPlaceholder,
                name: Some("short-lived".to_string()),
            },
        );
        let CommandResult::ClientRegistered { client, registered } = connected.result else {
            panic!("client connection 必须返回 ClientRegistered");
        };
        assert!(registered);

        // alive client 尚未创建 surface 是合法中间状态，不应在断开前产生 validation issue。
        assert!(state.surfaces.records().is_empty());
        assert!(connected.validation.is_clean());

        let disconnected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientDisconnected { client },
        );

        assert_eq!(disconnected.command, CoreCommand::CloseClient(client));
        assert_eq!(
            disconnected.result,
            CommandResult::ClientClosed {
                client,
                marked_dead: true,
                dead_surfaces: Vec::new(),
                closed_windows: Vec::new(),
                removed_from_workspace_count: 0,
                marked_window_dead_count: 0,
            }
        );

        // 断开采用 tombstone 而不是物理删除，诊断层才能保留稳定 ClientId 与 metadata。
        let record = state
            .clients
            .get(client)
            .expect("断开后的 client tombstone 必须保留");
        assert!(!record.alive);
        assert!(!state.clients.is_alive(client));
        assert!(disconnected.validation.is_clean());

        let bundle = state.debug_bundle();
        let snapshot_record = bundle
            .snapshot
            .clients
            .iter()
            .find(|record| record.id == client)
            .expect("DebugBundle 必须保留 dead client tombstone");

        assert!(!snapshot_record.alive);
        assert_eq!(snapshot_record.name.as_deref(), Some("short-lived"));
        assert!(bundle.is_clean());
    }

    /// 验证 client 断开事件会通过运行时桥级联关闭 surface 和 window。
    #[test]
    fn runtime_bridge_client_disconnected_cascades_to_surface_and_window() {
        let mut state = State::new();

        let connected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientConnected {
                client: Some(7),
                kind: ClientKind::WaylandPlaceholder,
                name: Some("app".to_string()),
            },
        );
        assert!(connected.validation.is_valid());

        let created = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceCreated {
                surface: 42,
                client: Some(7),
                role: SurfaceRole::XdgToplevel,
            },
        );
        assert!(created.validation.is_valid());

        let mapped = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            },
        );
        let CommandResult::WindowRegisteredForSurface { window, bound, .. } = mapped.result else {
            panic!("map 步骤必须返回 WindowRegisteredForSurface");
        };
        assert!(bound);

        let mapped_slot = state
            .compositor
            .current_workspace()
            .and_then(|workspace| {
                workspace
                    .slots
                    .iter()
                    .find(|slot| workspace.slot_window(slot.id) == Some(window))
                    .map(|slot| slot.id)
            })
            .expect("mapped window 必须属于当前 workspace 的一个 slot");
        let focused = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ActionRequested(Action::FocusSlot(mapped_slot)),
        );

        // 先经 public Action seam 聚焦目标窗口，才能封板 disconnect 后的 focus cleanup。
        assert_eq!(state.compositor.focus.window, Some(window));
        assert!(focused.validation.is_clean());

        let disconnected = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::ClientDisconnected { client: 7 },
        );
        assert_eq!(disconnected.command, CoreCommand::CloseClient(7));
        let CommandResult::ClientClosed {
            client,
            marked_dead,
            dead_surfaces,
            closed_windows,
            removed_from_workspace_count,
            marked_window_dead_count,
        } = disconnected.result
        else {
            panic!("client 断开必须返回 ClientClosed");
        };

        // 最后一步必须完整报告 client -> surface -> window 的纯数据级联。
        assert_eq!(client, 7);
        assert!(marked_dead);
        assert_eq!(dead_surfaces, vec![42]);
        assert_eq!(closed_windows, vec![window]);
        assert_eq!(removed_from_workspace_count, 1);
        assert_eq!(marked_window_dead_count, 1);

        // 级联后记录仍然存在，但三层生命周期都必须结束。
        assert!(!state.clients.is_alive(7));
        assert!(!state.surfaces.is_alive(42));
        assert!(!state.registry.is_alive(window));
        assert!(
            state
                .compositor
                .workspaces
                .iter()
                .all(|workspace| !workspace.window_ids().contains(&window))
        );

        // client disconnect 必须刷新焦点，不能让 live focus path 指向已关闭窗口。
        assert!(state.compositor.focus.slot < 4);
        assert_ne!(state.compositor.focus.window, Some(window));

        let bundle = state.debug_bundle();
        let client_snapshot = bundle
            .snapshot
            .clients
            .iter()
            .find(|record| record.id == 7)
            .expect("DebugBundle 必须保留 dead client");
        let surface_snapshot = bundle
            .snapshot
            .surfaces
            .iter()
            .find(|record| record.id == 42)
            .expect("DebugBundle 必须保留 dead surface");
        let window_snapshot = bundle
            .snapshot
            .windows
            .iter()
            .find(|record| record.id == window)
            .expect("DebugBundle 必须保留 dead window");

        // 级联采用 tombstone 保留历史 identity，同时从 workspace live path 清理窗口。
        assert!(!client_snapshot.alive);
        assert!(!surface_snapshot.alive);
        assert_eq!(surface_snapshot.client, Some(7));
        assert_eq!(surface_snapshot.window, Some(window));
        assert!(!window_snapshot.alive);
        assert!(!window_snapshot.referenced_by_workspace);

        // 运行时桥接完成级联后，状态必须重新满足全部不变量。
        assert!(disconnected.validation.is_clean());
        assert!(bundle.is_clean());
    }

    /// 验证桥接器会记录状态错误，但不会自动修复损坏的绑定。
    #[test]
    fn runtime_bridge_records_validation_issue_without_repairing_state() {
        let mut state = State::new();
        let surface = state.register_surface(SurfaceRole::XdgToplevel);

        // 测试直接制造缺失窗口绑定，用于验证桥接器的只记录、不修复语义。
        assert!(state.surfaces.bind_window(surface, 999));

        let result =
            CoreRuntimeBridge::handle_backend_event(&mut state, BackendEvent::ValidateRequested);

        // 执行后验证必须报告无效状态及稳定问题类型。
        assert!(!result.validation.is_valid());
        assert!(
            result
                .validation
                .issues
                .iter()
                .any(|issue| { issue.kind == ValidationIssueKind::SurfaceReferencesMissingWindow })
        );

        // 桥接器不得擅自清理或替换错误绑定。
        assert_eq!(
            state
                .surfaces
                .get(surface)
                .expect("损坏 surface 记录必须继续存在")
                .window,
            Some(999)
        );
    }

    /// 验证单步结果完整保存原始事件和翻译后的核心命令。
    #[test]
    fn runtime_bridge_result_stores_event_and_command() {
        let mut state = State::new();
        let event = BackendEvent::DebugRequested;

        let result = CoreRuntimeBridge::handle_backend_event(&mut state, event.clone());

        // 调用方必须能够从结果中追溯原始事实和实际执行命令。
        assert_eq!(result.event, event);
        assert_eq!(result.command, CoreCommand::DebugText);
    }
}
