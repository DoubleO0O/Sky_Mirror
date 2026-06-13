//! Smithay 输出尺寸变化事件适配探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不接真实 DRM、不接真实 Winit、不接 udev，也不创建真实 Output。
//!
//! 它只负责把未来后端输出尺寸变化信息转换为 `BackendEvent::OutputResized`。
//! 真正修改核心输出尺寸的逻辑，仍然发生在事件经过 `BackendDriverRunner`
//! 进入核心状态之后。新的输出尺寸会影响后续布局计算和渲染帧。

use crate::core::backend_event::BackendEvent;

/// Smithay 输出事件适配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该模块只生成纯数据 `BackendEvent`，
/// 不处理真实 DRM 或 Winit 输出。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayOutputEventMode {
    /// 纯探针模式。
    ///
    /// 不接真实 DRM，不接 Winit，也不创建真实 Output。
    ProbeOnly,
}

/// 输出尺寸变化描述信息。
///
/// 该结构不保存真实 Smithay 输出，只保存未来可由后端输出事件提取出的
/// 最小纯数据描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayOutputResizeDescriptor {
    /// 新输出宽度。
    ///
    /// 这里使用核心 `OutputState` 能理解的像素宽度。
    pub width: u32,

    /// 新输出高度。
    ///
    /// 这里使用核心 `OutputState` 能理解的像素高度。
    pub height: u32,
}

impl SmithayOutputResizeDescriptor {
    /// 创建一个输出尺寸变化描述。
    ///
    /// 本方法只保存纯数据尺寸，不校验真实显示器状态。
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// 判断该尺寸是否为非零尺寸。
    ///
    /// 核心 Validator 会把零宽或零高视为无效输出尺寸；这里提供只读辅助方法，
    /// 方便测试和未来后端进行显式判断，但事件探针不会自动过滤该尺寸。
    pub fn is_non_zero(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Smithay 输出事件适配探针。
///
/// 该类型不持有状态，也不保存真实输出。
/// 它只把输出尺寸变化描述转换成 `BackendEvent::OutputResized`。
pub struct SmithayOutputEventProbe;

impl SmithayOutputEventProbe {
    /// 返回当前适配器模式。
    pub fn mode() -> SmithayOutputEventMode {
        SmithayOutputEventMode::ProbeOnly
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only() -> bool {
        true
    }

    /// 把输出尺寸变化描述转换成 `BackendEvent`。
    ///
    /// 未来真实 Smithay、DRM 或 Winit 输出尺寸变化回调应先收集纯数据描述，
    /// 再通过该路径生成 `BackendEvent`，而不是直接修改核心 `State`。
    /// 真正的输出尺寸修改要等事件经过 `run_once()` 后由核心完成，并影响后续
    /// 布局计算和渲染帧。
    pub fn output_resized_event(descriptor: SmithayOutputResizeDescriptor) -> BackendEvent {
        BackendEvent::OutputResized {
            width: descriptor.width,
            height: descriptor.height,
        }
    }

    /// 返回当前阶段说明。
    pub fn mode_description() -> &'static str {
        "smithay-output-event-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{SmithayOutputEventMode, SmithayOutputEventProbe, SmithayOutputResizeDescriptor};
    use crate::core::backend_event::BackendEvent;

    /// 验证输出尺寸描述器会原样保存宽度和高度。
    #[test]
    fn output_resize_descriptor_builds_size() {
        let descriptor = SmithayOutputResizeDescriptor::new(2560, 1440);

        assert_eq!(descriptor.width, 2560);
        assert_eq!(descriptor.height, 1440);
        assert!(descriptor.is_non_zero());
    }

    /// 验证描述器能够只读识别任一维度为零的尺寸。
    #[test]
    fn output_resize_descriptor_detects_zero_size() {
        assert!(!SmithayOutputResizeDescriptor::new(0, 1440).is_non_zero());
        assert!(!SmithayOutputResizeDescriptor::new(2560, 0).is_non_zero());
    }

    /// 验证输出事件探针会生成完整的纯数据 OutputResized 事件。
    #[test]
    fn output_event_probe_creates_output_resized_event() {
        let event = SmithayOutputEventProbe::output_resized_event(
            SmithayOutputResizeDescriptor::new(2560, 1440),
        );

        assert_eq!(
            event,
            BackendEvent::OutputResized {
                width: 2560,
                height: 1440,
            }
        );
    }

    /// 验证探针不会拦截零尺寸，尺寸有效性仍由核心 Validator 判断。
    #[test]
    fn output_event_probe_preserves_zero_size() {
        let event = SmithayOutputEventProbe::output_resized_event(
            SmithayOutputResizeDescriptor::new(0, 900),
        );

        assert_eq!(
            event,
            BackendEvent::OutputResized {
                width: 0,
                height: 900,
            }
        );
    }

    /// 验证输出事件适配器固定保持纯探针模式。
    #[test]
    fn output_event_probe_reports_probe_mode() {
        assert!(SmithayOutputEventProbe::is_probe_only());
        assert_eq!(
            SmithayOutputEventProbe::mode(),
            SmithayOutputEventMode::ProbeOnly
        );
        assert_eq!(
            SmithayOutputEventProbe::mode_description(),
            "smithay-output-event-probe-only"
        );
    }
}
