//! 逻辑窗口注册表与轻量元数据。
//!
//! `WindowRegistry` 独立于 `CompositorState`，因为窗口 ID 与 metadata 的生命周期
//! 不属于 workspace、focus 或 output 状态。Registry 只负责创建、查询和标记窗口，
//! 不直接修改 Workspace；跨模块状态同步仍由全局 `State` 协调。
//!
//! 当前 metadata 不写入 session JSON。session 继续只保存窗口 ID 和布局结构，
//! 恢复后由 workspace 中的窗口引用补齐 mock 记录，避免改变既有持久化格式。

use crate::core::workspace::WindowId;

/// 窗口来源类型。
///
/// 当前系统还没有真实 Wayland client，因此大部分窗口都是 Mock。
/// 后续接入 Smithay surface 后，可以使用 WaylandPlaceholder 作为过渡类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowKind {
    /// 当前 MVP 中由 State::spawn_window 创建的测试窗口。
    Mock,

    /// 为未来真实 Wayland surface 预留的占位类型。
    WaylandPlaceholder,
}

/// 一个逻辑窗口的元数据记录。
///
/// WindowRecord 当前不保存真实 surface，只保存 compositor 需要追踪的最小窗口信息。
/// 这样后续把 WindowId 绑定到 Smithay surface 时，不需要重写 workspace、layout 或 focus。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRecord {
    /// 稳定逻辑窗口 ID。
    pub id: WindowId,

    /// 窗口标题。
    ///
    /// 当前 mock 窗口使用自动标题；未来真实 client 可从 xdg_toplevel title 更新。
    pub title: String,

    /// 应用 ID。
    ///
    /// 当前 mock 窗口没有真实 app_id；未来真实 client 可从 xdg_toplevel app_id 更新。
    pub app_id: Option<String>,

    /// 窗口来源类型。
    pub kind: WindowKind,

    /// 窗口是否仍然存活。
    ///
    /// 关闭窗口时先标记为 false，workspace 则负责移除可见引用。
    pub alive: bool,
}

/// 全局窗口注册表。
///
/// Registry 是 WindowId 的唯一分配者，同时保存每个窗口的轻量元数据。
/// 它不直接修改 Workspace、FocusState 或 RenderFrame；这些状态仍由
/// State 和 CompositorState 维护。
#[derive(Debug, Clone)]
pub struct WindowRegistry {
    /// 下一次创建窗口时应分配的稳定 ID。
    next_id: WindowId,

    /// 所有已注册窗口的轻量元数据。
    ///
    /// 已关闭窗口仍保留记录，并通过 `alive = false` 表达生命周期状态。
    windows: Vec<WindowRecord>,
}

impl WindowRegistry {
    /// 创建空注册表，并从窗口 ID 1 开始分配。
    pub fn new() -> Self {
        Self {
            next_id: 1,
            windows: Vec::new(),
        }
    }

    /// 清空当前 registry，并恢复为初始状态。
    ///
    /// 该方法用于 session load 成功后的 registry 重建。
    /// 因为 `State::new()` 会先创建默认 mock 窗口，如果随后加载 session 成功，
    /// 这些默认窗口 metadata 已经不再对应当前 workspace，必须清空后根据 session 内容重建。
    pub fn reset(&mut self) {
        // ID 分配器与 metadata 必须同时重置，避免旧窗口记录或计数器泄漏到恢复状态。
        self.next_id = 1;
        self.windows.clear();
    }

    /// 创建一个带默认 metadata 的 mock 窗口。
    ///
    /// 该方法是当前 `State::spawn_window()` 使用的唯一创建入口。
    pub fn create_mock(&mut self) -> WindowId {
        // 当前 next_id 是新窗口的稳定 ID，记录插入前先推进计数器。
        let id = self.next_id;
        self.next_id += 1;

        // mock metadata 只存在于运行时 registry，不改变 session JSON。
        self.windows.push(WindowRecord {
            id,
            title: format!("Mock Window {}", id),
            app_id: Some("sky-mirror.mock".to_string()),
            kind: WindowKind::Mock,
            alive: true,
        });

        println!("[WindowRegistry] Created mock window {}", id);
        id
    }

    /// 使用调用方提供的 metadata 创建逻辑窗口。
    ///
    /// 该入口为未来 Wayland surface 接入保留，但仍只建立逻辑记录，
    /// 不会把窗口分配到 workspace。
    pub fn create_with_metadata(
        &mut self,
        title: impl Into<String>,
        app_id: Option<String>,
        kind: WindowKind,
    ) -> WindowId {
        // WindowRegistry 始终是 WindowId 的唯一分配者。
        let id = self.next_id;
        self.next_id += 1;

        self.windows.push(WindowRecord {
            id,
            title: title.into(),
            app_id,
            kind,
            alive: true,
        });

        println!("[WindowRegistry] Created window {}", id);
        id
    }

    /// 确保恢复出来的窗口 ID 具有一条 mock metadata 记录。
    ///
    /// 已存在的记录不会重复插入，也不会把 dead 窗口强制复活。
    /// 无记录时创建恢复占位 metadata，并把 next_id 推进到该 ID 之后。
    pub fn ensure_mock(&mut self, id: WindowId) {
        // 无论记录是否已存在，后续新窗口 ID 都不能与恢复 ID 冲突。
        self.next_id = self.next_id.max(id.saturating_add(1));

        // 已存在的 metadata 保持原样，包括 alive=false 的生命周期状态。
        if self.windows.iter().any(|record| record.id == id) {
            return;
        }

        // session 不保存 metadata，因此为恢复引用创建可识别的 mock 占位记录。
        self.windows.push(WindowRecord {
            id,
            title: format!("Restored Window {}", id),
            app_id: Some("sky-mirror.restored".to_string()),
            kind: WindowKind::Mock,
            alive: true,
        });
    }

    /// 只读获取指定窗口的 metadata。
    pub fn get(&self, id: WindowId) -> Option<&WindowRecord> {
        self.windows.iter().find(|record| record.id == id)
    }

    /// 可变获取指定窗口的 metadata。
    ///
    /// Registry 只提供记录访问，不会根据 metadata 反向修改 workspace。
    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut WindowRecord> {
        self.windows.iter_mut().find(|record| record.id == id)
    }

    /// 返回所有窗口 metadata 的只读切片。
    ///
    /// 该方法用于 Inspector 生成调试快照，不允许调用方修改 registry。
    pub fn records(&self) -> &[WindowRecord] {
        &self.windows
    }

    /// 将指定窗口标记为不再存活。
    ///
    /// metadata 记录继续保留，便于未来诊断或延迟资源释放；可见引用由 Workspace 删除。
    pub fn mark_dead(&mut self, id: WindowId) -> bool {
        let Some(record) = self.get_mut(id) else {
            return false;
        };

        // 生命周期状态只在 registry 中修改，不直接操作 slot 或 focus。
        record.alive = false;
        println!("[WindowRegistry] Marked window {} dead", id);
        true
    }

    /// 判断指定窗口是否存在且仍然存活。
    pub fn is_alive(&self, id: WindowId) -> bool {
        self.get(id).is_some_and(|record| record.alive)
    }

    /// 返回下一次将分配的 WindowId，用于 session 保存。
    pub fn next_id(&self) -> WindowId {
        self.next_id
    }

    /// 恢复下一次分配的 WindowId。
    ///
    /// 至少规范化为 1，避免无效 session 将注册表退回保留值 0。
    pub fn set_next_id(&mut self, next_id: WindowId) {
        self.next_id = next_id.max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowKind, WindowRegistry};

    /// 验证 create_mock 会分配 ID 并注册完整的存活 metadata。
    #[test]
    fn create_mock_registers_alive_window_record() {
        let mut registry = WindowRegistry::new();

        let id = registry.create_mock();

        // 首个逻辑窗口必须使用约定的起始 ID 1。
        assert_eq!(id, 1);

        // 创建后 next_id 必须推进到 2，避免下一次分配重复。
        assert_eq!(registry.next_id(), 2);

        let record = registry.get(id).expect("新建窗口必须存在 metadata");

        // mock 窗口标题必须包含其稳定 ID。
        assert_eq!(record.title, "Mock Window 1");

        // mock app_id 必须使用当前 MVP 的固定标识。
        assert_eq!(record.app_id.as_deref(), Some("sky-mirror.mock"));

        // 默认创建来源必须标记为 Mock。
        assert_eq!(record.kind, WindowKind::Mock);

        // 新建窗口必须处于存活状态。
        assert!(record.alive);
        assert!(registry.is_alive(id));
    }

    /// 验证 mark_dead 只更新已有窗口的 alive 状态。
    #[test]
    fn mark_dead_updates_alive_flag() {
        let mut registry = WindowRegistry::new();
        let id = registry.create_mock();

        // 已注册窗口必须能够成功标记为 dead。
        assert!(registry.mark_dead(id));

        // dead 窗口不得继续被 is_alive 视为存活。
        assert!(!registry.is_alive(id));

        // 不存在的窗口不能产生虚假成功结果。
        assert!(!registry.mark_dead(999));
    }

    /// 验证 ensure_mock 会补齐恢复 metadata，并推进 next_id。
    #[test]
    fn ensure_mock_creates_restored_record_and_advances_next_id() {
        let mut registry = WindowRegistry::new();

        registry.ensure_mock(42);

        let record = registry.get(42).expect("恢复窗口必须补齐 metadata");

        // 恢复记录使用可区分于新建窗口的标题。
        assert_eq!(record.title, "Restored Window 42");

        // 补齐的恢复窗口默认视为存活。
        assert!(record.alive);

        // 后续新窗口 ID 必须位于恢复窗口 42 之后。
        assert!(registry.next_id() >= 43);
    }
}
