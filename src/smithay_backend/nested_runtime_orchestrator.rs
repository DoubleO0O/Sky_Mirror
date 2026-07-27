//! Phase 51N Linux-only nested runtime start/run/stop orchestration boundary。
//!
//! orchestrator 只管理 lifecycle state，并编排既有 [`NestedRuntimeLoop`]。它不读取或写入
//! core registry，不创建新的 BackendEvent/CoreCommand，也不把 bounded orchestration
//! 冒充完整 compositor runtime。

use crate::{
    core::state::State,
    smithay_backend::{
        nested_runtime_loop::{
            NestedRuntimeLiveAdmissionRunSummary, NestedRuntimeLiveUnmapRunSummary,
            NestedRuntimeLoop, NestedRuntimeLoopConfig, NestedRuntimeLoopError,
            NestedRuntimeLoopExitReason, NestedRuntimeLoopReport, NestedRuntimeLoopStopHandle,
            NestedRuntimeSurfaceCommitRunSummary,
        },
        real_accept_flow::ProductionProtocolBootstrapReport,
    },
};

/// Phase 51N orchestrator 尚未满足的独立能力条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeOrchestratorBlocker {
    /// 尚无 Linux test/CI 证明 start→run→external stop→clean shutdown。
    MissingLinuxLifecycleProof,

    /// 尚无日常 runtime 入口、完整 accept/protocol/surface/render/input 生命周期。
    MissingCompleteRuntimeLoop,
}

/// Phase 51N orchestrator 的保守 capability 报告。
#[must_use = "必须区分 lifecycle orchestration 与完整 compositor runtime"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeOrchestratorReadinessReport {
    /// 当前仍存在的 orchestration/runtime blockers。
    pub blockers: Vec<NestedRuntimeOrchestratorBlocker>,

    /// 是否已定义 Linux-only start/run/stop interface。
    pub orchestration_boundary_defined: bool,

    /// 是否已有 Linux proof 支持的 runtime orchestrator。
    pub runtime_orchestrator_available: bool,

    /// 是否已有 Linux proof 支持的 start/run/stop lifecycle。
    pub start_run_stop_available: bool,

    /// 是否已有 Linux proof 支持的 external stop+wakeup。
    pub external_stop_supported: bool,

    /// 是否已有 Linux proof 支持的 clean shutdown/final report。
    pub clean_shutdown_supported: bool,

    /// 是否已有完整长期 compositor runtime；本阶段固定为 `false`。
    pub long_running_loop_available: bool,

    /// 是否具备项目级 client accept 能力；本阶段固定为 `false`。
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

impl NestedRuntimeOrchestratorReadinessReport {
    /// 判断 start/run/stop proof 是否完整成立。
    pub fn is_start_run_stop_ready(&self) -> bool {
        self.runtime_orchestrator_available
            && self.start_run_stop_available
            && self.external_stop_supported
            && self.clean_shutdown_supported
    }
}

/// 返回 Phase 51N C 路线的精确 orchestration readiness。
#[must_use = "orchestration proof 不能代替完整 compositor runtime"]
pub fn nested_runtime_orchestrator_readiness_report() -> NestedRuntimeOrchestratorReadinessReport {
    NestedRuntimeOrchestratorReadinessReport {
        blockers: vec![NestedRuntimeOrchestratorBlocker::MissingCompleteRuntimeLoop],
        orchestration_boundary_defined: true,
        runtime_orchestrator_available: true,
        start_run_stop_available: true,
        external_stop_supported: true,
        clean_shutdown_supported: true,
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

/// nested runtime orchestration lifecycle state。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeLifecycleState {
    /// 配置已存在，但尚未创建真实 loop 资源。
    Created,

    /// loop 已创建并可进入 run。
    Started,

    /// 当前正在执行 bounded loop。
    Running,

    /// 已观察到 run 退出或 stop request，正在完成结构化收尾。
    Stopping,

    /// orchestration 已安全停止，不允许重新 start。
    Stopped,

    /// start 或 run 失败。
    Failed,
}

/// 可能触发 lifecycle transition 的公开操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedRuntimeOrchestratorOperation {
    /// 创建 loop 资源。
    Start,

    /// 执行 bounded loop。
    Run,

    /// 请求或完成停止。
    Stop,

    /// 获取 external stop+wakeup handle。
    StopHandle,
}

/// start/run/stop 的结构化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedRuntimeOrchestratorError {
    /// 当前 lifecycle state 不允许指定操作。
    InvalidTransition {
        /// 拒绝操作时的 state。
        state: NestedRuntimeLifecycleState,

        /// 被拒绝的操作。
        operation: NestedRuntimeOrchestratorOperation,
    },

    /// start 创建真实 loop 资源失败。
    StartFailed {
        /// 底层错误文本，仅用于诊断。
        message: String,
    },

    /// state 声称 loop 应存在，但内部资源缺失。
    MissingRuntimeLoop {
        /// 检测到不一致时的 state。
        state: NestedRuntimeLifecycleState,
    },
}

/// orchestrator 的固定 socket 与 bounded loop 配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeOrchestratorConfig {
    /// start 时绑定的 Wayland socket 名称。
    pub socket_name: String,

    /// run 时传给现有 nested runtime loop 的有限配置。
    pub loop_config: NestedRuntimeLoopConfig,
}

/// 一次 start transition 的纯数据报告。
#[must_use = "start report 必须用于确认资源创建与 lifecycle state"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeStartReport {
    /// start 前的 state。
    pub previous_state: NestedRuntimeLifecycleState,

    /// start 后的 state。
    pub state: NestedRuntimeLifecycleState,

    /// 是否真实创建了 nested runtime loop。
    pub started: bool,

    /// loop 实际绑定的 socket 名称。
    pub socket_name: String,
}

/// 一次直接 stop/finalize 的纯数据报告。
#[must_use = "stop report 必须用于确认 stop request、shutdown 与 validation"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeStopReport {
    /// stop 前的 state。
    pub previous_state: NestedRuntimeLifecycleState,

    /// stop 后的 state。
    pub state: NestedRuntimeLifecycleState,

    /// 是否向已有 loop 提交 stop flag。
    pub stop_requested: bool,

    /// 是否同时提交 calloop wakeup。
    pub wakeup_requested: bool,

    /// orchestrator 是否完成结构化停止。
    pub shutdown_completed: bool,

    /// stop 完成时核心状态是否通过 ValidationReport。
    pub validation_is_clean: bool,
}

/// start 后一次 run 到最终 lifecycle state 的汇总报告。
#[must_use = "lifecycle report 包含 loop 退出、shutdown、错误与 validation，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRuntimeLifecycleReport {
    /// run 是否从 Started 状态进入。
    pub started: bool,

    /// 是否真实进入 bounded loop。
    pub run_entered: bool,

    /// loop 是否消费了 stop request。
    pub stop_requested: bool,

    /// loop 是否观察到 wakeup request。
    pub wakeup_requested: bool,

    /// lifecycle 是否完成到 Stopped。
    pub shutdown_completed: bool,

    /// 既有 loop 的退出原因。
    pub loop_exit_reason: NestedRuntimeLoopExitReason,

    /// 实际执行的 pump iteration 数量。
    pub pump_iterations: usize,

    /// 通过既有 bridge 注册的 core client 数量。
    pub registered_clients: usize,

    /// 通过既有 bridge 关闭的 core client 数量。
    pub closed_clients: usize,

    /// loop 返回的原始结构化错误。
    pub errors: Vec<NestedRuntimeLoopError>,

    /// 最终 lifecycle state。
    pub final_state: NestedRuntimeLifecycleState,

    /// run 退出时核心状态是否通过 ValidationReport。
    pub validation_is_clean: bool,

    /// 本轮 live admission enqueue/drain 事实。
    pub live_admission: NestedRuntimeLiveAdmissionRunSummary,

    /// 本轮 live toplevel unmap drain 事实。
    pub live_unmap: NestedRuntimeLiveUnmapRunSummary,

    /// 本轮 `wl_surface.commit` backlog drain 事实。
    pub surface_commit: NestedRuntimeSurfaceCommitRunSummary,

    /// 完整原始 bounded-loop report。
    pub loop_report: NestedRuntimeLoopReport,

    /// 当前 orchestration capability 快照。
    pub readiness: NestedRuntimeOrchestratorReadinessReport,
}

impl NestedRuntimeLifecycleReport {
    /// 本轮是否完成无错误、validation-clean 的结构化停止。
    pub fn is_clean_shutdown(&self) -> bool {
        self.shutdown_completed
            && self.final_state == NestedRuntimeLifecycleState::Stopped
            && self.errors.is_empty()
            && self.validation_is_clean
    }
}

/// Linux-only nested runtime start/run/stop orchestrator。
///
/// orchestrator 只保存 lifecycle policy 与 [`NestedRuntimeLoop`] ownership。所有 client
/// mutation 继续由 loop/coordinator 内的既有 bridge 完成；本类型只读 ValidationReport。
pub struct NestedRuntimeOrchestrator {
    config: NestedRuntimeOrchestratorConfig,
    state: NestedRuntimeLifecycleState,
    runtime_loop: Option<NestedRuntimeLoop>,
}

impl NestedRuntimeOrchestrator {
    /// 创建尚未绑定 socket 或启动 loop 的 orchestrator。
    pub fn new(config: NestedRuntimeOrchestratorConfig) -> Self {
        Self {
            config,
            state: NestedRuntimeLifecycleState::Created,
            runtime_loop: None,
        }
    }

    /// 返回当前 lifecycle state。
    pub fn state(&self) -> NestedRuntimeLifecycleState {
        self.state
    }

    /// 从 Created 创建真实 nested runtime loop 并进入 Started。
    ///
    /// # Errors
    ///
    /// 非 Created 状态返回 structured transition error；socket/Display/calloop 初始化失败
    /// 返回 `StartFailed` 并进入 Failed。
    pub fn start(&mut self) -> Result<NestedRuntimeStartReport, NestedRuntimeOrchestratorError> {
        if self.state != NestedRuntimeLifecycleState::Created {
            return Err(self.invalid_transition(NestedRuntimeOrchestratorOperation::Start));
        }

        let previous_state = self.state;
        match NestedRuntimeLoop::with_production_protocol_bootstrap(&self.config.socket_name) {
            Ok(runtime_loop) => {
                let socket_name = runtime_loop.socket_name().to_owned();
                self.runtime_loop = Some(runtime_loop);
                self.state = NestedRuntimeLifecycleState::Started;
                Ok(NestedRuntimeStartReport {
                    previous_state,
                    state: self.state,
                    started: true,
                    socket_name,
                })
            }
            Err(error) => {
                self.state = NestedRuntimeLifecycleState::Failed;
                Err(NestedRuntimeOrchestratorError::StartFailed {
                    message: error.to_string(),
                })
            }
        }
    }

    /// 返回 external stop+wakeup handle。
    ///
    /// # Errors
    ///
    /// 只有 Started 或 Running 状态允许获取；Created/Stopped/Failed 返回 transition error。
    pub fn stop_handle(
        &self,
    ) -> Result<NestedRuntimeLoopStopHandle, NestedRuntimeOrchestratorError> {
        if !matches!(
            self.state,
            NestedRuntimeLifecycleState::Started | NestedRuntimeLifecycleState::Running
        ) {
            return Err(self.invalid_transition(NestedRuntimeOrchestratorOperation::StopHandle));
        }

        self.runtime_loop
            .as_ref()
            .map(NestedRuntimeLoop::stop_handle)
            .ok_or(NestedRuntimeOrchestratorError::MissingRuntimeLoop { state: self.state })
    }

    /// 返回当前 loop owner 可直接证明的 production bootstrap server truth。orchestrator
    /// 只转发，不为外部 registry/client bind 填充成功，也不将有限 lifecycle 编排扩张为
    /// buffer import、texture/renderer、damage、frame callback、input 或 core 已执行的结论。
    pub(crate) fn production_protocol_bootstrap_report(
        &self,
    ) -> Option<ProductionProtocolBootstrapReport> {
        self.runtime_loop
            .as_ref()
            .and_then(NestedRuntimeLoop::production_protocol_bootstrap_report)
    }

    /// 从 Started 进入 Running，执行既有 bounded loop，并生成 final lifecycle report。
    ///
    /// # Errors
    ///
    /// 未 start、已 stop 或资源不一致时返回 structured error，不执行 pump。
    pub fn run(
        &mut self,
        state: &mut State,
    ) -> Result<NestedRuntimeLifecycleReport, NestedRuntimeOrchestratorError> {
        if self.state != NestedRuntimeLifecycleState::Started {
            return Err(self.invalid_transition(NestedRuntimeOrchestratorOperation::Run));
        }

        self.state = NestedRuntimeLifecycleState::Running;
        let Some(runtime_loop) = self.runtime_loop.as_mut() else {
            self.state = NestedRuntimeLifecycleState::Failed;
            return Err(NestedRuntimeOrchestratorError::MissingRuntimeLoop { state: self.state });
        };

        // 只调用既有 loop seam；orchestrator 不读取或修改任何 core registry。
        let loop_report = runtime_loop.run_for_iterations(state, self.config.loop_config);
        Ok(self.finish_run(loop_report))
    }

    fn finish_run(&mut self, loop_report: NestedRuntimeLoopReport) -> NestedRuntimeLifecycleReport {
        let loop_failed = loop_report.exit_reason == NestedRuntimeLoopExitReason::Error
            || !loop_report.validation_is_clean;
        if loop_failed {
            self.state = NestedRuntimeLifecycleState::Failed;
        } else {
            self.state = NestedRuntimeLifecycleState::Stopping;
            self.state = NestedRuntimeLifecycleState::Stopped;
        }

        NestedRuntimeLifecycleReport {
            started: true,
            run_entered: true,
            stop_requested: loop_report.wakeup.stop_requested,
            wakeup_requested: loop_report.wakeup.wakeup_requested,
            shutdown_completed: self.state == NestedRuntimeLifecycleState::Stopped,
            loop_exit_reason: loop_report.exit_reason,
            pump_iterations: loop_report.iterations_run,
            registered_clients: loop_report.connected_clients_registered,
            closed_clients: loop_report.disconnected_clients_closed,
            errors: loop_report.errors.clone(),
            final_state: self.state,
            validation_is_clean: loop_report.validation_is_clean,
            live_admission: loop_report.live_admission,
            live_unmap: loop_report.live_unmap,
            surface_commit: loop_report.surface_commit.clone(),
            loop_report,
            readiness: nested_runtime_orchestrator_readiness_report(),
        }
    }

    /// 安全停止尚未运行或已完成的 orchestrator。
    ///
    /// Created 直接进入 Stopped；Started/Running 会通过既有 handle 请求 stop+wakeup；
    /// Stopping/Stopped 为幂等 finalize。该方法不直接修改 core。
    pub fn stop(&mut self, state: &State) -> NestedRuntimeStopReport {
        let previous_state = self.state;
        let mut stop_requested = false;
        let mut wakeup_requested = false;

        if matches!(
            self.state,
            NestedRuntimeLifecycleState::Started | NestedRuntimeLifecycleState::Running
        ) && let Some(runtime_loop) = self.runtime_loop.as_ref()
        {
            runtime_loop.stop_handle().request_stop_and_wakeup();
            stop_requested = true;
            wakeup_requested = true;
            self.state = NestedRuntimeLifecycleState::Stopping;
        }

        self.state = NestedRuntimeLifecycleState::Stopped;
        NestedRuntimeStopReport {
            previous_state,
            state: self.state,
            stop_requested,
            wakeup_requested,
            shutdown_completed: true,
            validation_is_clean: state.validate().is_clean(),
        }
    }

    fn invalid_transition(
        &self,
        operation: NestedRuntimeOrchestratorOperation,
    ) -> NestedRuntimeOrchestratorError {
        NestedRuntimeOrchestratorError::InvalidTransition {
            state: self.state,
            operation,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        os::unix::net::UnixStream,
        path::PathBuf,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    use wayland_client::{
        Connection, Dispatch, QueueHandle,
        globals::{GlobalListContents, registry_queue_init},
        protocol::{wl_compositor::WlCompositor, wl_registry::WlRegistry},
    };
    use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

    use super::{
        NestedRuntimeLifecycleState, NestedRuntimeOrchestrator, NestedRuntimeOrchestratorBlocker,
        NestedRuntimeOrchestratorConfig, NestedRuntimeOrchestratorError,
        NestedRuntimeOrchestratorOperation, nested_runtime_orchestrator_readiness_report,
    };
    use crate::{
        core::state::State,
        smithay_backend::{
            linux_toplevel_identity_registration::adapter_toplevel_identity_registration_report,
            linux_wl_surface_identity::{
                controlled_wl_surface_commit_observation_report,
                controlled_wl_surface_damage_commit_observation_report,
                controlled_wl_surface_frame_callback_commit_observation_report,
                controlled_wl_surface_null_attach_commit_observation_report,
                controlled_wl_surface_render_dirty_readiness_commit_observation_report,
            },
            nested_runtime_coordinator::{NestedRuntimePumpError, NestedRuntimePumpErrorKind},
            nested_runtime_loop::{
                NestedRuntimeLoopConfig, NestedRuntimeLoopError, NestedRuntimeLoopExitReason,
            },
            real_accept_flow::ProductionProtocolBootstrapReport,
            test_support::{assert_runtime_dir, unique_socket_name},
        },
    };

    fn config(name: &str, max_iterations: usize) -> NestedRuntimeOrchestratorConfig {
        NestedRuntimeOrchestratorConfig {
            socket_name: unique_socket_name(name),
            loop_config: NestedRuntimeLoopConfig {
                max_iterations,
                pump_timeout: Duration::ZERO,
                stop_when_idle: false,
                continue_after_error: false,
            },
        }
    }

    fn assert_start_keeps_production_globals_initialized(
        display: &crate::smithay_backend::wayland_display::SmithayWaylandDisplayProbe,
    ) {
        // `start()` 现在按 production owner 顺序完成两个 global bootstrap；既有受控
        // observation 测试仍可借 display 做后续登记/commit proof，但不得再次初始化 owner。
        // 这里显式验证已存在的前置条件，保留 controlled 语义，同时不把它外推为 client
        // discovery/bind 或 buffer/render/core 的完成事实。
        assert!(display.is_xdg_shell_global_initialized());
        assert!(display.is_wl_compositor_global_initialized());
    }

    const PRODUCTION_PROTOCOL_TEST_TIMEOUT: Duration = Duration::from_secs(5);
    const PRODUCTION_PROTOCOL_PUMP_TIMEOUT: Duration = Duration::from_millis(5);
    const PRODUCTION_PROTOCOL_MAX_PUMPS: usize = 1_000;
    const PRODUCTION_PROTOCOL_JOIN_POLL: Duration = Duration::from_millis(1);

    /// 外部 Wayland client 的纯数据证据；它刻意只存在于测试模块，不能流入 production
    /// getter 或 public API。服务端只能证明 owner 已完成 bootstrap，不能独立声称外部
    /// registry discovery/bind 成功；两类事实必须由此处真实 client 的独立 roundtrip 分开记录。
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    struct ExternalProtocolRoundtripEvidence {
        client_connected: bool,
        registry_roundtrip_completed: bool,
        wl_compositor_discovered: bool,
        xdg_wm_base_discovered: bool,
        wl_compositor_bound: bool,
        xdg_wm_base_bound: bool,
        client_finished_before_deadline: bool,
        server_finished_before_deadline: bool,
        socket_removed_after_drop: bool,
    }

    #[derive(Default)]
    struct ProductionProtocolClientState;

    impl Dispatch<WlRegistry, GlobalListContents> for ProductionProtocolClientState {
        fn event(
            _state: &mut Self,
            _proxy: &WlRegistry,
            _event: wayland_client::protocol::wl_registry::Event,
            _data: &GlobalListContents,
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
        }
    }

    wayland_client::delegate_noop!(ProductionProtocolClientState: ignore WlCompositor);
    wayland_client::delegate_noop!(ProductionProtocolClientState: ignore XdgWmBase);

    fn join_production_protocol_client_before_deadline(
        client_thread: thread::JoinHandle<()>,
    ) -> Result<(), String> {
        let deadline = Instant::now() + PRODUCTION_PROTOCOL_TEST_TIMEOUT;

        // Join 只在固定次数与固定 deadline 双重约束内轮询；这避免测试在 client panic、
        // 意外阻塞或 socket 生命周期异常时无限等待，同时仍让正常路径显式回收 thread owner。
        for _ in 0..PRODUCTION_PROTOCOL_MAX_PUMPS {
            if client_thread.is_finished() {
                return client_thread
                    .join()
                    .map_err(|_| "外部 protocol client thread panic".to_owned());
            }
            if Instant::now() >= deadline {
                return Err("外部 protocol client thread 未在 5 秒 deadline 前结束".to_owned());
            }
            thread::sleep(PRODUCTION_PROTOCOL_JOIN_POLL);
        }

        Err("外部 protocol client thread 超过固定 join 轮询上限".to_owned())
    }

    fn production_orchestrator_external_registry_roundtrip_evidence() -> Result<
        (
            ExternalProtocolRoundtripEvidence,
            ProductionProtocolBootstrapReport,
            usize,
        ),
        String,
    > {
        assert_runtime_dir();
        let test_started_at = Instant::now();
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .ok_or_else(|| "测试缺少 XDG_RUNTIME_DIR".to_owned())?;
        let mut runtime_config = config(
            "production-protocol-registry-roundtrip",
            PRODUCTION_PROTOCOL_MAX_PUMPS,
        );
        runtime_config.loop_config.pump_timeout = PRODUCTION_PROTOCOL_PUMP_TIMEOUT;
        let socket_path = PathBuf::from(runtime_dir).join(&runtime_config.socket_name);
        let mut state = State::new();
        let mut orchestrator = NestedRuntimeOrchestrator::new(runtime_config);
        let mut client_thread = None;

        let result = (|| -> Result<_, String> {
            let _start_report = orchestrator
                .start()
                .map_err(|error| format!("orchestrator start 失败: {error:?}"))?;
            if !socket_path.exists() {
                return Err("orchestrator start 后测试 socket path 不存在".to_owned());
            }
            let server_report = orchestrator
                .production_protocol_bootstrap_report()
                .ok_or_else(|| {
                    "production orchestrator start 后缺少 server bootstrap report".to_owned()
                })?;

            let stop_handle = orchestrator
                .stop_handle()
                .map_err(|error| format!("orchestrator stop handle 获取失败: {error:?}"))?;
            let (client_connected_sender, client_connected_receiver) = mpsc::channel();
            let (client_start_sender, client_start_receiver) = mpsc::channel();
            let (client_result_sender, client_result_receiver) = mpsc::channel();
            let client_socket_path = socket_path.clone();

            client_thread = Some(thread::spawn(move || {
                let client_result = (|| -> Result<ExternalProtocolRoundtripEvidence, String> {
                    let client_stream = UnixStream::connect(&client_socket_path)
                        .map_err(|error| format!("外部 client 连接测试 socket 失败: {error}"))?;
                    client_stream
                        .set_read_timeout(Some(PRODUCTION_PROTOCOL_TEST_TIMEOUT))
                        .map_err(|error| {
                            format!("外部 client 设置 socket read timeout 失败: {error}")
                        })?;
                    client_stream
                        .set_write_timeout(Some(PRODUCTION_PROTOCOL_TEST_TIMEOUT))
                        .map_err(|error| {
                            format!("外部 client 设置 socket write timeout 失败: {error}")
                        })?;
                    client_connected_sender
                        .send(())
                        .map_err(|_| "主线程在 client 连通前提前退出".to_owned())?;
                    // 只有主线程在固定 deadline 内观察到连通后才发出开始信号；客户端的
                    // 等待同样使用 recv_timeout，避免 barrier::wait 这类无超时原语在
                    // panic 或调度异常时永久卡住。主线程发信号后立即进入有限 server pump。
                    client_start_receiver
                        .recv_timeout(PRODUCTION_PROTOCOL_TEST_TIMEOUT)
                        .map_err(|error| {
                            format!("外部 client 未在 5 秒内收到 server run 信号: {error}")
                        })?;

                    let connection = Connection::from_socket(client_stream).map_err(|error| {
                        format!("外部 client 创建 Wayland connection 失败: {error}")
                    })?;
                    let (globals, mut event_queue) =
                        registry_queue_init::<ProductionProtocolClientState>(&connection).map_err(
                            |error| format!("外部 client 初始 registry roundtrip 失败: {error}"),
                        )?;
                    let (wl_compositor_discovered, xdg_wm_base_discovered) =
                        globals.contents().with_list(|registered_globals| {
                            (
                                registered_globals
                                    .iter()
                                    .any(|global| global.interface == "wl_compositor"),
                                registered_globals
                                    .iter()
                                    .any(|global| global.interface == "xdg_wm_base"),
                            )
                        });
                    let mut evidence = ExternalProtocolRoundtripEvidence {
                        client_connected: true,
                        registry_roundtrip_completed: true,
                        wl_compositor_discovered,
                        xdg_wm_base_discovered,
                        ..ExternalProtocolRoundtripEvidence::default()
                    };

                    if wl_compositor_discovered && xdg_wm_base_discovered {
                        let queue_handle = event_queue.handle();
                        let _compositor = globals
                            .bind::<WlCompositor, _, _>(&queue_handle, 1..=5, ())
                            .map_err(|error| {
                                format!("外部 client bind wl_compositor 失败: {error}")
                            })?;
                        evidence.wl_compositor_bound = true;
                        let _xdg_wm_base = globals
                            .bind::<XdgWmBase, _, _>(&queue_handle, 1..=7, ())
                            .map_err(|error| {
                                format!("外部 client bind xdg_wm_base 失败: {error}")
                            })?;
                        evidence.xdg_wm_base_bound = true;
                        let mut client_state = ProductionProtocolClientState;
                        event_queue.roundtrip(&mut client_state).map_err(|error| {
                            format!("外部 client bind 后 event-queue roundtrip 失败: {error}")
                        })?;
                    }

                    Ok(evidence)
                })();

                // 不论 discovery/bind 是成功、缺 global 还是 protocol error，client 都必须
                // 先回传结构化结果再请求 stop+wakeup。这样 server 的 owner 仍只由
                // orchestrator 生命周期回收，客户端不会通过无界等待或直接 display 控制 API
                // 影响它；buffer/render/core 工作也继续明确延后，绝不由本测试伪造完成。
                let _ = client_result_sender.send(client_result);
                stop_handle.request_stop_and_wakeup();
            }));

            client_connected_receiver
                .recv_timeout(PRODUCTION_PROTOCOL_TEST_TIMEOUT)
                .map_err(|error| format!("未在 5 秒内观察到外部 client 连通: {error}"))?;
            client_start_sender
                .send(())
                .map_err(|_| "外部 client 在 server run 信号前提前退出".to_owned())?;

            let server_started_at = Instant::now();
            let lifecycle_report = orchestrator
                .run(&mut state)
                .map_err(|error| format!("orchestrator bounded run 失败: {error:?}"))?;
            let server_finished_before_deadline =
                server_started_at.elapsed() <= PRODUCTION_PROTOCOL_TEST_TIMEOUT;

            let mut evidence = client_result_receiver
                .recv_timeout(PRODUCTION_PROTOCOL_TEST_TIMEOUT)
                .map_err(|error| format!("未在 5 秒内收到外部 client 结果: {error}"))??;
            evidence.server_finished_before_deadline = server_finished_before_deadline;
            Ok((evidence, server_report, lifecycle_report.pump_iterations))
        })();

        // 无论上面的任一步骤如何返回，都先通过公开 stop 生命周期请求 owner 收尾，再 drop
        // orchestrator 释放 socket/source。随后单独检查 path 已删除，避免把内存 drop 或
        // client thread 结束误当作 socket cleanup 已经证明。
        let _stop_report = orchestrator.stop(&state);
        drop(orchestrator);
        let socket_removed_after_drop = !socket_path.exists();
        let join_result = client_thread
            .map(join_production_protocol_client_before_deadline)
            .unwrap_or(Ok(()));

        match (result, join_result) {
            (Ok((mut evidence, server_report, pump_iterations)), Ok(())) => {
                // 该字段只在 JoinHandle 已成功回收后填写，因此它证明的是 client thread
                // 本身在 deadline 内结束，而不是仅证明 client closure 已准备返回或已经
                // 发送 result。client 与 server 的完成事实仍分开记录，避免并发时序混淆。
                evidence.client_finished_before_deadline =
                    test_started_at.elapsed() <= PRODUCTION_PROTOCOL_TEST_TIMEOUT;
                evidence.socket_removed_after_drop = socket_removed_after_drop;
                if !socket_removed_after_drop {
                    return Err("orchestrator drop 后测试 socket path 未清理".to_owned());
                }
                Ok((evidence, server_report, pump_iterations))
            }
            (Ok(_), Err(error)) => Err(error),
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(join_error)) => Err(format!("{error}; {join_error}")),
        }
    }

    #[test]
    fn production_orchestrator_external_registry_roundtrip() {
        let (evidence, server_report, pump_iterations) =
            production_orchestrator_external_registry_roundtrip_evidence()
                .expect("外部 registry roundtrip harness 必须在固定 deadline 内完成并清理 socket");

        assert!((1..=PRODUCTION_PROTOCOL_MAX_PUMPS).contains(&pump_iterations));
        println!(
            "phase56p production protocol roundtrip: actual_pumps={pump_iterations}/{} test_timeout={:?} pump_timeout={:?} join_poll={:?} external={evidence:?} server={server_report:?}",
            PRODUCTION_PROTOCOL_MAX_PUMPS,
            PRODUCTION_PROTOCOL_TEST_TIMEOUT,
            PRODUCTION_PROTOCOL_PUMP_TIMEOUT,
            PRODUCTION_PROTOCOL_JOIN_POLL,
        );
        assert!(evidence.client_connected);
        assert!(evidence.registry_roundtrip_completed);
        assert!(evidence.wl_compositor_discovered);
        assert!(evidence.xdg_wm_base_discovered);
        assert!(evidence.wl_compositor_bound);
        assert!(evidence.xdg_wm_base_bound);
        assert!(evidence.client_finished_before_deadline);
        assert!(evidence.server_finished_before_deadline);
        assert!(evidence.socket_removed_after_drop);
        assert!(server_report.bootstrap_attempted);
        assert!(server_report.wl_compositor_initialized);
        assert!(server_report.xdg_wm_base_initialized);
        assert!(server_report.socket_bound_after_bootstrap);
        assert!(!server_report.external_registry_discovered_both_globals);
        assert!(!server_report.external_client_bound_both_globals);
        assert!(!server_report.buffer_import_attempted);
        assert!(!server_report.buffer_imported);
        assert!(!server_report.texture_created);
        assert!(!server_report.renderer_called);
        assert!(!server_report.damage_submitted);
        assert!(!server_report.frame_callback_done_sent);
        assert!(!server_report.input_support);
        assert!(!server_report.core_mutation_invoked);
    }

    /// C 路线只上调 Linux CI 已证明的 start/run/stop capability。
    #[test]
    fn runtime_orchestrator_linux_proof_capabilities_are_precise() {
        let report = nested_runtime_orchestrator_readiness_report();

        assert_eq!(
            report.blockers,
            vec![NestedRuntimeOrchestratorBlocker::MissingCompleteRuntimeLoop]
        );
        assert!(report.orchestration_boundary_defined);
        assert!(report.runtime_orchestrator_available);
        assert!(report.start_run_stop_available);
        assert!(report.external_stop_supported);
        assert!(report.clean_shutdown_supported);
        assert!(!report.long_running_loop_available);
        assert!(!report.accepts_clients);
        assert!(!report.runtime_accept_loop_started);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.surface_support);
        assert!(!report.shell_role_support);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.is_start_run_stop_ready());
    }

    /// start 必须真实创建 loop，并从 Created 转换到 Started。
    #[test]
    fn runtime_orchestrator_start_transitions_state() {
        assert_runtime_dir();
        let mut orchestrator = NestedRuntimeOrchestrator::new(config("orchestrator-start", 0));

        let report = orchestrator.start().expect("Created 必须允许 start");

        assert!(report.started);
        assert_eq!(report.previous_state, NestedRuntimeLifecycleState::Created);
        assert_eq!(report.state, NestedRuntimeLifecycleState::Started);
        assert_eq!(orchestrator.state(), NestedRuntimeLifecycleState::Started);
        assert!(!report.socket_name.is_empty());
    }

    /// 重复 start 必须返回结构化 transition error，不覆盖已有 loop。
    #[test]
    fn runtime_orchestrator_rejects_double_start() {
        assert_runtime_dir();
        let mut orchestrator = NestedRuntimeOrchestrator::new(config("orchestrator-double", 0));
        orchestrator.start().expect("首次 start 必须成功");

        let error = orchestrator.start().expect_err("重复 start 必须失败");

        assert_eq!(
            error,
            NestedRuntimeOrchestratorError::InvalidTransition {
                state: NestedRuntimeLifecycleState::Started,
                operation: NestedRuntimeOrchestratorOperation::Start,
            }
        );
        assert_eq!(orchestrator.state(), NestedRuntimeLifecycleState::Started);
    }

    /// 未 start 的 run 必须返回结构化 error，且不执行 pump。
    #[test]
    fn runtime_orchestrator_run_requires_start() {
        let mut orchestrator = NestedRuntimeOrchestrator::new(config("orchestrator-no-start", 1));
        let mut state = State::new();

        let error = orchestrator
            .run(&mut state)
            .expect_err("Created 不得直接 run");

        assert_eq!(
            error,
            NestedRuntimeOrchestratorError::InvalidTransition {
                state: NestedRuntimeLifecycleState::Created,
                operation: NestedRuntimeOrchestratorOperation::Run,
            }
        );
        assert_eq!(orchestrator.state(), NestedRuntimeLifecycleState::Created);
        assert!(state.validate().is_clean());
    }

    /// 未 start 的 stop 必须安全完成，不创建 loop 或修改 core。
    #[test]
    fn runtime_orchestrator_stop_before_start_is_safe() {
        let mut orchestrator = NestedRuntimeOrchestrator::new(config("orchestrator-stop", 1));
        let state = State::new();

        let report = orchestrator.stop(&state);

        assert_eq!(report.previous_state, NestedRuntimeLifecycleState::Created);
        assert_eq!(report.state, NestedRuntimeLifecycleState::Stopped);
        assert!(!report.stop_requested);
        assert!(!report.wakeup_requested);
        assert!(report.shutdown_completed);
        assert!(report.validation_is_clean);
    }

    /// 零迭代 run 仍必须完成 Started→Running→Stopped 与 clean final report。
    #[test]
    fn runtime_orchestrator_zero_iteration_run_shuts_down_cleanly() {
        assert_runtime_dir();
        let mut orchestrator = NestedRuntimeOrchestrator::new(config("orchestrator-zero", 0));
        let mut state = State::new();
        orchestrator.start().expect("Created 必须允许 start");

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.started);
        assert!(report.run_entered);
        assert_eq!(report.pump_iterations, 0);
        assert_eq!(
            report.loop_exit_reason,
            NestedRuntimeLoopExitReason::MaxIterationsReached
        );
        assert_eq!(report.final_state, NestedRuntimeLifecycleState::Stopped);
        assert!(report.shutdown_completed);
        assert!(report.validation_is_clean);
        assert!(report.is_clean_shutdown());
    }

    /// Linux-only proof：orchestrator run 通过 loop live admission pump 完成 xdg_toplevel admission。
    #[test]
    fn runtime_orchestrator_run_drains_live_toplevel_admission() {
        assert_runtime_dir();
        let mut orchestrator =
            NestedRuntimeOrchestrator::new(config("orchestrator-live-admission", 1));
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let registration = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            adapter_toplevel_identity_registration_report(display)
                .expect("adapter identity registration proof 必须完成")
        };

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert_eq!(report.pump_iterations, 1);
        assert!(report.is_clean_shutdown());
        assert_eq!(report.final_state, NestedRuntimeLifecycleState::Stopped);
        assert!(report.loop_report.is_successful());
        assert_eq!(report.live_admission.owner_invocations, 1);
        assert_eq!(report.live_admission.enqueue_invocations, 1);
        assert_eq!(report.live_admission.admissions_enqueued, 1);
        assert_eq!(report.live_admission.drain_invocations, 1);
        assert_eq!(report.live_admission.admissions_consumed, 1);
        assert_eq!(report.live_admission.pending_admissions_after, 0);
        assert_eq!(report.live_admission, report.loop_report.live_admission);
        assert_eq!(report.live_unmap, report.loop_report.live_unmap);
        assert_eq!(report.live_unmap.drain_invocations, 1);
        let runtime_loop = orchestrator
            .runtime_loop
            .as_ref()
            .expect("Stopped orchestrator 保留 runtime loop report owner");
        assert_eq!(
            runtime_loop.admission_surface_mapping(registration.adapter_surface_id),
            Some(1)
        );
        let toplevel_mapping =
            runtime_loop.admission_toplevel_mapping(registration.adapter_toplevel_id);
        if report.live_unmap.ledger_unmaps > 0 {
            assert_eq!(toplevel_mapping, None);
            assert!(report.live_unmap.core_detaches > 0);
        } else {
            assert!(toplevel_mapping.is_some());
        }
        assert_eq!(runtime_loop.admission_pending_count(), 0);
        assert!(state.surfaces.get(1).is_some());
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator report 直接暴露 loop live unmap drain 事实。
    #[test]
    fn runtime_orchestrator_run_reports_live_toplevel_unmap() {
        assert_runtime_dir();
        let mut orchestrator =
            NestedRuntimeOrchestrator::new(config("orchestrator-live-unmap-report", 2));
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let registration = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            adapter_toplevel_identity_registration_report(display)
                .expect("adapter identity registration proof 必须完成")
        };

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert_eq!(report.pump_iterations, 2);
        assert!(report.is_clean_shutdown());
        assert_eq!(report.live_unmap, report.loop_report.live_unmap);
        assert_eq!(report.live_unmap.drain_invocations, 2);
        assert_eq!(report.live_unmap.live_unmap_observations, 1);
        assert_eq!(report.live_unmap.ledger_unmaps, 1);
        assert_eq!(report.live_unmap.core_detaches, 1);
        assert_eq!(report.live_unmap.surface_mappings_retained, 1);
        assert_eq!(report.live_unmap.toplevel_mappings_removed, 1);
        let runtime_loop = orchestrator
            .runtime_loop
            .as_ref()
            .expect("Stopped orchestrator 保留 runtime loop report owner");
        assert_eq!(
            runtime_loop.admission_surface_mapping(registration.adapter_surface_id),
            Some(1)
        );
        assert_eq!(
            runtime_loop.admission_toplevel_mapping(registration.adapter_toplevel_id),
            None
        );
        assert!(state.surfaces.is_alive(1));
        assert!(state.registry.records().iter().any(|record| !record.alive));
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator run 继承 loop 的 live-admission-aware idle 判断。
    #[test]
    fn runtime_orchestrator_stop_when_idle_drains_live_admission_backlog() {
        assert_runtime_dir();
        let mut config = config("orchestrator-live-idle-backlog", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_registration, second_registration) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_registration = adapter_toplevel_identity_registration_report(display)
                .expect("首次 adapter identity registration proof 必须完成");
            let second_registration = adapter_toplevel_identity_registration_report(display)
                .expect("第二次 adapter identity registration proof 必须完成");

            (first_registration, second_registration)
        };

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert_eq!(report.pump_iterations, 3);
        assert_eq!(report.loop_exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.is_clean_shutdown());
        assert_eq!(report.final_state, NestedRuntimeLifecycleState::Stopped);
        assert!(report.loop_report.is_successful());
        assert_eq!(report.live_admission.owner_invocations, 3);
        assert_eq!(report.live_admission.enqueue_invocations, 2);
        assert_eq!(report.live_admission.admissions_enqueued, 2);
        assert_eq!(report.live_admission.drain_invocations, 3);
        assert_eq!(report.live_admission.admissions_consumed, 2);
        assert_eq!(report.live_admission.pending_admissions_after, 0);
        assert_eq!(report.live_admission, report.loop_report.live_admission);
        let runtime_loop = orchestrator
            .runtime_loop
            .as_ref()
            .expect("Stopped orchestrator 保留 runtime loop report owner");
        assert_eq!(
            runtime_loop.admission_surface_mapping(first_registration.adapter_surface_id),
            Some(1)
        );
        assert_eq!(
            runtime_loop.admission_surface_mapping(second_registration.adapter_surface_id),
            Some(2)
        );
        assert_eq!(runtime_loop.admission_pending_count(), 0);
        assert!(state.surfaces.get(1).is_some());
        assert!(state.surfaces.get(2).is_some());
        assert_eq!(state.surfaces.records().len(), 2);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes runtime-drained commit backlog.
    #[test]
    fn runtime_orchestrator_run_reports_wl_surface_commit_backlog_drain() {
        assert_runtime_dir();
        let mut config = config("orchestrator-surface-commit-backlog", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("首个 controlled commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 controlled commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert_eq!(report.pump_iterations, 3);
        assert_eq!(report.loop_exit_reason, NestedRuntimeLoopExitReason::Idle);
        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(report.surface_commit.drain_invocations, 3);
        assert_eq!(report.surface_commit.commit_observations_drained, 2);
        assert_eq!(report.surface_commit.commit_observation_errors, 0);
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.drained_commit_sequences, vec![1, 2]);
        assert_eq!(report.surface_commit.buffer_attach_observations, 0);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 0);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 0);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 0);
        assert_eq!(report.surface_commit.frame_callback_observations, 0);
        assert_eq!(report.surface_commit.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes drained commit buffer evidence.
    #[test]
    fn runtime_orchestrator_run_reports_wl_surface_commit_buffer_evidence() {
        assert_runtime_dir();
        let mut config = config("orchestrator-surface-commit-buffer-evidence", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit = controlled_wl_surface_null_attach_commit_observation_report(display)
                .expect("首个 null attach commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.buffer_attach_observations, 1);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 1);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 0);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 0);
        assert_eq!(report.surface_commit.frame_callback_observations, 0);
        assert_eq!(report.surface_commit.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes drained commit damage evidence.
    #[test]
    fn runtime_orchestrator_run_reports_wl_surface_commit_damage_evidence() {
        assert_runtime_dir();
        let mut config = config("orchestrator-surface-commit-damage-evidence", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit = controlled_wl_surface_damage_commit_observation_report(display)
                .expect("首个 damage commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.buffer_attach_observations, 0);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 0);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 1);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 1);
        assert_eq!(report.surface_commit.frame_callback_observations, 0);
        assert_eq!(report.surface_commit.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes drained commit frame callback evidence.
    #[test]
    fn runtime_orchestrator_run_reports_wl_surface_commit_frame_callback_evidence() {
        assert_runtime_dir();
        let mut config = config("orchestrator-surface-commit-frame-callback-evidence", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit =
                controlled_wl_surface_frame_callback_commit_observation_report(display)
                    .expect("首个 frame callback commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(report.surface_commit.buffer_attach_observations, 0);
        assert_eq!(report.surface_commit.buffer_presence_observations, 0);
        assert_eq!(report.surface_commit.buffer_removed_observations, 0);
        assert_eq!(report.surface_commit.renderable_buffer_observations, 0);
        assert_eq!(report.surface_commit.damage_observations, 0);
        assert_eq!(report.surface_commit.surface_damage_rects, 0);
        assert_eq!(report.surface_commit.buffer_damage_rects, 0);
        assert_eq!(report.surface_commit.frame_callback_observations, 1);
        assert_eq!(report.surface_commit.frame_callback_count, 1);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(report.live_admission.admissions_consumed, 0);
        assert_eq!(report.live_unmap.core_detaches, 0);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes render-dirty readiness intents.
    #[test]
    fn runtime_orchestrator_run_reports_render_dirty_readiness_intents() {
        assert_runtime_dir();
        let mut config = config("orchestrator-render-dirty-readiness-intent", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(
            report.surface_commit.drained_commit_sequences,
            vec![first_commit.commit_sequence, second_commit.commit_sequence]
        );
        assert_eq!(
            report.surface_commit.render_dirty_readiness_intents.len(),
            2
        );
        let first_intent = &report.surface_commit.render_dirty_readiness_intents[0];
        let second_intent = &report.surface_commit.render_dirty_readiness_intents[1];
        assert_eq!(
            first_intent.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_intent.commit_sequence, first_commit.commit_sequence);
        assert_eq!(second_intent.commit_sequence, second_commit.commit_sequence);
        assert!(first_intent.buffer_attach_observed);
        assert!(!first_intent.buffer_present);
        assert!(first_intent.buffer_removed);
        assert!(!first_intent.renderable_buffer);
        assert!(first_intent.damage_observed);
        assert_eq!(first_intent.buffer_damage_rects, 1);
        assert!(first_intent.frame_callback_observed);
        assert_eq!(first_intent.frame_callback_count, 1);
        assert!(!first_intent.render_submitted);
        assert!(!first_intent.buffer_imported);
        assert!(!first_intent.frame_callback_done_sent);
        assert!(!first_intent.input_support);
        assert!(!second_intent.buffer_attach_observed);
        assert!(!second_intent.damage_observed);
        assert_eq!(second_intent.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes render-dirty intent queue drain.
    #[test]
    fn runtime_orchestrator_run_reports_render_dirty_intent_queue_drain() {
        assert_runtime_dir();
        let mut config = config("orchestrator-render-dirty-intent-queue", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(report.surface_commit.render_dirty_intents_enqueued, 2);
        assert_eq!(report.surface_commit.render_dirty_intents_drained, 2);
        let first_drained = &report.surface_commit.render_dirty_queue_drained_intents[0];
        let second_drained = &report.surface_commit.render_dirty_queue_drained_intents[1];
        assert_eq!(
            first_drained.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_drained.commit_sequence, first_commit.commit_sequence);
        assert_eq!(
            second_drained.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_drained.buffer_attach_observed);
        assert!(first_drained.buffer_removed);
        assert!(first_drained.damage_observed);
        assert_eq!(first_drained.buffer_damage_rects, 1);
        assert!(first_drained.frame_callback_observed);
        assert_eq!(first_drained.frame_callback_count, 1);
        assert!(!first_drained.render_submitted);
        assert!(!first_drained.buffer_imported);
        assert!(!first_drained.texture_created);
        assert!(!first_drained.frame_callback_done_sent);
        assert!(!first_drained.input_support);
        assert!(!second_drained.buffer_attach_observed);
        assert!(!second_drained.damage_observed);
        assert_eq!(second_drained.frame_callback_count, 0);
        assert!(!report.surface_commit.render_dirty_queue_render_submitted);
        assert!(!report.surface_commit.render_dirty_queue_buffer_imported);
        assert!(!report.surface_commit.render_dirty_queue_texture_created);
        assert!(
            !report
                .surface_commit
                .render_dirty_queue_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_dirty_queue_input_support);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes renderer-admission work intents.
    #[test]
    fn runtime_orchestrator_run_reports_renderer_admission_work_intents() {
        assert_runtime_dir();
        let mut config = config("orchestrator-renderer-admission-work-intent", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        assert_eq!(report.surface_commit.renderer_work_intents_created, 2);
        assert_eq!(report.surface_commit.renderer_work_intents.len(), 2);
        let first_work = &report.surface_commit.renderer_work_intents[0];
        let second_work = &report.surface_commit.renderer_work_intents[1];
        assert_eq!(
            first_work.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_work.commit_sequence, first_commit.commit_sequence);
        assert_eq!(second_work.commit_sequence, second_commit.commit_sequence);
        assert!(first_work.buffer_attach_observed);
        assert!(first_work.buffer_removed);
        assert!(first_work.damage_observed);
        assert_eq!(first_work.buffer_damage_rects, 1);
        assert!(first_work.frame_callback_observed);
        assert_eq!(first_work.frame_callback_count, 1);
        assert!(!first_work.render_submitted);
        assert!(!first_work.buffer_imported);
        assert!(!first_work.texture_created);
        assert!(!first_work.damage_submitted);
        assert!(!first_work.frame_callback_done_sent);
        assert!(!first_work.input_support);
        assert!(!first_work.core_mutation_invoked);
        assert!(!second_work.buffer_attach_observed);
        assert!(!second_work.damage_observed);
        assert_eq!(second_work.frame_callback_count, 0);
        assert!(!report.surface_commit.renderer_admission_render_submitted);
        assert!(!report.surface_commit.renderer_admission_buffer_imported);
        assert!(!report.surface_commit.renderer_admission_texture_created);
        assert!(!report.surface_commit.renderer_admission_damage_submitted);
        assert!(
            !report
                .surface_commit
                .renderer_admission_frame_callback_done_sent
        );
        assert!(!report.surface_commit.renderer_admission_input_support);
        assert!(
            !report
                .surface_commit
                .renderer_admission_core_mutation_invoked
        );
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// Linux-only proof：orchestrator final report exposes renderer owner boundary blockers.
    #[test]
    fn runtime_orchestrator_run_reports_renderer_owner_boundary_blocked_readiness() {
        assert_runtime_dir();
        let mut config = config("orchestrator-renderer-owner-boundary", 3);
        config.loop_config.stop_when_idle = true;
        let mut orchestrator = NestedRuntimeOrchestrator::new(config);
        let mut state = State::new();
        let _start_report = orchestrator.start().expect("Created 必须允许 start");
        let (first_commit, second_commit) = {
            let display = orchestrator
                .runtime_loop
                .as_mut()
                .expect("Started 必须持有 runtime loop")
                .display_mut_for_controlled_toplevel_registration();
            assert_start_keeps_production_globals_initialized(display);
            let first_commit =
                controlled_wl_surface_render_dirty_readiness_commit_observation_report(display)
                    .expect("首个 render-dirty readiness commit proof 必须完成");
            let second_commit = controlled_wl_surface_commit_observation_report(display)
                .expect("第二个 plain commit proof 必须完成");

            (first_commit, second_commit)
        };
        let surface_records_before = state.surfaces.records().len();
        let registry_records_before = state.registry.records().len();

        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");

        assert!(report.is_clean_shutdown());
        assert_eq!(report.surface_commit, report.loop_report.surface_commit);
        let consumed_count = report.surface_commit.renderer_owner_work_intents_consumed;
        assert_eq!(consumed_count, 2);
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_consumed_work_intents
                .len(),
            2
        );
        let first_consumed = &report.surface_commit.renderer_owner_consumed_work_intents[0];
        let second_consumed = &report.surface_commit.renderer_owner_consumed_work_intents[1];
        assert_eq!(
            first_consumed.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(first_consumed.commit_sequence, first_commit.commit_sequence);
        let second_sequence = second_consumed.commit_sequence;
        assert_eq!(second_sequence, second_commit.commit_sequence);
        assert!(first_consumed.buffer_attach_observed);
        assert!(first_consumed.buffer_removed);
        assert!(first_consumed.damage_observed);
        assert_eq!(first_consumed.buffer_damage_rects, 1);
        assert!(first_consumed.frame_callback_observed);
        assert_eq!(first_consumed.frame_callback_count, 1);
        assert!(!second_consumed.buffer_attach_observed);
        assert!(!second_consumed.damage_observed);
        assert_eq!(second_consumed.frame_callback_count, 0);
        assert!(report.surface_commit.renderer_owner_missing_renderer_owner);
        assert!(report.surface_commit.renderer_owner_missing_buffer_importer);
        assert!(report.surface_commit.renderer_owner_missing_texture_support);
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_shell_work_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_owner_shell_observed_work_intents
                .len(),
            2
        );
        assert!(report.surface_commit.renderer_owner_shell_available);
        assert!(
            !report
                .surface_commit
                .renderer_owner_shell_missing_renderer_owner
        );
        assert!(
            report
                .surface_commit
                .renderer_owner_shell_missing_buffer_importer
        );
        assert!(
            report
                .surface_commit
                .renderer_owner_shell_missing_texture_support
        );
        let first_shell = &report
            .surface_commit
            .renderer_owner_shell_observed_work_intents[0];
        let second_shell = &report
            .surface_commit
            .renderer_owner_shell_observed_work_intents[1];
        assert_eq!(first_shell.commit_sequence, first_commit.commit_sequence);
        assert_eq!(second_shell.commit_sequence, second_commit.commit_sequence);
        assert!(first_shell.buffer_attach_observed);
        assert!(first_shell.damage_observed);
        assert_eq!(first_shell.frame_callback_count, 1);
        assert!(!second_shell.buffer_attach_observed);
        assert!(!second_shell.damage_observed);
        assert_eq!(second_shell.frame_callback_count, 0);
        assert!(!report.surface_commit.renderer_owner_shell_buffer_imported);
        assert!(!report.surface_commit.renderer_owner_shell_texture_created);
        assert!(!report.surface_commit.renderer_owner_shell_renderer_called);
        assert!(!report.surface_commit.renderer_owner_shell_damage_submitted);
        assert!(
            !report
                .surface_commit
                .renderer_owner_shell_frame_callback_done_sent
        );
        assert!(!report.surface_commit.renderer_owner_shell_input_support);
        assert!(
            !report
                .surface_commit
                .renderer_owner_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_importer_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_importer_shell_work_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_importer_shell_observed_work_intents
                .len(),
            2
        );
        assert!(report.surface_commit.buffer_importer_shell_available);
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_missing_renderer_owner_shell
        );
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_missing_buffer_importer
        );
        assert!(
            report
                .surface_commit
                .buffer_importer_shell_missing_texture_support
        );
        let first_importer = &report
            .surface_commit
            .buffer_importer_shell_observed_work_intents[0];
        let second_importer = &report
            .surface_commit
            .buffer_importer_shell_observed_work_intents[1];
        assert_eq!(first_importer.commit_sequence, first_commit.commit_sequence);
        assert_eq!(
            second_importer.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_importer.buffer_attach_observed);
        assert!(first_importer.damage_observed);
        assert_eq!(first_importer.frame_callback_count, 1);
        assert!(!second_importer.buffer_attach_observed);
        assert!(!second_importer.damage_observed);
        assert_eq!(second_importer.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_importer_shell_buffer_imported);
        assert!(!report.surface_commit.buffer_importer_shell_texture_created);
        assert!(!report.surface_commit.buffer_importer_shell_renderer_called);
        assert!(!report.surface_commit.buffer_importer_shell_damage_submitted);
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_importer_shell_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_importer_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .texture_support_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .texture_support_shell_work_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .texture_support_shell_observed_work_intents
                .len(),
            2
        );
        assert!(report.surface_commit.texture_support_shell_available);
        assert!(
            !report
                .surface_commit
                .texture_support_shell_missing_buffer_importer_shell
        );
        assert!(
            !report
                .surface_commit
                .texture_support_shell_missing_texture_support
        );
        let first_texture = &report
            .surface_commit
            .texture_support_shell_observed_work_intents[0];
        let second_texture = &report
            .surface_commit
            .texture_support_shell_observed_work_intents[1];
        assert_eq!(first_texture.commit_sequence, first_commit.commit_sequence);
        assert_eq!(
            second_texture.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_texture.buffer_attach_observed);
        assert!(first_texture.damage_observed);
        assert_eq!(first_texture.frame_callback_count, 1);
        assert!(!second_texture.buffer_attach_observed);
        assert!(!second_texture.damage_observed);
        assert_eq!(second_texture.frame_callback_count, 0);
        assert!(!report.surface_commit.texture_support_shell_buffer_imported);
        assert!(!report.surface_commit.texture_support_shell_texture_created);
        assert!(!report.surface_commit.texture_support_shell_renderer_called);
        assert!(!report.surface_commit.texture_support_shell_damage_submitted);
        assert!(
            !report
                .surface_commit
                .texture_support_shell_frame_callback_done_sent
        );
        assert!(!report.surface_commit.texture_support_shell_input_support);
        assert!(
            !report
                .surface_commit
                .texture_support_shell_core_mutation_invoked
        );
        assert_eq!(
            report.surface_commit.render_operation_readiness_invocations,
            3
        );
        assert_eq!(report.surface_commit.render_operation_intents_created, 2);
        assert_eq!(report.surface_commit.render_operation_intents.len(), 2);
        let first_render_operation = &report.surface_commit.render_operation_intents[0];
        let second_render_operation = &report.surface_commit.render_operation_intents[1];
        assert_eq!(
            first_render_operation.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_operation.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_operation.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_operation.buffer_attach_observed);
        assert!(first_render_operation.damage_observed);
        assert_eq!(
            first_render_operation.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_operation.frame_callback_count, 1);
        assert!(!second_render_operation.buffer_attach_observed);
        assert!(!second_render_operation.damage_observed);
        assert_eq!(second_render_operation.damage_rect_count, 0);
        assert_eq!(second_render_operation.frame_callback_count, 0);
        assert!(!report.surface_commit.render_operation_buffer_imported);
        assert!(!report.surface_commit.render_operation_texture_created);
        assert!(!report.surface_commit.render_operation_renderer_called);
        assert!(!report.surface_commit.render_operation_damage_submitted);
        assert!(
            !report
                .surface_commit
                .render_operation_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_operation_input_support);
        assert!(!report.surface_commit.render_operation_core_mutation_invoked);
        assert_eq!(
            report
                .surface_commit
                .render_operation_queue_drain_invocations,
            3
        );
        assert_eq!(report.surface_commit.render_operation_intents_enqueued, 2);
        assert_eq!(report.surface_commit.render_operation_intents_drained, 2);
        assert_eq!(
            report
                .surface_commit
                .render_operation_queue_drained_intents
                .len(),
            2
        );
        let first_render_operation_drained =
            &report.surface_commit.render_operation_queue_drained_intents[0];
        let second_render_operation_drained =
            &report.surface_commit.render_operation_queue_drained_intents[1];
        assert_eq!(
            first_render_operation_drained.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_operation_drained.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_operation_drained.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_operation_drained.buffer_attach_observed);
        assert!(first_render_operation_drained.damage_observed);
        assert_eq!(
            first_render_operation_drained.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_operation_drained.frame_callback_count, 1);
        assert!(!second_render_operation_drained.buffer_attach_observed);
        assert!(!second_render_operation_drained.damage_observed);
        assert_eq!(second_render_operation_drained.damage_rect_count, 0);
        assert_eq!(second_render_operation_drained.frame_callback_count, 0);
        assert!(!report.surface_commit.render_operation_queue_buffer_imported);
        assert!(!report.surface_commit.render_operation_queue_texture_created);
        assert!(!report.surface_commit.render_operation_queue_renderer_called);
        assert!(
            !report
                .surface_commit
                .render_operation_queue_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_operation_queue_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_operation_queue_input_support);
        assert!(
            !report
                .surface_commit
                .render_operation_queue_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_boundary_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_intents_consumed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_consumed_intents
                .len(),
            2
        );
        let first_render_execution = &report
            .surface_commit
            .render_execution_owner_consumed_intents[0];
        let second_render_execution = &report
            .surface_commit
            .render_execution_owner_consumed_intents[1];
        assert_eq!(
            first_render_execution.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_execution.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_execution.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_execution.buffer_attach_observed);
        assert!(first_render_execution.damage_observed);
        assert_eq!(
            first_render_execution.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_execution.frame_callback_count, 1);
        assert!(!second_render_execution.buffer_attach_observed);
        assert!(!second_render_execution.damage_observed);
        assert_eq!(second_render_execution.damage_rect_count, 0);
        assert_eq!(second_render_execution.frame_callback_count, 0);
        assert!(report.surface_commit.render_execution_owner_missing_owner);
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_missing_frame_callback_done
        );
        assert!(!report.surface_commit.render_execution_owner_buffer_imported);
        assert!(!report.surface_commit.render_execution_owner_texture_created);
        assert!(!report.surface_commit.render_execution_owner_renderer_called);
        assert!(
            !report
                .surface_commit
                .render_execution_owner_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_execution_owner_input_support);
        assert!(
            !report
                .surface_commit
                .render_execution_owner_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_shell_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_execution_owner_shell_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.render_execution_owner_shell_available);
        let first_render_execution_shell = &report
            .surface_commit
            .render_execution_owner_shell_observed_intents[0];
        let second_render_execution_shell = &report
            .surface_commit
            .render_execution_owner_shell_observed_intents[1];
        assert_eq!(
            first_render_execution_shell.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_execution_shell.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_execution_shell.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_execution_shell.buffer_attach_observed);
        assert!(first_render_execution_shell.damage_observed);
        assert_eq!(
            first_render_execution_shell.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_execution_shell.frame_callback_count, 1);
        assert!(!second_render_execution_shell.buffer_attach_observed);
        assert!(!second_render_execution_shell.damage_observed);
        assert_eq!(second_render_execution_shell.damage_rect_count, 0);
        assert_eq!(second_render_execution_shell.frame_callback_count, 0);
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_execution_owner_shell_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_texture_created
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_input_support
        );
        assert!(
            !report
                .surface_commit
                .render_execution_owner_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_pipeline_skeleton_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_pipeline_skeleton_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_pipeline_skeleton_observed_intents
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_owner_available
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_missing_execution_owner_shell
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_pipeline_skeleton_missing_frame_callback_done
        );
        let first_render_pipeline_skeleton = &report
            .surface_commit
            .render_pipeline_skeleton_observed_intents[0];
        let second_render_pipeline_skeleton = &report
            .surface_commit
            .render_pipeline_skeleton_observed_intents[1];
        assert_eq!(
            first_render_pipeline_skeleton.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_pipeline_skeleton.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_pipeline_skeleton.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_pipeline_skeleton.buffer_attach_observed);
        assert!(first_render_pipeline_skeleton.damage_observed);
        assert_eq!(
            first_render_pipeline_skeleton.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_pipeline_skeleton.frame_callback_count, 1);
        assert!(!second_render_pipeline_skeleton.buffer_attach_observed);
        assert!(!second_render_pipeline_skeleton.damage_observed);
        assert_eq!(second_render_pipeline_skeleton.damage_rect_count, 0);
        assert_eq!(second_render_pipeline_skeleton.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_texture_created
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_frame_callback_done_sent
        );
        assert!(!report.surface_commit.render_pipeline_skeleton_input_support);
        assert!(
            !report
                .surface_commit
                .render_pipeline_skeleton_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .render_backend_capability_report_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .render_backend_capability_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .render_backend_capability_observed_intents
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_owner_available
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_backend_registered
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_backend_kind
                .is_none()
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_missing_pipeline_skeleton
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_backend_registration
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .render_backend_capability_missing_frame_callback_done
        );
        let first_render_backend_capability = &report
            .surface_commit
            .render_backend_capability_observed_intents[0];
        let second_render_backend_capability = &report
            .surface_commit
            .render_backend_capability_observed_intents[1];
        assert_eq!(
            first_render_backend_capability.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_render_backend_capability.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_render_backend_capability.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_render_backend_capability.buffer_attach_observed);
        assert!(first_render_backend_capability.damage_observed);
        assert_eq!(
            first_render_backend_capability.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_render_backend_capability.frame_callback_count, 1);
        assert!(!second_render_backend_capability.buffer_attach_observed);
        assert!(!second_render_backend_capability.damage_observed);
        assert_eq!(second_render_backend_capability.damage_rect_count, 0);
        assert_eq!(second_render_backend_capability.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .render_backend_capability_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_texture_created
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_input_support
        );
        assert!(
            !report
                .surface_commit
                .render_backend_capability_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_registration_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_registration_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_registration_observed_intents
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_owner_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_backend_registered
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_descriptor_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_missing_backend_capability
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_registration_missing_frame_callback_done
        );
        let first_renderer_backend_registration = &report
            .surface_commit
            .renderer_backend_registration_observed_intents[0];
        let second_renderer_backend_registration = &report
            .surface_commit
            .renderer_backend_registration_observed_intents[1];
        assert_eq!(
            first_renderer_backend_registration.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_renderer_backend_registration.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_renderer_backend_registration.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_renderer_backend_registration.buffer_attach_observed);
        assert!(first_renderer_backend_registration.damage_observed);
        assert_eq!(
            first_renderer_backend_registration.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_renderer_backend_registration.frame_callback_count, 1);
        assert!(!second_renderer_backend_registration.buffer_attach_observed);
        assert!(!second_renderer_backend_registration.damage_observed);
        assert_eq!(second_renderer_backend_registration.damage_rect_count, 0);
        assert_eq!(second_renderer_backend_registration.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_registration_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_shell_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_shell_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_shell_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.renderer_backend_owner_shell_available);
        assert!(report.surface_commit.renderer_backend_owner_shell_bound);
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_descriptor_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_missing_descriptor
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_buffer_import
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_shell_missing_frame_callback_done
        );
        let first_renderer_backend_owner_shell = &report
            .surface_commit
            .renderer_backend_owner_shell_observed_intents[0];
        let second_renderer_backend_owner_shell = &report
            .surface_commit
            .renderer_backend_owner_shell_observed_intents[1];
        assert_eq!(
            first_renderer_backend_owner_shell.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_renderer_backend_owner_shell.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_renderer_backend_owner_shell.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_renderer_backend_owner_shell.buffer_attach_observed);
        assert!(first_renderer_backend_owner_shell.damage_observed);
        assert_eq!(
            first_renderer_backend_owner_shell.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_renderer_backend_owner_shell.frame_callback_count, 1);
        assert!(!second_renderer_backend_owner_shell.buffer_attach_observed);
        assert!(!second_renderer_backend_owner_shell.damage_observed);
        assert_eq!(second_renderer_backend_owner_shell.damage_rect_count, 0);
        assert_eq!(second_renderer_backend_owner_shell.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_shell_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_resource_owner_readiness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_resource_owner_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_resource_owner_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.buffer_importer_owner_available);
        assert!(report.surface_commit.buffer_importer_owner_bound);
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_missing_renderer_backend_owner_shell
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_missing_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_resource_owner_missing_frame_callback_done
        );
        let first_buffer_import_resource_owner = &report
            .surface_commit
            .buffer_import_resource_owner_observed_intents[0];
        let second_buffer_import_resource_owner = &report
            .surface_commit
            .buffer_import_resource_owner_observed_intents[1];
        assert_eq!(
            first_buffer_import_resource_owner.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_resource_owner.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_resource_owner.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_resource_owner.buffer_attach_observed);
        assert!(first_buffer_import_resource_owner.damage_observed);
        assert_eq!(
            first_buffer_import_resource_owner.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_buffer_import_resource_owner.frame_callback_count, 1);
        assert!(!second_buffer_import_resource_owner.buffer_attach_observed);
        assert!(!second_buffer_import_resource_owner.damage_observed);
        assert_eq!(second_buffer_import_resource_owner.damage_rect_count, 0);
        assert_eq!(second_buffer_import_resource_owner.frame_callback_count, 0);
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_resource_owner_core_mutation_invoked
        );
        assert_eq!(report.surface_commit.buffer_import_planning_invocations, 3);
        assert_eq!(
            report
                .surface_commit
                .buffer_import_planning_intents_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_planning_observed_intents
                .len(),
            2
        );
        assert!(report.surface_commit.buffer_import_plan_available);
        assert!(report.surface_commit.buffer_import_plan_built);
        assert_eq!(report.surface_commit.buffer_import_candidates_observed, 1);
        assert_eq!(report.surface_commit.buffer_import_required_count, 0);
        assert!(
            report
                .surface_commit
                .buffer_import_planning_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_registered_backend_kind
                .is_some()
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_missing_buffer_importer_owner
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_missing_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_planning_missing_frame_callback_done
        );
        let first_buffer_import_plan = &report
            .surface_commit
            .buffer_import_planning_observed_intents[0];
        let second_buffer_import_plan = &report
            .surface_commit
            .buffer_import_planning_observed_intents[1];
        assert_eq!(
            first_buffer_import_plan.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_plan.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_plan.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_plan.buffer_attach_observed);
        assert!(!first_buffer_import_plan.buffer_present);
        assert!(first_buffer_import_plan.buffer_removed);
        assert!(first_buffer_import_plan.damage_observed);
        assert_eq!(
            first_buffer_import_plan.damage_rect_count,
            first_commit
                .surface_damage_rects
                .saturating_add(first_commit.buffer_damage_rects)
        );
        assert_eq!(first_buffer_import_plan.frame_callback_count, 1);
        assert!(!second_buffer_import_plan.buffer_attach_observed);
        assert!(!second_buffer_import_plan.buffer_present);
        assert!(!second_buffer_import_plan.damage_observed);
        assert_eq!(second_buffer_import_plan.damage_rect_count, 0);
        assert_eq!(second_buffer_import_plan.frame_callback_count, 0);
        assert!(!report.surface_commit.buffer_import_planning_buffer_imported);
        assert!(!report.surface_commit.buffer_import_planning_texture_created);
        assert!(!report.surface_commit.buffer_import_planning_renderer_called);
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_import_planning_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_import_planning_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_boundary_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_descriptors_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_observed_descriptors
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_descriptor_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_descriptor_registered
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_candidates_observed,
            1
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_importer_owner_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_renderer_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_registered_backend_kind
                .is_some()
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_planning_intent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_missing_plan
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_missing_importer_owner_evidence
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_missing_renderer_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_missing_frame_callback_done
        );
        let first_buffer_import_descriptor = &report
            .surface_commit
            .buffer_import_implementation_observed_descriptors[0];
        let second_buffer_import_descriptor = &report
            .surface_commit
            .buffer_import_implementation_observed_descriptors[1];
        assert_eq!(
            first_buffer_import_descriptor.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_descriptor.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_descriptor.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_descriptor.buffer_attach_observed);
        assert!(!first_buffer_import_descriptor.buffer_present);
        assert!(first_buffer_import_descriptor.buffer_removed);
        assert!(first_buffer_import_descriptor.candidate_evidence_observed);
        assert!(!first_buffer_import_descriptor.actual_import_required);
        assert!(first_buffer_import_descriptor.renderer_backend_descriptor_evidence_available);
        assert!(first_buffer_import_descriptor.importer_owner_evidence_available);
        assert!(!second_buffer_import_descriptor.buffer_attach_observed);
        assert!(!second_buffer_import_descriptor.buffer_present);
        assert!(!second_buffer_import_descriptor.buffer_removed);
        assert!(!second_buffer_import_descriptor.candidate_evidence_observed);
        assert!(!second_buffer_import_descriptor.actual_import_required);
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_proof_boundary_invocations,
            3
        );
        assert_eq!(
            report.surface_commit.buffer_import_adapter_proofs_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_observed_proofs
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_proof_boundary_available
        );
        assert!(report.surface_commit.buffer_import_adapter_proof_registered);
        assert_eq!(
            report
                .surface_commit
                .buffer_import_adapter_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_importer_owner_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_renderer_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_registered_backend_kind
                .is_some()
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_implementation_descriptor
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_missing_importer_owner_evidence
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_missing_renderer_descriptor_evidence
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_adapter_missing_frame_callback_done
        );
        let first_buffer_import_adapter_proof =
            &report.surface_commit.buffer_import_adapter_observed_proofs[0];
        let second_buffer_import_adapter_proof =
            &report.surface_commit.buffer_import_adapter_observed_proofs[1];
        assert_eq!(
            first_buffer_import_adapter_proof.adapter_surface_id,
            first_commit.adapter_surface_id
        );
        assert_eq!(
            first_buffer_import_adapter_proof.commit_sequence,
            first_commit.commit_sequence
        );
        assert_eq!(
            second_buffer_import_adapter_proof.commit_sequence,
            second_commit.commit_sequence
        );
        assert!(first_buffer_import_adapter_proof.buffer_attach_observed);
        assert!(!first_buffer_import_adapter_proof.buffer_present);
        assert!(first_buffer_import_adapter_proof.buffer_removed);
        assert!(first_buffer_import_adapter_proof.candidate_evidence_observed);
        assert!(!first_buffer_import_adapter_proof.actual_import_required);
        assert!(first_buffer_import_adapter_proof.renderer_backend_descriptor_evidence_available);
        assert!(first_buffer_import_adapter_proof.importer_owner_evidence_available);
        assert!(first_buffer_import_adapter_proof.implementation_descriptor_registered);
        assert!(!second_buffer_import_adapter_proof.buffer_attach_observed);
        assert!(!second_buffer_import_adapter_proof.buffer_present);
        assert!(!second_buffer_import_adapter_proof.buffer_removed);
        assert!(!second_buffer_import_adapter_proof.candidate_evidence_observed);
        assert!(!second_buffer_import_adapter_proof.actual_import_required);
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_buffer_import_attempted
        );
        assert!(!report.surface_commit.buffer_import_adapter_buffer_imported);
        assert!(!report.surface_commit.buffer_import_adapter_texture_created);
        assert!(!report.surface_commit.buffer_import_adapter_renderer_called);
        assert!(!report.surface_commit.buffer_import_adapter_damage_submitted);
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_import_adapter_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_import_adapter_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_gate_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_adapter_proofs_observed,
            2
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_observed_adapter_proofs
                .len(),
            2
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_gate_available
        );
        assert_eq!(
            report.surface_commit.buffer_import_preconditions_met_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_future_preconditions_met_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_candidates_observed,
            1
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_importer_owner_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_renderer_descriptor_evidence_available
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_registered_backend_kind
                .is_some()
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_adapter_proof
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_registered_adapter_proof
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_candidate
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_actual_import_requirement
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_actual_buffer_import
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_precondition_missing_frame_callback_done
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_observed_adapter_proofs[0],
            *first_buffer_import_adapter_proof
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_precondition_observed_adapter_proofs[1],
            *second_buffer_import_adapter_proof
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_precondition_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_dry_run_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_dry_run_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_guard_available
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_attempted_count,
            0
        );
        assert_eq!(report.surface_commit.buffer_import_execution_noop_count, 3);
        assert_eq!(
            report.surface_commit.buffer_import_execution_blocked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_actual_required_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_execution_preconditions_met_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_import_preconditions
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_adapter_proof
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_execution_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_frame_callback_done_sent
        );
        assert!(!report.surface_commit.buffer_import_execution_input_support);
        assert!(
            !report
                .surface_commit
                .buffer_import_execution_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_shell_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_shell_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_shell_available
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_real_implementation_available
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_admitted_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_blocked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_actual_required_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_implementation_owner_execution_reports_observed,
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_execution_dry_run_blocked
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_implementation_owner_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_implementation_owner_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_records
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_record_available
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_recorded_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_admission_checked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_admitted_count,
            0
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_blocked_count,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .buffer_import_actual_attempt_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_owner_shell_blocked
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_admission
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_texture_creation
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_renderer_call
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_damage_submit
        );
        assert!(
            report
                .surface_commit
                .buffer_import_actual_attempt_missing_frame_callback_done
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_texture_created
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_input_support
        );
        assert!(
            !report
                .surface_commit
                .buffer_import_actual_attempt_core_mutation_invoked
        );
        assert_eq!(
            report.surface_commit.shm_buffer_import_adapter_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .shm_buffer_import_adapter_reports
                .len(),
            3
        );
        assert!(report.surface_commit.shm_buffer_adapter_available);
        assert!(report.surface_commit.shm_buffer_import_route_selected);
        assert!(!report.surface_commit.shm_buffer_type_boundary_observed);
        assert!(!report.surface_commit.smithay_shm_access_type_available);
        assert!(report.surface_commit.shm_buffer_import_execution_blocked);
        assert!(report.surface_commit.shm_buffer_import_evidence_only_report);
        assert!(report.surface_commit.shm_buffer_import_no_texture_report);
        assert!(
            report
                .surface_commit
                .shm_buffer_import_unsupported_or_missing_wl_buffer
        );
        assert_eq!(
            report
                .surface_commit
                .shm_buffer_import_actual_required_count,
            0
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_missing_wl_buffer_type_boundary
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_missing_shm_buffer_access_evidence
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_no_actual_import_required
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_texture_creation_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_renderer_call_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_damage_submit_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_frame_callback_done_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_import_drm_gbm_dmabuf_forbidden
        );
        assert!(
            !report
                .surface_commit
                .shm_buffer_import_buffer_import_attempted
        );
        assert!(!report.surface_commit.shm_buffer_import_buffer_imported);
        assert!(!report.surface_commit.shm_buffer_import_texture_created);
        assert!(!report.surface_commit.shm_buffer_import_renderer_called);
        assert!(!report.surface_commit.shm_buffer_import_damage_submitted);
        assert!(
            !report
                .surface_commit
                .shm_buffer_import_frame_callback_done_sent
        );
        assert!(!report.surface_commit.shm_buffer_import_input_support);
        assert!(
            !report
                .surface_commit
                .shm_buffer_import_core_mutation_invoked
        );
        assert_eq!(report.surface_commit.shm_buffer_metadata_invocations, 3);
        assert_eq!(report.surface_commit.shm_buffer_metadata_reports.len(), 3);
        assert!(!report.surface_commit.shm_buffer_metadata_available);
        assert!(!report.surface_commit.shm_buffer_metadata_observed);
        assert!(report.surface_commit.shm_buffer_metadata_unavailable);
        assert!(!report.surface_commit.shm_buffer_metadata_width_observed);
        assert!(!report.surface_commit.shm_buffer_metadata_height_observed);
        assert!(!report.surface_commit.shm_buffer_metadata_stride_observed);
        assert!(!report.surface_commit.shm_buffer_metadata_format_observed);
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_unavailable_blocker
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_blocker_refinement_applied
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_no_real_wl_buffer_available
        );
        assert!(
            !report
                .surface_commit
                .shm_buffer_metadata_wl_buffer_available_but_not_shm
        );
        assert!(
            !report
                .surface_commit
                .shm_buffer_metadata_shm_like_candidate_missing_safe_accessor
        );
        assert!(
            !report
                .surface_commit
                .shm_buffer_metadata_insufficient_for_texture_precondition
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_missing_lifetime_cleanup_policy
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_evidence_only_not_import_execution
        );
        assert_eq!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_harness_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_paths_covered,
            21
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_all_paths_covered
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_no_real_wl_buffer_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_non_shm_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_metadata_unavailable_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_partially_available_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_insufficient_for_texture_precondition_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_missing_lifetime_cleanup_policy_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_validation_evidence_without_import_execution_path
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_texture_creation_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_renderer_call_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_damage_submit_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_frame_callback_done_forbidden
        );
        assert!(
            report
                .surface_commit
                .shm_buffer_metadata_drm_gbm_dmabuf_forbidden
        );
        assert!(
            !report
                .surface_commit
                .shm_buffer_metadata_buffer_import_attempted
        );
        assert!(!report.surface_commit.shm_buffer_metadata_buffer_imported);
        assert!(!report.surface_commit.shm_buffer_metadata_texture_created);
        assert!(!report.surface_commit.shm_buffer_metadata_renderer_called);
        assert!(!report.surface_commit.shm_buffer_metadata_damage_submitted);
        assert!(
            !report
                .surface_commit
                .shm_buffer_metadata_frame_callback_done_sent
        );
        assert!(!report.surface_commit.shm_buffer_metadata_input_support);
        assert!(
            !report
                .surface_commit
                .shm_buffer_metadata_core_mutation_invoked
        );
        assert_eq!(
            report.surface_commit.texture_precondition_audit_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .texture_precondition_audit_reports
                .len(),
            3
        );
        assert!(!report.surface_commit.texture_precondition_allowed);
        assert!(report.surface_commit.texture_precondition_blocked);
        assert!(
            report
                .surface_commit
                .texture_precondition_metadata_validation_passed
        );
        assert!(
            !report
                .surface_commit
                .texture_precondition_metadata_sufficient_for_texture_precondition
        );
        assert!(
            report
                .surface_commit
                .texture_precondition_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .texture_precondition_missing_texture_import_route
        );
        assert!(
            report
                .surface_commit
                .texture_precondition_missing_frame_callback_completion_policy
        );
        assert!(
            !report
                .surface_commit
                .texture_precondition_buffer_import_attempted
        );
        assert!(!report.surface_commit.texture_precondition_buffer_imported);
        assert!(!report.surface_commit.texture_precondition_texture_created);
        assert!(!report.surface_commit.texture_precondition_renderer_called);
        assert!(!report.surface_commit.texture_precondition_damage_submitted);
        assert!(
            !report
                .surface_commit
                .texture_precondition_frame_callback_done_sent
        );
        assert!(!report.surface_commit.texture_precondition_input_support);
        assert!(
            !report
                .surface_commit
                .texture_precondition_core_mutation_invoked
        );
        assert_eq!(report.surface_commit.texture_creation_noop_invocations, 3);
        assert_eq!(report.surface_commit.texture_creation_noop_reports.len(), 3);
        assert!(report.surface_commit.texture_creation_noop_available);
        assert!(report.surface_commit.texture_creation_blocked);
        assert!(!report.surface_commit.texture_creation_attempted);
        assert!(
            !report
                .surface_commit
                .texture_creation_texture_precondition_allowed
        );
        assert!(
            !report
                .surface_commit
                .texture_creation_metadata_sufficient_for_texture
        );
        assert!(
            !report
                .surface_commit
                .texture_creation_texture_owner_boundary_available
        );
        assert!(
            !report
                .surface_commit
                .texture_creation_renderer_backend_instance_available
        );
        assert!(
            !report
                .surface_commit
                .texture_creation_texture_import_route_available
        );
        assert!(
            !report
                .surface_commit
                .texture_creation_frame_callback_completion_policy_available
        );
        assert!(
            report
                .surface_commit
                .texture_creation_texture_precondition_not_allowed
        );
        assert!(
            report
                .surface_commit
                .texture_creation_metadata_insufficient_for_texture
        );
        assert!(
            report
                .surface_commit
                .texture_creation_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .texture_creation_missing_texture_import_route
        );
        assert!(
            report
                .surface_commit
                .texture_creation_missing_frame_callback_completion_policy
        );
        assert!(
            report
                .surface_commit
                .texture_creation_missing_texture_owner_boundary
        );
        assert!(
            report
                .surface_commit
                .texture_creation_runtime_evidence_without_texture_creation
        );
        assert!(report.surface_commit.texture_creation_explicitly_disabled);
        assert!(
            report
                .surface_commit
                .texture_creation_renderer_call_explicitly_disabled
        );
        assert!(
            !report
                .surface_commit
                .texture_creation_buffer_import_attempted
        );
        assert!(!report.surface_commit.texture_creation_buffer_imported);
        assert!(!report.surface_commit.texture_creation_texture_created);
        assert!(!report.surface_commit.texture_creation_renderer_called);
        assert!(!report.surface_commit.texture_creation_damage_submitted);
        assert!(
            !report
                .surface_commit
                .texture_creation_frame_callback_done_sent
        );
        assert!(!report.surface_commit.texture_creation_input_support);
        assert!(!report.surface_commit.texture_creation_core_mutation_invoked);
        assert_eq!(report.surface_commit.texture_owner_boundary_invocations, 3);
        assert_eq!(
            report.surface_commit.texture_owner_boundary_reports.len(),
            3
        );
        assert!(report.surface_commit.texture_owner_boundary_available);
        assert!(report.surface_commit.texture_owner_boundary_blocked);
        assert!(
            report
                .surface_commit
                .texture_owner_texture_creation_request_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_future_texture_handle_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_future_texture_lifetime_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_future_texture_cleanup_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_future_texture_release_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_future_texture_invalidation_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_renderer_backend_instance_available
        );
        assert!(
            !report
                .surface_commit
                .texture_owner_texture_import_route_available
        );
        assert!(
            report
                .surface_commit
                .texture_owner_texture_creation_noop_only
        );
        assert!(
            report
                .surface_commit
                .texture_owner_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .texture_owner_missing_texture_import_route
        );
        assert!(
            report
                .surface_commit
                .texture_owner_missing_future_texture_cleanup_policy
        );
        assert!(
            report
                .surface_commit
                .texture_owner_boundary_without_texture_creation
        );
        assert!(!report.surface_commit.texture_owner_buffer_import_attempted);
        assert!(!report.surface_commit.texture_owner_buffer_imported);
        assert!(!report.surface_commit.texture_owner_texture_created);
        assert!(!report.surface_commit.texture_owner_renderer_called);
        assert!(!report.surface_commit.texture_owner_damage_submitted);
        assert!(!report.surface_commit.texture_owner_frame_callback_done_sent);
        assert!(!report.surface_commit.texture_owner_input_support);
        assert!(!report.surface_commit.texture_owner_core_mutation_invoked);
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_instance_audit_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_instance_audit_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_instance_audit_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_instance_audit_blocked
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_instance_texture_owner_boundary_still_blocked
        );
        assert!(!report.surface_commit.renderer_backend_instance_available);
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_lifecycle_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_cleanup_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_availability_owner_defined
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_instance_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_instance_missing_cleanup_policy
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_instance_without_texture_creation
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_instance_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .texture_import_route_decision_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .texture_import_route_decision_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_decision_available
        );
        assert!(report.surface_commit.texture_import_route_decision_blocked);
        assert!(
            report
                .surface_commit
                .texture_import_route_renderer_backend_instance_audit_report_observed
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_renderer_backend_instance_audit_still_blocked
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_renderer_backend_instance_available
        );
        assert!(!report.surface_commit.texture_import_route_available);
        assert!(report.surface_commit.texture_import_route_owner_defined);
        assert!(
            !report
                .surface_commit
                .texture_import_route_import_buffer_call_allowed
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_future_texture_handle_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_texture_cleanup_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_texture_release_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_damage_mapping_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_frame_callback_completion_policy_defined
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_missing_import_buffer_call_policy
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_missing_future_texture_handle_ownership_policy
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_missing_damage_mapping_policy
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_missing_frame_callback_completion_policy
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_import_buffer_explicitly_disabled
        );
        assert!(
            report
                .surface_commit
                .texture_import_route_decision_without_import
        );
        assert!(
            !report
                .surface_commit
                .texture_import_route_buffer_import_attempted
        );
        assert!(!report.surface_commit.texture_import_route_buffer_imported);
        assert!(!report.surface_commit.texture_import_route_texture_created);
        assert!(!report.surface_commit.texture_import_route_renderer_called);
        assert!(!report.surface_commit.texture_import_route_damage_submitted);
        assert!(
            !report
                .surface_commit
                .texture_import_route_frame_callback_done_sent
        );
        assert!(!report.surface_commit.texture_import_route_input_support);
        assert!(
            !report
                .surface_commit
                .texture_import_route_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .damage_to_texture_mapping_audit_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .damage_to_texture_mapping_audit_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .damage_to_texture_mapping_audit_available
        );
        assert!(
            report
                .surface_commit
                .damage_to_texture_mapping_audit_blocked
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_texture_import_route_decision_report_observed
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_texture_import_route_decision_still_blocked
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_texture_import_route_available
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_future_texture_handle_owner_defined
        );
        assert!(report.surface_commit.damage_mapping_owner_defined);
        assert!(
            !report
                .surface_commit
                .damage_mapping_texture_region_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_surface_damage_mapping_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_buffer_damage_mapping_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_coordinate_space_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_renderer_damage_submission_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .damage_mapping_frame_callback_completion_policy_defined
        );
        assert!(!report.surface_commit.damage_submission_allowed);
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_texture_import_route
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_future_texture_handle_ownership_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_texture_region_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_surface_damage_mapping_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_buffer_damage_mapping_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_coordinate_space_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_renderer_damage_submission_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_missing_frame_callback_completion_policy
        );
        assert!(
            report
                .surface_commit
                .damage_mapping_damage_submission_explicitly_disabled
        );
        assert!(report.surface_commit.damage_mapping_without_texture);
        assert!(!report.surface_commit.damage_mapping_buffer_import_attempted);
        assert!(!report.surface_commit.damage_mapping_buffer_imported);
        assert!(!report.surface_commit.damage_mapping_texture_created);
        assert!(!report.surface_commit.damage_mapping_renderer_called);
        assert!(!report.surface_commit.damage_mapping_damage_submitted);
        assert!(
            !report
                .surface_commit
                .damage_mapping_frame_callback_done_sent
        );
        assert!(!report.surface_commit.damage_mapping_input_support);
        assert!(!report.surface_commit.damage_mapping_core_mutation_invoked);
        assert_eq!(
            report
                .surface_commit
                .frame_callback_completion_policy_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .frame_callback_completion_policy_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .frame_callback_completion_policy_available
        );
        assert!(
            report
                .surface_commit
                .frame_callback_completion_policy_blocked
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_damage_mapping_audit_observed
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_damage_mapping_audit_still_blocked
        );
        assert!(
            report
                .surface_commit
                .frame_callback_completion_owner_defined
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_render_success_required_before_done
        );
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_real_texture_available
        );
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_renderer_backend_instance_available
        );
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_damage_submission_available
        );
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_render_success_evidence_available
        );
        assert!(!report.surface_commit.frame_callback_done_allowed);
        assert!(
            report
                .surface_commit
                .frame_callback_policy_missing_real_texture
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_missing_damage_submission
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_missing_render_success_evidence
        );
        assert!(
            report
                .surface_commit
                .frame_callback_policy_done_explicitly_disabled
        );
        assert!(report.surface_commit.frame_callback_policy_without_render);
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_buffer_import_attempted
        );
        assert!(!report.surface_commit.frame_callback_policy_buffer_imported);
        assert!(!report.surface_commit.frame_callback_policy_texture_created);
        assert!(!report.surface_commit.frame_callback_policy_renderer_called);
        assert!(!report.surface_commit.frame_callback_policy_damage_submitted);
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_frame_callback_done_sent
        );
        assert!(!report.surface_commit.frame_callback_policy_input_support);
        assert!(
            !report
                .surface_commit
                .frame_callback_policy_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .real_texture_creation_readiness_decision_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .real_texture_creation_readiness_decision_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_readiness_decision_available
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_readiness_blocked
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_minimum_renderability_checklist_defined
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_frame_callback_policy_observed
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_frame_callback_policy_still_blocked
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_renderer_backend_instance_available
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_texture_import_route_available
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_future_texture_handle_owner_defined
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_texture_cleanup_policy_defined
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_damage_submission_available
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_render_success_evidence_available
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_frame_callback_done_allowed
        );
        assert!(!report.surface_commit.real_texture_creation_ready);
        assert!(!report.surface_commit.real_texture_creation_allowed);
        assert!(
            report
                .surface_commit
                .real_texture_creation_missing_renderer_backend_instance
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_missing_texture_import_route
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_missing_future_texture_handle_ownership_policy
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_missing_texture_cleanup_policy
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_missing_damage_submission
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_missing_render_success_evidence
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_frame_callback_done_disabled
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_explicitly_disabled
        );
        assert!(
            report
                .surface_commit
                .real_texture_creation_readiness_without_texture
        );
        assert!(
            !report
                .surface_commit
                .real_texture_creation_buffer_import_attempted
        );
        assert!(!report.surface_commit.real_texture_creation_buffer_imported);
        assert!(!report.surface_commit.real_texture_creation_texture_created);
        assert!(!report.surface_commit.real_texture_creation_renderer_called);
        assert!(!report.surface_commit.real_texture_creation_damage_submitted);
        assert!(
            !report
                .surface_commit
                .real_texture_creation_frame_callback_done_sent
        );
        assert!(!report.surface_commit.real_texture_creation_input_support);
        assert!(
            !report
                .surface_commit
                .real_texture_creation_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_blocked
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_readiness_decision_observed
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_readiness_still_blocked
        );
        assert!(report.surface_commit.renderer_backend_owner_defined);
        assert!(
            report
                .surface_commit
                .renderer_backend_lifecycle_owner_defined
        );
        assert!(report.surface_commit.renderer_backend_cleanup_owner_defined);
        assert!(report.surface_commit.renderer_backend_error_owner_defined);
        assert!(
            report
                .surface_commit
                .renderer_backend_availability_owner_defined
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_minimal_renderer_path_selected
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_instance_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_type_selected
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_runtime_storage_available
        );
        assert!(!report.surface_commit.renderer_backend_cleanup_implemented);
        assert!(
            !report
                .surface_commit
                .renderer_backend_render_target_binding_available
        );
        assert!(!report.surface_commit.renderer_backend_creation_allowed);
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_missing_instance
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_missing_concrete_type
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_missing_construction_route
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_missing_runtime_storage
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_missing_cleanup_implementation
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_missing_render_target_binding
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_creation_explicitly_disabled
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_owner_boundary_without_instance
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_owner_boundary_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_concrete_route_decision_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_concrete_route_decision_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_decision_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_decision_blocked
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_owner_boundary_observed
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_owner_boundary_still_blocked
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_type_candidate_defined
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_type_compiled
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_construction_route_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_runtime_storage_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_cleanup_policy_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_render_target_binding_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_construction_allowed
        );
        assert!(!report.surface_commit.renderer_backend_instance_created);
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_missing_compile_proof
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_missing_construction_route
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_missing_runtime_storage
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_missing_cleanup_policy
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_missing_render_target_binding
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_construction_explicitly_disabled
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_concrete_route_without_backend_instance
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_concrete_route_core_mutation_invoked
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_invocations,
            3
        );
        assert_eq!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_reports
                .len(),
            3
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_blocked
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_decision_observed
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_decision_still_blocked
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_concrete_type_compiled
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_construction_route_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_runtime_storage_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_cleanup_policy_available
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_render_target_binding_available
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_construction_allowed
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_instance_created
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_decision_still_blocked_blocker
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_dummy_construction_failed
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_render_target_binding_still_missing
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_texture_import_still_missing
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_render_invocation_deferred
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_texture_creation_deferred
        );
        assert!(
            report
                .surface_commit
                .renderer_backend_construction_route_proof_frame_callback_deferred
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_buffer_import_attempted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_buffer_imported
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_texture_created
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_renderer_called
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_damage_submitted
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_frame_callback_done_sent
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_input_support
        );
        assert!(
            !report
                .surface_commit
                .renderer_backend_construction_route_proof_core_mutation_invoked
        );
        assert!(!report.surface_commit.renderer_owner_buffer_imported);
        assert!(!report.surface_commit.renderer_owner_texture_created);
        assert!(!report.surface_commit.renderer_owner_renderer_called);
        assert!(!report.surface_commit.renderer_owner_damage_submitted);
        assert!(
            !report
                .surface_commit
                .renderer_owner_frame_callback_done_sent
        );
        assert!(!report.surface_commit.renderer_owner_input_support);
        assert!(!report.surface_commit.renderer_owner_core_mutation_invoked);
        assert!(!report.surface_commit.buffer_attached);
        assert!(!report.surface_commit.damage_submitted);
        assert!(!report.surface_commit.frame_callback_requested);
        assert!(!report.surface_commit.render_invoked);
        assert!(!report.surface_commit.input_invoked);
        assert!(!report.surface_commit.core_mutation_invoked);
        assert_eq!(state.surfaces.records().len(), surface_records_before);
        assert_eq!(state.registry.records().len(), registry_records_before);
        assert!(state.validate().is_clean());
    }

    /// loop 返回 pump error 时，orchestrator 必须保留原始结构并进入 Failed。
    #[test]
    fn runtime_orchestrator_preserves_structured_pump_errors() {
        assert_runtime_dir();
        let mut orchestrator = NestedRuntimeOrchestrator::new(config("orchestrator-error", 0));
        let mut state = State::new();
        orchestrator.start().expect("Created 必须允许 start");
        orchestrator.state = NestedRuntimeLifecycleState::Running;
        let mut loop_report = orchestrator
            .runtime_loop
            .as_mut()
            .expect("Started 必须持有 runtime loop")
            .run_for_iterations(&mut state, config("unused", 0).loop_config);
        let pump_error = NestedRuntimePumpError {
            kind: NestedRuntimePumpErrorKind::DisplayDispatch,
            message: "controlled orchestrator pump failure".to_owned(),
        };
        let loop_error = NestedRuntimeLoopError {
            iteration: 1,
            pump_errors: vec![pump_error],
        };
        loop_report.exit_reason = NestedRuntimeLoopExitReason::Error;
        loop_report.errors = vec![loop_error.clone()];

        let report = orchestrator.finish_run(loop_report);

        assert_eq!(report.loop_exit_reason, NestedRuntimeLoopExitReason::Error);
        assert_eq!(report.errors, vec![loop_error]);
        assert_eq!(report.final_state, NestedRuntimeLifecycleState::Failed);
        assert!(!report.shutdown_completed);
        assert!(!report.is_clean_shutdown());
        assert!(report.validation_is_clean);
    }

    /// 真实 lifecycle proof：external stop+wakeup 中断 run，并生成 clean shutdown report。
    #[test]
    fn runtime_orchestrator_stop_wakeup_exits_run() {
        assert_runtime_dir();
        let mut orchestration_config = config("orchestrator-wakeup", 1);
        orchestration_config.loop_config.pump_timeout = Duration::from_secs(5);
        let mut orchestrator = NestedRuntimeOrchestrator::new(orchestration_config);
        let mut state = State::new();
        orchestrator.start().expect("Created 必须允许 start");
        let stop_handle = orchestrator
            .stop_handle()
            .expect("Started 必须暴露 stop handle");

        let stopper = thread::spawn(move || {
            let wait_deadline = std::time::Instant::now() + Duration::from_secs(1);
            while !stop_handle.is_waiting() {
                assert!(
                    std::time::Instant::now() < wait_deadline,
                    "orchestrated loop 必须在有界时间内进入 wait"
                );
                thread::sleep(Duration::from_millis(1));
            }
            stop_handle.request_stop_and_wakeup();
        });
        let report = orchestrator.run(&mut state).expect("Started 必须允许 run");
        stopper.join().expect("external stopper 不得 panic");

        assert_eq!(
            report.loop_exit_reason,
            NestedRuntimeLoopExitReason::Interrupted
        );
        assert_eq!(report.pump_iterations, 1);
        assert!(report.stop_requested);
        assert!(report.wakeup_requested);
        assert!(report.shutdown_completed);
        assert_eq!(report.final_state, NestedRuntimeLifecycleState::Stopped);
        assert!(report.validation_is_clean);
        assert!(report.errors.is_empty());
        assert!(report.is_clean_shutdown());
        assert_eq!(orchestrator.state(), NestedRuntimeLifecycleState::Stopped);
        assert!(report.readiness.runtime_orchestrator_available);
        assert!(report.readiness.start_run_stop_available);
        assert!(report.readiness.external_stop_supported);
        assert!(report.readiness.clean_shutdown_supported);
        assert!(!report.readiness.long_running_loop_available);
    }

    /// Started 状态的直接 stop 必须通过既有 handle 请求 wakeup 并完成 clean finalize。
    #[test]
    fn runtime_orchestrator_direct_stop_requests_wakeup() {
        assert_runtime_dir();
        let mut orchestrator =
            NestedRuntimeOrchestrator::new(config("orchestrator-direct-stop", 1));
        let state = State::new();
        orchestrator.start().expect("Created 必须允许 start");

        let report = orchestrator.stop(&state);

        assert_eq!(report.previous_state, NestedRuntimeLifecycleState::Started);
        assert_eq!(report.state, NestedRuntimeLifecycleState::Stopped);
        assert!(report.stop_requested);
        assert!(report.wakeup_requested);
        assert!(report.shutdown_completed);
        assert!(report.validation_is_clean);
    }
}
