//! compositor 核心模块的预留边界。
//!
//! 当前主要实现集中在 `state` 模块的 `CompositorState` 中，因此本文件暂不定义类型。
//! 保留该模块是为了未来在不破坏现有集中状态模型的前提下承载更高层 compositor 组装逻辑。
//!
//! Boundary: 只有出现独立且可验证的组装职责时才应在此增加实现；
//! `CompositorState` 继续是 compositor-owned 可变状态的唯一容器。
