//! Phase 51L Linux-only nested lifecycle bounded runtime loop。
//!
//! loop 只重复编排 [`NestedRuntimeCoordinator::pump_once`]，负责有限迭代、idle/error/stop
//! 退出和纯数据报告。它不直接修改 core，也不把 bounded loop 冒充完整 compositor runtime。

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::{
    core::state::State,
    smithay_backend::nested_runtime_coordinator::{
        NestedRuntimeCoordinator, NestedRuntimePumpError, NestedRuntimePumpReport,
    },
};

/// Phase 51L bounded loop 尚未满足的独立能力条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeLoopBlocker {
    /// 尚无 Linux test/CI 证明 loop 可多次调用真实 coordinator pump。
    MissingLinuxBoundedLoopProof,

    /// stop flag 尚未接入 event source wakeup，不能立即打断正在等待的 pump。
    MissingWakeup,

    /// 尚无完整 compositor runtime、protocol/surface/render/input 生命周期。
    MissingCompleteRuntimeLoop,
}

/// Phase 51L bounded loop 的保守 capability 报告。
#[must_use = "必须区分 bounded loop、wakeup 与完整 compositor runtime"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopReadinessReport {
    /// 当前仍存在的 loop/runtime blockers。
    pub blockers: Vec<NestedRuntimeLoopBlocker>,

    /// 是否已定义 Linux-only bounded loop interface。
    pub loop_boundary_defined: bool,

    /// 是否已有 Linux proof 支持的 nested runtime loop。
    pub nested_runtime_loop_available: bool,

    /// 是否已有 Linux proof 支持的 bounded iteration loop。
    pub bounded_loop_available: bool,

    /// stop request 是否已由 Linux proof 支持。
    pub stop_requested_supported: bool,

    /// stop request 是否可唤醒正在阻塞的 event source；本阶段固定为 `false`。
    pub wakeup_supported: bool,

    /// 是否已有完整长期 compositor runtime；本阶段固定为 `false`。
    pub long_running_loop_available: bool,

    /// 是否已具备项目级 client accept 能力；本阶段固定为 `false`。
    pub accepts_clients: bool,

    /// 是否已启动长期 accept loop；本阶段固定为 `false`。
    pub runtime_accept_loop_started: bool,

    /// 是否已启动长期 protocol dispatch；本阶段固定为 `false`。
    pub protocol_dispatch_started: bool,

    /// 是否支持真实 surface；本阶段固定为 `false`。
    pub surface_support: bool,

    /// 是否支持 shell role；本阶段固定为 `false`。
    pub shell_role_support: bool,

    /// 是否支持真实 render；本阶段固定为 `false`。
    pub render_support: bool,

    /// 是否支持真实 input；本阶段固定为 `false`。
    pub input_support: bool,
}

impl NestedRuntimeLoopReadinessReport {
    /// 判断 bounded loop proof 是否完整成立。
    pub fn is_bounded_loop_ready(&self) -> bool {
        self.nested_runtime_loop_available
            && self.bounded_loop_available
            && self.stop_requested_supported
    }
}

/// 返回 Phase 51L B 路线的保守 loop readiness。
#[must_use = "bounded interface 不能代替 Linux proof 或完整 compositor runtime"]
pub fn nested_runtime_loop_readiness_report() -> NestedRuntimeLoopReadinessReport {
    NestedRuntimeLoopReadinessReport {
        blockers: vec![
            NestedRuntimeLoopBlocker::MissingLinuxBoundedLoopProof,
            NestedRuntimeLoopBlocker::MissingWakeup,
            NestedRuntimeLoopBlocker::MissingCompleteRuntimeLoop,
        ],
        loop_boundary_defined: true,
        nested_runtime_loop_available: false,
        bounded_loop_available: false,
        stop_requested_supported: false,
        wakeup_supported: false,
        long_running_loop_available: false,
        accepts_clients: false,
        runtime_accept_loop_started: false,
        protocol_dispatch_started: false,
        surface_support: false,
        shell_role_support: false,
        render_support: false,
        input_support: false,
    }
}

/// bounded loop 的有限执行配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedRuntimeLoopConfig {
    /// 最多调用 coordinator pump 的次数；`0` 会立即安全退出。
    pub max_iterations: usize,

    /// 每次 coordinator pump 允许等待 accept source 的最长时间。
    pub pump_timeout: Duration,

    /// 无 lifecycle 或 protocol 活动时是否立即以 `Idle` 退出。
    pub stop_when_idle: bool,

    /// pump 返回结构化错误后是否继续下一轮。
    pub continue_after_error: bool,
}

/// bounded loop 的退出原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeLoopExitReason {
    /// 已执行完 `max_iterations`，包括零迭代配置。
    MaxIterationsReached,

    /// `stop_when_idle` 观察到无活动 pump。
    Idle,

    /// cloneable stop handle 请求停止；请求在观察后被消费。
    StopRequested,

    /// pump 报告错误且配置要求立即退出。
    Error,
}

/// 某次 pump 在 loop 中产生的结构化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopError {
    /// 发生错误的 1-based loop iteration。
    pub iteration: usize,

    /// 该次 coordinator pump 返回的原始结构化错误。
    pub pump_errors: Vec<NestedRuntimePumpError>,
}

/// 可跨调用方 clone 的 cooperative stop request handle。
///
/// stop flag 不直接接触 coordinator 或 core。它只能在 pump 边界被观察，因此不会冒充
/// event-source wakeup；单次等待上限仍由 [`NestedRuntimeLoopConfig::pump_timeout`] 决定。
#[derive(Debug, Clone)]
pub struct NestedRuntimeLoopStopHandle {
    requested: Arc<AtomicBool>,
}

impl NestedRuntimeLoopStopHandle {
    fn new() -> Self {
        Self {
            requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 请求 loop 在下一次 pump 边界停止。
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::Release);
    }

    /// 返回尚未被 loop 消费的 stop request 状态。
    pub fn is_stop_requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }

    fn take_stop_request(&self) -> bool {
        self.requested.swap(false, Ordering::AcqRel)
    }
}

/// 一次 bounded loop run 的纯数据汇总报告。
#[must_use = "loop report 包含退出原因、pump 错误和 validation，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopReport {
    /// 实际调用 coordinator pump 的次数。
    pub iterations_run: usize,

    /// 按执行顺序保存的原始 single-pump reports。
    pub pump_reports: Vec<NestedRuntimePumpReport>,

    /// 所有 pump 注册的 core client 数量。
    pub connected_clients_registered: usize,

    /// 所有 pump 关闭的 core client 数量。
    pub disconnected_clients_closed: usize,

    /// 所有 pump 尝试调用 Display dispatch 的次数。
    pub dispatch_calls: usize,

    /// 按 iteration 保存的结构化 pump errors。
    pub errors: Vec<NestedRuntimeLoopError>,

    /// 本次 bounded run 的退出原因。
    pub exit_reason: NestedRuntimeLoopExitReason,

    /// loop 退出时核心状态是否通过 ValidationReport。
    pub validation_is_clean: bool,

    /// 当前 loop capability 快照。
    pub readiness: NestedRuntimeLoopReadinessReport,
}

impl NestedRuntimeLoopReport {
    /// 本轮是否没有 pump error，且最终 validation clean。
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
            && self.validation_is_clean
            && self.exit_reason != NestedRuntimeLoopExitReason::Error
    }
}

/// Linux-only nested lifecycle bounded runtime loop。
///
/// 本模块拥有 [`NestedRuntimeCoordinator`]，但 interface 只暴露有限执行与 cooperative
/// stop。循环实现不读取或写入 core registry；每一轮 mutation 仍严格走 coordinator 内
/// 既有的 `BackendEvent -> CoreCommand -> State` bridge。
pub struct NestedRuntimeLoop {
    coordinator: NestedRuntimeCoordinator,
    stop_handle: NestedRuntimeLoopStopHandle,
}

impl NestedRuntimeLoop {
    /// 使用指定 Wayland socket 名称创建 bounded loop。
    ///
    /// # Errors
    ///
    /// coordinator 的 Display、socket、calloop source 或 accept flow 初始化失败时返回错误。
    pub fn with_socket_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            coordinator: NestedRuntimeCoordinator::with_socket_name(name)?,
            stop_handle: NestedRuntimeLoopStopHandle::new(),
        })
    }

    /// 返回 loop 已绑定的 Wayland socket 名称。
    pub fn socket_name(&self) -> &str {
        self.coordinator.socket_name()
    }

    /// 返回可供其他调用方请求 cooperative stop 的 handle。
    pub fn stop_handle(&self) -> NestedRuntimeLoopStopHandle {
        self.stop_handle.clone()
    }

    /// 在硬性 iteration 上限内重复调用现有 coordinator pump。
    ///
    /// `max_iterations` 是防无限循环的不可绕过上限。stop、error 和 idle 只会提前退出；
    /// 本方法没有 protocol event-source wakeup，也不是完整 compositor runtime。
    pub fn run_for_iterations(
        &mut self,
        state: &mut State,
        config: NestedRuntimeLoopConfig,
    ) -> NestedRuntimeLoopReport {
        let stop_handle = self.stop_handle.clone();
        run_with_pump(state, config, &stop_handle, |state, _| {
            self.coordinator.pump_once(state, config.pump_timeout)
        })
    }
}

fn run_with_pump<F>(
    state: &mut State,
    config: NestedRuntimeLoopConfig,
    stop_handle: &NestedRuntimeLoopStopHandle,
    mut pump: F,
) -> NestedRuntimeLoopReport
where
    F: FnMut(&mut State, Duration) -> NestedRuntimePumpReport,
{
    // 不按调用方给出的上限预分配，避免极大但仍有限的 max_iterations 在 run 前触发巨额分配。
    let mut pump_reports = Vec::new();
    let mut connected_clients_registered = 0usize;
    let mut disconnected_clients_closed = 0usize;
    let mut dispatch_calls = 0usize;
    let mut errors = Vec::new();
    let mut exit_reason = NestedRuntimeLoopExitReason::MaxIterationsReached;

    if stop_handle.take_stop_request() {
        exit_reason = NestedRuntimeLoopExitReason::StopRequested;
    } else {
        for _ in 0..config.max_iterations {
            // 生产路径只能通过 coordinator pump；loop 不得绕过 bridge 直接修改 core。
            let report = pump(state, config.pump_timeout);
            let iteration = pump_reports.len().saturating_add(1);
            let report_is_idle = pump_report_is_idle(&report);
            let report_has_errors = !report.errors.is_empty();

            connected_clients_registered =
                connected_clients_registered.saturating_add(report.registered_core_clients.len());
            disconnected_clients_closed =
                disconnected_clients_closed.saturating_add(report.closed_core_clients.len());
            if report.dispatch_clients_called {
                dispatch_calls = dispatch_calls.saturating_add(1);
            }

            if report_has_errors {
                errors.push(NestedRuntimeLoopError {
                    iteration,
                    pump_errors: report.errors.clone(),
                });
            }
            pump_reports.push(report);

            if report_has_errors && !config.continue_after_error {
                exit_reason = NestedRuntimeLoopExitReason::Error;
                break;
            }
            if stop_handle.take_stop_request() {
                exit_reason = NestedRuntimeLoopExitReason::StopRequested;
                break;
            }
            if config.stop_when_idle && report_is_idle {
                exit_reason = NestedRuntimeLoopExitReason::Idle;
                break;
            }
        }
    }

    NestedRuntimeLoopReport {
        iterations_run: pump_reports.len(),
        pump_reports,
        connected_clients_registered,
        disconnected_clients_closed,
        dispatch_calls,
        errors,
        exit_reason,
        validation_is_clean: state.validate().is_clean(),
        readiness: nested_runtime_loop_readiness_report(),
    }
}

fn pump_report_is_idle(report: &NestedRuntimePumpReport) -> bool {
    report.accepted_clients == 0
        && report.inserted_clients == 0
        && report.connected_events_drained == 0
        && report.registered_core_clients.is_empty()
        && report.dispatched_requests == Some(0)
        && report.disconnected_events_drained == 0
        && report.closed_core_clients.is_empty()
        && report.errors.is_empty()
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixStream, path::Path, time::Duration};

    use super::{
        NestedRuntimeLoop, NestedRuntimeLoopBlocker, NestedRuntimeLoopConfig,
        NestedRuntimeLoopExitReason, NestedRuntimeLoopStopHandle,
        nested_runtime_loop_readiness_report, run_with_pump,
    };
    use crate::{
        core::state::State,
        smithay_backend::{
            nested_runtime_coordinator::{
                NestedRuntimePumpError, NestedRuntimePumpErrorKind, NestedRuntimePumpReport,
                nested_runtime_coordinator_readiness_report,
            },
            test_support::{assert_runtime_dir, unique_socket_name},
        },
    };

    fn config(max_iterations: usize) -> NestedRuntimeLoopConfig {
        NestedRuntimeLoopConfig {
            max_iterations,
            pump_timeout: Duration::ZERO,
            stop_when_idle: false,
            continue_after_error: false,
        }
    }

    fn synthetic_pump_report(errors: Vec<NestedRuntimePumpError>) -> NestedRuntimePumpReport {
        NestedRuntimePumpReport {
            accepted_clients: 0,
            inserted_clients: 0,
            connected_events_drained: 0,
            registered_core_clients: Vec::new(),
            dispatch_clients_called: true,
            dispatched_requests: Some(0),
            disconnected_events_drained: 0,
            closed_core_clients: Vec::new(),
            validation_is_clean: true,
            errors,
            readiness: nested_runtime_coordinator_readiness_report(),
        }
    }

    /// 验证 B 路线只定义 bounded interface，不预先声称 Linux proof 或完整 runtime。
    #[test]
    fn nested_runtime_loop_keeps_complete_runtime_capabilities_false() {
        let report = nested_runtime_loop_readiness_report();

        assert_eq!(
            report.blockers,
            vec![
                NestedRuntimeLoopBlocker::MissingLinuxBoundedLoopProof,
                NestedRuntimeLoopBlocker::MissingWakeup,
                NestedRuntimeLoopBlocker::MissingCompleteRuntimeLoop,
            ]
        );
        assert!(report.loop_boundary_defined);
        assert!(!report.nested_runtime_loop_available);
        assert!(!report.bounded_loop_available);
        assert!(!report.stop_requested_supported);
        assert!(!report.wakeup_supported);
        assert!(!report.long_running_loop_available);
        assert!(!report.accepts_clients);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(!report.is_bounded_loop_ready());
    }

    /// `max_iterations = 0` 必须安全退出，不能隐式执行一次 pump。
    #[test]
    fn nested_runtime_loop_zero_iterations_is_safe() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-zero");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let mut state = State::new();

        let report = runtime_loop.run_for_iterations(&mut state, config(0));

        assert_eq!(report.iterations_run, 0);
        assert!(report.pump_reports.is_empty());
        assert_eq!(report.dispatch_calls, 0);
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
    }

    /// max_iterations 必须成为不可绕过的硬上限，避免无限循环。
    #[test]
    fn nested_runtime_loop_respects_max_iterations() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-max");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let mut state = State::new();

        let report = runtime_loop.run_for_iterations(&mut state, config(3));

        assert_eq!(report.iterations_run, 3);
        assert_eq!(report.pump_reports.len(), 3);
        assert_eq!(report.dispatch_calls, 3);
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert!(report.validation_is_clean);
    }

    /// stop_when_idle 必须在第一次无活动 pump 后提前退出。
    #[test]
    fn nested_runtime_loop_exits_when_idle() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-idle");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let mut state = State::new();
        let mut idle_config = config(4);
        idle_config.stop_when_idle = true;

        let report = runtime_loop.run_for_iterations(&mut state, idle_config);

        assert_eq!(report.iterations_run, 1);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.validation_is_clean);
    }

    /// stop handle 可在一次 pump 后请求提前退出，且请求被消费。
    #[test]
    fn nested_runtime_loop_exits_on_stop_request() {
        let stop_handle = NestedRuntimeLoopStopHandle::new();
        let stop_from_pump = stop_handle.clone();
        let mut state = State::new();
        let mut calls = 0usize;

        let report = run_with_pump(&mut state, config(4), &stop_handle, |_, _| {
            calls = calls.saturating_add(1);
            stop_from_pump.request_stop();
            synthetic_pump_report(Vec::new())
        });

        assert_eq!(calls, 1);
        assert_eq!(report.iterations_run, 1);
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::StopRequested
        );
        assert!(!stop_handle.is_stop_requested());
        assert!(report.validation_is_clean);
    }

    /// public stop handle 在 run 前请求停止时不得额外执行 pump。
    #[test]
    fn nested_runtime_loop_public_stop_handle_exits_before_pump() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-public-stop");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let stop_handle = runtime_loop.stop_handle();
        let mut state = State::new();

        stop_handle.request_stop();
        let report = runtime_loop.run_for_iterations(&mut state, config(3));

        assert_eq!(report.iterations_run, 0);
        assert!(report.pump_reports.is_empty());
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::StopRequested
        );
        assert!(!stop_handle.is_stop_requested());
        assert!(report.validation_is_clean);
    }

    /// pump error 必须进入 loop report，并按配置以 Error 退出而不是 panic。
    #[test]
    fn nested_runtime_loop_reports_pump_error() {
        let stop_handle = NestedRuntimeLoopStopHandle::new();
        let mut state = State::new();
        let error = NestedRuntimePumpError {
            kind: NestedRuntimePumpErrorKind::DisplayDispatch,
            message: "controlled loop dispatch failure".to_owned(),
        };

        let report = run_with_pump(&mut state, config(3), &stop_handle, |_, _| {
            synthetic_pump_report(vec![error.clone()])
        });

        assert_eq!(report.iterations_run, 1);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Error);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].iteration, 1);
        assert_eq!(report.errors[0].pump_errors, vec![error]);
        assert!(!report.is_successful());
        assert!(report.validation_is_clean);
    }

    /// Linux-only 真实 proof：bounded loop 多次 pump 并保留 connected/disconnected lifecycle。
    #[test]
    fn nested_runtime_loop_runs_lifecycle_across_multiple_pumps() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-lifecycle");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("bounded loop 必须绑定测试 socket");
        let runtime_dir =
            std::env::var_os("XDG_RUNTIME_DIR").expect("Linux Smithay 测试需要 XDG_RUNTIME_DIR");
        let socket_path = Path::new(&runtime_dir).join(runtime_loop.socket_name());
        let client_stream =
            UnixStream::connect(socket_path).expect("测试 peer 必须连接真实 Wayland socket");
        let mut state = State::new();

        drop(client_stream);
        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 2,
                pump_timeout: Duration::from_secs(1),
                stop_when_idle: false,
                continue_after_error: false,
            },
        );

        assert_eq!(report.iterations_run, 2);
        assert_eq!(report.connected_clients_registered, 1);
        assert_eq!(report.disconnected_clients_closed, 1);
        assert_eq!(report.dispatch_calls, 2);
        assert!(report.errors.is_empty());
        assert_eq!(
            report.exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        let client = report.pump_reports[0].registered_core_clients[0];
        assert!(!state.clients.is_alive(client));
    }
}
