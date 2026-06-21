//! XDG toplevel lifecycle identity lookup 的纯数据 observation report。

use super::surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId};
use super::xdg_toplevel_identity::{XdgToplevelIdentityError, XdgToplevelIdentityMapping};

/// Linux xdg-shell handler 可观察的最小 toplevel lifecycle signal。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdgToplevelLifecycleSignal {
    /// Smithay `XdgShellHandler::toplevel_destroyed` handler seam。
    ToplevelDestroyed,
}

/// 一次成功 identity lookup 解析出的纯数据 observation。
///
/// 真实 `ToplevelSurface` 不进入该结构；后续层只能看到 adapter identity。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XdgToplevelLifecycleObservation {
    /// 触发 lookup 的 lifecycle signal。
    pub signal: XdgToplevelLifecycleSignal,
    /// Registry 已解析的 adapter toplevel identity。
    pub adapter_toplevel: AdapterToplevelId,
    /// Registry 中记录的 adapter surface owner。
    pub adapter_surface: AdapterSurfaceId,
}

/// Lifecycle observation boundary 的结构化拒绝原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdgToplevelLifecycleObservationError {
    /// Linux protocol object 无法提供 identity key。
    IdentitySourceUnavailable,
    /// Identity source 不能稳定区分 client 或 protocol ID 重用。
    IdentitySourceNotStable,
    /// Handler state 没有持有 identity registry。
    RegistryNotOwnedByHandlerState,
    /// Callback signal 指向尚未注册的 identity。
    UnknownIdentity,
    /// Callback signal 指向已移除并 tombstone 的 identity。
    TombstonedIdentity,
    /// Lookup mapping 的 surface owner 与 callback 上下文不一致。
    AdapterSurfaceMismatch {
        /// 已解析的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
        /// Registry 中记录的 adapter surface owner。
        mapped_surface: AdapterSurfaceId,
        /// Callback 上下文要求的 adapter surface owner。
        expected_surface: AdapterSurfaceId,
    },
    /// Registry 返回了 lookup 路径不应出现的其他结构化错误。
    RegistryLookupRejected(XdgToplevelIdentityError),
    /// Observation layer 明确禁止调用 admission ledger。
    LedgerMutationForbidden,
    /// Observation layer 明确禁止调用 core detach seam。
    CoreMutationForbidden,
}

/// Callback-like lifecycle signal 到 adapter identity lookup 的纯数据报告。
///
/// `callback_observed` 保持 false，因为 helper/handler wiring 不证明真实 runtime
/// 已调用 callback；ledger/core/runtime/render/input capability 同样不能由 lookup 推导。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgToplevelLifecycleObservationReport {
    /// 本次处理的 lifecycle signal。
    pub signal: XdgToplevelLifecycleSignal,
    /// 是否尝试了 registry lookup。
    pub identity_lookup_invoked: bool,
    /// 是否成功解析 AdapterToplevelId。
    pub adapter_toplevel_id_resolved: bool,
    /// 成功 observation 或结构化错误。
    pub observation: Result<XdgToplevelLifecycleObservation, XdgToplevelLifecycleObservationError>,
    /// 是否已证明真实 runtime callback invocation。
    pub callback_observed: bool,
    /// Observation layer 是否调用 ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// Observation layer 是否调用 core detach。
    pub core_detach_invoked: bool,
    /// 真实 xdg-shell runtime 是否可用。
    pub real_xdg_shell_runtime_available: bool,
    /// protocol dispatch 是否启动。
    pub protocol_dispatch_started: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
}

impl XdgToplevelLifecycleObservationReport {
    /// 从只读 registry lookup 结果构造 observation report。
    pub fn from_lookup(
        signal: XdgToplevelLifecycleSignal,
        lookup: Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError>,
        expected_surface: Option<AdapterSurfaceId>,
    ) -> Self {
        let observation = match lookup {
            Ok(mapping) => {
                if let Some(expected_surface) = expected_surface
                    && mapping.adapter_surface != expected_surface
                {
                    Err(
                        XdgToplevelLifecycleObservationError::AdapterSurfaceMismatch {
                            adapter_toplevel: mapping.adapter_toplevel,
                            mapped_surface: mapping.adapter_surface,
                            expected_surface,
                        },
                    )
                } else {
                    Ok(XdgToplevelLifecycleObservation {
                        signal,
                        adapter_toplevel: mapping.adapter_toplevel,
                        adapter_surface: mapping.adapter_surface,
                    })
                }
            }
            Err(XdgToplevelIdentityError::UnknownIdentity) => {
                Err(XdgToplevelLifecycleObservationError::UnknownIdentity)
            }
            Err(XdgToplevelIdentityError::StaleIdentity) => {
                Err(XdgToplevelLifecycleObservationError::TombstonedIdentity)
            }
            Err(source) => {
                Err(XdgToplevelLifecycleObservationError::RegistryLookupRejected(source))
            }
        };
        let adapter_toplevel_id_resolved = observation.is_ok();

        Self {
            signal,
            identity_lookup_invoked: true,
            adapter_toplevel_id_resolved,
            observation,
            callback_observed: false,
            ledger_unmap_invoked: false,
            core_detach_invoked: false,
            real_xdg_shell_runtime_available: false,
            protocol_dispatch_started: false,
            render_support: false,
            input_support: false,
        }
    }

    /// 构造 identity source 提取失败的报告；此时 registry lookup 未发生。
    pub fn identity_source_failed(
        signal: XdgToplevelLifecycleSignal,
        error: XdgToplevelLifecycleObservationError,
    ) -> Self {
        Self {
            signal,
            identity_lookup_invoked: false,
            adapter_toplevel_id_resolved: false,
            observation: Err(error),
            callback_observed: false,
            ledger_unmap_invoked: false,
            core_detach_invoked: false,
            real_xdg_shell_runtime_available: false,
            protocol_dispatch_started: false,
            render_support: false,
            input_support: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        XdgToplevelLifecycleObservationError, XdgToplevelLifecycleObservationReport,
        XdgToplevelLifecycleSignal,
    };
    use crate::smithay_backend::xdg_toplevel_identity::AdapterOwnedToplevelIdentityRegistry;
    use crate::smithay_backend::{
        AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId, XdgToplevelIdentityError,
        XdgToplevelIdentityMapping,
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    fn mapping() -> XdgToplevelIdentityMapping {
        XdgToplevelIdentityMapping {
            adapter_toplevel: toplevel(7),
            adapter_surface: surface(41),
        }
    }

    #[test]
    fn linux_xdg_lifecycle_observation_resolves_adapter_toplevel_id() {
        let report = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            Ok(mapping()),
            Some(surface(41)),
        );

        assert!(report.identity_lookup_invoked);
        assert!(report.adapter_toplevel_id_resolved);
        let observation = report.observation.expect("已注册 identity 必须 resolve");
        assert_eq!(observation.adapter_toplevel, toplevel(7));
        assert_eq!(observation.adapter_surface, surface(41));
        assert_eq!(
            observation.signal,
            XdgToplevelLifecycleSignal::ToplevelDestroyed
        );
        assert!(!report.callback_observed);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_detach_invoked);
    }

    #[test]
    fn linux_xdg_lifecycle_observation_rejects_unknown_identity() {
        let report = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            Err(XdgToplevelIdentityError::UnknownIdentity),
            None,
        );

        assert_eq!(
            report.observation,
            Err(XdgToplevelLifecycleObservationError::UnknownIdentity)
        );
        assert!(report.identity_lookup_invoked);
        assert!(!report.adapter_toplevel_id_resolved);
    }

    #[test]
    fn linux_xdg_lifecycle_observation_rejects_tombstone_identity() {
        let report = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            Err(XdgToplevelIdentityError::StaleIdentity),
            None,
        );

        assert_eq!(
            report.observation,
            Err(XdgToplevelLifecycleObservationError::TombstonedIdentity)
        );
        assert!(!report.adapter_toplevel_id_resolved);
    }

    #[test]
    fn linux_xdg_lifecycle_observation_rejects_surface_mismatch() {
        let report = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            Ok(mapping()),
            Some(surface(42)),
        );

        assert_eq!(
            report.observation,
            Err(
                XdgToplevelLifecycleObservationError::AdapterSurfaceMismatch {
                    adapter_toplevel: toplevel(7),
                    mapped_surface: surface(41),
                    expected_surface: surface(42),
                }
            )
        );
        assert!(!report.adapter_toplevel_id_resolved);
    }

    #[test]
    fn linux_xdg_lifecycle_observation_keeps_runtime_false() {
        let report = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            Ok(mapping()),
            None,
        );

        assert!(!report.callback_observed);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_detach_invoked);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
    }

    #[test]
    fn linux_xdg_lifecycle_observation_failure_does_not_mutate_registry() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        registry
            .register(11_u64, surface(41))
            .expect("测试 mapping 必须注册成功");
        let active_before = registry.active_len();
        let tombstones_before = registry.tombstone_count();

        let unknown = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            registry.lookup(&99),
            None,
        );

        assert_eq!(
            unknown.observation,
            Err(XdgToplevelLifecycleObservationError::UnknownIdentity)
        );
        assert_eq!(registry.active_len(), active_before);
        assert_eq!(registry.tombstone_count(), tombstones_before);

        registry
            .remove(&11, surface(41))
            .expect("测试 mapping 必须移除成功");
        let active_after_remove = registry.active_len();
        let tombstones_after_remove = registry.tombstone_count();
        let stale = XdgToplevelLifecycleObservationReport::from_lookup(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            registry.lookup(&11),
            None,
        );

        assert_eq!(
            stale.observation,
            Err(XdgToplevelLifecycleObservationError::TombstonedIdentity)
        );
        assert_eq!(registry.active_len(), active_after_remove);
        assert_eq!(registry.tombstone_count(), tombstones_after_remove);
    }

    #[test]
    fn linux_xdg_lifecycle_observation_source_failure_skips_lookup() {
        let report = XdgToplevelLifecycleObservationReport::identity_source_failed(
            XdgToplevelLifecycleSignal::ToplevelDestroyed,
            XdgToplevelLifecycleObservationError::IdentitySourceUnavailable,
        );

        assert!(!report.identity_lookup_invoked);
        assert!(!report.adapter_toplevel_id_resolved);
        assert_eq!(
            report.observation,
            Err(XdgToplevelLifecycleObservationError::IdentitySourceUnavailable)
        );
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.core_detach_invoked);
    }
}
