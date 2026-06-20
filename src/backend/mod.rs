//! 进程运行期 backend 的模块入口。
//!
//! 这些模块与 `core` 中的纯数据 backend driver 契约不同：这里预留系统资源所有权，
//! 但当前只有日志型 DRM stub，EGL 和输入模块仍为空边界。

/// 日志型 DRM 生命周期 stub。
pub mod drm;
/// EGL 系统资源的保留模块。
pub mod egl;
/// 系统输入资源的保留模块。
pub mod input;
