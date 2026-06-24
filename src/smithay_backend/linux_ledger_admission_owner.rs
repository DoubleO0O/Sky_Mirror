use std::collections::HashMap;

use crate::core::{
    state::State,
    surface::{SurfaceId, SurfaceRole},
    window::WindowKind,
};

use super::{
    linux_wl_surface_identity::SurfaceIdentityKey,
    surface_xdg_admission::{
        AdapterSurfaceId, AdapterToplevelId, SurfaceAdmissionIntent, SurfaceXdgAdmissionError,
        SurfaceXdgAdmissionLedger, SurfaceXdgAdmissionReport, XdgToplevelAdmissionIntent,
    },
};

/// Adapter-ledger admission ownership proof 的结构化 blocker。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLedgerAdmissionBlocker {
    /// Adapter surface identity 尚未注册到 ledger。
    MissingSurfaceAdmission,
    /// Adapter toplevel identity 尚未注册到 adapter identity registry。
    MissingToplevelIdentityRegistration,
    /// Surface toplevel identity key 不在 adapter-owned registry 中。
    UnknownToplevelIdentity,
}

/// Adapter-ledger admission proof 中可定位的操作阶段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLedgerAdmissionOperation {
    /// 检查 surface admission 前置条件。
    CheckSurfaceAdmissionPrerequisite,
    /// 检查 toplevel identity registration 前置条件。
    CheckToplevelIdentityPrerequisite,
    /// 提交 toplevel admission intent。
    SubmitToplevelAdmission,
    /// 验证 admission 结果。
    VerifyAdmissionReport,
}

/// Adapter-ledger admission ownership proof 的纯数据错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLedgerAdmissionError {
    /// 前置 capability 尚未满足。
    Blocked(AdapterLedgerAdmissionBlocker),
    /// Ledger 拒绝 surface admission。
    SurfaceAdmissionRejected {
        /// Ledger 返回的错误。
        source: SurfaceXdgAdmissionError,
    },
    /// Ledger 拒绝 toplevel admission。
    ToplevelAdmissionRejected {
        /// Ledger 返回的错误。
        source: SurfaceXdgAdmissionError,
    },
    /// Ledger admit_toplevel 产生的 core WindowId 与预期不匹配。
    UnexpectedCoreWindow {
        /// 已分配的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
        /// 核心返回的 core window identity。
        core_window: crate::core::workspace::WindowId,
    },
    /// Admission 后的 core validation 不 clean。
    CoreValidationFailed,
}

/// Adapter-ledger admission ownership proof 报告。
///
/// 成功表示 adapter identity → ledger admission → BackendEvent::ToplevelMapped →
/// core WindowId 路径在受控条件下接通。不推导 real Wayland dispatch、render 或 input。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterLedgerAdmissionReport {
    /// 是否已检查 surface identity 与 ledger mapping。
    pub surface_admission_prerequisite_met: bool,
    /// 分配的 adapter surface ID。
    pub adapter_surface_id: AdapterSurfaceId,
    /// 对应的 core surface ID。
    pub core_surface_id: SurfaceId,
    /// 链接的 adapter toplevel identity key。
    pub adapter_toplevel_id: AdapterToplevelId,
    /// Core WindowId（由 admit_toplevel 内部 BackendEvent seam 自主分配）。
    pub core_window_id: crate::core::workspace::WindowId,
    /// Ledger 是否包含正确的 toplevel → core window mapping。
    pub ledger_toplevel_mapping_consistent: bool,
    /// Ledger 是否包含正确的 surface → toplevel mapping。
    pub ledger_surface_mapping_consistent: bool,
    /// 是否已调用 ledger admit_toplevel。
    pub ledger_admit_invoked: bool,
    /// Ledger admit_toplevel 的完整结果。
    pub ledger_report: SurfaceXdgAdmissionReport,
    /// 后续 core validation 是否 clean。
    pub core_validation_clean: bool,
    /// 是否分配 core window identity（admit_toplevel 内部 seam 会分配）。
    pub core_window_allocated: bool,
    /// 是否调用 ledger admit_surface。
    pub ledger_admit_surface_invoked: bool,
    /// 是否已有可用的真实 xdg-shell runtime。
    pub real_xdg_shell_runtime_available: bool,
    /// 是否已有可用的真实 compositor runtime。
    pub real_compositor_runtime_available: bool,
    /// Render 是否可用。
    pub render_support: bool,
    /// Input 是否可用。
    pub input_support: bool,
    /// 当前仍未满足的后续前置条件。
    pub blockers: Vec<AdapterLedgerAdmissionBlocker>,
}

/// 从 adapter identity 走 ledger admit_toplevel 的 pure-data proof。
///
/// 本函数不连接真实 Wayland socket，也不通过 real callback 驱动 handler mutation。
/// 它直接使用既有的 `SurfaceXdgAdmissionLedger` 和 `XdgToplevelAdmissionIntent` 接口，
/// 证明从 adapter identity 到 core WindowId 的 ownership 在纯数据层是接通的。
pub fn adapter_ledger_admission_report(
    ledger: &mut SurfaceXdgAdmissionLedger,
    state: &mut State,
) -> Result<AdapterLedgerAdmissionReport, AdapterLedgerAdmissionError> {
    // 本 proof 使用硬编码 adapter identity 值。受控 callback 驱动的 identity
    // 注册（Phase 52T）产生的 live mapping 在 Linux-only runtime 运行后的
    // 后续阶段可以作为 identity source。以下值确保测试环境身份不冲突。
    let adapter_surface = AdapterSurfaceId::new(
        crate::smithay_backend::surface_xdg_admission::ProtocolObjectId::new(1001).ok_or(
            AdapterLedgerAdmissionError::Blocked(
                AdapterLedgerAdmissionBlocker::MissingSurfaceAdmission,
            ),
        )?,
    );
    let adapter_toplevel = AdapterToplevelId::new(
        crate::smithay_backend::surface_xdg_admission::ProtocolObjectId::new(2001).ok_or(
            AdapterLedgerAdmissionError::Blocked(
                AdapterLedgerAdmissionBlocker::MissingToplevelIdentityRegistration,
            ),
        )?,
    );

    // Step 1: admit_surface
    // 不预注册 core surface；bridge 会根据 BackendEvent::SurfaceCreated 做唯一注册。
    // 使用显式 ID 42 供 bridge 注册，避免与自动分配 ID 1 冲突。
    let core_surface: SurfaceId = 42;
    let surface_intent = SurfaceAdmissionIntent {
        adapter_surface,
        core_surface,
        client: None,
        role: SurfaceRole::XdgToplevel,
    };
    let _surface_report = ledger
        .admit_surface(state, surface_intent)
        .map_err(|source| AdapterLedgerAdmissionError::SurfaceAdmissionRejected { source })?;

    // Step 2: admit_toplevel
    let toplevel_intent = XdgToplevelAdmissionIntent {
        adapter_toplevel,
        adapter_surface,
        title: "Phase 52U controlled toplevel".to_string(),
        app_id: Some("sky-mirror-phase52u".to_string()),
        kind: WindowKind::Mock,
    };
    let toplevel_report = ledger
        .admit_toplevel(state, toplevel_intent)
        .map_err(|source| AdapterLedgerAdmissionError::ToplevelAdmissionRejected { source })?;

    let core_window = toplevel_report.core_window().ok_or(
        AdapterLedgerAdmissionError::ToplevelAdmissionRejected {
            source: SurfaceXdgAdmissionError::UnexpectedCoreResult,
        },
    )?;
    let ledger_window =
        ledger
            .toplevel_mapping(adapter_toplevel)
            .ok_or(AdapterLedgerAdmissionError::Blocked(
                AdapterLedgerAdmissionBlocker::UnknownToplevelIdentity,
            ))?;
    if core_window != ledger_window {
        return Err(AdapterLedgerAdmissionError::UnexpectedCoreWindow {
            adapter_toplevel,
            core_window: ledger_window,
        });
    }

    let validation_clean = toplevel_report.validation_is_clean();
    if !validation_clean {
        return Err(AdapterLedgerAdmissionError::CoreValidationFailed);
    }

    let ledger_toplevel_consistent = ledger.toplevel_mapping(adapter_toplevel) == Some(core_window);
    let ledger_surface_consistent = {
        // 确认 admit_surface 已经建立 mapping
        ledger.surface_mapping(adapter_surface) == Some(core_surface)
    };

    Ok(AdapterLedgerAdmissionReport {
        surface_admission_prerequisite_met: true,
        adapter_surface_id: adapter_surface,
        core_surface_id: core_surface,
        adapter_toplevel_id: adapter_toplevel,
        core_window_id: core_window,
        ledger_toplevel_mapping_consistent: ledger_toplevel_consistent,
        ledger_surface_mapping_consistent: ledger_surface_consistent,
        ledger_admit_invoked: true,
        ledger_report: toplevel_report,
        core_validation_clean: validation_clean,
        core_window_allocated: true,
        ledger_admit_surface_invoked: true,
        real_xdg_shell_runtime_available: false,
        real_compositor_runtime_available: false,
        render_support: false,
        input_support: false,
        blockers: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use crate::core::state::State;
    use crate::smithay_backend::surface_xdg_admission::{
        AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId, SurfaceXdgAdmissionLedger,
        XdgToplevelAdmissionIntent,
    };

    use super::{AdapterLedgerAdmissionError, adapter_ledger_admission_report};

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    #[test]
    fn adapter_ledger_admission_accepts_toplevel_with_core_window() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();

        let report = adapter_ledger_admission_report(&mut ledger, &mut state)
            .expect("pure-data ledger admission proof 必须成功");

        assert!(report.surface_admission_prerequisite_met);
        assert!(report.ledger_admit_surface_invoked);
        assert!(report.ledger_admit_invoked);
        assert!(report.ledger_toplevel_mapping_consistent);
        assert!(report.ledger_surface_mapping_consistent);
        assert!(report.core_validation_clean);
        assert!(report.core_window_allocated);
        assert!(report.core_window_id > 0);
        assert_eq!(
            ledger.toplevel_mapping(report.adapter_toplevel_id),
            Some(report.core_window_id)
        );
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn adapter_ledger_admission_report_is_conservative() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();

        let report = adapter_ledger_admission_report(&mut ledger, &mut state)
            .expect("pure-data ledger admission proof 必须成功");

        assert!(report.ledger_admit_invoked);
        assert!(report.core_window_allocated);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.real_compositor_runtime_available);
        assert!(!report.render_support);
        assert!(!report.input_support);
    }

    #[test]
    fn ledger_toplevel_mapping_returns_allocated_core_window() {
        let mut ledger = SurfaceXdgAdmissionLedger::new();
        let mut state = State::new();

        let report = adapter_ledger_admission_report(&mut ledger, &mut state)
            .expect("pure-data ledger admission proof 必须成功");

        assert!(ledger.toplevel_mapping(toplevel(2001)).is_some());
        assert!(ledger.surface_mapping(surface(1001)).is_some());
    }
}
