//! Phase 51K Linux-only nested lifecycle single-pump coordinator。
//!
//! coordinator 只按固定顺序编排现有 [`NestedRealAcceptFlow`]：accept/insert 与
//! connected bridge、一次 Display dispatch、disconnected bridge。它不直接修改 core
//! registry，也不把单次 pump 冒充长期 compositor event loop。

use std::{io, time::Duration};

use crate::{
    core::{client::ClientId as CoreClientId, state::State},
    smithay_backend::real_accept_flow::NestedRealAcceptFlow,
};

/// Phase 51K coordinator 尚未满足的独立能力条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeCoordinatorBlocker {
    /// 尚无 Linux test/CI 证明一次 coordinator pump 串通完整 client lifecycle。
    MissingLinuxLifecyclePumpProof,

    /// 尚无可持续运行、具备停止语义的长期 runtime loop。
    MissingLongRunningLoop,
}

/// Phase 51K coordinator 的保守 capability 报告。
///
/// `coordinator_boundary_defined` 说明接口已存在；五个 single-pump 字段由 Linux CI
/// lifecycle proof 支持。长期 loop、surface 和 render 不属于本阶段。
#[must_use = "必须区分 single-pump boundary、Linux proof 与长期 compositor loop"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeCoordinatorReadinessReport {
    /// 当前仍存在的 coordinator/runtime blockers。
    pub blockers: Vec<NestedRuntimeCoordinatorBlocker>,

    /// 是否已定义 Linux-only coordinator interface。
    pub coordinator_boundary_defined: bool,

    /// 是否已有 Linux proof 支持的 nested runtime coordinator。
    pub nested_runtime_coordinator_available: bool,

    /// 是否已有 Linux proof 支持的 single pump。
    pub single_pump_available: bool,

    /// connected bridge 是否已由 coordinator runtime proof 调用。
    pub connected_bridge_invoked: bool,

    /// disconnect bridge 是否已由 coordinator runtime proof 调用。
    pub disconnect_bridge_invoked: bool,

    /// Display dispatch 是否已由 coordinator runtime proof 调用。
    pub display_dispatch_invoked: bool,

    /// 是否已具备项目级 client accept 能力；本阶段固定为 `false`。
    pub accepts_clients: bool,

    /// 是否已启动长期 accept loop；本阶段固定为 `false`。
    pub runtime_accept_loop_started: bool,

    /// 是否已启动长期 protocol dispatch；本阶段固定为 `false`。
    pub protocol_dispatch_started: bool,

    /// 是否已有长期 runtime loop；本阶段固定为 `false`。
    pub long_running_loop_available: bool,

    /// 是否支持真实 surface；本阶段固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；本阶段固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实 render；本阶段固定为 `false`。
    pub render_support: bool,

    /// 是否支持真实 input；本阶段固定为 `false`。
    pub input_support: bool,
}

impl NestedRuntimeCoordinatorReadinessReport {
    /// 判断 Linux single-pump lifecycle proof 是否已完整成立。
    pub fn is_single_pump_ready(&self) -> bool {
        self.nested_runtime_coordinator_available
            && self.single_pump_available
            && self.connected_bridge_invoked
            && self.disconnect_bridge_invoked
            && self.display_dispatch_invoked
    }
}

/// 返回 Phase 51K C 路线经 Linux lifecycle proof 支持的 coordinator readiness。
#[must_use = "single-pump proof 不能代替长期 runtime loop"]
pub fn nested_runtime_coordinator_readiness_report() -> NestedRuntimeCoordinatorReadinessReport {
    NestedRuntimeCoordinatorReadinessReport {
        blockers: vec![NestedRuntimeCoordinatorBlocker::MissingLongRunningLoop],
        coordinator_boundary_defined: true,
        nested_runtime_coordinator_available: true,
        single_pump_available: true,
        connected_bridge_invoked: true,
        disconnect_bridge_invoked: true,
        display_dispatch_invoked: true,
        accepts_clients: false,
        runtime_accept_loop_started: false,
        protocol_dispatch_started: false,
        long_running_loop_available: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        input_support: false,
    }
}

/// single pump 中可结构化返回的错误阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimePumpErrorKind {
    /// listening socket/calloop accept source pump 失败。
    AcceptSourcePump,

    /// Display client dispatch 失败。
    DisplayDispatch,
}

/// single pump 的结构化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimePumpError {
    /// 错误发生的 coordinator 阶段。
    pub kind: NestedRuntimePumpErrorKind,

    /// 底层错误文本，仅用于诊断。
    pub message: String,
}

/// 一次 nested lifecycle pump 的纯数据汇总报告。
///
/// report 不持有 Display、socket、client 或 `State` 引用；核心 client IDs 只来自既有
/// session bridge outcome，coordinator 不猜测或分配 core identity。
#[must_use = "single-pump report 包含错误、bridge 结果和 validation，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimePumpReport {
    /// accept source 本轮观察到的 client 数量。
    pub accepted_clients: usize,

    /// 成功 insert 到 Display 的 client 数量。
    pub inserted_clients: usize,

    /// 本轮 drain 的 connected event 数量。
    pub connected_events_drained: usize,

    /// connected bridge 注册得到的 core client IDs。
    pub registered_core_clients: Vec<CoreClientId>,

    /// 本轮是否尝试调用 Display dispatch。
    pub dispatch_clients_called: bool,

    /// dispatch 成功时返回的 request 数量；失败时为 `None`。
    pub dispatched_requests: Option<usize>,

    /// 本轮 drain 的 disconnected event 数量。
    pub disconnected_events_drained: usize,

    /// disconnected bridge 关闭的 core client IDs。
    pub closed_core_clients: Vec<CoreClientId>,

    /// pump 结束时核心状态是否通过 ValidationReport。
    pub validation_is_clean: bool,

    /// accept/dispatch 阶段的结构化错误，按发生顺序保存。
    pub errors: Vec<NestedRuntimePumpError>,

    /// 当前 coordinator capability 快照。
    pub readiness: NestedRuntimeCoordinatorReadinessReport,
}

impl NestedRuntimePumpReport {
    /// 本轮是否没有错误且最终 validation clean。
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty() && self.validation_is_clean
    }
}

/// Linux-only nested client lifecycle single-pump coordinator。
///
/// coordinator 只拥有并编排 [`NestedRealAcceptFlow`]。connected/disconnected mutation
/// 继续由 flow 内的 session bridge 走 `BackendEvent -> CoreCommand -> State`；本类型
/// 不直接写任何 core registry。调用方可以周期调用 [`Self::pump_once`]，但该接口本身
/// 没有 run/stop/wakeup 语义，因此不等于长期 runtime loop。
pub struct NestedRuntimeCoordinator {
    flow: NestedRealAcceptFlow,
}

impl NestedRuntimeCoordinator {
    /// 使用指定 Wayland socket 名称创建 coordinator。
    ///
    /// # Errors
    ///
    /// Display、socket、calloop source 或 accept flow 初始化失败时返回原始错误链。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            flow: NestedRealAcceptFlow::with_socket_name(name)?,
        })
    }

    /// 返回 coordinator 已绑定的 Wayland socket 名称。
    pub fn socket_name(&self) -> &str {
        self.flow.socket_name()
    }

    /// 执行一次 accept/connected → Display dispatch → disconnected lifecycle pump。
    ///
    /// accept 与 dispatch 错误会进入 report；coordinator 不 panic，也不会绕过既有
    /// bridge。即使 dispatch 失败，已由 callback 产生的 disconnect event 仍会被安全
    /// drain，避免把 active session mapping 留在半完成状态。
    pub fn pump_once(&mut self, state: &mut State, timeout: Duration) -> NestedRuntimePumpReport {
        self.pump_once_with_dispatch(state, timeout, |flow| flow.dispatch_wayland_clients_once())
    }

    // 内部 seam 只允许测试注入 dispatch error；production 始终调用真实 Display dispatch。
    fn pump_once_with_dispatch<F>(
        &mut self,
        state: &mut State,
        timeout: Duration,
        dispatch: F,
    ) -> NestedRuntimePumpReport
    where
        F: FnOnce(&mut NestedRealAcceptFlow) -> io::Result<usize>,
    {
        let mut accepted_clients = 0;
        let mut inserted_clients = 0;
        let mut connected_events_drained = 0;
        let mut registered_core_clients = Vec::new();
        let mut errors = Vec::new();

        match self.flow.pump_once(state, timeout) {
            Ok(report) => {
                accepted_clients = report.accepted_stream_count();
                inserted_clients = report.inserted_client_count();
                connected_events_drained = report.connected_records.len();
                registered_core_clients = report.registered_core_clients();
            }
            Err(error) => errors.push(NestedRuntimePumpError {
                kind: NestedRuntimePumpErrorKind::AcceptSourcePump,
                message: error.to_string(),
            }),
        }

        let dispatched_requests = match dispatch(&mut self.flow) {
            Ok(count) => Some(count),
            Err(error) => {
                errors.push(NestedRuntimePumpError {
                    kind: NestedRuntimePumpErrorKind::DisplayDispatch,
                    message: error.to_string(),
                });
                None
            }
        };

        // callback 只写 session event；coordinator 必须回到既有 disconnect bridge。
        let disconnected = self.flow.bridge_pending_disconnects(state);
        let disconnected_events_drained = disconnected.disconnected_count();
        let closed_core_clients = disconnected.closed_core_clients();

        NestedRuntimePumpReport {
            accepted_clients,
            inserted_clients,
            connected_events_drained,
            registered_core_clients,
            dispatch_clients_called: true,
            dispatched_requests,
            disconnected_events_drained,
            closed_core_clients,
            validation_is_clean: state.validate().is_clean(),
            errors,
            readiness: nested_runtime_coordinator_readiness_report(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io, os::unix::net::UnixStream, path::Path, time::Duration};

    use super::{
        NestedRuntimeCoordinator, NestedRuntimeCoordinatorBlocker, NestedRuntimePumpErrorKind,
        nested_runtime_coordinator_readiness_report,
    };
    use crate::{
        core::state::State,
        smithay_backend::test_support::{assert_runtime_dir, unique_socket_name},
    };

    /// 验证 C 路线只上调 Linux proof 支持的 single-pump 字段，不冒充长期 loop。
    #[test]
    fn nested_runtime_coordinator_proof_capabilities_are_precise() {
        let report = nested_runtime_coordinator_readiness_report();

        assert_eq!(
            report.blockers,
            vec![NestedRuntimeCoordinatorBlocker::MissingLongRunningLoop]
        );
        assert!(report.coordinator_boundary_defined);
        assert!(report.nested_runtime_coordinator_available);
        assert!(report.single_pump_available);
        assert!(report.connected_bridge_invoked);
        assert!(report.disconnect_bridge_invoked);
        assert!(report.display_dispatch_invoked);
        assert!(!report.accepts_clients);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.long_running_loop_available);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.is_single_pump_ready());
    }

    /// 验证没有 client 的 single pump 安全返回，不 panic、不制造 core mutation。
    #[test]
    fn nested_runtime_pump_noop_is_safe() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-noop");
        let mut coordinator = NestedRuntimeCoordinator::with_socket_name(&socket_name)
            .expect("coordinator 必须绑定测试 socket");
        let mut state = State::new();

        let report = coordinator.pump_once(&mut state, Duration::ZERO);

        assert_eq!(report.accepted_clients, 0);
        assert_eq!(report.inserted_clients, 0);
        assert_eq!(report.connected_events_drained, 0);
        assert!(report.registered_core_clients.is_empty());
        assert!(report.dispatch_clients_called);
        assert_eq!(report.disconnected_events_drained, 0);
        assert!(report.closed_core_clients.is_empty());
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(state.clients.records().is_empty());
    }

    /// Linux-only 真实 proof：一次 coordinator pump 串通 connected 与 disconnected lifecycle。
    #[test]
    fn nested_runtime_coordinator_single_pump_runs_full_lifecycle() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-lifecycle");
        let mut coordinator = NestedRuntimeCoordinator::with_socket_name(&socket_name)
            .expect("coordinator 必须绑定测试 socket");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(coordinator.socket_name());
        let client_stream =
            UnixStream::connect(socket_path).expect("测试 peer 必须连接真实 Wayland socket");
        let mut state = State::new();

        // peer 在 pump 前关闭；server 仍会 accept 已建立连接，随后 dispatch 观察 EOF。
        drop(client_stream);
        let report = coordinator.pump_once(&mut state, Duration::from_secs(1));

        assert_eq!(report.accepted_clients, 1);
        assert_eq!(report.inserted_clients, 1);
        assert_eq!(report.connected_events_drained, 1);
        assert_eq!(report.registered_core_clients.len(), 1);
        assert!(report.dispatch_clients_called);
        assert_eq!(report.disconnected_events_drained, 1);
        assert_eq!(report.closed_core_clients, report.registered_core_clients);
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(report.readiness.is_single_pump_ready());
        assert!(!report.readiness.accepts_clients);
        assert!(!report.readiness.runtime_accept_loop_started);
        assert!(!report.readiness.protocol_dispatch_started);
        assert!(!report.readiness.long_running_loop_available);
        let client = report.registered_core_clients[0];
        assert!(!state.clients.is_alive(client));
        assert!(state.clients.get(client).is_some());

        // 第二次周期调用必须 no-op，不重复注册或关闭。
        let record_count = state.clients.records().len();
        let duplicate = coordinator.pump_once(&mut state, Duration::ZERO);
        assert_eq!(duplicate.connected_events_drained, 0);
        assert_eq!(duplicate.disconnected_events_drained, 0);
        assert!(duplicate.registered_core_clients.is_empty());
        assert!(duplicate.closed_core_clients.is_empty());
        assert_eq!(state.clients.records().len(), record_count);
        assert!(duplicate.validation_is_clean);
    }

    /// 验证 Display dispatch error 返回结构化报告，同时保持 core validation clean。
    #[test]
    fn nested_runtime_pump_reports_dispatch_failure() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-runtime-dispatch-error");
        let mut coordinator = NestedRuntimeCoordinator::with_socket_name(&socket_name)
            .expect("coordinator 必须绑定测试 socket");
        let mut state = State::new();

        let report = coordinator.pump_once_with_dispatch(&mut state, Duration::ZERO, |_| {
            Err(io::Error::other("controlled dispatch failure"))
        });

        assert!(report.dispatch_clients_called);
        assert_eq!(report.dispatched_requests, None);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(
            report.errors[0].kind,
            NestedRuntimePumpErrorKind::DisplayDispatch
        );
        assert!(
            report.errors[0]
                .message
                .contains("controlled dispatch failure")
        );
        assert!(!report.is_successful());
        assert!(report.validation_is_clean);
        assert!(state.clients.records().is_empty());
    }
}
