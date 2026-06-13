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

/// Smithay Linux adapter 真实 protocol global 注册的可行性模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterRealGlobalRegistrationMode {
    /// 尚未执行可行性检查。
    Disabled,

    /// activation gate 或必要 handler 边界阻止了真实注册。
    FeasibilityBlocked,

    /// 为未来受控 inert 注册保留；Phase 49A fallback 不会产生该状态。
    InertRegistrationAttempted,

    /// 为未来受控 inert 注册保留；Phase 49A fallback 不会产生该状态。
    InertRegistrationSucceeded,
}

/// 阻止真实 protocol global 注册可行性切片的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterRealGlobalRegistrationBlocker {
    /// Phase 48 activation gate 阻止了真实 global registration target。
    ActivationGateBlocked,

    /// 尚未建立 client bind 所需的 global handler。
    GlobalBindHandlerUnavailable,

    /// 尚未建立协议请求处理 handler。
    ProtocolRequestHandlerUnavailable,

    /// 当前 adapter 不支持 surface 请求。
    SurfaceRequestsUnsupported,

    /// 当前 adapter 不支持真实 client handling。
    ClientHandlingUnsupported,

    /// 当前 adapter 不支持协议 dispatch。
    ProtocolDispatchUnsupported,

    /// 当前 adapter 不支持进入核心集成边界。
    CoreIntegrationUnsupported,
}

/// Smithay Linux adapter 真实 protocol global 注册可行性报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterRealGlobalRegistrationReport {
    /// 当前可行性模式。
    pub mode: SmithayLinuxAdapterRealGlobalRegistrationMode,

    /// 实际进入真实注册调用的 global；blocked fallback 恒为空。
    pub attempted_kinds: Vec<SmithayLinuxAdapterGlobalKind>,

    /// 实际完成真实注册的 global；blocked fallback 恒为空。
    pub succeeded_kinds: Vec<SmithayLinuxAdapterGlobalKind>,

    /// 被可行性边界阻止的 global，按计划顺序排列。
    pub blocked_kinds: Vec<SmithayLinuxAdapterGlobalKind>,

    /// 来自 Phase 48 activation gate 的原始 blocker。
    pub activation_blockers: Vec<SmithayLinuxAdapterActivationBlocker>,

    /// 更具体的真实 registration 可行性 blocker。
    pub blockers: Vec<SmithayLinuxAdapterRealGlobalRegistrationBlocker>,

    /// 实际真实注册 global 数量；blocked fallback 恒为零。
    pub real_registered_count: usize,

    /// 当前报告是否仍然只属于受控 skeleton 可行性边界。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter 未来 protocol global handler 的类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterGlobalHandlerKind {
    /// `wl_compositor` 及其 surface/subsurface handler 边界。
    CompositorGlobalHandler,

    /// `wl_shm`、pool 和 buffer handler 边界。
    ShmGlobalHandler,

    /// `xdg_wm_base` 及其 shell object handler 边界。
    XdgWmBaseGlobalHandler,
}

/// Smithay Linux adapter protocol global handler 的准备状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterGlobalHandlerReadiness {
    /// 尚未建立任何 handler 边界。
    Missing,

    /// trait 与安全边界审计表明当前不能建立 inert handler。
    FeasibilityBlocked,

    /// 为未来纯编译边界规划保留；Phase 49B fallback 不会产生该状态。
    InertBoundaryPlanned,

    /// 为未来安全编译边界保留；Phase 49B fallback 不会产生该状态。
    InertBoundaryCompiled,
}

/// 阻止 protocol global handler 边界就绪的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterGlobalHandlerBlocker {
    /// 缺少 global bind trait 实现。
    MissingGlobalDispatchImplementation,

    /// 缺少 protocol object request trait 实现。
    MissingDispatchImplementation,

    /// 缺少 shared-memory buffer handler。
    MissingBufferHandler,

    /// 缺少 shared-memory state handler。
    MissingShmHandler,

    /// 缺少 compositor surface handler。
    MissingCompositorHandler,

    /// 缺少 XDG shell handler。
    MissingXdgShellHandler,

    /// 当前 adapter 不支持 surface 请求。
    SurfaceRequestsUnsupported,

    /// 当前 adapter 不支持真实 client handling。
    ClientHandlingUnsupported,

    /// 当前 adapter 不支持协议 dispatch。
    ProtocolDispatchUnsupported,

    /// 当前 adapter 不支持进入核心集成边界。
    CoreIntegrationUnsupported,

    /// Phase 48 activation gate 仍阻止真实 global registration。
    ActivationGateBlocked,
}

/// 单类 protocol global handler 的准备状态报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterGlobalHandlerReadinessReport {
    /// handler 类别。
    pub kind: SmithayLinuxAdapterGlobalHandlerKind,

    /// 当前准备状态。
    pub readiness: SmithayLinuxAdapterGlobalHandlerReadiness,

    /// 按稳定顺序排列且非空的 blocker。
    pub blockers: Vec<SmithayLinuxAdapterGlobalHandlerBlocker>,

    /// 当前报告是否仍然只属于 skeleton 可行性边界。
    pub skeleton_only: bool,
}

/// 全部 protocol global handler 的聚合边界报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterGlobalHandlerBoundaryReport {
    /// 按 compositor、SHM、XDG 顺序排列的 handler 报告。
    pub reports: Vec<SmithayLinuxAdapterGlobalHandlerReadinessReport>,

    /// 已安全建立 inert 编译边界的 handler 数量；当前恒为零。
    pub ready_count: usize,

    /// 被可行性边界阻止的 handler 数量。
    pub blocked_count: usize,

    /// 当前报告是否仍然只属于 skeleton 可行性边界。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter 未来可能观察到的协议请求类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterProtocolRequestKind {
    /// 未来 compositor surface 创建请求的纯数据标识。
    CompositorCreateSurface,

    /// 未来 compositor region 创建请求的纯数据标识。
    CompositorCreateRegion,

    /// 未来 shared-memory pool 创建请求的纯数据标识。
    ShmCreatePool,

    /// 未来 XDG positioner 创建请求的纯数据标识。
    XdgWmBaseCreatePositioner,

    /// 未来 XDG surface 获取请求的纯数据标识。
    XdgWmBaseGetXdgSurface,

    /// 未来 XDG ping 回复请求的纯数据标识。
    XdgWmBasePong,
}

/// Smithay Linux adapter 拒绝协议请求的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterUnsupportedRequestReason {
    /// 当前 adapter 仍然只提供 skeleton。
    SkeletonOnly,

    /// 当前 adapter 不支持协议 request dispatch。
    ProtocolDispatchUnsupported,

    /// 当前 adapter 不支持真实 surface。
    RealSurfaceUnsupported,

    /// 当前 adapter 不支持 XDG toplevel。
    XdgToplevelUnsupported,

    /// 当前 adapter 不支持客户端处理。
    ClientHandlingUnsupported,

    /// 当前 adapter 不支持进入核心接纳流程。
    CoreAdmissionUnsupported,
}

/// Smithay Linux adapter 对协议请求的保守结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterProtocolRequestOutcome {
    /// 请求被明确拒绝，不会被静默忽略或伪装为成功。
    RejectedUnsupported {
        /// 拒绝该请求的固定结构化原因。
        reason: SmithayLinuxAdapterUnsupportedRequestReason,
    },
}

/// 单次 unsupported protocol request observation。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterProtocolRequestObservation {
    /// 从 1 开始递增的稳定 ledger 序号。
    pub sequence: u64,

    /// 被观察到的纯数据请求类别。
    pub kind: SmithayLinuxAdapterProtocolRequestKind,

    /// 当前阶段恒为明确拒绝的结果。
    pub outcome: SmithayLinuxAdapterProtocolRequestOutcome,

    /// 当前 observation 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter unsupported protocol request ledger 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterProtocolRequestLedgerReport {
    /// 已观察请求数量。
    pub observed_count: usize,

    /// 被明确拒绝为 unsupported 的请求数量。
    pub rejected_unsupported_count: usize,

    /// 按 observation 顺序保存的纯数据 ledger。
    pub observations: Vec<SmithayLinuxAdapterProtocolRequestObservation>,

    /// 当前 ledger 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter 观察到的纯数据 client session 标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmithayLinuxAdapterClientSessionId(u64);

impl SmithayLinuxAdapterClientSessionId {
    /// 返回从 1 开始分配的稳定纯数据标识值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Smithay Linux adapter client session ledger 的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterClientSessionState {
    /// client session 只在 skeleton 可见性边界中被观察到。
    ObservedSkeleton,

    /// client session 已被明确记录为当前阶段不支持。
    RejectedUnsupported,
}

/// Smithay Linux adapter 拒绝 client session 的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterClientUnsupportedReason {
    /// 当前 adapter 仍然只提供 skeleton。
    SkeletonOnly,

    /// 当前 adapter 不支持接受真实 client。
    ClientAcceptUnsupported,

    /// 当前 adapter 不支持协议 dispatch。
    ProtocolDispatchUnsupported,

    /// 当前 adapter 不支持接入核心状态。
    CoreIntegrationUnsupported,
}

/// Smithay Linux adapter 对 client session observation 的保守结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxAdapterClientSessionOutcome {
    /// client session 被明确拒绝，不会被静默忽略或伪装为成功。
    RejectedUnsupported {
        /// 拒绝 client session 的固定结构化原因。
        reason: SmithayLinuxAdapterClientUnsupportedReason,
    },
}

/// 单次 unsupported client session observation。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayLinuxAdapterClientSessionObservation {
    /// 从 1 开始递增的稳定 ledger 序号。
    pub sequence: u64,

    /// 从 1 开始分配的纯数据 session 标识。
    pub session_id: SmithayLinuxAdapterClientSessionId,

    /// 当前阶段恒为明确拒绝的 ledger 状态。
    pub state: SmithayLinuxAdapterClientSessionState,

    /// 当前阶段恒为明确拒绝的结果。
    pub outcome: SmithayLinuxAdapterClientSessionOutcome,

    /// 当前 observation 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter unsupported client session ledger 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterClientSessionLedgerReport {
    /// 已观察 client session 数量。
    pub observed_count: usize,

    /// 被明确拒绝为 unsupported 的 client session 数量。
    pub rejected_unsupported_count: usize,

    /// 按 observation 顺序保存的纯数据 ledger。
    pub observations: Vec<SmithayLinuxAdapterClientSessionObservation>,

    /// 当前 ledger 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter 未来可能激活的真实能力目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterActivationTarget {
    /// 真实事件循环。
    RealEventLoop,

    /// 真实 client 接受路径。
    RealClientAccept,

    /// 真实 protocol global 注册。
    RealProtocolGlobalRegistration,

    /// 真实协议请求分发。
    RealProtocolDispatch,

    /// 真实 surface 生命周期。
    RealSurfaceLifecycle,

    /// 真实 XDG toplevel 生命周期。
    RealXdgToplevelLifecycle,

    /// 核心接纳流程。
    CoreAdmission,

    /// GPU 渲染。
    GpuRendering,
}

/// 阻止 Smithay Linux adapter 激活真实能力的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterActivationBlocker {
    /// 当前 adapter 仍然只提供 skeleton。
    SkeletonOnly,

    /// 当前 adapter 不支持真实事件循环。
    EventLoopUnsupported,

    /// 当前 adapter 不支持接受真实 client。
    ClientAcceptUnsupported,

    /// 当前 adapter 不支持真实 protocol global 注册。
    ProtocolGlobalRegistrationUnsupported,

    /// 当前 adapter 不支持协议 dispatch。
    ProtocolDispatchUnsupported,

    /// 当前 adapter 不支持真实 surface。
    RealSurfaceUnsupported,

    /// 当前 adapter 不支持 XDG toplevel。
    XdgToplevelUnsupported,

    /// 当前 adapter 不支持进入核心接纳流程。
    CoreAdmissionUnsupported,

    /// 当前 adapter 不支持 GPU 渲染。
    GpuRenderingUnsupported,

    /// 尚未实现协议 dispatch。
    MissingDispatchImplementation,

    /// 尚未实现 protocol global 注册。
    MissingProtocolGlobalImplementation,

    /// 尚未实现真实 surface 生命周期。
    MissingSurfaceLifecycleImplementation,

    /// 尚未实现核心集成。
    MissingCoreIntegration,
}

/// Smithay Linux adapter 真实能力激活决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxAdapterActivationDecision {
    /// 当前目标被明确阻止激活。
    Blocked,
}

/// 单项 Smithay Linux adapter 真实能力激活报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterActivationReport {
    /// 被评估的真实能力目标。
    pub target: SmithayLinuxAdapterActivationTarget,

    /// 当前阶段恒为 blocked 的激活决策。
    pub decision: SmithayLinuxAdapterActivationDecision,

    /// 按固定顺序排列且非空的阻止原因。
    pub blockers: Vec<SmithayLinuxAdapterActivationBlocker>,

    /// 当前报告是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter 全部真实能力的激活 gate 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterActivationGateReport {
    /// 按固定 target 顺序排列的激活报告。
    pub reports: Vec<SmithayLinuxAdapterActivationReport>,

    /// 被阻止激活的 target 数量。
    pub blocked_count: usize,

    /// 允许激活的 target 数量；当前阶段恒为零。
    pub allowed_count: usize,

    /// 当前 gate 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter 真实能力激活 attempt 的保守结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmithayLinuxAdapterActivationAttemptOutcome {
    /// attempt 被 activation gate 明确阻止。
    Blocked {
        /// 从目标 activation report 复制的稳定非空 blocker。
        blockers: Vec<SmithayLinuxAdapterActivationBlocker>,
    },
}

/// 单次 Smithay Linux adapter 真实能力激活 attempt observation。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterActivationAttemptObservation {
    /// 从 1 开始递增的稳定 ledger 序号。
    pub sequence: u64,

    /// 被尝试激活的真实能力目标。
    pub target: SmithayLinuxAdapterActivationTarget,

    /// activation gate 返回的决策；当前阶段恒为 blocked。
    pub decision: SmithayLinuxAdapterActivationDecision,

    /// 当前阶段恒为带结构化 blocker 的 blocked 结果。
    pub outcome: SmithayLinuxAdapterActivationAttemptOutcome,

    /// 当前 observation 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

/// Smithay Linux adapter activation attempt ledger 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxAdapterActivationAttemptLedgerReport {
    /// 已观察 activation attempt 数量。
    pub observed_count: usize,

    /// 被 activation gate 阻止的 attempt 数量。
    pub blocked_count: usize,

    /// 允许激活的 attempt 数量；当前阶段恒为零。
    pub allowed_count: usize,

    /// 按 observation 顺序保存的纯数据 ledger。
    pub observations: Vec<SmithayLinuxAdapterActivationAttemptObservation>,

    /// 当前 ledger 是否严格属于 skeleton。
    pub skeleton_only: bool,
}

const ACTIVATION_TARGETS: [SmithayLinuxAdapterActivationTarget; 8] = [
    SmithayLinuxAdapterActivationTarget::RealEventLoop,
    SmithayLinuxAdapterActivationTarget::RealClientAccept,
    SmithayLinuxAdapterActivationTarget::RealProtocolGlobalRegistration,
    SmithayLinuxAdapterActivationTarget::RealProtocolDispatch,
    SmithayLinuxAdapterActivationTarget::RealSurfaceLifecycle,
    SmithayLinuxAdapterActivationTarget::RealXdgToplevelLifecycle,
    SmithayLinuxAdapterActivationTarget::CoreAdmission,
    SmithayLinuxAdapterActivationTarget::GpuRendering,
];

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

    /// adapter 提供真实能力激活 gate。
    ActivationGatePresent,

    /// adapter 提供纯数据 activation attempt ledger。
    ActivationAttemptLedgerPresent,

    /// 所有已观察 activation attempt 都被 gate 明确阻止。
    ActivationAttemptsBlocked,

    /// adapter 提供真实 global registration 的受控可行性边界。
    RealGlobalRegistrationFeasibilityPresent,

    /// 最近一次真实 global registration 可行性检查被明确阻止。
    RealGlobalRegistrationBlocked,

    /// 真实 global registration 当前未启用。
    RealGlobalRegistrationNotEnabled,

    /// adapter 提供纯数据 protocol global handler 边界。
    GlobalHandlerBoundaryPresent,

    /// 三类 protocol global handler 当前均被可行性边界阻止。
    GlobalHandlersFeasibilityBlocked,

    /// global bind trait 实现尚未建立。
    GlobalDispatchImplementationMissing,

    /// protocol object request trait 实现尚未建立。
    DispatchImplementationMissing,

    /// 所有真实能力当前都被明确阻止激活。
    AllRealCapabilitiesBlocked,

    /// 真实事件循环被阻止激活。
    RealEventLoopBlocked,

    /// 真实 client 接受路径被阻止激活。
    RealClientAcceptBlocked,

    /// 真实协议分发被阻止激活。
    RealProtocolDispatchBlocked,

    /// 真实 surface 生命周期被阻止激活。
    RealSurfaceLifecycleBlocked,

    /// 核心接纳流程被阻止激活。
    CoreAdmissionBlocked,

    /// GPU 渲染被阻止激活。
    GpuRenderingBlocked,

    /// adapter 未运行真实事件循环。
    EventLoopNotRunning,

    /// adapter 未接受客户端连接。
    ClientsNotAccepted,

    /// adapter 提供纯数据 client session ledger。
    ClientSessionLedgerPresent,

    /// 所有已观察 client session 都被明确拒绝为 unsupported。
    ClientSessionsRejectedUnsupported,

    /// client session 因当前不支持接受真实 client 而被拒绝。
    ClientAcceptUnsupported,

    /// adapter 未分发协议事件。
    ProtocolEventsNotDispatched,

    /// adapter 提供 inert protocol request ledger。
    InertProtocolRequestLedgerPresent,

    /// 所有已观察协议请求都被明确拒绝为 unsupported。
    ProtocolRequestsRejectedUnsupported,

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

    /// 是否提供真实能力激活 gate。
    pub has_activation_gate: bool,

    /// 是否提供纯数据 activation attempt ledger。
    pub has_activation_attempt_ledger: bool,

    /// 是否提供真实 global registration 的受控可行性边界。
    pub has_real_global_registration_feasibility: bool,

    /// 是否提供 protocol global handler 的纯数据可行性边界。
    pub has_global_handler_boundary: bool,

    /// 是否提供显式 event pump 边界。
    pub has_event_pump_boundary: bool,

    /// 是否支持执行一次 skeleton tick；不代表真实事件分发。
    pub pumps_once: bool,

    /// 是否运行调度循环。
    pub runs_event_loop: bool,

    /// 是否接受真实客户端连接。
    pub accepts_clients: bool,

    /// 是否提供纯数据 client session ledger。
    pub has_client_session_ledger: bool,

    /// 是否提供 protocol global 的纯数据计划边界。
    pub has_protocol_global_plan_boundary: bool,

    /// 是否提供 protocol global registration skeleton 边界。
    pub has_protocol_global_registration_boundary: bool,

    /// 是否提供 inert protocol request ledger。
    pub has_inert_protocol_request_ledger: bool,

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
            has_activation_gate: true,
            has_activation_attempt_ledger: true,
            has_real_global_registration_feasibility: true,
            has_global_handler_boundary: true,
            has_event_pump_boundary: true,
            pumps_once: true,
            runs_event_loop: false,
            accepts_clients: false,
            has_client_session_ledger: true,
            has_protocol_global_plan_boundary: true,
            has_protocol_global_registration_boundary: true,
            has_inert_protocol_request_ledger: true,
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

    /// adapter 当前真实能力激活 gate 报告。
    pub activation_gate: SmithayLinuxAdapterActivationGateReport,

    /// adapter 当前 activation attempt ledger。
    pub activation_attempt_ledger: SmithayLinuxAdapterActivationAttemptLedgerReport,

    /// adapter 当前 unsupported client session ledger。
    pub client_session_ledger: SmithayLinuxAdapterClientSessionLedgerReport,

    /// adapter 当前 protocol global 计划报告。
    pub global_plan: SmithayLinuxAdapterGlobalPlanReport,

    /// adapter 当前 registration skeleton 报告。
    pub global_registration_report: Option<SmithayLinuxAdapterGlobalRegistrationReport>,

    /// adapter 当前真实 global registration 可行性报告。
    pub real_global_registration_report: Option<SmithayLinuxAdapterRealGlobalRegistrationReport>,

    /// adapter 当前 protocol global handler 可行性边界。
    pub global_handler_boundary: SmithayLinuxAdapterGlobalHandlerBoundaryReport,

    /// adapter 当前 unsupported protocol request ledger。
    pub protocol_request_ledger: SmithayLinuxAdapterProtocolRequestLedgerReport,

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

    /// 最近一次真实 protocol global registration 可行性报告。
    real_global_registration_report: Option<SmithayLinuxAdapterRealGlobalRegistrationReport>,

    /// 已观察的 unsupported protocol request ledger。
    protocol_request_observations: Vec<SmithayLinuxAdapterProtocolRequestObservation>,

    /// 下一个 protocol request observation 序号。
    next_protocol_request_sequence: u64,

    /// 已观察的 unsupported client session ledger。
    client_session_observations: Vec<SmithayLinuxAdapterClientSessionObservation>,

    /// 下一个 client session observation 序号。
    next_client_session_sequence: u64,

    /// 下一个纯数据 client session 标识。
    next_client_session_id: u64,

    /// 已观察的 blocked activation attempt ledger。
    activation_attempt_observations: Vec<SmithayLinuxAdapterActivationAttemptObservation>,

    /// 下一个 activation attempt observation 序号。
    next_activation_attempt_sequence: u64,
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
            real_global_registration_report: None,
            protocol_request_observations: Vec::new(),
            next_protocol_request_sequence: 1,
            client_session_observations: Vec::new(),
            next_client_session_sequence: 1,
            next_client_session_id: 1,
            activation_attempt_observations: Vec::new(),
            next_activation_attempt_sequence: 1,
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
        let mut diagnostics = Vec::with_capacity(40);

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
        if capabilities.has_activation_gate {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ActivationGatePresent);
        }
        if capabilities.has_activation_attempt_ledger {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ActivationAttemptLedgerPresent);
            if !self.activation_attempt_observations.is_empty() {
                diagnostics.push(SmithayLinuxAdapterDiagnostic::ActivationAttemptsBlocked);
            }
        }
        if capabilities.has_real_global_registration_feasibility {
            diagnostics
                .push(SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationFeasibilityPresent);
            if self.real_global_registration_report.is_some() {
                diagnostics.push(SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationBlocked);
            }
            diagnostics.push(SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationNotEnabled);
        }
        if capabilities.has_global_handler_boundary {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::GlobalHandlerBoundaryPresent);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::GlobalHandlersFeasibilityBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::GlobalDispatchImplementationMissing);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::DispatchImplementationMissing);
        }
        if capabilities.has_activation_gate {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::AllRealCapabilitiesBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::RealEventLoopBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::RealClientAcceptBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::RealProtocolDispatchBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::RealSurfaceLifecycleBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::CoreAdmissionBlocked);
            diagnostics.push(SmithayLinuxAdapterDiagnostic::GpuRenderingBlocked);
        }
        if !capabilities.runs_event_loop {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::EventLoopNotRunning);
        }
        if !capabilities.accepts_clients {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ClientsNotAccepted);
        }
        if capabilities.has_client_session_ledger {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ClientSessionLedgerPresent);
            if !self.client_session_observations.is_empty() {
                diagnostics.push(SmithayLinuxAdapterDiagnostic::ClientSessionsRejectedUnsupported);
                diagnostics.push(SmithayLinuxAdapterDiagnostic::ClientAcceptUnsupported);
            }
        }
        if !capabilities.dispatches_protocol_events {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::ProtocolEventsNotDispatched);
        }
        if capabilities.has_inert_protocol_request_ledger {
            diagnostics.push(SmithayLinuxAdapterDiagnostic::InertProtocolRequestLedgerPresent);
            if !self.protocol_request_observations.is_empty() {
                diagnostics
                    .push(SmithayLinuxAdapterDiagnostic::ProtocolRequestsRejectedUnsupported);
            }
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

    /// 返回最近一次真实 protocol global registration 可行性报告。
    pub fn real_global_registration_report(
        &self,
    ) -> Option<SmithayLinuxAdapterRealGlobalRegistrationReport> {
        self.real_global_registration_report.clone()
    }

    /// 返回 protocol global handler 的稳定纯数据可行性边界。
    pub fn global_handler_boundary_report(&self) -> SmithayLinuxAdapterGlobalHandlerBoundaryReport {
        let reports = vec![
            global_handler_readiness_report(
                SmithayLinuxAdapterGlobalHandlerKind::CompositorGlobalHandler,
            ),
            global_handler_readiness_report(SmithayLinuxAdapterGlobalHandlerKind::ShmGlobalHandler),
            global_handler_readiness_report(
                SmithayLinuxAdapterGlobalHandlerKind::XdgWmBaseGlobalHandler,
            ),
        ];

        SmithayLinuxAdapterGlobalHandlerBoundaryReport {
            ready_count: 0,
            blocked_count: reports.len(),
            reports,
            skeleton_only: true,
        }
    }

    /// 把协议请求记录为明确拒绝的 inert ledger observation。
    pub fn observe_unsupported_protocol_request(
        &mut self,
        kind: SmithayLinuxAdapterProtocolRequestKind,
    ) -> SmithayLinuxAdapterProtocolRequestObservation {
        let observation = SmithayLinuxAdapterProtocolRequestObservation {
            sequence: self.next_protocol_request_sequence,
            kind,
            outcome: SmithayLinuxAdapterProtocolRequestOutcome::RejectedUnsupported {
                reason: unsupported_request_reason(kind),
            },
            skeleton_only: true,
        };

        self.next_protocol_request_sequence = self.next_protocol_request_sequence.saturating_add(1);
        self.protocol_request_observations.push(observation);

        observation
    }

    /// 返回当前 unsupported protocol request ledger 的纯数据报告。
    pub fn protocol_request_ledger_report(&self) -> SmithayLinuxAdapterProtocolRequestLedgerReport {
        SmithayLinuxAdapterProtocolRequestLedgerReport {
            observed_count: self.protocol_request_observations.len(),
            rejected_unsupported_count: self.protocol_request_observations.len(),
            observations: self.protocol_request_observations.clone(),
            skeleton_only: true,
        }
    }

    /// 把 client session 记录为明确拒绝的纯数据 ledger observation。
    pub fn observe_unsupported_client_session(
        &mut self,
    ) -> SmithayLinuxAdapterClientSessionObservation {
        let observation = SmithayLinuxAdapterClientSessionObservation {
            sequence: self.next_client_session_sequence,
            session_id: SmithayLinuxAdapterClientSessionId(self.next_client_session_id),
            state: SmithayLinuxAdapterClientSessionState::RejectedUnsupported,
            outcome: SmithayLinuxAdapterClientSessionOutcome::RejectedUnsupported {
                reason: SmithayLinuxAdapterClientUnsupportedReason::ClientAcceptUnsupported,
            },
            skeleton_only: true,
        };

        self.next_client_session_sequence = self.next_client_session_sequence.saturating_add(1);
        self.next_client_session_id = self.next_client_session_id.saturating_add(1);
        self.client_session_observations.push(observation);

        observation
    }

    /// 返回当前 unsupported client session ledger 的纯数据报告。
    pub fn client_session_ledger_report(&self) -> SmithayLinuxAdapterClientSessionLedgerReport {
        SmithayLinuxAdapterClientSessionLedgerReport {
            observed_count: self.client_session_observations.len(),
            rejected_unsupported_count: self.client_session_observations.len(),
            observations: self.client_session_observations.clone(),
            skeleton_only: true,
        }
    }

    /// 返回指定真实能力目标的固定阻塞报告。
    pub fn activation_report_for(
        &self,
        target: SmithayLinuxAdapterActivationTarget,
    ) -> SmithayLinuxAdapterActivationReport {
        SmithayLinuxAdapterActivationReport {
            target,
            decision: SmithayLinuxAdapterActivationDecision::Blocked,
            blockers: activation_blockers_for(target),
            skeleton_only: true,
        }
    }

    /// 返回全部真实能力目标的固定激活 gate 报告。
    pub fn activation_gate_report(&self) -> SmithayLinuxAdapterActivationGateReport {
        let reports = ACTIVATION_TARGETS
            .into_iter()
            .map(|target| self.activation_report_for(target))
            .collect::<Vec<_>>();

        SmithayLinuxAdapterActivationGateReport {
            blocked_count: reports.len(),
            allowed_count: 0,
            skeleton_only: true,
            reports,
        }
    }

    /// 判断指定真实能力目标是否允许激活；当前阶段恒为 `false`。
    pub fn can_activate(&self, target: SmithayLinuxAdapterActivationTarget) -> bool {
        !matches!(
            self.activation_report_for(target).decision,
            SmithayLinuxAdapterActivationDecision::Blocked
        )
    }

    /// 通过 activation gate 记录一次被阻止的真实能力激活 attempt。
    pub fn attempt_activate(
        &mut self,
        target: SmithayLinuxAdapterActivationTarget,
    ) -> SmithayLinuxAdapterActivationAttemptObservation {
        let activation_report = self.activation_report_for(target);
        let observation = SmithayLinuxAdapterActivationAttemptObservation {
            sequence: self.next_activation_attempt_sequence,
            target,
            decision: activation_report.decision,
            outcome: SmithayLinuxAdapterActivationAttemptOutcome::Blocked {
                blockers: activation_report.blockers,
            },
            skeleton_only: true,
        };

        self.next_activation_attempt_sequence =
            self.next_activation_attempt_sequence.saturating_add(1);
        self.activation_attempt_observations
            .push(observation.clone());

        observation
    }

    /// 返回当前 activation attempt ledger 的纯数据报告。
    pub fn activation_attempt_ledger_report(
        &self,
    ) -> SmithayLinuxAdapterActivationAttemptLedgerReport {
        SmithayLinuxAdapterActivationAttemptLedgerReport {
            observed_count: self.activation_attempt_observations.len(),
            blocked_count: self.activation_attempt_observations.len(),
            allowed_count: 0,
            observations: self.activation_attempt_observations.clone(),
            skeleton_only: true,
        }
    }

    /// 通过 activation gate 执行一次真实 global registration 可行性检查。
    ///
    /// Phase 49A fallback 只记录 blocked attempt 和纯数据报告，不访问 Display
    /// handle，也不执行任何真实 protocol global 注册。
    pub fn attempt_real_global_registration_feasibility(
        &mut self,
    ) -> SmithayLinuxAdapterRealGlobalRegistrationReport {
        let handler_boundary = self.global_handler_boundary_report();
        let activation_attempt = self
            .attempt_activate(SmithayLinuxAdapterActivationTarget::RealProtocolGlobalRegistration);
        let SmithayLinuxAdapterActivationAttemptOutcome::Blocked {
            blockers: activation_blockers,
        } = activation_attempt.outcome;
        let report = SmithayLinuxAdapterRealGlobalRegistrationReport {
            mode: SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked,
            attempted_kinds: Vec::new(),
            succeeded_kinds: Vec::new(),
            blocked_kinds: handler_boundary
                .reports
                .iter()
                .map(|report| global_kind_for_handler(report.kind))
                .collect(),
            activation_blockers,
            blockers: real_global_registration_blockers(),
            real_registered_count: 0,
            skeleton_only: true,
        };
        self.real_global_registration_report = Some(report.clone());

        report
    }

    /// 返回 adapter 当前状态的纯数据只读快照。
    pub fn snapshot(&self) -> SmithayLinuxAdapterSnapshot {
        SmithayLinuxAdapterSnapshot {
            lifecycle: self.lifecycle,
            pump_state: self.pump_state,
            capabilities: self.capabilities(),
            activation_gate: self.activation_gate_report(),
            activation_attempt_ledger: self.activation_attempt_ledger_report(),
            client_session_ledger: self.client_session_ledger_report(),
            global_plan: self.global_plan_report(),
            global_registration_report: self.global_registration_report(),
            real_global_registration_report: self.real_global_registration_report(),
            global_handler_boundary: self.global_handler_boundary_report(),
            protocol_request_ledger: self.protocol_request_ledger_report(),
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

fn unsupported_request_reason(
    kind: SmithayLinuxAdapterProtocolRequestKind,
) -> SmithayLinuxAdapterUnsupportedRequestReason {
    match kind {
        SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface
        | SmithayLinuxAdapterProtocolRequestKind::XdgWmBaseGetXdgSurface => {
            SmithayLinuxAdapterUnsupportedRequestReason::RealSurfaceUnsupported
        }
        SmithayLinuxAdapterProtocolRequestKind::CompositorCreateRegion
        | SmithayLinuxAdapterProtocolRequestKind::ShmCreatePool
        | SmithayLinuxAdapterProtocolRequestKind::XdgWmBaseCreatePositioner
        | SmithayLinuxAdapterProtocolRequestKind::XdgWmBasePong => {
            SmithayLinuxAdapterUnsupportedRequestReason::ProtocolDispatchUnsupported
        }
    }
}

fn activation_blockers_for(
    target: SmithayLinuxAdapterActivationTarget,
) -> Vec<SmithayLinuxAdapterActivationBlocker> {
    use SmithayLinuxAdapterActivationBlocker as Blocker;

    match target {
        SmithayLinuxAdapterActivationTarget::RealEventLoop => {
            vec![Blocker::SkeletonOnly, Blocker::EventLoopUnsupported]
        }
        SmithayLinuxAdapterActivationTarget::RealClientAccept => {
            vec![Blocker::SkeletonOnly, Blocker::ClientAcceptUnsupported]
        }
        SmithayLinuxAdapterActivationTarget::RealProtocolGlobalRegistration => vec![
            Blocker::SkeletonOnly,
            Blocker::ProtocolGlobalRegistrationUnsupported,
            Blocker::MissingProtocolGlobalImplementation,
        ],
        SmithayLinuxAdapterActivationTarget::RealProtocolDispatch => vec![
            Blocker::SkeletonOnly,
            Blocker::ProtocolDispatchUnsupported,
            Blocker::MissingDispatchImplementation,
        ],
        SmithayLinuxAdapterActivationTarget::RealSurfaceLifecycle => vec![
            Blocker::SkeletonOnly,
            Blocker::RealSurfaceUnsupported,
            Blocker::MissingSurfaceLifecycleImplementation,
        ],
        SmithayLinuxAdapterActivationTarget::RealXdgToplevelLifecycle => vec![
            Blocker::SkeletonOnly,
            Blocker::XdgToplevelUnsupported,
            Blocker::MissingSurfaceLifecycleImplementation,
        ],
        SmithayLinuxAdapterActivationTarget::CoreAdmission => vec![
            Blocker::SkeletonOnly,
            Blocker::CoreAdmissionUnsupported,
            Blocker::MissingCoreIntegration,
        ],
        SmithayLinuxAdapterActivationTarget::GpuRendering => {
            vec![Blocker::SkeletonOnly, Blocker::GpuRenderingUnsupported]
        }
    }
}

fn real_global_registration_blockers() -> Vec<SmithayLinuxAdapterRealGlobalRegistrationBlocker> {
    use SmithayLinuxAdapterRealGlobalRegistrationBlocker as Blocker;

    vec![
        Blocker::ActivationGateBlocked,
        Blocker::GlobalBindHandlerUnavailable,
        Blocker::ProtocolRequestHandlerUnavailable,
        Blocker::SurfaceRequestsUnsupported,
        Blocker::ClientHandlingUnsupported,
        Blocker::ProtocolDispatchUnsupported,
        Blocker::CoreIntegrationUnsupported,
    ]
}

fn global_handler_readiness_report(
    kind: SmithayLinuxAdapterGlobalHandlerKind,
) -> SmithayLinuxAdapterGlobalHandlerReadinessReport {
    use SmithayLinuxAdapterGlobalHandlerBlocker as Blocker;

    let blockers = match kind {
        SmithayLinuxAdapterGlobalHandlerKind::CompositorGlobalHandler => vec![
            Blocker::MissingGlobalDispatchImplementation,
            Blocker::MissingDispatchImplementation,
            Blocker::MissingCompositorHandler,
            Blocker::SurfaceRequestsUnsupported,
            Blocker::ClientHandlingUnsupported,
            Blocker::ProtocolDispatchUnsupported,
            Blocker::CoreIntegrationUnsupported,
            Blocker::ActivationGateBlocked,
        ],
        SmithayLinuxAdapterGlobalHandlerKind::ShmGlobalHandler => vec![
            Blocker::MissingGlobalDispatchImplementation,
            Blocker::MissingDispatchImplementation,
            Blocker::MissingBufferHandler,
            Blocker::MissingShmHandler,
            Blocker::ClientHandlingUnsupported,
            Blocker::ProtocolDispatchUnsupported,
            Blocker::CoreIntegrationUnsupported,
            Blocker::ActivationGateBlocked,
        ],
        SmithayLinuxAdapterGlobalHandlerKind::XdgWmBaseGlobalHandler => vec![
            Blocker::MissingGlobalDispatchImplementation,
            Blocker::MissingDispatchImplementation,
            Blocker::MissingXdgShellHandler,
            Blocker::SurfaceRequestsUnsupported,
            Blocker::ClientHandlingUnsupported,
            Blocker::ProtocolDispatchUnsupported,
            Blocker::CoreIntegrationUnsupported,
            Blocker::ActivationGateBlocked,
        ],
    };

    SmithayLinuxAdapterGlobalHandlerReadinessReport {
        kind,
        readiness: SmithayLinuxAdapterGlobalHandlerReadiness::FeasibilityBlocked,
        blockers,
        skeleton_only: true,
    }
}

fn global_kind_for_handler(
    kind: SmithayLinuxAdapterGlobalHandlerKind,
) -> SmithayLinuxAdapterGlobalKind {
    match kind {
        SmithayLinuxAdapterGlobalHandlerKind::CompositorGlobalHandler => {
            SmithayLinuxAdapterGlobalKind::Compositor
        }
        SmithayLinuxAdapterGlobalHandlerKind::ShmGlobalHandler => {
            SmithayLinuxAdapterGlobalKind::Shm
        }
        SmithayLinuxAdapterGlobalHandlerKind::XdgWmBaseGlobalHandler => {
            SmithayLinuxAdapterGlobalKind::XdgWmBase
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
        SmithayLinuxAdapterActivationAttemptLedgerReport,
        SmithayLinuxAdapterActivationAttemptOutcome, SmithayLinuxAdapterActivationBlocker,
        SmithayLinuxAdapterActivationDecision, SmithayLinuxAdapterActivationTarget,
        SmithayLinuxAdapterClientSessionLedgerReport, SmithayLinuxAdapterClientSessionOutcome,
        SmithayLinuxAdapterClientSessionState, SmithayLinuxAdapterClientUnsupportedReason,
        SmithayLinuxAdapterDiagnostic, SmithayLinuxAdapterError,
        SmithayLinuxAdapterGlobalHandlerBlocker, SmithayLinuxAdapterGlobalHandlerKind,
        SmithayLinuxAdapterGlobalHandlerReadiness, SmithayLinuxAdapterGlobalKind,
        SmithayLinuxAdapterGlobalPlan, SmithayLinuxAdapterGlobalRegistrationOperation,
        SmithayLinuxAdapterGlobalRegistrationState, SmithayLinuxAdapterLifecycle,
        SmithayLinuxAdapterOperation, SmithayLinuxAdapterProtocolRequestKind,
        SmithayLinuxAdapterProtocolRequestLedgerReport, SmithayLinuxAdapterProtocolRequestOutcome,
        SmithayLinuxAdapterPumpOperation, SmithayLinuxAdapterPumpState,
        SmithayLinuxAdapterPumpStats, SmithayLinuxAdapterRealGlobalRegistrationBlocker,
        SmithayLinuxAdapterRealGlobalRegistrationMode, SmithayLinuxAdapterSkeleton,
        SmithayLinuxAdapterUnsupportedRequestReason,
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
        assert_eq!(adapter.real_global_registration_report(), None);
        assert_eq!(
            adapter.activation_attempt_ledger_report(),
            SmithayLinuxAdapterActivationAttemptLedgerReport {
                observed_count: 0,
                blocked_count: 0,
                allowed_count: 0,
                observations: Vec::new(),
                skeleton_only: true,
            }
        );
        assert_eq!(
            adapter.client_session_ledger_report(),
            SmithayLinuxAdapterClientSessionLedgerReport {
                observed_count: 0,
                rejected_unsupported_count: 0,
                observations: Vec::new(),
                skeleton_only: true,
            }
        );
        assert_eq!(
            adapter.protocol_request_ledger_report(),
            SmithayLinuxAdapterProtocolRequestLedgerReport {
                observed_count: 0,
                rejected_unsupported_count: 0,
                observations: Vec::new(),
                skeleton_only: true,
            }
        );
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
            SmithayLinuxAdapterDiagnostic::ActivationGatePresent,
            SmithayLinuxAdapterDiagnostic::ActivationAttemptLedgerPresent,
            SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationFeasibilityPresent,
            SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationNotEnabled,
            SmithayLinuxAdapterDiagnostic::GlobalHandlerBoundaryPresent,
            SmithayLinuxAdapterDiagnostic::GlobalHandlersFeasibilityBlocked,
            SmithayLinuxAdapterDiagnostic::GlobalDispatchImplementationMissing,
            SmithayLinuxAdapterDiagnostic::DispatchImplementationMissing,
            SmithayLinuxAdapterDiagnostic::AllRealCapabilitiesBlocked,
            SmithayLinuxAdapterDiagnostic::RealEventLoopBlocked,
            SmithayLinuxAdapterDiagnostic::RealClientAcceptBlocked,
            SmithayLinuxAdapterDiagnostic::RealProtocolDispatchBlocked,
            SmithayLinuxAdapterDiagnostic::RealSurfaceLifecycleBlocked,
            SmithayLinuxAdapterDiagnostic::CoreAdmissionBlocked,
            SmithayLinuxAdapterDiagnostic::GpuRenderingBlocked,
            SmithayLinuxAdapterDiagnostic::EventLoopNotRunning,
            SmithayLinuxAdapterDiagnostic::ClientsNotAccepted,
            SmithayLinuxAdapterDiagnostic::ClientSessionLedgerPresent,
            SmithayLinuxAdapterDiagnostic::ProtocolEventsNotDispatched,
            SmithayLinuxAdapterDiagnostic::InertProtocolRequestLedgerPresent,
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
        assert_eq!(first.activation_gate.reports.len(), 8);
        assert_eq!(first.activation_gate.blocked_count, 8);
        assert_eq!(first.activation_gate.allowed_count, 0);
        assert!(first.activation_gate.skeleton_only);
        assert_eq!(first.activation_attempt_ledger.observed_count, 0);
        assert_eq!(first.activation_attempt_ledger.blocked_count, 0);
        assert_eq!(first.activation_attempt_ledger.allowed_count, 0);
        assert!(first.activation_attempt_ledger.observations.is_empty());
        assert!(first.activation_attempt_ledger.skeleton_only);
        assert_eq!(first.global_plan.planned_count, 3);
        assert_eq!(first.global_plan.registered_count, 0);
        assert!(first.global_plan.skeleton_only);
        assert_eq!(first.global_registration_report, None);
        assert_eq!(first.real_global_registration_report, None);
        assert_eq!(first.global_handler_boundary.ready_count, 0);
        assert_eq!(first.global_handler_boundary.blocked_count, 3);
        assert_eq!(first.global_handler_boundary.reports.len(), 3);
        assert!(first.global_handler_boundary.skeleton_only);
        assert_eq!(first.client_session_ledger.observed_count, 0);
        assert_eq!(first.client_session_ledger.rejected_unsupported_count, 0);
        assert!(first.client_session_ledger.observations.is_empty());
        assert!(first.client_session_ledger.skeleton_only);
        assert_eq!(first.protocol_request_ledger.observed_count, 0);
        assert_eq!(first.protocol_request_ledger.rejected_unsupported_count, 0);
        assert!(first.protocol_request_ledger.observations.is_empty());
        assert!(first.protocol_request_ledger.skeleton_only);
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
    fn unsupported_protocol_request_reason_mapping_is_fixed() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-request-reasons");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        let expected = [
            (
                SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
                SmithayLinuxAdapterUnsupportedRequestReason::RealSurfaceUnsupported,
            ),
            (
                SmithayLinuxAdapterProtocolRequestKind::CompositorCreateRegion,
                SmithayLinuxAdapterUnsupportedRequestReason::ProtocolDispatchUnsupported,
            ),
            (
                SmithayLinuxAdapterProtocolRequestKind::ShmCreatePool,
                SmithayLinuxAdapterUnsupportedRequestReason::ProtocolDispatchUnsupported,
            ),
            (
                SmithayLinuxAdapterProtocolRequestKind::XdgWmBaseCreatePositioner,
                SmithayLinuxAdapterUnsupportedRequestReason::ProtocolDispatchUnsupported,
            ),
            (
                SmithayLinuxAdapterProtocolRequestKind::XdgWmBaseGetXdgSurface,
                SmithayLinuxAdapterUnsupportedRequestReason::RealSurfaceUnsupported,
            ),
            (
                SmithayLinuxAdapterProtocolRequestKind::XdgWmBasePong,
                SmithayLinuxAdapterUnsupportedRequestReason::ProtocolDispatchUnsupported,
            ),
        ];

        for (index, (kind, reason)) in expected.into_iter().enumerate() {
            let observation = adapter.observe_unsupported_protocol_request(kind);

            assert_eq!(observation.sequence, index as u64 + 1);
            assert_eq!(observation.kind, kind);
            assert_eq!(
                observation.outcome,
                SmithayLinuxAdapterProtocolRequestOutcome::RejectedUnsupported { reason }
            );
            assert!(observation.skeleton_only);
        }

        let report = adapter.protocol_request_ledger_report();
        assert_eq!(report.observed_count, expected.len());
        assert_eq!(report.rejected_unsupported_count, expected.len());
        assert!(report.skeleton_only);
        assert!(report.observations.iter().all(|observation| matches!(
            observation.outcome,
            SmithayLinuxAdapterProtocolRequestOutcome::RejectedUnsupported { .. }
        )));
    }

    #[test]
    fn unsupported_protocol_request_ledger_preserves_adapter_state() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-request-ledger-state");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("registration skeleton 必须能够建立");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let registration_report = adapter.global_registration_report();
        let global_plan = adapter.global_plan_report();

        let first = adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );
        let second = adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::ShmCreatePool,
        );

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
        assert_eq!(adapter.global_registration_report(), registration_report);
        assert_eq!(adapter.global_plan_report(), global_plan);

        let report = adapter.protocol_request_ledger_report();
        assert_eq!(report.observations, vec![first, second]);
        assert_eq!(report.observed_count, 2);
        assert_eq!(report.rejected_unsupported_count, 2);

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.protocol_request_ledger, report);
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ProtocolRequestsRejectedUnsupported)
        );
    }

    #[test]
    fn unsupported_client_session_ledger_is_ordered_and_preserves_adapter_state() {
        assert_runtime_dir();

        let socket_name = unique_socket_name("adapter-client-session-ledger");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("registration skeleton 必须能够建立");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let registration_report = adapter.global_registration_report();
        let global_plan = adapter.global_plan_report();
        let protocol_request_ledger = adapter.protocol_request_ledger_report();

        let first = adapter.observe_unsupported_client_session();
        let second = adapter.observe_unsupported_client_session();

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(first.session_id.value(), 1);
        assert_eq!(second.session_id.value(), 2);
        for observation in [first, second] {
            assert_eq!(
                observation.state,
                SmithayLinuxAdapterClientSessionState::RejectedUnsupported
            );
            assert_eq!(
                observation.outcome,
                SmithayLinuxAdapterClientSessionOutcome::RejectedUnsupported {
                    reason: SmithayLinuxAdapterClientUnsupportedReason::ClientAcceptUnsupported,
                }
            );
            assert!(observation.skeleton_only);
        }

        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
        assert_eq!(adapter.global_registration_report(), registration_report);
        assert_eq!(adapter.global_plan_report(), global_plan);
        assert_eq!(
            adapter.protocol_request_ledger_report(),
            protocol_request_ledger
        );

        let report = adapter.client_session_ledger_report();
        assert_eq!(report.observations, vec![first, second]);
        assert_eq!(report.observed_count, 2);
        assert_eq!(report.rejected_unsupported_count, 2);
        assert!(report.skeleton_only);
        assert!(report.observations.iter().all(|observation| matches!(
            observation.outcome,
            SmithayLinuxAdapterClientSessionOutcome::RejectedUnsupported {
                reason: SmithayLinuxAdapterClientUnsupportedReason::ClientAcceptUnsupported,
            }
        )));

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.client_session_ledger, report);
        assert_eq!(snapshot.protocol_request_ledger, protocol_request_ledger);
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ClientSessionLedgerPresent)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ClientSessionsRejectedUnsupported)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ClientAcceptUnsupported)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ProtocolRequestsRejectedUnsupported)
        );
    }

    #[test]
    fn activation_gate_blocks_every_target_with_fixed_reasons_without_mutation() {
        assert_runtime_dir();

        use SmithayLinuxAdapterActivationBlocker as Blocker;
        use SmithayLinuxAdapterActivationTarget as Target;

        let socket_name = unique_socket_name("adapter-activation-gate");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("registration skeleton 必须能够建立");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );
        adapter.observe_unsupported_client_session();

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let registration_report = adapter.global_registration_report();
        let global_plan = adapter.global_plan_report();
        let protocol_request_ledger = adapter.protocol_request_ledger_report();
        let client_session_ledger = adapter.client_session_ledger_report();
        let expected = [
            (
                Target::RealEventLoop,
                vec![Blocker::SkeletonOnly, Blocker::EventLoopUnsupported],
            ),
            (
                Target::RealClientAccept,
                vec![Blocker::SkeletonOnly, Blocker::ClientAcceptUnsupported],
            ),
            (
                Target::RealProtocolGlobalRegistration,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::ProtocolGlobalRegistrationUnsupported,
                    Blocker::MissingProtocolGlobalImplementation,
                ],
            ),
            (
                Target::RealProtocolDispatch,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::ProtocolDispatchUnsupported,
                    Blocker::MissingDispatchImplementation,
                ],
            ),
            (
                Target::RealSurfaceLifecycle,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::RealSurfaceUnsupported,
                    Blocker::MissingSurfaceLifecycleImplementation,
                ],
            ),
            (
                Target::RealXdgToplevelLifecycle,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::XdgToplevelUnsupported,
                    Blocker::MissingSurfaceLifecycleImplementation,
                ],
            ),
            (
                Target::CoreAdmission,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::CoreAdmissionUnsupported,
                    Blocker::MissingCoreIntegration,
                ],
            ),
            (
                Target::GpuRendering,
                vec![Blocker::SkeletonOnly, Blocker::GpuRenderingUnsupported],
            ),
        ];

        for (target, blockers) in &expected {
            let report = adapter.activation_report_for(*target);

            assert_eq!(report.target, *target);
            assert_eq!(
                report.decision,
                SmithayLinuxAdapterActivationDecision::Blocked
            );
            assert_eq!(report.blockers.as_slice(), blockers.as_slice());
            assert!(!report.blockers.is_empty());
            assert!(report.skeleton_only);
            assert!(!adapter.can_activate(*target));
        }

        let gate = adapter.activation_gate_report();
        assert_eq!(
            gate.reports
                .iter()
                .map(|report| report.target)
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(target, _)| *target)
                .collect::<Vec<_>>()
        );
        assert_eq!(gate.blocked_count, expected.len());
        assert_eq!(gate.allowed_count, 0);
        assert!(gate.skeleton_only);
        assert!(gate.reports.iter().all(|report| {
            report.decision == SmithayLinuxAdapterActivationDecision::Blocked
                && !report.blockers.is_empty()
                && report.skeleton_only
        }));

        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
        assert_eq!(adapter.global_registration_report(), registration_report);
        assert_eq!(adapter.global_plan_report(), global_plan);
        assert_eq!(
            adapter.protocol_request_ledger_report(),
            protocol_request_ledger
        );
        assert_eq!(
            adapter.client_session_ledger_report(),
            client_session_ledger
        );

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.activation_gate, gate);
        assert_eq!(snapshot.protocol_request_ledger, protocol_request_ledger);
        assert_eq!(snapshot.client_session_ledger, client_session_ledger);
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ActivationGatePresent)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::AllRealCapabilitiesBlocked)
        );
    }

    #[test]
    fn activation_attempt_ledger_records_gate_blockers_without_state_transition() {
        assert_runtime_dir();

        use SmithayLinuxAdapterActivationBlocker as Blocker;
        use SmithayLinuxAdapterActivationTarget as Target;

        let socket_name = unique_socket_name("adapter-activation-attempt-ledger");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("registration skeleton 必须能够建立");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );
        adapter.observe_unsupported_client_session();

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let registration_report = adapter.global_registration_report();
        let global_plan = adapter.global_plan_report();
        let protocol_request_ledger = adapter.protocol_request_ledger_report();
        let client_session_ledger = adapter.client_session_ledger_report();
        let activation_gate = adapter.activation_gate_report();
        let expected = [
            (
                Target::RealEventLoop,
                vec![Blocker::SkeletonOnly, Blocker::EventLoopUnsupported],
            ),
            (
                Target::RealClientAccept,
                vec![Blocker::SkeletonOnly, Blocker::ClientAcceptUnsupported],
            ),
            (
                Target::RealProtocolGlobalRegistration,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::ProtocolGlobalRegistrationUnsupported,
                    Blocker::MissingProtocolGlobalImplementation,
                ],
            ),
            (
                Target::RealProtocolDispatch,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::ProtocolDispatchUnsupported,
                    Blocker::MissingDispatchImplementation,
                ],
            ),
            (
                Target::RealSurfaceLifecycle,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::RealSurfaceUnsupported,
                    Blocker::MissingSurfaceLifecycleImplementation,
                ],
            ),
            (
                Target::RealXdgToplevelLifecycle,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::XdgToplevelUnsupported,
                    Blocker::MissingSurfaceLifecycleImplementation,
                ],
            ),
            (
                Target::CoreAdmission,
                vec![
                    Blocker::SkeletonOnly,
                    Blocker::CoreAdmissionUnsupported,
                    Blocker::MissingCoreIntegration,
                ],
            ),
            (
                Target::GpuRendering,
                vec![Blocker::SkeletonOnly, Blocker::GpuRenderingUnsupported],
            ),
        ];
        let mut observations = Vec::new();

        for (index, (target, blockers)) in expected.iter().enumerate() {
            let gate_report = adapter.activation_report_for(*target);
            let observation = adapter.attempt_activate(*target);

            assert_eq!(observation.sequence, index as u64 + 1);
            assert_eq!(observation.target, *target);
            assert_eq!(
                observation.decision,
                SmithayLinuxAdapterActivationDecision::Blocked
            );
            assert_eq!(gate_report.blockers.as_slice(), blockers.as_slice());
            assert_eq!(
                observation.outcome,
                SmithayLinuxAdapterActivationAttemptOutcome::Blocked {
                    blockers: gate_report.blockers,
                }
            );
            assert!(matches!(
                &observation.outcome,
                SmithayLinuxAdapterActivationAttemptOutcome::Blocked { blockers }
                    if !blockers.is_empty()
            ));
            assert!(observation.skeleton_only);
            assert!(!adapter.can_activate(*target));
            observations.push(observation);
        }

        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
        assert_eq!(adapter.global_registration_report(), registration_report);
        assert_eq!(adapter.global_plan_report(), global_plan);
        assert_eq!(
            adapter.protocol_request_ledger_report(),
            protocol_request_ledger
        );
        assert_eq!(
            adapter.client_session_ledger_report(),
            client_session_ledger
        );
        assert_eq!(adapter.activation_gate_report(), activation_gate);

        let ledger = adapter.activation_attempt_ledger_report();
        assert_eq!(ledger.observed_count, expected.len());
        assert_eq!(ledger.blocked_count, expected.len());
        assert_eq!(ledger.allowed_count, 0);
        assert_eq!(ledger.observations, observations);
        assert!(ledger.skeleton_only);
        assert!(ledger.observations.iter().all(|observation| {
            observation.decision == SmithayLinuxAdapterActivationDecision::Blocked
                && matches!(
                    &observation.outcome,
                    SmithayLinuxAdapterActivationAttemptOutcome::Blocked { blockers }
                        if !blockers.is_empty()
                )
        }));

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.activation_attempt_ledger, ledger);
        assert_eq!(snapshot.activation_gate, activation_gate);
        assert_eq!(snapshot.protocol_request_ledger, protocol_request_ledger);
        assert_eq!(snapshot.client_session_ledger, client_session_ledger);
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ActivationAttemptLedgerPresent)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::ActivationAttemptsBlocked)
        );
    }

    #[test]
    fn guarded_real_global_registration_feasibility_is_blocked_without_side_effects() {
        assert_runtime_dir();

        use SmithayLinuxAdapterActivationBlocker as ActivationBlocker;
        use SmithayLinuxAdapterGlobalKind as GlobalKind;
        use SmithayLinuxAdapterRealGlobalRegistrationBlocker as FeasibilityBlocker;

        let socket_name = unique_socket_name("adapter-real-global-feasibility");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("registration skeleton 必须能够建立");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );
        adapter.observe_unsupported_client_session();

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let registration_report = adapter.global_registration_report();
        let global_plan = adapter.global_plan_report();
        let protocol_request_ledger = adapter.protocol_request_ledger_report();
        let client_session_ledger = adapter.client_session_ledger_report();
        let gate_report = adapter.activation_report_for(
            SmithayLinuxAdapterActivationTarget::RealProtocolGlobalRegistration,
        );
        let ledger_before = adapter.activation_attempt_ledger_report();

        let report = adapter.attempt_real_global_registration_feasibility();

        assert_eq!(
            report.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        assert!(report.attempted_kinds.is_empty());
        assert!(report.succeeded_kinds.is_empty());
        assert_eq!(
            report.blocked_kinds,
            vec![
                GlobalKind::Compositor,
                GlobalKind::Shm,
                GlobalKind::XdgWmBase,
            ]
        );
        assert_eq!(report.activation_blockers, gate_report.blockers);
        assert_eq!(
            report.activation_blockers,
            vec![
                ActivationBlocker::SkeletonOnly,
                ActivationBlocker::ProtocolGlobalRegistrationUnsupported,
                ActivationBlocker::MissingProtocolGlobalImplementation,
            ]
        );
        assert_eq!(
            report.blockers,
            vec![
                FeasibilityBlocker::ActivationGateBlocked,
                FeasibilityBlocker::GlobalBindHandlerUnavailable,
                FeasibilityBlocker::ProtocolRequestHandlerUnavailable,
                FeasibilityBlocker::SurfaceRequestsUnsupported,
                FeasibilityBlocker::ClientHandlingUnsupported,
                FeasibilityBlocker::ProtocolDispatchUnsupported,
                FeasibilityBlocker::CoreIntegrationUnsupported,
            ]
        );
        assert!(!report.blockers.is_empty());
        assert_eq!(report.real_registered_count, 0);
        assert!(report.skeleton_only);
        assert_eq!(
            adapter.real_global_registration_report(),
            Some(report.clone())
        );
        assert!(
            !adapter
                .can_activate(SmithayLinuxAdapterActivationTarget::RealProtocolGlobalRegistration)
        );

        let ledger = adapter.activation_attempt_ledger_report();
        assert_eq!(ledger.observed_count, ledger_before.observed_count + 1);
        assert_eq!(ledger.blocked_count, ledger_before.blocked_count + 1);
        assert_eq!(ledger.allowed_count, 0);
        let observation = ledger
            .observations
            .last()
            .expect("feasibility attempt 必须写入 activation ledger");
        assert_eq!(
            observation.target,
            SmithayLinuxAdapterActivationTarget::RealProtocolGlobalRegistration
        );
        assert_eq!(
            observation.decision,
            SmithayLinuxAdapterActivationDecision::Blocked
        );
        assert_eq!(
            observation.outcome,
            SmithayLinuxAdapterActivationAttemptOutcome::Blocked {
                blockers: report.activation_blockers.clone(),
            }
        );

        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
        assert_eq!(adapter.global_registration_report(), registration_report);
        assert_eq!(adapter.global_plan_report(), global_plan);
        assert_eq!(
            adapter.protocol_request_ledger_report(),
            protocol_request_ledger
        );
        assert_eq!(
            adapter.client_session_ledger_report(),
            client_session_ledger
        );

        let snapshot = adapter.snapshot();
        assert_eq!(
            snapshot.real_global_registration_report,
            Some(report.clone())
        );
        assert_eq!(snapshot.activation_attempt_ledger, ledger);
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationFeasibilityPresent)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationBlocked)
        );
        assert!(
            snapshot
                .diagnostics
                .contains(&SmithayLinuxAdapterDiagnostic::RealGlobalRegistrationNotEnabled)
        );

        let runtime_report = BackendRuntimeReport::from(&adapter);
        assert_eq!(
            runtime_report.bootstrap_mode,
            BackendBootstrapMode::ProbeOnly
        );
        assert!(!runtime_report.capabilities.supports_real_wayland_surfaces);
        assert!(!runtime_report.capabilities.supports_gpu_rendering);
        assert!(runtime_report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterEventPumpSkeleton {
                accepts_clients: false,
                dispatches_protocol_events: false,
                registers_protocol_globals: false,
                ..
            }
        )));
        assert!(runtime_report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterRealGlobalRegistrationFeasibility {
                attempted: true,
                blocked_count: 3,
                real_registered_count: 0,
                registration_enabled: false,
                accepts_clients: false,
                dispatches_protocol_events: false,
                skeleton_only: true,
            }
        )));
    }

    #[test]
    fn global_handler_boundary_is_stable_blocked_and_read_only() {
        assert_runtime_dir();

        use SmithayLinuxAdapterGlobalHandlerBlocker as Blocker;
        use SmithayLinuxAdapterGlobalHandlerKind as Kind;

        let socket_name = unique_socket_name("adapter-global-handler-boundary");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("adapter skeleton 必须能够构造");
        adapter
            .register_planned_globals_skeleton()
            .expect("registration skeleton 必须能够建立");
        adapter.start_pump().expect("NotStarted 必须允许启动 pump");
        let last_result = adapter
            .pump_once()
            .expect("Ready 必须允许一次 skeleton tick");
        adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );
        adapter.observe_unsupported_client_session();
        let feasibility = adapter.attempt_real_global_registration_feasibility();

        let lifecycle = adapter.lifecycle();
        let pump_state = adapter.pump_state();
        let pump_stats = adapter.pump_stats();
        let activation_attempt_ledger = adapter.activation_attempt_ledger_report();
        let real_global_registration_report = adapter.real_global_registration_report();
        let protocol_request_ledger = adapter.protocol_request_ledger_report();
        let client_session_ledger = adapter.client_session_ledger_report();

        let first = adapter.global_handler_boundary_report();
        let second = adapter.global_handler_boundary_report();

        assert_eq!(first, second);
        assert_eq!(first.ready_count, 0);
        assert_eq!(first.blocked_count, 3);
        assert!(first.skeleton_only);
        assert_eq!(
            first
                .reports
                .iter()
                .map(|report| report.kind)
                .collect::<Vec<_>>(),
            vec![
                Kind::CompositorGlobalHandler,
                Kind::ShmGlobalHandler,
                Kind::XdgWmBaseGlobalHandler,
            ]
        );
        assert!(first.reports.iter().all(|report| {
            report.readiness == SmithayLinuxAdapterGlobalHandlerReadiness::FeasibilityBlocked
                && !report.blockers.is_empty()
                && report.skeleton_only
        }));

        let compositor = &first.reports[0];
        assert!(
            compositor
                .blockers
                .contains(&Blocker::MissingGlobalDispatchImplementation)
        );
        assert!(
            compositor
                .blockers
                .contains(&Blocker::MissingCompositorHandler)
        );

        let shm = &first.reports[1];
        for blocker in [
            Blocker::MissingGlobalDispatchImplementation,
            Blocker::MissingDispatchImplementation,
            Blocker::MissingBufferHandler,
            Blocker::MissingShmHandler,
        ] {
            assert!(shm.blockers.contains(&blocker));
        }

        let xdg = &first.reports[2];
        for blocker in [
            Blocker::MissingGlobalDispatchImplementation,
            Blocker::MissingDispatchImplementation,
            Blocker::MissingXdgShellHandler,
        ] {
            assert!(xdg.blockers.contains(&blocker));
        }

        assert_eq!(adapter.lifecycle(), lifecycle);
        assert_eq!(adapter.pump_state(), pump_state);
        assert_eq!(adapter.pump_stats(), pump_stats);
        assert_eq!(adapter.last_pump_result(), Some(last_result));
        assert_eq!(
            adapter.activation_attempt_ledger_report(),
            activation_attempt_ledger
        );
        assert_eq!(
            adapter.real_global_registration_report(),
            real_global_registration_report
        );
        assert_eq!(
            adapter.protocol_request_ledger_report(),
            protocol_request_ledger
        );
        assert_eq!(
            adapter.client_session_ledger_report(),
            client_session_ledger
        );
        assert_eq!(
            feasibility.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        assert!(feasibility.succeeded_kinds.is_empty());
        assert_eq!(feasibility.real_registered_count, 0);

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.global_handler_boundary, first);
        for diagnostic in [
            SmithayLinuxAdapterDiagnostic::GlobalHandlerBoundaryPresent,
            SmithayLinuxAdapterDiagnostic::GlobalHandlersFeasibilityBlocked,
            SmithayLinuxAdapterDiagnostic::GlobalDispatchImplementationMissing,
            SmithayLinuxAdapterDiagnostic::DispatchImplementationMissing,
        ] {
            assert!(snapshot.diagnostics.contains(&diagnostic));
        }

        let capabilities = adapter.capabilities();
        assert!(capabilities.has_global_handler_boundary);
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);

        let runtime_report = BackendRuntimeReport::from(&adapter);
        assert_eq!(
            runtime_report.bootstrap_mode,
            BackendBootstrapMode::ProbeOnly
        );
        assert!(runtime_report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterGlobalHandlerBoundaryPresent {
                report_count: 3,
                ready_count: 0,
                blocked_count: 3,
                registers_protocol_globals: false,
                dispatches_protocol_events: false,
                accepts_clients: false,
                skeleton_only: true,
            }
        )));
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
        assert!(capabilities.has_activation_gate);
        assert!(capabilities.has_activation_attempt_ledger);
        assert!(capabilities.has_real_global_registration_feasibility);
        assert!(capabilities.has_global_handler_boundary);
        assert!(capabilities.has_event_pump_boundary);
        assert!(capabilities.pumps_once);
        assert!(!capabilities.runs_event_loop);
        assert!(!capabilities.accepts_clients);
        assert!(capabilities.has_client_session_ledger);
        assert!(capabilities.has_protocol_global_plan_boundary);
        assert!(capabilities.has_protocol_global_registration_boundary);
        assert!(capabilities.has_inert_protocol_request_ledger);
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
            BackendRuntimeDiagnostic::AdapterInertProtocolRequestLedger {
                observed_count: 0,
                rejected_unsupported_count: 0,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterClientSessionLedger {
                observed_count: 0,
                rejected_unsupported_count: 0,
                accepts_clients: false,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterActivationGatePresent {
                report_count: 8,
                blocked_count: 8,
                allowed_count: 0,
                accepts_clients: false,
                registers_protocol_globals: false,
                dispatches_protocol_events: false,
                supports_real_wayland_surfaces: false,
                supports_gpu_rendering: false,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterActivationAttemptLedgerPresent {
                observed_count: 0,
                blocked_count: 0,
                allowed_count: 0,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterRealGlobalRegistrationFeasibility {
                attempted: false,
                blocked_count: 0,
                real_registered_count: 0,
                registration_enabled: false,
                accepts_clients: false,
                dispatches_protocol_events: false,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterGlobalHandlerBoundaryPresent {
                report_count: 3,
                ready_count: 0,
                blocked_count: 3,
                registers_protocol_globals: false,
                dispatches_protocol_events: false,
                accepts_clients: false,
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
        adapter.observe_unsupported_protocol_request(
            SmithayLinuxAdapterProtocolRequestKind::CompositorCreateSurface,
        );
        adapter.observe_unsupported_client_session();
        adapter.attempt_activate(SmithayLinuxAdapterActivationTarget::RealProtocolDispatch);
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
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterInertProtocolRequestLedger {
                observed_count: 1,
                rejected_unsupported_count: 1,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterClientSessionLedger {
                observed_count: 1,
                rejected_unsupported_count: 1,
                accepts_clients: false,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterActivationGatePresent {
                report_count: 8,
                blocked_count: 8,
                allowed_count: 0,
                accepts_clients: false,
                registers_protocol_globals: false,
                dispatches_protocol_events: false,
                supports_real_wayland_surfaces: false,
                supports_gpu_rendering: false,
                skeleton_only: true,
            }
        )));
        assert!(report.has_diagnostic(|diagnostic| matches!(
            diagnostic,
            BackendRuntimeDiagnostic::AdapterActivationAttemptLedgerPresent {
                observed_count: 1,
                blocked_count: 1,
                allowed_count: 0,
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
            "impl GlobalDispatch",
            "impl wayland_server::GlobalDispatch",
            "GlobalDispatch<",
            "impl Dispatch",
            "impl wayland_server::Dispatch",
            "Dispatch<",
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

        for line in production.lines().map(str::trim) {
            assert!(
                !(line.starts_with("use ")
                    && (line.contains("GlobalDispatch") || line.contains("Dispatch"))),
                "adapter skeleton 生产代码不得导入协议 dispatch trait: {line}"
            );
        }
    }
}
