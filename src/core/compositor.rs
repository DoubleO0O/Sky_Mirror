//! compositor 核心模块的预留边界。
//!
//! 当前主要实现集中在 `state` 模块的 `CompositorState` 中，因此本文件暂不定义类型。
//! 保留该模块是为了未来在不破坏现有集中状态模型的前提下承载更高层 compositor 组装逻辑。
//!
//! TODO: 只有当 Smithay 集成需要独立的协议处理组装边界时，才在此增加实现；
//! 当前阶段继续由 `CompositorState` 作为唯一 compositor-owned 可变状态容器。
