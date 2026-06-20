//! DRM backend 生命周期占位实现。
//!
//! 当前类型只用于验证 compositor 的 backend 创建与启动时序，并未连接真实的
//! DRM connector、GBM、EGL 或 Smithay backend。后续接入真实硬件后，可以在保持
//! `CompositorState` 集中持有 backend 的前提下替换这里的内部实现。
//! backend 属于进程运行期资源，不参与 session 序列化或恢复。
//!
//! Boundary: 构造或初始化这个 stub 不表示设备已打开、输出已发现或渲染器已就绪。

/// compositor 当前使用的最小 DRM backend stub。
///
/// 该类型暂时不保存设备或输出资源；它只提供明确的创建与初始化边界。
pub struct DrmBackend;

impl DrmBackend {
    /// 创建 backend 生命周期占位对象。
    ///
    /// 当前只构造一个无字段占位对象并打印日志，不打开任何 DRM 设备。
    pub fn new() -> Self {
        println!("[Backend] DRM backend created");
        Self
    }

    /// 初始化 backend 资源边界。
    ///
    /// 当前只打印初始化日志，表示 compositor 已进入 backend 启动阶段。
    /// 真实设备资源仍应由未来 Smithay/DRM/GBM 实现负责。
    /// Contract: 在接入系统资源前，这个方法不得暗示设备、输出或渲染能力可用。
    pub fn init(&mut self) {
        println!("[Backend] DRM initialized");
    }
}
