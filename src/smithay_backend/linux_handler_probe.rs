#![cfg(all(feature = "smithay-linux", target_os = "linux"))]

//! Smithay handler 边界的隔离类型形状探针与 requirement matrix。
//!
//! 本模块只记录隔离类型形状的编译审计结果和 blocker evidence。它不实现
//! Smithay handler trait，不持有原生对象，也不进入 adapter、runtime 或核心状态。
//! Requirement matrix 描述建立 handler 前缺少什么，不表示这些入口已部分可用。
//! Reduction plan 只为后续隔离研究选择顺序，不提升任何 adapter capability。

use super::linux_adapter::SmithayLinuxAdapterGlobalHandlerKind;

/// 隔离 handler compile probe 的完成范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxHandlerProbeKind {
    /// 只验证隔离 inert handler 类型和报告可以编译。
    TypeShapeOnly,

    /// trait 形状已审计，但因会建立可执行处理边界而被阻止。
    TraitShapeBlocked,

    /// 为受控后续实现保留的隔离 trait 编译完成状态。
    TraitShapeCompiled,
}

/// 阻止 compile probe 扩展到真实 Smithay handler trait 的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerProbeBlocker {
    /// trait 实现会定义真实 client bind 入口。
    TraitImplWouldExposeClientBind,

    /// trait 实现会定义真实 protocol request dispatch 入口。
    TraitImplWouldExposeRequestDispatch,

    /// compositor handler 会要求真实 surface 回调边界。
    TraitImplWouldRequireSurfaceHandler,

    /// shared-memory 路径会要求真实 buffer handler。
    TraitImplWouldRequireBufferHandler,

    /// shared-memory 路径会要求真实 SHM handler state。
    TraitImplWouldRequireShmHandler,

    /// XDG shell 路径会要求真实 shell handler state 和回调。
    TraitImplWouldRequireXdgShellHandler,

    /// delegate 宏会安装实际协议分发实现面。
    DelegateMacroWouldInstallDispatchSurface,

    /// trait 形状本身不能使 runtime global 可用。
    CreateGlobalStillRequiredForRuntimeUse,

    /// compile probe 禁止接入生产 adapter。
    AdapterIntegrationForbidden,
}

/// 隔离 handler compile probe 的纯数据报告。
///
/// Diagnostic-only: `compiled_type_shape` 只证明无状态占位类型可编译；它不证明
/// `GlobalDispatch`、`Dispatch`、delegate 宏或 runtime global 已建立。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerProbeReport {
    /// 当前 probe 的完成范围。
    pub kind: SmithayLinuxHandlerProbeKind,

    /// 隔离 inert handler 类型是否已编译。
    pub compiled_type_shape: bool,

    /// Smithay handler trait 形状是否已实现并编译。
    pub compiled_trait_shape: bool,

    /// 是否使用了 Smithay delegate 宏。
    pub uses_delegate_macros: bool,

    /// 是否调用了真实 global 创建入口。
    pub calls_create_global: bool,

    /// 是否调用了其他真实 global 注册入口。
    pub calls_register_global: bool,

    /// probe 是否接触生产 adapter。
    pub touches_adapter: bool,

    /// probe 是否接触纯模型核心。
    pub touches_core: bool,

    /// 阻止 trait 或 runtime 集成的稳定原因。
    pub blockers: Vec<SmithayLinuxHandlerProbeBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// 不持有任何 Smithay 原生对象或生产状态的隔离类型形状。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SmithayLinuxInertHandlerProbe;

impl SmithayLinuxInertHandlerProbe {
    /// 构造无状态的隔离类型形状。
    pub const fn new() -> Self {
        Self
    }
}

/// 建立各类 protocol global handler 前必须满足的稳定要求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerRequirement {
    /// global bind 入口所需的 trait 边界。
    GlobalDispatchBind,

    /// protocol object request 入口所需的 trait 边界。
    DispatchRequest,

    /// compositor surface 状态与回调边界。
    CompositorHandler,

    /// shared-memory buffer 生命周期回调边界。
    BufferHandler,

    /// shared-memory state 回调边界。
    ShmHandler,

    /// XDG shell state 与回调边界。
    XdgShellHandler,

    /// compositor surface request 处理能力。
    SurfaceRequestHandling,

    /// XDG surface request 处理能力。
    XdgSurfaceRequestHandling,

    /// client 与 protocol object 的身份可见性。
    ClientObjectVisibility,

    /// protocol resource 的生命周期跟踪能力。
    ProtocolResourceTracking,

    /// surface 到核心窗口接纳的映射边界。
    CoreAdmissionMapping,
}

/// Handler requirement matrix 中单项要求的保守状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerRequirementState {
    /// 必需 trait 或 handler 尚未实现。
    Missing,

    /// 当前 skeleton policy 或 activation gate 阻止该能力。
    Blocked,

    /// 为尚未进入审计的 requirement 保留。
    NotAttempted,
}

/// 支撑 handler requirement 状态判断的纯数据证据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmithayLinuxHandlerRequirementEvidence {
    /// 该要求会建立真实 client bind 入口。
    RequiresClientBindEntry,

    /// 该要求会建立真实 protocol request 入口。
    RequiresProtocolRequestEntry,

    /// 该要求依赖真实 surface 生命周期。
    RequiresSurfaceLifecycleSupport,

    /// 该要求依赖真实 buffer 生命周期。
    RequiresBufferLifecycleSupport,

    /// 该要求依赖 shared-memory callback。
    RequiresShmCallbackSupport,

    /// 该要求依赖 XDG shell callback。
    RequiresXdgShellCallbackSupport,

    /// 该要求依赖 client 身份映射。
    RequiresClientIdentityMapping,

    /// 该要求依赖 protocol resource 跟踪。
    RequiresResourceTracking,

    /// 该要求依赖核心窗口接纳映射。
    RequiresCoreWindowAdmission,

    /// 该要求会安装实际 protocol dispatch 实现。
    WouldInstallDispatchImplementation,

    /// 常规 Smithay handler 集成路径会需要 delegate 宏。
    WouldRequireDelegateMacro,

    /// Adapter activation gate 阻止该要求进入生产路径。
    BlockedByActivationGate,

    /// skeleton-only policy 阻止该要求进入生产路径。
    BlockedBySkeletonOnlyPolicy,
}

/// 单项 handler requirement 及其 blocker evidence。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerRequirementMatrixItem {
    /// requirement 所属的 global handler。
    pub handler: SmithayLinuxAdapterGlobalHandlerKind,

    /// 必须满足的稳定要求。
    pub requirement: SmithayLinuxHandlerRequirement,

    /// 当前要求的保守状态。
    pub state: SmithayLinuxHandlerRequirementState,

    /// 支撑当前状态判断的非空稳定证据。
    pub evidence: Vec<SmithayLinuxHandlerRequirementEvidence>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Handler requirement matrix 的纯数据报告。
///
/// Matrix 将 `Missing` 与 `Blocked` 分开：前者表示 trait/handler 不存在，后者表示
/// 即使数据形状存在也不能越过当前安全策略。两者都不等于 runtime readiness。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerRequirementMatrixReport {
    /// 按 handler 和 requirement 固定顺序排列的矩阵项。
    pub items: Vec<SmithayLinuxHandlerRequirementMatrixItem>,

    /// 尚未实现的 trait 或 handler requirement 数量。
    pub missing_count: usize,

    /// 被能力边界阻止的 requirement 数量。
    pub blocked_count: usize,

    /// 已安全就绪的 requirement 数量；当前保守矩阵恒为零。
    pub ready_count: usize,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// 可从 handler requirement matrix 中选择的缩减目标。
///
/// 每个变体只命名一个未来研究边界，不表示相应 trait、handler 或 bridge 已实现。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxHandlerReductionCandidate {
    /// 隔离研究三类 global 共用的 bind trait 形状。
    GlobalDispatchBindShape,

    /// 隔离研究 protocol object request trait 形状。
    DispatchRequestShape,

    /// 隔离研究 compositor handler 所需的类型边界。
    CompositorHandlerShape,

    /// 隔离研究 shared-memory handler 所需的类型边界。
    ShmHandlerShape,

    /// 隔离研究 XDG shell handler 所需的类型边界。
    XdgShellHandlerShape,

    /// 规划 client 与 protocol object 的身份模型。
    ClientIdentityModel,

    /// 规划 protocol resource 的生命周期模型。
    ProtocolResourceModel,

    /// 规划 surface request 到纯数据生命周期的桥接边界。
    SurfaceLifecycleBridge,

    /// 规划 surface 到核心窗口接纳的桥接边界。
    CoreAdmissionBridge,
}

/// Reduction candidate 可能引入的结构化风险。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxHandlerReductionRisk {
    /// 研究目标可能暴露真实 client bind 参数或入口。
    IntroducesClientBindEntry,

    /// 研究目标可能暴露真实 protocol request dispatch 参数或入口。
    IntroducesRequestDispatchEntry,

    /// 常规集成路径可能要求 delegate 宏。
    RequiresDelegateMacro,

    /// 研究目标依赖真实 surface 生命周期。
    RequiresSurfaceLifecycle,

    /// 研究目标依赖 shared-memory buffer 生命周期。
    RequiresBufferLifecycle,

    /// 研究目标依赖 XDG object 生命周期。
    RequiresXdgLifecycle,

    /// 研究目标依赖稳定 client 身份模型。
    RequiresClientIdentity,

    /// 研究目标依赖 protocol resource 跟踪。
    RequiresResourceTracking,

    /// 研究目标依赖核心窗口接纳。
    RequiresCoreAdmission,

    /// 研究目标若接入生产路径会打开真实 protocol surface。
    WouldOpenRealProtocolSurface,

    /// 研究必须保持在 handler planning/probe 层，不能接入 adapter。
    MustRemainIsolated,
}

/// Reduction candidate 在当前计划中的选择结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxHandlerReductionDecision {
    /// 选为下一阶段唯一的隔离研究目标。
    SelectedFirst,

    /// 有价值但必须等待更早的 requirement 被缩减。
    Deferred,

    /// 当前安全边界明确禁止进入实现研究。
    Blocked,
}

/// 单项 handler requirement reduction candidate 报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerReductionCandidateReport {
    /// 被评估的稳定候选项。
    pub candidate: SmithayLinuxHandlerReductionCandidate,

    /// 当前计划对该候选项的选择结果。
    pub decision: SmithayLinuxHandlerReductionDecision,

    /// 该候选项对应的非空 requirement 集合。
    pub related_requirements: Vec<SmithayLinuxHandlerRequirement>,

    /// 在进入任何实现前必须处理的非空风险集合。
    pub risks: Vec<SmithayLinuxHandlerReductionRisk>,

    /// 不依赖运行时状态的固定选择理由。
    pub rationale: &'static str,

    /// 当前 candidate 是否仍然只属于结构规划边界。
    pub skeleton_only: bool,
}

/// Handler requirement reduction 的稳定纯数据计划。
///
/// Planning-only: 该报告不进入 adapter snapshot、capability、activation gate 或
/// runtime report。`selected_first` 只是后续隔离研究输入，不是可执行能力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxHandlerReductionPlanReport {
    /// 按依赖和风险固定顺序排列的候选项。
    pub candidates: Vec<SmithayLinuxHandlerReductionCandidateReport>,

    /// 唯一优先研究候选；matrix 不再满足前置条件时为 `None`。
    pub selected_first: Option<SmithayLinuxHandlerReductionCandidate>,

    /// 标记为 `SelectedFirst` 的候选数量。
    pub selected_count: usize,

    /// 标记为 `Deferred` 的候选数量。
    pub deferred_count: usize,

    /// 标记为 `Blocked` 的候选数量。
    pub blocked_count: usize,

    /// 当前计划是否仍然只属于结构规划边界。
    pub skeleton_only: bool,
}

/// Bind shape 使用的纯 synthetic client 标识。
///
/// 数值只用于隔离测试和诊断，不对应真实 client、进程、socket 或文件描述符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmithayLinuxBindClientSyntheticId(u64);

impl SmithayLinuxBindClientSyntheticId {
    /// 从任意 inert 数值构造 synthetic ID；零值同样有效。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// 返回 synthetic ID 的原始数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Bind client identity 的稳定来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindClientIdentitySource {
    /// Identity 仅由隔离 synthetic 数据提供。
    SyntheticOnly,
}

/// Bind client identity 的稳定建模状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindClientIdentityState {
    /// Synthetic identity 已建模，但不代表真实 client 可用。
    SyntheticModeled,
}

/// 阻止 synthetic client identity 进入真实 bind 路径的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindClientIdentityBlocker {
    /// 真实 Smithay client object 仍不可用。
    RealClientObjectUnavailable,

    /// 当前阶段禁止接收真实 client。
    ClientAcceptForbidden,

    /// 当前阶段禁止管理 socket client。
    SocketClientHandlingForbidden,

    /// Identity model 禁止接入生产 adapter。
    AdapterIntegrationForbidden,

    /// `GlobalDispatch` bind 入口仍禁止实现。
    GlobalDispatchBindStillForbidden,
}

/// 隔离 synthetic bind client identity 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxBindClientIdentityReport {
    /// Identity 数据的稳定来源。
    pub source: SmithayLinuxBindClientIdentitySource,

    /// Identity 的稳定建模状态。
    pub state: SmithayLinuxBindClientIdentityState,

    /// 用于隔离诊断的固定 synthetic ID。
    pub synthetic_id: SmithayLinuxBindClientSyntheticId,

    /// 当前 identity 是否代表真实 client。
    pub represents_real_client: bool,

    /// 当前模型是否接收真实 client。
    pub accepts_client: bool,

    /// 当前模型是否接触 socket 或文件描述符。
    pub touches_socket: bool,

    /// 当前模型是否接触生产 adapter。
    pub touches_adapter: bool,

    /// 阻止 identity 进入真实 bind 路径的非空原因。
    pub blockers: Vec<SmithayLinuxBindClientIdentityBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Bind shape 使用的纯 synthetic global resource 标识。
///
/// 数值只用于隔离测试和诊断，不对应真实 Wayland resource、object ID 或系统资源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmithayLinuxBindGlobalResourceSyntheticId(u64);

impl SmithayLinuxBindGlobalResourceSyntheticId {
    /// 从任意 inert 数值构造 synthetic ID；零值同样有效。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// 返回 synthetic ID 的原始数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Bind global resource identity 的稳定来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindGlobalResourceIdentitySource {
    /// Identity 仅由隔离 synthetic 数据提供。
    SyntheticOnly,
}

/// Bind global resource identity 的稳定建模状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindGlobalResourceIdentityState {
    /// Synthetic identity 已建模，但不代表真实 protocol resource 可用。
    SyntheticModeled,
}

/// 阻止 synthetic global resource identity 进入真实 bind 路径的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindGlobalResourceIdentityBlocker {
    /// 真实 Wayland resource object 仍不可用。
    RealResourceObjectUnavailable,

    /// 当前阶段禁止真实 protocol global 注册。
    RealGlobalRegistrationForbidden,

    /// 真实 protocol resource 生命周期跟踪仍不可用。
    ProtocolResourceTrackingUnavailable,

    /// Identity model 禁止接入生产 adapter。
    AdapterIntegrationForbidden,

    /// `GlobalDispatch` bind 入口仍禁止实现。
    GlobalDispatchBindStillForbidden,
}

/// 隔离 synthetic bind global resource identity 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxBindGlobalResourceIdentityReport {
    /// Identity 数据的稳定来源。
    pub source: SmithayLinuxBindGlobalResourceIdentitySource,

    /// Identity 的稳定建模状态。
    pub state: SmithayLinuxBindGlobalResourceIdentityState,

    /// 用于隔离诊断的固定 synthetic ID。
    pub synthetic_id: SmithayLinuxBindGlobalResourceSyntheticId,

    /// 当前 identity 是否代表真实 Wayland resource。
    pub represents_real_resource: bool,

    /// 当前 identity 是否来自真实 protocol global 注册。
    pub comes_from_real_global_registration: bool,

    /// 当前模型是否跟踪真实 protocol resource。
    pub tracks_protocol_resource: bool,

    /// 当前模型是否接触生产 adapter。
    pub touches_adapter: bool,

    /// 阻止 identity 进入真实 bind 路径的非空原因。
    pub blockers: Vec<SmithayLinuxBindGlobalResourceIdentityBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Bind shape 使用的纯 synthetic global data 标识。
///
/// 数值只用于隔离测试和诊断，不对应真实 Smithay global data 或 protocol global。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmithayLinuxBindGlobalDataSyntheticId(u64);

impl SmithayLinuxBindGlobalDataSyntheticId {
    /// 从任意 inert 数值构造 synthetic ID；零值同样有效。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// 返回 synthetic ID 的原始数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Bind global data 的稳定来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindGlobalDataSource {
    /// Global data 仅由隔离 synthetic 数据提供。
    SyntheticOnly,
}

/// Bind global data 的稳定建模状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindGlobalDataState {
    /// Synthetic global data 已建模，但不代表真实 protocol global 可用。
    SyntheticModeled,
}

/// 阻止 synthetic global data 进入真实 bind 路径的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindGlobalDataBlocker {
    /// 真实 Smithay global data 仍不可用。
    RealGlobalDataUnavailable,

    /// 当前阶段禁止真实 protocol global 注册。
    RealGlobalRegistrationForbidden,

    /// 真实 protocol global object 仍不可用。
    ProtocolGlobalObjectUnavailable,

    /// Global data model 禁止接入生产 adapter。
    AdapterIntegrationForbidden,

    /// `GlobalDispatch` bind 入口仍禁止实现。
    GlobalDispatchBindStillForbidden,
}

/// 隔离 synthetic bind global data 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxBindGlobalDataReport {
    /// Global data 的稳定来源。
    pub source: SmithayLinuxBindGlobalDataSource,

    /// Global data 的稳定建模状态。
    pub state: SmithayLinuxBindGlobalDataState,

    /// 用于隔离诊断的固定 synthetic ID。
    pub synthetic_id: SmithayLinuxBindGlobalDataSyntheticId,

    /// 当前模型是否代表真实 Smithay global data。
    pub represents_real_global_data: bool,

    /// 当前模型是否来自真实 protocol global 注册。
    pub comes_from_real_global_registration: bool,

    /// 当前模型是否跟踪真实 protocol global。
    pub tracks_protocol_global: bool,

    /// 当前模型是否接触生产 adapter。
    pub touches_adapter: bool,

    /// 阻止 global data 进入真实 bind 路径的非空原因。
    pub blockers: Vec<SmithayLinuxBindGlobalDataBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Bind shape 使用的纯 synthetic handler state 标识。
///
/// 数值只用于隔离测试和诊断，不对应真实 Smithay、adapter 或 protocol dispatch state。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmithayLinuxBindHandlerStateSyntheticId(u64);

impl SmithayLinuxBindHandlerStateSyntheticId {
    /// 从任意 inert 数值构造 synthetic ID；零值同样有效。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// 返回 synthetic ID 的原始数值。
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Bind handler state 的稳定来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindHandlerStateSource {
    /// Handler state 仅由隔离 synthetic 数据提供。
    SyntheticOnly,
}

/// Bind handler state 的稳定建模状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindHandlerStateState {
    /// Synthetic handler state 已建模，但不代表真实 Smithay state 可用。
    SyntheticModeled,
}

/// 阻止 synthetic handler state 进入真实 bind 路径的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxBindHandlerStateBlocker {
    /// 真实 Smithay handler state 仍不可用。
    RealSmithayHandlerStateUnavailable,

    /// 当前模型禁止接入生产 adapter state。
    AdapterStateIntegrationForbidden,

    /// 真实 protocol dispatch state 仍不可用。
    ProtocolDispatchStateUnavailable,

    /// 原生 display handle 必须继续保持隐藏。
    DisplayHandleStillHidden,

    /// `GlobalDispatch` bind 入口仍禁止实现。
    GlobalDispatchBindStillForbidden,
}

/// 隔离 synthetic bind handler state 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxBindHandlerStateReport {
    /// Handler state 的稳定来源。
    pub source: SmithayLinuxBindHandlerStateSource,

    /// Handler state 的稳定建模状态。
    pub state: SmithayLinuxBindHandlerStateState,

    /// 用于隔离诊断的固定 synthetic ID。
    pub synthetic_id: SmithayLinuxBindHandlerStateSyntheticId,

    /// 当前模型是否代表真实 Smithay handler state。
    pub represents_real_handler_state: bool,

    /// 当前模型是否接触生产 adapter。
    pub touches_adapter: bool,

    /// 当前模型是否接触真实 protocol dispatch state。
    pub touches_dispatch_state: bool,

    /// 当前模型是否接触原生 display handle。
    pub touches_display_handle: bool,

    /// 阻止 handler state 进入真实 bind 路径的非空原因。
    pub blockers: Vec<SmithayLinuxBindHandlerStateBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// 原生 display handle 的稳定访问策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleAccessPolicy {
    /// Handle 必须保持在本模块不可见的私有边界内。
    Hidden,
}

/// Display handle 诊断信息的稳定脱敏级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleRedaction {
    /// 报告不包含任何真实 handle 值、引用或可访问对象。
    FullyRedacted,
}

/// 阻止 display handle 进入 bind 或 adapter 路径的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleAccessBlocker {
    /// 真实 handle 必须继续保留在私有实现边界内。
    RealDisplayHandleMustRemainPrivate,

    /// 当前阶段禁止通过 public API 暴露 handle。
    DisplayHandleExposureForbidden,

    /// 当前阶段禁止在 probe 模型中存储 handle。
    DisplayHandleStorageForbidden,

    /// 当前阶段禁止通过 handle 注册真实 protocol global。
    GlobalRegistrationForbidden,

    /// 当前阶段禁止把 handle 接入 adapter public API。
    AdapterPublicApiExposureForbidden,

    /// `GlobalDispatch` bind 入口仍禁止实现。
    GlobalDispatchBindStillForbidden,
}

/// Display handle 访问边界的纯数据脱敏报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandleAccessReport {
    /// 当前稳定访问策略。
    pub policy: SmithayLinuxDisplayHandleAccessPolicy,

    /// 当前稳定脱敏级别。
    pub redaction: SmithayLinuxDisplayHandleRedaction,

    /// 当前报告是否代表真实 handle。
    pub represents_real_display_handle: bool,

    /// 当前报告是否暴露真实 handle。
    pub exposes_display_handle: bool,

    /// 当前报告是否存储真实 handle。
    pub stores_display_handle: bool,

    /// 当前报告是否读取真实 handle。
    pub reads_display_handle: bool,

    /// 当前边界是否允许调用真实 global 创建入口。
    pub can_call_create_global: bool,

    /// 当前边界是否允许调用真实 global 注册入口。
    pub can_call_register_global: bool,

    /// 当前报告是否接触 adapter public API。
    pub touches_adapter_public_api: bool,

    /// 阻止 handle 进入真实路径的非空原因。
    pub blockers: Vec<SmithayLinuxDisplayHandleAccessBlocker>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// `GlobalDispatch` bind 入口会暴露的稳定输入概念。
///
/// 这些变体只记录类型形状，不持有相应的 Smithay 原生对象。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxGlobalDispatchBindInput {
    /// 发起 bind 的 client 概念。
    ClientObject,

    /// bind 后创建的 global resource 概念。
    GlobalResourceObject,

    /// global 注册时关联的数据概念。
    GlobalDataObject,

    /// bind 签名中的 display handle 概念；真实 handle 必须保持隐藏。
    DisplayHandleObject,

    /// 接收 bind 回调的 handler state 概念。
    HandlerState,
}

/// 阻止 bind 输入概念进入真实 trait 或 adapter 的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxGlobalDispatchBindBlocker {
    /// Client 身份和所有权尚未建模。
    ClientObjectNotModeled,

    /// Protocol resource 身份和生命周期尚未建模。
    ResourceObjectNotModeled,

    /// Global data 的稳定表示尚未建模。
    GlobalDataNotModeled,

    /// 原生 display handle 不能从 probe 层暴露。
    DisplayHandleMustRemainHidden,

    /// Handler state 尚未接入任何协议处理边界。
    HandlerStateNotIntegrated,

    /// Shape probe 禁止接入生产 adapter。
    AdapterIntegrationForbidden,

    /// 继续实现会建立真实 client bind 入口。
    WouldCreateClientBindEntry,

    /// 继续实现会依赖真实 protocol global 注册。
    WouldRequireRealGlobalRegistration,
}

/// 单项 `GlobalDispatch` bind 输入形状及其 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxGlobalDispatchBindShapeItem {
    /// 被审计的稳定输入概念。
    pub input: SmithayLinuxGlobalDispatchBindInput,

    /// 当前输入概念是否已有可用于 trait 实现的模型。
    pub modeled: bool,

    /// 阻止该输入进入真实 bind 入口的非空原因。
    pub blockers: Vec<SmithayLinuxGlobalDispatchBindBlocker>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// `GlobalDispatch` bind 入口的隔离纯数据形状报告。
///
/// 该报告不读取 adapter 或 bootstrap，不创建 global，也不安装 trait 实现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxGlobalDispatchBindShapeReport {
    /// 按 bind 输入概念固定顺序排列的报告项。
    pub items: Vec<SmithayLinuxGlobalDispatchBindShapeItem>,

    /// 已具备隔离模型的输入数量；不代表相应真实对象可用。
    pub modeled_count: usize,

    /// 仍被 blocker 阻止的输入数量。
    pub blocked_count: usize,

    /// 当前输入模型是否足以编译真实 trait 实现。
    pub can_compile_trait_impl: bool,

    /// 当前报告是否可以接入生产 adapter。
    pub can_attach_to_adapter: bool,

    /// 当前报告是否可以注册真实 protocol global。
    pub can_register_global: bool,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// `GlobalDispatch` bind 形状的最终 readiness。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxGlobalDispatchBindReadiness {
    /// 当前输入模型不足以进入真实 trait、adapter 或 protocol 路径。
    NotReady,
}

/// Final seal 中单个 bind 输入的稳定状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxGlobalDispatchBindSealedInputState {
    /// 输入只有隔离 synthetic 模型。
    SyntheticModeled,

    /// 输入保持隐藏并完全脱敏。
    HiddenRedacted,
}

/// 阻止 sealed bind shape 进入真实运行路径的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxGlobalDispatchBindFinalBlocker {
    /// 原生 display handle 必须继续隐藏。
    DisplayHandleHidden,

    /// 真实 display handle 当前不可用于 bind。
    RealDisplayHandleUnavailable,

    /// 真实 protocol global 注册仍禁止。
    GlobalRegistrationForbidden,

    /// 真实 client bind 入口仍禁止。
    ClientBindEntryForbidden,

    /// 生产 adapter 集成仍禁止。
    AdapterIntegrationForbidden,

    /// 真实 trait 实现仍禁止。
    TraitImplementationForbidden,

    /// Protocol request dispatch 仍禁止。
    DispatchRequestForbidden,

    /// Surface 生命周期能力仍不可用。
    SurfaceLifecycleUnavailable,

    /// Core admission 能力仍不可用。
    CoreAdmissionUnavailable,
}

/// Final seal 中单个 bind 输入的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxGlobalDispatchBindSealedInputItem {
    /// 被封板的稳定 bind 输入概念。
    pub input: SmithayLinuxGlobalDispatchBindInput,

    /// 当前输入的稳定 sealed 状态。
    pub state: SmithayLinuxGlobalDispatchBindSealedInputState,

    /// 当前输入是否只有 synthetic 模型。
    pub modeled: bool,

    /// 当前输入是否必须保持隐藏。
    pub hidden: bool,

    /// 当前输入是否完全脱敏。
    pub redacted: bool,

    /// 阻止输入进入真实路径的非空原因。
    pub blockers: Vec<SmithayLinuxGlobalDispatchBindFinalBlocker>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// `GlobalDispatch` bind shape 的最终纯数据封板报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxGlobalDispatchBindFinalSealReport {
    /// 当前整体 readiness。
    pub readiness: SmithayLinuxGlobalDispatchBindReadiness,

    /// 按既有 bind shape 固定顺序排列的 sealed 输入。
    pub inputs: Vec<SmithayLinuxGlobalDispatchBindSealedInputItem>,

    /// 只有 synthetic 模型的输入数量。
    pub synthetic_modeled_count: usize,

    /// 保持隐藏并完全脱敏的输入数量。
    pub hidden_redacted_count: usize,

    /// 仍带有 blocker 的输入数量。
    pub blocked_count: usize,

    /// 当前 sealed shape 是否足以编译真实 trait 实现。
    pub can_compile_trait_impl: bool,

    /// 当前 sealed shape 是否可以接入生产 adapter。
    pub can_attach_to_adapter: bool,

    /// 当前 sealed shape 是否可以注册真实 protocol global。
    pub can_register_global: bool,

    /// 当前 sealed shape 是否可以进入 protocol request dispatch。
    pub can_dispatch_requests: bool,

    /// 当前 sealed shape 是否可以创建真实 surface。
    pub can_create_surfaces: bool,

    /// 当前 sealed shape 是否可以进入 core admission。
    pub can_enter_core_admission: bool,

    /// 下一项允许研究的 internal-only policy 名称。
    pub next_safe_target: Option<&'static str>,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Display handle internal-only access gate 的稳定决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalAccessDecision {
    /// 当前前置条件不足，禁止任何真实 handle 访问。
    Blocked,
}

/// Internal access gate 评估的未来目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalAccessTarget {
    /// 未来真实 protocol global 注册路径。
    FutureGlobalRegistration,

    /// 未来 `GlobalDispatch` bind 路径。
    FutureGlobalDispatchBind,
}

/// 未来 internal-only handle 访问必须满足的稳定前置条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalAccessPrecondition {
    /// Adapter 在私有实现边界中持有 handle。
    AdapterOwnsDisplayHandleInternally,

    /// Adapter public API 不暴露 handle。
    AdapterDoesNotExposeDisplayHandlePublicly,

    /// Activation gate 允许真实 protocol global 注册。
    ActivationGateAllowsRealProtocolGlobalRegistration,

    /// Global registration plan 已脱离 skeleton。
    GlobalRegistrationPlanPromotedFromSkeleton,

    /// `GlobalDispatch` trait 边界已经编译。
    GlobalDispatchTraitBoundaryCompiled,

    /// Protocol request dispatch 边界已经定义。
    DispatchRequestBoundaryDefined,

    /// Handler state 已在私有实现边界中集成。
    HandlerStateIntegratedInternally,

    /// Display handle 脱敏策略继续保持。
    DisplayHandleRedactionPolicyPreserved,
}

/// Internal access gate 单项前置条件的稳定状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalAccessPreconditionState {
    /// 纯策略前置条件已经满足。
    Satisfied,

    /// 所需结构边界尚未建立。
    Missing,

    /// 当前安全策略明确阻止该前置条件。
    Blocked,
}

/// 阻止 internal-only handle 访问的稳定原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalAccessBlocker {
    /// 当前阶段禁止访问真实 handle。
    RealDisplayHandleAccessForbidden,

    /// 当前阶段禁止读取真实 handle。
    DisplayHandleReadForbidden,

    /// 当前阶段禁止存储真实 handle。
    DisplayHandleStorageForbidden,

    /// 当前阶段禁止通过 public API 暴露 handle。
    PublicExposureForbidden,

    /// Activation gate 仍阻止真实能力。
    ActivationGateBlocked,

    /// 真实 protocol global 注册仍禁止。
    GlobalRegistrationForbidden,

    /// `GlobalDispatch` trait 边界仍缺失。
    GlobalDispatchTraitMissing,

    /// Protocol request dispatch 边界仍缺失。
    DispatchRequestBoundaryMissing,

    /// Handler state 当前只有 synthetic 模型。
    HandlerStateOnlySynthetic,

    /// 生产 adapter 集成仍禁止。
    AdapterIntegrationForbidden,
}

/// Internal access gate 的单项纯数据前置条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandleInternalAccessPreconditionItem {
    /// 被评估的稳定前置条件。
    pub precondition: SmithayLinuxDisplayHandleInternalAccessPrecondition,

    /// 当前稳定状态。
    pub state: SmithayLinuxDisplayHandleInternalAccessPreconditionState,

    /// 阻止非满足状态进入真实路径的原因。
    pub blockers: Vec<SmithayLinuxDisplayHandleInternalAccessBlocker>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Display handle internal-only access gate 的纯数据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandleInternalAccessGateReport {
    /// 当前评估的未来目标。
    pub target: SmithayLinuxDisplayHandleInternalAccessTarget,

    /// 当前 gate 决策。
    pub decision: SmithayLinuxDisplayHandleInternalAccessDecision,

    /// 按稳定顺序排列的前置条件。
    pub preconditions: Vec<SmithayLinuxDisplayHandleInternalAccessPreconditionItem>,

    /// 已满足的纯策略前置条件数量。
    pub satisfied_count: usize,

    /// 尚未建立的结构前置条件数量。
    pub missing_count: usize,

    /// 被安全策略明确阻止的前置条件数量。
    pub blocked_count: usize,

    /// 当前 gate 是否允许读取真实 handle。
    pub can_read_display_handle: bool,

    /// 当前 gate 是否允许存储真实 handle。
    pub can_store_display_handle: bool,

    /// 当前 gate 是否允许暴露真实 handle。
    pub can_expose_display_handle: bool,

    /// 当前 gate 是否允许调用真实 global 创建入口。
    pub can_call_create_global: bool,

    /// 当前 gate 是否允许调用真实 global 注册入口。
    pub can_call_register_global: bool,

    /// 当前 gate 是否允许编译真实 `GlobalDispatch`。
    pub can_compile_global_dispatch: bool,

    /// 当前 gate 是否允许进入 protocol request dispatch。
    pub can_dispatch_requests: bool,

    /// 当前 gate 是否允许接入生产 adapter。
    pub can_attach_to_adapter: bool,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Adapter public API 的 display handle 暴露结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandlePublicApiExposureDecision {
    /// 静态证据未发现受限 public API surface。
    NotExposed,
}

/// Display handle public API non-exposure 审计的稳定 surface。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandlePublicApiSurface {
    /// Public API 返回 display handle。
    DisplayHandleReturnValue,

    /// Public API 接收 display handle 参数。
    DisplayHandleArgument,

    /// Public API 返回 display 对象。
    DisplayReturnValue,

    /// Public API 接收 display 对象参数。
    DisplayArgument,

    /// Public API 返回可变 bootstrap。
    MutableBootstrapReturnValue,

    /// Public API 接收可变 bootstrap 参数。
    MutableBootstrapArgument,

    /// Public API 暴露真实 global 创建入口。
    CreateGlobalEntrypoint,

    /// Public API 暴露真实 global 注册入口。
    RegisterGlobalEntrypoint,

    /// Adapter capability 暗示真实能力已打开。
    AdapterCapabilityFlag,
}

/// Public API non-exposure 单项证据的稳定状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandlePublicApiEvidenceState {
    /// 对应 public API surface 不存在。
    Absent,

    /// 对应 capability 明确保持保守 false。
    ConservativeFalse,
}

/// Public API non-exposure 静态证据的稳定限制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandlePublicApiEvidenceLimitation {
    /// 证据只描述当前静态 API 边界。
    StaticEvidenceOnly,

    /// 证据不读取 adapter runtime state。
    DoesNotReadAdapterRuntimeState,

    /// 证据不证明 handle internal ownership 已建立。
    DoesNotProveInternalOwnership,

    /// 证据不允许读取或持有真实 handle。
    DoesNotPermitDisplayHandleAccess,

    /// 证据不允许真实 protocol global 注册。
    DoesNotPermitGlobalRegistration,

    /// 证据不允许实现真实 dispatch trait。
    DoesNotPermitTraitImplementation,
}

/// Public API non-exposure 的单项纯数据证据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandlePublicApiEvidenceItem {
    /// 被审计的稳定 public API surface。
    pub surface: SmithayLinuxDisplayHandlePublicApiSurface,

    /// 当前静态证据状态。
    pub state: SmithayLinuxDisplayHandlePublicApiEvidenceState,

    /// 当前证据不能证明或允许的非空限制。
    pub limitations: Vec<SmithayLinuxDisplayHandlePublicApiEvidenceLimitation>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Adapter public API non-exposure 的纯数据静态证据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandlePublicApiEvidenceReport {
    /// 当前稳定 non-exposure 结论。
    pub decision: SmithayLinuxDisplayHandlePublicApiExposureDecision,

    /// 按稳定顺序排列的 public API surface 证据。
    pub items: Vec<SmithayLinuxDisplayHandlePublicApiEvidenceItem>,

    /// 被静态证据判定为不存在的 surface 数量。
    pub absent_count: usize,

    /// 明确保持保守 false 的 capability surface 数量。
    pub conservative_false_count: usize,

    /// 暴露受限 public API surface 的数量。
    pub exposed_count: usize,

    /// 证据是否足以满足 public non-exposure 前置条件。
    pub can_satisfy_public_non_exposure_precondition: bool,

    /// 当前证据是否允许读取真实 handle。
    pub can_read_display_handle: bool,

    /// 当前证据是否允许存储真实 handle。
    pub can_store_display_handle: bool,

    /// 当前证据是否允许暴露真实 handle。
    pub can_expose_display_handle: bool,

    /// 当前证据是否允许调用真实 global 创建入口。
    pub can_call_create_global: bool,

    /// 当前证据是否允许调用真实 global 注册入口。
    pub can_call_register_global: bool,

    /// 当前证据是否允许编译真实 `GlobalDispatch`。
    pub can_compile_global_dispatch: bool,

    /// 当前证据是否允许接入生产 adapter。
    pub can_attach_to_adapter: bool,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Adapter internal display handle ownership 的静态证据结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalOwnershipDecision {
    /// 静态源码边界足以证明 adapter 私有持有链存在。
    StaticPrivateOwnershipEvidencePresent,

    /// 当前静态源码边界不足以证明 adapter 私有持有链。
    StaticEvidenceInsufficient,
}

/// Internal ownership 静态证据的稳定来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource {
    /// Bootstrap 内部持有 Wayland display 边界。
    BootstrapBoundary,

    /// Linux runtime 私有持有 bootstrap 边界。
    LinuxRuntimeBoundary,

    /// Linux adapter public API non-exposure 审计。
    LinuxAdapterPublicApiAudit,
}

/// Internal ownership 单项静态证据状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalOwnershipEvidenceState {
    /// 对应边界存在私有持有关系。
    PresentPrivate,

    /// 对应 adapter public API surface 不存在。
    AbsentPublic,

    /// 对应 capability 明确保持保守 false。
    ConservativeFalse,

    /// 对应静态证据尚未建立。
    Missing,
}

/// Internal ownership 静态证据不能证明或允许的稳定限制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayLinuxDisplayHandleInternalOwnershipLimitation {
    /// 证据只描述当前静态源码边界。
    StaticEvidenceOnly,

    /// 证据不读取真实 display handle。
    DoesNotReadDisplayHandle,

    /// 证据不证明真实 handle access safety。
    DoesNotProveAccessSafety,

    /// 证据不允许真实 protocol global 注册。
    DoesNotPermitGlobalRegistration,

    /// 证据不允许实现真实 dispatch trait。
    DoesNotPermitTraitImplementation,

    /// 证据不允许接入生产 adapter。
    DoesNotPermitAdapterIntegration,
}

/// Internal ownership 的单项纯数据静态证据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem {
    /// 当前证据的稳定来源。
    pub source: SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource,

    /// 当前静态证据状态。
    pub state: SmithayLinuxDisplayHandleInternalOwnershipEvidenceState,

    /// 当前证据不能证明或允许的非空限制。
    pub limitations: Vec<SmithayLinuxDisplayHandleInternalOwnershipLimitation>,

    /// 当前 item 是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// Adapter internal display handle ownership 的纯数据静态证据报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayLinuxDisplayHandleInternalOwnershipEvidenceReport {
    /// 当前稳定 internal ownership 结论。
    pub decision: SmithayLinuxDisplayHandleInternalOwnershipDecision,

    /// 按稳定顺序排列的 ownership 证据。
    pub items: Vec<SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem>,

    /// 存在私有持有关系的证据数量。
    pub present_private_count: usize,

    /// Adapter public API 不存在受限 surface 的证据数量。
    pub absent_public_count: usize,

    /// 明确保持保守 false 的证据数量。
    pub conservative_false_count: usize,

    /// 尚未建立的证据数量。
    pub missing_count: usize,

    /// 当前静态证据是否足以满足 internal ownership 前置条件。
    pub can_satisfy_internal_ownership_precondition: bool,

    /// 当前证据是否允许读取真实 handle。
    pub can_read_display_handle: bool,

    /// 当前证据是否允许存储真实 handle。
    pub can_store_display_handle: bool,

    /// 当前证据是否允许暴露真实 handle。
    pub can_expose_display_handle: bool,

    /// 当前证据是否允许调用真实 global 创建入口。
    pub can_call_create_global: bool,

    /// 当前证据是否允许调用真实 global 注册入口。
    pub can_call_register_global: bool,

    /// 当前证据是否允许编译真实 `GlobalDispatch`。
    pub can_compile_global_dispatch: bool,

    /// 当前证据是否允许接入生产 adapter。
    pub can_attach_to_adapter: bool,

    /// 当前报告是否仍然只描述结构骨架。
    pub skeleton_only: bool,
}

/// 返回 handler trait 审计的固定保守报告。
pub fn smithay_linux_handler_probe_report() -> SmithayLinuxHandlerProbeReport {
    SmithayLinuxHandlerProbeReport {
        kind: SmithayLinuxHandlerProbeKind::TypeShapeOnly,
        compiled_type_shape: true,
        compiled_trait_shape: false,
        uses_delegate_macros: false,
        calls_create_global: false,
        calls_register_global: false,
        touches_adapter: false,
        touches_core: false,
        blockers: vec![
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeClientBind,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeRequestDispatch,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireSurfaceHandler,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireBufferHandler,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireShmHandler,
            SmithayLinuxHandlerProbeBlocker::TraitImplWouldRequireXdgShellHandler,
            SmithayLinuxHandlerProbeBlocker::DelegateMacroWouldInstallDispatchSurface,
            SmithayLinuxHandlerProbeBlocker::CreateGlobalStillRequiredForRuntimeUse,
            SmithayLinuxHandlerProbeBlocker::AdapterIntegrationForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回 handler requirements 和 blocker evidence 的固定矩阵。
pub fn smithay_linux_handler_requirement_matrix_report()
-> SmithayLinuxHandlerRequirementMatrixReport {
    use SmithayLinuxAdapterGlobalHandlerKind as Handler;
    use SmithayLinuxHandlerRequirement as Requirement;
    use SmithayLinuxHandlerRequirementEvidence as Evidence;
    use SmithayLinuxHandlerRequirementState as State;

    let items = vec![
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::GlobalDispatchBind,
            State::Missing,
            vec![
                Evidence::RequiresClientBindEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::DispatchRequest,
            State::Missing,
            vec![
                Evidence::RequiresProtocolRequestEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::CompositorHandler,
            State::Missing,
            vec![
                Evidence::RequiresSurfaceLifecycleSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::SurfaceRequestHandling,
            State::Blocked,
            vec![Evidence::RequiresSurfaceLifecycleSupport],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::ClientObjectVisibility,
            State::Blocked,
            vec![Evidence::RequiresClientIdentityMapping],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::ProtocolResourceTracking,
            State::Blocked,
            vec![Evidence::RequiresResourceTracking],
        ),
        requirement_item(
            Handler::CompositorGlobalHandler,
            Requirement::CoreAdmissionMapping,
            State::Blocked,
            vec![Evidence::RequiresCoreWindowAdmission],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::GlobalDispatchBind,
            State::Missing,
            vec![
                Evidence::RequiresClientBindEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::DispatchRequest,
            State::Missing,
            vec![
                Evidence::RequiresProtocolRequestEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::BufferHandler,
            State::Missing,
            vec![
                Evidence::RequiresBufferLifecycleSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::ShmHandler,
            State::Missing,
            vec![
                Evidence::RequiresShmCallbackSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::ClientObjectVisibility,
            State::Blocked,
            vec![Evidence::RequiresClientIdentityMapping],
        ),
        requirement_item(
            Handler::ShmGlobalHandler,
            Requirement::ProtocolResourceTracking,
            State::Blocked,
            vec![Evidence::RequiresResourceTracking],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::GlobalDispatchBind,
            State::Missing,
            vec![
                Evidence::RequiresClientBindEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::DispatchRequest,
            State::Missing,
            vec![
                Evidence::RequiresProtocolRequestEntry,
                Evidence::WouldInstallDispatchImplementation,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::XdgShellHandler,
            State::Missing,
            vec![
                Evidence::RequiresXdgShellCallbackSupport,
                Evidence::WouldInstallDispatchImplementation,
                Evidence::WouldRequireDelegateMacro,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::XdgSurfaceRequestHandling,
            State::Blocked,
            vec![
                Evidence::RequiresSurfaceLifecycleSupport,
                Evidence::RequiresXdgShellCallbackSupport,
            ],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::ClientObjectVisibility,
            State::Blocked,
            vec![Evidence::RequiresClientIdentityMapping],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::ProtocolResourceTracking,
            State::Blocked,
            vec![Evidence::RequiresResourceTracking],
        ),
        requirement_item(
            Handler::XdgWmBaseGlobalHandler,
            Requirement::CoreAdmissionMapping,
            State::Blocked,
            vec![Evidence::RequiresCoreWindowAdmission],
        ),
    ];
    let missing_count = items
        .iter()
        .filter(|item| item.state == State::Missing)
        .count();
    let blocked_count = items
        .iter()
        .filter(|item| item.state == State::Blocked)
        .count();

    SmithayLinuxHandlerRequirementMatrixReport {
        items,
        missing_count,
        blocked_count,
        ready_count: 0,
        skeleton_only: true,
    }
}

/// 根据当前 requirement matrix 生成固定顺序的 reduction plan。
///
/// `GlobalDispatchBindShape` 只有在三类 global handler 都仍将
/// `GlobalDispatchBind` 标记为 `Missing` 时才会成为唯一 first target。该选择只允许
/// 后续研究 bind 类型形状；不得接入 adapter、创建 global 或处理 request。
pub fn smithay_linux_handler_reduction_plan_report() -> SmithayLinuxHandlerReductionPlanReport {
    use SmithayLinuxHandlerReductionCandidate as Candidate;
    use SmithayLinuxHandlerReductionDecision as Decision;
    use SmithayLinuxHandlerReductionRisk as Risk;
    use SmithayLinuxHandlerRequirement as Requirement;

    let matrix = smithay_linux_handler_requirement_matrix_report();
    let bind_shape_is_common_missing =
        requirement_is_missing_for_all_handlers(&matrix, Requirement::GlobalDispatchBind);
    let bind_decision = if bind_shape_is_common_missing {
        Decision::SelectedFirst
    } else {
        Decision::Blocked
    };
    let candidates = vec![
        reduction_candidate(
            Candidate::GlobalDispatchBindShape,
            bind_decision,
            vec![Requirement::GlobalDispatchBind],
            vec![
                Risk::IntroducesClientBindEntry,
                Risk::WouldOpenRealProtocolSurface,
                Risk::MustRemainIsolated,
            ],
            "三类 global handler 的共同前置 requirement；只研究 bind 类型形状，不接入 adapter。",
        ),
        reduction_candidate(
            Candidate::DispatchRequestShape,
            Decision::Deferred,
            vec![Requirement::DispatchRequest],
            vec![
                Risk::IntroducesRequestDispatchEntry,
                Risk::WouldOpenRealProtocolSurface,
                Risk::MustRemainIsolated,
            ],
            "Request dispatch 会建立可执行协议入口，必须晚于隔离 bind 形状研究。",
        ),
        reduction_candidate(
            Candidate::CompositorHandlerShape,
            Decision::Deferred,
            vec![
                Requirement::CompositorHandler,
                Requirement::SurfaceRequestHandling,
            ],
            vec![
                Risk::RequiresDelegateMacro,
                Risk::RequiresSurfaceLifecycle,
                Risk::WouldOpenRealProtocolSurface,
                Risk::MustRemainIsolated,
            ],
            "Compositor handler 依赖 surface 回调和 dispatch 安装面，不能作为首个缩减目标。",
        ),
        reduction_candidate(
            Candidate::ShmHandlerShape,
            Decision::Deferred,
            vec![Requirement::BufferHandler, Requirement::ShmHandler],
            vec![
                Risk::RequiresDelegateMacro,
                Risk::RequiresBufferLifecycle,
                Risk::WouldOpenRealProtocolSurface,
                Risk::MustRemainIsolated,
            ],
            "SHM handler 同时依赖 buffer 生命周期和 dispatch 安装面，应在共同 bind 边界之后研究。",
        ),
        reduction_candidate(
            Candidate::XdgShellHandlerShape,
            Decision::Deferred,
            vec![
                Requirement::XdgShellHandler,
                Requirement::XdgSurfaceRequestHandling,
            ],
            vec![
                Risk::RequiresDelegateMacro,
                Risk::RequiresSurfaceLifecycle,
                Risk::RequiresXdgLifecycle,
                Risk::WouldOpenRealProtocolSurface,
                Risk::MustRemainIsolated,
            ],
            "XDG handler 依赖 shell 与 surface 生命周期，当前只能保留为后续隔离研究项。",
        ),
        reduction_candidate(
            Candidate::ClientIdentityModel,
            Decision::Deferred,
            vec![Requirement::ClientObjectVisibility],
            vec![Risk::RequiresClientIdentity, Risk::MustRemainIsolated],
            "Client identity 是 bind/resource 参数建模的依赖，但本计划不接收或保存真实 client。",
        ),
        reduction_candidate(
            Candidate::ProtocolResourceModel,
            Decision::Deferred,
            vec![Requirement::ProtocolResourceTracking],
            vec![Risk::RequiresResourceTracking, Risk::MustRemainIsolated],
            "Resource model 必须先定义稳定生命周期语义，不能由真实 protocol object 隐式承担。",
        ),
        reduction_candidate(
            Candidate::SurfaceLifecycleBridge,
            Decision::Blocked,
            vec![
                Requirement::SurfaceRequestHandling,
                Requirement::XdgSurfaceRequestHandling,
            ],
            vec![
                Risk::RequiresSurfaceLifecycle,
                Risk::RequiresXdgLifecycle,
                Risk::WouldOpenRealProtocolSurface,
                Risk::MustRemainIsolated,
            ],
            "真实 surface request 仍被 activation 与 skeleton policy 阻止，禁止提前建立 bridge。",
        ),
        reduction_candidate(
            Candidate::CoreAdmissionBridge,
            Decision::Blocked,
            vec![Requirement::CoreAdmissionMapping],
            vec![
                Risk::RequiresSurfaceLifecycle,
                Risk::RequiresCoreAdmission,
                Risk::MustRemainIsolated,
            ],
            "核心接纳依赖已验证的 surface/resource 生命周期，当前不得跨越 handler planning 边界。",
        ),
    ];
    let selected_first = candidates
        .iter()
        .find(|report| report.decision == Decision::SelectedFirst)
        .map(|report| report.candidate);
    let selected_count = candidates
        .iter()
        .filter(|report| report.decision == Decision::SelectedFirst)
        .count();
    let deferred_count = candidates
        .iter()
        .filter(|report| report.decision == Decision::Deferred)
        .count();
    let blocked_count = candidates
        .iter()
        .filter(|report| report.decision == Decision::Blocked)
        .count();

    SmithayLinuxHandlerReductionPlanReport {
        candidates,
        selected_first,
        selected_count,
        deferred_count,
        blocked_count,
        skeleton_only: true,
    }
}

/// 返回固定的 synthetic bind client identity 报告。
///
/// 该函数只构造纯数据，不读取 adapter、bootstrap、socket 或真实 client 状态。
pub fn smithay_linux_bind_client_identity_report() -> SmithayLinuxBindClientIdentityReport {
    use SmithayLinuxBindClientIdentityBlocker as Blocker;

    SmithayLinuxBindClientIdentityReport {
        source: SmithayLinuxBindClientIdentitySource::SyntheticOnly,
        state: SmithayLinuxBindClientIdentityState::SyntheticModeled,
        synthetic_id: SmithayLinuxBindClientSyntheticId::new(1),
        represents_real_client: false,
        accepts_client: false,
        touches_socket: false,
        touches_adapter: false,
        blockers: vec![
            Blocker::RealClientObjectUnavailable,
            Blocker::ClientAcceptForbidden,
            Blocker::SocketClientHandlingForbidden,
            Blocker::AdapterIntegrationForbidden,
            Blocker::GlobalDispatchBindStillForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回固定的 synthetic bind global resource identity 报告。
///
/// 该函数只构造纯数据，不读取 adapter、bootstrap 或真实 protocol resource 状态。
pub fn smithay_linux_bind_global_resource_identity_report()
-> SmithayLinuxBindGlobalResourceIdentityReport {
    use SmithayLinuxBindGlobalResourceIdentityBlocker as Blocker;

    SmithayLinuxBindGlobalResourceIdentityReport {
        source: SmithayLinuxBindGlobalResourceIdentitySource::SyntheticOnly,
        state: SmithayLinuxBindGlobalResourceIdentityState::SyntheticModeled,
        synthetic_id: SmithayLinuxBindGlobalResourceSyntheticId::new(1),
        represents_real_resource: false,
        comes_from_real_global_registration: false,
        tracks_protocol_resource: false,
        touches_adapter: false,
        blockers: vec![
            Blocker::RealResourceObjectUnavailable,
            Blocker::RealGlobalRegistrationForbidden,
            Blocker::ProtocolResourceTrackingUnavailable,
            Blocker::AdapterIntegrationForbidden,
            Blocker::GlobalDispatchBindStillForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回固定的 synthetic bind global data 报告。
///
/// 该函数只构造纯数据，不读取 adapter、bootstrap 或真实 protocol global 状态。
pub fn smithay_linux_bind_global_data_report() -> SmithayLinuxBindGlobalDataReport {
    use SmithayLinuxBindGlobalDataBlocker as Blocker;

    SmithayLinuxBindGlobalDataReport {
        source: SmithayLinuxBindGlobalDataSource::SyntheticOnly,
        state: SmithayLinuxBindGlobalDataState::SyntheticModeled,
        synthetic_id: SmithayLinuxBindGlobalDataSyntheticId::new(1),
        represents_real_global_data: false,
        comes_from_real_global_registration: false,
        tracks_protocol_global: false,
        touches_adapter: false,
        blockers: vec![
            Blocker::RealGlobalDataUnavailable,
            Blocker::RealGlobalRegistrationForbidden,
            Blocker::ProtocolGlobalObjectUnavailable,
            Blocker::AdapterIntegrationForbidden,
            Blocker::GlobalDispatchBindStillForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回固定的 synthetic bind handler state 报告。
///
/// 该函数只构造纯数据，不读取 adapter、bootstrap、display handle 或 dispatch state。
pub fn smithay_linux_bind_handler_state_report() -> SmithayLinuxBindHandlerStateReport {
    use SmithayLinuxBindHandlerStateBlocker as Blocker;

    SmithayLinuxBindHandlerStateReport {
        source: SmithayLinuxBindHandlerStateSource::SyntheticOnly,
        state: SmithayLinuxBindHandlerStateState::SyntheticModeled,
        synthetic_id: SmithayLinuxBindHandlerStateSyntheticId::new(1),
        represents_real_handler_state: false,
        touches_adapter: false,
        touches_dispatch_state: false,
        touches_display_handle: false,
        blockers: vec![
            Blocker::RealSmithayHandlerStateUnavailable,
            Blocker::AdapterStateIntegrationForbidden,
            Blocker::ProtocolDispatchStateUnavailable,
            Blocker::DisplayHandleStillHidden,
            Blocker::GlobalDispatchBindStillForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回固定的 display handle 访问策略和脱敏报告。
///
/// 该函数只构造纯数据，不读取 adapter、bootstrap 或任何真实 handle。
pub fn smithay_linux_display_handle_access_report() -> SmithayLinuxDisplayHandleAccessReport {
    use SmithayLinuxDisplayHandleAccessBlocker as Blocker;

    SmithayLinuxDisplayHandleAccessReport {
        policy: SmithayLinuxDisplayHandleAccessPolicy::Hidden,
        redaction: SmithayLinuxDisplayHandleRedaction::FullyRedacted,
        represents_real_display_handle: false,
        exposes_display_handle: false,
        stores_display_handle: false,
        reads_display_handle: false,
        can_call_create_global: false,
        can_call_register_global: false,
        touches_adapter_public_api: false,
        blockers: vec![
            Blocker::RealDisplayHandleMustRemainPrivate,
            Blocker::DisplayHandleExposureForbidden,
            Blocker::DisplayHandleStorageForbidden,
            Blocker::GlobalRegistrationForbidden,
            Blocker::AdapterPublicApiExposureForbidden,
            Blocker::GlobalDispatchBindStillForbidden,
        ],
        skeleton_only: true,
    }
}

/// 返回 `GlobalDispatch` bind 输入概念的固定保守形状报告。
///
/// 除 DisplayHandleObject 外的输入都只有 synthetic 模型。所有 item 仍有 blocker，
/// 因此不建立 trait、adapter 或 global readiness。
pub fn smithay_linux_global_dispatch_bind_shape_report() -> SmithayLinuxGlobalDispatchBindShapeReport
{
    use SmithayLinuxGlobalDispatchBindBlocker as Blocker;
    use SmithayLinuxGlobalDispatchBindInput as Input;

    let items = vec![
        global_dispatch_bind_shape_item(
            Input::ClientObject,
            true,
            vec![
                Blocker::WouldCreateClientBindEntry,
                Blocker::WouldRequireRealGlobalRegistration,
                Blocker::AdapterIntegrationForbidden,
            ],
        ),
        global_dispatch_bind_shape_item(
            Input::GlobalResourceObject,
            true,
            vec![
                Blocker::WouldCreateClientBindEntry,
                Blocker::WouldRequireRealGlobalRegistration,
                Blocker::AdapterIntegrationForbidden,
            ],
        ),
        global_dispatch_bind_shape_item(
            Input::GlobalDataObject,
            true,
            vec![
                Blocker::WouldRequireRealGlobalRegistration,
                Blocker::AdapterIntegrationForbidden,
            ],
        ),
        global_dispatch_bind_shape_item(
            Input::DisplayHandleObject,
            false,
            vec![
                Blocker::DisplayHandleMustRemainHidden,
                Blocker::WouldRequireRealGlobalRegistration,
                Blocker::AdapterIntegrationForbidden,
            ],
        ),
        global_dispatch_bind_shape_item(
            Input::HandlerState,
            true,
            vec![
                Blocker::WouldCreateClientBindEntry,
                Blocker::WouldRequireRealGlobalRegistration,
                Blocker::AdapterIntegrationForbidden,
            ],
        ),
    ];
    let modeled_count = items.iter().filter(|item| item.modeled).count();
    let blocked_count = items
        .iter()
        .filter(|item| !item.blockers.is_empty())
        .count();

    SmithayLinuxGlobalDispatchBindShapeReport {
        items,
        modeled_count,
        blocked_count,
        can_compile_trait_impl: false,
        can_attach_to_adapter: false,
        can_register_global: false,
        skeleton_only: true,
    }
}

/// 返回 `GlobalDispatch` bind shape 的最终保守封板报告。
///
/// 该函数只汇总既有 shape 和访问策略纯数据，不读取 adapter、bootstrap 或系统对象。
pub fn smithay_linux_global_dispatch_bind_final_seal_report()
-> SmithayLinuxGlobalDispatchBindFinalSealReport {
    use SmithayLinuxDisplayHandleAccessPolicy as AccessPolicy;
    use SmithayLinuxDisplayHandleRedaction as Redaction;
    use SmithayLinuxGlobalDispatchBindInput as Input;
    use SmithayLinuxGlobalDispatchBindSealedInputState as State;

    let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
    let display_policy = smithay_linux_display_handle_access_report();
    let display_is_hidden = display_policy.policy == AccessPolicy::Hidden;
    let display_is_redacted = display_policy.redaction == Redaction::FullyRedacted;
    let inputs = bind_shape
        .items
        .into_iter()
        .map(|item| {
            let is_display = item.input == Input::DisplayHandleObject;
            let state = if is_display {
                State::HiddenRedacted
            } else {
                State::SyntheticModeled
            };
            let blockers = final_seal_blockers(&item, is_display);

            SmithayLinuxGlobalDispatchBindSealedInputItem {
                input: item.input,
                state,
                modeled: item.modeled,
                hidden: is_display && display_is_hidden,
                redacted: is_display && display_is_redacted,
                blockers,
                skeleton_only: item.skeleton_only,
            }
        })
        .collect::<Vec<_>>();
    let synthetic_modeled_count = inputs
        .iter()
        .filter(|item| item.state == State::SyntheticModeled && item.modeled)
        .count();
    let hidden_redacted_count = inputs
        .iter()
        .filter(|item| item.state == State::HiddenRedacted && item.hidden && item.redacted)
        .count();
    let blocked_count = inputs
        .iter()
        .filter(|item| !item.blockers.is_empty())
        .count();

    SmithayLinuxGlobalDispatchBindFinalSealReport {
        readiness: SmithayLinuxGlobalDispatchBindReadiness::NotReady,
        inputs,
        synthetic_modeled_count,
        hidden_redacted_count,
        blocked_count,
        can_compile_trait_impl: bind_shape.can_compile_trait_impl,
        can_attach_to_adapter: bind_shape.can_attach_to_adapter,
        can_register_global: bind_shape.can_register_global,
        can_dispatch_requests: false,
        can_create_surfaces: false,
        can_enter_core_admission: false,
        next_safe_target: Some("DisplayHandleInternalAccessGatePolicy"),
        skeleton_only: bind_shape.skeleton_only && display_policy.skeleton_only,
    }
}

/// 返回 adapter public API non-exposure 的固定静态证据报告。
///
/// 该函数只汇总 planning 层纯数据，不读取 adapter、bootstrap 或任何真实 handle。
pub fn smithay_linux_display_handle_public_api_evidence_report()
-> SmithayLinuxDisplayHandlePublicApiEvidenceReport {
    use SmithayLinuxDisplayHandleAccessPolicy as AccessPolicy;
    use SmithayLinuxDisplayHandlePublicApiEvidenceState as State;
    use SmithayLinuxDisplayHandlePublicApiSurface as Surface;
    use SmithayLinuxDisplayHandleRedaction as Redaction;
    use SmithayLinuxGlobalDispatchBindInput as Input;
    use SmithayLinuxGlobalDispatchBindReadiness as Readiness;
    use SmithayLinuxHandlerReductionCandidate as Candidate;

    let display_policy = smithay_linux_display_handle_access_report();
    let final_seal = smithay_linux_global_dispatch_bind_final_seal_report();
    let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
    let reduction_plan = smithay_linux_handler_reduction_plan_report();
    let matrix = smithay_linux_handler_requirement_matrix_report();
    let probe = smithay_linux_handler_probe_report();

    let items = vec![
        public_api_evidence_item(Surface::DisplayHandleReturnValue, State::Absent),
        public_api_evidence_item(Surface::DisplayHandleArgument, State::Absent),
        public_api_evidence_item(Surface::DisplayReturnValue, State::Absent),
        public_api_evidence_item(Surface::DisplayArgument, State::Absent),
        public_api_evidence_item(Surface::MutableBootstrapReturnValue, State::Absent),
        public_api_evidence_item(Surface::MutableBootstrapArgument, State::Absent),
        public_api_evidence_item(Surface::CreateGlobalEntrypoint, State::Absent),
        public_api_evidence_item(Surface::RegisterGlobalEntrypoint, State::Absent),
        public_api_evidence_item(Surface::AdapterCapabilityFlag, State::ConservativeFalse),
    ];
    let absent_count = items
        .iter()
        .filter(|item| item.state == State::Absent)
        .count();
    let conservative_false_count = items
        .iter()
        .filter(|item| item.state == State::ConservativeFalse)
        .count();
    let public_non_exposure_is_preserved = display_policy.policy == AccessPolicy::Hidden
        && display_policy.redaction == Redaction::FullyRedacted
        && !display_policy.exposes_display_handle
        && !display_policy.touches_adapter_public_api
        && final_seal.readiness == Readiness::NotReady
        && !final_seal.can_attach_to_adapter
        && !final_seal.can_register_global
        && bind_shape
            .items
            .iter()
            .find(|item| item.input == Input::DisplayHandleObject)
            .is_some_and(|item| !item.modeled)
        && reduction_plan.selected_first == Some(Candidate::GlobalDispatchBindShape)
        && matrix.ready_count == 0
        && !probe.compiled_trait_shape;

    SmithayLinuxDisplayHandlePublicApiEvidenceReport {
        decision: SmithayLinuxDisplayHandlePublicApiExposureDecision::NotExposed,
        items,
        absent_count,
        conservative_false_count,
        exposed_count: 0,
        can_satisfy_public_non_exposure_precondition: public_non_exposure_is_preserved,
        can_read_display_handle: false,
        can_store_display_handle: false,
        can_expose_display_handle: false,
        can_call_create_global: false,
        can_call_register_global: false,
        can_compile_global_dispatch: false,
        can_attach_to_adapter: false,
        skeleton_only: display_policy.skeleton_only
            && final_seal.skeleton_only
            && bind_shape.skeleton_only
            && reduction_plan.skeleton_only
            && matrix.skeleton_only
            && probe.skeleton_only,
    }
}

/// 返回 adapter internal display handle ownership 的固定静态证据报告。
///
/// 该函数只汇总 planning 层纯数据，不读取 adapter、bootstrap 或任何真实 handle。
pub fn smithay_linux_display_handle_internal_ownership_evidence_report()
-> SmithayLinuxDisplayHandleInternalOwnershipEvidenceReport {
    use SmithayLinuxDisplayHandleAccessPolicy as AccessPolicy;
    use SmithayLinuxDisplayHandleInternalOwnershipDecision as Decision;
    use SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource as Source;
    use SmithayLinuxDisplayHandleInternalOwnershipEvidenceState as State;
    use SmithayLinuxDisplayHandlePublicApiExposureDecision as ExposureDecision;
    use SmithayLinuxDisplayHandleRedaction as Redaction;
    use SmithayLinuxGlobalDispatchBindInput as Input;
    use SmithayLinuxGlobalDispatchBindReadiness as Readiness;
    use SmithayLinuxHandlerReductionCandidate as Candidate;

    let display_policy = smithay_linux_display_handle_access_report();
    let public_api_evidence = smithay_linux_display_handle_public_api_evidence_report();
    let final_seal = smithay_linux_global_dispatch_bind_final_seal_report();
    let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
    let reduction_plan = smithay_linux_handler_reduction_plan_report();
    let matrix = smithay_linux_handler_requirement_matrix_report();
    let probe = smithay_linux_handler_probe_report();

    let items = vec![
        internal_ownership_evidence_item(Source::BootstrapBoundary, State::PresentPrivate),
        internal_ownership_evidence_item(Source::LinuxRuntimeBoundary, State::PresentPrivate),
        internal_ownership_evidence_item(Source::LinuxAdapterPublicApiAudit, State::AbsentPublic),
    ];
    let present_private_count = items
        .iter()
        .filter(|item| item.state == State::PresentPrivate)
        .count();
    let absent_public_count = items
        .iter()
        .filter(|item| item.state == State::AbsentPublic)
        .count();
    let conservative_false_count = items
        .iter()
        .filter(|item| item.state == State::ConservativeFalse)
        .count();
    let missing_count = items
        .iter()
        .filter(|item| item.state == State::Missing)
        .count();
    let static_private_ownership_evidence_is_present = present_private_count == 2
        && absent_public_count == 1
        && conservative_false_count == 0
        && missing_count == 0
        && public_api_evidence.decision == ExposureDecision::NotExposed
        && public_api_evidence.exposed_count == 0
        && public_api_evidence.conservative_false_count == 1
        && public_api_evidence.can_satisfy_public_non_exposure_precondition
        && display_policy.policy == AccessPolicy::Hidden
        && display_policy.redaction == Redaction::FullyRedacted
        && !display_policy.represents_real_display_handle
        && !display_policy.reads_display_handle
        && !display_policy.stores_display_handle
        && !display_policy.exposes_display_handle
        && final_seal.readiness == Readiness::NotReady
        && !final_seal.can_compile_trait_impl
        && !final_seal.can_attach_to_adapter
        && !final_seal.can_register_global
        && bind_shape
            .items
            .iter()
            .find(|item| item.input == Input::DisplayHandleObject)
            .is_some_and(|item| !item.modeled)
        && reduction_plan.selected_first == Some(Candidate::GlobalDispatchBindShape)
        && matrix.ready_count == 0
        && !probe.compiled_trait_shape;

    SmithayLinuxDisplayHandleInternalOwnershipEvidenceReport {
        decision: if static_private_ownership_evidence_is_present {
            Decision::StaticPrivateOwnershipEvidencePresent
        } else {
            Decision::StaticEvidenceInsufficient
        },
        items,
        present_private_count,
        absent_public_count,
        conservative_false_count,
        missing_count,
        can_satisfy_internal_ownership_precondition: static_private_ownership_evidence_is_present,
        can_read_display_handle: false,
        can_store_display_handle: false,
        can_expose_display_handle: false,
        can_call_create_global: false,
        can_call_register_global: false,
        can_compile_global_dispatch: false,
        can_attach_to_adapter: false,
        skeleton_only: display_policy.skeleton_only
            && public_api_evidence.skeleton_only
            && final_seal.skeleton_only
            && bind_shape.skeleton_only
            && reduction_plan.skeleton_only
            && matrix.skeleton_only
            && probe.skeleton_only,
    }
}

/// 返回 display handle internal-only access gate 的固定保守报告。
///
/// 该函数只汇总 planning 层纯数据，不读取 adapter、bootstrap 或任何真实 handle。
pub fn smithay_linux_display_handle_internal_access_gate_report()
-> SmithayLinuxDisplayHandleInternalAccessGateReport {
    use SmithayLinuxDisplayHandleInternalAccessBlocker as Blocker;
    use SmithayLinuxDisplayHandleInternalAccessPrecondition as Precondition;
    use SmithayLinuxDisplayHandleInternalAccessPreconditionState as State;
    use SmithayLinuxDisplayHandleRedaction as Redaction;
    use SmithayLinuxGlobalDispatchBindInput as Input;
    use SmithayLinuxGlobalDispatchBindReadiness as Readiness;
    use SmithayLinuxHandlerReductionCandidate as Candidate;

    let display_policy = smithay_linux_display_handle_access_report();
    let final_seal = smithay_linux_global_dispatch_bind_final_seal_report();
    let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
    let reduction_plan = smithay_linux_handler_reduction_plan_report();
    let matrix = smithay_linux_handler_requirement_matrix_report();
    let probe = smithay_linux_handler_probe_report();
    let public_api_evidence = smithay_linux_display_handle_public_api_evidence_report();
    let internal_ownership_evidence =
        smithay_linux_display_handle_internal_ownership_evidence_report();

    let public_non_exposure_is_proven =
        public_api_evidence.can_satisfy_public_non_exposure_precondition;
    let internal_ownership_is_proven =
        internal_ownership_evidence.can_satisfy_internal_ownership_precondition;
    let redaction_is_preserved = display_policy.redaction == Redaction::FullyRedacted
        && !display_policy.represents_real_display_handle
        && !display_policy.reads_display_handle
        && !display_policy.stores_display_handle;
    let registration_is_blocked = final_seal.readiness == Readiness::NotReady
        && !final_seal.can_register_global
        && matrix.ready_count == 0;
    let trait_boundary_is_missing = !probe.compiled_trait_shape
        && !final_seal.can_compile_trait_impl
        && reduction_plan.selected_first == Some(Candidate::GlobalDispatchBindShape);
    let handler_state_is_synthetic = bind_shape
        .items
        .iter()
        .find(|item| item.input == Input::HandlerState)
        .is_some_and(|item| item.modeled && bind_shape.can_attach_to_adapter == false);

    let preconditions = vec![
        internal_access_precondition_item(
            Precondition::AdapterOwnsDisplayHandleInternally,
            if internal_ownership_is_proven {
                State::Satisfied
            } else {
                State::Blocked
            },
            if internal_ownership_is_proven {
                Vec::new()
            } else {
                vec![
                    Blocker::RealDisplayHandleAccessForbidden,
                    Blocker::DisplayHandleReadForbidden,
                    Blocker::DisplayHandleStorageForbidden,
                    Blocker::AdapterIntegrationForbidden,
                ]
            },
        ),
        internal_access_precondition_item(
            Precondition::AdapterDoesNotExposeDisplayHandlePublicly,
            if public_non_exposure_is_proven {
                State::Satisfied
            } else {
                State::Blocked
            },
            if public_non_exposure_is_proven {
                Vec::new()
            } else {
                vec![
                    Blocker::PublicExposureForbidden,
                    Blocker::AdapterIntegrationForbidden,
                ]
            },
        ),
        internal_access_precondition_item(
            Precondition::ActivationGateAllowsRealProtocolGlobalRegistration,
            State::Blocked,
            vec![
                Blocker::ActivationGateBlocked,
                Blocker::GlobalRegistrationForbidden,
            ],
        ),
        internal_access_precondition_item(
            Precondition::GlobalRegistrationPlanPromotedFromSkeleton,
            if registration_is_blocked {
                State::Blocked
            } else {
                State::Missing
            },
            vec![Blocker::GlobalRegistrationForbidden],
        ),
        internal_access_precondition_item(
            Precondition::GlobalDispatchTraitBoundaryCompiled,
            if trait_boundary_is_missing {
                State::Missing
            } else {
                State::Blocked
            },
            vec![Blocker::GlobalDispatchTraitMissing],
        ),
        internal_access_precondition_item(
            Precondition::DispatchRequestBoundaryDefined,
            State::Missing,
            vec![Blocker::DispatchRequestBoundaryMissing],
        ),
        internal_access_precondition_item(
            Precondition::HandlerStateIntegratedInternally,
            if handler_state_is_synthetic {
                State::Missing
            } else {
                State::Blocked
            },
            vec![
                Blocker::HandlerStateOnlySynthetic,
                Blocker::AdapterIntegrationForbidden,
            ],
        ),
        internal_access_precondition_item(
            Precondition::DisplayHandleRedactionPolicyPreserved,
            if redaction_is_preserved {
                State::Satisfied
            } else {
                State::Blocked
            },
            if redaction_is_preserved {
                Vec::new()
            } else {
                vec![Blocker::PublicExposureForbidden]
            },
        ),
    ];
    let satisfied_count = preconditions
        .iter()
        .filter(|item| item.state == State::Satisfied)
        .count();
    let missing_count = preconditions
        .iter()
        .filter(|item| item.state == State::Missing)
        .count();
    let blocked_count = preconditions
        .iter()
        .filter(|item| item.state == State::Blocked)
        .count();

    SmithayLinuxDisplayHandleInternalAccessGateReport {
        target: SmithayLinuxDisplayHandleInternalAccessTarget::FutureGlobalRegistration,
        decision: SmithayLinuxDisplayHandleInternalAccessDecision::Blocked,
        preconditions,
        satisfied_count,
        missing_count,
        blocked_count,
        can_read_display_handle: false,
        can_store_display_handle: false,
        can_expose_display_handle: false,
        can_call_create_global: false,
        can_call_register_global: false,
        can_compile_global_dispatch: false,
        can_dispatch_requests: false,
        can_attach_to_adapter: false,
        skeleton_only: display_policy.skeleton_only
            && final_seal.skeleton_only
            && bind_shape.skeleton_only
            && reduction_plan.skeleton_only
            && matrix.skeleton_only
            && probe.skeleton_only
            && public_api_evidence.skeleton_only
            && internal_ownership_evidence.skeleton_only,
    }
}

fn requirement_item(
    handler: SmithayLinuxAdapterGlobalHandlerKind,
    requirement: SmithayLinuxHandlerRequirement,
    state: SmithayLinuxHandlerRequirementState,
    mut evidence: Vec<SmithayLinuxHandlerRequirementEvidence>,
) -> SmithayLinuxHandlerRequirementMatrixItem {
    evidence.extend([
        SmithayLinuxHandlerRequirementEvidence::BlockedByActivationGate,
        SmithayLinuxHandlerRequirementEvidence::BlockedBySkeletonOnlyPolicy,
    ]);

    SmithayLinuxHandlerRequirementMatrixItem {
        handler,
        requirement,
        state,
        evidence,
        skeleton_only: true,
    }
}

fn global_dispatch_bind_shape_item(
    input: SmithayLinuxGlobalDispatchBindInput,
    modeled: bool,
    blockers: Vec<SmithayLinuxGlobalDispatchBindBlocker>,
) -> SmithayLinuxGlobalDispatchBindShapeItem {
    SmithayLinuxGlobalDispatchBindShapeItem {
        input,
        modeled,
        blockers,
        skeleton_only: true,
    }
}

fn final_seal_blockers(
    item: &SmithayLinuxGlobalDispatchBindShapeItem,
    is_display: bool,
) -> Vec<SmithayLinuxGlobalDispatchBindFinalBlocker> {
    use SmithayLinuxGlobalDispatchBindBlocker as ShapeBlocker;
    use SmithayLinuxGlobalDispatchBindFinalBlocker as FinalBlocker;

    let mut blockers = Vec::new();
    for blocker in &item.blockers {
        let blocker = match blocker {
            ShapeBlocker::DisplayHandleMustRemainHidden => FinalBlocker::DisplayHandleHidden,
            ShapeBlocker::AdapterIntegrationForbidden => FinalBlocker::AdapterIntegrationForbidden,
            ShapeBlocker::WouldCreateClientBindEntry => FinalBlocker::ClientBindEntryForbidden,
            ShapeBlocker::WouldRequireRealGlobalRegistration => {
                FinalBlocker::GlobalRegistrationForbidden
            }
            ShapeBlocker::ClientObjectNotModeled
            | ShapeBlocker::ResourceObjectNotModeled
            | ShapeBlocker::GlobalDataNotModeled
            | ShapeBlocker::HandlerStateNotIntegrated => FinalBlocker::TraitImplementationForbidden,
        };
        push_unique_final_blocker(&mut blockers, blocker);
    }
    if is_display {
        push_unique_final_blocker(&mut blockers, FinalBlocker::RealDisplayHandleUnavailable);
    }
    for blocker in [
        FinalBlocker::TraitImplementationForbidden,
        FinalBlocker::DispatchRequestForbidden,
        FinalBlocker::SurfaceLifecycleUnavailable,
        FinalBlocker::CoreAdmissionUnavailable,
    ] {
        push_unique_final_blocker(&mut blockers, blocker);
    }

    blockers
}

fn push_unique_final_blocker(
    blockers: &mut Vec<SmithayLinuxGlobalDispatchBindFinalBlocker>,
    blocker: SmithayLinuxGlobalDispatchBindFinalBlocker,
) {
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

fn public_api_evidence_item(
    surface: SmithayLinuxDisplayHandlePublicApiSurface,
    state: SmithayLinuxDisplayHandlePublicApiEvidenceState,
) -> SmithayLinuxDisplayHandlePublicApiEvidenceItem {
    use SmithayLinuxDisplayHandlePublicApiEvidenceLimitation as Limitation;

    SmithayLinuxDisplayHandlePublicApiEvidenceItem {
        surface,
        state,
        limitations: vec![
            Limitation::StaticEvidenceOnly,
            Limitation::DoesNotReadAdapterRuntimeState,
            Limitation::DoesNotProveInternalOwnership,
            Limitation::DoesNotPermitDisplayHandleAccess,
            Limitation::DoesNotPermitGlobalRegistration,
            Limitation::DoesNotPermitTraitImplementation,
        ],
        skeleton_only: true,
    }
}

fn internal_ownership_evidence_item(
    source: SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource,
    state: SmithayLinuxDisplayHandleInternalOwnershipEvidenceState,
) -> SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem {
    use SmithayLinuxDisplayHandleInternalOwnershipLimitation as Limitation;

    SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem {
        source,
        state,
        limitations: vec![
            Limitation::StaticEvidenceOnly,
            Limitation::DoesNotReadDisplayHandle,
            Limitation::DoesNotProveAccessSafety,
            Limitation::DoesNotPermitGlobalRegistration,
            Limitation::DoesNotPermitTraitImplementation,
            Limitation::DoesNotPermitAdapterIntegration,
        ],
        skeleton_only: true,
    }
}

fn internal_access_precondition_item(
    precondition: SmithayLinuxDisplayHandleInternalAccessPrecondition,
    state: SmithayLinuxDisplayHandleInternalAccessPreconditionState,
    blockers: Vec<SmithayLinuxDisplayHandleInternalAccessBlocker>,
) -> SmithayLinuxDisplayHandleInternalAccessPreconditionItem {
    SmithayLinuxDisplayHandleInternalAccessPreconditionItem {
        precondition,
        state,
        blockers,
        skeleton_only: true,
    }
}

fn reduction_candidate(
    candidate: SmithayLinuxHandlerReductionCandidate,
    decision: SmithayLinuxHandlerReductionDecision,
    related_requirements: Vec<SmithayLinuxHandlerRequirement>,
    risks: Vec<SmithayLinuxHandlerReductionRisk>,
    rationale: &'static str,
) -> SmithayLinuxHandlerReductionCandidateReport {
    SmithayLinuxHandlerReductionCandidateReport {
        candidate,
        decision,
        related_requirements,
        risks,
        rationale,
        skeleton_only: true,
    }
}

fn requirement_is_missing_for_all_handlers(
    matrix: &SmithayLinuxHandlerRequirementMatrixReport,
    requirement: SmithayLinuxHandlerRequirement,
) -> bool {
    use SmithayLinuxHandlerRequirementState as State;

    let matching = matrix
        .items
        .iter()
        .filter(|item| item.requirement == requirement)
        .collect::<Vec<_>>();

    matching.len() == 3 && matching.iter().all(|item| item.state == State::Missing)
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayLinuxBindClientIdentityBlocker, SmithayLinuxBindClientIdentitySource,
        SmithayLinuxBindClientIdentityState, SmithayLinuxBindClientSyntheticId,
        SmithayLinuxBindGlobalDataBlocker, SmithayLinuxBindGlobalDataSource,
        SmithayLinuxBindGlobalDataState, SmithayLinuxBindGlobalDataSyntheticId,
        SmithayLinuxBindGlobalResourceIdentityBlocker,
        SmithayLinuxBindGlobalResourceIdentitySource, SmithayLinuxBindGlobalResourceIdentityState,
        SmithayLinuxBindGlobalResourceSyntheticId, SmithayLinuxBindHandlerStateBlocker,
        SmithayLinuxBindHandlerStateSource, SmithayLinuxBindHandlerStateState,
        SmithayLinuxBindHandlerStateSyntheticId, SmithayLinuxDisplayHandleAccessBlocker,
        SmithayLinuxDisplayHandleAccessPolicy, SmithayLinuxDisplayHandleInternalAccessBlocker,
        SmithayLinuxDisplayHandleInternalAccessDecision,
        SmithayLinuxDisplayHandleInternalAccessPrecondition,
        SmithayLinuxDisplayHandleInternalAccessPreconditionState,
        SmithayLinuxDisplayHandleInternalAccessTarget,
        SmithayLinuxDisplayHandleInternalOwnershipDecision,
        SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource,
        SmithayLinuxDisplayHandleInternalOwnershipEvidenceState,
        SmithayLinuxDisplayHandleInternalOwnershipLimitation,
        SmithayLinuxDisplayHandlePublicApiEvidenceLimitation,
        SmithayLinuxDisplayHandlePublicApiEvidenceState,
        SmithayLinuxDisplayHandlePublicApiExposureDecision,
        SmithayLinuxDisplayHandlePublicApiSurface, SmithayLinuxDisplayHandleRedaction,
        SmithayLinuxGlobalDispatchBindBlocker, SmithayLinuxGlobalDispatchBindFinalBlocker,
        SmithayLinuxGlobalDispatchBindInput, SmithayLinuxGlobalDispatchBindReadiness,
        SmithayLinuxGlobalDispatchBindSealedInputState, SmithayLinuxHandlerProbeBlocker,
        SmithayLinuxHandlerProbeKind, SmithayLinuxHandlerReductionCandidate,
        SmithayLinuxHandlerReductionDecision, SmithayLinuxHandlerReductionRisk,
        SmithayLinuxHandlerRequirement, SmithayLinuxHandlerRequirementEvidence,
        SmithayLinuxHandlerRequirementState, SmithayLinuxInertHandlerProbe,
        smithay_linux_bind_client_identity_report, smithay_linux_bind_global_data_report,
        smithay_linux_bind_global_resource_identity_report,
        smithay_linux_bind_handler_state_report, smithay_linux_display_handle_access_report,
        smithay_linux_display_handle_internal_access_gate_report,
        smithay_linux_display_handle_internal_ownership_evidence_report,
        smithay_linux_display_handle_public_api_evidence_report,
        smithay_linux_global_dispatch_bind_final_seal_report,
        smithay_linux_global_dispatch_bind_shape_report, smithay_linux_handler_probe_report,
        smithay_linux_handler_reduction_plan_report,
        smithay_linux_handler_requirement_matrix_report,
    };
    use crate::smithay_backend::{
        linux_adapter::{
            SmithayLinuxAdapterGlobalHandlerBlocker, SmithayLinuxAdapterGlobalHandlerKind,
            SmithayLinuxAdapterGlobalHandlerReadiness,
            SmithayLinuxAdapterRealGlobalRegistrationMode, SmithayLinuxAdapterSkeleton,
        },
        runtime_facade::{BackendBootstrapMode, BackendRuntimeReport},
        test_support::{assert_runtime_dir, unique_socket_name},
    };

    #[test]
    fn type_shape_and_report_remain_inert() {
        let probe = SmithayLinuxInertHandlerProbe::new();
        let copied_probe = probe;
        let default_probe = SmithayLinuxInertHandlerProbe::default();
        let report = smithay_linux_handler_probe_report();

        assert_eq!(probe, copied_probe);
        assert_eq!(probe, default_probe);
        assert_eq!(report.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(report.compiled_type_shape);
        assert!(!report.compiled_trait_shape);
        assert!(!report.uses_delegate_macros);
        assert!(!report.calls_create_global);
        assert!(!report.calls_register_global);
        assert!(!report.touches_adapter);
        assert!(!report.touches_core);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::AdapterIntegrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::CreateGlobalStillRequiredForRuntimeUse)
        );
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeClientBind)
        );
        assert!(
            report
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::TraitImplWouldExposeRequestDispatch)
        );
    }

    #[test]
    fn requirement_matrix_has_stable_complete_order() {
        use SmithayLinuxAdapterGlobalHandlerKind as Handler;
        use SmithayLinuxHandlerRequirement as Requirement;
        use SmithayLinuxHandlerRequirementState as State;

        let report = smithay_linux_handler_requirement_matrix_report();
        let actual = report
            .items
            .iter()
            .map(|item| (item.handler, item.requirement, item.state))
            .collect::<Vec<_>>();
        let expected = vec![
            (
                Handler::CompositorGlobalHandler,
                Requirement::GlobalDispatchBind,
                State::Missing,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::DispatchRequest,
                State::Missing,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::CompositorHandler,
                State::Missing,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::SurfaceRequestHandling,
                State::Blocked,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::ClientObjectVisibility,
                State::Blocked,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::ProtocolResourceTracking,
                State::Blocked,
            ),
            (
                Handler::CompositorGlobalHandler,
                Requirement::CoreAdmissionMapping,
                State::Blocked,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::GlobalDispatchBind,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::DispatchRequest,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::BufferHandler,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::ShmHandler,
                State::Missing,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::ClientObjectVisibility,
                State::Blocked,
            ),
            (
                Handler::ShmGlobalHandler,
                Requirement::ProtocolResourceTracking,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::GlobalDispatchBind,
                State::Missing,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::DispatchRequest,
                State::Missing,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::XdgShellHandler,
                State::Missing,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::XdgSurfaceRequestHandling,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::ClientObjectVisibility,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::ProtocolResourceTracking,
                State::Blocked,
            ),
            (
                Handler::XdgWmBaseGlobalHandler,
                Requirement::CoreAdmissionMapping,
                State::Blocked,
            ),
        ];

        assert_eq!(actual, expected);
        assert!(report.skeleton_only);
        assert!(!report.items.is_empty());
        assert_eq!(report.ready_count, 0);
        assert_eq!(
            report.missing_count + report.blocked_count,
            report.items.len()
        );
        assert_eq!(report.missing_count, 10);
        assert_eq!(report.blocked_count, 10);
        assert!(report.items.iter().all(|item| {
            item.skeleton_only
                && !item.evidence.is_empty()
                && item.state != SmithayLinuxHandlerRequirementState::NotAttempted
        }));
    }

    #[test]
    fn requirement_matrix_evidence_matches_requirement_semantics() {
        use SmithayLinuxHandlerRequirement as Requirement;
        use SmithayLinuxHandlerRequirementEvidence as Evidence;

        let report = smithay_linux_handler_requirement_matrix_report();
        let evidence_for = |requirement| {
            report
                .items
                .iter()
                .filter(|item| item.requirement == requirement)
                .flat_map(|item| item.evidence.iter().copied())
                .collect::<Vec<_>>()
        };

        assert!(
            evidence_for(Requirement::GlobalDispatchBind)
                .contains(&Evidence::RequiresClientBindEntry)
        );
        assert!(
            evidence_for(Requirement::DispatchRequest)
                .contains(&Evidence::RequiresProtocolRequestEntry)
        );
        assert!(
            evidence_for(Requirement::SurfaceRequestHandling)
                .contains(&Evidence::RequiresSurfaceLifecycleSupport)
        );
        assert!(
            evidence_for(Requirement::BufferHandler)
                .contains(&Evidence::RequiresBufferLifecycleSupport)
        );
        assert!(
            evidence_for(Requirement::ShmHandler).contains(&Evidence::RequiresShmCallbackSupport)
        );
        assert!(
            evidence_for(Requirement::XdgShellHandler)
                .contains(&Evidence::RequiresXdgShellCallbackSupport)
        );
        assert!(
            evidence_for(Requirement::ClientObjectVisibility)
                .contains(&Evidence::RequiresClientIdentityMapping)
        );
        assert!(
            evidence_for(Requirement::ProtocolResourceTracking)
                .contains(&Evidence::RequiresResourceTracking)
        );
        assert!(
            evidence_for(Requirement::CoreAdmissionMapping)
                .contains(&Evidence::RequiresCoreWindowAdmission)
        );
        assert!(report.items.iter().all(|item| {
            item.evidence.contains(&Evidence::BlockedByActivationGate)
                && item
                    .evidence
                    .contains(&Evidence::BlockedBySkeletonOnlyPolicy)
        }));
    }

    #[test]
    fn reduction_plan_selects_only_common_bind_shape_in_stable_order() {
        use SmithayLinuxHandlerReductionCandidate as Candidate;
        use SmithayLinuxHandlerReductionDecision as Decision;
        use SmithayLinuxHandlerReductionRisk as Risk;
        use SmithayLinuxHandlerRequirement as Requirement;

        let report = smithay_linux_handler_reduction_plan_report();
        let actual_order = report
            .candidates
            .iter()
            .map(|candidate| candidate.candidate)
            .collect::<Vec<_>>();

        assert_eq!(
            actual_order,
            vec![
                Candidate::GlobalDispatchBindShape,
                Candidate::DispatchRequestShape,
                Candidate::CompositorHandlerShape,
                Candidate::ShmHandlerShape,
                Candidate::XdgShellHandlerShape,
                Candidate::ClientIdentityModel,
                Candidate::ProtocolResourceModel,
                Candidate::SurfaceLifecycleBridge,
                Candidate::CoreAdmissionBridge,
            ]
        );
        assert!(report.skeleton_only);
        assert!(!report.candidates.is_empty());
        assert_eq!(report.selected_count, 1);
        assert_eq!(report.deferred_count, 6);
        assert_eq!(report.blocked_count, 2);
        assert_eq!(
            report.selected_first,
            Some(Candidate::GlobalDispatchBindShape)
        );
        assert_eq!(
            report.selected_count + report.deferred_count + report.blocked_count,
            report.candidates.len()
        );

        let bind = reduction_candidate_report(&report, Candidate::GlobalDispatchBindShape);
        assert_eq!(bind.decision, Decision::SelectedFirst);
        assert!(
            bind.related_requirements
                .contains(&Requirement::GlobalDispatchBind)
        );
        assert!(bind.risks.contains(&Risk::IntroducesClientBindEntry));
        assert!(bind.risks.contains(&Risk::MustRemainIsolated));

        let dispatch = reduction_candidate_report(&report, Candidate::DispatchRequestShape);
        assert_ne!(dispatch.decision, Decision::SelectedFirst);
        assert!(
            dispatch
                .related_requirements
                .contains(&Requirement::DispatchRequest)
        );
        assert!(
            dispatch
                .risks
                .contains(&Risk::IntroducesRequestDispatchEntry)
        );

        let surface = reduction_candidate_report(&report, Candidate::SurfaceLifecycleBridge);
        assert_eq!(surface.decision, Decision::Blocked);
        assert!(
            surface
                .related_requirements
                .contains(&Requirement::SurfaceRequestHandling)
        );
        assert!(
            surface
                .related_requirements
                .contains(&Requirement::XdgSurfaceRequestHandling)
        );
        assert!(surface.risks.contains(&Risk::RequiresSurfaceLifecycle));

        let core = reduction_candidate_report(&report, Candidate::CoreAdmissionBridge);
        assert_eq!(core.decision, Decision::Blocked);
        assert!(
            core.related_requirements
                .contains(&Requirement::CoreAdmissionMapping)
        );
        assert!(core.risks.contains(&Risk::RequiresCoreAdmission));

        assert!(report.candidates.iter().all(|candidate| {
            candidate.skeleton_only
                && !candidate.related_requirements.is_empty()
                && !candidate.risks.is_empty()
                && !candidate.rationale.is_empty()
        }));
    }

    #[test]
    fn reduction_plan_remains_aligned_with_matrix_and_probe() {
        let plan = smithay_linux_handler_reduction_plan_report();
        let matrix = smithay_linux_handler_requirement_matrix_report();
        let probe = smithay_linux_handler_probe_report();
        let selected = plan
            .selected_first
            .expect("reduction plan 必须选择唯一 first target");
        let selected_report = reduction_candidate_report(&plan, selected);

        assert_eq!(
            selected,
            SmithayLinuxHandlerReductionCandidate::GlobalDispatchBindShape
        );
        assert!(plan.candidates.iter().all(|candidate| {
            candidate.related_requirements.iter().all(|requirement| {
                matrix
                    .items
                    .iter()
                    .any(|item| item.requirement == *requirement)
            })
        }));
        assert!(
            selected_report
                .related_requirements
                .iter()
                .all(|requirement| matrix
                    .items
                    .iter()
                    .any(|item| item.requirement == *requirement))
        );
        assert_eq!(matrix.ready_count, 0);
        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);
        assert!(!probe.calls_create_global);
        assert!(!probe.calls_register_global);
        assert!(!probe.touches_adapter);
        assert!(!probe.touches_core);
    }

    #[test]
    fn global_dispatch_bind_shape_is_stable_and_fully_blocked() {
        use SmithayLinuxGlobalDispatchBindBlocker as Blocker;
        use SmithayLinuxGlobalDispatchBindInput as Input;

        let report = smithay_linux_global_dispatch_bind_shape_report();
        let actual_order = report
            .items
            .iter()
            .map(|item| item.input)
            .collect::<Vec<_>>();

        assert!(report.skeleton_only);
        assert_eq!(
            actual_order,
            vec![
                Input::ClientObject,
                Input::GlobalResourceObject,
                Input::GlobalDataObject,
                Input::DisplayHandleObject,
                Input::HandlerState,
            ]
        );
        assert_eq!(report.items.len(), 5);
        assert_eq!(report.modeled_count, 4);
        assert_eq!(report.blocked_count, 5);
        assert!(!report.can_compile_trait_impl);
        assert!(!report.can_attach_to_adapter);
        assert!(!report.can_register_global);
        assert!(
            report
                .items
                .iter()
                .all(|item| !item.blockers.is_empty() && item.skeleton_only)
        );

        let client = bind_shape_item(&report, Input::ClientObject);
        assert!(client.modeled);
        assert!(!client.blockers.contains(&Blocker::ClientObjectNotModeled));
        assert!(
            client
                .blockers
                .contains(&Blocker::WouldCreateClientBindEntry)
        );
        assert!(
            client
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        let resource = bind_shape_item(&report, Input::GlobalResourceObject);
        assert!(resource.modeled);
        assert!(
            !resource
                .blockers
                .contains(&Blocker::ResourceObjectNotModeled)
        );
        assert!(
            resource
                .blockers
                .contains(&Blocker::WouldCreateClientBindEntry)
        );
        assert!(
            resource
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        let global_data = bind_shape_item(&report, Input::GlobalDataObject);
        assert!(global_data.modeled);
        assert!(
            !global_data
                .blockers
                .contains(&Blocker::GlobalDataNotModeled)
        );
        assert!(
            global_data
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        assert!(
            !bind_shape_item(&report, Input::DisplayHandleObject).modeled
                && bind_shape_item(&report, Input::DisplayHandleObject)
                    .blockers
                    .contains(&Blocker::DisplayHandleMustRemainHidden)
        );
        let handler_state = bind_shape_item(&report, Input::HandlerState);
        assert!(handler_state.modeled);
        assert!(
            !handler_state
                .blockers
                .contains(&Blocker::HandlerStateNotIntegrated)
        );
        assert!(
            handler_state
                .blockers
                .contains(&Blocker::WouldCreateClientBindEntry)
        );
        assert!(
            handler_state
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        assert!(
            handler_state
                .blockers
                .contains(&Blocker::AdapterIntegrationForbidden)
        );
        assert!(report.items.iter().all(|item| {
            item.blockers
                .contains(&Blocker::AdapterIntegrationForbidden)
        }));
    }

    #[test]
    fn bind_client_identity_is_stable_synthetic_data() {
        use SmithayLinuxBindClientIdentityBlocker as Blocker;

        let zero = SmithayLinuxBindClientSyntheticId::new(0);
        let report = smithay_linux_bind_client_identity_report();

        assert_eq!(zero.value(), 0);
        assert_eq!(
            report.source,
            SmithayLinuxBindClientIdentitySource::SyntheticOnly
        );
        assert_eq!(
            report.state,
            SmithayLinuxBindClientIdentityState::SyntheticModeled
        );
        assert_eq!(report.synthetic_id.value(), 1);
        assert!(!report.represents_real_client);
        assert!(!report.accepts_client);
        assert!(!report.touches_socket);
        assert!(!report.touches_adapter);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&Blocker::RealClientObjectUnavailable)
        );
        assert!(report.blockers.contains(&Blocker::ClientAcceptForbidden));
        assert!(
            report
                .blockers
                .contains(&Blocker::SocketClientHandlingForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::AdapterIntegrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::GlobalDispatchBindStillForbidden)
        );
    }

    #[test]
    fn bind_global_resource_identity_is_stable_synthetic_data() {
        use SmithayLinuxBindGlobalResourceIdentityBlocker as Blocker;

        let zero = SmithayLinuxBindGlobalResourceSyntheticId::new(0);
        let report = smithay_linux_bind_global_resource_identity_report();

        assert_eq!(zero.value(), 0);
        assert_eq!(
            report.source,
            SmithayLinuxBindGlobalResourceIdentitySource::SyntheticOnly
        );
        assert_eq!(
            report.state,
            SmithayLinuxBindGlobalResourceIdentityState::SyntheticModeled
        );
        assert_eq!(report.synthetic_id.value(), 1);
        assert!(!report.represents_real_resource);
        assert!(!report.comes_from_real_global_registration);
        assert!(!report.tracks_protocol_resource);
        assert!(!report.touches_adapter);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&Blocker::RealResourceObjectUnavailable)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::RealGlobalRegistrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::ProtocolResourceTrackingUnavailable)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::AdapterIntegrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::GlobalDispatchBindStillForbidden)
        );
    }

    #[test]
    fn bind_global_data_is_stable_synthetic_data() {
        use SmithayLinuxBindGlobalDataBlocker as Blocker;

        let zero = SmithayLinuxBindGlobalDataSyntheticId::new(0);
        let report = smithay_linux_bind_global_data_report();

        assert_eq!(zero.value(), 0);
        assert_eq!(
            report.source,
            SmithayLinuxBindGlobalDataSource::SyntheticOnly
        );
        assert_eq!(
            report.state,
            SmithayLinuxBindGlobalDataState::SyntheticModeled
        );
        assert_eq!(report.synthetic_id.value(), 1);
        assert!(!report.represents_real_global_data);
        assert!(!report.comes_from_real_global_registration);
        assert!(!report.tracks_protocol_global);
        assert!(!report.touches_adapter);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&Blocker::RealGlobalDataUnavailable)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::RealGlobalRegistrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::ProtocolGlobalObjectUnavailable)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::AdapterIntegrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::GlobalDispatchBindStillForbidden)
        );
    }

    #[test]
    fn bind_handler_state_is_stable_synthetic_data() {
        use SmithayLinuxBindHandlerStateBlocker as Blocker;

        let zero = SmithayLinuxBindHandlerStateSyntheticId::new(0);
        let report = smithay_linux_bind_handler_state_report();

        assert_eq!(zero.value(), 0);
        assert_eq!(
            report.source,
            SmithayLinuxBindHandlerStateSource::SyntheticOnly
        );
        assert_eq!(
            report.state,
            SmithayLinuxBindHandlerStateState::SyntheticModeled
        );
        assert_eq!(report.synthetic_id.value(), 1);
        assert!(!report.represents_real_handler_state);
        assert!(!report.touches_adapter);
        assert!(!report.touches_dispatch_state);
        assert!(!report.touches_display_handle);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&Blocker::RealSmithayHandlerStateUnavailable)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::AdapterStateIntegrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&Blocker::ProtocolDispatchStateUnavailable)
        );
        assert!(report.blockers.contains(&Blocker::DisplayHandleStillHidden));
        assert!(
            report
                .blockers
                .contains(&Blocker::GlobalDispatchBindStillForbidden)
        );
    }

    #[test]
    fn display_handle_access_policy_is_fully_redacted() {
        use SmithayLinuxDisplayHandleAccessBlocker as AccessBlocker;
        use SmithayLinuxGlobalDispatchBindBlocker as BindBlocker;
        use SmithayLinuxGlobalDispatchBindInput as Input;

        let report = smithay_linux_display_handle_access_report();

        assert_eq!(report.policy, SmithayLinuxDisplayHandleAccessPolicy::Hidden);
        assert_eq!(
            report.redaction,
            SmithayLinuxDisplayHandleRedaction::FullyRedacted
        );
        assert!(!report.represents_real_display_handle);
        assert!(!report.exposes_display_handle);
        assert!(!report.stores_display_handle);
        assert!(!report.reads_display_handle);
        assert!(!report.can_call_create_global);
        assert!(!report.can_call_register_global);
        assert!(!report.touches_adapter_public_api);
        assert!(report.skeleton_only);
        assert!(!report.blockers.is_empty());
        assert!(
            report
                .blockers
                .contains(&AccessBlocker::RealDisplayHandleMustRemainPrivate)
        );
        assert!(
            report
                .blockers
                .contains(&AccessBlocker::DisplayHandleExposureForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&AccessBlocker::DisplayHandleStorageForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&AccessBlocker::GlobalRegistrationForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&AccessBlocker::AdapterPublicApiExposureForbidden)
        );
        assert!(
            report
                .blockers
                .contains(&AccessBlocker::GlobalDispatchBindStillForbidden)
        );

        let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
        assert!(bind_shape_item(&bind_shape, Input::ClientObject).modeled);
        assert!(bind_shape_item(&bind_shape, Input::GlobalResourceObject).modeled);
        assert!(bind_shape_item(&bind_shape, Input::GlobalDataObject).modeled);
        assert!(bind_shape_item(&bind_shape, Input::HandlerState).modeled);
        let display_handle = bind_shape_item(&bind_shape, Input::DisplayHandleObject);
        assert!(!display_handle.modeled);
        assert!(
            display_handle
                .blockers
                .contains(&BindBlocker::DisplayHandleMustRemainHidden)
        );
        assert_eq!(bind_shape.modeled_count, 4);
        assert_eq!(bind_shape.blocked_count, 5);
        assert!(!bind_shape.can_compile_trait_impl);
        assert!(!bind_shape.can_attach_to_adapter);
        assert!(!bind_shape.can_register_global);
    }

    #[test]
    fn global_dispatch_bind_final_seal_remains_not_ready() {
        use SmithayLinuxGlobalDispatchBindFinalBlocker as FinalBlocker;
        use SmithayLinuxGlobalDispatchBindInput as Input;
        use SmithayLinuxGlobalDispatchBindSealedInputState as State;
        use SmithayLinuxHandlerReductionCandidate as Candidate;
        use SmithayLinuxHandlerReductionDecision as Decision;
        use SmithayLinuxHandlerReductionRisk as Risk;
        use SmithayLinuxHandlerRequirement as Requirement;
        use SmithayLinuxHandlerRequirementEvidence as Evidence;

        let seal = smithay_linux_global_dispatch_bind_final_seal_report();
        let actual_order = seal
            .inputs
            .iter()
            .map(|item| item.input)
            .collect::<Vec<_>>();

        assert_eq!(
            seal.readiness,
            SmithayLinuxGlobalDispatchBindReadiness::NotReady
        );
        assert_eq!(
            actual_order,
            vec![
                Input::ClientObject,
                Input::GlobalResourceObject,
                Input::GlobalDataObject,
                Input::DisplayHandleObject,
                Input::HandlerState,
            ]
        );
        assert_eq!(seal.inputs.len(), 5);
        assert_eq!(seal.synthetic_modeled_count, 4);
        assert_eq!(seal.hidden_redacted_count, 1);
        assert_eq!(seal.blocked_count, 5);
        assert!(!seal.can_compile_trait_impl);
        assert!(!seal.can_attach_to_adapter);
        assert!(!seal.can_register_global);
        assert!(!seal.can_dispatch_requests);
        assert!(!seal.can_create_surfaces);
        assert!(!seal.can_enter_core_admission);
        assert_eq!(
            seal.next_safe_target,
            Some("DisplayHandleInternalAccessGatePolicy")
        );
        assert!(seal.skeleton_only);
        assert!(
            seal.inputs
                .iter()
                .all(|item| !item.blockers.is_empty() && item.skeleton_only)
        );

        for input in [
            Input::ClientObject,
            Input::GlobalResourceObject,
            Input::GlobalDataObject,
            Input::HandlerState,
        ] {
            let item = final_seal_input(&seal, input);
            assert_eq!(item.state, State::SyntheticModeled);
            assert!(item.modeled);
            assert!(!item.hidden);
            assert!(!item.redacted);
        }
        let display = final_seal_input(&seal, Input::DisplayHandleObject);
        assert_eq!(display.state, State::HiddenRedacted);
        assert!(!display.modeled);
        assert!(display.hidden);
        assert!(display.redacted);
        assert!(
            display
                .blockers
                .contains(&FinalBlocker::DisplayHandleHidden)
        );
        assert!(
            display
                .blockers
                .contains(&FinalBlocker::RealDisplayHandleUnavailable)
        );
        assert!(seal.inputs.iter().any(|item| {
            item.blockers
                .contains(&FinalBlocker::GlobalRegistrationForbidden)
        }));
        assert!(seal.inputs.iter().any(|item| {
            item.blockers
                .contains(&FinalBlocker::ClientBindEntryForbidden)
        }));
        assert!(seal.inputs.iter().all(|item| {
            item.blockers
                .contains(&FinalBlocker::TraitImplementationForbidden)
        }));
        assert!(seal.inputs.iter().all(|item| {
            item.blockers
                .contains(&FinalBlocker::DispatchRequestForbidden)
        }));
        assert!(seal.inputs.iter().all(|item| {
            item.blockers
                .contains(&FinalBlocker::SurfaceLifecycleUnavailable)
        }));
        assert!(seal.inputs.iter().all(|item| {
            item.blockers
                .contains(&FinalBlocker::CoreAdmissionUnavailable)
        }));

        let shape = smithay_linux_global_dispatch_bind_shape_report();
        assert_eq!(shape.modeled_count, 4);
        assert_eq!(shape.blocked_count, 5);
        assert!(!bind_shape_item(&shape, Input::DisplayHandleObject).modeled);
        assert!(!shape.can_compile_trait_impl);
        assert!(!shape.can_attach_to_adapter);
        assert!(!shape.can_register_global);

        let display_policy = smithay_linux_display_handle_access_report();
        assert_eq!(
            display_policy.policy,
            SmithayLinuxDisplayHandleAccessPolicy::Hidden
        );
        assert_eq!(
            display_policy.redaction,
            SmithayLinuxDisplayHandleRedaction::FullyRedacted
        );
        assert!(!display_policy.represents_real_display_handle);
        assert!(!display_policy.exposes_display_handle);
        assert!(!display_policy.stores_display_handle);
        assert!(!display_policy.reads_display_handle);
        assert!(!display_policy.can_call_create_global);
        assert!(!display_policy.can_call_register_global);

        let matrix = smithay_linux_handler_requirement_matrix_report();
        let bind_requirements = matrix
            .items
            .iter()
            .filter(|item| item.requirement == Requirement::GlobalDispatchBind)
            .collect::<Vec<_>>();
        assert_eq!(matrix.ready_count, 0);
        assert_eq!(bind_requirements.len(), 3);
        assert!(
            bind_requirements
                .iter()
                .all(|item| item.evidence.contains(&Evidence::RequiresClientBindEntry))
        );

        let plan = smithay_linux_handler_reduction_plan_report();
        assert_eq!(
            plan.selected_first,
            Some(Candidate::GlobalDispatchBindShape)
        );
        assert!(
            reduction_candidate_report(&plan, Candidate::GlobalDispatchBindShape)
                .risks
                .contains(&Risk::IntroducesClientBindEntry)
        );
        assert_eq!(
            reduction_candidate_report(&plan, Candidate::SurfaceLifecycleBridge).decision,
            Decision::Blocked
        );
        assert_eq!(
            reduction_candidate_report(&plan, Candidate::CoreAdmissionBridge).decision,
            Decision::Blocked
        );

        let probe = smithay_linux_handler_probe_report();
        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);
    }

    #[test]
    fn display_handle_public_api_evidence_proves_non_exposure_only() {
        use SmithayLinuxDisplayHandleInternalAccessPrecondition as Precondition;
        use SmithayLinuxDisplayHandleInternalAccessPreconditionState as PreconditionState;
        use SmithayLinuxDisplayHandlePublicApiEvidenceLimitation as Limitation;
        use SmithayLinuxDisplayHandlePublicApiEvidenceState as EvidenceState;
        use SmithayLinuxDisplayHandlePublicApiSurface as Surface;
        use SmithayLinuxGlobalDispatchBindInput as Input;
        use SmithayLinuxHandlerReductionCandidate as Candidate;

        let evidence = smithay_linux_display_handle_public_api_evidence_report();
        let actual_order = evidence
            .items
            .iter()
            .map(|item| item.surface)
            .collect::<Vec<_>>();

        assert_eq!(
            evidence.decision,
            SmithayLinuxDisplayHandlePublicApiExposureDecision::NotExposed
        );
        assert_eq!(
            actual_order,
            vec![
                Surface::DisplayHandleReturnValue,
                Surface::DisplayHandleArgument,
                Surface::DisplayReturnValue,
                Surface::DisplayArgument,
                Surface::MutableBootstrapReturnValue,
                Surface::MutableBootstrapArgument,
                Surface::CreateGlobalEntrypoint,
                Surface::RegisterGlobalEntrypoint,
                Surface::AdapterCapabilityFlag,
            ]
        );
        assert_eq!(evidence.items.len(), 9);
        assert_eq!(evidence.absent_count, 8);
        assert_eq!(evidence.conservative_false_count, 1);
        assert_eq!(evidence.exposed_count, 0);
        assert!(evidence.can_satisfy_public_non_exposure_precondition);
        assert!(!evidence.can_read_display_handle);
        assert!(!evidence.can_store_display_handle);
        assert!(!evidence.can_expose_display_handle);
        assert!(!evidence.can_call_create_global);
        assert!(!evidence.can_call_register_global);
        assert!(!evidence.can_compile_global_dispatch);
        assert!(!evidence.can_attach_to_adapter);
        assert!(evidence.skeleton_only);
        assert!(
            evidence
                .items
                .iter()
                .all(|item| !item.limitations.is_empty() && item.skeleton_only)
        );
        for surface in [
            Surface::DisplayHandleReturnValue,
            Surface::DisplayHandleArgument,
            Surface::DisplayReturnValue,
            Surface::DisplayArgument,
            Surface::MutableBootstrapReturnValue,
            Surface::MutableBootstrapArgument,
            Surface::CreateGlobalEntrypoint,
            Surface::RegisterGlobalEntrypoint,
        ] {
            assert_eq!(
                public_api_evidence_item_by_surface(&evidence, surface).state,
                EvidenceState::Absent
            );
        }
        let capability =
            public_api_evidence_item_by_surface(&evidence, Surface::AdapterCapabilityFlag);
        assert_eq!(capability.state, EvidenceState::ConservativeFalse);
        for limitation in [
            Limitation::StaticEvidenceOnly,
            Limitation::DoesNotReadAdapterRuntimeState,
            Limitation::DoesNotProveInternalOwnership,
            Limitation::DoesNotPermitDisplayHandleAccess,
            Limitation::DoesNotPermitGlobalRegistration,
            Limitation::DoesNotPermitTraitImplementation,
        ] {
            assert!(
                evidence
                    .items
                    .iter()
                    .all(|item| item.limitations.contains(&limitation))
            );
        }

        let gate = smithay_linux_display_handle_internal_access_gate_report();
        assert_eq!(
            gate.decision,
            SmithayLinuxDisplayHandleInternalAccessDecision::Blocked
        );
        assert_eq!(gate.satisfied_count, 3);
        assert_eq!(gate.missing_count, 3);
        assert_eq!(gate.blocked_count, 2);
        assert_eq!(
            internal_access_precondition(
                &gate,
                Precondition::AdapterDoesNotExposeDisplayHandlePublicly
            )
            .state,
            PreconditionState::Satisfied
        );
        assert_eq!(
            internal_access_precondition(
                &gate,
                Precondition::DisplayHandleRedactionPolicyPreserved
            )
            .state,
            PreconditionState::Satisfied
        );
        assert!(!gate.can_read_display_handle);
        assert!(!gate.can_store_display_handle);
        assert!(!gate.can_expose_display_handle);
        assert!(!gate.can_call_create_global);
        assert!(!gate.can_call_register_global);
        assert!(!gate.can_compile_global_dispatch);
        assert!(!gate.can_attach_to_adapter);

        let display_policy = smithay_linux_display_handle_access_report();
        assert_eq!(
            display_policy.policy,
            SmithayLinuxDisplayHandleAccessPolicy::Hidden
        );
        assert_eq!(
            display_policy.redaction,
            SmithayLinuxDisplayHandleRedaction::FullyRedacted
        );
        assert!(!display_policy.represents_real_display_handle);
        assert!(!display_policy.exposes_display_handle);
        assert!(!display_policy.stores_display_handle);
        assert!(!display_policy.reads_display_handle);

        let final_seal = smithay_linux_global_dispatch_bind_final_seal_report();
        assert_eq!(
            final_seal.readiness,
            SmithayLinuxGlobalDispatchBindReadiness::NotReady
        );
        assert!(!final_seal.can_compile_trait_impl);
        assert!(!final_seal.can_register_global);
        let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
        assert_eq!(bind_shape.modeled_count, 4);
        assert_eq!(bind_shape.blocked_count, 5);
        assert!(!bind_shape_item(&bind_shape, Input::DisplayHandleObject).modeled);
        let matrix = smithay_linux_handler_requirement_matrix_report();
        assert_eq!(matrix.ready_count, 0);
        let plan = smithay_linux_handler_reduction_plan_report();
        assert_eq!(
            plan.selected_first,
            Some(Candidate::GlobalDispatchBindShape)
        );
        let probe = smithay_linux_handler_probe_report();
        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);

        let adapter_source = include_str!("linux_adapter.rs");
        let adapter_production = adapter_source
            .split("#[cfg(test)]")
            .next()
            .expect("adapter source 应包含生产代码");
        for forbidden_public_surface in [
            "pub fn display_handle",
            "pub fn display(",
            "-> DisplayHandle",
            ": DisplayHandle",
            "-> Display<",
            ": Display<",
            "pub fn bootstrap_mut",
            "-> &mut SmithayBootstrapProbe",
            ": &mut SmithayBootstrapProbe",
            "pub fn create_global",
            "pub fn register_global",
        ] {
            assert!(
                !adapter_production.contains(forbidden_public_surface),
                "adapter production public API 不得暴露 {forbidden_public_surface}"
            );
        }

        assert_runtime_dir();
        let socket_name = unique_socket_name("phase49n-public-api-evidence");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("Phase 49N 边界测试应能构造 adapter skeleton");
        let registration = adapter.attempt_real_global_registration_feasibility();
        assert_eq!(
            registration.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        assert_eq!(adapter.global_handler_boundary_report().ready_count, 0);
        let capabilities = adapter.capabilities();
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
    }

    #[test]
    fn display_handle_internal_ownership_evidence_proves_private_holder_only() {
        use SmithayLinuxDisplayHandleInternalAccessPrecondition as Precondition;
        use SmithayLinuxDisplayHandleInternalAccessPreconditionState as PreconditionState;
        use SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource as Source;
        use SmithayLinuxDisplayHandleInternalOwnershipEvidenceState as EvidenceState;
        use SmithayLinuxDisplayHandleInternalOwnershipLimitation as Limitation;
        use SmithayLinuxGlobalDispatchBindInput as Input;
        use SmithayLinuxHandlerReductionCandidate as Candidate;

        let evidence = smithay_linux_display_handle_internal_ownership_evidence_report();
        let actual_order = evidence
            .items
            .iter()
            .map(|item| item.source)
            .collect::<Vec<_>>();

        assert_eq!(
            evidence.decision,
            SmithayLinuxDisplayHandleInternalOwnershipDecision::
                StaticPrivateOwnershipEvidencePresent
        );
        assert_eq!(
            actual_order,
            vec![
                Source::BootstrapBoundary,
                Source::LinuxRuntimeBoundary,
                Source::LinuxAdapterPublicApiAudit,
            ]
        );
        assert_eq!(evidence.items.len(), 3);
        assert_eq!(evidence.present_private_count, 2);
        assert_eq!(evidence.absent_public_count, 1);
        assert_eq!(evidence.conservative_false_count, 0);
        assert_eq!(evidence.missing_count, 0);
        match evidence.decision {
            SmithayLinuxDisplayHandleInternalOwnershipDecision::
                StaticPrivateOwnershipEvidencePresent => {
                    assert!(evidence.can_satisfy_internal_ownership_precondition);
                }
            SmithayLinuxDisplayHandleInternalOwnershipDecision::StaticEvidenceInsufficient => {
                assert!(!evidence.can_satisfy_internal_ownership_precondition);
            }
        }
        assert!(!evidence.can_read_display_handle);
        assert!(!evidence.can_store_display_handle);
        assert!(!evidence.can_expose_display_handle);
        assert!(!evidence.can_call_create_global);
        assert!(!evidence.can_call_register_global);
        assert!(!evidence.can_compile_global_dispatch);
        assert!(!evidence.can_attach_to_adapter);
        assert!(evidence.skeleton_only);
        assert_eq!(
            internal_ownership_evidence_item_by_source(&evidence, Source::BootstrapBoundary).state,
            EvidenceState::PresentPrivate
        );
        assert_eq!(
            internal_ownership_evidence_item_by_source(&evidence, Source::LinuxRuntimeBoundary)
                .state,
            EvidenceState::PresentPrivate
        );
        assert_eq!(
            internal_ownership_evidence_item_by_source(
                &evidence,
                Source::LinuxAdapterPublicApiAudit
            )
            .state,
            EvidenceState::AbsentPublic
        );
        for limitation in [
            Limitation::StaticEvidenceOnly,
            Limitation::DoesNotReadDisplayHandle,
            Limitation::DoesNotProveAccessSafety,
            Limitation::DoesNotPermitGlobalRegistration,
            Limitation::DoesNotPermitTraitImplementation,
            Limitation::DoesNotPermitAdapterIntegration,
        ] {
            assert!(
                evidence
                    .items
                    .iter()
                    .all(|item| item.limitations.contains(&limitation) && item.skeleton_only)
            );
        }

        let bootstrap_source = include_str!("bootstrap.rs");
        let bootstrap_production = bootstrap_source
            .split("#[cfg(test)]")
            .next()
            .expect("bootstrap source 应包含生产代码");
        assert!(bootstrap_production.contains("display: SmithayWaylandDisplayProbe"));
        assert!(!bootstrap_production.contains("pub display: SmithayWaylandDisplayProbe"));
        assert!(bootstrap_production.contains("pub fn display_handle"));

        let runtime_source = include_str!("linux_runtime.rs");
        let runtime_production = runtime_source
            .split("#[cfg(test)]")
            .next()
            .expect("linux runtime source 应包含生产代码");
        assert!(runtime_production.contains("bootstrap: SmithayBootstrapProbe"));
        assert!(!runtime_production.contains("pub bootstrap: SmithayBootstrapProbe"));
        assert!(!runtime_production.contains("pub fn bootstrap(&"));
        assert!(!runtime_production.contains("pub fn bootstrap_mut"));

        let adapter_source = include_str!("linux_adapter.rs");
        let adapter_production = adapter_source
            .split("#[cfg(test)]")
            .next()
            .expect("adapter source 应包含生产代码");
        assert!(adapter_production.contains("bootstrap: SmithayBootstrapProbe"));
        assert!(!adapter_production.contains("pub bootstrap: SmithayBootstrapProbe"));
        for forbidden_public_surface in [
            "pub fn display_handle",
            "pub fn display(",
            "-> DisplayHandle",
            ": DisplayHandle",
            "-> Display<",
            ": Display<",
            "pub fn bootstrap(",
            "pub fn bootstrap_mut",
            "-> &mut SmithayBootstrapProbe",
            ": &mut SmithayBootstrapProbe",
            "pub fn create_global",
            "pub fn register_global",
        ] {
            assert!(
                !adapter_production.contains(forbidden_public_surface),
                "adapter production public API 不得暴露 {forbidden_public_surface}"
            );
        }

        let public_api_evidence = smithay_linux_display_handle_public_api_evidence_report();
        assert_eq!(
            public_api_evidence.decision,
            SmithayLinuxDisplayHandlePublicApiExposureDecision::NotExposed
        );
        assert_eq!(public_api_evidence.exposed_count, 0);
        assert_eq!(public_api_evidence.conservative_false_count, 1);
        assert!(public_api_evidence.can_satisfy_public_non_exposure_precondition);

        let gate = smithay_linux_display_handle_internal_access_gate_report();
        assert_eq!(
            gate.decision,
            SmithayLinuxDisplayHandleInternalAccessDecision::Blocked
        );
        assert_eq!(gate.satisfied_count, 3);
        assert_eq!(gate.missing_count, 3);
        assert_eq!(gate.blocked_count, 2);
        assert_eq!(
            internal_access_precondition(&gate, Precondition::AdapterOwnsDisplayHandleInternally)
                .state,
            PreconditionState::Satisfied
        );
        assert!(!gate.can_read_display_handle);
        assert!(!gate.can_store_display_handle);
        assert!(!gate.can_expose_display_handle);
        assert!(!gate.can_call_create_global);
        assert!(!gate.can_call_register_global);
        assert!(!gate.can_compile_global_dispatch);
        assert!(!gate.can_dispatch_requests);
        assert!(!gate.can_attach_to_adapter);
        assert!(gate.skeleton_only);

        let display_policy = smithay_linux_display_handle_access_report();
        assert_eq!(
            display_policy.policy,
            SmithayLinuxDisplayHandleAccessPolicy::Hidden
        );
        assert_eq!(
            display_policy.redaction,
            SmithayLinuxDisplayHandleRedaction::FullyRedacted
        );
        let final_seal = smithay_linux_global_dispatch_bind_final_seal_report();
        assert_eq!(
            final_seal.readiness,
            SmithayLinuxGlobalDispatchBindReadiness::NotReady
        );
        assert!(!final_seal.can_compile_trait_impl);
        assert!(!final_seal.can_attach_to_adapter);
        assert!(!final_seal.can_register_global);
        let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
        assert!(!bind_shape_item(&bind_shape, Input::DisplayHandleObject).modeled);
        assert_eq!(bind_shape.modeled_count, 4);
        assert_eq!(bind_shape.blocked_count, 5);
        let plan = smithay_linux_handler_reduction_plan_report();
        assert_eq!(
            plan.selected_first,
            Some(Candidate::GlobalDispatchBindShape)
        );
        let matrix = smithay_linux_handler_requirement_matrix_report();
        assert_eq!(matrix.ready_count, 0);
        let probe = smithay_linux_handler_probe_report();
        assert!(!probe.compiled_trait_shape);

        assert_runtime_dir();
        let socket_name = unique_socket_name("phase49o-internal-ownership");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("Phase 49O 边界测试应能构造 adapter skeleton");
        let registration = adapter.attempt_real_global_registration_feasibility();
        assert_eq!(
            registration.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        assert_eq!(adapter.global_handler_boundary_report().ready_count, 0);
        let capabilities = adapter.capabilities();
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
    }

    #[test]
    fn display_handle_internal_access_gate_remains_blocked() {
        use SmithayLinuxDisplayHandleInternalAccessBlocker as Blocker;
        use SmithayLinuxDisplayHandleInternalAccessPrecondition as Precondition;
        use SmithayLinuxDisplayHandleInternalAccessPreconditionState as State;
        use SmithayLinuxGlobalDispatchBindInput as Input;
        use SmithayLinuxHandlerReductionCandidate as Candidate;
        use SmithayLinuxHandlerReductionDecision as Decision;
        use SmithayLinuxHandlerReductionRisk as Risk;

        let gate = smithay_linux_display_handle_internal_access_gate_report();
        let actual_order = gate
            .preconditions
            .iter()
            .map(|item| item.precondition)
            .collect::<Vec<_>>();

        assert_eq!(
            gate.target,
            SmithayLinuxDisplayHandleInternalAccessTarget::FutureGlobalRegistration
        );
        assert_eq!(
            gate.decision,
            SmithayLinuxDisplayHandleInternalAccessDecision::Blocked
        );
        assert_eq!(
            actual_order,
            vec![
                Precondition::AdapterOwnsDisplayHandleInternally,
                Precondition::AdapterDoesNotExposeDisplayHandlePublicly,
                Precondition::ActivationGateAllowsRealProtocolGlobalRegistration,
                Precondition::GlobalRegistrationPlanPromotedFromSkeleton,
                Precondition::GlobalDispatchTraitBoundaryCompiled,
                Precondition::DispatchRequestBoundaryDefined,
                Precondition::HandlerStateIntegratedInternally,
                Precondition::DisplayHandleRedactionPolicyPreserved,
            ]
        );
        assert!(!gate.preconditions.is_empty());
        assert_eq!(gate.satisfied_count, 3);
        assert_eq!(gate.missing_count, 3);
        assert_eq!(gate.blocked_count, 2);
        assert_eq!(
            gate.satisfied_count + gate.missing_count + gate.blocked_count,
            gate.preconditions.len()
        );
        assert!(!gate.can_read_display_handle);
        assert!(!gate.can_store_display_handle);
        assert!(!gate.can_expose_display_handle);
        assert!(!gate.can_call_create_global);
        assert!(!gate.can_call_register_global);
        assert!(!gate.can_compile_global_dispatch);
        assert!(!gate.can_dispatch_requests);
        assert!(!gate.can_attach_to_adapter);
        assert!(gate.skeleton_only);
        assert!(gate.preconditions.iter().all(|item| {
            item.skeleton_only && (item.state == State::Satisfied || !item.blockers.is_empty())
        }));
        assert_eq!(
            internal_access_precondition(&gate, Precondition::AdapterOwnsDisplayHandleInternally)
                .state,
            State::Satisfied
        );
        assert_eq!(
            internal_access_precondition(
                &gate,
                Precondition::AdapterDoesNotExposeDisplayHandlePublicly
            )
            .state,
            State::Satisfied
        );
        for precondition in [
            Precondition::ActivationGateAllowsRealProtocolGlobalRegistration,
            Precondition::GlobalRegistrationPlanPromotedFromSkeleton,
            Precondition::GlobalDispatchTraitBoundaryCompiled,
            Precondition::DispatchRequestBoundaryDefined,
            Precondition::HandlerStateIntegratedInternally,
        ] {
            assert_ne!(
                internal_access_precondition(&gate, precondition).state,
                State::Satisfied
            );
        }
        assert_eq!(
            internal_access_precondition(
                &gate,
                Precondition::DisplayHandleRedactionPolicyPreserved
            )
            .state,
            State::Satisfied
        );
        let all_blockers = gate
            .preconditions
            .iter()
            .flat_map(|item| item.blockers.iter().copied())
            .collect::<Vec<_>>();
        for blocker in [
            Blocker::ActivationGateBlocked,
            Blocker::GlobalRegistrationForbidden,
            Blocker::GlobalDispatchTraitMissing,
            Blocker::DispatchRequestBoundaryMissing,
            Blocker::HandlerStateOnlySynthetic,
            Blocker::AdapterIntegrationForbidden,
        ] {
            assert!(all_blockers.contains(&blocker));
        }

        let display_policy = smithay_linux_display_handle_access_report();
        assert_eq!(
            display_policy.policy,
            SmithayLinuxDisplayHandleAccessPolicy::Hidden
        );
        assert_eq!(
            display_policy.redaction,
            SmithayLinuxDisplayHandleRedaction::FullyRedacted
        );
        assert!(!display_policy.represents_real_display_handle);
        assert!(!display_policy.exposes_display_handle);
        assert!(!display_policy.stores_display_handle);
        assert!(!display_policy.reads_display_handle);
        assert!(!display_policy.can_call_create_global);
        assert!(!display_policy.can_call_register_global);

        let final_seal = smithay_linux_global_dispatch_bind_final_seal_report();
        assert_eq!(
            final_seal.readiness,
            SmithayLinuxGlobalDispatchBindReadiness::NotReady
        );
        assert_eq!(
            final_seal.next_safe_target,
            Some("DisplayHandleInternalAccessGatePolicy")
        );
        assert!(!final_seal.can_compile_trait_impl);
        assert!(!final_seal.can_register_global);
        assert!(!final_seal.can_attach_to_adapter);

        let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
        assert_eq!(bind_shape.modeled_count, 4);
        assert_eq!(bind_shape.blocked_count, 5);
        assert!(!bind_shape_item(&bind_shape, Input::DisplayHandleObject).modeled);

        let matrix = smithay_linux_handler_requirement_matrix_report();
        assert_eq!(matrix.ready_count, 0);
        assert!(matrix.items.iter().any(|item| {
            item.requirement == SmithayLinuxHandlerRequirement::GlobalDispatchBind
        }));

        let plan = smithay_linux_handler_reduction_plan_report();
        assert_eq!(
            plan.selected_first,
            Some(Candidate::GlobalDispatchBindShape)
        );
        assert!(
            reduction_candidate_report(&plan, Candidate::GlobalDispatchBindShape)
                .risks
                .contains(&Risk::IntroducesClientBindEntry)
        );
        assert_eq!(
            reduction_candidate_report(&plan, Candidate::SurfaceLifecycleBridge).decision,
            Decision::Blocked
        );
        assert_eq!(
            reduction_candidate_report(&plan, Candidate::CoreAdmissionBridge).decision,
            Decision::Blocked
        );

        let probe = smithay_linux_handler_probe_report();
        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);

        assert_runtime_dir();
        let socket_name = unique_socket_name("phase49m-internal-gate");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("Phase 49M 边界测试应能构造 adapter skeleton");
        let registration = adapter.attempt_real_global_registration_feasibility();
        assert_eq!(
            registration.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        let handler_boundary = adapter.global_handler_boundary_report();
        assert_eq!(handler_boundary.ready_count, 0);
        let capabilities = adapter.capabilities();
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
    }

    #[test]
    fn global_dispatch_bind_shape_aligns_with_reduction_matrix_and_probe() {
        use SmithayLinuxGlobalDispatchBindBlocker as Blocker;
        use SmithayLinuxGlobalDispatchBindInput as Input;
        use SmithayLinuxHandlerReductionCandidate as Candidate;
        use SmithayLinuxHandlerReductionRisk as Risk;
        use SmithayLinuxHandlerRequirement as Requirement;
        use SmithayLinuxHandlerRequirementEvidence as Evidence;

        let bind_shape = smithay_linux_global_dispatch_bind_shape_report();
        let plan = smithay_linux_handler_reduction_plan_report();
        let matrix = smithay_linux_handler_requirement_matrix_report();
        let probe = smithay_linux_handler_probe_report();
        let selected = reduction_candidate_report(&plan, Candidate::GlobalDispatchBindShape);
        let bind_requirements = matrix
            .items
            .iter()
            .filter(|item| item.requirement == Requirement::GlobalDispatchBind)
            .collect::<Vec<_>>();

        assert_eq!(
            plan.selected_first,
            Some(Candidate::GlobalDispatchBindShape)
        );
        assert!(selected.risks.contains(&Risk::IntroducesClientBindEntry));
        assert!(!bind_shape.can_compile_trait_impl);
        assert!(!bind_shape.can_attach_to_adapter);
        assert!(!bind_shape.can_register_global);
        assert_eq!(bind_shape.modeled_count, 4);
        assert_eq!(bind_shape.blocked_count, 5);
        assert_eq!(bind_requirements.len(), 3);
        assert!(
            bind_requirements
                .iter()
                .all(|item| { item.evidence.contains(&Evidence::RequiresClientBindEntry) })
        );
        let client = bind_shape_item(&bind_shape, Input::ClientObject);
        assert!(client.modeled);
        assert!(!client.blockers.contains(&Blocker::ClientObjectNotModeled));
        assert!(
            client
                .blockers
                .contains(&Blocker::WouldCreateClientBindEntry)
        );
        assert!(
            client
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        let resource = bind_shape_item(&bind_shape, Input::GlobalResourceObject);
        assert!(resource.modeled);
        assert!(
            !resource
                .blockers
                .contains(&Blocker::ResourceObjectNotModeled)
        );
        assert!(
            resource
                .blockers
                .contains(&Blocker::WouldCreateClientBindEntry)
        );
        assert!(
            resource
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        let global_data = bind_shape_item(&bind_shape, Input::GlobalDataObject);
        assert!(global_data.modeled);
        assert!(
            !global_data
                .blockers
                .contains(&Blocker::GlobalDataNotModeled)
        );
        assert!(
            global_data
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        assert!(
            !bind_shape_item(&bind_shape, Input::DisplayHandleObject).modeled
                && bind_shape_item(&bind_shape, Input::DisplayHandleObject)
                    .blockers
                    .contains(&Blocker::DisplayHandleMustRemainHidden)
        );
        let handler_state = bind_shape_item(&bind_shape, Input::HandlerState);
        assert!(handler_state.modeled);
        assert!(
            !handler_state
                .blockers
                .contains(&Blocker::HandlerStateNotIntegrated)
        );
        assert!(
            handler_state
                .blockers
                .contains(&Blocker::WouldCreateClientBindEntry)
        );
        assert!(
            handler_state
                .blockers
                .contains(&Blocker::WouldRequireRealGlobalRegistration)
        );
        assert!(
            handler_state
                .blockers
                .contains(&Blocker::AdapterIntegrationForbidden)
        );
        assert_eq!(matrix.ready_count, 0);
        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);
    }

    #[test]
    fn existing_adapter_boundaries_remain_blocked() {
        assert_runtime_dir();
        let socket_name = unique_socket_name("phase49c-boundary");
        let mut adapter = SmithayLinuxAdapterSkeleton::with_socket_name(socket_name)
            .expect("Phase 49C 边界测试应能构造 adapter skeleton");
        let capabilities = adapter.capabilities();

        assert!(!capabilities.runs_event_loop);
        assert!(!capabilities.accepts_clients);
        assert!(!capabilities.registers_protocol_globals);
        assert!(!capabilities.dispatches_protocol_events);
        assert!(!capabilities.supports_real_wayland_surfaces);
        assert!(!capabilities.supports_gpu_rendering);
        assert!(capabilities.is_skeleton_only);

        let handler_boundary = adapter.global_handler_boundary_report();
        assert_eq!(handler_boundary.ready_count, 0);
        assert_eq!(handler_boundary.blocked_count, 3);
        assert!(handler_boundary.skeleton_only);
        assert!(handler_boundary.reports.iter().all(|report| {
            report.readiness == SmithayLinuxAdapterGlobalHandlerReadiness::FeasibilityBlocked
                && report.skeleton_only
                && !report.blockers.is_empty()
        }));
        let matrix = smithay_linux_handler_requirement_matrix_report();
        assert_eq!(matrix.ready_count, handler_boundary.ready_count);
        assert_eq!(
            matrix
                .items
                .iter()
                .map(|item| item.handler)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            handler_boundary.blocked_count
        );
        for handler_report in &handler_boundary.reports {
            let requirements = matrix
                .items
                .iter()
                .filter(|item| item.handler == handler_report.kind)
                .map(|item| item.requirement)
                .collect::<Vec<_>>();

            assert!(requirements.contains(&SmithayLinuxHandlerRequirement::GlobalDispatchBind));
            assert!(requirements.contains(&SmithayLinuxHandlerRequirement::DispatchRequest));
            assert!(handler_report.blockers.contains(
                &SmithayLinuxAdapterGlobalHandlerBlocker::MissingGlobalDispatchImplementation
            ));
            assert!(
                handler_report.blockers.contains(
                    &SmithayLinuxAdapterGlobalHandlerBlocker::MissingDispatchImplementation
                )
            );
            assert!(
                handler_report
                    .blockers
                    .contains(&SmithayLinuxAdapterGlobalHandlerBlocker::ActivationGateBlocked)
            );
            match handler_report.kind {
                SmithayLinuxAdapterGlobalHandlerKind::CompositorGlobalHandler => {
                    assert!(
                        requirements.contains(&SmithayLinuxHandlerRequirement::CompositorHandler)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::SurfaceRequestHandling)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::CoreAdmissionMapping)
                    );
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::MissingCompositorHandler
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::SurfaceRequestsUnsupported
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::CoreIntegrationUnsupported
                    ));
                }
                SmithayLinuxAdapterGlobalHandlerKind::ShmGlobalHandler => {
                    assert!(requirements.contains(&SmithayLinuxHandlerRequirement::BufferHandler));
                    assert!(requirements.contains(&SmithayLinuxHandlerRequirement::ShmHandler));
                    assert!(
                        handler_report.blockers.contains(
                            &SmithayLinuxAdapterGlobalHandlerBlocker::MissingBufferHandler
                        )
                    );
                    assert!(
                        handler_report
                            .blockers
                            .contains(&SmithayLinuxAdapterGlobalHandlerBlocker::MissingShmHandler)
                    );
                }
                SmithayLinuxAdapterGlobalHandlerKind::XdgWmBaseGlobalHandler => {
                    assert!(
                        requirements.contains(&SmithayLinuxHandlerRequirement::XdgShellHandler)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::XdgSurfaceRequestHandling)
                    );
                    assert!(
                        requirements
                            .contains(&SmithayLinuxHandlerRequirement::CoreAdmissionMapping)
                    );
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::MissingXdgShellHandler
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::SurfaceRequestsUnsupported
                    ));
                    assert!(handler_report.blockers.contains(
                        &SmithayLinuxAdapterGlobalHandlerBlocker::CoreIntegrationUnsupported
                    ));
                }
            }
        }

        let registration = adapter.attempt_real_global_registration_feasibility();
        assert_eq!(
            registration.mode,
            SmithayLinuxAdapterRealGlobalRegistrationMode::FeasibilityBlocked
        );
        assert!(registration.attempted_kinds.is_empty());
        assert!(registration.succeeded_kinds.is_empty());
        assert_eq!(registration.real_registered_count, 0);
        assert!(registration.skeleton_only);

        let snapshot = adapter.snapshot();
        assert_eq!(snapshot.global_handler_boundary.ready_count, 0);
        assert!(!snapshot.capabilities.accepts_clients);
        assert!(!snapshot.capabilities.registers_protocol_globals);
        assert!(!snapshot.capabilities.dispatches_protocol_events);
        assert!(!snapshot.capabilities.supports_real_wayland_surfaces);
        assert!(!snapshot.capabilities.supports_gpu_rendering);

        let runtime_report = BackendRuntimeReport::from(&adapter);
        assert_eq!(
            runtime_report.bootstrap_mode,
            BackendBootstrapMode::ProbeOnly
        );
        assert!(!runtime_report.capabilities.supports_real_wayland_surfaces);
        assert!(!runtime_report.capabilities.supports_gpu_rendering);
    }

    #[test]
    fn probe_report_and_requirement_matrix_remain_aligned() {
        let probe = smithay_linux_handler_probe_report();
        let matrix = smithay_linux_handler_requirement_matrix_report();

        assert_eq!(probe.kind, SmithayLinuxHandlerProbeKind::TypeShapeOnly);
        assert!(!probe.compiled_trait_shape);
        assert_eq!(matrix.ready_count, 0);
        assert!(matrix.items.iter().any(|item| {
            item.requirement == SmithayLinuxHandlerRequirement::GlobalDispatchBind
                && item.state == SmithayLinuxHandlerRequirementState::Missing
        }));
        assert!(matrix.items.iter().any(|item| {
            item.requirement == SmithayLinuxHandlerRequirement::DispatchRequest
                && item.state == SmithayLinuxHandlerRequirementState::Missing
        }));
        assert!(
            probe
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::AdapterIntegrationForbidden)
        );
        assert!(
            probe
                .blockers
                .contains(&SmithayLinuxHandlerProbeBlocker::CreateGlobalStillRequiredForRuntimeUse)
        );
    }

    #[test]
    fn production_sources_keep_probe_isolated() {
        let probe_source = include_str!("linux_handler_probe.rs");
        let production_source = probe_source
            .split("#[cfg(test)]")
            .next()
            .expect("probe source 应包含生产代码");
        let adapter_source = include_str!("linux_adapter.rs");
        let runtime_source = include_str!("runtime_facade.rs");

        let forbidden_probe_tokens = [
            ("crate::", "core"),
            ("crate::", "backend"),
            ("Backend", "Event"),
            ("Core", "Command"),
            ("BackendDriver", "Runner"),
            ("smithay", "::"),
            ("Display", "<"),
            ("Raw", "Fd"),
            ("Owned", "Fd"),
            ("Borrowed", "Fd"),
            ("Unix", "Stream"),
            ("Resource", "<"),
            ("wayland_server::", "Resource"),
            ("wl_", "resource"),
            ("GlobalData", "<"),
            ("wayland_server::", "GlobalData"),
            ("SmithayHandlerState", "<"),
            ("DispatchState", "<"),
            ("wayland_server::", "DispatchState"),
            ("AdapterState", "<"),
            ("Object", "Id"),
            ("object_", "id"),
            ("process::", "id"),
            ("accept", "("),
            ("create_", "global("),
            (".create_", "global"),
            ("register_", "global("),
            (".register_", "global"),
            ("impl Global", "Dispatch"),
            ("GlobalDispatch", "<"),
            ("impl Dis", "patch"),
            ("delegate_", "compositor"),
            ("delegate_", "shm"),
            ("delegate_xdg_", "shell"),
            ("wl_", "surface"),
            ("xdg_", "toplevel"),
            ("xdg_wm_", "base"),
            ("impl Buffer", "Handler"),
            ("impl Shm", "Handler"),
            ("impl Compositor", "Handler"),
            ("impl XdgShell", "Handler"),
            ("d", "rm"),
            ("g", "bm"),
            ("lib", "input"),
            ("u", "dev"),
            ("x", "11"),
            ("vul", "kan"),
        ];
        for (left, right) in forbidden_probe_tokens {
            let token = format!("{left}{right}");
            assert!(
                !production_source.contains(&token),
                "production probe 不得包含受限入口 {token}"
            );
        }
        let display_handle_api_token = ["display_", "handle"].concat();
        let production_without_synthetic_display_flag = production_source
            .replace("touches_display_handle", "")
            .replace("represents_real_display_handle", "")
            .replace("exposes_display_handle", "")
            .replace("stores_display_handle", "")
            .replace("reads_display_handle", "")
            .replace("can_read_display_handle", "")
            .replace("can_store_display_handle", "")
            .replace("can_expose_display_handle", "")
            .replace("smithay_linux_display_handle_access_report", "")
            .replace(
                "smithay_linux_display_handle_internal_access_gate_report",
                "",
            )
            .replace(
                "smithay_linux_display_handle_public_api_evidence_report",
                "",
            )
            .replace(
                "smithay_linux_display_handle_internal_ownership_evidence_report",
                "",
            );
        assert!(
            !production_without_synthetic_display_flag.contains(&display_handle_api_token),
            "production probe 除纯数据布尔字段外不得读取原生 display handle"
        );
        let display_handle_token = ["Display", "Handle"].concat();
        let production_without_bind_diagnostics = production_source
            .replace("DisplayHandleObject", "")
            .replace("DisplayHandleMustRemainHidden", "")
            .replace("DisplayHandleStillHidden", "")
            .replace("SmithayLinuxDisplayHandleAccessPolicy", "")
            .replace("SmithayLinuxDisplayHandleRedaction", "")
            .replace("SmithayLinuxDisplayHandleAccessBlocker", "")
            .replace("SmithayLinuxDisplayHandleAccessReport", "")
            .replace("RealDisplayHandleMustRemainPrivate", "")
            .replace("DisplayHandleExposureForbidden", "")
            .replace("DisplayHandleStorageForbidden", "")
            .replace("DisplayHandleHidden", "")
            .replace("RealDisplayHandleUnavailable", "")
            .replace("DisplayHandleInternalAccessGatePolicy", "")
            .replace("SmithayLinuxDisplayHandleInternalAccessDecision", "")
            .replace("SmithayLinuxDisplayHandleInternalAccessTarget", "")
            .replace("SmithayLinuxDisplayHandleInternalAccessPrecondition", "")
            .replace(
                "SmithayLinuxDisplayHandleInternalAccessPreconditionState",
                "",
            )
            .replace("SmithayLinuxDisplayHandleInternalAccessBlocker", "")
            .replace(
                "SmithayLinuxDisplayHandleInternalAccessPreconditionItem",
                "",
            )
            .replace("SmithayLinuxDisplayHandleInternalAccessGateReport", "")
            .replace("AdapterOwnsDisplayHandleInternally", "")
            .replace("AdapterDoesNotExposeDisplayHandlePublicly", "")
            .replace("DisplayHandleRedactionPolicyPreserved", "")
            .replace("RealDisplayHandleAccessForbidden", "")
            .replace("DisplayHandleReadForbidden", "")
            .replace("SmithayLinuxDisplayHandlePublicApiExposureDecision", "")
            .replace("SmithayLinuxDisplayHandlePublicApiSurface", "")
            .replace("SmithayLinuxDisplayHandlePublicApiEvidenceState", "")
            .replace("SmithayLinuxDisplayHandlePublicApiEvidenceLimitation", "")
            .replace("SmithayLinuxDisplayHandlePublicApiEvidenceItem", "")
            .replace("SmithayLinuxDisplayHandlePublicApiEvidenceReport", "")
            .replace("DisplayHandleReturnValue", "")
            .replace("DisplayHandleArgument", "")
            .replace("DoesNotPermitDisplayHandleAccess", "")
            .replace("SmithayLinuxDisplayHandleInternalOwnershipDecision", "")
            .replace(
                "SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource",
                "",
            )
            .replace(
                "SmithayLinuxDisplayHandleInternalOwnershipEvidenceState",
                "",
            )
            .replace("SmithayLinuxDisplayHandleInternalOwnershipLimitation", "")
            .replace("SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem", "")
            .replace(
                "SmithayLinuxDisplayHandleInternalOwnershipEvidenceReport",
                "",
            )
            .replace("DoesNotReadDisplayHandle", "");
        assert!(
            !production_without_bind_diagnostics.contains(&display_handle_token),
            "production probe 除固定诊断名称外不得暴露原生 display handle"
        );

        let probe_type = ["SmithayLinux", "InertHandlerProbe"].concat();
        let probe_report_type = ["SmithayLinux", "HandlerProbeReport"].concat();
        let probe_function = ["smithay_linux_", "handler_probe_report"].concat();
        let matrix_report_type = ["SmithayLinux", "HandlerRequirementMatrixReport"].concat();
        let matrix_function = ["smithay_linux_", "handler_requirement_matrix_report"].concat();
        let reduction_report_type = ["SmithayLinux", "HandlerReductionPlanReport"].concat();
        let reduction_function = ["smithay_linux_", "handler_reduction_plan_report"].concat();
        let bind_shape_report_type = ["SmithayLinux", "GlobalDispatchBindShapeReport"].concat();
        let bind_shape_function = ["smithay_linux_", "global_dispatch_bind_shape_report"].concat();
        let client_identity_report_type = ["SmithayLinux", "BindClientIdentityReport"].concat();
        let client_identity_function = ["smithay_linux_", "bind_client_identity_report"].concat();
        let resource_identity_report_type =
            ["SmithayLinux", "BindGlobalResourceIdentityReport"].concat();
        let resource_identity_function =
            ["smithay_linux_", "bind_global_resource_identity_report"].concat();
        let global_data_report_type = ["SmithayLinux", "BindGlobalDataReport"].concat();
        let global_data_function = ["smithay_linux_", "bind_global_data_report"].concat();
        let handler_state_report_type = ["SmithayLinux", "BindHandlerStateReport"].concat();
        let handler_state_function = ["smithay_linux_", "bind_handler_state_report"].concat();
        let display_access_report_type = ["SmithayLinux", "DisplayHandleAccessReport"].concat();
        let display_access_function = ["smithay_linux_", "display_handle_access_report"].concat();
        let final_seal_report_type = ["SmithayLinux", "GlobalDispatchBindFinalSealReport"].concat();
        let final_seal_function =
            ["smithay_linux_", "global_dispatch_bind_final_seal_report"].concat();
        let internal_gate_report_type =
            ["SmithayLinux", "DisplayHandleInternalAccessGateReport"].concat();
        let internal_gate_function = [
            "smithay_linux_",
            "display_handle_internal_access_gate_report",
        ]
        .concat();
        let public_api_evidence_report_type =
            ["SmithayLinux", "DisplayHandlePublicApiEvidenceReport"].concat();
        let public_api_evidence_function = [
            "smithay_linux_",
            "display_handle_public_api_evidence_report",
        ]
        .concat();
        let internal_ownership_evidence_report_type = [
            "SmithayLinux",
            "DisplayHandleInternalOwnershipEvidenceReport",
        ]
        .concat();
        let internal_ownership_evidence_function = [
            "smithay_linux_",
            "display_handle_internal_ownership_evidence_report",
        ]
        .concat();
        for source in [adapter_source, runtime_source] {
            assert!(!source.contains(&probe_type));
            assert!(!source.contains(&probe_report_type));
            assert!(!source.contains(&probe_function));
            assert!(!source.contains(&matrix_report_type));
            assert!(!source.contains(&matrix_function));
            assert!(!source.contains(&reduction_report_type));
            assert!(!source.contains(&reduction_function));
            assert!(!source.contains(&bind_shape_report_type));
            assert!(!source.contains(&bind_shape_function));
            assert!(!source.contains(&client_identity_report_type));
            assert!(!source.contains(&client_identity_function));
            assert!(!source.contains(&resource_identity_report_type));
            assert!(!source.contains(&resource_identity_function));
            assert!(!source.contains(&global_data_report_type));
            assert!(!source.contains(&global_data_function));
            assert!(!source.contains(&handler_state_report_type));
            assert!(!source.contains(&handler_state_function));
            assert!(!source.contains(&display_access_report_type));
            assert!(!source.contains(&display_access_function));
            assert!(!source.contains(&final_seal_report_type));
            assert!(!source.contains(&final_seal_function));
            assert!(!source.contains(&internal_gate_report_type));
            assert!(!source.contains(&internal_gate_function));
            assert!(!source.contains(&public_api_evidence_report_type));
            assert!(!source.contains(&public_api_evidence_function));
            assert!(!source.contains(&internal_ownership_evidence_report_type));
            assert!(!source.contains(&internal_ownership_evidence_function));
        }
    }

    fn internal_ownership_evidence_item_by_source(
        report: &super::SmithayLinuxDisplayHandleInternalOwnershipEvidenceReport,
        source: SmithayLinuxDisplayHandleInternalOwnershipEvidenceSource,
    ) -> &super::SmithayLinuxDisplayHandleInternalOwnershipEvidenceItem {
        report
            .items
            .iter()
            .find(|item| item.source == source)
            .expect("internal ownership evidence report 必须包含指定 source")
    }

    fn public_api_evidence_item_by_surface(
        report: &super::SmithayLinuxDisplayHandlePublicApiEvidenceReport,
        surface: SmithayLinuxDisplayHandlePublicApiSurface,
    ) -> &super::SmithayLinuxDisplayHandlePublicApiEvidenceItem {
        report
            .items
            .iter()
            .find(|item| item.surface == surface)
            .expect("public API evidence report 必须包含指定 surface")
    }

    fn internal_access_precondition(
        report: &super::SmithayLinuxDisplayHandleInternalAccessGateReport,
        precondition: SmithayLinuxDisplayHandleInternalAccessPrecondition,
    ) -> &super::SmithayLinuxDisplayHandleInternalAccessPreconditionItem {
        report
            .preconditions
            .iter()
            .find(|item| item.precondition == precondition)
            .expect("internal access gate 必须包含指定 precondition")
    }

    fn final_seal_input(
        report: &super::SmithayLinuxGlobalDispatchBindFinalSealReport,
        input: SmithayLinuxGlobalDispatchBindInput,
    ) -> &super::SmithayLinuxGlobalDispatchBindSealedInputItem {
        report
            .inputs
            .iter()
            .find(|item| item.input == input)
            .expect("final seal report 必须包含指定 input")
    }

    fn bind_shape_item(
        report: &super::SmithayLinuxGlobalDispatchBindShapeReport,
        input: SmithayLinuxGlobalDispatchBindInput,
    ) -> &super::SmithayLinuxGlobalDispatchBindShapeItem {
        report
            .items
            .iter()
            .find(|item| item.input == input)
            .expect("bind shape report 必须包含指定 input")
    }

    fn reduction_candidate_report(
        report: &super::SmithayLinuxHandlerReductionPlanReport,
        candidate: SmithayLinuxHandlerReductionCandidate,
    ) -> &super::SmithayLinuxHandlerReductionCandidateReport {
        report
            .candidates
            .iter()
            .find(|report| report.candidate == candidate)
            .expect("reduction plan 必须包含指定 candidate")
    }
}
