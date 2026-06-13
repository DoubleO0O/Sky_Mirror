//! Smithay toplevel 映射事件适配探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不保存真实 `xdg_toplevel`，不接 xdg-shell，不注册 `xdg_wm_base`，
//! 也不注册 `wl_compositor`。
//!
//! 它只负责把未来 xdg_toplevel map 信息转换为 `BackendEvent::ToplevelMapped`。
//! 注意：`SurfaceCreated` 只表示 surface 出现，`ToplevelMapped` 才表示该 surface
//! 被映射为 compositor 内部可以管理的窗口。

use crate::core::{backend_event::BackendEvent, surface::SurfaceId, window::WindowKind};

/// Smithay toplevel event 适配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该模块只生成纯数据 `BackendEvent`，
/// 不处理真实 `xdg_toplevel`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayToplevelEventMode {
    /// 纯探针模式。
    ///
    /// 不保存真实 `xdg_toplevel`，不注册 xdg-shell。
    ProbeOnly,
}

/// toplevel 映射描述信息。
///
/// 该结构不保存真实 `xdg_toplevel`，只保存未来可由 Smithay xdg-shell callback
/// 提取出的最小纯数据 metadata。
///
/// `SurfaceCreated` 与 `ToplevelMapped` 的语义不同：
/// `SurfaceCreated` 表示 surface 被创建；`ToplevelMapped` 表示该 surface 已映射为窗口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayToplevelMapDescriptor {
    /// 被映射成窗口的 surface ID。
    pub surface: SurfaceId,

    /// 窗口标题。
    ///
    /// 真实 `xdg_toplevel` 的 title 可能为空或后续变化；当前只记录 map 时的标题。
    pub title: String,

    /// 应用 ID。
    ///
    /// 真实 `xdg_toplevel` 的 app_id 可能为空；`None` 表示当前无法确定。
    pub app_id: Option<String>,

    /// 映射后的窗口类型。
    ///
    /// 当前 Smithay 探针层默认使用 `WindowKind::WaylandPlaceholder`。
    pub kind: WindowKind,
}

impl SmithayToplevelMapDescriptor {
    /// 创建一个 toplevel map 描述。
    ///
    /// 默认 kind 使用 `WaylandPlaceholder`，因为该事件来自 Smithay/Wayland 路径。
    pub fn new(surface: SurfaceId, title: impl Into<String>, app_id: Option<String>) -> Self {
        Self {
            surface,
            title: title.into(),
            app_id,
            kind: WindowKind::WaylandPlaceholder,
        }
    }

    /// 创建一个允许指定 `WindowKind` 的 toplevel map 描述。
    ///
    /// 该方法主要用于测试或未来扩展；普通 Smithay 路径应使用
    /// `WaylandPlaceholder`。
    pub fn with_kind(
        surface: SurfaceId,
        title: impl Into<String>,
        app_id: Option<String>,
        kind: WindowKind,
    ) -> Self {
        Self {
            surface,
            title: title.into(),
            app_id,
            kind,
        }
    }
}

/// Smithay toplevel event 适配探针。
///
/// 该类型不持有状态，也不保存真实 `xdg_toplevel`。
/// 它只把 toplevel map 描述转换成 `BackendEvent::ToplevelMapped`。
pub struct SmithayToplevelEventProbe;

impl SmithayToplevelEventProbe {
    /// 返回当前适配器模式。
    pub fn mode() -> SmithayToplevelEventMode {
        SmithayToplevelEventMode::ProbeOnly
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only() -> bool {
        true
    }

    /// 把 toplevel map 描述转换成 `BackendEvent`。
    ///
    /// 未来真实 Smithay xdg_toplevel map callback 应先收集纯数据描述，
    /// 再通过该路径生成 `BackendEvent`，而不是直接修改核心 `State`。
    /// 只有该事件经过核心处理后，surface 才会映射为逻辑窗口。
    pub fn toplevel_mapped_event(descriptor: SmithayToplevelMapDescriptor) -> BackendEvent {
        BackendEvent::ToplevelMapped {
            surface: descriptor.surface,
            title: descriptor.title,
            app_id: descriptor.app_id,
            kind: descriptor.kind,
        }
    }

    /// 返回当前阶段说明。
    pub fn mode_description() -> &'static str {
        "smithay-toplevel-event-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayToplevelEventMode, SmithayToplevelEventProbe, SmithayToplevelMapDescriptor,
    };
    use crate::core::{backend_event::BackendEvent, window::WindowKind};

    /// 验证默认描述器会把 Smithay 路径标记为 Wayland 占位窗口。
    #[test]
    fn toplevel_map_descriptor_uses_wayland_placeholder_by_default() {
        let descriptor =
            SmithayToplevelMapDescriptor::new(42, "Terminal", Some("foot".to_string()));

        assert_eq!(descriptor.surface, 42);
        assert_eq!(descriptor.title, "Terminal");
        assert_eq!(descriptor.app_id, Some("foot".to_string()));
        assert_eq!(descriptor.kind, WindowKind::WaylandPlaceholder);
    }

    /// 验证显式窗口类型构造方法会原样保留调用方提供的纯数据类型。
    #[test]
    fn toplevel_map_descriptor_with_kind_preserves_kind() {
        let descriptor =
            SmithayToplevelMapDescriptor::with_kind(42, "Mock", None, WindowKind::Mock);

        assert_eq!(descriptor.surface, 42);
        assert_eq!(descriptor.title, "Mock");
        assert_eq!(descriptor.app_id, None);
        assert_eq!(descriptor.kind, WindowKind::Mock);
    }

    /// 验证 toplevel 事件探针会生成完整的纯数据 ToplevelMapped 事件。
    #[test]
    fn toplevel_event_probe_creates_toplevel_mapped_event() {
        let event = SmithayToplevelEventProbe::toplevel_mapped_event(
            SmithayToplevelMapDescriptor::new(42, "Terminal", Some("foot".to_string())),
        );

        assert_eq!(
            event,
            BackendEvent::ToplevelMapped {
                surface: 42,
                title: "Terminal".to_string(),
                app_id: Some("foot".to_string()),
                kind: WindowKind::WaylandPlaceholder,
            }
        );
    }

    /// 验证 toplevel 事件适配器固定保持纯探针模式。
    #[test]
    fn toplevel_event_probe_reports_probe_mode() {
        assert!(SmithayToplevelEventProbe::is_probe_only());
        assert_eq!(
            SmithayToplevelEventProbe::mode(),
            SmithayToplevelEventMode::ProbeOnly
        );
        assert_eq!(
            SmithayToplevelEventProbe::mode_description(),
            "smithay-toplevel-event-probe-only"
        );
    }
}
