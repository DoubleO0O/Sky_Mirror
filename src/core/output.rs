//! compositor 当前输出状态的最小纯数据表示。
//!
//! OutputState 目前只保存逻辑尺寸，不连接真实 DRM connector 或 Smithay output。
//! 它由 CompositorState 持有，但刻意不进入 session：未来真实 backend 启动时应根据
//! 实际输出发现结果重新建立该状态。

use crate::core::layout::OutputSize;

/// 当前 compositor 使用的输出尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputState {
    /// 提供给 LayoutEngine 的逻辑输出尺寸。
    pub size: OutputSize,
}

impl OutputState {
    /// 使用明确尺寸创建输出状态。
    ///
    /// 该入口为未来真实 output discovery 或测试构造保留。
    pub fn new(size: OutputSize) -> Self {
        Self { size }
    }

    /// 创建当前 MVP 使用的默认虚拟输出。
    ///
    /// 1920x1080 只集中存在于 OutputState，而不会泄漏到 EventLoop。
    pub fn default_virtual() -> Self {
        Self {
            size: OutputSize {
                width: 1920,
                height: 1080,
            },
        }
    }

    /// 更新输出尺寸并记录日志。
    ///
    /// 此操作只修改 output 数据，不触碰 workspace、focus 或 session。
    /// 后续真实 backend 可在 mode change 时通过 Action 路径调用该入口。
    pub fn resize(&mut self, size: OutputSize) {
        // 输出尺寸集中写入 OutputState，避免 EventLoop 或 LayoutEngine 保存重复副本。
        self.size = size;
        println!(
            "[Output] resized to {}x{}",
            self.size.width, self.size.height
        );
    }

    /// 返回当前输出尺寸的 Copy 值。
    pub fn size(&self) -> OutputSize {
        self.size
    }
}
