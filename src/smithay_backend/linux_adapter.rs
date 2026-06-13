#![cfg(all(feature = "smithay-linux", target_os = "linux"))]

//! Linux Smithay adapter 的受控结构骨架。
//!
//! 本模块只定义资源所有权、能力边界和生命周期转换。它不启动调度循环、不接受
//! 客户端，也不注册协议对象。底层 Display 与 listening socket 继续由
//! `SmithayBootstrapProbe` 封装，本模块不暴露其内部系统类型。

use std::{error::Error, fmt};

use crate::smithay_backend::bootstrap::SmithayBootstrapProbe;

/// Smithay Linux adapter skeleton 的生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterLifecycle {
    /// 资源已准备，但尚未启动任何真实 compositor 行为。
    Prepared,

    /// 已请求关闭，等待完成受控收尾。
    ShutdownRequested,

    /// skeleton 生命周期已经结束。
    Stopped,
}

/// Smithay Linux adapter skeleton 支持的 event pump 操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterPumpOperation {
    /// 启动纯状态 event pump 边界。
    StartPump,

    /// 执行一次不分发协议事件的 skeleton tick。
    PumpOnce,

    /// 停止 event pump 边界。
    StopPump,
}

/// Smithay Linux adapter skeleton 的 event pump 状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterPumpState {
    /// event pump 尚未启动。
    NotStarted,

    /// event pump 边界已准备接收 skeleton tick。
    Ready,

    /// adapter 已请求关闭，event pump 等待停止。
    StopRequested,

    /// event pump 已停止。
    Stopped,
}

/// 单次 skeleton pump 的保守结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterPumpResult {
    /// 本次操作完成后的 pump 状态。
    pub state: SmithayLinuxAdapterPumpState,

    /// 从一开始累计的 skeleton tick 序号。
    pub tick_index: u64,

    /// 本次及此前处理的客户端数量；skeleton 阶段恒为零。
    pub processed_clients: u64,

    /// 本次及此前处理的协议事件数量；skeleton 阶段恒为零。
    pub processed_protocol_events: u64,

    /// 本次及此前注册的协议 global 数量；skeleton 阶段恒为零。
    pub registered_globals: u64,

    /// 当前结果是否严格来自 skeleton 实现。
    pub is_skeleton_only: bool,
}

/// Smithay Linux adapter skeleton 的累计 pump 统计。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterPumpStats {
    /// 已执行的 skeleton tick 总数。
    pub total_ticks: u64,

    /// 已处理的客户端总数；skeleton 阶段恒为零。
    pub processed_clients: u64,

    /// 已处理的协议事件总数；skeleton 阶段恒为零。
    pub processed_protocol_events: u64,

    /// 已注册的协议 global 总数；skeleton 阶段恒为零。
    pub registered_globals: u64,
}

impl SmithayLinuxAdapterPumpStats {
    const fn empty() -> Self {
        Self {
            total_ticks: 0,
            processed_clients: 0,
            processed_protocol_events: 0,
            registered_globals: 0,
        }
    }
}

/// Smithay Linux adapter 未来计划提供的协议 global 类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterGlobalKind {
    /// Wayland compositor global 的纯数据计划。
    Compositor,

    /// Wayland shared-memory global 的纯数据计划。
    Shm,

    /// XDG shell base global 的纯数据计划。
    XdgWmBase,
}

/// Smithay Linux adapter global 计划的注册状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterGlobalRegistrationState {
    /// 只存在计划，尚未执行任何真实注册。
    PlannedOnly,

    /// registration skeleton ledger 已记录该 global。
    RegistrationSkeleton,

    /// 为未来阶段保留的已注册状态；Phase 48E 不会产生该状态。
    Registered,
}

/// 单项 Smithay Linux adapter protocol global 计划。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterGlobalPlan {
    /// global 的稳定类别。
    pub kind: SmithayLinuxAdapterGlobalKind,

    /// global 的协议名称；仅为纯字符串计划。
    pub name: &'static str,

    /// 计划提供的固定协议版本。
    pub version: u32,

    /// 当前 ledger 状态；初始为计划，Phase 48E 可进入 registration skeleton。
    pub state: SmithayLinuxAdapterGlobalRegistrationState,

    /// 当前计划是否仍然只属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter protocol global 计划报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterGlobalPlanReport {
    /// 按稳定顺序排列的 global 计划。
    pub planned: Vec<SmithayLinuxAdapterGlobalPlan>,

    /// global 计划总数。
    pub planned_count: usize,

    /// 真实注册 global 数量；skeleton 阶段恒为零。
    pub registered_count: usize,

    /// 当前报告是否仍然只描述 skeleton 计划。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter global registration skeleton 操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterGlobalRegistrationOperation {
    /// 把所有计划 global 记录到 registration skeleton ledger。
    RegisterPlannedGlobalsSkeleton,
}

/// Smithay Linux adapter global registration skeleton 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterGlobalRegistrationReport {
    /// 是否已经尝试建立 registration skeleton。
    pub attempted: bool,

    /// 进入 registration skeleton 状态的 global 数量。
    pub skeleton_registered_count: usize,

    /// 真实注册的 global 数量；Phase 48E 恒为零。
    pub real_registered_count: usize,

    /// global 计划总数。
    pub planned_count: usize,

    /// 按稳定顺序排列的当前 global ledger。
    pub globals: Vec<SmithayLinuxAdapterGlobalPlan>,

    /// 当前报告是否仍然只描述 skeleton。
    pub skeleton_only: bool,
}

const PROTOCOL_GLOBAL_PLAN: [SmithayLinuxAdapterGlobalPlan; 3] = [
    SmithayLinuxAdapterGlobalPlan {
        kind: SmithayLinuxAdapterGlobalKind::Compositor,
        name: "wl_compositor",
        version: 6,
        state: SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly,
        skeleton_only: true,
    },
    SmithayLinuxAdapterGlobalPlan {
        kind: SmithayLinuxAdapterGlobalKind::Shm,
        name: "wl_shm",
        version: 1,
        state: SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly,
        skeleton_only: true,
    },
    SmithayLinuxAdapterGlobalPlan {
        kind: SmithayLinuxAdapterGlobalKind::XdgWmBase,
        name: "xdg_wm_base",
        version: 6,
        state: SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly,
        skeleton_only: true,
    },
];

/// Smithay Linux adapter skeleton 的结构化诊断类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterDiagnostic {
    /// 当前 adapter 仍然只提供结构骨架。
    SkeletonOnly,

    /// adapter 持有已封装的 Wayland Display 资源。
    HoldsWaylandDisplay,

    /// adapter 持有已封装的 listening socket。
    HoldsListeningSocket,

    /// adapter 提供 event pump 的显式状态边界。
    EventPumpBoundaryPresent,

    /// adapter 未运行真实事件循环。
    EventLoopNotRunning,

    /// adapter 未接受客户端连接。
    ClientsNotAccepted,

    /// adapter 未分发协议事件。
    ProtocolEventsNotDispatched,

    /// adapter 提供 protocol global 计划边界。
    ProtocolGlobalPlanPresent,

    /// 所有 protocol global 当前都只存在于计划中。
    ProtocolGlobalsPlannedOnly,

    /// adapter 提供 protocol global registration skeleton 边界。
    ProtocolGlobalRegistrationBoundaryPresent,

    /// registration 边界只记录 skeleton ledger。
    ProtocolGlobalRegistrationSkeletonOnly,

    /// registration skeleton 已经执行过一次。
    ProtocolGlobalsRegistrationAttempted,

    /// 真实 protocol global 注册在当前阶段不受支持。
    ProtocolGlobalsRealRegistrationUnsupported,

    /// adapter 未注册协议 global。
    ProtocolGlobalsNotRegistered,

    /// adapter 不支持真实 Wayland surface。
    RealSurfacesUnsupported,

    /// adapter 不支持 GPU 渲染。
    GpuRenderingUnsupported,

    /// adapter 已收到关闭请求。
    ShutdownRequested,

    /// adapter 生命周期已经停止。
    AdapterStopped,
}

/// Smithay Linux adapter skeleton 当前具备的保守能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterCapabilities {
    /// 是否持有 Wayland Display 资源。
    pub holds_wayland_display: bool,

    /// 是否持有 Wayland listening socket。
    pub holds_listening_socket: bool,

    /// 是否提供显式 adapter 生命周期边界。
    pub has_adapter_lifecycle_boundary: bool,

    /// 是否提供显式 event pump 边界。
    pub has_event_pump_boundary: bool,

    /// 是否支持执行一次 skeleton tick；不代表真实事件分发。
    pub pumps_once: bool,

    /// 是否运行调度循环。
    pub runs_event_loop: bool,

    /// 是否接受真实客户端连接。
    pub accepts_clients: bool,

    /// 是否提供 protocol global 的纯数据计划边界。
    pub has_protocol_global_plan_boundary: bool,

    /// 是否提供 protocol global registration skeleton 边界。
    pub has_protocol_global_registration_boundary: bool,

    /// 是否注册协议对象。
    pub registers_protocol_globals: bool,

    /// 是否分发真实协议事件。
    pub dispatches_protocol_events: bool,

    /// 是否接入真实 Wayland surface。
    pub supports_real_wayland_surfaces: bool,

    /// 是否接入 GPU 渲染。
    pub supports_gpu_rendering: bool,

    /// 当前实现是否仍然仅为结构骨架。
    pub is_skeleton_only: bool,
}

impl SmithayLinuxAdapterCapabilities {
    /// 返回 Phase 48A adapter skeleton 的固定保守能力集合。
    pub const fn skeleton_only() -> Self {
        Self {
            holds_wayland_display: true,
            holds_listening_socket: true,
            has_adapter_lifecycle_boundary: true,
            has_event_pump_boundary: true,
            pumps_once: true,
            runs_event_loop: false,
            accepts_clients: false,
            has_protocol_global_plan_boundary: true,
            has_protocol_global_registration_boundary: true,
            registers_protocol_globals: false,
            dispatches_protocol_events: false,
            supports_real_wayland_surfaces: false,
            supports_gpu_rendering: false,
            is_skeleton_only: true,
        }
    }
}

/// Smithay Linux adapter skeleton 的稳定只读状态快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterSnapshot {
    /// adapter 当前生命周期。
    pub lifecycle: SmithayLinuxAdapterLifecycle,

    /// event pump 当前状态。
    pub pump_state: SmithayLinuxAdapterPumpState,

    /// adapter 当前保守能力集合。
    pub capabilities: SmithayLinuxAdapterCapabilities,

    /// adapter 当前 protocol global 计划报告。
    pub global_plan: SmithayLinuxAdapterGlobalPlanReport,

    /// adapter 当前 registration skeleton 报告。
    pub global_registration_report: Option<SmithayLinuxAdapterGlobalRegistrationReport>,

    /// event pump 当前累计统计。
    pub pump_stats: SmithayLinuxAdapterPumpStats,

    /// 最近一次成功的 skeleton pump 结果。
    pub last_pump_result: Option<SmithayLinuxAdapterPumpResult>,

    /// 按稳定顺序生成的结构化诊断。
    pub diagnostics: Vec<SmithayLinuxAdapterDiagnostic>,

    /// bootstrap 已绑定的 listening socket 名称。
    pub socket_name: String,

    /// 当前 adapter 是否仍然只提供结构骨架。
    pub is_skeleton_only: bool,
}

/// Smithay Linux adapter skeleton 支持的生命周期操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterOperation {
    /// 请求进入关闭流程。
    RequestShutdown,

    /// 完成已经请求的关闭流程。
    FinishShutdown,
}

/// Smithay Linux adapter skeleton 的结构化错误。
#[derive(Debug)]
pub enum SmithayLinuxAdapterError {
    /// Display 或 listening socket 初始化失败。
    ResourceInitialization {
        /// 保留可跨线程传递的底层错误信息。
        source: Box<dyn Error + Send + Sync>,
    },

    /// 请求的生命周期操作不适用于当前状态。
    InvalidLifecycleTransition {
        /// 收到操作时的生命周期状态。
        from: SmithayLinuxAdapterLifecycle,

        /// 被拒绝的生命周期操作。
        operation: SmithayLinuxAdapterOperation,
    },

    /// 请求的 event pump 操作不适用于当前状态。
    InvalidPumpTransition {
        /// 收到操作时的 event pump 状态。
        from: SmithayLinuxAdapterPumpState,

        /// 被拒绝的 event pump 操作。
        operation: SmithayLinuxAdapterPumpOperation,
    },

    /// 请求的 global registration skeleton 操作已经执行过。
    InvalidGlobalRegistrationTransition {
        /// 收到操作时是否已经尝试 registration skeleton。
        attempted: bool,

        /// 被拒绝的 registration skeleton 操作。
        operation: SmithayLinuxAdapterGlobalRegistrationOperation,
    },
}

impl fmt::Display for SmithayLinuxAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceInitialization { source } => {
                write!(formatter, "Smithay Linux adapter 资源初始化失败: {source}")
            }
            Self::InvalidLifecycleTransition { from, operation } => write!(
                formatter,
                "Smithay Linux adapter 生命周期转换无效: state={from:?}, operation={operation:?}"
            ),
            Self::InvalidPumpTransition { from, operation } => write!(
                formatter,
                "Smithay Linux adapter event pump 转换无效: state={from:?}, operation={operation:?}"
            ),
            Self::InvalidGlobalRegistrationTransition {
                attempted,
                operation,
            } => write!(
                formatter,
                "Smithay Linux adapter global registration skeleton 转换无效: \
                 attempted={attempted}, operation={operation:?}"
            ),
        }
    }
}

impl Error for SmithayLinuxAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResourceInitialization { source } => Some(source.as_ref()),
            Self::InvalidLifecycleTransition { .. }
            | Self::InvalidPumpTransition { .. }
            | Self::InvalidGlobalRegistrationTransition { .. } => None,
        }
    }
}

/// 真实 Smithay adapter 的 Phase 48A 结构骨架。
///
/// 该结构只持有已经封装的 Linux bootstrap 资源和显式生命周期状态。它不提供
/// bootstrap 可变访问，不暴露系统 handle，也没有启动真实 compositor 的入口。
pub struct SmithayLinuxAdapterSkeleton {
    /// 已封装的 Display 与 listening socket 资源。
    bootstrap: SmithayBootstrapProbe,

    /// 当前 adapter skeleton 生命周期。
    lifecycle: SmithayLinuxAdapterLifecycle,

    /// 当前 event pump skeleton 状态。
    pump_state: SmithayLinuxAdapterPumpState,

    /// event pump skeleton 累计统计。
    pump_stats: SmithayLinuxAdapterPumpStats,

    /// 最近一次成功的 skeleton pump 结果。
    last_pump_result: Option<SmithayLinuxAdapterPumpResult>,

    /// 一次性 protocol global registration skeleton ledger。
    global_registration_report: Option<SmithayLinuxAdapterGlobalRegistrationReport>,
}

impl SmithayLinuxAdapterSkeleton {
    /// 使用自动选择的 socket 名称构造 adapter skeleton。
    pub fn new_auto() -> Result<Self, SmithayLinuxAdapterError> {
        let bootstrap = SmithayBootstrapProbe::new_auto().map_err(resource_initialization_error)?;

        Ok(Self::from_bootstrap(bootstrap))
    }

    /// 使用指定 socket 名称构造 adapter skeleton。
    pub fn with_socket_name(
        socket_name: impl Into<String>,
    ) -> Result<Self, SmithayLinuxAdapterError> {
        let socket_name = socket_name.into();
        let bootstrap = SmithayBootstrapProbe::with_socket_name(&socket_name)
            .map_err(resource_initialization_error)?;

        Ok(Self::from_bootstrap(bootstrap))
    }

    /// 使用已构造的 bootstrap 资源创建 adapter skeleton。
    pub fn from_bootstrap(bootstrap: SmithayBootstrapProbe) -> Self {
        Self {
            bootstrap,
            lifecycle: SmithayLinuxAdapterLifecycle::Prepared,
            pump_state: SmithayLinuxAdapterPumpState::NotStarted,
            pump_stats: SmithayLinuxAdapterPumpStats::empty(),
            last_pump_result: None,
            global_registration_report: None,
        }
    }

    /// 返回当前生命周期状态。
    pub fn lifecycle(&self) -> SmithayLinuxAdapterLifecycle {
        self.lifecycle
    }

    /// 返回固定的保守能力集合。
    pub fn capabilities(&self) -> SmithayLinuxAdapterCapabilities {
        SmithayLinuxAdapterCapabilities::skeleton_only()
    }

    /// 返回当前 event pump skeleton 状态。
    pub fn pump_state(&self) -> SmithayLinuxAdapterPumpState {
        self.pump_state
    }

    /// 返回 event pump skeleton 累计统计的只读快照。
    pub fn pump_stats(&self) -> SmithayLinuxAdapterPumpStats {
        self.pump_stats
    }

    /// 返回最近一次成功的 skeleton pump 结果。
    pub fn last_pump_result(&self) -> Option<SmithayLinuxAdapterPumpResult> {
        self.last_pump_result
    }

    /// 返回按稳定顺序生成的 adapter 结构化诊断。
    pub fn diagnostics(&self) -> Vec<SmithayLinuxAdapterDiagnostic> {
        let capabilities = self.capabilities();
        let mut diagnostics = Vec::with_capacity(18);

        if capabilities.is_skeleton_only {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::SkeletonOnly);
        }
        if capabilities.holds_wayland_display {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::HoldsWaylandDisplay);
        }
        if capabilities.holds_listening_socket {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::HoldsListeningSocket);
        }
        if capabilities.has_event_pump_boundary {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::EventPumpBoundaryPresent);
        }
        if !capabilities.runs_event_loop {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::EventLoopNotRunning);
        }
        if !capabilities.accepts_clients {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ClientsNotAccepted);
        }
        if !capabilities.dispatches_protocol_events {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ProtocolEventsNotDispatched);
        }
        if capabilities.has_protocol_global_plan_boundary {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalPlanPresent);
            if self.global_registration_report.is_none() {
                diagnostics.push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalsPlannedOnly);
            }
        }
        if capabilities.has_protocol_global_registration_boundary {
            diagnostics
                .push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalRegistrationBoundaryPresent);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalRegistrationSkeletonOnly);
            if self.global_registration_report.is_some() {
                diagnostics
                    .push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalsRegistrationAttempted);
            }
            diagnostics
                .push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalsRealRegistrationUnsupported);
        }
        if !capabilities.registers_protocol_globals {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ProtocolGlobalsNotRegistered);
        }
        if !capabilities.supports_real_wayland_surfaces {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::RealSurfacesUnsupported);
        }
        if !capabilities.supports_gpu_rendering {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::GpuRenderingUnsupported);
        }

        match self.lifecycle {
            SmithayLinuxAdapterLifecycle::Prepared => {}
            SmithayLinuxAdapterLifecycle::ShutdownRequested => {
                diagnostics.push(SmithayLinuxAdapterDiagnostic::ShutdownRequested);
            }
            SmithayLinuxAdapterLifecycle::Stopped => {
                diagnostics.push(SmithayLinuxAdapterDiagnostic::AdapterStopped);
            }
        }

        diagnostics
    }

    /// 返回按固定顺序排列的 protocol global 纯数据计划。
    pub fn planned_globals(&self) -> Vec<SmithayLinuxAdapterGlobalPlan> {
        self.global_registration_report.as_ref().map_or_else(
            || PROTOCOL_GLOBAL_PLAN.to_vec(),
            |report| report.globals.clone(),
        )
    }

    /// 返回当前 protocol global 计划的保守报告。
    pub fn global_plan_report(&self) -> SmithayLinuxAdapterGlobalPlanReport {
        let planned = self.planned_globals();

        SmithayLinuxAdapterGlobalPlanReport {
            planned_count: planned.len(),
            registered_count: 0,
            skeleton_only: planned.iter().all(|plan| plan.skeleton_only),
            planned,
        }
    }

    /// 记录全部计划 global 的一次性 registration skeleton ledger。
    pub fn register_planned_globals_skeleton(
        &mut self,
    ) -> Result<SmithayLinuxAdapterGlobalRegistrationReport, SmithayLinuxAdapterError> {
        let operation =
            SmithayLinuxAdapterGlobalRegistrationOperation::RegisterPlannedGlobalsSkeleton;

        if self.global_registration_report.is_some() {
            return Err(
                SmithayLinuxAdapterError::InvalidGlobalRegistrationTransition {
                    attempted: true,
                    operation,
                },
            );
        }

        let mut globals = PROTOCOL_GLOBAL_PLAN.to_vec();
        for global in &mut globals {
            global.state = SmithayLinuxAdapterGlobalRegistrationState::RegistrationSkeleton;
        }

        let report = SmithayLinuxAdapterGlobalRegistrationReport {
            attempted: true,
            skeleton_registered_count: globals.len(),
            real_registered_count: 0,
            planned_count: globals.len(),
            skeleton_only: globals.iter().all(|global| global.skeleton_only),
            globals,
        };
        self.global_registration_report = Some(report.clone());

        Ok(report)
    }

    /// 返回当前 registration skeleton ledger 报告。
    pub fn global_registration_report(
        &self,
    ) -> Option<SmithayLinuxAdapterGlobalRegistrationReport> {
        self.global_registration_report.clone()
    }

    /// 返回 adapter 当前状态的纯数据只读快照。
    pub fn snapshot(&self) -> SmithayLinuxAdapterSnapshot {
        SmithayLinuxAdapterSnapshot {
            lifecycle: self.lifecycle,
            pump_state: self.pump_state,
            capabilities: self.capabilities(),
            global_plan: self.global_plan_report(),
            global_registration_report: self.global_registration_report(),
            pump_stats: self.pump_stats,
            last_pump_result: self.last_pump_result,
            diagnostics: self.diagnostics(),
            socket_name: self.socket_name_string(),
            is_skeleton_only: self.is_skeleton_only(),
        }
    }

    /// 返回 bootstrap 已绑定的 listening socket 名称。
    pub fn socket_name_string(&self) -> String {
        self.bootstrap.socket_name_string()
    }

    /// 从 `Prepared` 转换到 `ShutdownRequested`。
    pub fn request_shutdown(&mut self) -> Result<(), SmithayLinuxAdapterError> {
        self.transition(
            SmithayLinuxAdapterLifecycle::Prepared,
            SmithayLinuxAdapterLifecycle::ShutdownRequested,
            SmithayLinuxAdapterOperation::RequestShutdown,
        )?;

        if self.pump_state == SmithayLinuxAdapterPumpState::Ready {
            self.pump_state = SmithayLinuxAdapterPumpState::StopRequested;
        }

        Ok(())
    }

    /// 从 `ShutdownRequested` 转换到 `Stopped`。
    pub fn finish_shutdown(&mut self) -> Result<(), SmithayLinuxAdapterError> {
        self.transition(
            SmithayLinuxAdapterLifecycle::ShutdownRequested,
            SmithayLinuxAdapterLifecycle::Stopped,
            SmithayLinuxAdapterOperation::FinishShutdown,
        )?;
        self.pump_state = SmithayLinuxAdapterPumpState::Stopped;

        Ok(())
    }

    /// 从 `NotStarted` 转换到 `Ready`，不启动真实事件循环。
    pub fn start_pump(&mut self) -> Result<(), SmithayLinuxAdapterError> {
        if self.lifecycle != SmithayLinuxAdapterLifecycle::Prepared
            || self.pump_state != SmithayLinuxAdapterPumpState::NotStarted
        {
            return Err(self.invalid_pump_transition(SmithayLinuxAdapterPumpOperation::StartPump));
        }

        self.pump_state = SmithayLinuxAdapterPumpState::Ready;

        Ok(())
    }

    /// 在 `Ready` 状态执行一次纯计数 skeleton tick。
    pub fn pump_once(&mut self) -> Result<SmithayLinuxAdapterPumpResult, SmithayLinuxAdapterError> {
        if self.lifecycle != SmithayLinuxAdapterLifecycle::Prepared
            || self.pump_state != SmithayLinuxAdapterPumpState::Ready
        {
            return Err(self.invalid_pump_transition(SmithayLinuxAdapterPumpOperation::PumpOnce));
        }

        self.pump_stats.total_ticks = self.pump_stats.total_ticks.saturating_add(1);

        let result = SmithayLinuxAdapterPumpResult {
            state: self.pump_state,
            tick_index: self.pump_stats.total_ticks,
            processed_clients: self.pump_stats.processed_clients,
            processed_protocol_events: self.pump_stats.processed_protocol_events,
            registered_globals: self.pump_stats.registered_globals,
            is_skeleton_only: true,
        };
        self.last_pump_result = Some(result);

        Ok(result)
    }

    /// 停止 event pump skeleton；不执行真实资源或协议收尾。
    pub fn stop_pump(&mut self) -> Result<(), SmithayLinuxAdapterError> {
        if self.lifecycle == SmithayLinuxAdapterLifecycle::Stopped
            || self.pump_state == SmithayLinuxAdapterPumpState::Stopped
        {
            return Err(self.invalid_pump_transition(SmithayLinuxAdapterPumpOperation::StopPump));
        }

        self.pump_state = SmithayLinuxAdapterPumpState::Stopped;

        Ok(())
    }

    /// 当前实例是否仍严格保持 skeleton 边界。
    pub fn is_skeleton_only(&self) -> bool {
        self.bootstrap.is_probe_only() && self.capabilities().is_skeleton_only
    }

    fn transition(
        &mut self,
        expected: SmithayLinuxAdapterLifecycle,
        next: SmithayLinuxAdapterLifecycle,
        operation: SmithayLinuxAdapterOperation,
    ) -> Result<(), SmithayLinuxAdapterError> {
        if self.lifecycle != expected {
            return Err(SmithayLinuxAdapterError::InvalidLifecycleTransition {
                from: self.lifecycle,
                operation,
            });
        }

        self.lifecycle = next;

        Ok(())
    }

    fn invalid_pump_transition(
        &self,
        operation: SmithayLinuxAdapterPumpOperation,
    ) -> SmithayLinuxAdapterError {
        SmithayLinuxAdapterError::InvalidPumpTransition {
            from: self.pump_state,
            operation,
        }
    }
}

fn resource_initialization_error(source: Box<dyn Error>) -> SmithayLinuxAdapterError {
    SmithayLinuxAdapterError::ResourceInitialization {
        source: Box::new(std::io::Error::other(source.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayLinuxAdapterDiagnostic, SmithayLinuxAdapterError, SmithayLinuxAdapterGlobalKind,
        SmithayLinuxAdapterGlobalPlan, SmithayLinuxAdapterGlobalRegistrationOperation,
        SmithayLinuxAdapterGlobalRegistrationState, SmithayLinuxAdapterLifecycle,
        SmithayLinuxAdapterOperation, SmithayLinuxAdapterPumpOperation,
        SmithayLinuxAdapterPumpState, SmithayLinuxAdapterPumpStats, SmithayLinuxAdapterSkeleton,
    };
    use crate::smithay_backend::{
        runtime_facade::{BackendBootstrapMode, BackendRuntimeDiagnostic, BackendRuntimeReport},
        test_support::{assert_runtime_dir, unique_socket_name},
    };

    #[test]
    fn adapter_skeleton_constructs_with_requested_socket() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-skeleton");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name.clone())
            .expect("adapter skeleton 必须持有指定名称的 bootstrap socket");

        assert_eq!(adapter.lifecycle(), SmithayLinuxAdapterLifecycle::Prepared);
        assert_eq!(
            adapter.pump_state(),
            SmithayLinuxAdapterPumpState::NotStarted
        );
        assert_eq!(
            adapter.pump_stats(),
            SmithayLinuxAdapterPumpStats {
                total_ticks: 0,
                processed_clients: 0,
                processed_protocol_events: 0,
                registered_globals: 0,
            }
        );
        assert_eq!(adapter.socket_name_string(), socket_name);
        assert_eq!(adapter.last_pump_result(), None);
        assert_eq!(adapter.global_registration_report(), None);
        assert!(adapter.is_skeleton_only());
    }

    #[test]
    fn adapter_diagnostics_have_stable_conservative_order() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-diagnostics");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        let expected = vec![
            SmithayLinuxAdapterDiagnostic::SkeletonOnly,
            SmithayLinuxAdapterDiagnostic::HoldsWaylandDisplay,
            SmithayLinuxAdapterDiagnostic::HoldsListeningSocket,
            SmithayLinuxAdapterDiagnostic::EventPumpBoundaryPresent,
            SmithayLinuxAdapterDiagnostic::EventLoopNotRunning,
            SmithayLinuxAdapterDiagnostic::ClientsNotAccepted,
            SmithayLinuxAdapterDiagnostic::ProtocolEventsNotDispatched,
            SmithayLinuxAdapterDiagnostic::ProtocolGlobalPlanPresent,
            SmithayLinuxAdapterDiagnostic::ProtocolGlobalsPlannedOnly,
            SmithayLinuxAdapterDiagnostic::ProtocolGlobalRegistrationBoundaryPresent,
            SmithayLinuxAdapterDiagnostic::ProtocolGlobalRegistrationSkeletonOnly,
            SmithayLinuxAdapterDiagnostic::ProtocolGlobalsRealRegistrationUnsupported,
            SmithayLinuxAdapterDiagnostic::ProtocolGlobalsNotRegistered,
            SmithayLinuxAdapterDiagnostic::RealSurfacesUnsupported,
            SmithayLinuxAdapterDiagnostic::GpuRenderingUnsupported,
        ];

        assert_eq!(adapter.diagnostics(), expected);
        assert_eq!(adapter.diagnostics(), expected);
    }

    #[test]
    fn adapter_snapshot_is_stable_comparable_read_only_data() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-snapshot");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name.clone())
            .expect("adapter skeleton 必须能够构造");
        let stats_before = adapter.pump_stats();
        let first = adapter.snapshot();
        let second = adapter.snapshot();

        assert_eq!(first, second);
        assert_eq!(first.clone(), first);
        assert_eq!(first.lifecycle, SmithayLinuxAdapterLifecycle::Prepared);
        assert_eq!(first.pump_state, SmithayLinuxAdapterPumpState::NotStarted);
        assert_eq!(first.pump_stats, stats_before);
        assert_eq!(first.last_pump_result, None);
        assert_eq!(first.global_plan.planned_count, 3);
        assert_eq!(first.global_plan.registered_count, 0);
        assert!(first.global_plan.skeleton_only);
        assert_eq!(first.global_registration_report, None);
        assert_eq!(first.socket_name, socket_name);
        assert!(first.is_skeleton_only);
        assert_eq!(adapter.pump_stats(), stats_before);
    }

    #[test]
    fn protocol_global_plan_has_stable_order_names_versions_and_states() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-global-plan");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        let expected = vec![
            SmithayLinuxAdapterGlobalPlan {
                kind: SmithayLinuxAdapterGlobalKind::Compositor,
                name: "wl_compositor",
                version: 6,
                state: SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly,
                skeleton_only: true,
            },
            SmithayLinuxAdapterGlobalPlan {
                kind: SmithayLinuxAdapterGlobalKind::Shm,
                name: "wl_shm",
                version: 1,
                state: SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly,
                skeleton_only: true,
            },
            SmithayLinuxAdapterGlobalPlan {
                kind: SmithayLinuxAdapterGlobalKind::XdgWmBase,
                name: "xdg_wm_base",
                version: 6,
                state: SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly,
                skeleton_only: true,
            },
        ];

        assert_eq!(adapter.planned_globals(), expected);
        assert_eq!(adapter.planned_globals(), expected);
        assert!(adapter.planned_globals().iter().all(|plan| {
            plan.state == SmithayLinuxAdapterGlobalRegistrationState::PlannedOnly
                && plan.state != SmithayLinuxAdapterGlobalRegistrationState::Registered
                && plan.skeleton_only
        }));

        let report = adapter.global_plan_report();
        assert_eq!(report.planned, expected);
        assert_eq!(report.planned_count, 3);
        assert_eq!(report.registered_count, 0);
        assert!(report.skeleton_only);
        assert_eq!(adapter.snapshot().global_plan, report);
    }

    #[test]
    fn protocol_global_plan_queries_do_not_mutate_adapter_state() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-global-plan-read-only");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let last_pump_result = adapter.last_pump_result();

        let first = adapter.planned_globals();
        let second = adapter.global_plan_report();

        assert_eq!(first, second.planned);
        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), last_pump_result);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
    }

    #[test]
    fn protocol_global_registration_skeleton_records_once_without_real_registration() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-global-registration");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let report = adapter
            .register_planned_globals_skeleton()
            .expect("首次 registration skeleton 必须成功");

        assert!(report.attempted);
        assert_eq!(report.planned_count, 3);
        assert_eq!(report.skeleton_registered_count, 3);
        assert_eq!(report.real_registered_count, 0);
        assert!(report.skeleton_only);
        assert!(report.globals.iter().all(|global| {
            global.state == SmithayLinuxAdapterGlobalRegistrationState::RegistrationSkeleton
                && global.state != SmithayLinuxAdapterGlobalRegistrationState::Registered
                && global.skeleton_only
        }));
        assert_eq!(adapter.global_registration_report(), Some(report.clone()));
        assert_eq!(adapter.planned_globals(), report.globals);
        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));

        let plan_report = adapter.global_plan_report();
        assert_eq!(plan_report.planned_count, 3);
        assert_eq!(plan_report.registered_count, 0);
        assert!(plan_report.planned.iter().all(|global| {
            global.state == SmithayLinuxAdapterGlobalRegistrationState::RegistrationSkeleton
        }));

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.global_registration_report, Some(report));
        assert_eq!(snapshot.diagnostics, adapter.diagnostics());
        assert_eq!(adapter.diagnostics(), adapter.diagnostics());
        assert!(
            !snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ProtocolGlobalsPlannedOnly)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ProtocolGlobalsRegistrationAttempted)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ProtocolGlobalRegistrationSkeletonOnly)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ProtocolGlobalsNotRegistered)
        );
    }

    #[test]
    fn repeated_protocol_global_registration_skeleton_returns_structured_error() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-global-registration-repeat");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("首次 registration skeleton 必须成功");
        let report_before = adapter
            .global_registration_report()
            .expect("首次调用后必须保存报告");

        let error = adapter
            .register_planned_globals_skeleton()
            .expect_err("重复 registration skeleton 必须失败");

        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidGlobalRegistrationTransition {
                attempted: true,
                operation:
                    SmithayLinuxAdapterGlobalRegistrationOperation::RegisterPlannedGlobalsSkeleton,
            }
        ));
        assert_eq!(adapter.global_registration_report(), Some(report_before));
    }

    #[test]
    fn adapter_skeleton_capabilities_remain_conservative() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-capabilities");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        let capabilities = adapter.capabilities();

        assert!(capabilities.holds_wayland_display);
        assert!(capabilities.holds_listening_socket);
        assert!(capabilities.has_adapter_lifecycle_boundary);
        assert!(capabilities.has_event_pump_boundary);
        assert!(capabilities.pumps_once);
        assert!(!capabilities.runs_event_loop);
        assert!(!capabilities.accepts_clients);
        assert!(capabilities.has_protocol_global_plan_boundary);
        assert!(capabilities.has_protocol_global_registration_boundary);
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
        assert!(capabilities.is_skeleton_only);
    }

    #[test]
    fn adapter_skeleton_follows_shutdown_lifecycle() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-lifecycle");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter
            .request_shutdown()
            .expect("Prepared 必须允许请求关闭");
        assert_eq!(
            adapter.lifecycle(),
            SmithayLinuxAdapterLifecycle::ShutdownRequested
        );
        let error = adapter
            .start_pump()
            .expect_err("ShutdownRequested 不得启动 pump");
        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidPumpTransition {
                from: SmithayLinuxAdapterPumpState::NotStarted,
                operation: SmithayLinuxAdapterPumpOperation::StartPump,
            }
        ));

        adapter
            .finish_shutdown()
            .expect("ShutdownRequested 必须允许完成关闭");
        assert_eq!(adapter.lifecycle(), SmithayLinuxAdapterLifecycle::Stopped);
        assert!(adapter.is_skeleton_only());
    }

    #[test]
    fn adapter_skeleton_pumps_ticks_without_processing_real_work() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-pump-ticks");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        assert_eq!(adapter.pump_state(), SmithayLinuxAdapterPumpState::Ready);
        assert_eq!(adapter.last_pump_result(), None);

        let first = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        assert_eq!(first.state, SmithayLinuxAdapterPumpState::Ready);
        assert_eq!(first.tick_index, 1);
        assert_eq!(first.processed_clients, 0);
        assert_eq!(first.processed_protocol_events, 0);
        assert_eq!(first.registered_globals, 0);
        assert!(first.is_skeleton_only);
        assert_eq!(adapter.last_pump_result(), Some(first));
        assert_eq!(adapter.snapshot().last_pump_result, Some(first));

        let second = adapter
            .pump_once()
            .expect("Ready 必须允许后续 skeleton tick");
        assert_eq!(second.tick_index, 2);
        assert_eq!(
            adapter.pump_stats(),
            SmithayLinuxAdapterPumpStats {
                total_ticks: 2,
                processed_clients: 0,
                processed_protocol_events: 0,
                registered_globals: 0,
            }
        );
        assert_eq!(adapter.last_pump_result(), Some(second));
    }

    #[test]
    fn pump_once_before_start_returns_structured_error() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-pump-before-start");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        let error = adapter
            .pump_once()
            .expect_err("NotStarted 不得执行 skeleton tick");

        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidPumpTransition {
                from: SmithayLinuxAdapterPumpState::NotStarted,
                operation: SmithayLinuxAdapterPumpOperation::PumpOnce,
            }
        ));
    }

    #[test]
    fn stopped_pump_rejects_further_operations() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-stop-pump");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        adapter.stop_pump().expect("Ready 必须允许停止 pump");
        assert_eq!(adapter.pump_state(), SmithayLinuxAdapterPumpState::Stopped);

        for (error, operation) in [
            (
                adapter.start_pump().expect_err("Stopped 不得重新启动 pump"),
                SmithayLinuxAdapterPumpOperation::StartPump,
            ),
            (
                adapter
                    .pump_once()
                    .expect_err("Stopped 不得执行 skeleton tick"),
                SmithayLinuxAdapterPumpOperation::PumpOnce,
            ),
            (
                adapter.stop_pump().expect_err("Stopped 不得重复停止 pump"),
                SmithayLinuxAdapterPumpOperation::StopPump,
            ),
        ] {
            assert!(matches!(
                error,
                SmithayLinuxAdapterError::InvalidPumpTransition {
                    from: SmithayLinuxAdapterPumpState::Stopped,
                    operation: actual_operation,
                } if actual_operation == operation
            ));
        }
    }

    #[test]
    fn failed_pump_and_stop_preserve_last_successful_result() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-last-pump-result");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let successful = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter.stop_pump().expect("Ready 必须允许停止 pump");

        let error = adapter
            .pump_once()
            .expect_err("Stopped 不得执行 skeleton tick");
        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidPumpTransition {
                from: SmithayLinuxAdapterPumpState::Stopped,
                operation: SmithayLinuxAdapterPumpOperation::PumpOnce,
            }
        ));
        assert_eq!(adapter.last_pump_result(), Some(successful));
        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.pump_state, SmithayLinuxAdapterPumpState::Stopped);
        assert_eq!(snapshot.last_pump_result, Some(successful));
    }

    #[test]
    fn shutdown_requests_and_finishes_pump_stop() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-pump-shutdown");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let successful = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter
            .request_shutdown()
            .expect("Prepared 必须允许请求关闭");
        assert_eq!(
            adapter.pump_state(),
            SmithayLinuxAdapterPumpState::StopRequested
        );
        assert!(
            adapter
                .diagnostics()
                .contains(&SmithayLinuxAdapterDiagnostic::ShutdownRequested)
        );

        let error = adapter
            .pump_once()
            .expect_err("ShutdownRequested 不得执行 skeleton tick");
        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidPumpTransition {
                from: SmithayLinuxAdapterPumpState::StopRequested,
                operation: SmithayLinuxAdapterPumpOperation::PumpOnce,
            }
        ));

        adapter
            .finish_shutdown()
            .expect("ShutdownRequested 必须允许完成关闭");
        assert_eq!(adapter.pump_state(), SmithayLinuxAdapterPumpState::Stopped);
        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.lifecycle, SmithayLinuxAdapterLifecycle::Stopped);
        assert_eq!(snapshot.pump_state, SmithayLinuxAdapterPumpState::Stopped);
        assert_eq!(snapshot.last_pump_result, Some(successful));
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::AdapterStopped)
        );
    }

    #[test]
    fn pump_can_stop_before_it_is_started() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-stop-before-start");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter
            .stop_pump()
            .expect("NotStarted 允许直接进入 Stopped");
        assert_eq!(adapter.pump_state(), SmithayLinuxAdapterPumpState::Stopped);
        assert_eq!(adapter.pump_stats().total_ticks, 0);
    }

    #[test]
    fn repeated_shutdown_request_returns_structured_error() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-repeat-shutdown");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        adapter.request_shutdown().expect("首次请求关闭必须成功");

        let error = adapter
            .request_shutdown()
            .expect_err("重复请求关闭必须返回错误");

        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidLifecycleTransition {
                from: SmithayLinuxAdapterLifecycle::ShutdownRequested,
                operation: SmithayLinuxAdapterOperation::RequestShutdown,
            }
        ));
    }

    #[test]
    fn finish_before_shutdown_request_returns_structured_error() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-finish-early");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");

        let error = adapter
            .finish_shutdown()
            .expect_err("Prepared 不得直接完成关闭");

        assert!(matches!(
            error,
            SmithayLinuxAdapterError::InvalidLifecycleTransition {
                from: SmithayLinuxAdapterLifecycle::Prepared,
                operation: SmithayLinuxAdapterOperation::FinishShutdown,
            }
        ));
    }

    #[test]
    fn adapter_skeleton_builds_conservative_runtime_report() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-report");
        let adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name.clone())
            .expect("adapter skeleton 必须能够构造");
        let report = BackendRuntimeReport::from(&adapter);

        assert_eq!(report.backend_name, "smithay-linux-adapter-skeleton");
        assert_eq!(report.bootstrap_mode, BackendBootstrapMode::ProbeOnly);
        assert_eq!(report.socket_name.as_deref(), Some(socket_name.as_str()));
        assert!(report.capabilities.can_create_display);
        assert!(report.capabilities.can_create_socket);
        assert!(!report.capabilities.supports_real_wayland_surfaces);
        assert!(!report.capabilities.supports_gpu_rendering);
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterEventPumpSkeleton {
                has_event_pump_boundary: true,
                pumps_once: true,
                runs_event_loop: false,
                accepts_clients: false,
                dispatches_protocol_events: false,
                registers_protocol_globals: false,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterProtocolGlobalPlan {
                planned_count: 3,
                registered_count: 0,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterProtocolGlobalRegistrationSkeleton {
                attempted: false,
                skeleton_registered_count: 0,
                real_registered_count: 0,
                skeleton_only: true,
            }
        )));
    }

    #[test]
    fn registered_skeleton_runtime_report_remains_conservative() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-registration-report");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("首次 registration skeleton 必须成功");
        let report = BackendRuntimeReport::from(&adapter);

        assert_eq!(report.bootstrap_mode, BackendBootstrapMode::ProbeOnly);
        assert!(!report.capabilities.supports_real_wayland_surfaces);
        assert!(!report.capabilities.supports_gpu_rendering);
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterProtocolGlobalRegistrationSkeleton {
                attempted: true,
                skeleton_registered_count: 3,
                real_registered_count: 0,
                skeleton_only: true,
            }
        )));
    }

    #[test]
    fn adapter_production_code_keeps_system_and_core_boundaries() {
        let source = include_str!("linux_adapter.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(production, _)| production);

        for forbidden in [
            "crate::core",
            "crate::backend",
            "BackendEvent",
            "CoreCommand",
            "BackendDriverRunner",
            "smithay::",
            "DisplayHandle",
            "Display<",
            "display_handle",
            "display(",
            "wayland_server::Display",
            "GlobalDispatch",
            "impl Dispatch",
            "create_global",
            "register_global",
            "delegate_",
            "calloop",
            "run_once",
            "accept(",
            "wl_surface",
            "xdg_toplevel",
            "drm",
            "gbm",
            "libinput",
            "udev",
            "x11",
            "vulkan",
        ] {
            assert!(
                !production.contains(forbidden),
                "adapter skeleton 生产代码不得引用边界外入口: {forbidden}"
            );
        }
    }
}
