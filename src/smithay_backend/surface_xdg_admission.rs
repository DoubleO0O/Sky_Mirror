//! Surface/XDG protocol object 接纳的跨平台纯数据 contract。
//!
//! 真实 Smithay protocol object 必须留在 Linux adapter；本模块只保存 adapter identity
//! 与 core ID 的映射。identity contract 可在 default build 中验证，但不表示真实 protocol
//! object、dispatch、render 或 input runtime 已存在。

use std::{collections::BTreeMap, num::NonZeroU64};

use crate::core::{
    backend_event::BackendEvent,
    client::ClientId,
    command::CommandResult,
    runtime_bridge::{CoreRuntimeBridge, RuntimeEventResult},
    state::State,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
    workspace::WindowId,
};

/// adapter 观察到的 protocol object 的非零纯数据身份。
///
/// 该值不是 Wayland object，也不保存任何 Smithay handle；不同 adapter identity wrapper
/// 用它保留稳定数值，同时避免 surface 与 toplevel 被直接混用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolObjectId(NonZeroU64);

impl ProtocolObjectId {
    /// 从非零数值创建 protocol object identity；零值返回 `None`。
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// 返回稳定的非零数值。
    pub const fn value(self) -> u64 {
        self.0.get()
    }
}

/// adapter 层观察到的 surface-like object 身份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdapterSurfaceId(ProtocolObjectId);

impl AdapterSurfaceId {
    /// 使用 protocol object identity 创建 surface identity。
    pub const fn new(protocol_object: ProtocolObjectId) -> Self {
        Self(protocol_object)
    }

    /// 返回未区分对象种类的底层 protocol identity。
    pub const fn protocol_object_id(self) -> ProtocolObjectId {
        self.0
    }

    /// 返回稳定的非零数值。
    pub const fn value(self) -> u64 {
        self.0.value()
    }
}

/// adapter 层观察到的 xdg-toplevel-like object 身份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdapterToplevelId(ProtocolObjectId);

impl AdapterToplevelId {
    /// 使用 protocol object identity 创建 toplevel identity。
    pub const fn new(protocol_object: ProtocolObjectId) -> Self {
        Self(protocol_object)
    }

    /// 返回未区分对象种类的底层 protocol identity。
    pub const fn protocol_object_id(self) -> ProtocolObjectId {
        self.0
    }

    /// 返回稳定的非零数值。
    pub const fn value(self) -> u64 {
        self.0.value()
    }
}

/// Phase 52A 之后仍阻止真实 protocol runtime 的能力缺口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceXdgAdmissionBlocker {
    /// 尚未提供 Linux-only 真实 protocol type 编译边界。
    MissingLinuxProtocolCompileBoundary,
    /// 尚未接入真实 `wl_surface` runtime。
    MissingRealWlSurfaceRuntime,
    /// 尚未接入真实 `xdg_toplevel` runtime。
    MissingRealXdgToplevelRuntime,
    /// 尚未启动真实 protocol globals 与 request dispatch。
    MissingProtocolDispatch,
}

/// Phase 52A 纯数据 contract 与未完成 runtime 能力的精确报告。
#[must_use = "必须区分 admission contract 与真实 protocol runtime"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceXdgAdmissionReadinessReport {
    /// 当前仍存在的 Linux/runtime blockers。
    pub blockers: Vec<SurfaceXdgAdmissionBlocker>,
    /// surface admission contract 是否可用。
    pub surface_admission_contract_available: bool,
    /// xdg toplevel admission contract 是否可用。
    pub xdg_toplevel_admission_contract_available: bool,
    /// adapter surface identity 是否可用。
    pub adapter_surface_identity_available: bool,
    /// adapter toplevel identity 是否可用。
    pub adapter_toplevel_identity_available: bool,
    /// adapter surface 到 core `SurfaceId` 的映射是否可用。
    pub surface_to_core_mapping_available: bool,
    /// adapter toplevel 到 core `WindowId` 的映射是否可用。
    pub window_to_core_mapping_available: bool,
    /// core `SurfaceId -> WindowId` link contract 是否可用。
    pub surface_window_link_available: bool,
    /// Linux-only protocol type 编译边界是否可用；Phase 52A B 路线固定为 `false`。
    pub linux_protocol_compile_boundary_available: bool,
    /// 真实 `wl_surface` runtime 是否可用；固定为 `false`。
    pub real_wl_surface_runtime_available: bool,
    /// 真实 `xdg_toplevel` runtime 是否可用；固定为 `false`。
    pub real_xdg_toplevel_runtime_available: bool,
    /// 真实 protocol dispatch 是否已启动；固定为 `false`。
    pub protocol_dispatch_started: bool,
    /// 真实 render 是否可用；固定为 `false`。
    pub render_support: bool,
    /// 真实 input 是否可用；固定为 `false`。
    pub input_support: bool,
}

impl SurfaceXdgAdmissionReadinessReport {
    /// 判断 Phase 52A 纯数据 admission contract 是否完整，不推导 runtime readiness。
    pub fn is_contract_ready(&self) -> bool {
        self.surface_admission_contract_available
            && self.xdg_toplevel_admission_contract_available
            && self.adapter_surface_identity_available
            && self.adapter_toplevel_identity_available
            && self.surface_to_core_mapping_available
            && self.window_to_core_mapping_available
            && self.surface_window_link_available
    }
}

/// 返回 Phase 52A B 路线的保守 capability 快照。
#[must_use = "纯数据 mapping proof 不能冒充 wl_surface 或 xdg-shell runtime"]
pub fn surface_xdg_admission_readiness_report() -> SurfaceXdgAdmissionReadinessReport {
    SurfaceXdgAdmissionReadinessReport {
        blockers: vec![
            SurfaceXdgAdmissionBlocker::MissingLinuxProtocolCompileBoundary,
            SurfaceXdgAdmissionBlocker::MissingRealWlSurfaceRuntime,
            SurfaceXdgAdmissionBlocker::MissingRealXdgToplevelRuntime,
            SurfaceXdgAdmissionBlocker::MissingProtocolDispatch,
        ],
        surface_admission_contract_available: true,
        xdg_toplevel_admission_contract_available: true,
        adapter_surface_identity_available: true,
        adapter_toplevel_identity_available: true,
        surface_to_core_mapping_available: true,
        window_to_core_mapping_available: true,
        surface_window_link_available: true,
        linux_protocol_compile_boundary_available: false,
        real_wl_surface_runtime_available: false,
        real_xdg_toplevel_runtime_available: false,
        protocol_dispatch_started: false,
        render_support: false,
        input_support: false,
    }
}

/// adapter surface observation 进入 core seam 所需的纯数据意图。
///
/// `core_surface` 是 adapter 为本次映射选定的稳定 core ID；真实 protocol object
/// 不得保存在本类型中，也不得跨越 backend/core seam。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceAdmissionIntent {
    /// adapter 层的 surface identity。
    pub adapter_surface: AdapterSurfaceId,
    /// 准备通过既有事件/命令 seam 注册的 core `SurfaceId`。
    pub core_surface: SurfaceId,
    /// 可选的现有 core client 归属。
    pub client: Option<ClientId>,
    /// surface 的纯数据角色。
    pub role: SurfaceRole,
}

/// adapter xdg-toplevel observation 进入 core seam 所需的纯数据意图。
///
/// toplevel identity 必须引用已经接受的 adapter surface identity；真实 xdg object
/// 留在 adapter，title/app_id/kind 作为纯数据进入既有 `ToplevelMapped` 事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgToplevelAdmissionIntent {
    /// adapter 层的 toplevel identity。
    pub adapter_toplevel: AdapterToplevelId,
    /// 该 toplevel 所属的已接受 adapter surface identity。
    pub adapter_surface: AdapterSurfaceId,
    /// core window title metadata。
    pub title: String,
    /// 可选 core application ID metadata。
    pub app_id: Option<String>,
    /// core window kind；本 contract 不推导 renderer 或 protocol runtime。
    pub kind: WindowKind,
}

/// 已接受 surface identity 的稳定映射记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceAdmissionMapping {
    /// adapter surface identity。
    pub adapter_surface: AdapterSurfaceId,
    /// 对应的 core surface identity。
    pub core_surface: SurfaceId,
}

/// 已接受 toplevel identity 的稳定 mapping 与 surface link 记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelAdmissionMapping {
    /// adapter toplevel identity。
    pub adapter_toplevel: AdapterToplevelId,
    /// 被引用的 adapter surface identity。
    pub adapter_surface: AdapterSurfaceId,
    /// adapter surface 对应的 core surface identity。
    pub core_surface: SurfaceId,
    /// 既有 core seam 创建并返回的 window identity。
    pub core_window: WindowId,
}

/// Surface/XDG admission 被拒绝时的结构化原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceXdgAdmissionError {
    /// adapter surface identity 已经绑定到另一个或同一个 core surface。
    DuplicateSurface {
        /// 重复提交的 adapter identity。
        adapter_surface: AdapterSurfaceId,
        /// ledger 中已经存在的 core surface。
        existing_surface: SurfaceId,
    },
    /// 既有 core seam 拒绝了 surface 注册，例如 core ID 已被占用。
    CoreSurfaceRejected {
        /// 被拒绝的 adapter identity。
        adapter_surface: AdapterSurfaceId,
        /// 被拒绝的 core surface identity。
        core_surface: SurfaceId,
    },
    /// adapter toplevel identity 已经绑定到 core window。
    DuplicateToplevel {
        /// 重复提交的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
        /// ledger 中已经存在的 core window。
        existing_window: WindowId,
    },
    /// adapter surface 已经拥有一个已接受的 toplevel/window mapping。
    SurfaceAlreadyHasToplevel {
        /// 已经完成 toplevel admission 的 adapter surface。
        adapter_surface: AdapterSurfaceId,
        /// 该 surface 已绑定的 adapter toplevel identity。
        existing_toplevel: AdapterToplevelId,
        /// 该 surface 已绑定的 core window identity。
        existing_window: WindowId,
    },
    /// toplevel 引用了尚未经过 surface admission 的 adapter identity。
    OrphanToplevel {
        /// 被拒绝的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
        /// 缺失 mapping 的 adapter surface identity。
        adapter_surface: AdapterSurfaceId,
    },
    /// ledger 中的 adapter surface mapping 已不对应存活 core surface。
    StaleSurfaceMapping {
        /// stale adapter surface identity。
        adapter_surface: AdapterSurfaceId,
        /// ledger 保存但 core 已缺失或 dead 的 surface identity。
        core_surface: SurfaceId,
    },
    /// 既有 core seam 未能把 toplevel 绑定到已验证的 surface。
    CoreToplevelRejected {
        /// 被拒绝的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
        /// 目标 core surface identity。
        core_surface: SurfaceId,
    },
    /// 既有 core seam 返回了与 surface admission 不匹配的结果。
    UnexpectedCoreResult,
}

/// 一次成功 admission 的纯数据 mapping 与完整 runtime seam 证据。
#[must_use = "admission report 包含 core mapping 与 ValidationReport，不能忽略"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceXdgAdmissionReport {
    /// surface identity 已通过既有 core seam 完成注册。
    SurfaceAdmitted {
        /// adapter/core identity 映射。
        mapping: SurfaceAdmissionMapping,
        /// `BackendEvent -> CoreCommand -> State` 的完整结果。
        runtime: RuntimeEventResult,
    },
    /// toplevel identity 已通过既有 core seam 创建 window 并完成 surface link。
    ToplevelAdmitted {
        /// adapter/core identity 与 surface link mapping。
        mapping: ToplevelAdmissionMapping,
        /// `BackendEvent -> CoreCommand -> State` 的完整结果。
        runtime: RuntimeEventResult,
    },
}

impl SurfaceXdgAdmissionReport {
    /// 返回本次 admission 产生或引用的 core `SurfaceId`。
    pub fn core_surface(&self) -> Option<SurfaceId> {
        match self {
            Self::SurfaceAdmitted { mapping, .. } => Some(mapping.core_surface),
            Self::ToplevelAdmitted { mapping, .. } => Some(mapping.core_surface),
        }
    }

    /// 返回本次 admission 产生的 core `WindowId`；surface-only 报告返回 `None`。
    pub fn core_window(&self) -> Option<WindowId> {
        match self {
            Self::SurfaceAdmitted { .. } => None,
            Self::ToplevelAdmitted { mapping, .. } => Some(mapping.core_window),
        }
    }

    /// 返回既有 runtime seam 的完整结果。
    pub fn runtime(&self) -> &RuntimeEventResult {
        match self {
            Self::SurfaceAdmitted { runtime, .. } => runtime,
            Self::ToplevelAdmitted { runtime, .. } => runtime,
        }
    }

    /// 判断 core mutation 后的 `ValidationReport` 是否 clean。
    pub fn validation_is_clean(&self) -> bool {
        self.runtime().validation.is_clean()
    }
}

/// adapter identity 到 core identity 的纯数据 admission ledger。
///
/// ledger 不保存真实 protocol object，也不直接修改 registry。所有 core mutation
/// 必须通过既有 `CoreRuntimeBridge`；只有 core 明确接受后才保存 mapping。
#[derive(Debug, Default)]
pub struct SurfaceXdgAdmissionLedger {
    surfaces: BTreeMap<AdapterSurfaceId, SurfaceId>,
    toplevels: BTreeMap<AdapterToplevelId, WindowId>,
    surface_toplevels: BTreeMap<AdapterSurfaceId, (AdapterToplevelId, WindowId)>,
}

impl SurfaceXdgAdmissionLedger {
    /// 创建空 mapping ledger。
    pub fn new() -> Self {
        Self::default()
    }

    /// 查询 adapter surface 已接受的 core mapping。
    pub fn surface_mapping(&self, adapter_surface: AdapterSurfaceId) -> Option<SurfaceId> {
        self.surfaces.get(&adapter_surface).copied()
    }

    /// 查询 adapter toplevel 已接受的 core mapping。
    pub fn toplevel_mapping(&self, adapter_toplevel: AdapterToplevelId) -> Option<WindowId> {
        self.toplevels.get(&adapter_toplevel).copied()
    }

    /// 通过既有 backend event/runtime bridge 接受一个纯数据 surface intent。
    pub fn admit_surface(
        &mut self,
        state: &mut State,
        intent: SurfaceAdmissionIntent,
    ) -> Result<SurfaceXdgAdmissionReport, SurfaceXdgAdmissionError> {
        if let Some(existing_surface) = self.surface_mapping(intent.adapter_surface) {
            return Err(SurfaceXdgAdmissionError::DuplicateSurface {
                adapter_surface: intent.adapter_surface,
                existing_surface,
            });
        }

        // 真实 object 先在 adapter 转成 identity；core 只接收既有纯数据事件。
        let runtime = CoreRuntimeBridge::handle_backend_event(
            state,
            BackendEvent::SurfaceCreated {
                surface: intent.core_surface,
                client: intent.client,
                role: intent.role,
            },
        );

        match runtime.result {
            CommandResult::SurfaceRegistered {
                surface,
                registered: true,
            } if surface == intent.core_surface => {
                let mapping = SurfaceAdmissionMapping {
                    adapter_surface: intent.adapter_surface,
                    core_surface: surface,
                };
                self.surfaces.insert(intent.adapter_surface, surface);
                Ok(SurfaceXdgAdmissionReport::SurfaceAdmitted { mapping, runtime })
            }
            CommandResult::SurfaceRegistered { .. } => {
                Err(SurfaceXdgAdmissionError::CoreSurfaceRejected {
                    adapter_surface: intent.adapter_surface,
                    core_surface: intent.core_surface,
                })
            }
            _ => Err(SurfaceXdgAdmissionError::UnexpectedCoreResult),
        }
    }

    /// 通过既有 backend event/runtime bridge 接受一个纯数据 xdg-toplevel intent。
    pub fn admit_toplevel(
        &mut self,
        state: &mut State,
        intent: XdgToplevelAdmissionIntent,
    ) -> Result<SurfaceXdgAdmissionReport, SurfaceXdgAdmissionError> {
        if let Some(existing_window) = self.toplevel_mapping(intent.adapter_toplevel) {
            return Err(SurfaceXdgAdmissionError::DuplicateToplevel {
                adapter_toplevel: intent.adapter_toplevel,
                existing_window,
            });
        }

        if let Some((existing_toplevel, existing_window)) =
            self.surface_toplevels.get(&intent.adapter_surface).copied()
        {
            return Err(SurfaceXdgAdmissionError::SurfaceAlreadyHasToplevel {
                adapter_surface: intent.adapter_surface,
                existing_toplevel,
                existing_window,
            });
        }

        let Some(core_surface) = self.surface_mapping(intent.adapter_surface) else {
            return Err(SurfaceXdgAdmissionError::OrphanToplevel {
                adapter_toplevel: intent.adapter_toplevel,
                adapter_surface: intent.adapter_surface,
            });
        };

        // mapping 只代表 adapter 曾经接受过该 identity；dispatch 前仍需确认 core live state。
        if !state.surfaces.is_alive(core_surface) {
            return Err(SurfaceXdgAdmissionError::StaleSurfaceMapping {
                adapter_surface: intent.adapter_surface,
                core_surface,
            });
        }

        // surface/window link 仍由既有 State command seam 完成；ledger 不直接写 registry。
        let runtime = CoreRuntimeBridge::handle_backend_event(
            state,
            BackendEvent::ToplevelMapped {
                surface: core_surface,
                title: intent.title,
                app_id: intent.app_id,
                kind: intent.kind,
            },
        );

        match runtime.result {
            CommandResult::WindowRegisteredForSurface {
                surface,
                window,
                bound: true,
            } if surface == core_surface => {
                let mapping = ToplevelAdmissionMapping {
                    adapter_toplevel: intent.adapter_toplevel,
                    adapter_surface: intent.adapter_surface,
                    core_surface,
                    core_window: window,
                };
                self.toplevels.insert(intent.adapter_toplevel, window);
                self.surface_toplevels
                    .insert(intent.adapter_surface, (intent.adapter_toplevel, window));
                Ok(SurfaceXdgAdmissionReport::ToplevelAdmitted { mapping, runtime })
            }
            CommandResult::WindowRegisteredForSurface { .. } => {
                Err(SurfaceXdgAdmissionError::CoreToplevelRejected {
                    adapter_toplevel: intent.adapter_toplevel,
                    core_surface,
                })
            }
            _ => Err(SurfaceXdgAdmissionError::UnexpectedCoreResult),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId, SurfaceAdmissionIntent,
        SurfaceXdgAdmissionBlocker, SurfaceXdgAdmissionError, SurfaceXdgAdmissionLedger,
        XdgToplevelAdmissionIntent, surface_xdg_admission_readiness_report,
    };
    use crate::core::{
        backend_event::BackendEvent, command::CoreCommand, runtime_bridge::CoreRuntimeBridge,
        state::State, surface::SurfaceRole, window::WindowKind,
    };

    fn surface_id(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(
            ProtocolObjectId::new(value).expect("测试 adapter surface identity 必须非零"),
        )
    }

    fn toplevel_id(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(
            ProtocolObjectId::new(value).expect("测试 adapter toplevel identity 必须非零"),
        )
    }

    fn admit_test_surface(
        ledger: &mut SurfaceXdgAdmissionLedger,
        state: &mut State,
        adapter_surface: AdapterSurfaceId,
        core_surface: u64,
    ) {
        let _report = ledger
            .admit_surface(
                state,
                SurfaceAdmissionIntent {
                    adapter_surface,
                    core_surface,
                    client: None,
                    role: SurfaceRole::XdgToplevel,
                },
            )
            .expect("测试 surface admission 必须成功");
    }

    #[test]
    fn adapter_surface_identity_round_trips() {
        let protocol = ProtocolObjectId::new(41).expect("非零 protocol object ID 必须有效");
        let adapter = AdapterSurfaceId::new(protocol);

        assert_eq!(adapter.protocol_object_id(), protocol);
        assert_eq!(adapter.value(), 41);
        assert!(ProtocolObjectId::new(0).is_none());
    }

    #[test]
    fn adapter_toplevel_identity_round_trips() {
        let protocol = ProtocolObjectId::new(73).expect("非零 protocol object ID 必须有效");
        let adapter = AdapterToplevelId::new(protocol);

        assert_eq!(adapter.protocol_object_id(), protocol);
        assert_eq!(adapter.value(), 73);
    }

    #[test]
    fn surface_xdg_admission_keeps_runtime_capability_false() {
        let report = surface_xdg_admission_readiness_report();

        assert!(report.surface_admission_contract_available);
        assert!(report.xdg_toplevel_admission_contract_available);
        assert!(report.adapter_surface_identity_available);
        assert!(report.adapter_toplevel_identity_available);
        assert!(report.surface_to_core_mapping_available);
        assert!(report.window_to_core_mapping_available);
        assert!(report.surface_window_link_available);
        assert!(!report.linux_protocol_compile_boundary_available);
        assert!(!report.real_wl_surface_runtime_available);
        assert!(!report.real_xdg_toplevel_runtime_available);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.is_contract_ready());
        assert_eq!(
            report.blockers,
            vec![
                SurfaceXdgAdmissionBlocker::MissingLinuxProtocolCompileBoundary,
                SurfaceXdgAdmissionBlocker::MissingRealWlSurfaceRuntime,
                SurfaceXdgAdmissionBlocker::MissingRealXdgToplevelRuntime,
                SurfaceXdgAdmissionBlocker::MissingProtocolDispatch,
            ]
        );
    }

    #[test]
    fn surface_xdg_admission_contract_is_pure_data() {
        let source = include_str!("surface_xdg_admission.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("生产源码区段必须存在");

        for forbidden in [
            "use smithay",
            "wayland_server::",
            "wayland_protocols::",
            "GlobalDispatch",
            "impl Dispatch",
        ] {
            assert!(
                !production.contains(forbidden),
                "纯数据 admission contract 不得包含平台 protocol 类型或 dispatch: {forbidden}"
            );
        }
    }

    #[test]
    fn surface_xdg_admission_default_facade_is_available() {
        use crate::smithay_backend::{
            AdapterSurfaceId as FacadeSurfaceId, AdapterToplevelId as FacadeToplevelId,
            ProtocolObjectId as FacadeProtocolObjectId, SurfaceXdgAdmissionLedger as FacadeLedger,
            surface_xdg_admission_readiness_report as facade_readiness,
        };

        let protocol = FacadeProtocolObjectId::new(81).expect("facade identity 必须可构造");
        let _surface = FacadeSurfaceId::new(protocol);
        let _toplevel = FacadeToplevelId::new(protocol);
        let _ledger = FacadeLedger::new();
        assert!(facade_readiness().is_contract_ready());
    }

    #[test]
    fn surface_admission_registers_core_surface_id() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(11);
        let intent = SurfaceAdmissionIntent {
            adapter_surface,
            core_surface: 9001,
            client: None,
            role: SurfaceRole::Unknown,
        };

        let report = ledger
            .admit_surface(&mut state, intent.clone())
            .expect("首个 surface admission 必须成功");

        assert_eq!(ledger.surface_mapping(adapter_surface), Some(9001));
        assert_eq!(report.core_surface(), Some(9001));
        assert!(report.validation_is_clean());
        assert_eq!(
            report.runtime().event,
            BackendEvent::SurfaceCreated {
                surface: 9001,
                client: None,
                role: SurfaceRole::Unknown,
            }
        );
        assert_eq!(
            report.runtime().command,
            CoreCommand::RegisterSurface {
                surface: Some(9001),
                client: None,
                role: SurfaceRole::Unknown,
            }
        );
        assert!(state.surfaces.is_alive(9001));
        assert!(state.validate().is_clean());
    }

    #[test]
    fn duplicate_surface_admission_is_rejected_without_core_mutation() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(12);
        let _report = ledger
            .admit_surface(
                &mut state,
                SurfaceAdmissionIntent {
                    adapter_surface,
                    core_surface: 9002,
                    client: None,
                    role: SurfaceRole::Unknown,
                },
            )
            .expect("首个 surface admission 必须成功");
        let surface_count = state.surfaces.records().len();

        let error = ledger
            .admit_surface(
                &mut state,
                SurfaceAdmissionIntent {
                    adapter_surface,
                    core_surface: 9003,
                    client: None,
                    role: SurfaceRole::XdgToplevel,
                },
            )
            .expect_err("重复 adapter surface identity 必须被拒绝");

        assert_eq!(
            error,
            SurfaceXdgAdmissionError::DuplicateSurface {
                adapter_surface,
                existing_surface: 9002,
            }
        );
        assert_eq!(ledger.surface_mapping(adapter_surface), Some(9002));
        assert_eq!(state.surfaces.records().len(), surface_count);
        assert!(state.surfaces.get(9003).is_none());
        assert!(state.validate().is_clean());
    }

    #[test]
    fn xdg_admission_registers_core_window_id() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(21);
        let adapter_toplevel = toplevel_id(22);
        admit_test_surface(&mut ledger, &mut state, adapter_surface, 9101);
        let intent = XdgToplevelAdmissionIntent {
            adapter_toplevel,
            adapter_surface,
            title: "Phase 52A".to_owned(),
            app_id: Some("sky-mirror.phase52a".to_owned()),
            kind: WindowKind::WaylandPlaceholder,
        };

        let report = ledger
            .admit_toplevel(&mut state, intent)
            .expect("首个 xdg toplevel admission 必须成功");
        let window = report.core_window().expect("报告必须包含 core WindowId");

        assert_eq!(ledger.toplevel_mapping(adapter_toplevel), Some(window));
        assert_eq!(state.surfaces.window_for_surface(9101), Some(window));
        assert!(state.registry.is_alive(window));
        assert!(report.validation_is_clean());
        assert_eq!(
            report.runtime().event,
            BackendEvent::ToplevelMapped {
                surface: 9101,
                title: "Phase 52A".to_owned(),
                app_id: Some("sky-mirror.phase52a".to_owned()),
                kind: WindowKind::WaylandPlaceholder,
            }
        );
        assert_eq!(
            report.runtime().command,
            CoreCommand::RegisterWindowForSurface {
                surface: 9101,
                title: "Phase 52A".to_owned(),
                app_id: Some("sky-mirror.phase52a".to_owned()),
                kind: WindowKind::WaylandPlaceholder,
            }
        );
        assert!(state.validate().is_clean());
    }

    #[test]
    fn duplicate_toplevel_admission_is_rejected_without_core_mutation() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(31);
        let adapter_toplevel = toplevel_id(32);
        admit_test_surface(&mut ledger, &mut state, adapter_surface, 9201);
        let first = ledger
            .admit_toplevel(
                &mut state,
                XdgToplevelAdmissionIntent {
                    adapter_toplevel,
                    adapter_surface,
                    title: "first".to_owned(),
                    app_id: None,
                    kind: WindowKind::WaylandPlaceholder,
                },
            )
            .expect("首个 toplevel admission 必须成功");
        let existing_window = first.core_window().expect("首个报告必须包含 WindowId");
        let window_count = state.registry.records().len();

        let error = ledger
            .admit_toplevel(
                &mut state,
                XdgToplevelAdmissionIntent {
                    adapter_toplevel,
                    adapter_surface,
                    title: "duplicate".to_owned(),
                    app_id: None,
                    kind: WindowKind::WaylandPlaceholder,
                },
            )
            .expect_err("重复 adapter toplevel identity 必须被拒绝");

        assert_eq!(
            error,
            SurfaceXdgAdmissionError::DuplicateToplevel {
                adapter_toplevel,
                existing_window,
            }
        );
        assert_eq!(state.registry.records().len(), window_count);
        assert_eq!(
            ledger.toplevel_mapping(adapter_toplevel),
            Some(existing_window)
        );
        assert!(state.validate().is_clean());
    }

    #[test]
    fn second_toplevel_for_same_surface_is_rejected_without_core_mutation() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(35);
        let first_toplevel = toplevel_id(36);
        let second_toplevel = toplevel_id(37);
        admit_test_surface(&mut ledger, &mut state, adapter_surface, 9251);
        let first = ledger
            .admit_toplevel(
                &mut state,
                XdgToplevelAdmissionIntent {
                    adapter_toplevel: first_toplevel,
                    adapter_surface,
                    title: "first".to_owned(),
                    app_id: None,
                    kind: WindowKind::WaylandPlaceholder,
                },
            )
            .expect("首个 surface toplevel 必须成功");
        let existing_window = first.core_window().expect("首个报告必须包含 WindowId");
        let window_count = state.registry.records().len();

        let error = ledger
            .admit_toplevel(
                &mut state,
                XdgToplevelAdmissionIntent {
                    adapter_toplevel: second_toplevel,
                    adapter_surface,
                    title: "second".to_owned(),
                    app_id: None,
                    kind: WindowKind::WaylandPlaceholder,
                },
            )
            .expect_err("同一 adapter surface 不得接纳第二个 toplevel");

        assert_eq!(
            error,
            SurfaceXdgAdmissionError::SurfaceAlreadyHasToplevel {
                adapter_surface,
                existing_toplevel: first_toplevel,
                existing_window,
            }
        );
        assert_eq!(state.registry.records().len(), window_count);
        assert!(ledger.toplevel_mapping(second_toplevel).is_none());
        assert!(state.validate().is_clean());
    }

    #[test]
    fn orphan_xdg_admission_is_rejected() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(41);
        let adapter_toplevel = toplevel_id(42);
        let window_count = state.registry.records().len();

        let error = ledger
            .admit_toplevel(
                &mut state,
                XdgToplevelAdmissionIntent {
                    adapter_toplevel,
                    adapter_surface,
                    title: "orphan".to_owned(),
                    app_id: None,
                    kind: WindowKind::WaylandPlaceholder,
                },
            )
            .expect_err("没有 surface mapping 的 toplevel 必须被拒绝");

        assert_eq!(
            error,
            SurfaceXdgAdmissionError::OrphanToplevel {
                adapter_toplevel,
                adapter_surface,
            }
        );
        assert_eq!(state.registry.records().len(), window_count);
        assert!(ledger.toplevel_mapping(adapter_toplevel).is_none());
        assert!(state.validate().is_clean());
    }

    #[test]
    fn stale_surface_mapping_rejects_xdg_without_core_window() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();
        let adapter_surface = surface_id(51);
        let adapter_toplevel = toplevel_id(52);
        admit_test_surface(&mut ledger, &mut state, adapter_surface, 9301);
        let close = CoreRuntimeBridge::handle_backend_event(
            &mut state,
            BackendEvent::SurfaceClosed { surface: 9301 },
        );
        assert!(close.validation.is_clean());
        let window_count = state.registry.records().len();

        let error = ledger
            .admit_toplevel(
                &mut state,
                XdgToplevelAdmissionIntent {
                    adapter_toplevel,
                    adapter_surface,
                    title: "stale".to_owned(),
                    app_id: None,
                    kind: WindowKind::WaylandPlaceholder,
                },
            )
            .expect_err("dead core surface mapping 必须被拒绝");

        assert_eq!(
            error,
            SurfaceXdgAdmissionError::StaleSurfaceMapping {
                adapter_surface,
                core_surface: 9301,
            }
        );
        assert_eq!(state.registry.records().len(), window_count);
        assert!(ledger.toplevel_mapping(adapter_toplevel).is_none());
        assert!(state.validate().is_clean());
    }
}
