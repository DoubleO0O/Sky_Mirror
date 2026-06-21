//! Linux-only Smithay toplevel identity source 与 adapter-owned mapping wrapper。
//!
//! Registry 只保存 `xdg_toplevel` resource 的 `ObjectId` 包装 key；不会保存
//! `ToplevelSurface`，也不会调用 admission ledger、core 或 protocol runtime。

use smithay::reexports::wayland_server::{Resource, backend::ObjectId};
use smithay::wayland::shell::xdg::ToplevelSurface;

use super::surface_xdg_admission::AdapterSurfaceId;
use super::xdg_toplevel_identity::{
    AdapterOwnedToplevelIdentityRegistry, XdgToplevelIdentityError, XdgToplevelIdentityMapping,
};

/// Linux adapter 从 Smithay toplevel resource 提取的稳定 identity key。
///
/// Wayland backend 保证 `ObjectId` 在协议数值 ID 被复用后仍区分旧、新对象；因此
/// registry 保存该轻量 key，而不是保存具有生命周期和 callback 语义的真实对象。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinuxXdgToplevelIdentityKey(ObjectId);

/// 从 Smithay protocol object 提取稳定 identity 时的结构化错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgToplevelIdentitySourceError {
    /// 候选来源只能提供会跨 client/重用周期冲突的数值 ID。
    IdentitySourceNotStable,
    /// Smithay resource 没有可用的非空 ObjectId。
    SmithayIdentityUnavailable,
}

impl LinuxXdgToplevelIdentityKey {
    /// 从 Smithay `ToplevelSurface` 的 xdg_toplevel resource 提取 identity key。
    pub fn from_toplevel(
        toplevel: &ToplevelSurface,
    ) -> Result<Self, LinuxXdgToplevelIdentitySourceError> {
        let object_id = toplevel.xdg_toplevel().id();
        if object_id.is_null() {
            return Err(LinuxXdgToplevelIdentitySourceError::SmithayIdentityUnavailable);
        }
        Ok(Self(object_id))
    }
}

/// Linux-only adapter-owned toplevel identity registry。
///
/// 该 registry 先把真实 protocol identity 收敛为 key，再分配纯数据
/// `AdapterToplevelId`。Mapping 本身不代表 callback 已发生，也不会触发 ledger。
#[derive(Debug, Default)]
pub struct LinuxXdgToplevelIdentityRegistry {
    inner: AdapterOwnedToplevelIdentityRegistry<LinuxXdgToplevelIdentityKey>,
}

impl LinuxXdgToplevelIdentityRegistry {
    /// 创建没有 active mapping 或 tombstone 的 registry。
    pub fn new() -> Self {
        Self::default()
    }

    /// 只提取 stable key，不注册 mapping。
    pub fn key_for_toplevel(
        toplevel: &ToplevelSurface,
    ) -> Result<LinuxXdgToplevelIdentityKey, LinuxXdgToplevelIdentitySourceError> {
        LinuxXdgToplevelIdentityKey::from_toplevel(toplevel)
    }

    /// 为已经提取的 identity key 分配 AdapterToplevelId。
    pub fn register(
        &mut self,
        identity: LinuxXdgToplevelIdentityKey,
        adapter_surface: AdapterSurfaceId,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        self.inner.register(identity, adapter_surface)
    }

    /// 从 Smithay toplevel 提取 key 并注册；本方法不由 Phase 52F handler 调用。
    pub fn register_toplevel(
        &mut self,
        toplevel: &ToplevelSurface,
        adapter_surface: AdapterSurfaceId,
    ) -> Result<XdgToplevelIdentityMapping, LinuxXdgToplevelIdentityOperationError> {
        let identity = Self::key_for_toplevel(toplevel)?;
        self.register(identity, adapter_surface).map_err(Into::into)
    }

    /// 只读查询 key 对应的 adapter mapping。
    pub fn lookup(
        &self,
        identity: &LinuxXdgToplevelIdentityKey,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        self.inner.lookup(identity)
    }

    /// 查询 Smithay toplevel 对应的 adapter mapping，不产生 mutation。
    pub fn lookup_toplevel(
        &self,
        toplevel: &ToplevelSurface,
    ) -> Result<XdgToplevelIdentityMapping, LinuxXdgToplevelIdentityOperationError> {
        let identity = Self::key_for_toplevel(toplevel)?;
        self.lookup(&identity).map_err(Into::into)
    }

    /// 移除 mapping 并留下 identity/AdapterToplevelId tombstone。
    ///
    /// 这只是 adapter identity ownership 变更，不调用 admission ledger 或 core。
    pub fn remove(
        &mut self,
        identity: &LinuxXdgToplevelIdentityKey,
        adapter_surface: AdapterSurfaceId,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        self.inner.remove(identity, adapter_surface)
    }

    /// 返回 active mapping 数量。
    pub fn active_len(&self) -> usize {
        self.inner.active_len()
    }

    /// 判断 registry 是否没有 active mapping。
    pub fn is_empty(&self) -> bool {
        self.active_len() == 0
    }

    /// 返回已退休 protocol identity 数量。
    pub fn tombstone_count(&self) -> usize {
        self.inner.tombstone_count()
    }
}

/// Linux wrapper 同时表达 identity source 与纯数据 registry 的失败。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgToplevelIdentityOperationError {
    /// Smithay identity source 无法提供稳定 key。
    Source(LinuxXdgToplevelIdentitySourceError),
    /// 纯数据 registry 拒绝 mapping 操作。
    Mapping(XdgToplevelIdentityError),
}

impl From<LinuxXdgToplevelIdentitySourceError> for LinuxXdgToplevelIdentityOperationError {
    fn from(source: LinuxXdgToplevelIdentitySourceError) -> Self {
        Self::Source(source)
    }
}

impl From<XdgToplevelIdentityError> for LinuxXdgToplevelIdentityOperationError {
    fn from(source: XdgToplevelIdentityError) -> Self {
        Self::Mapping(source)
    }
}

/// Phase 52F mapping 建立后仍存在的 runtime/lifecycle blocker。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxXdgToplevelIdentityBlocker {
    /// Handler callback 尚未调用 registry。
    MissingProductionLifecycleBridge,
    /// Linux runtime 尚未明确持有并调用 admission ledger。
    MissingLedgerCallerOwnership,
    /// 尚未观察到真实 xdg_toplevel lifecycle callback。
    MissingRealCallbackObservation,
    /// xdg-shell global 尚未初始化。
    MissingXdgShellGlobalInitialization,
}

/// Linux toplevel identity mapping 的保守 readiness report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxXdgToplevelIdentityReadinessReport {
    /// Adapter-owned identity mapping 是否存在。
    pub identity_mapping_available: bool,
    /// 当前 ObjectId source 是否具备跨 client/协议 ID 重用的稳定性。
    pub identity_source_stable: bool,
    /// ToplevelSurface identity hook 是否可提取 stable key。
    pub toplevel_surface_identity_hook_available: bool,
    /// AdapterToplevelId 单调分配是否可用。
    pub adapter_toplevel_id_allocation_available: bool,
    /// AdapterToplevelId lookup 是否可用。
    pub adapter_toplevel_lookup_available: bool,
    /// Duplicate identity 是否结构化拒绝。
    pub duplicate_toplevel_rejected: bool,
    /// Tombstone/stale identity 是否结构化拒绝。
    pub stale_toplevel_rejected: bool,
    /// Linux boundary 是否已调用 ledger unmap。
    pub ledger_unmap_invoked: bool,
    /// 是否观察到真实 callback。
    pub callback_observed: bool,
    /// 真实 xdg-shell runtime 是否可用。
    pub real_xdg_shell_runtime_available: bool,
    /// protocol dispatch 是否启动。
    pub protocol_dispatch_started: bool,
    /// render 是否可用。
    pub render_support: bool,
    /// input 是否可用。
    pub input_support: bool,
    /// 尚未完成的 lifecycle/runtime 前置条件。
    pub blockers: Vec<LinuxXdgToplevelIdentityBlocker>,
}

/// 返回 Phase 52F identity mapping readiness；不推导 callback/runtime 能力。
pub fn linux_xdg_toplevel_identity_readiness_report() -> LinuxXdgToplevelIdentityReadinessReport {
    LinuxXdgToplevelIdentityReadinessReport {
        identity_mapping_available: true,
        identity_source_stable: true,
        toplevel_surface_identity_hook_available: true,
        adapter_toplevel_id_allocation_available: true,
        adapter_toplevel_lookup_available: true,
        duplicate_toplevel_rejected: true,
        stale_toplevel_rejected: true,
        ledger_unmap_invoked: false,
        callback_observed: false,
        real_xdg_shell_runtime_available: false,
        protocol_dispatch_started: false,
        render_support: false,
        input_support: false,
        blockers: vec![
            LinuxXdgToplevelIdentityBlocker::MissingProductionLifecycleBridge,
            LinuxXdgToplevelIdentityBlocker::MissingLedgerCallerOwnership,
            LinuxXdgToplevelIdentityBlocker::MissingRealCallbackObservation,
            LinuxXdgToplevelIdentityBlocker::MissingXdgShellGlobalInitialization,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LinuxXdgToplevelIdentityBlocker, LinuxXdgToplevelIdentityRegistry,
        linux_xdg_toplevel_identity_readiness_report,
    };

    #[test]
    fn linux_xdg_toplevel_identity_registry_starts_empty() {
        let registry = LinuxXdgToplevelIdentityRegistry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.active_len(), 0);
        assert_eq!(registry.tombstone_count(), 0);
    }

    #[test]
    fn linux_xdg_toplevel_identity_keeps_runtime_false() {
        let report = linux_xdg_toplevel_identity_readiness_report();

        assert!(report.identity_mapping_available);
        assert!(report.identity_source_stable);
        assert!(report.toplevel_surface_identity_hook_available);
        assert!(report.adapter_toplevel_id_allocation_available);
        assert!(report.adapter_toplevel_lookup_available);
        assert!(report.duplicate_toplevel_rejected);
        assert!(report.stale_toplevel_rejected);
        assert!(!report.ledger_unmap_invoked);
        assert!(!report.callback_observed);
        assert!(!report.real_xdg_shell_runtime_available);
        assert!(!report.protocol_dispatch_started);
        assert!(!report.render_support);
        assert!(!report.input_support);
        assert_eq!(
            report.blockers,
            vec![
                LinuxXdgToplevelIdentityBlocker::MissingProductionLifecycleBridge,
                LinuxXdgToplevelIdentityBlocker::MissingLedgerCallerOwnership,
                LinuxXdgToplevelIdentityBlocker::MissingRealCallbackObservation,
                LinuxXdgToplevelIdentityBlocker::MissingXdgShellGlobalInitialization,
            ]
        );
    }
}
