//! Adapter-owned xdg toplevel identity registry 的纯数据事务核心。

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use super::surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId};

/// Adapter identity key 对应的纯数据 toplevel mapping。
///
/// 这里不保存 Smithay/Wayland 对象；真实 protocol identity 只由 Linux-only wrapper
/// 作为 registry key 持有，core 与 admission ledger 只会看到 adapter ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XdgToplevelIdentityMapping {
    /// Registry 分配或确认的 adapter toplevel identity。
    pub adapter_toplevel: AdapterToplevelId,
    /// 该 toplevel 声明归属的 adapter surface identity。
    pub adapter_surface: AdapterSurfaceId,
}

/// Adapter-owned toplevel identity 操作的结构化拒绝原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdgToplevelIdentityError {
    /// 同一个 active identity 已经注册。
    DuplicateIdentity {
        /// 已存在且保持不变的 mapping。
        existing: XdgToplevelIdentityMapping,
    },
    /// 请求的 identity 从未注册。
    UnknownIdentity,
    /// identity 已移除并进入 tombstone，禁止复用。
    StaleIdentity,
    /// identity 当前归属的 adapter surface 与请求不一致。
    AdapterSurfaceMismatch {
        /// 当前 mapping 的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
        /// Registry 中记录的 surface identity。
        mapped_surface: AdapterSurfaceId,
        /// 本次请求提供的 surface identity。
        requested_surface: AdapterSurfaceId,
    },
    /// 显式请求的 AdapterToplevelId 正由 active mapping 使用。
    AdapterToplevelIdInUse {
        /// 冲突的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
    },
    /// 显式请求的 AdapterToplevelId 已随旧 mapping 退休。
    RetiredAdapterToplevelId {
        /// 已退休、禁止复用的 adapter toplevel identity。
        adapter_toplevel: AdapterToplevelId,
    },
    /// 单调 adapter identity 空间已耗尽。
    IdentityExhausted,
}

/// 与平台对象类型解耦的 adapter-owned registry 实现。
///
/// `K` 在 Linux production 中是 Wayland `ObjectId`，测试中可使用纯数值 key。
/// Registry 只在所有前置检查成功后提交 HashMap/HashSet 变更，失败不会留下半成品。
#[derive(Debug)]
pub(crate) struct AdapterOwnedToplevelIdentityRegistry<K> {
    next_adapter_id: Option<u64>,
    mappings: HashMap<K, XdgToplevelIdentityMapping>,
    identity_by_adapter_id: HashMap<AdapterToplevelId, K>,
    tombstones: HashSet<K>,
    retired_adapter_ids: HashSet<AdapterToplevelId>,
}

impl<K> Default for AdapterOwnedToplevelIdentityRegistry<K> {
    fn default() -> Self {
        Self {
            next_adapter_id: Some(1),
            mappings: HashMap::new(),
            identity_by_adapter_id: HashMap::new(),
            tombstones: HashSet::new(),
            retired_adapter_ids: HashSet::new(),
        }
    }
}

impl<K> AdapterOwnedToplevelIdentityRegistry<K>
where
    K: Clone + Eq + Hash,
{
    /// 创建空 registry；自动分配从 AdapterToplevelId 1 开始。
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 为 identity 分配从不复用的 AdapterToplevelId 并提交 mapping。
    pub(crate) fn register(
        &mut self,
        identity: K,
        adapter_surface: AdapterSurfaceId,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        let mut candidate = self
            .next_adapter_id
            .ok_or(XdgToplevelIdentityError::IdentityExhausted)?;

        loop {
            let protocol_object = ProtocolObjectId::new(candidate)
                .ok_or(XdgToplevelIdentityError::IdentityExhausted)?;
            let adapter_toplevel = AdapterToplevelId::new(protocol_object);
            if !self.identity_by_adapter_id.contains_key(&adapter_toplevel)
                && !self.retired_adapter_ids.contains(&adapter_toplevel)
            {
                return self.register_with_id(identity, adapter_surface, adapter_toplevel);
            }
            candidate = candidate
                .checked_add(1)
                .ok_or(XdgToplevelIdentityError::IdentityExhausted)?;
        }
    }

    /// 使用显式 AdapterToplevelId 注册 identity；active/retired ID 均禁止复用。
    pub(crate) fn register_with_id(
        &mut self,
        identity: K,
        adapter_surface: AdapterSurfaceId,
        adapter_toplevel: AdapterToplevelId,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        if let Some(existing) = self.mappings.get(&identity).copied() {
            if existing.adapter_surface != adapter_surface {
                return Err(XdgToplevelIdentityError::AdapterSurfaceMismatch {
                    adapter_toplevel: existing.adapter_toplevel,
                    mapped_surface: existing.adapter_surface,
                    requested_surface: adapter_surface,
                });
            }
            return Err(XdgToplevelIdentityError::DuplicateIdentity { existing });
        }
        if self.tombstones.contains(&identity) {
            return Err(XdgToplevelIdentityError::StaleIdentity);
        }
        if self.identity_by_adapter_id.contains_key(&adapter_toplevel) {
            return Err(XdgToplevelIdentityError::AdapterToplevelIdInUse { adapter_toplevel });
        }
        if self.retired_adapter_ids.contains(&adapter_toplevel) {
            return Err(XdgToplevelIdentityError::RetiredAdapterToplevelId { adapter_toplevel });
        }

        let next_adapter_id = match self.next_adapter_id {
            Some(next) if adapter_toplevel.value() >= next => {
                adapter_toplevel.value().checked_add(1)
            }
            current => current,
        };
        let mapping = XdgToplevelIdentityMapping {
            adapter_toplevel,
            adapter_surface,
        };

        // 所有可失败检查已经完成；从这里开始一次性提交双向 mapping。
        self.mappings.insert(identity.clone(), mapping);
        self.identity_by_adapter_id
            .insert(adapter_toplevel, identity);
        self.next_adapter_id = next_adapter_id;
        Ok(mapping)
    }

    /// 只读查询 identity；unknown 与已移除 stale identity 明确区分。
    pub(crate) fn lookup(
        &self,
        identity: &K,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        if let Some(mapping) = self.mappings.get(identity).copied() {
            return Ok(mapping);
        }
        if self.tombstones.contains(identity) {
            return Err(XdgToplevelIdentityError::StaleIdentity);
        }
        Err(XdgToplevelIdentityError::UnknownIdentity)
    }

    /// 移除 active mapping 并同时写入 identity 与 AdapterToplevelId tombstone。
    pub(crate) fn remove(
        &mut self,
        identity: &K,
        adapter_surface: AdapterSurfaceId,
    ) -> Result<XdgToplevelIdentityMapping, XdgToplevelIdentityError> {
        let mapping = self.lookup(identity)?;
        if mapping.adapter_surface != adapter_surface {
            return Err(XdgToplevelIdentityError::AdapterSurfaceMismatch {
                adapter_toplevel: mapping.adapter_toplevel,
                mapped_surface: mapping.adapter_surface,
                requested_surface: adapter_surface,
            });
        }

        // surface ownership 校验成功后才提交 removal，失败路径不会产生 tombstone。
        self.mappings.remove(identity);
        self.identity_by_adapter_id
            .remove(&mapping.adapter_toplevel);
        self.tombstones.insert(identity.clone());
        self.retired_adapter_ids.insert(mapping.adapter_toplevel);
        Ok(mapping)
    }

    /// 返回 active mapping 数量。
    pub(crate) fn active_len(&self) -> usize {
        self.mappings.len()
    }

    /// 返回已移除 identity tombstone 数量。
    pub(crate) fn tombstone_count(&self) -> usize {
        self.tombstones.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{AdapterOwnedToplevelIdentityRegistry, XdgToplevelIdentityError};
    use crate::smithay_backend::surface_xdg_admission::{
        AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId,
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(ProtocolObjectId::new(value).expect("测试 identity 必须非零"))
    }

    #[test]
    fn linux_xdg_toplevel_identity_allocates_adapter_toplevel_id() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();

        let mapping = registry
            .register(11_u64, surface(41))
            .expect("首次注册必须成功");

        assert_eq!(mapping.adapter_toplevel, toplevel(1));
        assert_eq!(mapping.adapter_surface, surface(41));
        assert_eq!(registry.lookup(&11), Ok(mapping));
        assert_eq!(registry.active_len(), 1);
    }

    #[test]
    fn linux_xdg_toplevel_identity_rejects_duplicate_identity() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        let original = registry
            .register(11_u64, surface(41))
            .expect("首次注册必须成功");

        assert_eq!(
            registry.register(11_u64, surface(41)),
            Err(XdgToplevelIdentityError::DuplicateIdentity { existing: original })
        );
        assert_eq!(registry.lookup(&11), Ok(original));
        assert_eq!(registry.active_len(), 1);
    }

    #[test]
    fn linux_xdg_toplevel_identity_rejects_duplicate_surface_mismatch() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        let original = registry
            .register(11_u64, surface(41))
            .expect("首次注册必须成功");

        assert_eq!(
            registry.register(11_u64, surface(42)),
            Err(XdgToplevelIdentityError::AdapterSurfaceMismatch {
                adapter_toplevel: original.adapter_toplevel,
                mapped_surface: surface(41),
                requested_surface: surface(42),
            })
        );
        assert_eq!(registry.lookup(&11), Ok(original));
        assert_eq!(registry.active_len(), 1);
    }

    #[test]
    fn linux_xdg_toplevel_identity_rejects_unknown_lookup() {
        let registry = AdapterOwnedToplevelIdentityRegistry::<u64>::new();

        assert_eq!(
            registry.lookup(&99),
            Err(XdgToplevelIdentityError::UnknownIdentity)
        );
    }

    #[test]
    fn linux_xdg_toplevel_identity_rejects_surface_mismatch_transactionally() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        let original = registry
            .register(11_u64, surface(41))
            .expect("首次注册必须成功");

        assert_eq!(
            registry.remove(&11, surface(42)),
            Err(XdgToplevelIdentityError::AdapterSurfaceMismatch {
                adapter_toplevel: original.adapter_toplevel,
                mapped_surface: surface(41),
                requested_surface: surface(42),
            })
        );
        assert_eq!(registry.lookup(&11), Ok(original));
        assert_eq!(registry.tombstone_count(), 0);
    }

    #[test]
    fn linux_xdg_toplevel_identity_tombstone_rejects_reuse() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        let original = registry
            .register(11_u64, surface(41))
            .expect("首次注册必须成功");

        assert_eq!(registry.remove(&11, surface(41)), Ok(original));
        assert_eq!(
            registry.lookup(&11),
            Err(XdgToplevelIdentityError::StaleIdentity)
        );
        assert_eq!(
            registry.register(11_u64, surface(41)),
            Err(XdgToplevelIdentityError::StaleIdentity)
        );
        assert_eq!(
            registry.remove(&11, surface(41)),
            Err(XdgToplevelIdentityError::StaleIdentity)
        );
        assert_eq!(registry.active_len(), 0);
        assert_eq!(registry.tombstone_count(), 1);
    }

    #[test]
    fn linux_xdg_toplevel_identity_rejects_active_and_retired_adapter_id_reuse() {
        let mut registry = AdapterOwnedToplevelIdentityRegistry::new();
        let requested = toplevel(7);
        let original = registry
            .register_with_id(11_u64, surface(41), requested)
            .expect("显式 ID 首次注册必须成功");

        assert_eq!(
            registry.register_with_id(12_u64, surface(42), requested),
            Err(XdgToplevelIdentityError::AdapterToplevelIdInUse {
                adapter_toplevel: requested,
            })
        );
        assert_eq!(registry.remove(&11, surface(41)), Ok(original));
        assert_eq!(
            registry.register_with_id(12_u64, surface(42), requested),
            Err(XdgToplevelIdentityError::RetiredAdapterToplevelId {
                adapter_toplevel: requested,
            })
        );
        assert_eq!(registry.active_len(), 0);
    }
}
