//! 未来 Wayland/Smithay surface 与逻辑窗口的纯数据绑定占位层。
//!
//! 本模块只管理数字 ID、协议角色、ClientId 归属和 WindowId 关联，不保存真实
//! Wayland surface、client 或任何 Smithay 对象。后续接入真实协议对象时，
//! 可以在 backend/protocol 边界把真实对象映射到这里的稳定 SurfaceId。

use crate::core::{client::ClientId, workspace::WindowId};

/// 未来真实 Wayland surface 的占位 ID。
///
/// SurfaceId 与 WindowId 不同：
/// WindowId 是 compositor 内部窗口管理 ID；
/// SurfaceId 是未来 protocol/backend 层识别真实 surface 的 ID。
/// 当前阶段 SurfaceId 只是纯数据数字，不绑定任何 Smithay 对象。
pub type SurfaceId = u64;

/// surface 在 Wayland 协议中的角色。
///
/// 当前只作为未来 Smithay 接入前的纯数据占位，不持有真实 protocol object。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRole {
    /// 普通 xdg_toplevel 主窗口。
    XdgToplevel,

    /// 弹出窗口，例如 xdg_popup。
    XdgPopup,

    /// layer-shell surface，例如 panel、dock、overlay。
    LayerShell,

    /// 角色尚未确定或暂时未知。
    Unknown,
}

/// 单个 surface 的纯数据绑定记录。
///
/// SurfaceRecord 不保存真实 Smithay surface，只记录未来需要建立的关系。
/// client 表示这个 surface 属于哪个 Wayland client；window 表示这个 surface
/// 是否已经映射成 compositor 内部逻辑窗口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRecord {
    /// 稳定 surface ID。
    pub id: SurfaceId,

    /// 该 surface 所属的 client。
    ///
    /// None 表示当前 surface 尚未绑定到 client，或者来自早期测试或占位流程。
    /// ClientId、SurfaceId 和 WindowId 属于不同层级，不能相互替代。
    pub client: Option<ClientId>,

    /// 该 surface 当前绑定到的逻辑窗口。
    ///
    /// 普通 toplevel 通常会绑定一个 WindowId；popup 或未映射 surface 可能暂时为 None。
    pub window: Option<WindowId>,

    /// surface 的协议角色。
    pub role: SurfaceRole,

    /// surface 是否仍然存活。
    pub alive: bool,
}

/// surface 占位注册表。
///
/// SurfaceRegistry 是未来 Smithay surface 对象进入核心状态前的纯数据边界。
/// 它只分配 SurfaceId、记录 role、ClientId 归属和 WindowId 绑定关系，
/// 不保存真实 Wayland 或 Smithay 对象，也不接入真实 client。
#[derive(Debug, Clone)]
pub struct SurfaceRegistry {
    /// 下一次分配的 SurfaceId。
    next_id: SurfaceId,

    /// 当前已知 surface 记录。
    surfaces: Vec<SurfaceRecord>,
}

impl SurfaceRegistry {
    /// 创建空 surface 注册表。
    pub fn new() -> Self {
        Self {
            next_id: 1,
            surfaces: Vec::new(),
        }
    }

    /// 注册一个尚未绑定逻辑窗口的 surface。
    ///
    /// 该入口适合未来先收到 wl_surface，再收到 xdg role 或 map 事件的流程。
    pub fn register_surface(&mut self, role: SurfaceRole) -> SurfaceId {
        self.register_surface_for_client(None, role)
    }

    /// 自动分配 ID 并注册一个可选绑定 client 的 surface。
    ///
    /// `client` 只记录纯数据归属，不代表真实 client 已经进入 Wayland display。
    pub fn register_surface_for_client(
        &mut self,
        client: Option<ClientId>,
        role: SurfaceRole,
    ) -> SurfaceId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);

        self.surfaces.push(SurfaceRecord {
            id,
            client,
            window: None,
            role,
            alive: true,
        });

        id
    }

    /// 使用外部指定 ID 注册一个未绑定窗口的 surface。
    ///
    /// 该方法为未来真实 backend 或 Smithay 提供稳定 ID 映射。
    /// 如果指定 ID 已存在，则保留原记录并返回 false。
    pub fn register_surface_with_id(&mut self, surface: SurfaceId, role: SurfaceRole) -> bool {
        self.register_surface_with_id_for_client(surface, None, role)
    }

    /// 使用外部指定 ID 注册一个可选绑定 client 的 surface。
    ///
    /// 该方法只保存 ID 归属关系，不校验 client 是否存在；上层 Validator
    /// 会报告缺失 client，从而允许后端事件顺序被忠实记录。
    pub fn register_surface_with_id_for_client(
        &mut self,
        surface: SurfaceId,
        client: Option<ClientId>,
        role: SurfaceRole,
    ) -> bool {
        if self.get(surface).is_some() {
            return false;
        }

        // 后续自动分配必须避开外部已经占用的 SurfaceId。
        self.next_id = self.next_id.max(surface.saturating_add(1));

        self.surfaces.push(SurfaceRecord {
            id: surface,
            client,
            window: None,
            role,
            alive: true,
        });

        true
    }

    /// 注册一个直接绑定到逻辑窗口的 surface。
    ///
    /// 当前 WaylandPlaceholder 窗口通过该入口建立 XdgToplevel 占位关系。
    pub fn register_for_window(&mut self, window: WindowId, role: SurfaceRole) -> SurfaceId {
        let id = self.next_id;
        self.next_id += 1;

        self.surfaces.push(SurfaceRecord {
            id,
            client: None,
            window: Some(window),
            role,
            alive: true,
        });

        id
    }

    /// 将已有 surface 绑定到逻辑窗口。
    ///
    /// 找不到 SurfaceId 时保持注册表不变并返回 false。
    pub fn bind_window(&mut self, surface: SurfaceId, window: WindowId) -> bool {
        let Some(record) = self.surfaces.iter_mut().find(|record| record.id == surface) else {
            return false;
        };

        record.window = Some(window);
        true
    }

    /// 将已有 surface 绑定到 client。
    ///
    /// 该方法只写入 `ClientId -> SurfaceId` 的纯数据归属，不创建窗口，
    /// 也不接入真实 Smithay client。找不到 SurfaceId 时返回 false。
    pub fn bind_client(&mut self, surface: SurfaceId, client: ClientId) -> bool {
        let Some(record) = self.surfaces.iter_mut().find(|record| record.id == surface) else {
            return false;
        };

        record.client = Some(client);
        true
    }

    /// 标记指定 surface 不再存活。
    ///
    /// 记录会继续保留用于诊断，找不到 SurfaceId 时返回 false。
    pub fn mark_dead(&mut self, surface: SurfaceId) -> bool {
        let Some(record) = self.surfaces.iter_mut().find(|record| record.id == surface) else {
            return false;
        };

        record.alive = false;
        true
    }

    /// 标记绑定到某个窗口的全部存活 surface 为 dead。
    ///
    /// 返回实际发生生命周期变化的记录数量，已经 dead 的记录不会重复计数。
    pub fn mark_dead_for_window(&mut self, window: WindowId) -> usize {
        let mut count = 0;

        for record in &mut self.surfaces {
            if record.window == Some(window) && record.alive {
                record.alive = false;
                count += 1;
            }
        }

        count
    }

    /// 标记指定 client 拥有的所有 surface 为 dead。
    ///
    /// 返回本次从 alive 变为 dead 的 surface ID 列表。已经 dead 的记录不会
    /// 重复计入，所有记录仍保留在 registry 中供 Inspector 和 Validator 读取。
    pub fn mark_dead_for_client(&mut self, client: ClientId) -> Vec<SurfaceId> {
        let mut dead_surfaces = Vec::new();

        for record in &mut self.surfaces {
            if record.client == Some(client) && record.alive {
                record.alive = false;
                dead_surfaces.push(record.id);
            }
        }

        dead_surfaces
    }

    /// 返回指定 client 当前拥有且绑定了 window 的所有 WindowId。
    ///
    /// 该方法只读取 surface 记录，不检查 WindowRegistry。返回值按 surface
    /// 记录顺序稳定去重，因为多个 surface 未来可能关联同一个逻辑窗口。
    pub fn windows_for_client(&self, client: ClientId) -> Vec<WindowId> {
        let mut windows = Vec::new();

        for record in &self.surfaces {
            if record.client == Some(client) {
                if let Some(window) = record.window {
                    if !windows.contains(&window) {
                        windows.push(window);
                    }
                }
            }
        }

        windows
    }

    /// 只读查找指定 surface。
    pub fn get(&self, surface: SurfaceId) -> Option<&SurfaceRecord> {
        self.surfaces.iter().find(|record| record.id == surface)
    }

    /// 返回指定 surface 当前绑定的逻辑窗口。
    ///
    /// surface 不存在或尚未绑定窗口时返回 None。
    pub fn window_for_surface(&self, surface: SurfaceId) -> Option<WindowId> {
        self.get(surface).and_then(|record| record.window)
    }

    /// 返回指定 surface 当前所属的 client。
    ///
    /// surface 不存在或尚未记录归属时返回 None。
    pub fn client_for_surface(&self, surface: SurfaceId) -> Option<ClientId> {
        self.get(surface).and_then(|record| record.client)
    }

    /// 返回某个 client 当前拥有的所有 surface ID。
    ///
    /// 返回结果保持 registry 记录顺序，并同时包含 alive 与 dead surface，
    /// 便于 Inspector 和未来诊断工具查看完整历史关系。
    pub fn surfaces_for_client(&self, client: ClientId) -> Vec<SurfaceId> {
        self.surfaces
            .iter()
            .filter(|record| record.client == Some(client))
            .map(|record| record.id)
            .collect()
    }

    /// 判断指定 surface 是否存在且仍然存活。
    pub fn is_alive(&self, surface: SurfaceId) -> bool {
        self.get(surface).is_some_and(|record| record.alive)
    }

    /// 返回所有 surface 记录的只读切片。
    ///
    /// Inspector 和 Validator 可以读取该切片，但不能修改绑定或生命周期。
    pub fn records(&self) -> &[SurfaceRecord] {
        &self.surfaces
    }

    /// 返回下一次将分配的 SurfaceId。
    pub fn next_id(&self) -> SurfaceId {
        self.next_id
    }
}

#[cfg(test)]
mod tests {
    use super::{SurfaceRegistry, SurfaceRole};

    /// 验证注册未绑定 surface 时会分配 ID 并保留角色。
    #[test]
    fn surface_registry_registers_unbound_surface() {
        let mut registry = SurfaceRegistry::new();

        let surface = registry.register_surface(SurfaceRole::Unknown);
        let record = registry.get(surface).expect("新 surface 必须存在记录");

        // 首个 SurfaceId 必须从 1 开始，并推进下一分配值。
        assert_eq!(surface, 1);
        assert_eq!(registry.next_id(), 2);

        // 未绑定 surface 不应提前关联任何 WindowId。
        assert_eq!(record.client, None);
        assert_eq!(record.window, None);
        assert_eq!(record.role, SurfaceRole::Unknown);
        assert!(record.alive);
    }

    /// 验证外部指定的 SurfaceId 会原样写入，并推进自动分配计数器。
    #[test]
    fn surface_registry_register_surface_with_id_uses_external_id() {
        let mut registry = SurfaceRegistry::new();

        // 指定 ID 注册必须成功，并保留外部系统提供的稳定 identity。
        assert!(registry.register_surface_with_id(42, SurfaceRole::XdgToplevel));
        assert!(registry.get(42).is_some());

        // 后续自动分配不得与显式注册的 ID 冲突。
        assert!(registry.next_id() >= 43);
    }

    /// 验证注册 surface 时可以直接保存 client 归属。
    #[test]
    fn surface_registry_registers_surface_for_client() {
        let mut registry = SurfaceRegistry::new();

        let surface = registry.register_surface_for_client(Some(7), SurfaceRole::XdgToplevel);
        let record = registry.get(surface).expect("归属 surface 必须存在记录");

        // ClientId 只表达归属，surface 此时仍未绑定任何 WindowId。
        assert_eq!(record.client, Some(7));
        assert_eq!(record.window, None);
        assert_eq!(registry.client_for_surface(surface), Some(7));
    }

    /// 验证已有 surface 可以在后续阶段绑定 client。
    #[test]
    fn surface_registry_bind_client_updates_owner() {
        let mut registry = SurfaceRegistry::new();
        let surface = registry.register_surface(SurfaceRole::Unknown);

        assert!(registry.bind_client(surface, 7));

        // 绑定后必须可以通过 surface 和 client 两个方向读取归属。
        assert_eq!(registry.client_for_surface(surface), Some(7));
        assert_eq!(registry.surfaces_for_client(7), vec![surface]);

        // 不存在的 SurfaceId 不得产生虚假成功结果。
        assert!(!registry.bind_client(999, 7));
    }

    /// 验证 client 的 surface 列表保留记录顺序并包含 dead surface。
    #[test]
    fn surface_registry_surfaces_for_client_includes_dead_records() {
        let mut registry = SurfaceRegistry::new();
        let first = registry.register_surface_for_client(Some(7), SurfaceRole::XdgToplevel);
        let unrelated = registry.register_surface_for_client(Some(8), SurfaceRole::XdgPopup);
        let second = registry.register_surface_for_client(Some(7), SurfaceRole::LayerShell);

        assert!(registry.mark_dead(first));

        // client 7 的结果必须保持原插入顺序，并包含已经 dead 的首个 surface。
        assert_eq!(registry.surfaces_for_client(7), vec![first, second]);
        assert_eq!(registry.surfaces_for_client(8), vec![unrelated]);
    }

    /// 验证重复的外部 SurfaceId 不会覆盖或追加第二条记录。
    #[test]
    fn surface_registry_register_surface_with_id_rejects_duplicate_id() {
        let mut registry = SurfaceRegistry::new();

        // 第一次注册建立唯一记录，第二次相同 ID 必须明确失败。
        assert!(registry.register_surface_with_id(42, SurfaceRole::XdgToplevel));
        assert!(!registry.register_surface_with_id(42, SurfaceRole::XdgPopup));

        // 拒绝重复注册后，指定 ID 仍然只能对应一条记录。
        assert_eq!(
            registry
                .records()
                .iter()
                .filter(|record| record.id == 42)
                .count(),
            1
        );
    }

    /// 验证可以直接注册绑定到逻辑窗口的 surface。
    #[test]
    fn surface_registry_registers_surface_for_window() {
        let mut registry = SurfaceRegistry::new();

        let surface = registry.register_for_window(42, SurfaceRole::XdgToplevel);
        let record = registry.get(surface).expect("绑定 surface 必须存在记录");

        // 绑定关系和协议角色必须完整保留。
        assert_eq!(record.client, None);
        assert_eq!(record.window, Some(42));
        assert_eq!(record.role, SurfaceRole::XdgToplevel);
        assert!(record.alive);
    }

    /// 验证已有 surface 可以在后续协议阶段绑定逻辑窗口。
    #[test]
    fn surface_registry_bind_window_updates_record() {
        let mut registry = SurfaceRegistry::new();
        let surface = registry.register_surface(SurfaceRole::XdgPopup);

        // 已存在的 SurfaceId 必须能够完成窗口绑定。
        assert!(registry.bind_window(surface, 7));
        assert_eq!(
            registry
                .get(surface)
                .expect("surface 记录必须继续存在")
                .window,
            Some(7)
        );

        // 不存在的 SurfaceId 不得产生虚假成功结果。
        assert!(!registry.bind_window(999, 7));
    }

    /// 验证关闭窗口时会标记全部绑定 surface 为 dead。
    #[test]
    fn surface_registry_mark_dead_for_window_marks_all_bound_surfaces() {
        let mut registry = SurfaceRegistry::new();
        let first = registry.register_for_window(9, SurfaceRole::XdgToplevel);
        let second = registry.register_for_window(9, SurfaceRole::XdgPopup);
        let unrelated = registry.register_for_window(10, SurfaceRole::LayerShell);

        let count = registry.mark_dead_for_window(9);

        // 只有目标窗口的两个存活 surface 应计入返回数量。
        assert_eq!(count, 2);
        assert!(!registry.get(first).expect("首个 surface 必须存在").alive);
        assert!(!registry.get(second).expect("第二个 surface 必须存在").alive);

        // 其他窗口绑定的 surface 必须保持存活。
        assert!(
            registry
                .get(unrelated)
                .expect("无关 surface 必须存在")
                .alive
        );
    }

    /// 验证关闭 client 时只标记该 client 拥有的存活 surface。
    #[test]
    fn surface_registry_mark_dead_for_client_marks_owned_surfaces() {
        let mut registry = SurfaceRegistry::new();
        let first = registry.register_surface_for_client(Some(7), SurfaceRole::XdgToplevel);
        let second = registry.register_surface_for_client(Some(7), SurfaceRole::XdgPopup);
        let already_dead = registry.register_surface_for_client(Some(7), SurfaceRole::LayerShell);
        let unrelated = registry.register_surface_for_client(Some(8), SurfaceRole::Unknown);
        assert!(registry.mark_dead(already_dead));

        let dead_surfaces = registry.mark_dead_for_client(7);

        // 返回值只包含本次发生 alive -> dead 变化的 owned surfaces。
        assert_eq!(dead_surfaces, vec![first, second]);
        assert!(!registry.is_alive(first));
        assert!(!registry.is_alive(second));
        assert!(!registry.is_alive(already_dead));

        // 其他 client 的 surface 不得被级联关闭。
        assert!(registry.is_alive(unrelated));

        // 重复调用不得再次返回已经 dead 的记录。
        assert!(registry.mark_dead_for_client(7).is_empty());
    }

    /// 验证 client 对应的窗口列表会保持顺序并去除重复 WindowId。
    #[test]
    fn surface_registry_windows_for_client_returns_unique_windows() {
        let mut registry = SurfaceRegistry::new();
        let first = registry.register_surface_for_client(Some(7), SurfaceRole::XdgToplevel);
        let duplicate = registry.register_surface_for_client(Some(7), SurfaceRole::XdgPopup);
        let second = registry.register_surface_for_client(Some(7), SurfaceRole::LayerShell);
        let unrelated = registry.register_surface_for_client(Some(8), SurfaceRole::XdgToplevel);

        assert!(registry.bind_window(first, 10));
        assert!(registry.bind_window(duplicate, 10));
        assert!(registry.bind_window(second, 11));
        assert!(registry.bind_window(unrelated, 12));

        // 同一 client 的重复 WindowId 只能出现一次，且保持首次发现顺序。
        assert_eq!(registry.windows_for_client(7), vec![10, 11]);
        assert_eq!(registry.windows_for_client(8), vec![12]);
    }
}
