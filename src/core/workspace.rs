//! workspace、固定 slot 与窗口 stack 的核心数据模型。
//!
//! 每个 Workspace 始终拥有 4 个 Slot，避免动态增删容器导致 focus、layout 和
//! session restore 的 ID 失配。Slot 可以为空、包含单个窗口，或包含一个具有
//! active index 的 Z 轴窗口堆栈。

/// compositor 内部使用的稳定逻辑窗口 ID。
///
/// 当前 ID 不代表真实 Wayland surface；它由 WindowRegistry 单调分配，
/// 用于 workspace、focus、session、layout 和 render 数据之间建立关联。
pub type WindowId = u64;

/// workspace 当前使用的布局模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// 优先显示 focused slot 的 active window；焦点无效时回退到首个 occupied slot。
    Fullscreen,
    /// 最多显示 slot 0 和 slot 1，左右分屏。
    Split,
    /// 最多显示四个 slot，组成 2x2 网格。
    Grid,
}

/// Workspace 中的一个固定位置。
pub struct Slot {
    /// slot 的稳定 ID，当前合法值固定为 0..=3。
    pub id: u8,
    /// slot 当前承载的窗口内容。
    pub content: SlotContent,
}

/// 同一 slot 内的 Z 轴窗口堆栈。
///
/// `windows` 保留所有逻辑窗口，`active` 指向当前可见并可聚焦的窗口。
/// LayoutEngine 不会展开整个 stack，只通过 `active_window()` 获取一个窗口。
pub struct Stack {
    /// 按加入顺序保存的窗口 ID。
    pub windows: Vec<WindowId>,
    /// 当前 active window 在 `windows` 中的索引。
    pub active: usize,
}

impl Stack {
    /// 由两个窗口创建 stack，并让新加入的第二个窗口成为 active。
    pub fn new(first: WindowId, second: WindowId) -> Self {
        Self {
            windows: vec![first, second],
            active: 1,
        }
    }

    /// 返回当前 active window。
    ///
    /// 空 stack 返回 None；非空时通过取模容忍 session 中超出范围的 active 索引，
    /// 避免读取恢复数据时发生 panic。
    pub fn active_window(&self) -> Option<WindowId> {
        // 空容器没有合法索引，必须在取模前提前返回。
        if self.windows.is_empty() {
            return None;
        }

        self.windows.get(self.active % self.windows.len()).copied()
    }

    /// 将窗口追加到 stack，并立即把新窗口设为 active。
    ///
    /// 这样新建窗口会成为该 slot 当前可见窗口，focus 刷新也能直接读取它。
    pub fn push(&mut self, window: WindowId) {
        self.windows.push(window);
        self.active = self.windows.len() - 1;
    }

    /// 从 stack 中删除指定窗口，并修正 active 索引。
    ///
    /// 找不到窗口时保持 stack 不变并返回 false。删除后必须重新校正 active：
    /// 删除 active 之前的窗口时索引左移；删除末尾 active 时回退到最后一个合法索引；
    /// 删除 active 本身且后方仍有窗口时，保留原索引即可自然指向补位窗口。
    pub fn remove_window(&mut self, window: WindowId) -> bool {
        // 先定位稳定 WindowId，避免按 active 索引误删其他窗口。
        let Some(index) = self
            .windows
            .iter()
            .position(|candidate| *candidate == window)
        else {
            return false;
        };

        // Vec 删除后，位于删除位置右侧的窗口会整体向左补位。
        self.windows.remove(index);

        if self.windows.is_empty() {
            // 空 stack 没有合法索引，统一规范化为 0。
            self.active = 0;
        } else if self.active >= self.windows.len() {
            // 删除最后一个 active 窗口或恢复了越界索引时，回退到最后一个合法位置。
            self.active = self.windows.len() - 1;
        } else if index < self.active {
            // 删除位置位于 active 之前时，原 active 窗口左移一位，索引也必须同步减一。
            self.active -= 1;
        }

        true
    }

    /// 循环切换到 stack 中的下一个窗口。
    ///
    /// 到达末尾后回到索引 0；空 stack 返回 None 且把 active 规范化为 0。
    pub fn next(&mut self) -> Option<WindowId> {
        // 防御性处理 session 恢复或未来删除逻辑可能产生的空 stack。
        if self.windows.is_empty() {
            self.active = 0;
            return None;
        }

        // 模运算提供 wrap around 行为。
        self.active = (self.active + 1) % self.windows.len();
        self.active_window()
    }

    /// 判断 stack 是否不包含任何窗口。
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }
}

/// Slot 可承载的三种互斥内容。
pub enum SlotContent {
    /// slot 当前没有窗口。
    Empty,
    /// slot 只包含一个窗口。
    Single(WindowId),
    /// slot 包含多个窗口，只有 Stack 的 active window 对外可见。
    Stack(Stack),
}

/// 一个具有固定四个 slot 的工作区。
///
/// workspace ID 是稳定标识，不依赖其在 `Vec<Workspace>` 中的下标。
/// 固定数组保证布局、焦点和 session restore 始终面对相同数量的 slot。
pub struct Workspace {
    /// 工作区稳定 ID。
    pub id: u32,
    /// 当前工作区使用的布局模式。
    pub layout: LayoutMode,
    /// 固定 slot 0、1、2、3。
    ///
    /// 不允许动态 slot 数量，避免导航和恢复时出现结构不一致。
    pub slots: [Slot; 4],
}

impl Workspace {
    /// 创建一个 Fullscreen 布局、四个 slot 全部为空的 workspace。
    pub fn new(id: u32) -> Self {
        Self {
            id,
            layout: LayoutMode::Fullscreen,
            slots: [
                Slot {
                    id: 0,
                    content: SlotContent::Empty,
                },
                Slot {
                    id: 1,
                    content: SlotContent::Empty,
                },
                Slot {
                    id: 2,
                    content: SlotContent::Empty,
                },
                Slot {
                    id: 3,
                    content: SlotContent::Empty,
                },
            ],
        }
    }

    /// 将窗口分配到 workspace。
    ///
    /// 优先使用第一个 Empty slot；四个 slot 都被占用后，固定将新窗口加入
    /// slot 0 的 stack。该策略保持 slot 数量不变，并为 Z 轴堆叠提供确定性入口。
    pub fn assign_window(&mut self, window: WindowId) {
        // 新窗口先寻找独立空位，避免在仍有可用 slot 时过早创建 stack。
        for slot in &mut self.slots {
            if matches!(slot.content, SlotContent::Empty) {
                println!(
                    "[Workspace] Assigned window {} to workspace {}, slot {}",
                    window, self.id, slot.id
                );
                slot.content = SlotContent::Single(window);
                // 已完成唯一一次分配，立即返回以避免继续进入 stack fallback。
                return;
            }
        }

        // 所有 slot 均已占用，固定使用 slot 0 承载额外窗口。
        match &mut self.slots[0].content {
            // 第一次溢出时，把原 Single 和新窗口转换为二元素 Stack。
            SlotContent::Single(existing) => {
                // 复制旧 ID 后再替换整个枚举值，避免丢失原窗口。
                let existing = *existing;
                self.slots[0].content = SlotContent::Stack(Stack::new(existing, window));
                println!(
                    "[Workspace] Added window {} to stack in workspace {}, slot 0",
                    window, self.id
                );
            }
            // slot 0 已经是 Stack 时直接追加，并由 push 将新窗口设为 active。
            SlotContent::Stack(stack) => {
                stack.push(window);
                println!(
                    "[Workspace] Added window {} to stack in workspace {}, slot 0",
                    window, self.id
                );
            }
            // 正常流程下四个 slot 已满，因此该分支是防御性 fallback。
            // 即使状态被外部恢复为不一致，也仍可安全保存新窗口而不 panic。
            SlotContent::Empty => {
                self.slots[0].content = SlotContent::Single(window);
                println!(
                    "[Workspace] Assigned window {} to workspace {}, slot 0",
                    window, self.id
                );
            }
        }
    }

    /// 从 workspace 的全部固定 slot 或 stack 中删除指定窗口。
    ///
    /// Empty slot 直接跳过；Single 命中后变为 Empty；Stack 命中后根据剩余窗口数
    /// 保持 Stack、降级为 Single 或清空为 Empty。方法会扫描全部四个 slot，确保
    /// session 或异常输入造成的重复 WindowId 不会在窗口销毁后继续被 layout 读取。
    pub fn remove_window(&mut self, window: WindowId) -> bool {
        let mut removed = false;

        for slot in &mut self.slots {
            let removed_from_slot = match &mut slot.content {
                // Empty 不包含任何窗口，不需要修改。
                SlotContent::Empty => false,
                // Single 只有在 WindowId 匹配时才清空当前 slot。
                SlotContent::Single(existing) => {
                    if *existing != window {
                        false
                    } else {
                        slot.content = SlotContent::Empty;
                        true
                    }
                }
                // Stack 先删除窗口，再根据剩余数量恢复 SlotContent 不变量。
                SlotContent::Stack(stack) => {
                    let mut removed_from_stack = false;

                    // 异常恢复数据可能在同一 Stack 中重复引用 WindowId，必须全部清理。
                    while stack.remove_window(window) {
                        removed_from_stack = true;
                    }

                    if !removed_from_stack {
                        false
                    } else {
                        match stack.windows.len() {
                            // 没有剩余窗口时，slot 不应继续保存空 Stack。
                            0 => slot.content = SlotContent::Empty,
                            // 只剩一个窗口时降级为 Single，避免用 Stack 表达单窗口状态。
                            1 => {
                                let remaining = stack.windows[0];
                                slot.content = SlotContent::Single(remaining);
                            }
                            // 两个及以上窗口仍满足 Stack 不变量，无需替换内容。
                            _ => {}
                        }

                        true
                    }
                }
            };

            if removed_from_slot {
                // destroy 必须清理全部重复引用，不能在首个命中后提前返回。
                removed = true;
            }
        }

        if removed {
            println!(
                "[Workspace] Removed window {} from workspace {}",
                window, self.id
            );
        }

        removed
    }

    /// 收集 workspace 中引用的全部逻辑窗口 ID。
    ///
    /// Single 贡献一个 ID，Stack 贡献其完整窗口列表，Empty 不产生内容。
    /// 该方法主要用于 session 恢复后为 WindowRegistry 补齐未序列化的 metadata，
    /// 不会改变 slot、stack active 索引或窗口顺序。
    pub fn window_ids(&self) -> Vec<WindowId> {
        let mut windows = Vec::new();

        for slot in &self.slots {
            match &slot.content {
                // Empty slot 没有任何窗口引用。
                SlotContent::Empty => {}
                // Single 直接贡献其唯一窗口 ID。
                SlotContent::Single(window) => windows.push(*window),
                // Stack 中所有窗口都需要 metadata，而不仅是当前 active window。
                SlotContent::Stack(stack) => windows.extend(stack.windows.iter().copied()),
            }
        }

        windows
    }

    /// 返回指定 slot 当前可见的窗口。
    ///
    /// 这是 layout、focus 和 scene 读取 slot 的统一入口：
    /// Empty 返回 None，Single 返回自身，Stack 只返回 active window。
    pub fn slot_window(&self, slot_id: u8) -> Option<WindowId> {
        // 使用稳定 slot ID 查找；不存在的 ID 安全返回 None。
        let slot = self.slots.iter().find(|slot| slot.id == slot_id)?;

        match &slot.content {
            // 空 slot 没有可见窗口。
            SlotContent::Empty => None,
            // 单窗口 slot 直接返回其 ID。
            SlotContent::Single(window) => Some(*window),
            // Stack 对外只暴露 active window，隐藏其余 Z 轴窗口。
            SlotContent::Stack(stack) => stack.active_window(),
        }
    }

    /// 在指定 slot 的 stack 中切换到下一个 active window。
    ///
    /// Single 返回原窗口，Empty 或不存在的 slot 返回 None；只有 Stack 会改变内部索引。
    pub fn next_in_stack(&mut self, slot_id: u8) -> Option<WindowId> {
        // 在可变借用 slot 前复制 workspace ID，避免打印日志时再次借用 self。
        let workspace_id = self.id;

        // slot ID 无效时通过 `?` 安全返回 None。
        let slot = self.slots.iter_mut().find(|slot| slot.id == slot_id)?;

        match &mut slot.content {
            // Empty 没有可切换窗口。
            SlotContent::Empty => None,
            // Single 不需要改变状态，但仍返回其可见窗口。
            SlotContent::Single(window) => Some(*window),
            // Stack 执行 wrap around，并返回新的 active window。
            SlotContent::Stack(stack) => {
                let active = stack.next();
                println!(
                    "[Workspace] Stack switched in workspace {}, slot {}, active={:?}",
                    workspace_id, slot_id, active
                );
                active
            }
        }
    }

    /// 从当前 slot 向后查找下一个包含 active window 的 slot。
    ///
    /// 搜索支持 wrap around，并且最多检查固定数组中的四个位置。
    /// 如果 workspace 没有任何可见窗口，则返回 None。
    pub fn next_occupied_slot(&self, current_slot: u8) -> Option<u8> {
        let len = self.slots.len();

        // 当前 ID 不存在时从索引 0 作为防御性起点，避免 panic。
        let current = self
            .slots
            .iter()
            .position(|slot| slot.id == current_slot)
            .unwrap_or(0);

        // 从下一个位置开始，最后一次 offset 会回到当前 slot。
        // 因此只有当前 slot 被占用时也能返回其 ID。
        for offset in 1..=len {
            let slot = &self.slots[(current + offset) % len];
            if self.slot_window(slot.id).is_some() {
                return Some(slot.id);
            }
        }

        None
    }

    /// 从当前 slot 向前查找上一个包含 active window 的 slot。
    ///
    /// 与 `next_occupied_slot()` 相同，该搜索支持 wrap around 且不会越界。
    pub fn prev_occupied_slot(&self, current_slot: u8) -> Option<u8> {
        let len = self.slots.len();

        // 当前 ID 无效时使用索引 0，保证恢复异常数据时仍能安全导航。
        let current = self
            .slots
            .iter()
            .position(|slot| slot.id == current_slot)
            .unwrap_or(0);

        // 使用模运算向前移动；offset 等于 len 时会回到当前 slot。
        for offset in 1..=len {
            let index = (current + len - (offset % len)) % len;
            let slot = &self.slots[index];
            if self.slot_window(slot.id).is_some() {
                return Some(slot.id);
            }
        }

        None
    }
}

// 以下测试直接位于 workspace 模块内部，用于保护固定 slot 与 stack 的核心不变量。
#[cfg(test)]
mod tests {
    use super::{SlotContent, Stack, Workspace};

    /// 验证新建 Workspace 始终拥有 ID 为 0..=3 的四个空 slot。
    #[test]
    fn new_workspace_has_four_fixed_empty_slots() {
        let workspace = Workspace::new(7);

        // 固定四个 slot 是 focus、layout 和 session restore 共同依赖的结构不变量。
        assert_eq!(workspace.slots.len(), 4);

        // slot ID 必须稳定对应数组顺序，不能由运行时动态分配。
        assert_eq!(
            workspace
                .slots
                .iter()
                .map(|slot| slot.id)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        // 新 workspace 不应预先包含任何窗口。
        assert!(
            workspace
                .slots
                .iter()
                .all(|slot| matches!(slot.content, SlotContent::Empty))
        );
    }

    /// 验证前四个窗口依次进入四个独立 slot，而不会提前形成 stack。
    #[test]
    fn first_four_windows_use_independent_slots() {
        let mut workspace = Workspace::new(0);

        for window in 1..=4 {
            workspace.assign_window(window);
        }

        // slot 0..3 必须按窗口创建顺序分别保存窗口 1..4。
        assert_eq!(workspace.slot_window(0), Some(1));
        assert_eq!(workspace.slot_window(1), Some(2));
        assert_eq!(workspace.slot_window(2), Some(3));
        assert_eq!(workspace.slot_window(3), Some(4));
    }

    /// 验证第五个窗口在 slot 全满后进入 slot 0 的 stack，并成为 active window。
    #[test]
    fn fifth_window_enters_slot_zero_stack() {
        let mut workspace = Workspace::new(0);

        for window in 1..=5 {
            workspace.assign_window(window);
        }

        let SlotContent::Stack(stack) = &workspace.slots[0].content else {
            panic!("第五个窗口加入后，slot 0 必须转换为 Stack");
        };

        // Single(1) 转换为 Stack 时必须保留旧窗口，并追加新窗口 5。
        assert_eq!(stack.windows, vec![1, 5]);

        // Stack::new 必须让第二个窗口成为 active。
        assert_eq!(stack.active, 1);

        // 统一读取接口必须返回 stack 的 active window，而不是第一个历史窗口。
        assert_eq!(workspace.slot_window(0), Some(5));
    }

    /// 验证向 stack 追加窗口后，active 索引会指向刚加入的新窗口。
    #[test]
    fn stack_push_activates_new_window() {
        let mut stack = Stack::new(1, 2);

        stack.push(3);

        // push 必须保留原顺序并把新窗口追加到末尾。
        assert_eq!(stack.windows, vec![1, 2, 3]);

        // active 必须指向新窗口所在的最后一个索引。
        assert_eq!(stack.active, 2);

        // 对外可见窗口必须同步变为新加入的窗口 3。
        assert_eq!(stack.active_window(), Some(3));
    }

    /// 验证 stack 的 next 操作会在窗口列表中循环切换。
    #[test]
    fn stack_next_wraps_around() {
        let mut stack = Stack::new(1, 2);

        // 新建 stack 默认激活第二个窗口。
        assert_eq!(stack.active_window(), Some(2));

        // 从最后一个窗口继续 next 时必须回到第一个窗口。
        assert_eq!(stack.next(), Some(1));

        // 再次 next 必须回到第二个窗口，形成稳定循环。
        assert_eq!(stack.next(), Some(2));
    }

    /// 验证删除 stack 的 active window 后，active 索引会回到合法范围。
    #[test]
    fn stack_remove_active_window_repairs_active_index() {
        let mut stack = Stack::new(1, 2);
        stack.push(3);

        // 删除当前 active window 3 必须成功。
        assert!(stack.remove_window(3));

        // 删除只影响目标窗口，原有窗口顺序必须保持不变。
        assert_eq!(stack.windows, vec![1, 2]);

        // active 必须落在新 Vec 的合法范围内。
        assert!(stack.active < stack.windows.len());

        // 删除末尾 active 后应回退到剩余的最后一个窗口 2。
        assert_eq!(stack.active_window(), Some(2));
    }

    /// 验证删除非 active 窗口不会让当前 active window 错位。
    #[test]
    fn stack_remove_non_active_window_preserves_active_window() {
        let mut stack = Stack::new(1, 2);
        stack.push(3);

        // 当前 active 是窗口 3；删除它之前的窗口 1 必须同步左移 active 索引。
        assert_eq!(stack.active_window(), Some(3));
        assert!(stack.remove_window(1));

        assert_eq!(stack.windows, vec![2, 3]);
        assert!(stack.active < stack.windows.len());

        // 删除非 active 成员后，可见窗口仍必须是原 active 窗口 3。
        assert_eq!(stack.active_window(), Some(3));
    }

    /// 验证删除 Single 窗口后，对应固定 slot 会恢复为 Empty。
    #[test]
    fn workspace_remove_single_window_makes_slot_empty() {
        let mut workspace = Workspace::new(0);
        workspace.assign_window(1);

        // 当前 workspace 中存在窗口 1，因此删除必须成功。
        assert!(workspace.remove_window(1));

        // Single 删除后不得保留无效窗口内容，slot 0 必须变为空。
        assert!(matches!(workspace.slots[0].content, SlotContent::Empty));

        // 统一读取接口必须同步反映 Empty 状态。
        assert_eq!(workspace.slot_window(0), None);
    }

    /// 验证删除窗口会清理同一 workspace 内的全部重复引用。
    #[test]
    fn workspace_remove_window_clears_all_duplicate_references() {
        let mut workspace = Workspace::new(0);

        // session 或异常外部数据可能把同一 WindowId 恢复到多个固定 slot。
        workspace.slots[0].content = SlotContent::Single(7);
        workspace.slots[1].content = SlotContent::Single(7);

        assert!(workspace.remove_window(7));

        // destroy 完成后不得残留任何 dead WindowId，否则 layout 仍可能把它显示出来。
        assert!(!workspace.window_ids().contains(&7));
        assert!(matches!(workspace.slots[0].content, SlotContent::Empty));
        assert!(matches!(workspace.slots[1].content, SlotContent::Empty));
    }

    /// 验证删除窗口会清理同一 Stack 内的全部重复引用。
    #[test]
    fn workspace_remove_window_clears_duplicate_references_inside_stack() {
        let mut workspace = Workspace::new(0);
        workspace.slots[0].content = SlotContent::Stack(Stack {
            windows: vec![7, 8, 7],
            active: 1,
        });

        assert!(workspace.remove_window(7));

        // 重复 WindowId 全部移除后只剩窗口 8，SlotContent 必须同步降级为 Single。
        assert!(!workspace.window_ids().contains(&7));
        assert!(matches!(workspace.slots[0].content, SlotContent::Single(8)));
    }

    /// 验证异常单元素 Stack 删除最后一个窗口后会规范化为 Empty。
    #[test]
    fn workspace_remove_last_stack_window_makes_slot_empty() {
        let mut workspace = Workspace::new(0);
        workspace.slots[0].content = SlotContent::Stack(Stack {
            windows: vec![7],
            active: 0,
        });

        // session 恢复可能带来不满足常规二元素起点的 Stack，删除路径仍必须安全收束。
        assert!(workspace.remove_window(7));

        assert!(matches!(workspace.slots[0].content, SlotContent::Empty));
        assert_eq!(workspace.slot_window(0), None);
    }

    /// 验证 Stack 删除到只剩一个窗口时，会降级为 Single。
    #[test]
    fn workspace_remove_from_stack_downgrades_to_single_when_one_left() {
        let mut workspace = Workspace::new(0);
        for window in 1..=5 {
            workspace.assign_window(window);
        }

        // slot 0 的 active 窗口 5 必须能从 Stack([1, 5]) 中删除。
        assert!(workspace.remove_window(5));

        // 只剩窗口 1 时必须使用 Single 表达，避免保留单元素 Stack。
        assert!(matches!(workspace.slots[0].content, SlotContent::Single(1)));

        // 删除后 slot 0 的可见窗口必须回到剩余窗口 1。
        assert_eq!(workspace.slot_window(0), Some(1));
    }

    /// 验证 window_ids 同时包含 Single 和 Stack 中的所有窗口。
    #[test]
    fn workspace_window_ids_includes_single_and_stack_windows() {
        let mut workspace = Workspace::new(0);
        for window in 1..=5 {
            workspace.assign_window(window);
        }

        let windows = workspace.window_ids();

        // 四个 Single 与 slot 0 Stack 中的两个引用去重后应覆盖窗口 1..=5。
        assert_eq!(windows.len(), 5);

        // 收集结果必须包含每个逻辑窗口 ID，包括非 active 的 stack 窗口 1。
        for window in 1..=5 {
            assert!(windows.contains(&window));
        }
    }
}
