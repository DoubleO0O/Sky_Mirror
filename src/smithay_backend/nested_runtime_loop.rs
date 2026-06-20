//! Phase 51L Linux-only nested lifecycle bounded runtime loop。
//!
//! loop 只重复编排 [`NestedRuntimeCoordinator::pump_once`]，负责有限迭代、idle/error/stop
//! 退出和纯数据报告。它不直接修改 core，也不把 bounded loop 冒充完整 compositor runtime。

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use smithay::reexports::calloop::LoopSignal;

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

    /// stop request 是否可唤醒正在阻塞的 event source；Linux wakeup proof 前为 `false`。
    pub wakeup_supported: bool,

    /// 是否已有 Linux proof 支持的 interruptible poll wait。
    pub interruptible_wait_available: bool,

    /// cooperative stop 是否可打断正在进行的 pump wait。
    pub stop_can_interrupt_wait: bool,

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

    /// 判断真实 wakeup/interruptible wait proof 是否完整成立。
    pub fn is_interruptible_wait_ready(&self) -> bool {
        self.wakeup_supported && self.interruptible_wait_available && self.stop_can_interrupt_wait
    }
}

/// 返回 Phase 51M B 路线的 wakeup readiness；既有 bounded proof 保持成立。
#[must_use = "wakeup interface 不能代替 Linux interrupt proof 或完整 compositor runtime"]
pub fn nested_runtime_loop_readiness_report() -> NestedRuntimeLoopReadinessReport {
    NestedRuntimeLoopReadinessReport {
        blockers: vec![
            NestedRuntimeLoopBlocker::MissingWakeup,
            NestedRuntimeLoopBlocker::MissingCompleteRuntimeLoop,
        ],
        loop_boundary_defined: true,
        nested_runtime_loop_available: true,
        bounded_loop_available: true,
        stop_requested_supported: true,
        wakeup_supported: false,
        interruptible_wait_available: false,
        stop_can_interrupt_wait: false,
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

    /// 外部 stop+wakeup 在 pump wait 中触发，并使 poll 提前返回。
    Interrupted,
}

/// 某次 pump 在 loop 中产生的结构化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLoopError {
    /// 发生错误的 1-based loop iteration。
    pub iteration: usize,

    /// 该次 coordinator pump 返回的原始结构化错误。
    pub pump_errors: Vec<NestedRuntimePumpError>,
}

#[derive(Debug)]
struct NestedRuntimeWakeupState {
    stop_requested: AtomicBool,
    waiting: AtomicBool,
    wakeup_requested: AtomicBool,
    interrupt_requested_while_waiting: AtomicBool,
}

impl NestedRuntimeWakeupState {
    fn new() -> Self {
        Self {
            stop_requested: AtomicBool::new(false),
            waiting: AtomicBool::new(false),
            wakeup_requested: AtomicBool::new(false),
            interrupt_requested_while_waiting: AtomicBool::new(false),
        }
    }
}

/// 可跨调用方 clone 的 cooperative stop/wakeup handle。
///
/// handle 不持有 coordinator 或 core。`request_stop` 保留既有 cooperative 语义；
/// [`Self::request_stop_and_wakeup`] 额外通知 calloop poll，让等待无需耗尽完整 timeout。
#[derive(Debug, Clone)]
pub struct NestedRuntimeLoopStopHandle {
    state: Arc<NestedRuntimeWakeupState>,
    loop_signal: LoopSignal,
}

impl NestedRuntimeLoopStopHandle {
    fn new(loop_signal: LoopSignal) -> Self {
        Self {
            state: Arc::new(NestedRuntimeWakeupState::new()),
            loop_signal,
        }
    }

    /// 请求 loop 在下一次 pump 边界停止。
    pub fn request_stop(&self) {
        self.state.stop_requested.store(true, Ordering::Release);
    }

    /// 请求停止并唤醒正在等待的 calloop poll。
    ///
    /// wakeup 只制造 poll notifier event；loop 返回后仍必须通过既有 coordinator seam
    /// 完成报告和 ValidationReport，不能借此直接修改 core。
    pub fn request_stop_and_wakeup(&self) {
        self.request_stop();
        self.state.wakeup_requested.store(true, Ordering::Release);
        if self.state.waiting.load(Ordering::Acquire) {
            self.state
                .interrupt_requested_while_waiting
                .store(true, Ordering::Release);
        }
        self.loop_signal.wakeup();
    }

    /// 返回尚未被 loop 消费的 stop request 状态。
    pub fn is_stop_requested(&self) -> bool {
        self.state.stop_requested.load(Ordering::Acquire)
    }

    /// 返回 loop 当前是否位于一次 coordinator pump wait 区间。
    ///
    /// 本方法只读原子状态，供外部协调 stop+wakeup；它不驱动 pump，也不访问 core。
    pub fn is_waiting(&self) -> bool {
        self.state.waiting.load(Ordering::Acquire)
    }

    fn take_stop_request(&self) -> bool {
        self.state.stop_requested.swap(false, Ordering::AcqRel)
    }

    fn begin_wait(&self) {
        self.state.waiting.store(true, Ordering::Release);
    }

    fn end_wait(&self) {
        self.state.waiting.store(false, Ordering::Release);
    }

    fn take_wakeup_request(&self) -> bool {
        self.state.wakeup_requested.swap(false, Ordering::AcqRel)
    }

    fn take_wait_interrupt(&self) -> bool {
        self.state
            .interrupt_requested_while_waiting
            .swap(false, Ordering::AcqRel)
    }
}

/// 一次 bounded run 中观察到的 wakeup/interrupt 事实。
#[must_use = "wakeup report 区分请求、真实 wait interrupt 与完整 timeout"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedRuntimeWakeupReport {
    /// 本轮是否调用了 stop+wakeup interface。
    pub wakeup_requested: bool,

    /// 本轮是否消费了 stop request。
    pub stop_requested: bool,

    /// wakeup 是否发生在 loop 标记为 waiting 的区间。
    pub wait_interrupted: bool,

    /// 从 run 进入到退出的实际耗时。
    pub elapsed_before_exit: Duration,

    /// 本轮配置的单次 pump timeout。
    pub configured_pump_timeout: Duration,

    /// 已观察到 wait interrupt，且 run 在完整 pump timeout 前退出。
    pub exited_before_timeout: bool,
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

    /// 本轮 stop/wakeup 与 interruptible wait 事实。
    pub wakeup: NestedRuntimeWakeupReport,
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
        let coordinator = NestedRuntimeCoordinator::with_socket_name(name)?;
        let stop_handle = NestedRuntimeLoopStopHandle::new(coordinator.loop_signal());

        Ok(Self {
            coordinator,
            stop_handle,
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
    let started_at = Instant::now();
    // 不按调用方给出的上限预分配，避免极大但仍有限的 max_iterations 在 run 前触发巨额分配。
    let mut pump_reports = Vec::new();
    let mut connected_clients_registered = 0usize;
    let mut disconnected_clients_closed = 0usize;
    let mut dispatch_calls = 0usize;
    let mut errors = Vec::new();
    let mut exit_reason = NestedRuntimeLoopExitReason::MaxIterationsReached;
    let mut wakeup_requested = false;
    let mut stop_requested = false;
    let mut wait_interrupted = false;

    if stop_handle.take_stop_request() {
        stop_requested = true;
        wakeup_requested = stop_handle.take_wakeup_request();
        wait_interrupted = stop_handle.take_wait_interrupt();
        exit_reason = NestedRuntimeLoopExitReason::StopRequested;
    } else {
        for _ in 0..config.max_iterations {
            // 生产路径只能通过 coordinator pump；loop 不得绕过 bridge 直接修改 core。
            stop_handle.begin_wait();
            let report = pump(state, config.pump_timeout);
            stop_handle.end_wait();
            wakeup_requested |= stop_handle.take_wakeup_request();
            wait_interrupted |= stop_handle.take_wait_interrupt();
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
                stop_requested = true;
                exit_reason = if wait_interrupted {
                    NestedRuntimeLoopExitReason::Interrupted
                } else {
                    NestedRuntimeLoopExitReason::StopRequested
                };
                break;
            }
            if config.stop_when_idle && report_is_idle {
                exit_reason = NestedRuntimeLoopExitReason::Idle;
                break;
            }
        }
    }

    let elapsed_before_exit = started_at.elapsed();
    let exited_before_timeout = wait_interrupted && elapsed_before_exit < config.pump_timeout;

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
        wakeup: NestedRuntimeWakeupReport {
            wakeup_requested,
            stop_requested,
            wait_interrupted,
            elapsed_before_exit,
            configured_pump_timeout: config.pump_timeout,
            exited_before_timeout,
        },
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
    use std::{
        os::unix::net::UnixStream,
        path::Path,
        thread,
        time::{Duration, Instant},
    };

    use smithay::reexports::calloop::EventLoop;

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

    fn isolated_stop_handle() -> (EventLoop<'static, ()>, NestedRuntimeLoopStopHandle) {
        let event_loop: EventLoop<'static, ()> =
            EventLoop::try_new().expect("测试 stop handle 必须拥有真实 calloop notifier");
        let stop_handle = NestedRuntimeLoopStopHandle::new(event_loop.get_signal());
        (event_loop, stop_handle)
    }

    /// 验证 C 路线只上调 Linux proof 支持的 bounded/stop 字段，不冒充完整 runtime。
    #[test]
    fn nested_runtime_loop_proof_capabilities_are_precise() {
        let report = nested_runtime_loop_readiness_report();

        assert_eq!(
            report.blockers,
            vec![
                NestedRuntimeLoopBlocker::MissingWakeup,
                NestedRuntimeLoopBlocker::MissingCompleteRuntimeLoop,
            ]
        );
        assert!(report.loop_boundary_defined);
        assert!(report.nested_runtime_loop_available);
        assert!(report.bounded_loop_available);
        assert!(report.stop_requested_supported);
        assert!(!report.wakeup_supported);
        assert!(!report.interruptible_wait_available);
        assert!(!report.stop_can_interrupt_wait);
        assert!(!report.long_running_loop_available);
        assert!(!report.accepts_clients);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.is_bounded_loop_ready());
        assert!(!report.is_interruptible_wait_ready());
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
        let (_event_loop, stop_handle) = isolated_stop_handle();
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
        assert!(report.readiness.is_bounded_loop_ready());
        assert!(!report.readiness.wakeup_supported);
        assert!(!report.readiness.long_running_loop_available);
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
        let (_event_loop, stop_handle) = isolated_stop_handle();
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

    /// 真实 calloop proof：外部 wakeup 必须打断等待中的长 pump timeout。
    #[test]
    fn nested_runtime_loop_wakeup_interrupts_wait() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("nested-loop-wakeup");
        let mut runtime_loop = NestedRuntimeLoop::with_socket_name(&socket_name)
            .expect("interruptible loop 必须绑定测试 socket");
        let wakeup_handle = runtime_loop.stop_handle();
        let configured_timeout = Duration::from_secs(5);
        let mut state = State::new();

        let interrupter = thread::spawn(move || {
            let wait_deadline = Instant::now() + Duration::from_secs(1);
            while !wakeup_handle.is_waiting() {
                assert!(
                    Instant::now() < wait_deadline,
                    "loop 必须在有界时间内进入 pump wait"
                );
                thread::sleep(Duration::from_millis(1));
            }
            wakeup_handle.request_stop_and_wakeup();
        });
        let started_at = Instant::now();
        let report = runtime_loop.run_for_iterations(
            &mut state,
            NestedRuntimeLoopConfig {
                max_iterations: 1,
                pump_timeout: configured_timeout,
                stop_when_idle: false,
                continue_after_error: false,
            },
        );
        let observed_elapsed = started_at.elapsed();
        interrupter.join().expect("wakeup thread 不得 panic");

        assert_eq!(report.iterations_run, 1);
        assert_eq!(report.pump_reports.len(), 1);
        assert_eq!(report.exit_reason, NestedRuntimeLoopExitReason::Interrupted);
        assert!(report.wakeup.wakeup_requested);
        assert!(report.wakeup.stop_requested);
        assert!(report.wakeup.wait_interrupted);
        assert_eq!(report.wakeup.configured_pump_timeout, configured_timeout);
        assert!(report.wakeup.exited_before_timeout);
        assert!(report.wakeup.elapsed_before_exit < configured_timeout);
        assert!(observed_elapsed < Duration::from_secs(2));
        assert!(report.validation_is_clean);
        assert!(report.is_successful());
        assert!(!report.readiness.wakeup_supported);
        assert!(!report.readiness.long_running_loop_available);
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
        assert!(report.readiness.is_bounded_loop_ready());
        assert!(!report.readiness.wakeup_supported);
        assert!(!report.readiness.long_running_loop_available);
        let client = report.pump_reports[0].registered_core_clients[0];
        assert!(!state.clients.is_alive(client));
    }
}
