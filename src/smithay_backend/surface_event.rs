//! Smithay surface 生命周期事件适配探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不保存真实 `wl_surface`，不注册 `wl_compositor`，也不接 xdg-shell。
//!
//! 它只负责把未来 surface 创建和关闭信息转换为纯数据 `BackendEvent`。
//! 注意：`SurfaceCreated` 只代表 surface 出现，不代表窗口已经创建；
//! `SurfaceClosed` 只关闭指定 surface 及其绑定窗口，不等于 client 已断开。

use crate::core::{
    backend_event::BackendEvent,
    client::ClientId,
    surface::{SurfaceId, SurfaceRole},
};

/// Smithay surface event 适配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该模块只生成纯数据 `BackendEvent`，
/// 不处理真实 `wl_surface`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithaySurfaceEventMode {
    /// 纯探针模式。
    ///
    /// 不保存真实 `wl_surface`，不注册 `wl_compositor`，不接 xdg-shell。
    ProbeOnly,
}

/// surface 创建描述信息。
///
/// 该结构不保存真实 `wl_surface`，只保存未来可由 Smithay callback 提取出的
/// 最小纯数据 metadata。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithaySurfaceCreationDescriptor {
    /// 要创建的 surface ID。
    pub surface: SurfaceId,

    /// 可选的 surface 所属 client。
    ///
    /// `None` 表示当前无法确定 client 归属。
    pub client: Option<ClientId>,

    /// surface 协议角色。
    ///
    /// 注意：不同 role 的 surface 不一定都会变成窗口。
    pub role: SurfaceRole,
}

impl SmithaySurfaceCreationDescriptor {
    /// 创建一个 surface 描述。
    pub fn new(surface: SurfaceId, client: Option<ClientId>, role: SurfaceRole) -> Self {
        Self {
            surface,
            client,
            role,
        }
    }

    /// 创建一个属于指定 client 的 surface 描述。
    pub fn for_client(surface: SurfaceId, client: ClientId, role: SurfaceRole) -> Self {
        Self {
            surface,
            client: Some(client),
            role,
        }
    }

    /// 创建一个暂时没有 client 归属的 surface 描述。
    pub fn without_client(surface: SurfaceId, role: SurfaceRole) -> Self {
        Self {
            surface,
            client: None,
            role,
        }
    }
}

/// Smithay surface 事件适配探针。
///
/// 该类型不持有状态，也不保存真实 surface。
/// 它只把 surface 创建或关闭信息转换成对应的纯数据 `BackendEvent`，
/// 不直接修改核心 `State`。
pub struct SmithaySurfaceEventProbe;

impl SmithaySurfaceEventProbe {
    /// 返回当前适配器模式。
    pub fn mode() -> SmithaySurfaceEventMode {
        SmithaySurfaceEventMode::ProbeOnly
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only() -> bool {
        true
    }

    /// 把 surface 创建描述转换成 `BackendEvent`。
    ///
    /// 未来真实 Smithay surface callback 应先收集纯数据描述，
    /// 再通过该路径生成 `BackendEvent`，而不是直接修改核心 `State`。
    /// 该事件只注册 surface 事实，不等于窗口已经创建。
    pub fn surface_created_event(descriptor: SmithaySurfaceCreationDescriptor) -> BackendEvent {
        BackendEvent::SurfaceCreated {
            surface: descriptor.surface,
            client: descriptor.client,
            role: descriptor.role,
        }
    }

    /// 生成 surface 关闭事件。
    ///
    /// 当前只生成纯数据 `BackendEvent::SurfaceClosed`，不保存真实 `wl_surface`，
    /// 也不直接修改核心 `State`。本阶段不接 xdg-shell，也不注册
    /// `wl_compositor`。真正关闭 surface 及其绑定窗口的逻辑，会在该事件经过
    /// `BackendDriverRunner` 和 `CoreRuntimeBridge` 后由核心处理。
    ///
    /// `SurfaceClosed` 只关闭指定 surface 及其绑定窗口；`ClientDisconnected`
    /// 则关闭该 client 拥有的所有 surface 和窗口。
    pub fn surface_closed_event(surface: SurfaceId) -> BackendEvent {
        BackendEvent::SurfaceClosed { surface }
    }

    /// 返回当前阶段说明。
    pub fn mode_description() -> &'static str {
        "smithay-surface-event-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithaySurfaceCreationDescriptor, SmithaySurfaceEventMode, SmithaySurfaceEventProbe,
    };
    use crate::core::{backend_event::BackendEvent, surface::SurfaceRole};

    /// 验证 surface 描述构造方法会正确保留 ID、client 归属和协议角色。
    #[test]
    fn surface_creation_descriptor_builders_work() {
        let generic = SmithaySurfaceCreationDescriptor::new(1, Some(7), SurfaceRole::XdgToplevel);
        let owned = SmithaySurfaceCreationDescriptor::for_client(2, 8, SurfaceRole::LayerShell);
        let unowned = SmithaySurfaceCreationDescriptor::without_client(3, SurfaceRole::Unknown);

        assert_eq!(generic.surface, 1);
        assert_eq!(generic.client, Some(7));
        assert_eq!(generic.role, SurfaceRole::XdgToplevel);
        assert_eq!(owned.client, Some(8));
        assert_eq!(owned.role, SurfaceRole::LayerShell);
        assert_eq!(unowned.client, None);
        assert_eq!(unowned.role, SurfaceRole::Unknown);
    }

    /// 验证 surface 事件探针会生成完整的纯数据 SurfaceCreated 事件。
    #[test]
    fn surface_event_probe_creates_surface_created_event() {
        let event = SmithaySurfaceEventProbe::surface_created_event(
            SmithaySurfaceCreationDescriptor::for_client(42, 7, SurfaceRole::XdgToplevel),
        );

        assert_eq!(
            event,
            BackendEvent::SurfaceCreated {
                surface: 42,
                client: Some(7),
                role: SurfaceRole::XdgToplevel,
            }
        );
    }

    /// 验证 Smithay 探针层只生成 SurfaceClosed 纯数据事件，不直接关闭核心记录。
    #[test]
    fn surface_event_probe_creates_surface_closed_event() {
        let event = SmithaySurfaceEventProbe::surface_closed_event(42);

        assert_eq!(event, BackendEvent::SurfaceClosed { surface: 42 });
    }

    /// 验证 surface 事件适配器固定保持纯探针模式。
    #[test]
    fn surface_event_probe_reports_probe_mode() {
        assert!(SmithaySurfaceEventProbe::is_probe_only());
        assert_eq!(
            SmithaySurfaceEventProbe::mode(),
            SmithaySurfaceEventMode::ProbeOnly
        );
        assert_eq!(
            SmithaySurfaceEventProbe::mode_description(),
            "smithay-surface-event-probe-only"
        );
    }
}
