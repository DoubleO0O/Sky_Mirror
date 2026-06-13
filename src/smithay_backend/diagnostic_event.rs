//! Smithay 诊断请求事件适配探针。
//!
//! 本模块只在启用 `smithay-probe` feature 时编译。
//! 当前阶段不直接读取核心 `State`，也不直接调用核心完整诊断或状态验证入口。
//!
//! 它只负责把未来后端调试入口、开发者命令或调试快捷键转换为核心可理解的
//! `BackendEvent::DebugRequested` 或 `BackendEvent::ValidateRequested`。
//! 真正生成诊断文本的逻辑，仍然发生在事件经过 `BackendDriverRunner` 进入核心之后。

use crate::core::backend_event::BackendEvent;

/// Smithay 诊断事件适配器当前模式。
///
/// 当前只允许 `ProbeOnly`，表示该模块只生成纯数据 `BackendEvent`，
/// 不直接读取 `State`，也不直接生成诊断文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayDiagnosticEventMode {
    /// 纯探针模式。
    ///
    /// 不直接调用核心完整诊断或状态验证入口。
    ProbeOnly,
}

/// 诊断请求类型。
///
/// `DebugText` 用于请求完整诊断包文本，`Validate` 用于请求状态验证文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithayDiagnosticRequestKind {
    /// 请求完整诊断包文本。
    ///
    /// 该文本通常包含状态快照与验证报告两部分。
    DebugText,

    /// 请求状态一致性验证文本。
    ///
    /// 该文本只关注 Validator 报告。
    Validate,
}

/// 诊断请求描述信息。
///
/// 该结构只保存“请求哪种诊断”的纯数据，不直接访问核心 `State`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmithayDiagnosticRequestDescriptor {
    /// 诊断请求类型。
    ///
    /// `DebugText` 和 `Validate` 最终会被转换为不同的 `BackendEvent`。
    pub kind: SmithayDiagnosticRequestKind,
}

impl SmithayDiagnosticRequestDescriptor {
    /// 创建一个诊断请求描述。
    ///
    /// 本方法只保存请求类型，不读取状态或生成文本。
    pub fn new(kind: SmithayDiagnosticRequestKind) -> Self {
        Self { kind }
    }

    /// 创建完整诊断文本请求。
    ///
    /// 真正的诊断包文本会在 `run_once()` 后由核心生成。
    pub fn debug_text() -> Self {
        Self::new(SmithayDiagnosticRequestKind::DebugText)
    }

    /// 创建状态验证请求。
    ///
    /// 真正的验证文本会在 `run_once()` 后由核心生成。
    pub fn validate() -> Self {
        Self::new(SmithayDiagnosticRequestKind::Validate)
    }
}

/// Smithay 诊断事件适配探针。
///
/// 该类型不持有 `State`，也不直接生成诊断文本。
/// 它只把诊断请求描述转换成对应的 `BackendEvent`。
pub struct SmithayDiagnosticEventProbe;

impl SmithayDiagnosticEventProbe {
    /// 返回当前适配器模式。
    pub fn mode() -> SmithayDiagnosticEventMode {
        SmithayDiagnosticEventMode::ProbeOnly
    }

    /// 当前是否仍然只是纯探针模式。
    pub fn is_probe_only() -> bool {
        true
    }

    /// 把诊断请求描述转换成 `BackendEvent`。
    ///
    /// 未来真实调试入口应先生成该纯数据描述，再通过该路径请求核心诊断文本，
    /// 而不是直接读取核心 `State`。完整诊断请求与验证请求保持不同事件语义。
    pub fn diagnostic_requested_event(
        descriptor: SmithayDiagnosticRequestDescriptor,
    ) -> BackendEvent {
        match descriptor.kind {
            SmithayDiagnosticRequestKind::DebugText => BackendEvent::DebugRequested,
            SmithayDiagnosticRequestKind::Validate => BackendEvent::ValidateRequested,
        }
    }

    /// 直接生成完整诊断文本请求事件。
    ///
    /// 该方法只生成事件，不读取状态；文本仍由核心在 `run_once()` 后生成。
    pub fn debug_requested_event() -> BackendEvent {
        BackendEvent::DebugRequested
    }

    /// 直接生成状态验证请求事件。
    ///
    /// 该方法只生成事件，不运行 Validator；验证文本仍由核心生成。
    pub fn validate_requested_event() -> BackendEvent {
        BackendEvent::ValidateRequested
    }

    /// 返回当前阶段说明。
    pub fn mode_description() -> &'static str {
        "smithay-diagnostic-event-probe-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithayDiagnosticEventMode, SmithayDiagnosticEventProbe,
        SmithayDiagnosticRequestDescriptor, SmithayDiagnosticRequestKind,
    };
    use crate::core::backend_event::BackendEvent;

    /// 验证诊断请求描述器辅助构造方法会生成对应请求类型。
    #[test]
    fn diagnostic_request_descriptor_builders_work() {
        assert_eq!(
            SmithayDiagnosticRequestDescriptor::debug_text().kind,
            SmithayDiagnosticRequestKind::DebugText
        );
        assert_eq!(
            SmithayDiagnosticRequestDescriptor::validate().kind,
            SmithayDiagnosticRequestKind::Validate
        );
    }

    /// 验证完整诊断请求会转换为 DebugRequested 事件。
    #[test]
    fn diagnostic_event_probe_creates_debug_requested_event() {
        assert_eq!(
            SmithayDiagnosticEventProbe::diagnostic_requested_event(
                SmithayDiagnosticRequestDescriptor::debug_text(),
            ),
            BackendEvent::DebugRequested
        );
    }

    /// 验证状态验证请求会转换为 ValidateRequested 事件。
    #[test]
    fn diagnostic_event_probe_creates_validate_requested_event() {
        assert_eq!(
            SmithayDiagnosticEventProbe::diagnostic_requested_event(
                SmithayDiagnosticRequestDescriptor::validate(),
            ),
            BackendEvent::ValidateRequested
        );
    }

    /// 验证诊断事件适配器固定保持纯探针模式。
    #[test]
    fn diagnostic_event_probe_reports_probe_mode() {
        assert!(SmithayDiagnosticEventProbe::is_probe_only());
        assert_eq!(
            SmithayDiagnosticEventProbe::mode(),
            SmithayDiagnosticEventMode::ProbeOnly
        );
        assert_eq!(
            SmithayDiagnosticEventProbe::mode_description(),
            "smithay-diagnostic-event-probe-only"
        );
    }
}
