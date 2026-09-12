//! Linux-only SHM render 原子 admission 的纯数据决策核。
//!
//! 此模块不持有 `WlBuffer`、renderer、ledger 或 Core `State`。coordinator 在保持真实
//! resource FIFO 不动时组装一次 snapshot，本模块只决定 defer、单侧回收、双侧拒绝或
//! 授权一次资源转移。所有实际 owner mutation 仍留在 coordinator/display/Winit 边界。

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    core::{client::ClientId as CoreClientId, surface::SurfaceId, workspace::WindowId},
    smithay_backend::{
        client_session::NestedClientSessionId,
        surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId},
    },
};

/// coordinator 持有的 adapter surface→toplevel 活跃关系与 retirement tombstone。
///
/// owner 只保存 adapter ID，不保存 Core mapping；每次实际 render 仍必须重新查询
/// admission ledger 与 `State`，因此这里不是第二条 Core 状态真相。
#[derive(Debug, Default)]
pub(crate) struct RuntimeShmRenderIdentityOwner {
    active: BTreeMap<AdapterSurfaceId, AdapterToplevelId>,
    retired: BTreeSet<(AdapterSurfaceId, AdapterToplevelId)>,
}

impl RuntimeShmRenderIdentityOwner {
    /// 创建空 owner。
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 在 coordinator 已确认 ledger admission 成功后记录 active adapter 关系。
    ///
    /// 同值重复或冲突重绑均返回 `false`，且不覆盖既有关系；已退休 pair 禁止复用。
    pub(crate) fn record_admission(
        &mut self,
        adapter_surface: AdapterSurfaceId,
        adapter_toplevel: AdapterToplevelId,
    ) -> bool {
        if self.active.contains_key(&adapter_surface)
            || self.retired.contains(&(adapter_surface, adapter_toplevel))
        {
            return false;
        }
        self.active.insert(adapter_surface, adapter_toplevel);
        true
    }

    /// 只在 exact active pair 上提交 removal，并留下不可复用 tombstone。
    pub(crate) fn record_unmap(
        &mut self,
        adapter_surface: AdapterSurfaceId,
        adapter_toplevel: AdapterToplevelId,
    ) -> bool {
        if self.active.get(&adapter_surface).copied() != Some(adapter_toplevel) {
            return false;
        }
        self.active.remove(&adapter_surface);
        self.retired.insert((adapter_surface, adapter_toplevel));
        true
    }

    /// 查询当前 active adapter toplevel；不查询或缓存任何 Core identity。
    pub(crate) fn active_toplevel(
        &self,
        adapter_surface: AdapterSurfaceId,
    ) -> Option<AdapterToplevelId> {
        self.active.get(&adapter_surface).copied()
    }

    /// 判断 exact adapter pair 是否已经退休，用于拒绝迟到 commit。
    pub(crate) fn is_retired(
        &self,
        adapter_surface: AdapterSurfaceId,
        adapter_toplevel: AdapterToplevelId,
    ) -> bool {
        self.retired.contains(&(adapter_surface, adapter_toplevel))
    }
}

/// commit observation 与真实 resource FIFO 共享的严格 token。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RuntimeShmRenderCommitToken {
    /// adapter surface identity；不是 Core `SurfaceId`。
    pub adapter_surface: AdapterSurfaceId,
    /// handler owner 分配的全局单调 commit sequence。
    pub commit_sequence: u64,
}

/// coordinator 在不转移真实 resource 时组装的只读 admission snapshot。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeShmRenderAdmissionInput {
    /// 待处理 observation 的队首 token。
    pub observation_token: RuntimeShmRenderCommitToken,
    /// 当前 resource 队首 token；`None` 可能是尚未入队、无效或已销毁。
    pub resource_token: Option<RuntimeShmRenderCommitToken>,
    /// observation token 对应 resource 是否已由 destroy/cleanup owner tombstone。
    pub resource_tombstoned: bool,
    /// coordinator 是否已经成功或终止处理过同一 token。
    pub token_already_terminal: bool,
    /// resource 从真实 Wayland owner 解出的 adapter session。
    pub source_session: Option<NestedClientSessionId>,
    /// 同一 flow 的 session bridge 只读解析结果。
    pub resolved_core_client: Option<CoreClientId>,
    /// lifecycle/admission FIFO 是否仍可能补齐当前 identity。
    pub lifecycle_resolution_pending: bool,
    /// 当前 active adapter surface 对应的 toplevel；不保存 Core mapping。
    pub adapter_toplevel: Option<AdapterToplevelId>,
    /// admission ledger 对 adapter surface 的 Core mapping。
    pub ledger_surface: Option<SurfaceId>,
    /// admission ledger 对 adapter toplevel 的 Core mapping。
    pub ledger_window: Option<WindowId>,
    /// ledger surface 在当前 Core 中是否仍存活。
    pub core_surface_alive: bool,
    /// 当前 Core surface 的 client owner。
    pub core_surface_client: Option<CoreClientId>,
    /// 当前 Core surface 绑定的 window。
    pub core_window_for_surface: Option<WindowId>,
    /// ledger window 在当前 Core registry 中是否仍存活。
    pub core_window_alive: bool,
    /// commit 是否包含真实 damage observation；只传给 completion gate，不在此放宽身份验证。
    pub damage_observed: bool,
}

/// 已确认不可进入 renderer 的结构化原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeShmRenderAdmissionRejectReason {
    /// resource 没有可解析的 adapter session。
    MissingSourceSession,
    /// session 已断开或从未由同一 flow 注册。
    UnknownSourceSession,
    /// lifecycle 排空后仍没有 active adapter toplevel。
    MissingAdapterToplevel,
    /// admission ledger 没有 surface mapping。
    MissingLedgerSurface,
    /// admission ledger 没有 toplevel mapping。
    MissingLedgerWindow,
    /// ledger surface 已在 Core 中结束生命周期。
    DeadCoreSurface,
    /// ledger window 已在 Core 中结束生命周期。
    DeadCoreWindow,
    /// Core surface 的 client 与 session bridge 结果冲突。
    CoreClientMismatch,
    /// ledger toplevel window 与 Core surface 当前 window 冲突。
    CoreWindowMismatch,
    /// destroy/cleanup 已回收该 observation 对应的 resource。
    DestroyedResource,
    /// observation 比 resource 队首更旧，说明该 commit 没有可用 resource。
    MissingResourceForCommit,
    /// resource 比 observation 队首更旧，必须先回收旧 resource 防止永久阻塞。
    StaleResourceToken,
    /// sequence 相同但 adapter surface 不同，身份来源发生不可恢复冲突。
    TokenIdentityMismatch,
    /// 同一 token 已经完成或被拒绝，迟到/重复事件不得再次执行副作用。
    DuplicateOrLateToken,
}

/// 原子 admission 的纯数据决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeShmRenderAdmissionDecision {
    /// lifecycle 仍可能补齐身份或 resource；两侧 FIFO 原样保留。
    DeferIdentity,
    /// 只拒绝 observation，真实 resource 队首保持不动。
    RejectObservation {
        /// 拒绝原因。
        reason: RuntimeShmRenderAdmissionRejectReason,
    },
    /// 只拒绝并 tombstone 真实 resource，observation 队首保持不动。
    RejectResource {
        /// 拒绝原因。
        reason: RuntimeShmRenderAdmissionRejectReason,
    },
    /// 精确匹配后同时拒绝 observation/resource，并 tombstone token。
    RejectBoth {
        /// 拒绝原因。
        reason: RuntimeShmRenderAdmissionRejectReason,
    },
    /// 所有 identity 与存活性验证成功，允许 coordinator 恰好 take 一次真实 resource。
    Ready {
        /// 已确认的 adapter toplevel。
        adapter_toplevel: AdapterToplevelId,
        /// session bridge 解析出的 Core client。
        core_client: CoreClientId,
        /// ledger 解析出的 Core surface。
        core_surface: SurfaceId,
        /// ledger 与 Core surface 一致的 live Core window。
        core_window: WindowId,
        /// 传给后续 completion gate 的真实 damage observation。
        damage_observed: bool,
    },
}

/// 成功呈现后的 frame callback 完成门输入。
///
/// 这是纯数据二次门：renderer/report 不能自行授权 callback；只有刚通过 identity
/// admission 的同一 token、真实 damage、同一 commit 观察到的精确 callback 数量与成功
/// presentation 同时成立，才允许 coordinator 发出 `done`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeShmFrameCompletionInput {
    /// GLES import/draw 与 Winit/EGL submit 均已成功。
    pub presentation_succeeded: bool,
    /// 同一 token 已通过本轮 live ledger/Core identity admission。
    pub identity_authorized: bool,
    /// 同一 commit 观察到真实 surface/buffer damage。
    pub damage_observed: bool,
    /// 同一 commit 观察到 frame callback 请求。
    pub callback_observed: bool,
    /// token 在本轮之前已 terminal，表示 duplicate/late。
    pub duplicate_or_late: bool,
    /// commit observation 声明的 callback 数量。
    pub observation_callback_count: usize,
    /// 从同一 `WlBuffer` resource 一起捕获的 callback 数量。
    pub captured_callback_count: usize,
}

/// 返回唯一允许发送 `done` 的 callback 数量；任一条件不完整即 fail closed 为零。
pub(crate) const fn authorized_frame_callback_count(
    input: RuntimeShmFrameCompletionInput,
) -> usize {
    if input.presentation_succeeded
        && input.identity_authorized
        && input.damage_observed
        && input.callback_observed
        && !input.duplicate_or_late
        && input.observation_callback_count > 0
        && input.observation_callback_count == input.captured_callback_count
    {
        input.captured_callback_count
    } else {
        0
    }
}

/// 在真实 resource 仍由 display FIFO 持有时决定本轮原子处理动作。
pub(crate) fn decide_runtime_shm_render_admission(
    input: RuntimeShmRenderAdmissionInput,
) -> RuntimeShmRenderAdmissionDecision {
    if input.token_already_terminal {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::DuplicateOrLateToken,
        };
    }

    let Some(resource_token) = input.resource_token else {
        if input.resource_tombstoned {
            return RuntimeShmRenderAdmissionDecision::RejectObservation {
                reason: RuntimeShmRenderAdmissionRejectReason::DestroyedResource,
            };
        }
        return if input.lifecycle_resolution_pending {
            RuntimeShmRenderAdmissionDecision::DeferIdentity
        } else {
            RuntimeShmRenderAdmissionDecision::RejectObservation {
                reason: RuntimeShmRenderAdmissionRejectReason::MissingResourceForCommit,
            }
        };
    };

    if resource_token != input.observation_token {
        if resource_token.commit_sequence < input.observation_token.commit_sequence {
            return RuntimeShmRenderAdmissionDecision::RejectResource {
                reason: RuntimeShmRenderAdmissionRejectReason::StaleResourceToken,
            };
        }
        if resource_token.commit_sequence > input.observation_token.commit_sequence {
            return RuntimeShmRenderAdmissionDecision::RejectObservation {
                reason: RuntimeShmRenderAdmissionRejectReason::MissingResourceForCommit,
            };
        }
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::TokenIdentityMismatch,
        };
    }

    if input.source_session.is_none() {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::MissingSourceSession,
        };
    }
    let Some(core_client) = input.resolved_core_client else {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::UnknownSourceSession,
        };
    };

    let (Some(adapter_toplevel), Some(core_surface), Some(core_window)) = (
        input.adapter_toplevel,
        input.ledger_surface,
        input.ledger_window,
    ) else {
        if input.lifecycle_resolution_pending {
            return RuntimeShmRenderAdmissionDecision::DeferIdentity;
        }
        let reason = if input.adapter_toplevel.is_none() {
            RuntimeShmRenderAdmissionRejectReason::MissingAdapterToplevel
        } else if input.ledger_surface.is_none() {
            RuntimeShmRenderAdmissionRejectReason::MissingLedgerSurface
        } else {
            RuntimeShmRenderAdmissionRejectReason::MissingLedgerWindow
        };
        return RuntimeShmRenderAdmissionDecision::RejectBoth { reason };
    };

    if !input.core_surface_alive {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::DeadCoreSurface,
        };
    }
    if input.core_surface_client != Some(core_client) {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::CoreClientMismatch,
        };
    }
    if input.core_window_for_surface != Some(core_window) {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::CoreWindowMismatch,
        };
    }
    if !input.core_window_alive {
        return RuntimeShmRenderAdmissionDecision::RejectBoth {
            reason: RuntimeShmRenderAdmissionRejectReason::DeadCoreWindow,
        };
    }

    RuntimeShmRenderAdmissionDecision::Ready {
        adapter_toplevel,
        core_client,
        core_surface,
        core_window,
        damage_observed: input.damage_observed,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeShmFrameCompletionInput, RuntimeShmRenderAdmissionDecision,
        RuntimeShmRenderAdmissionInput, RuntimeShmRenderAdmissionRejectReason,
        RuntimeShmRenderCommitToken, RuntimeShmRenderIdentityOwner,
        authorized_frame_callback_count, decide_runtime_shm_render_admission,
    };
    use crate::smithay_backend::{
        client_session::NestedClientSessionId,
        surface_xdg_admission::{AdapterSurfaceId, AdapterToplevelId, ProtocolObjectId},
    };

    fn surface(value: u64) -> AdapterSurfaceId {
        AdapterSurfaceId::new(ProtocolObjectId::new(value).expect("测试 surface ID 必须非零"))
    }

    fn toplevel(value: u64) -> AdapterToplevelId {
        AdapterToplevelId::new(ProtocolObjectId::new(value).expect("测试 toplevel ID 必须非零"))
    }

    fn token(surface_value: u64, sequence: u64) -> RuntimeShmRenderCommitToken {
        RuntimeShmRenderCommitToken {
            adapter_surface: surface(surface_value),
            commit_sequence: sequence,
        }
    }

    fn valid_input() -> RuntimeShmRenderAdmissionInput {
        RuntimeShmRenderAdmissionInput {
            observation_token: token(11, 7),
            resource_token: Some(token(11, 7)),
            resource_tombstoned: false,
            token_already_terminal: false,
            source_session: NestedClientSessionId::new(3),
            resolved_core_client: Some(41),
            lifecycle_resolution_pending: false,
            adapter_toplevel: Some(toplevel(21)),
            ledger_surface: Some(51),
            ledger_window: Some(61),
            core_surface_alive: true,
            core_surface_client: Some(41),
            core_window_for_surface: Some(61),
            core_window_alive: true,
            damage_observed: true,
        }
    }

    /// Red：未知或已断开的 adapter session 不能猜测 Core client；两侧 token 必须
    /// 同步拒绝并 tombstone，避免永久堵塞 FIFO。
    #[test]
    fn unknown_session_rejects_matching_resource_and_observation() {
        let mut input = valid_input();
        input.resolved_core_client = None;

        assert_eq!(
            decide_runtime_shm_render_admission(input),
            RuntimeShmRenderAdmissionDecision::RejectBoth {
                reason: RuntimeShmRenderAdmissionRejectReason::UnknownSourceSession,
            }
        );
    }

    /// Red：admission lifecycle 尚可能到达时，未知 surface/toplevel 必须原样保留两侧，
    /// 不能为通过测试而提前消费真实 WlBuffer。
    #[test]
    fn unresolved_identity_defers_without_consuming_either_fifo() {
        let mut input = valid_input();
        input.adapter_toplevel = None;
        input.ledger_surface = None;
        input.ledger_window = None;
        input.lifecycle_resolution_pending = true;

        assert_eq!(
            decide_runtime_shm_render_admission(input),
            RuntimeShmRenderAdmissionDecision::DeferIdentity
        );
    }

    /// Red：lifecycle 已排空后仍缺 ledger surface 属于确定拒绝，不能永久 defer。
    #[test]
    fn missing_ledger_surface_after_lifecycle_drain_rejects_both() {
        let mut input = valid_input();
        input.ledger_surface = None;

        assert_eq!(
            decide_runtime_shm_render_admission(input),
            RuntimeShmRenderAdmissionDecision::RejectBoth {
                reason: RuntimeShmRenderAdmissionRejectReason::MissingLedgerSurface,
            }
        );
    }

    /// Red：dead surface/window 或 surface→window/client 冲突必须 fail closed。
    #[test]
    fn dead_or_mismatched_core_identity_rejects_before_resource_transfer() {
        let cases = [
            (
                {
                    let mut input = valid_input();
                    input.core_surface_alive = false;
                    input
                },
                RuntimeShmRenderAdmissionRejectReason::DeadCoreSurface,
            ),
            (
                {
                    let mut input = valid_input();
                    input.core_window_alive = false;
                    input
                },
                RuntimeShmRenderAdmissionRejectReason::DeadCoreWindow,
            ),
            (
                {
                    let mut input = valid_input();
                    input.core_window_for_surface = Some(99);
                    input
                },
                RuntimeShmRenderAdmissionRejectReason::CoreWindowMismatch,
            ),
            (
                {
                    let mut input = valid_input();
                    input.core_surface_client = Some(42);
                    input
                },
                RuntimeShmRenderAdmissionRejectReason::CoreClientMismatch,
            ),
        ];

        for (input, reason) in cases {
            assert_eq!(
                decide_runtime_shm_render_admission(input),
                RuntimeShmRenderAdmissionDecision::RejectBoth { reason }
            );
        }
    }

    /// Red：destroy callback 已回收 resource 后，匹配 observation 只能被拒绝；不得再次
    /// take resource 或发送 frame done。
    #[test]
    fn destroyed_resource_rejects_only_the_remaining_observation() {
        let mut input = valid_input();
        input.resource_token = None;
        input.resource_tombstoned = true;

        assert_eq!(
            decide_runtime_shm_render_admission(input),
            RuntimeShmRenderAdmissionDecision::RejectObservation {
                reason: RuntimeShmRenderAdmissionRejectReason::DestroyedResource,
            }
        );
    }

    /// Red：A/B 交错时只允许处理 sequence 较旧的一侧。A resource 在 B observation 前
    /// 必须先 tombstone A resource 并保留 B；反向则拒绝旧 observation 并保留 B resource。
    #[test]
    fn token_mismatch_reconciles_only_the_older_fifo_head() {
        let mut stale_resource = valid_input();
        stale_resource.observation_token = token(22, 8);
        assert_eq!(
            decide_runtime_shm_render_admission(stale_resource),
            RuntimeShmRenderAdmissionDecision::RejectResource {
                reason: RuntimeShmRenderAdmissionRejectReason::StaleResourceToken,
            }
        );

        let mut stale_observation = valid_input();
        stale_observation.resource_token = Some(token(22, 8));
        assert_eq!(
            decide_runtime_shm_render_admission(stale_observation),
            RuntimeShmRenderAdmissionDecision::RejectObservation {
                reason: RuntimeShmRenderAdmissionRejectReason::MissingResourceForCommit,
            }
        );
    }

    /// Red：完整身份与精确 token 只授权资源转移；damage 是否存在由之后的 completion
    /// gate 决定 frame done，不能混入 admission。
    #[test]
    fn exact_live_identity_authorizes_single_resource_transfer() {
        assert_eq!(
            decide_runtime_shm_render_admission(valid_input()),
            RuntimeShmRenderAdmissionDecision::Ready {
                adapter_toplevel: toplevel(21),
                core_client: 41,
                core_surface: 51,
                core_window: 61,
                damage_observed: true,
            }
        );
    }

    /// Red：已经成功或拒绝过的 token 再次到达时必须同时拒绝并清理，绝不再次
    /// import、submit 或发送 frame done。
    #[test]
    fn duplicate_or_late_terminal_token_is_rejected() {
        let mut input = valid_input();
        input.token_already_terminal = true;

        assert_eq!(
            decide_runtime_shm_render_admission(input),
            RuntimeShmRenderAdmissionDecision::RejectBoth {
                reason: RuntimeShmRenderAdmissionRejectReason::DuplicateOrLateToken,
            }
        );
    }

    /// Red：A/B admission 映射必须保持独立；B destroy 只能 retire B，不能改变 A 的
    /// active toplevel。迟到 B token 必须可由 tombstone 识别。
    #[test]
    fn identity_owner_keeps_a_live_when_b_is_retired() {
        let mut owner = RuntimeShmRenderIdentityOwner::new();
        assert!(owner.record_admission(surface(11), toplevel(21)));
        assert!(owner.record_admission(surface(12), toplevel(22)));
        assert!(owner.record_unmap(surface(12), toplevel(22)));

        assert_eq!(owner.active_toplevel(surface(11)), Some(toplevel(21)));
        assert_eq!(owner.active_toplevel(surface(12)), None);
        assert!(owner.is_retired(surface(12), toplevel(22)));
        assert!(!owner.is_retired(surface(11), toplevel(21)));
    }

    /// Red：冲突 admission 与不匹配 unmap 都不能覆盖/删除现有 active mapping。
    #[test]
    fn identity_owner_rejects_conflicting_rebind_and_wrong_unmap() {
        let mut owner = RuntimeShmRenderIdentityOwner::new();
        assert!(owner.record_admission(surface(11), toplevel(21)));
        assert!(!owner.record_admission(surface(11), toplevel(22)));
        assert!(!owner.record_unmap(surface(11), toplevel(22)));
        assert_eq!(owner.active_toplevel(surface(11)), Some(toplevel(21)));
    }

    /// Red：只有本轮呈现成功、身份已授权、damage 与 callback 同时存在，且 token 不是
    /// duplicate/late 时，completion gate 才能放行精确 callback 数量。
    #[test]
    fn frame_completion_requires_every_success_condition() {
        let valid = RuntimeShmFrameCompletionInput {
            presentation_succeeded: true,
            identity_authorized: true,
            damage_observed: true,
            callback_observed: true,
            duplicate_or_late: false,
            observation_callback_count: 2,
            captured_callback_count: 2,
        };
        assert_eq!(authorized_frame_callback_count(valid), 2);

        for denied in [
            RuntimeShmFrameCompletionInput {
                presentation_succeeded: false,
                ..valid
            },
            RuntimeShmFrameCompletionInput {
                identity_authorized: false,
                ..valid
            },
            RuntimeShmFrameCompletionInput {
                damage_observed: false,
                ..valid
            },
            RuntimeShmFrameCompletionInput {
                callback_observed: false,
                ..valid
            },
            RuntimeShmFrameCompletionInput {
                duplicate_or_late: true,
                ..valid
            },
            RuntimeShmFrameCompletionInput {
                captured_callback_count: 0,
                ..valid
            },
        ] {
            assert_eq!(authorized_frame_callback_count(denied), 0);
        }
    }
}
