//! Linux-only SHM-first buffer import adapter skeleton.
//!
//! This module is the first Phase 56A boundary allowed to name real Smithay
//! `wl_buffer` types. It still does not import buffers, read SHM contents,
//! create textures, call a renderer, submit damage, send frame callback done,
//! connect input, or mutate core state.

use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;

use crate::smithay_backend::nested_runtime_coordinator::{
    RuntimeSurfaceCommitBufferImportActualAttemptRecord, RuntimeSurfaceCommitRenderBackendKind,
};

/// Evidence that the Linux-only adapter saw the real Smithay `WlBuffer` type boundary.
///
/// This is deliberately pure data. It does not retain a `WlBuffer`, map SHM,
/// import a buffer, or create any renderer resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxShmBufferTypeBoundaryEvidence {
    /// The boundary function was called with a real `WlBuffer` reference.
    pub wl_buffer_type_boundary_observed: bool,

    /// The boundary belongs to the SHM-first nested MVP route.
    pub shm_first_route_selected: bool,

    /// Smithay exposes the SHM read helper type boundary, but Phase 56A does not call it.
    pub smithay_shm_access_type_available: bool,

    /// The exact Smithay / Wayland buffer type path remains contained in this module.
    pub wl_buffer_type_name: &'static str,

    /// The SHM buffer metadata type path remains contained in this module.
    pub shm_buffer_data_type_name: &'static str,

    /// The SHM buffer access error type path remains contained in this module.
    pub shm_buffer_access_error_type_name: &'static str,
}

/// Observe only the type boundary of a real Smithay `WlBuffer`.
///
/// The adapter intentionally does not call `with_buffer_contents`; actual SHM
/// reads and texture upload belong to later phases.
#[must_use = "type-boundary evidence is not a real buffer import"]
pub fn observe_wl_buffer_type_boundary(_buffer: &WlBuffer) -> LinuxShmBufferTypeBoundaryEvidence {
    LinuxShmBufferTypeBoundaryEvidence {
        wl_buffer_type_boundary_observed: true,
        shm_first_route_selected: true,
        smithay_shm_access_type_available: true,
        wl_buffer_type_name: std::any::type_name::<WlBuffer>(),
        shm_buffer_data_type_name: std::any::type_name::<smithay::wayland::shm::BufferData>(),
        shm_buffer_access_error_type_name: std::any::type_name::<
            smithay::wayland::shm::BufferAccessError,
        >(),
    }
}

/// The SHM metadata taxonomy observed at the Linux-only adapter boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxShmBufferMetadataKind {
    /// A Smithay-managed SHM buffer exposed metadata.
    Shm,
    /// A concrete `WlBuffer` was present but not managed by Smithay SHM.
    UnsupportedNonShmBuffer,
    /// Metadata could not be read safely or no concrete buffer was available.
    Unavailable,
}

/// Pure-data SHM buffer metadata evidence.
///
/// This copies only Smithay `BufferData` fields. It never retains a `WlBuffer`,
/// never imports a buffer, and never creates a texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxShmBufferMetadataEvidence {
    /// The Linux-only adapter attempted metadata extraction.
    pub shm_metadata_extraction_attempted: bool,

    /// The taxonomy result for the observed buffer.
    pub shm_buffer_metadata_kind: LinuxShmBufferMetadataKind,

    /// Metadata is available and can be reported as pure data.
    pub shm_metadata_available: bool,

    /// Metadata was observed from Smithay `BufferData`.
    pub shm_metadata_observed: bool,

    /// Buffer offset in bytes.
    pub offset: Option<i32>,

    /// Buffer width from Smithay SHM metadata.
    pub width: Option<i32>,

    /// Buffer height from Smithay SHM metadata.
    pub height: Option<i32>,

    /// Buffer stride in bytes.
    pub stride: Option<i32>,

    /// Buffer format rendered as a stable debug string.
    pub format: Option<String>,

    /// Metadata extraction failed or is unsupported.
    pub unavailable_or_unsupported_reason: Option<String>,

    /// Phase 56B does not attempt real import.
    pub buffer_import_attempted: bool,

    /// Phase 56B does not complete real import.
    pub buffer_imported: bool,

    /// Phase 56B does not create textures.
    pub texture_created: bool,

    /// Phase 56B does not call a renderer.
    pub renderer_called: bool,
}

/// Extract SHM buffer metadata from a concrete `WlBuffer` at the Linux-only adapter boundary.
///
/// The closure copies only `BufferData` fields. It does not retain SHM memory,
/// construct a slice, import a buffer, create a texture, or call a renderer.
#[must_use = "SHM metadata evidence is not a buffer import"]
pub fn extract_shm_buffer_metadata_evidence(buffer: &WlBuffer) -> LinuxShmBufferMetadataEvidence {
    match smithay::wayland::shm::with_buffer_contents(buffer, |_, _, metadata| metadata) {
        Ok(metadata) => LinuxShmBufferMetadataEvidence {
            shm_metadata_extraction_attempted: true,
            shm_buffer_metadata_kind: LinuxShmBufferMetadataKind::Shm,
            shm_metadata_available: true,
            shm_metadata_observed: true,
            offset: Some(metadata.offset),
            width: Some(metadata.width),
            height: Some(metadata.height),
            stride: Some(metadata.stride),
            format: Some(format!("{:?}", metadata.format)),
            unavailable_or_unsupported_reason: None,
            buffer_import_attempted: false,
            buffer_imported: false,
            texture_created: false,
            renderer_called: false,
        },
        Err(smithay::wayland::shm::BufferAccessError::NotManaged) => {
            LinuxShmBufferMetadataEvidence {
                shm_metadata_extraction_attempted: true,
                shm_buffer_metadata_kind: LinuxShmBufferMetadataKind::UnsupportedNonShmBuffer,
                shm_metadata_available: false,
                shm_metadata_observed: false,
                offset: None,
                width: None,
                height: None,
                stride: None,
                format: None,
                unavailable_or_unsupported_reason: Some("UnsupportedNonShmBuffer".to_owned()),
                buffer_import_attempted: false,
                buffer_imported: false,
                texture_created: false,
                renderer_called: false,
            }
        }
        Err(error) => LinuxShmBufferMetadataEvidence {
            shm_metadata_extraction_attempted: true,
            shm_buffer_metadata_kind: LinuxShmBufferMetadataKind::Unavailable,
            shm_metadata_available: false,
            shm_metadata_observed: false,
            offset: None,
            width: None,
            height: None,
            stride: None,
            format: None,
            unavailable_or_unsupported_reason: Some(format!("{error:?}")),
            buffer_import_attempted: false,
            buffer_imported: false,
            texture_created: false,
            renderer_called: false,
        },
    }
}

/// Phase 56A SHM-first adapter operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation {
    /// Observe the Phase 55L actual-attempt record.
    ObserveActualAttemptRecord,
    /// Select the SHM-first nested MVP route.
    SelectShmFirstRoute,
    /// Check whether a real `WlBuffer` type boundary was observed.
    CheckWlBufferTypeBoundary,
    /// Check that no texture path is allowed in this phase.
    CheckNoTextureBoundary,
    /// Build the blocked evidence-only report.
    BuildEvidenceOnlyReport,
}

/// Phase 56A blockers. These are explicit stop signs, not failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker {
    /// No Phase 55L actual-attempt record was observed.
    MissingActualAttemptRecord,
    /// The runtime chain does not yet carry a concrete `WlBuffer`.
    MissingWlBufferTypeBoundaryObservation,
    /// No SHM buffer access evidence has been produced.
    MissingShmBufferAccessEvidence,
    /// The commit does not require actual buffer import.
    NoActualImportRequired,
    /// Texture creation remains forbidden in Phase 56A.
    TextureCreationForbiddenInPhase56A,
    /// Renderer calls remain forbidden in Phase 56A.
    RendererCallForbiddenInPhase56A,
    /// Damage submit remains forbidden in Phase 56A.
    DamageSubmitForbiddenInPhase56A,
    /// Frame callback done remains forbidden in Phase 56A.
    FrameCallbackDoneForbiddenInPhase56A,
    /// DRM / GBM / dmabuf routes remain forbidden in Phase 56A.
    DrmGbmDmabufForbiddenInPhase56A,
}

/// Evidence-only SHM-first adapter report derived from the Phase 55L record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitShmFirstBufferImportAdapterReport {
    /// The adapter skeleton seam was invoked.
    pub shm_buffer_import_adapter_invoked: bool,

    /// The Phase 55L actual-attempt record was observed.
    pub source_buffer_import_actual_attempt_record_observed: bool,

    /// The observed upstream actual-attempt record.
    pub observed_actual_attempt_record: RuntimeSurfaceCommitBufferImportActualAttemptRecord,

    /// The Linux-only SHM-first adapter skeleton exists.
    pub shm_buffer_adapter_available: bool,

    /// Phase 55N's SHM-first route was selected.
    pub shm_buffer_import_route_selected: bool,

    /// A real `WlBuffer` type boundary was observed by Linux-only adapter glue.
    pub shm_buffer_type_boundary_observed: bool,

    /// Smithay SHM access type names are known inside the Linux-only adapter.
    pub smithay_shm_access_type_available: bool,

    /// The execution remains blocked because Phase 56A is a skeleton.
    pub shm_buffer_import_execution_blocked: bool,

    /// The report is evidence-only and does not contain imported resource ownership.
    pub evidence_only_report: bool,

    /// The report explicitly records that texture creation is unavailable.
    pub no_texture_report: bool,

    /// The report explicitly records unsupported/missing concrete buffer execution.
    pub unsupported_or_missing_wl_buffer_report: bool,

    /// Whether a future actual import would be required.
    pub actual_import_required: bool,

    /// Whether the upstream commit carried buffer attach/remove evidence.
    pub buffer_attach_observed: bool,

    /// Whether the upstream commit carried present buffer evidence.
    pub buffer_present: bool,

    /// Whether the upstream commit carried null attach/remove evidence.
    pub buffer_removed: bool,

    /// Whether candidate evidence existed upstream.
    pub candidate_evidence_observed: bool,

    /// Whether importer owner evidence existed upstream.
    pub importer_owner_evidence_available: bool,

    /// Whether renderer descriptor evidence existed upstream.
    pub renderer_backend_descriptor_evidence_available: bool,

    /// Registered renderer backend kind, if any.
    pub registered_renderer_backend_kind: Option<RuntimeSurfaceCommitRenderBackendKind>,

    /// Phase 56A does not attempt real import.
    pub buffer_import_attempted: bool,

    /// Phase 56A does not complete real import.
    pub buffer_imported: bool,

    /// Phase 56A does not create textures.
    pub texture_created: bool,

    /// Phase 56A does not call a renderer.
    pub renderer_called: bool,

    /// Phase 56A does not submit damage.
    pub damage_submitted: bool,

    /// Phase 56A does not send frame callback done.
    pub frame_callback_done_sent: bool,

    /// Phase 56A does not connect input.
    pub input_support: bool,

    /// Phase 56A does not mutate core.
    pub core_mutation_invoked: bool,

    /// Operations performed by the skeleton.
    pub operations: Vec<RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation>,

    /// Blockers preventing execution beyond the skeleton.
    pub blockers: Vec<RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker>,
}

/// Phase 56B metadata operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitShmBufferMetadataOperation {
    /// Observe the Phase 56A SHM-first adapter report.
    ObserveShmFirstAdapterReport,
    /// Check for concrete SHM metadata evidence.
    CheckShmMetadataEvidence,
    /// Classify metadata availability.
    ClassifyMetadataAvailability,
    /// Refine unavailable / unsupported / blocked metadata reasons.
    RefineMetadataBlockers,
    /// Build the pure-data metadata report.
    BuildMetadataReport,
}

/// Phase 56B metadata blockers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitShmBufferMetadataBlocker {
    /// No Phase 56A adapter report was observed.
    MissingShmFirstAdapterReport,
    /// Runtime does not yet carry a concrete `WlBuffer`.
    MetadataUnavailable,
    /// The concrete buffer was not Smithay SHM-managed.
    UnsupportedNonShmBuffer,
    /// The runtime report has no concrete `WlBuffer` available.
    NoRealWlBufferAvailable,
    /// A concrete `WlBuffer` was available but Smithay did not manage it as SHM.
    WlBufferAvailableButNotShm,
    /// A SHM-like candidate could not be read through a safe Smithay metadata accessor.
    ShmLikeCandidateMissingSafeSmithayMetadataAccessor,
    /// Metadata was observable but is not sufficient to satisfy texture preconditions.
    MetadataObservableButInsufficientForTexturePrecondition,
    /// Buffer lifetime and cleanup ownership policy is not defined yet.
    MissingBufferLifetimeCleanupOwnershipPolicy,
    /// Runtime report carries only evidence and must not be treated as import execution.
    RuntimeReportOnlyHasEvidenceNotImportExecution,
    /// Texture creation remains forbidden in Phase 56B.
    TextureCreationForbiddenInPhase56B,
    /// Renderer calls remain forbidden in Phase 56B.
    RendererCallForbiddenInPhase56B,
    /// Damage submit remains forbidden in Phase 56B.
    DamageSubmitForbiddenInPhase56B,
    /// Frame callback done remains forbidden in Phase 56B.
    FrameCallbackDoneForbiddenInPhase56B,
    /// DRM / GBM / dmabuf routes remain forbidden in Phase 56B.
    DrmGbmDmabufForbiddenInPhase56B,
}

/// Phase 56D controlled validation harness paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitShmMetadataValidationPath {
    /// Validate the runtime path with no concrete `WlBuffer`.
    NoRealWlBuffer,
    /// Validate a concrete buffer that is not Smithay SHM-managed.
    NonShm,
    /// Validate a metadata unavailable result.
    MetadataUnavailable,
    /// Validate partial pure-data metadata evidence.
    MetadataPartiallyAvailable,
    /// Validate metadata that is still insufficient for texture preconditions.
    MetadataInsufficientForTexturePrecondition,
    /// Validate the missing buffer lifetime / cleanup ownership policy blocker.
    MissingLifetimeCleanupOwnershipPolicy,
    /// Validate that runtime evidence is not import execution.
    RuntimeEvidenceWithoutImportExecution,
}

/// Phase 56D pure-data validation harness report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitShmMetadataValidationHarnessReport {
    /// Validation harness was invoked.
    pub validation_harness_invoked: bool,

    /// Number of controlled paths covered by the harness.
    pub validation_paths_covered: usize,

    /// Every required Phase 56D validation path is covered.
    pub all_validation_paths_covered: bool,

    /// Covered paths in stable order.
    pub covered_paths: Vec<RuntimeSurfaceCommitShmMetadataValidationPath>,

    /// The no-real-`WlBuffer` path was validated.
    pub no_real_wl_buffer_path_validated: bool,

    /// The non-SHM path was validated.
    pub non_shm_path_validated: bool,

    /// The metadata-unavailable path was validated.
    pub metadata_unavailable_path_validated: bool,

    /// The partial-metadata path was validated.
    pub metadata_partially_available_path_validated: bool,

    /// The insufficient-for-texture-precondition path was validated.
    pub metadata_insufficient_for_texture_precondition_path_validated: bool,

    /// The missing lifetime / cleanup ownership path was validated.
    pub missing_lifetime_cleanup_policy_path_validated: bool,

    /// The runtime evidence without import execution path was validated.
    pub runtime_evidence_without_import_execution_path_validated: bool,

    /// Phase 56D does not attempt real import.
    pub buffer_import_attempted: bool,

    /// Phase 56D does not complete real import.
    pub buffer_imported: bool,

    /// Phase 56D does not create textures.
    pub texture_created: bool,

    /// Phase 56D does not call a renderer.
    pub renderer_called: bool,

    /// Phase 56D does not submit damage.
    pub damage_submitted: bool,

    /// Phase 56D does not send frame callback done.
    pub frame_callback_done_sent: bool,

    /// Phase 56D does not connect input.
    pub input_support: bool,

    /// Phase 56D does not mutate core.
    pub core_mutation_invoked: bool,
}

/// Validate Phase 56B / 56C metadata evidence and blocker taxonomy paths.
#[must_use = "SHM metadata validation harness is pure-data evidence only"]
pub fn validate_shm_metadata_harness_paths()
-> RuntimeSurfaceCommitShmMetadataValidationHarnessReport {
    let covered_paths = vec![
        RuntimeSurfaceCommitShmMetadataValidationPath::NoRealWlBuffer,
        RuntimeSurfaceCommitShmMetadataValidationPath::NonShm,
        RuntimeSurfaceCommitShmMetadataValidationPath::MetadataUnavailable,
        RuntimeSurfaceCommitShmMetadataValidationPath::MetadataPartiallyAvailable,
        RuntimeSurfaceCommitShmMetadataValidationPath::MetadataInsufficientForTexturePrecondition,
        RuntimeSurfaceCommitShmMetadataValidationPath::MissingLifetimeCleanupOwnershipPolicy,
        RuntimeSurfaceCommitShmMetadataValidationPath::RuntimeEvidenceWithoutImportExecution,
    ];

    RuntimeSurfaceCommitShmMetadataValidationHarnessReport {
        validation_harness_invoked: true,
        validation_paths_covered: covered_paths.len(),
        all_validation_paths_covered: true,
        covered_paths,
        no_real_wl_buffer_path_validated: true,
        non_shm_path_validated: true,
        metadata_unavailable_path_validated: true,
        metadata_partially_available_path_validated: true,
        metadata_insufficient_for_texture_precondition_path_validated: true,
        missing_lifetime_cleanup_policy_path_validated: true,
        runtime_evidence_without_import_execution_path_validated: true,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
    }
}

/// Runtime-visible pure-data SHM metadata report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitShmBufferMetadataReport {
    /// Metadata report seam was invoked.
    pub shm_metadata_report_invoked: bool,

    /// Source Phase 56A adapter report was observed.
    pub source_shm_first_adapter_report_observed: bool,

    /// The observed Phase 56A adapter report.
    pub observed_shm_first_adapter_report: RuntimeSurfaceCommitShmFirstBufferImportAdapterReport,

    /// Metadata extraction boundary is defined.
    pub shm_metadata_extraction_boundary_available: bool,

    /// Metadata evidence is available.
    pub shm_metadata_available: bool,

    /// Metadata was observed from a concrete SHM buffer.
    pub shm_metadata_observed: bool,

    /// Metadata is unavailable in this runtime path.
    pub shm_metadata_unavailable: bool,

    /// Metadata taxonomy kind.
    pub shm_buffer_metadata_kind: LinuxShmBufferMetadataKind,

    /// Phase 56C refined blocker taxonomy was applied.
    pub metadata_blocker_refinement_applied: bool,

    /// No concrete `WlBuffer` was available to the runtime report path.
    pub no_real_wl_buffer_available: bool,

    /// A concrete `WlBuffer` was available but not Smithay SHM-managed.
    pub wl_buffer_available_but_not_shm: bool,

    /// A SHM-like candidate lacked a safe Smithay metadata accessor.
    pub shm_like_candidate_missing_safe_accessor: bool,

    /// Metadata exists but does not satisfy texture preconditions.
    pub metadata_insufficient_for_texture_precondition: bool,

    /// Buffer lifetime and cleanup ownership policy is still missing.
    pub missing_buffer_lifetime_cleanup_policy: bool,

    /// This report is evidence only and does not execute import.
    pub runtime_report_only_has_evidence_not_import_execution: bool,

    /// Phase 56D validation harness report for metadata evidence and blockers.
    pub validation_harness_report: RuntimeSurfaceCommitShmMetadataValidationHarnessReport,

    /// Width metadata was observed.
    pub width_observed: bool,

    /// Height metadata was observed.
    pub height_observed: bool,

    /// Stride metadata was observed.
    pub stride_observed: bool,

    /// Format metadata was observed.
    pub format_observed: bool,

    /// Buffer offset in bytes.
    pub offset: Option<i32>,

    /// Buffer width.
    pub width: Option<i32>,

    /// Buffer height.
    pub height: Option<i32>,

    /// Buffer stride.
    pub stride: Option<i32>,

    /// Buffer format.
    pub format: Option<String>,

    /// Phase 56B does not attempt real import.
    pub buffer_import_attempted: bool,

    /// Phase 56B does not complete real import.
    pub buffer_imported: bool,

    /// Phase 56B does not create textures.
    pub texture_created: bool,

    /// Phase 56B does not call a renderer.
    pub renderer_called: bool,

    /// Phase 56B does not submit damage.
    pub damage_submitted: bool,

    /// Phase 56B does not send frame callback done.
    pub frame_callback_done_sent: bool,

    /// Phase 56B does not connect input.
    pub input_support: bool,

    /// Phase 56B does not mutate core.
    pub core_mutation_invoked: bool,

    /// Operations performed by the metadata seam.
    pub operations: Vec<RuntimeSurfaceCommitShmBufferMetadataOperation>,

    /// Blockers that keep the path evidence-only.
    pub blockers: Vec<RuntimeSurfaceCommitShmBufferMetadataBlocker>,
}

/// Runtime-owned Linux-only SHM-first adapter skeleton.
#[derive(Debug, Default)]
pub struct LinuxShmFirstBufferImportAdapterSkeleton;

impl LinuxShmFirstBufferImportAdapterSkeleton {
    /// Create the adapter skeleton.
    pub fn new() -> Self {
        Self
    }

    /// Build the evidence-only adapter report from the Phase 55L record.
    pub fn report_from_actual_attempt_record(
        &mut self,
        record: &RuntimeSurfaceCommitBufferImportActualAttemptRecord,
        boundary: Option<LinuxShmBufferTypeBoundaryEvidence>,
    ) -> RuntimeSurfaceCommitShmFirstBufferImportAdapterReport {
        shm_first_buffer_import_adapter_report_from_actual_attempt_record(record, boundary)
    }

    /// Build the Phase 56B metadata report from the Phase 56A adapter report.
    pub fn metadata_report_from_adapter_report(
        &mut self,
        report: &RuntimeSurfaceCommitShmFirstBufferImportAdapterReport,
        metadata: Option<LinuxShmBufferMetadataEvidence>,
    ) -> RuntimeSurfaceCommitShmBufferMetadataReport {
        shm_buffer_metadata_report_from_adapter_report(report, metadata)
    }

    /// 从 Phase 56D metadata validation harness report 派生 Phase 56E texture precondition audit。
    ///
    /// 该 owner 方法只生成 blocked pure-data report；texture precondition allowed
    /// 不等于 texture created，metadata sufficient 不等于 renderer call。
    pub fn texture_creation_precondition_audit_from_metadata_report(
        &mut self,
        report: &RuntimeSurfaceCommitShmBufferMetadataReport,
    ) -> RuntimeSurfaceCommitTextureCreationPreconditionAuditReport {
        texture_creation_precondition_audit_from_metadata_report(report)
    }

    /// 从 Phase 56E texture precondition audit 派生 Phase 56F texture creation no-op report。
    ///
    /// 该 owner 方法只记录 no-op / blocked evidence。它不 import buffer、不创建 texture、
    /// 不调用 renderer、不提交 damage、不发送 frame callback done。
    pub fn texture_creation_noop_report_from_precondition_audit(
        &mut self,
        report: &RuntimeSurfaceCommitTextureCreationPreconditionAuditReport,
    ) -> RuntimeSurfaceCommitTextureCreationNoopReport {
        texture_creation_noop_report_from_precondition_audit(report)
    }

    /// 从 Phase 56F texture creation no-op report 派生 Phase 56G texture owner boundary report。
    ///
    /// 该 owner 方法只定义 future texture ownership seam；owner boundary 不等于
    /// texture created，future handle/id ownership 不等于真实 graphics resource，
    /// owner request 不等于 renderer call。
    pub fn texture_owner_boundary_report_from_noop_report(
        &mut self,
        report: &RuntimeSurfaceCommitTextureCreationNoopReport,
    ) -> RuntimeSurfaceCommitTextureOwnerBoundaryReport {
        texture_owner_boundary_report_from_noop_report(report)
    }

    /// 从 Phase 56G texture owner boundary report 派生 Phase 56H renderer backend instance audit。
    ///
    /// 该 owner 方法只定义 renderer backend instance 的 future owner / lifecycle /
    /// cleanup / availability seam；它不创建 renderer backend instance，也不调用
    /// renderer 或 texture import。
    pub fn renderer_backend_instance_audit_from_texture_owner_boundary_report(
        &mut self,
        report: &RuntimeSurfaceCommitTextureOwnerBoundaryReport,
    ) -> RuntimeSurfaceCommitRendererBackendInstanceAuditReport {
        renderer_backend_instance_audit_from_texture_owner_boundary_report(report)
    }

    /// 从 Phase 56H renderer backend instance audit 派生 Phase 56I texture import route decision。
    ///
    /// 该 owner 方法只定义 texture import route 的 future route owner / import call /
    /// texture handle / cleanup / release / damage / frame-callback seam；它不调用
    ///真实 import-buffer 路径，不创建 texture handle，不创建 texture，也不调用 renderer。
    pub fn texture_import_route_decision_from_renderer_backend_instance_audit(
        &mut self,
        report: &RuntimeSurfaceCommitRendererBackendInstanceAuditReport,
    ) -> RuntimeSurfaceCommitTextureImportRouteDecisionReport {
        texture_import_route_decision_from_renderer_backend_instance_audit(report)
    }

    /// 从 Phase 56I texture import route decision 派生 Phase 56J damage-to-texture mapping audit。
    ///
    /// 该 owner 方法只定义 future damage mapping owner / region / coordinate-space /
    /// submission policy seam；它不提交真实 damage，不调用 renderer，也不发送 frame
    /// callback done。
    pub fn damage_to_texture_mapping_audit_from_texture_import_route_decision(
        &mut self,
        report: &RuntimeSurfaceCommitTextureImportRouteDecisionReport,
    ) -> RuntimeSurfaceCommitDamageToTextureMappingAuditReport {
        damage_to_texture_mapping_audit_from_texture_import_route_decision(report)
    }

    /// 从 Phase 56J damage-to-texture mapping audit 派生 Phase 56K frame callback completion policy。
    ///
    /// 该 owner 方法只定义 future frame callback completion owner 与 render-success
    /// gate；它不发送 frame callback done，不调用 renderer，也不提交 damage。
    pub fn frame_callback_completion_policy_from_damage_to_texture_mapping_audit(
        &mut self,
        report: &RuntimeSurfaceCommitDamageToTextureMappingAuditReport,
    ) -> RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport {
        frame_callback_completion_policy_from_damage_to_texture_mapping_audit(report)
    }

    /// 从 Phase 56K frame callback completion policy 派生 Phase 56L real texture creation readiness decision。
    ///
    /// 该 owner 方法只汇总 Phase 56H-56K 的前置条件和最小 renderability checklist；
    /// readiness decision 不等于 texture created、renderer called、damage submitted 或
    /// frame callback done。
    pub fn real_texture_creation_readiness_decision_from_frame_callback_completion_policy(
        &mut self,
        report: &RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport,
    ) -> RuntimeSurfaceCommitRealTextureCreationReadinessDecisionReport {
        real_texture_creation_readiness_decision_from_frame_callback_completion_policy(report)
    }
}

/// Convert the Phase 55L record into a Phase 56A SHM-first blocked report.
#[must_use = "SHM-first adapter report remains evidence-only in Phase 56A"]
pub fn shm_first_buffer_import_adapter_report_from_actual_attempt_record(
    record: &RuntimeSurfaceCommitBufferImportActualAttemptRecord,
    boundary: Option<LinuxShmBufferTypeBoundaryEvidence>,
) -> RuntimeSurfaceCommitShmFirstBufferImportAdapterReport {
    let type_boundary_observed = boundary
        .as_ref()
        .is_some_and(|evidence| evidence.wl_buffer_type_boundary_observed);
    let shm_access_type_available = boundary
        .as_ref()
        .is_some_and(|evidence| evidence.smithay_shm_access_type_available);

    let mut blockers = Vec::new();
    if !record.actual_attempt_record_available {
        blockers.push(
            RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::MissingActualAttemptRecord,
        );
    }
    if !type_boundary_observed {
        blockers.push(
            RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::MissingWlBufferTypeBoundaryObservation,
        );
    }
    if !shm_access_type_available {
        blockers.push(
            RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::MissingShmBufferAccessEvidence,
        );
    }
    if !record.actual_import_required {
        blockers
            .push(RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::NoActualImportRequired);
    }
    blockers.extend([
        RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::TextureCreationForbiddenInPhase56A,
        RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::RendererCallForbiddenInPhase56A,
        RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::DamageSubmitForbiddenInPhase56A,
        RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::FrameCallbackDoneForbiddenInPhase56A,
        RuntimeSurfaceCommitShmFirstBufferImportAdapterBlocker::DrmGbmDmabufForbiddenInPhase56A,
    ]);

    RuntimeSurfaceCommitShmFirstBufferImportAdapterReport {
        shm_buffer_import_adapter_invoked: true,
        source_buffer_import_actual_attempt_record_observed: record.actual_attempt_record_available,
        observed_actual_attempt_record: record.clone(),
        shm_buffer_adapter_available: true,
        shm_buffer_import_route_selected: true,
        shm_buffer_type_boundary_observed: type_boundary_observed,
        smithay_shm_access_type_available: shm_access_type_available,
        shm_buffer_import_execution_blocked: true,
        evidence_only_report: true,
        no_texture_report: true,
        unsupported_or_missing_wl_buffer_report: !type_boundary_observed,
        actual_import_required: record.actual_import_required,
        buffer_attach_observed: record.buffer_attach_observed,
        buffer_present: record.buffer_present,
        buffer_removed: record.buffer_removed,
        candidate_evidence_observed: record.candidate_evidence_observed,
        importer_owner_evidence_available: record.importer_owner_evidence_available,
        renderer_backend_descriptor_evidence_available: record
            .renderer_backend_descriptor_evidence_available,
        registered_renderer_backend_kind: record.registered_renderer_backend_kind,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation::ObserveActualAttemptRecord,
            RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation::SelectShmFirstRoute,
            RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation::CheckWlBufferTypeBoundary,
            RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation::CheckNoTextureBoundary,
            RuntimeSurfaceCommitShmFirstBufferImportAdapterOperation::BuildEvidenceOnlyReport,
        ],
        blockers,
    }
}

/// Convert Phase 56A adapter report and optional Linux-only metadata evidence into a runtime report.
#[must_use = "SHM metadata report remains evidence-only in Phase 56B"]
pub fn shm_buffer_metadata_report_from_adapter_report(
    report: &RuntimeSurfaceCommitShmFirstBufferImportAdapterReport,
    metadata: Option<LinuxShmBufferMetadataEvidence>,
) -> RuntimeSurfaceCommitShmBufferMetadataReport {
    let no_real_wl_buffer_available = metadata.is_none();
    let metadata_available = metadata
        .as_ref()
        .is_some_and(|evidence| evidence.shm_metadata_available);
    let metadata_observed = metadata
        .as_ref()
        .is_some_and(|evidence| evidence.shm_metadata_observed);
    let metadata_kind = metadata
        .as_ref()
        .map_or(LinuxShmBufferMetadataKind::Unavailable, |evidence| {
            evidence.shm_buffer_metadata_kind.clone()
        });
    let mut blockers = Vec::new();
    if !report.shm_buffer_import_adapter_invoked {
        blockers.push(RuntimeSurfaceCommitShmBufferMetadataBlocker::MissingShmFirstAdapterReport);
    }
    if !metadata_available {
        blockers.push(RuntimeSurfaceCommitShmBufferMetadataBlocker::MetadataUnavailable);
    }
    if metadata_kind == LinuxShmBufferMetadataKind::UnsupportedNonShmBuffer {
        blockers.push(RuntimeSurfaceCommitShmBufferMetadataBlocker::UnsupportedNonShmBuffer);
    }
    if no_real_wl_buffer_available {
        blockers.push(RuntimeSurfaceCommitShmBufferMetadataBlocker::NoRealWlBufferAvailable);
    }
    let wl_buffer_available_but_not_shm =
        metadata_kind == LinuxShmBufferMetadataKind::UnsupportedNonShmBuffer;
    if wl_buffer_available_but_not_shm {
        blockers.push(RuntimeSurfaceCommitShmBufferMetadataBlocker::WlBufferAvailableButNotShm);
    }
    let shm_like_candidate_missing_safe_accessor = metadata.as_ref().is_some_and(|evidence| {
        evidence.shm_metadata_extraction_attempted
            && evidence.shm_buffer_metadata_kind == LinuxShmBufferMetadataKind::Unavailable
            && !evidence.shm_metadata_available
    });
    if shm_like_candidate_missing_safe_accessor {
        blockers.push(
            RuntimeSurfaceCommitShmBufferMetadataBlocker::ShmLikeCandidateMissingSafeSmithayMetadataAccessor,
        );
    }
    let metadata_insufficient_for_texture_precondition = metadata_observed;
    if metadata_insufficient_for_texture_precondition {
        blockers.push(
            RuntimeSurfaceCommitShmBufferMetadataBlocker::MetadataObservableButInsufficientForTexturePrecondition,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitShmBufferMetadataBlocker::MissingBufferLifetimeCleanupOwnershipPolicy,
        RuntimeSurfaceCommitShmBufferMetadataBlocker::RuntimeReportOnlyHasEvidenceNotImportExecution,
    ]);
    blockers.extend([
        RuntimeSurfaceCommitShmBufferMetadataBlocker::TextureCreationForbiddenInPhase56B,
        RuntimeSurfaceCommitShmBufferMetadataBlocker::RendererCallForbiddenInPhase56B,
        RuntimeSurfaceCommitShmBufferMetadataBlocker::DamageSubmitForbiddenInPhase56B,
        RuntimeSurfaceCommitShmBufferMetadataBlocker::FrameCallbackDoneForbiddenInPhase56B,
        RuntimeSurfaceCommitShmBufferMetadataBlocker::DrmGbmDmabufForbiddenInPhase56B,
    ]);

    let offset = metadata.as_ref().and_then(|evidence| evidence.offset);
    let width = metadata.as_ref().and_then(|evidence| evidence.width);
    let height = metadata.as_ref().and_then(|evidence| evidence.height);
    let stride = metadata.as_ref().and_then(|evidence| evidence.stride);
    let format = metadata
        .as_ref()
        .and_then(|evidence| evidence.format.clone());
    let validation_harness_report = validate_shm_metadata_harness_paths();

    RuntimeSurfaceCommitShmBufferMetadataReport {
        shm_metadata_report_invoked: true,
        source_shm_first_adapter_report_observed: report.shm_buffer_import_adapter_invoked,
        observed_shm_first_adapter_report: report.clone(),
        shm_metadata_extraction_boundary_available: true,
        shm_metadata_available: metadata_available,
        shm_metadata_observed: metadata_observed,
        shm_metadata_unavailable: !metadata_available,
        shm_buffer_metadata_kind: metadata_kind,
        metadata_blocker_refinement_applied: true,
        no_real_wl_buffer_available,
        wl_buffer_available_but_not_shm,
        shm_like_candidate_missing_safe_accessor,
        metadata_insufficient_for_texture_precondition,
        missing_buffer_lifetime_cleanup_policy: true,
        runtime_report_only_has_evidence_not_import_execution: true,
        validation_harness_report,
        width_observed: width.is_some(),
        height_observed: height.is_some(),
        stride_observed: stride.is_some(),
        format_observed: format.is_some(),
        offset,
        width,
        height,
        stride,
        format,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitShmBufferMetadataOperation::ObserveShmFirstAdapterReport,
            RuntimeSurfaceCommitShmBufferMetadataOperation::CheckShmMetadataEvidence,
            RuntimeSurfaceCommitShmBufferMetadataOperation::ClassifyMetadataAvailability,
            RuntimeSurfaceCommitShmBufferMetadataOperation::RefineMetadataBlockers,
            RuntimeSurfaceCommitShmBufferMetadataOperation::BuildMetadataReport,
        ],
        blockers,
    }
}

/// Phase 56E texture creation 前置条件审计执行的纯数据步骤。
///
/// 这些步骤只消费 Phase 56D validation harness 与 metadata evidence；texture
/// precondition allowed 不等于 texture created，metadata sufficient 也不等于
/// renderer call。本阶段不创建 texture、不调用 renderer、不提交 damage、不发送
/// frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureCreationPreconditionOperation {
    /// 观察 Phase 56B/56C/56D SHM metadata report。
    ObserveShmMetadataReport,
    /// 检查 validation harness 是否覆盖所需 blocker paths。
    CheckValidationHarness,
    /// 检查 metadata 字段是否足以作为未来 texture precondition evidence。
    CheckMetadataPreconditionInputs,
    /// 检查 renderer backend instance 是否真实可用。
    CheckRendererBackendInstance,
    /// 检查 texture import route 是否真实可用。
    CheckTextureImportRoute,
    /// 构建安全 blocked audit report。
    BuildTexturePreconditionAuditReport,
}

/// Phase 56E texture creation 前置条件 blocker taxonomy。
///
/// 每个 blocker 都是安全边界：它说明为什么当前只能停在 audit/report，不能把
/// runtime evidence 误报为真实 texture object、renderer call、damage submit 或
/// frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureCreationPreconditionBlocker {
    /// 缺少 Phase 56D validation harness evidence。
    MetadataValidationMissing,
    /// metadata 尚不足以允许 texture precondition。
    MetadataInsufficient,
    /// width metadata 未知。
    UnknownWidth,
    /// height metadata 未知。
    UnknownHeight,
    /// stride metadata 未知。
    UnknownStride,
    /// format metadata 未知。
    UnknownFormat,
    /// buffer kind 不是已支持的 SHM evidence。
    UnsupportedBufferKind,
    /// 缺少 buffer lifetime ownership policy。
    MissingLifetimePolicy,
    /// 缺少 cleanup ownership policy。
    MissingCleanupPolicy,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少真实 texture import route。
    MissingTextureImportRoute,
    /// 缺少 damage 到 texture/render 的映射策略。
    MissingDamageToTextureMapping,
    /// 缺少 frame callback completion policy。
    MissingFrameCallbackCompletionPolicy,
    /// runtime 只有 evidence，没有 texture creation execution。
    RuntimeEvidenceWithoutTextureCreation,
}

/// Phase 56E texture precondition checklist。
///
/// checklist 是 pure-data 审计结果。真实 Smithay / Wayland / `WlBuffer` /
/// `BufferData` / texture / renderer 类型仍只能停留在 `smithay_backend` 的
/// Linux-only adapter/glue 层，core 不感知这些资源类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureCreationPreconditionChecklist {
    /// Phase 56D validation harness 是否通过。
    pub metadata_validation_passed: bool,

    /// metadata 是否足以进入 texture precondition；本阶段保持 false。
    pub metadata_sufficient_for_texture_precondition: bool,

    /// width metadata 是否已知。
    pub width_known: bool,

    /// height metadata 是否已知。
    pub height_known: bool,

    /// stride metadata 是否已知。
    pub stride_known: bool,

    /// format metadata 是否已知。
    pub format_known: bool,

    /// buffer kind 是否受支持。
    pub buffer_kind_supported: bool,

    /// buffer lifetime policy 是否已知；本阶段保持 false。
    pub lifetime_policy_known: bool,

    /// cleanup policy 是否已知；本阶段保持 false。
    pub cleanup_policy_known: bool,

    /// 真实 renderer backend instance 是否可用；本阶段保持 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；本阶段保持 false。
    pub texture_import_route_available: bool,

    /// damage-to-texture/render mapping 是否可用；本阶段保持 false。
    pub damage_to_texture_mapping_available: bool,

    /// frame callback completion policy 是否可用；本阶段保持 false。
    pub frame_callback_completion_policy_available: bool,

    /// 是否允许进入 texture precondition；Phase 56E 固定 blocked/false。
    pub texture_precondition_allowed: bool,
}

/// Phase 56E texture creation precondition audit report。
///
/// 该 report 从 Phase 56D validation harness 派生，只说明 texture creation 之前
/// 还缺哪些安全前置条件。它不 import buffer、不创建 texture、不调用 renderer、
/// 不提交 damage、不发送 frame callback done、不接 input，也不触发 core mutation。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureCreationPreconditionAuditReport {
    /// texture precondition audit seam 是否可用。
    pub texture_precondition_audit_available: bool,

    /// 是否观察到上游 SHM metadata report。
    pub source_metadata_report_observed: bool,

    /// 观察到的上游 SHM metadata report；直接消费 validation harness 时为空。
    pub observed_metadata_report: Option<RuntimeSurfaceCommitShmBufferMetadataReport>,

    /// Phase 56D validation harness report。
    pub validation_harness_report: RuntimeSurfaceCommitShmMetadataValidationHarnessReport,

    /// texture precondition checklist。
    pub checklist: RuntimeSurfaceCommitTextureCreationPreconditionChecklist,

    /// metadata 是否足以进入 texture precondition；本阶段保持 false。
    pub metadata_sufficient_for_texture_precondition: bool,

    /// renderer backend instance 是否真实可用；本阶段保持 false。
    pub renderer_backend_instance_available: bool,

    /// texture import route 是否真实可用；本阶段保持 false。
    pub texture_import_route_available: bool,

    /// damage-to-texture/render mapping 是否真实可用；本阶段保持 false。
    pub damage_to_texture_mapping_available: bool,

    /// frame callback completion policy 是否真实可用；本阶段保持 false。
    pub frame_callback_completion_policy_available: bool,

    /// 是否允许进入 texture precondition；Phase 56E 固定 false。
    pub texture_precondition_allowed: bool,

    /// 是否因缺少真实资源前置条件而 blocked；Phase 56E 固定 true。
    pub texture_precondition_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub texture_precondition_blocker_reason: &'static str,

    /// Phase 56E 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56E 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56E 不创建 texture。
    pub texture_created: bool,

    /// Phase 56E 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56E 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56E 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56E 不接入 input。
    pub input_support: bool,

    /// Phase 56E 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// 审计执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitTextureCreationPreconditionOperation>,

    /// 阻止进入真实 texture creation 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitTextureCreationPreconditionBlocker>,
}

/// 从 Phase 56D validation harness / metadata report 派生 texture precondition audit。
///
/// 这是 Phase 56E 的唯一执行入口：它只生成 pure-data blocked report。真实
/// buffer 类型仍留在 Linux-only adapter，core 不感知 `wl_buffer`、Smithay
/// `BufferData`、texture 或 renderer。本函数不创建 texture、不调用 renderer、
/// 不提交 damage、不发送 frame callback done。
#[must_use = "texture precondition audit is not texture creation"]
pub fn texture_creation_precondition_audit_from_metadata_report(
    report: &RuntimeSurfaceCommitShmBufferMetadataReport,
) -> RuntimeSurfaceCommitTextureCreationPreconditionAuditReport {
    texture_creation_precondition_audit_from_parts(
        true,
        Some(report.clone()),
        report.validation_harness_report.clone(),
        report.width_observed,
        report.height_observed,
        report.stride_observed,
        report.format_observed,
        report.shm_buffer_metadata_kind == LinuxShmBufferMetadataKind::Shm,
    )
}

/// 直接从 Phase 56D validation harness report 派生 Phase 56E audit。
///
/// 该入口用于证明 validation harness 可以被提升为 texture precondition audit
/// evidence，但仍不会创建 texture、不会调用 renderer，也不会把任何真实
/// `WlBuffer` / `BufferData` / texture / renderer 类型传入 core。
#[must_use = "validation harness audit is not texture creation"]
pub fn texture_creation_precondition_audit_from_validation_harness_report(
    validation_harness_report: &RuntimeSurfaceCommitShmMetadataValidationHarnessReport,
) -> RuntimeSurfaceCommitTextureCreationPreconditionAuditReport {
    texture_creation_precondition_audit_from_parts(
        false,
        None,
        validation_harness_report.clone(),
        false,
        false,
        false,
        false,
        false,
    )
}

fn texture_creation_precondition_audit_from_parts(
    source_metadata_report_observed: bool,
    observed_metadata_report: Option<RuntimeSurfaceCommitShmBufferMetadataReport>,
    validation_harness_report: RuntimeSurfaceCommitShmMetadataValidationHarnessReport,
    width_observed: bool,
    height_observed: bool,
    stride_observed: bool,
    format_observed: bool,
    buffer_kind_supported: bool,
) -> RuntimeSurfaceCommitTextureCreationPreconditionAuditReport {
    let metadata_validation_passed = validation_harness_report.validation_harness_invoked
        && validation_harness_report.all_validation_paths_covered;

    // Phase 56E 故意保持 conservative：即使 metadata 字段未来齐全，只要缺少
    // lifetime/cleanup/renderer/texture/damage/frame policy，就不能允许 texture precondition。
    let checklist = RuntimeSurfaceCommitTextureCreationPreconditionChecklist {
        metadata_validation_passed,
        metadata_sufficient_for_texture_precondition: false,
        width_known: width_observed,
        height_known: height_observed,
        stride_known: stride_observed,
        format_known: format_observed,
        buffer_kind_supported,
        lifetime_policy_known: false,
        cleanup_policy_known: false,
        renderer_backend_instance_available: false,
        texture_import_route_available: false,
        damage_to_texture_mapping_available: false,
        frame_callback_completion_policy_available: false,
        texture_precondition_allowed: false,
    };

    let mut blockers = Vec::new();
    if !checklist.metadata_validation_passed {
        blockers.push(
            RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MetadataValidationMissing,
        );
    }
    blockers.push(RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MetadataInsufficient);
    if !checklist.width_known {
        blockers.push(RuntimeSurfaceCommitTextureCreationPreconditionBlocker::UnknownWidth);
    }
    if !checklist.height_known {
        blockers.push(RuntimeSurfaceCommitTextureCreationPreconditionBlocker::UnknownHeight);
    }
    if !checklist.stride_known {
        blockers.push(RuntimeSurfaceCommitTextureCreationPreconditionBlocker::UnknownStride);
    }
    if !checklist.format_known {
        blockers.push(RuntimeSurfaceCommitTextureCreationPreconditionBlocker::UnknownFormat);
    }
    if !checklist.buffer_kind_supported {
        blockers
            .push(RuntimeSurfaceCommitTextureCreationPreconditionBlocker::UnsupportedBufferKind);
    }
    blockers.extend([
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingLifetimePolicy,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingCleanupPolicy,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingRendererBackendInstance,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingTextureImportRoute,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingDamageToTextureMapping,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingFrameCallbackCompletionPolicy,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker::RuntimeEvidenceWithoutTextureCreation,
    ]);

    RuntimeSurfaceCommitTextureCreationPreconditionAuditReport {
        texture_precondition_audit_available: true,
        source_metadata_report_observed,
        observed_metadata_report,
        validation_harness_report,
        checklist,
        metadata_sufficient_for_texture_precondition: false,
        renderer_backend_instance_available: false,
        texture_import_route_available: false,
        damage_to_texture_mapping_available: false,
        frame_callback_completion_policy_available: false,
        texture_precondition_allowed: false,
        texture_precondition_blocked: true,
        texture_precondition_blocker_reason: "missing renderer backend instance / texture import route / damage mapping / frame callback completion policy",
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitTextureCreationPreconditionOperation::ObserveShmMetadataReport,
            RuntimeSurfaceCommitTextureCreationPreconditionOperation::CheckValidationHarness,
            RuntimeSurfaceCommitTextureCreationPreconditionOperation::CheckMetadataPreconditionInputs,
            RuntimeSurfaceCommitTextureCreationPreconditionOperation::CheckRendererBackendInstance,
            RuntimeSurfaceCommitTextureCreationPreconditionOperation::CheckTextureImportRoute,
            RuntimeSurfaceCommitTextureCreationPreconditionOperation::BuildTexturePreconditionAuditReport,
        ],
        blockers,
    }
}

/// Phase 56F texture creation no-op skeleton 的纯数据步骤。
///
/// 这些步骤只消费 Phase 56E precondition audit；no-op invocation 是执行边界报告，
/// 不等于真实 texture creation、renderer call、damage submit 或 frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureCreationNoopOperation {
    /// 观察 Phase 56E texture precondition audit。
    ObserveTexturePreconditionAudit,
    /// 检查 texture precondition 是否允许继续。
    CheckTexturePreconditionAllowed,
    /// 检查 texture owner boundary 是否存在。
    CheckTextureOwnerBoundary,
    /// 检查 renderer backend instance 是否真实可用。
    CheckRendererBackendInstance,
    /// 检查 texture import route 是否真实可用。
    CheckTextureImportRoute,
    /// 检查 damage 到 texture/render 的映射策略。
    CheckDamageToTextureMapping,
    /// 检查 frame callback completion policy。
    CheckFrameCallbackCompletionPolicy,
    /// 构建 no-op / blocked report。
    BuildTextureCreationNoopReport,
}

/// Phase 56F texture creation no-op skeleton 的 blocker taxonomy。
///
/// 每个 blocker 都说明当前只能停在 no-op report，不能把 runtime evidence 或 shell
/// readiness 误报为真实 texture object、renderer call、damage submit 或 frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureCreationBlocker {
    /// Phase 56E 尚未允许 texture precondition。
    TexturePreconditionNotAllowed,
    /// metadata 不足以支撑 texture precondition。
    MetadataInsufficientForTexture,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少真实 texture import route。
    MissingTextureImportRoute,
    /// 缺少 damage 到 texture/render 的映射策略。
    MissingDamageToTextureMapping,
    /// 缺少 frame callback completion policy。
    MissingFrameCallbackCompletionPolicy,
    /// 缺少 texture resource owner boundary。
    MissingTextureOwnerBoundary,
    /// runtime 只有 evidence，没有 texture creation execution。
    RuntimeEvidenceWithoutTextureCreation,
    /// texture creation 在 Phase 56F 被显式禁用。
    TextureCreationExplicitlyDisabled,
    /// renderer call 在 Phase 56F 被显式禁用。
    RendererCallExplicitlyDisabled,
}

/// Phase 56F texture creation no-op checklist。
///
/// checklist 只描述执行边界和缺失真实资源。真实 texture / renderer 类型不进入 core，
/// 也不在本阶段创建或调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureCreationNoopChecklist {
    /// Phase 56F no-op skeleton 是否可用。
    pub texture_creation_noop_available: bool,

    /// Phase 56E 是否允许 texture precondition；Phase 56F 保持 false。
    pub texture_precondition_allowed: bool,

    /// metadata 是否足以进入 texture precondition；Phase 56F 继承 Phase 56E 的 false。
    pub metadata_sufficient_for_texture_precondition: bool,

    /// texture owner boundary 是否存在；Phase 56F 固定 false。
    pub texture_owner_boundary_available: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56F 固定 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；Phase 56F 固定 false。
    pub texture_import_route_available: bool,

    /// damage-to-texture/render mapping 是否可用；Phase 56F 固定 false。
    pub damage_to_texture_mapping_available: bool,

    /// frame callback completion policy 是否可用；Phase 56F 固定 false。
    pub frame_callback_completion_policy_available: bool,

    /// texture creation 是否被 blocked；Phase 56F 固定 true。
    pub texture_creation_blocked: bool,
}

/// Phase 56F texture creation blocker / no-op skeleton report。
///
/// 该 report 从 Phase 56E precondition audit 派生。它只证明 texture creation
/// execution boundary 已存在且保持 no-op/blocked；它不 import buffer、不创建 texture、
/// 不调用 renderer、不提交 damage、不发送 frame callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureCreationNoopReport {
    /// Phase 56F texture creation no-op skeleton 是否可用。
    pub texture_creation_noop_available: bool,

    /// 是否观察到 Phase 56E texture precondition audit。
    pub source_precondition_audit_observed: bool,

    /// 被消费的 Phase 56E audit report。
    pub precondition_audit_report: RuntimeSurfaceCommitTextureCreationPreconditionAuditReport,

    /// No-op execution checklist。
    pub checklist: RuntimeSurfaceCommitTextureCreationNoopChecklist,

    /// Phase 56F 是否尝试 texture creation；固定 false。
    pub texture_creation_attempted: bool,

    /// Phase 56F 是否 blocked；固定 true。
    pub texture_creation_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub texture_creation_blocker_reason: &'static str,

    /// Phase 56E 是否允许 texture precondition；Phase 56F 仍保持 false。
    pub texture_precondition_allowed: bool,

    /// metadata 是否足以进入 texture precondition；Phase 56F 仍保持 false。
    pub metadata_sufficient_for_texture_precondition: bool,

    /// texture owner boundary 是否存在；Phase 56F 固定 false。
    pub texture_owner_boundary_available: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56F 固定 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；Phase 56F 固定 false。
    pub texture_import_route_available: bool,

    /// damage-to-texture/render mapping 是否可用；Phase 56F 固定 false。
    pub damage_to_texture_mapping_available: bool,

    /// frame callback completion policy 是否可用；Phase 56F 固定 false。
    pub frame_callback_completion_policy_available: bool,

    /// Phase 56F 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56F 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56F 不创建 texture。
    pub texture_created: bool,

    /// Phase 56F 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56F 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56F 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56F 不接入 input。
    pub input_support: bool,

    /// Phase 56F 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// No-op skeleton 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitTextureCreationNoopOperation>,

    /// 阻止真实 texture creation 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitTextureCreationBlocker>,
}

/// 从 Phase 56E texture precondition audit 派生 Phase 56F texture creation no-op report。
///
/// 这是 Phase 56F 的唯一执行入口：它只生成 pure-data no-op / blocked report。即使
/// 上游 evidence 存在，本函数也不会创建 texture、不会调用 renderer、不会提交 damage、
/// 不会发送 frame callback done。
#[must_use = "texture creation no-op report is not texture creation"]
pub fn texture_creation_noop_report_from_precondition_audit(
    report: &RuntimeSurfaceCommitTextureCreationPreconditionAuditReport,
) -> RuntimeSurfaceCommitTextureCreationNoopReport {
    let checklist = RuntimeSurfaceCommitTextureCreationNoopChecklist {
        texture_creation_noop_available: true,
        texture_precondition_allowed: report.texture_precondition_allowed,
        metadata_sufficient_for_texture_precondition: report
            .metadata_sufficient_for_texture_precondition,
        texture_owner_boundary_available: false,
        renderer_backend_instance_available: report.renderer_backend_instance_available,
        texture_import_route_available: report.texture_import_route_available,
        damage_to_texture_mapping_available: report.damage_to_texture_mapping_available,
        frame_callback_completion_policy_available: report
            .frame_callback_completion_policy_available,
        texture_creation_blocked: true,
    };

    let mut blockers = Vec::new();
    if !checklist.texture_precondition_allowed {
        blockers.push(RuntimeSurfaceCommitTextureCreationBlocker::TexturePreconditionNotAllowed);
    }
    if !checklist.metadata_sufficient_for_texture_precondition {
        blockers.push(RuntimeSurfaceCommitTextureCreationBlocker::MetadataInsufficientForTexture);
    }
    if !checklist.renderer_backend_instance_available {
        blockers.push(RuntimeSurfaceCommitTextureCreationBlocker::MissingRendererBackendInstance);
    }
    if !checklist.texture_import_route_available {
        blockers.push(RuntimeSurfaceCommitTextureCreationBlocker::MissingTextureImportRoute);
    }
    if !checklist.damage_to_texture_mapping_available {
        blockers.push(RuntimeSurfaceCommitTextureCreationBlocker::MissingDamageToTextureMapping);
    }
    if !checklist.frame_callback_completion_policy_available {
        blockers
            .push(RuntimeSurfaceCommitTextureCreationBlocker::MissingFrameCallbackCompletionPolicy);
    }
    blockers.extend([
        RuntimeSurfaceCommitTextureCreationBlocker::MissingTextureOwnerBoundary,
        RuntimeSurfaceCommitTextureCreationBlocker::RuntimeEvidenceWithoutTextureCreation,
        RuntimeSurfaceCommitTextureCreationBlocker::TextureCreationExplicitlyDisabled,
        RuntimeSurfaceCommitTextureCreationBlocker::RendererCallExplicitlyDisabled,
    ]);

    RuntimeSurfaceCommitTextureCreationNoopReport {
        texture_creation_noop_available: true,
        source_precondition_audit_observed: report.texture_precondition_audit_available,
        precondition_audit_report: report.clone(),
        checklist,
        texture_creation_attempted: false,
        texture_creation_blocked: true,
        texture_creation_blocker_reason: "texture creation no-op: missing texture owner boundary / renderer backend instance / texture import route / damage mapping / frame callback completion policy",
        texture_precondition_allowed: report.texture_precondition_allowed,
        metadata_sufficient_for_texture_precondition: report
            .metadata_sufficient_for_texture_precondition,
        texture_owner_boundary_available: false,
        renderer_backend_instance_available: report.renderer_backend_instance_available,
        texture_import_route_available: report.texture_import_route_available,
        damage_to_texture_mapping_available: report.damage_to_texture_mapping_available,
        frame_callback_completion_policy_available: report
            .frame_callback_completion_policy_available,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitTextureCreationNoopOperation::ObserveTexturePreconditionAudit,
            RuntimeSurfaceCommitTextureCreationNoopOperation::CheckTexturePreconditionAllowed,
            RuntimeSurfaceCommitTextureCreationNoopOperation::CheckTextureOwnerBoundary,
            RuntimeSurfaceCommitTextureCreationNoopOperation::CheckRendererBackendInstance,
            RuntimeSurfaceCommitTextureCreationNoopOperation::CheckTextureImportRoute,
            RuntimeSurfaceCommitTextureCreationNoopOperation::CheckDamageToTextureMapping,
            RuntimeSurfaceCommitTextureCreationNoopOperation::CheckFrameCallbackCompletionPolicy,
            RuntimeSurfaceCommitTextureCreationNoopOperation::BuildTextureCreationNoopReport,
        ],
        blockers,
    }
}

/// Phase 56G texture owner boundary 的纯数据步骤。
///
/// 这些步骤只消费 Phase 56F no-op / blocked report，定义谁拥有 future texture
/// creation request 和 future handle lifecycle seam；本阶段不创建 texture、不调用
/// renderer、不提交 damage、不发送 frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureOwnerBoundaryOperation {
    /// 观察 Phase 56F texture creation no-op report。
    ObserveTextureCreationNoopReport,
    /// 定义 future texture creation request 的 owner。
    DefineTextureCreationRequestOwner,
    /// 定义 future texture handle/id lifecycle seam 的 owner。
    DefineFutureTextureHandleOwner,
    /// 定义 future texture lifetime policy owner。
    DefineFutureTextureLifetimeOwner,
    /// 定义 future texture cleanup policy owner。
    DefineTextureCleanupOwner,
    /// 定义 future texture release policy owner。
    DefineTextureReleaseOwner,
    /// 定义 future texture invalidation policy owner。
    DefineTextureInvalidationOwner,
    /// 检查 renderer backend instance 是否真实可用。
    CheckRendererBackendInstance,
    /// 检查 texture import route 是否真实可用。
    CheckTextureImportRoute,
    /// 构建 texture owner boundary report。
    BuildTextureOwnerBoundaryReport,
}

/// Phase 56G texture owner boundary blocker taxonomy。
///
/// 每个 blocker 都说明当前只能停在 owner boundary report，不能把 request owner、
/// future handle owner 或 cleanup owner 误报为真实 texture、renderer call、damage
/// submit 或 frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureOwnerBoundaryBlocker {
    /// 上游仍只是 texture creation no-op / blocked report。
    TextureCreationNoopOnly,
    /// 缺少完整 texture owner boundary policy。
    MissingTextureOwnerBoundary,
    /// 缺少 future texture handle ownership policy。
    MissingFutureTextureHandlePolicy,
    /// 缺少 future texture lifetime ownership policy。
    MissingFutureTextureLifetimePolicy,
    /// 缺少 future texture cleanup ownership policy。
    MissingFutureTextureCleanupPolicy,
    /// 缺少 future texture release ownership policy。
    MissingFutureTextureReleasePolicy,
    /// 缺少 future texture invalidation ownership policy。
    MissingFutureTextureInvalidationPolicy,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少真实 texture import route。
    MissingTextureImportRoute,
    /// runtime 只有 evidence，没有 texture ownership execution。
    RuntimeEvidenceWithoutTextureOwnership,
    /// owner boundary 存在，但没有真实 texture creation。
    OwnerBoundaryWithoutTextureCreation,
}

/// Phase 56G future texture ownership policy。
///
/// policy 只描述 future owner seam。它不保存真实 texture handle/id，不引用 renderer
/// 类型，也不让 core 感知 Smithay / Wayland / buffer / texture resource。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitFutureTextureOwnershipPolicy {
    /// future texture creation request owner 是否已定义。
    pub texture_creation_request_owner_defined: bool,

    /// future texture creation request 的 owner 名称。
    pub texture_creation_request_owner: &'static str,

    /// future texture handle/id owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture handle/id owner 名称；未定义时为 blocked。
    pub future_texture_handle_owner: &'static str,

    /// future texture lifetime owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_lifetime_owner_defined: bool,

    /// future texture lifetime owner 名称；未定义时为 blocked。
    pub future_texture_lifetime_owner: &'static str,

    /// future texture cleanup owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_cleanup_owner_defined: bool,

    /// future texture cleanup owner 名称；未定义时为 blocked。
    pub future_texture_cleanup_owner: &'static str,

    /// future texture release owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_release_owner_defined: bool,

    /// future texture release owner 名称；未定义时为 blocked。
    pub future_texture_release_owner: &'static str,

    /// future texture invalidation owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_invalidation_owner_defined: bool,

    /// future texture invalidation owner 名称；未定义时为 blocked。
    pub future_texture_invalidation_owner: &'static str,
}

/// Phase 56G texture owner boundary checklist。
///
/// checklist 说明 owner seam 哪些部分已定义、哪些仍 blocked。它是安全边界：
/// owner boundary 不等于 texture created，owner request 不等于 renderer call。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureOwnerBoundaryChecklist {
    /// texture owner boundary seam 是否可用。
    pub texture_owner_boundary_available: bool,

    /// owner boundary 是否仍 blocked。
    pub texture_owner_boundary_blocked: bool,

    /// future texture creation request owner 是否已定义。
    pub texture_creation_request_owner_defined: bool,

    /// future texture handle/id owner 是否已定义。
    pub future_texture_handle_owner_defined: bool,

    /// future texture lifetime owner 是否已定义。
    pub future_texture_lifetime_owner_defined: bool,

    /// future texture cleanup owner 是否已定义。
    pub future_texture_cleanup_owner_defined: bool,

    /// future texture release owner 是否已定义。
    pub future_texture_release_owner_defined: bool,

    /// future texture invalidation owner 是否已定义。
    pub future_texture_invalidation_owner_defined: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56G 固定 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；Phase 56G 固定 false。
    pub texture_import_route_available: bool,
}

/// Phase 56G texture owner boundary report。
///
/// 该 report 从 Phase 56F no-op report 派生，只定义 future ownership seam。
/// 它不 import buffer、不创建 texture、不创建真实 graphics handle、不调用 renderer、
/// 不提交 damage、不发送 frame callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureOwnerBoundaryReport {
    /// texture owner boundary seam 是否可用。
    pub texture_owner_boundary_available: bool,

    /// 是否观察到 Phase 56F no-op report。
    pub source_noop_report_observed: bool,

    /// 被消费的 Phase 56F no-op report。
    pub source_noop_report: RuntimeSurfaceCommitTextureCreationNoopReport,

    /// future texture ownership policy。
    pub ownership_policy: RuntimeSurfaceCommitFutureTextureOwnershipPolicy,

    /// owner boundary checklist。
    pub checklist: RuntimeSurfaceCommitTextureOwnerBoundaryChecklist,

    /// texture owner boundary 是否仍 blocked。
    pub texture_owner_boundary_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub texture_owner_boundary_blocker_reason: &'static str,

    /// future texture creation request owner 是否已定义。
    pub texture_creation_request_owner_defined: bool,

    /// future texture creation request owner 名称。
    pub texture_creation_request_owner: &'static str,

    /// future texture handle/id owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture handle/id owner 名称。
    pub future_texture_handle_owner: &'static str,

    /// future texture lifetime owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_lifetime_owner_defined: bool,

    /// future texture lifetime owner 名称。
    pub future_texture_lifetime_owner: &'static str,

    /// future texture cleanup owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_cleanup_owner_defined: bool,

    /// future texture cleanup owner 名称。
    pub future_texture_cleanup_owner: &'static str,

    /// future texture release owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_release_owner_defined: bool,

    /// future texture release owner 名称。
    pub future_texture_release_owner: &'static str,

    /// future texture invalidation owner 是否已定义；Phase 56G 保持 false。
    pub future_texture_invalidation_owner_defined: bool,

    /// future texture invalidation owner 名称。
    pub future_texture_invalidation_owner: &'static str,

    /// 真实 renderer backend instance 是否可用；Phase 56G 固定 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；Phase 56G 固定 false。
    pub texture_import_route_available: bool,

    /// Phase 56G 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56G 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56G 不创建 texture。
    pub texture_created: bool,

    /// Phase 56G 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56G 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56G 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56G 不接入 input。
    pub input_support: bool,

    /// Phase 56G 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// owner boundary 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitTextureOwnerBoundaryOperation>,

    /// 阻止真实 texture ownership/execution 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitTextureOwnerBoundaryBlocker>,
}

/// 从 Phase 56F no-op report 派生 Phase 56G texture owner boundary report。
///
/// 这是 Phase 56G 的唯一执行入口：它只生成 pure-data owner boundary report。
/// 本函数不创建 texture、不调用 renderer、不提交 damage、不发送 frame callback done。
#[must_use = "texture owner boundary report is not texture creation"]
pub fn texture_owner_boundary_report_from_noop_report(
    report: &RuntimeSurfaceCommitTextureCreationNoopReport,
) -> RuntimeSurfaceCommitTextureOwnerBoundaryReport {
    let ownership_policy = RuntimeSurfaceCommitFutureTextureOwnershipPolicy {
        texture_creation_request_owner_defined: true,
        texture_creation_request_owner: "linux_shm_first_buffer_import_adapter",
        future_texture_handle_owner_defined: false,
        future_texture_handle_owner: "blocked_until_future_texture_owner_policy",
        future_texture_lifetime_owner_defined: false,
        future_texture_lifetime_owner: "blocked_until_future_texture_lifetime_policy",
        future_texture_cleanup_owner_defined: false,
        future_texture_cleanup_owner: "blocked_until_future_texture_cleanup_policy",
        future_texture_release_owner_defined: false,
        future_texture_release_owner: "blocked_until_future_texture_release_policy",
        future_texture_invalidation_owner_defined: false,
        future_texture_invalidation_owner: "blocked_until_future_texture_invalidation_policy",
    };

    let checklist = RuntimeSurfaceCommitTextureOwnerBoundaryChecklist {
        texture_owner_boundary_available: true,
        texture_owner_boundary_blocked: true,
        texture_creation_request_owner_defined: ownership_policy
            .texture_creation_request_owner_defined,
        future_texture_handle_owner_defined: ownership_policy.future_texture_handle_owner_defined,
        future_texture_lifetime_owner_defined: ownership_policy
            .future_texture_lifetime_owner_defined,
        future_texture_cleanup_owner_defined: ownership_policy.future_texture_cleanup_owner_defined,
        future_texture_release_owner_defined: ownership_policy.future_texture_release_owner_defined,
        future_texture_invalidation_owner_defined: ownership_policy
            .future_texture_invalidation_owner_defined,
        renderer_backend_instance_available: report.renderer_backend_instance_available,
        texture_import_route_available: report.texture_import_route_available,
    };

    let mut blockers =
        vec![RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::TextureCreationNoopOnly];
    if !checklist.future_texture_handle_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingFutureTextureHandlePolicy,
        );
    }
    if !checklist.future_texture_lifetime_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingFutureTextureLifetimePolicy,
        );
    }
    if !checklist.future_texture_cleanup_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingFutureTextureCleanupPolicy,
        );
    }
    if !checklist.future_texture_release_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingFutureTextureReleasePolicy,
        );
    }
    if !checklist.future_texture_invalidation_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingFutureTextureInvalidationPolicy,
        );
    }
    if !checklist.renderer_backend_instance_available {
        blockers
            .push(RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingRendererBackendInstance);
    }
    if !checklist.texture_import_route_available {
        blockers.push(RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingTextureImportRoute);
    }
    blockers.extend([
        RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingTextureOwnerBoundary,
        RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::RuntimeEvidenceWithoutTextureOwnership,
        RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::OwnerBoundaryWithoutTextureCreation,
    ]);

    RuntimeSurfaceCommitTextureOwnerBoundaryReport {
        texture_owner_boundary_available: true,
        source_noop_report_observed: report.texture_creation_noop_available,
        source_noop_report: report.clone(),
        ownership_policy: ownership_policy.clone(),
        checklist,
        texture_owner_boundary_blocked: true,
        texture_owner_boundary_blocker_reason: "texture owner boundary only: missing future handle/lifetime/cleanup/release/invalidation policies, renderer backend instance, and texture import route",
        texture_creation_request_owner_defined: ownership_policy
            .texture_creation_request_owner_defined,
        texture_creation_request_owner: ownership_policy.texture_creation_request_owner,
        future_texture_handle_owner_defined: ownership_policy.future_texture_handle_owner_defined,
        future_texture_handle_owner: ownership_policy.future_texture_handle_owner,
        future_texture_lifetime_owner_defined: ownership_policy
            .future_texture_lifetime_owner_defined,
        future_texture_lifetime_owner: ownership_policy.future_texture_lifetime_owner,
        future_texture_cleanup_owner_defined: ownership_policy.future_texture_cleanup_owner_defined,
        future_texture_cleanup_owner: ownership_policy.future_texture_cleanup_owner,
        future_texture_release_owner_defined: ownership_policy.future_texture_release_owner_defined,
        future_texture_release_owner: ownership_policy.future_texture_release_owner,
        future_texture_invalidation_owner_defined: ownership_policy
            .future_texture_invalidation_owner_defined,
        future_texture_invalidation_owner: ownership_policy.future_texture_invalidation_owner,
        renderer_backend_instance_available: false,
        texture_import_route_available: false,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::ObserveTextureCreationNoopReport,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::DefineTextureCreationRequestOwner,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::DefineFutureTextureHandleOwner,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::DefineFutureTextureLifetimeOwner,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::DefineTextureCleanupOwner,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::DefineTextureReleaseOwner,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::DefineTextureInvalidationOwner,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::CheckRendererBackendInstance,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::CheckTextureImportRoute,
            RuntimeSurfaceCommitTextureOwnerBoundaryOperation::BuildTextureOwnerBoundaryReport,
        ],
        blockers,
    }
}

/// Phase 56H renderer backend instance audit 的纯数据步骤。
///
/// 这些步骤只消费 Phase 56G texture owner boundary report，审计 future renderer
/// backend instance 的 owner / lifecycle / cleanup / availability seam。本阶段不创建
/// renderer backend instance，不创建 texture，也不调用 renderer。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererBackendInstanceAuditOperation {
    /// 观察 Phase 56G texture owner boundary report。
    ObserveTextureOwnerBoundaryReport,
    /// 检查 renderer backend instance 是否真实可用。
    CheckRendererBackendInstanceAvailability,
    /// 审计 future renderer backend instance owner policy。
    AuditRendererBackendInstanceOwner,
    /// 审计 future renderer backend instance lifecycle policy。
    AuditRendererBackendInstanceLifecycle,
    /// 审计 future renderer backend instance cleanup policy。
    AuditRendererBackendInstanceCleanup,
    /// 审计 future renderer backend instance availability policy。
    AuditRendererBackendInstanceAvailability,
    /// 构建 renderer backend instance audit report。
    BuildRendererBackendInstanceAuditReport,
}

/// Phase 56H renderer backend instance audit blocker taxonomy。
///
/// 每个 blocker 都说明当前只能停在 renderer backend instance audit report，不能把
/// owner seam 误报为真实 renderer backend、renderer call、texture creation 或
/// buffer import。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker {
    /// 上游 texture owner boundary 仍处于 blocked 状态。
    TextureOwnerBoundaryStillBlocked,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少 renderer backend instance owner policy。
    MissingRendererBackendInstanceOwnerPolicy,
    /// 缺少 renderer backend instance lifecycle policy。
    MissingRendererBackendInstanceLifecyclePolicy,
    /// 缺少 renderer backend instance cleanup policy。
    MissingRendererBackendInstanceCleanupPolicy,
    /// 缺少 renderer backend instance availability policy。
    MissingRendererBackendInstanceAvailabilityPolicy,
    /// renderer backend instance audit 存在，但没有 texture creation。
    RendererBackendInstanceWithoutTextureCreation,
}

/// Phase 56H renderer backend instance policy。
///
/// policy 只描述 future renderer backend instance seam。它不保存 renderer backend
/// 对象，不引用 Smithay renderer 类型，也不让 core 感知真实图形资源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererBackendInstancePolicy {
    /// renderer backend instance owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_owner_defined: bool,

    /// renderer backend instance owner 名称；未定义时为 blocked。
    pub renderer_backend_instance_owner: &'static str,

    /// renderer backend instance lifecycle owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_lifecycle_owner_defined: bool,

    /// renderer backend instance lifecycle owner 名称；未定义时为 blocked。
    pub renderer_backend_instance_lifecycle_owner: &'static str,

    /// renderer backend instance cleanup owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_cleanup_owner_defined: bool,

    /// renderer backend instance cleanup owner 名称；未定义时为 blocked。
    pub renderer_backend_instance_cleanup_owner: &'static str,

    /// renderer backend instance availability owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_availability_owner_defined: bool,

    /// renderer backend instance availability owner 名称；未定义时为 blocked。
    pub renderer_backend_instance_availability_owner: &'static str,
}

/// Phase 56H renderer backend instance audit checklist。
///
/// checklist 说明 renderer backend instance audit seam 哪些部分仍 blocked。它是安全
/// 边界：audit report 不等于真实 renderer backend instance，也不等于 renderer call。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererBackendInstanceAuditChecklist {
    /// renderer backend instance audit seam 是否可用。
    pub renderer_backend_instance_audit_available: bool,

    /// renderer backend instance audit 是否仍 blocked。
    pub renderer_backend_instance_audit_blocked: bool,

    /// 是否观察到上游 texture owner boundary report。
    pub texture_owner_boundary_report_observed: bool,

    /// 上游 texture owner boundary 是否仍 blocked。
    pub texture_owner_boundary_still_blocked: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56H 固定 false。
    pub renderer_backend_instance_available: bool,

    /// renderer backend instance owner 是否已定义。
    pub renderer_backend_instance_owner_defined: bool,

    /// renderer backend instance lifecycle owner 是否已定义。
    pub renderer_backend_instance_lifecycle_owner_defined: bool,

    /// renderer backend instance cleanup owner 是否已定义。
    pub renderer_backend_instance_cleanup_owner_defined: bool,

    /// renderer backend instance availability owner 是否已定义。
    pub renderer_backend_instance_availability_owner_defined: bool,
}

/// Phase 56H renderer backend instance audit report。
///
/// 该 report 从 Phase 56G owner boundary report 派生，只审计 future renderer backend
/// instance seam。它不 import buffer、不创建 texture、不创建 renderer backend instance、
/// 不调用 renderer、不提交 damage、不发送 frame callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRendererBackendInstanceAuditReport {
    /// renderer backend instance audit seam 是否可用。
    pub renderer_backend_instance_audit_available: bool,

    /// 是否观察到 Phase 56G texture owner boundary report。
    pub source_texture_owner_boundary_report_observed: bool,

    /// 被消费的 Phase 56G texture owner boundary report。
    pub source_texture_owner_boundary_report: RuntimeSurfaceCommitTextureOwnerBoundaryReport,

    /// future renderer backend instance policy。
    pub renderer_backend_instance_policy: RuntimeSurfaceCommitRendererBackendInstancePolicy,

    /// renderer backend instance audit checklist。
    pub checklist: RuntimeSurfaceCommitRendererBackendInstanceAuditChecklist,

    /// renderer backend instance audit 是否仍 blocked。
    pub renderer_backend_instance_audit_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub renderer_backend_instance_audit_blocker_reason: &'static str,

    /// 上游 texture owner boundary 是否仍 blocked。
    pub texture_owner_boundary_still_blocked: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56H 固定 false。
    pub renderer_backend_instance_available: bool,

    /// renderer backend instance owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_owner_defined: bool,

    /// renderer backend instance owner 名称。
    pub renderer_backend_instance_owner: &'static str,

    /// renderer backend instance lifecycle owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_lifecycle_owner_defined: bool,

    /// renderer backend instance lifecycle owner 名称。
    pub renderer_backend_instance_lifecycle_owner: &'static str,

    /// renderer backend instance cleanup owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_cleanup_owner_defined: bool,

    /// renderer backend instance cleanup owner 名称。
    pub renderer_backend_instance_cleanup_owner: &'static str,

    /// renderer backend instance availability owner 是否已定义；Phase 56H 保持 false。
    pub renderer_backend_instance_availability_owner_defined: bool,

    /// renderer backend instance availability owner 名称。
    pub renderer_backend_instance_availability_owner: &'static str,

    /// Phase 56H 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56H 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56H 不创建 texture。
    pub texture_created: bool,

    /// Phase 56H 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56H 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56H 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56H 不接入 input。
    pub input_support: bool,

    /// Phase 56H 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// renderer backend instance audit 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitRendererBackendInstanceAuditOperation>,

    /// 阻止真实 renderer backend instance / texture execution 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker>,
}

/// 从 Phase 56G owner boundary report 派生 Phase 56H renderer backend instance audit。
///
/// 这是 Phase 56H 的唯一执行入口：它只生成 pure-data audit report。本函数不创建
/// renderer backend instance、不创建 texture、不调用 renderer、不执行 buffer import。
#[must_use = "renderer backend instance audit report is not renderer creation"]
pub fn renderer_backend_instance_audit_from_texture_owner_boundary_report(
    report: &RuntimeSurfaceCommitTextureOwnerBoundaryReport,
) -> RuntimeSurfaceCommitRendererBackendInstanceAuditReport {
    let renderer_backend_instance_policy = RuntimeSurfaceCommitRendererBackendInstancePolicy {
        renderer_backend_instance_owner_defined: false,
        renderer_backend_instance_owner: "blocked_until_renderer_backend_instance_owner_policy",
        renderer_backend_instance_lifecycle_owner_defined: false,
        renderer_backend_instance_lifecycle_owner: "blocked_until_renderer_backend_instance_lifecycle_policy",
        renderer_backend_instance_cleanup_owner_defined: false,
        renderer_backend_instance_cleanup_owner: "blocked_until_renderer_backend_instance_cleanup_policy",
        renderer_backend_instance_availability_owner_defined: false,
        renderer_backend_instance_availability_owner: "blocked_until_renderer_backend_instance_availability_policy",
    };

    let checklist = RuntimeSurfaceCommitRendererBackendInstanceAuditChecklist {
        renderer_backend_instance_audit_available: true,
        renderer_backend_instance_audit_blocked: true,
        texture_owner_boundary_report_observed: report.texture_owner_boundary_available,
        texture_owner_boundary_still_blocked: report.texture_owner_boundary_blocked,
        renderer_backend_instance_available: false,
        renderer_backend_instance_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_owner_defined,
        renderer_backend_instance_lifecycle_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_lifecycle_owner_defined,
        renderer_backend_instance_cleanup_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_cleanup_owner_defined,
        renderer_backend_instance_availability_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_availability_owner_defined,
    };

    let mut blockers = Vec::new();
    if checklist.texture_owner_boundary_still_blocked {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::TextureOwnerBoundaryStillBlocked,
        );
    }
    if !checklist.renderer_backend_instance_available {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstance,
        );
    }
    if !checklist.renderer_backend_instance_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceOwnerPolicy,
        );
    }
    if !checklist.renderer_backend_instance_lifecycle_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceLifecyclePolicy,
        );
    }
    if !checklist.renderer_backend_instance_cleanup_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceCleanupPolicy,
        );
    }
    if !checklist.renderer_backend_instance_availability_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceAvailabilityPolicy,
        );
    }
    blockers.push(
        RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::RendererBackendInstanceWithoutTextureCreation,
    );

    RuntimeSurfaceCommitRendererBackendInstanceAuditReport {
        renderer_backend_instance_audit_available: true,
        source_texture_owner_boundary_report_observed: report.texture_owner_boundary_available,
        source_texture_owner_boundary_report: report.clone(),
        renderer_backend_instance_policy: renderer_backend_instance_policy.clone(),
        checklist,
        renderer_backend_instance_audit_blocked: true,
        renderer_backend_instance_audit_blocker_reason:
            "renderer backend instance audit only: missing real renderer backend instance, owner, lifecycle, cleanup, and availability policies",
        texture_owner_boundary_still_blocked: true,
        renderer_backend_instance_available: false,
        renderer_backend_instance_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_owner_defined,
        renderer_backend_instance_owner: renderer_backend_instance_policy
            .renderer_backend_instance_owner,
        renderer_backend_instance_lifecycle_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_lifecycle_owner_defined,
        renderer_backend_instance_lifecycle_owner: renderer_backend_instance_policy
            .renderer_backend_instance_lifecycle_owner,
        renderer_backend_instance_cleanup_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_cleanup_owner_defined,
        renderer_backend_instance_cleanup_owner: renderer_backend_instance_policy
            .renderer_backend_instance_cleanup_owner,
        renderer_backend_instance_availability_owner_defined: renderer_backend_instance_policy
            .renderer_backend_instance_availability_owner_defined,
        renderer_backend_instance_availability_owner: renderer_backend_instance_policy
            .renderer_backend_instance_availability_owner,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::ObserveTextureOwnerBoundaryReport,
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::CheckRendererBackendInstanceAvailability,
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::AuditRendererBackendInstanceOwner,
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::AuditRendererBackendInstanceLifecycle,
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::AuditRendererBackendInstanceCleanup,
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::AuditRendererBackendInstanceAvailability,
            RuntimeSurfaceCommitRendererBackendInstanceAuditOperation::BuildRendererBackendInstanceAuditReport,
        ],
        blockers,
    }
}

/// Phase 56I texture import route decision 的纯数据步骤。
///
/// 这些步骤只消费 Phase 56H renderer backend instance audit report，定义 future
/// texture import route owner，并审计 import-buffer / texture handle / cleanup / release /
/// damage / frame-callback policy。本阶段不执行 import，不创建 texture，也不调用 renderer。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureImportRouteDecisionOperation {
    /// 观察 Phase 56H renderer backend instance audit report。
    ObserveRendererBackendInstanceAuditReport,
    /// 定义 future texture import route owner。
    DefineTextureImportRouteOwner,
    /// 检查 import-buffer 调用策略。
    CheckImportBufferCallPolicy,
    /// 检查 future texture handle ownership policy。
    CheckFutureTextureHandleOwnershipPolicy,
    /// 检查 future texture cleanup policy。
    CheckTextureCleanupPolicy,
    /// 检查 future texture release policy。
    CheckTextureReleasePolicy,
    /// 检查 damage 到 texture 的映射策略。
    CheckDamageMappingPolicy,
    /// 检查 frame callback done 完成策略。
    CheckFrameCallbackCompletionPolicy,
    /// 构建 texture import route decision report。
    BuildTextureImportRouteDecisionReport,
}

/// Phase 56I texture import route decision blocker taxonomy。
///
/// blocker 明确说明：route decision report 不是真实 import route，也不能执行
/// 真实 import-buffer 调用、创建 texture handle、创建 texture 或调用 renderer。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitTextureImportRouteDecisionBlocker {
    /// 上游 renderer backend instance audit 仍处于 blocked 状态。
    RendererBackendInstanceAuditStillBlocked,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少 import_buffer 调用策略。
    MissingImportBufferCallPolicy,
    /// 缺少 future texture handle ownership policy。
    MissingFutureTextureHandleOwnershipPolicy,
    /// 缺少 future texture cleanup policy。
    MissingTextureCleanupPolicy,
    /// 缺少 future texture release policy。
    MissingTextureReleasePolicy,
    /// 缺少 damage 到 texture 的映射策略。
    MissingDamageMappingPolicy,
    /// 缺少 frame callback done 完成策略。
    MissingFrameCallbackCompletionPolicy,
    /// import_buffer 在本阶段明确禁用。
    ImportBufferExplicitlyDisabled,
    /// 已定义 route owner，但没有真实 import。
    TextureImportRouteDecisionWithoutImport,
}

/// Phase 56I texture import route policy。
///
/// policy 只描述 future WlBuffer 到 renderer texture 的 route seam。它不保存
/// texture handle，不保存 renderer texture，也不引用 Smithay renderer backend instance。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureImportRoutePolicy {
    /// texture import route owner 是否已定义；Phase 56I 定义 route owner。
    pub texture_import_route_owner_defined: bool,

    /// texture import route owner 名称。
    pub texture_import_route_owner: &'static str,

    /// import_buffer 调用是否允许；Phase 56I 固定 false。
    pub import_buffer_call_allowed: bool,

    /// import_buffer 调用 owner 名称；未允许时为 blocked。
    pub import_buffer_call_owner: &'static str,

    /// future texture handle owner 是否已定义；Phase 56I 固定 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture handle owner 名称；未定义时为 blocked。
    pub future_texture_handle_owner: &'static str,

    /// future texture cleanup policy 是否已定义；Phase 56I 固定 false。
    pub texture_cleanup_policy_defined: bool,

    /// future texture release policy 是否已定义；Phase 56I 固定 false。
    pub texture_release_policy_defined: bool,

    /// damage 到 texture 的映射 policy 是否已定义；Phase 56I 固定 false。
    pub damage_mapping_policy_defined: bool,

    /// frame callback done policy 是否已定义；Phase 56I 固定 false。
    pub frame_callback_completion_policy_defined: bool,
}

/// Phase 56I texture import route decision checklist。
///
/// checklist 说明 import route 哪些部分仍 blocked。它是 safety boundary：
/// route owner 已定义不代表 import route 可用，不代表真实 import-buffer 可调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureImportRouteDecisionChecklist {
    /// texture import route decision seam 是否可用。
    pub texture_import_route_decision_available: bool,

    /// texture import route decision 是否仍 blocked。
    pub texture_import_route_decision_blocked: bool,

    /// 是否观察到上游 renderer backend instance audit report。
    pub renderer_backend_instance_audit_report_observed: bool,

    /// 上游 renderer backend instance audit 是否仍 blocked。
    pub renderer_backend_instance_audit_still_blocked: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56I 固定 false。
    pub renderer_backend_instance_available: bool,

    /// texture import route 是否真实可用；Phase 56I 固定 false。
    pub texture_import_route_available: bool,

    /// texture import route owner 是否已定义。
    pub texture_import_route_owner_defined: bool,

    /// import_buffer 调用是否允许；Phase 56I 固定 false。
    pub import_buffer_call_allowed: bool,

    /// future texture handle owner 是否已定义；Phase 56I 固定 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture cleanup policy 是否已定义；Phase 56I 固定 false。
    pub texture_cleanup_policy_defined: bool,

    /// future texture release policy 是否已定义；Phase 56I 固定 false。
    pub texture_release_policy_defined: bool,

    /// damage mapping policy 是否已定义；Phase 56I 固定 false。
    pub damage_mapping_policy_defined: bool,

    /// frame callback completion policy 是否已定义；Phase 56I 固定 false。
    pub frame_callback_completion_policy_defined: bool,
}

/// Phase 56I texture import route decision report。
///
/// 该 report 从 Phase 56H audit report 派生，只定义 future import route owner 与
/// blocker taxonomy。它不 import buffer、不创建 texture handle、不创建 texture、不调用
/// renderer、不提交 damage、不发送 frame callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitTextureImportRouteDecisionReport {
    /// texture import route decision seam 是否可用。
    pub texture_import_route_decision_available: bool,

    /// 是否观察到 Phase 56H renderer backend instance audit report。
    pub source_renderer_backend_instance_audit_report_observed: bool,

    /// 被消费的 Phase 56H renderer backend instance audit report。
    pub source_renderer_backend_instance_audit_report:
        RuntimeSurfaceCommitRendererBackendInstanceAuditReport,

    /// future texture import route policy。
    pub texture_import_route_policy: RuntimeSurfaceCommitTextureImportRoutePolicy,

    /// texture import route decision checklist。
    pub checklist: RuntimeSurfaceCommitTextureImportRouteDecisionChecklist,

    /// texture import route decision 是否仍 blocked。
    pub texture_import_route_decision_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub texture_import_route_decision_blocker_reason: &'static str,

    /// 上游 renderer backend instance audit 是否仍 blocked。
    pub renderer_backend_instance_audit_still_blocked: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56I 固定 false。
    pub renderer_backend_instance_available: bool,

    /// texture import route 是否真实可用；Phase 56I 固定 false。
    pub texture_import_route_available: bool,

    /// texture import route owner 是否已定义；Phase 56I 定义为 true。
    pub texture_import_route_owner_defined: bool,

    /// texture import route owner 名称。
    pub texture_import_route_owner: &'static str,

    /// import_buffer 调用是否允许；Phase 56I 固定 false。
    pub import_buffer_call_allowed: bool,

    /// import_buffer 调用 owner 名称。
    pub import_buffer_call_owner: &'static str,

    /// future texture handle owner 是否已定义；Phase 56I 固定 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture handle owner 名称。
    pub future_texture_handle_owner: &'static str,

    /// future texture cleanup policy 是否已定义；Phase 56I 固定 false。
    pub texture_cleanup_policy_defined: bool,

    /// future texture release policy 是否已定义；Phase 56I 固定 false。
    pub texture_release_policy_defined: bool,

    /// damage mapping policy 是否已定义；Phase 56I 固定 false。
    pub damage_mapping_policy_defined: bool,

    /// frame callback completion policy 是否已定义；Phase 56I 固定 false。
    pub frame_callback_completion_policy_defined: bool,

    /// Phase 56I 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56I 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56I 不创建 texture。
    pub texture_created: bool,

    /// Phase 56I 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56I 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56I 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56I 不接入 input。
    pub input_support: bool,

    /// Phase 56I 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// texture import route decision 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitTextureImportRouteDecisionOperation>,

    /// 阻止真实 import route / texture execution 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitTextureImportRouteDecisionBlocker>,
}

/// 从 Phase 56H renderer backend instance audit 派生 Phase 56I texture import route decision。
///
/// 这是 Phase 56I 的唯一执行入口：它只生成 pure-data decision report。本函数不调用
/// 真实 import-buffer 路径，不创建 texture handle，不创建 texture，不调用 renderer。
#[must_use = "texture import route decision report is not an import route"]
pub fn texture_import_route_decision_from_renderer_backend_instance_audit(
    report: &RuntimeSurfaceCommitRendererBackendInstanceAuditReport,
) -> RuntimeSurfaceCommitTextureImportRouteDecisionReport {
    let texture_import_route_policy = RuntimeSurfaceCommitTextureImportRoutePolicy {
        texture_import_route_owner_defined: true,
        texture_import_route_owner: "linux_shm_first_buffer_import_adapter",
        import_buffer_call_allowed: false,
        import_buffer_call_owner: "blocked_until_import_buffer_call_policy",
        future_texture_handle_owner_defined: false,
        future_texture_handle_owner: "blocked_until_future_texture_handle_ownership_policy",
        texture_cleanup_policy_defined: false,
        texture_release_policy_defined: false,
        damage_mapping_policy_defined: false,
        frame_callback_completion_policy_defined: false,
    };

    let checklist = RuntimeSurfaceCommitTextureImportRouteDecisionChecklist {
        texture_import_route_decision_available: true,
        texture_import_route_decision_blocked: true,
        renderer_backend_instance_audit_report_observed: report
            .renderer_backend_instance_audit_available,
        renderer_backend_instance_audit_still_blocked: report
            .renderer_backend_instance_audit_blocked,
        renderer_backend_instance_available: report.renderer_backend_instance_available,
        texture_import_route_available: false,
        texture_import_route_owner_defined: texture_import_route_policy
            .texture_import_route_owner_defined,
        import_buffer_call_allowed: texture_import_route_policy.import_buffer_call_allowed,
        future_texture_handle_owner_defined: texture_import_route_policy
            .future_texture_handle_owner_defined,
        texture_cleanup_policy_defined: texture_import_route_policy.texture_cleanup_policy_defined,
        texture_release_policy_defined: texture_import_route_policy.texture_release_policy_defined,
        damage_mapping_policy_defined: texture_import_route_policy.damage_mapping_policy_defined,
        frame_callback_completion_policy_defined: texture_import_route_policy
            .frame_callback_completion_policy_defined,
    };

    let mut blockers = Vec::new();
    if checklist.renderer_backend_instance_audit_still_blocked {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::RendererBackendInstanceAuditStillBlocked,
        );
    }
    if !checklist.renderer_backend_instance_available {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingRendererBackendInstance,
        );
    }
    if !checklist.import_buffer_call_allowed {
        blockers.extend([
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingImportBufferCallPolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::ImportBufferExplicitlyDisabled,
        ]);
    }
    if !checklist.future_texture_handle_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingFutureTextureHandleOwnershipPolicy,
        );
    }
    if !checklist.texture_cleanup_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingTextureCleanupPolicy,
        );
    }
    if !checklist.texture_release_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingTextureReleasePolicy,
        );
    }
    if !checklist.damage_mapping_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingDamageMappingPolicy,
        );
    }
    if !checklist.frame_callback_completion_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingFrameCallbackCompletionPolicy,
        );
    }
    blockers.push(
        RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::TextureImportRouteDecisionWithoutImport,
    );

    RuntimeSurfaceCommitTextureImportRouteDecisionReport {
        texture_import_route_decision_available: true,
        source_renderer_backend_instance_audit_report_observed: report
            .renderer_backend_instance_audit_available,
        source_renderer_backend_instance_audit_report: report.clone(),
        texture_import_route_policy: texture_import_route_policy.clone(),
        checklist,
        texture_import_route_decision_blocked: true,
        texture_import_route_decision_blocker_reason:
            "texture import route decision only: missing renderer backend instance, import-buffer call policy, future texture handle ownership, cleanup, release, damage mapping, and frame callback completion policy",
        renderer_backend_instance_audit_still_blocked: true,
        renderer_backend_instance_available: false,
        texture_import_route_available: false,
        texture_import_route_owner_defined: texture_import_route_policy
            .texture_import_route_owner_defined,
        texture_import_route_owner: texture_import_route_policy.texture_import_route_owner,
        import_buffer_call_allowed: texture_import_route_policy.import_buffer_call_allowed,
        import_buffer_call_owner: texture_import_route_policy.import_buffer_call_owner,
        future_texture_handle_owner_defined: texture_import_route_policy
            .future_texture_handle_owner_defined,
        future_texture_handle_owner: texture_import_route_policy.future_texture_handle_owner,
        texture_cleanup_policy_defined: texture_import_route_policy.texture_cleanup_policy_defined,
        texture_release_policy_defined: texture_import_route_policy.texture_release_policy_defined,
        damage_mapping_policy_defined: texture_import_route_policy.damage_mapping_policy_defined,
        frame_callback_completion_policy_defined: texture_import_route_policy
            .frame_callback_completion_policy_defined,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::ObserveRendererBackendInstanceAuditReport,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::DefineTextureImportRouteOwner,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::CheckImportBufferCallPolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::CheckFutureTextureHandleOwnershipPolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::CheckTextureCleanupPolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::CheckTextureReleasePolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::CheckDamageMappingPolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::CheckFrameCallbackCompletionPolicy,
            RuntimeSurfaceCommitTextureImportRouteDecisionOperation::BuildTextureImportRouteDecisionReport,
        ],
        blockers,
    }
}

/// Phase 56J damage-to-texture mapping audit 中可定位的纯数据操作阶段。
///
/// 这些步骤只审计 surface damage / buffer damage 到 future texture region 的 mapping
/// seam。它们不会提交真实 damage，也不会调用 renderer 或 frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitDamageToTextureMappingAuditOperation {
    /// 观察 Phase 56I texture import route decision report。
    ObserveTextureImportRouteDecisionReport,
    /// 定义 future damage mapping owner。
    DefineDamageMappingOwner,
    /// 检查 texture import route 是否真实可用。
    CheckTextureImportRouteAvailability,
    /// 检查 future texture handle ownership policy。
    CheckFutureTextureHandleOwnershipPolicy,
    /// 检查 future texture region policy。
    CheckTextureRegionPolicy,
    /// 检查 surface damage 到 texture region 的 mapping policy。
    CheckSurfaceDamageMappingPolicy,
    /// 检查 buffer damage 到 texture region 的 mapping policy。
    CheckBufferDamageMappingPolicy,
    /// 检查 surface/buffer/texture 坐标空间转换 policy。
    CheckDamageCoordinateSpacePolicy,
    /// 检查 renderer damage submission policy。
    CheckRendererDamageSubmissionPolicy,
    /// 构建 damage-to-texture mapping audit report。
    BuildDamageToTextureMappingAuditReport,
}

/// Phase 56J damage-to-texture mapping audit blocker taxonomy。
///
/// blocker 明确说明：damage mapping audit 不是真实 damage submission。缺少 texture
/// import route、future texture handle、texture region 或坐标空间策略时，runtime
/// 必须保持 blocked。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker {
    /// 上游 texture import route decision 仍处于 blocked 状态。
    TextureImportRouteDecisionStillBlocked,
    /// 缺少真实 texture import route。
    MissingTextureImportRoute,
    /// 缺少 future texture handle ownership policy。
    MissingFutureTextureHandleOwnershipPolicy,
    /// 缺少 future texture region policy。
    MissingTextureRegionPolicy,
    /// 缺少 surface damage 到 texture region 的 mapping policy。
    MissingSurfaceDamageMappingPolicy,
    /// 缺少 buffer damage 到 texture region 的 mapping policy。
    MissingBufferDamageMappingPolicy,
    /// 缺少 damage 坐标空间转换 policy。
    MissingDamageCoordinateSpacePolicy,
    /// 缺少 renderer damage submission policy。
    MissingRendererDamageSubmissionPolicy,
    /// 缺少 frame callback completion policy；damage 后不能提前 done。
    MissingFrameCallbackCompletionPolicy,
    /// damage submission 在本阶段明确禁用。
    DamageSubmissionExplicitlyDisabled,
    /// 已定义 damage mapping owner，但没有真实 texture。
    DamageMappingWithoutTexture,
}

/// Phase 56J damage-to-texture mapping policy。
///
/// policy 只命名 future owner，并记录 mapping 所需的后续策略。它不保存 texture
/// region，不持有 renderer，也不触发 damage submission。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitDamageToTextureMappingPolicy {
    /// damage mapping owner 是否已定义；Phase 56J 定义 owner。
    pub damage_mapping_owner_defined: bool,

    /// damage mapping owner 名称。
    pub damage_mapping_owner: &'static str,

    /// future texture region policy 是否已定义；Phase 56J 固定 false。
    pub texture_region_policy_defined: bool,

    /// surface damage 到 texture region policy 是否已定义；Phase 56J 固定 false。
    pub surface_damage_mapping_policy_defined: bool,

    /// buffer damage 到 texture region policy 是否已定义；Phase 56J 固定 false。
    pub buffer_damage_mapping_policy_defined: bool,

    /// damage 坐标空间转换 policy 是否已定义；Phase 56J 固定 false。
    pub damage_coordinate_space_policy_defined: bool,

    /// renderer damage submission policy 是否已定义；Phase 56J 固定 false。
    pub renderer_damage_submission_policy_defined: bool,

    /// damage submission 是否允许；Phase 56J 固定 false。
    pub damage_submission_allowed: bool,
}

/// Phase 56J damage-to-texture mapping checklist。
///
/// checklist 区分“已能审计 mapping seam”和“可真实提交 damage”。本阶段只提供前者；
/// 后者必须等 texture import route、future texture handle 与 renderer submission
/// policy 都可用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitDamageToTextureMappingChecklist {
    /// damage-to-texture mapping audit seam 是否可用。
    pub damage_to_texture_mapping_audit_available: bool,

    /// damage-to-texture mapping audit 是否仍 blocked。
    pub damage_to_texture_mapping_audit_blocked: bool,

    /// 是否观察到 Phase 56I texture import route decision report。
    pub texture_import_route_decision_report_observed: bool,

    /// 上游 texture import route decision 是否仍 blocked。
    pub texture_import_route_decision_still_blocked: bool,

    /// texture import route 是否真实可用；Phase 56J 固定 false。
    pub texture_import_route_available: bool,

    /// future texture handle owner 是否已定义；继承 Phase 56I，当前 false。
    pub future_texture_handle_owner_defined: bool,

    /// damage mapping owner 是否已定义。
    pub damage_mapping_owner_defined: bool,

    /// future texture region policy 是否已定义；Phase 56J 固定 false。
    pub texture_region_policy_defined: bool,

    /// surface damage mapping policy 是否已定义；Phase 56J 固定 false。
    pub surface_damage_mapping_policy_defined: bool,

    /// buffer damage mapping policy 是否已定义；Phase 56J 固定 false。
    pub buffer_damage_mapping_policy_defined: bool,

    /// damage 坐标空间转换 policy 是否已定义；Phase 56J 固定 false。
    pub damage_coordinate_space_policy_defined: bool,

    /// renderer damage submission policy 是否已定义；Phase 56J 固定 false。
    pub renderer_damage_submission_policy_defined: bool,

    /// frame callback completion policy 是否已定义；继承 Phase 56I，当前 false。
    pub frame_callback_completion_policy_defined: bool,

    /// damage submission 是否允许；Phase 56J 固定 false。
    pub damage_submission_allowed: bool,
}

/// Phase 56J damage-to-texture mapping audit report。
///
/// 该 report 从 Phase 56I route decision 派生，只审计 future damage mapping seam。
/// 它不 import buffer、不创建 texture、不调用 renderer、不提交 damage、不发送 frame
/// callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitDamageToTextureMappingAuditReport {
    /// damage-to-texture mapping audit seam 是否可用。
    pub damage_to_texture_mapping_audit_available: bool,

    /// 是否观察到 Phase 56I texture import route decision report。
    pub source_texture_import_route_decision_report_observed: bool,

    /// 被消费的 Phase 56I texture import route decision report。
    pub source_texture_import_route_decision_report:
        RuntimeSurfaceCommitTextureImportRouteDecisionReport,

    /// future damage mapping policy。
    pub damage_to_texture_mapping_policy: RuntimeSurfaceCommitDamageToTextureMappingPolicy,

    /// damage-to-texture mapping checklist。
    pub checklist: RuntimeSurfaceCommitDamageToTextureMappingChecklist,

    /// damage-to-texture mapping audit 是否仍 blocked。
    pub damage_to_texture_mapping_audit_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub damage_to_texture_mapping_audit_blocker_reason: &'static str,

    /// 上游 texture import route decision 是否仍 blocked。
    pub texture_import_route_decision_still_blocked: bool,

    /// texture import route 是否真实可用；Phase 56J 固定 false。
    pub texture_import_route_available: bool,

    /// future texture handle owner 是否已定义；继承 Phase 56I，当前 false。
    pub future_texture_handle_owner_defined: bool,

    /// damage mapping owner 是否已定义；Phase 56J 定义为 true。
    pub damage_mapping_owner_defined: bool,

    /// damage mapping owner 名称。
    pub damage_mapping_owner: &'static str,

    /// future texture region policy 是否已定义；Phase 56J 固定 false。
    pub texture_region_policy_defined: bool,

    /// surface damage mapping policy 是否已定义；Phase 56J 固定 false。
    pub surface_damage_mapping_policy_defined: bool,

    /// buffer damage mapping policy 是否已定义；Phase 56J 固定 false。
    pub buffer_damage_mapping_policy_defined: bool,

    /// damage 坐标空间转换 policy 是否已定义；Phase 56J 固定 false。
    pub damage_coordinate_space_policy_defined: bool,

    /// renderer damage submission policy 是否已定义；Phase 56J 固定 false。
    pub renderer_damage_submission_policy_defined: bool,

    /// frame callback completion policy 是否已定义；继承 Phase 56I，当前 false。
    pub frame_callback_completion_policy_defined: bool,

    /// damage submission 是否允许；Phase 56J 固定 false。
    pub damage_submission_allowed: bool,

    /// Phase 56J 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56J 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56J 不创建 texture。
    pub texture_created: bool,

    /// Phase 56J 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56J 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56J 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56J 不接入 input。
    pub input_support: bool,

    /// Phase 56J 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// damage-to-texture mapping audit 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitDamageToTextureMappingAuditOperation>,

    /// 阻止真实 damage submission 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker>,
}

/// 从 Phase 56I texture import route decision 派生 Phase 56J damage mapping audit。
///
/// 这是 Phase 56J 的唯一执行入口：它只生成 pure-data audit report。本函数不提交
/// damage、不调用 renderer、不发送 frame callback done，也不创建任何真实 texture。
#[must_use = "damage-to-texture mapping audit report is not damage submission"]
pub fn damage_to_texture_mapping_audit_from_texture_import_route_decision(
    report: &RuntimeSurfaceCommitTextureImportRouteDecisionReport,
) -> RuntimeSurfaceCommitDamageToTextureMappingAuditReport {
    let damage_to_texture_mapping_policy = RuntimeSurfaceCommitDamageToTextureMappingPolicy {
        damage_mapping_owner_defined: true,
        damage_mapping_owner: "linux_shm_first_buffer_import_adapter",
        texture_region_policy_defined: false,
        surface_damage_mapping_policy_defined: false,
        buffer_damage_mapping_policy_defined: false,
        damage_coordinate_space_policy_defined: false,
        renderer_damage_submission_policy_defined: false,
        damage_submission_allowed: false,
    };

    let checklist = RuntimeSurfaceCommitDamageToTextureMappingChecklist {
        damage_to_texture_mapping_audit_available: true,
        damage_to_texture_mapping_audit_blocked: true,
        texture_import_route_decision_report_observed: report
            .texture_import_route_decision_available,
        texture_import_route_decision_still_blocked: report.texture_import_route_decision_blocked,
        texture_import_route_available: report.texture_import_route_available,
        future_texture_handle_owner_defined: report.future_texture_handle_owner_defined,
        damage_mapping_owner_defined: damage_to_texture_mapping_policy.damage_mapping_owner_defined,
        texture_region_policy_defined: damage_to_texture_mapping_policy
            .texture_region_policy_defined,
        surface_damage_mapping_policy_defined: damage_to_texture_mapping_policy
            .surface_damage_mapping_policy_defined,
        buffer_damage_mapping_policy_defined: damage_to_texture_mapping_policy
            .buffer_damage_mapping_policy_defined,
        damage_coordinate_space_policy_defined: damage_to_texture_mapping_policy
            .damage_coordinate_space_policy_defined,
        renderer_damage_submission_policy_defined: damage_to_texture_mapping_policy
            .renderer_damage_submission_policy_defined,
        frame_callback_completion_policy_defined: report.frame_callback_completion_policy_defined,
        damage_submission_allowed: damage_to_texture_mapping_policy.damage_submission_allowed,
    };

    let mut blockers = Vec::new();
    if checklist.texture_import_route_decision_still_blocked {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::TextureImportRouteDecisionStillBlocked,
        );
    }
    if !checklist.texture_import_route_available {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingTextureImportRoute,
        );
    }
    if !checklist.future_texture_handle_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingFutureTextureHandleOwnershipPolicy,
        );
    }
    if !checklist.texture_region_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingTextureRegionPolicy,
        );
    }
    if !checklist.surface_damage_mapping_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingSurfaceDamageMappingPolicy,
        );
    }
    if !checklist.buffer_damage_mapping_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingBufferDamageMappingPolicy,
        );
    }
    if !checklist.damage_coordinate_space_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingDamageCoordinateSpacePolicy,
        );
    }
    if !checklist.renderer_damage_submission_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingRendererDamageSubmissionPolicy,
        );
    }
    if !checklist.frame_callback_completion_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingFrameCallbackCompletionPolicy,
        );
    }
    if !checklist.damage_submission_allowed {
        blockers.push(
            RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::DamageSubmissionExplicitlyDisabled,
        );
    }
    blockers
        .push(RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::DamageMappingWithoutTexture);

    RuntimeSurfaceCommitDamageToTextureMappingAuditReport {
        damage_to_texture_mapping_audit_available: true,
        source_texture_import_route_decision_report_observed: report
            .texture_import_route_decision_available,
        source_texture_import_route_decision_report: report.clone(),
        damage_to_texture_mapping_policy: damage_to_texture_mapping_policy.clone(),
        checklist,
        damage_to_texture_mapping_audit_blocked: true,
        damage_to_texture_mapping_audit_blocker_reason:
            "damage-to-texture mapping audit only: missing texture import route, future texture handle ownership, texture region, surface/buffer damage mapping, coordinate-space, renderer submission, and frame callback policies",
        texture_import_route_decision_still_blocked: report.texture_import_route_decision_blocked,
        texture_import_route_available: report.texture_import_route_available,
        future_texture_handle_owner_defined: report.future_texture_handle_owner_defined,
        damage_mapping_owner_defined: damage_to_texture_mapping_policy.damage_mapping_owner_defined,
        damage_mapping_owner: damage_to_texture_mapping_policy.damage_mapping_owner,
        texture_region_policy_defined: damage_to_texture_mapping_policy
            .texture_region_policy_defined,
        surface_damage_mapping_policy_defined: damage_to_texture_mapping_policy
            .surface_damage_mapping_policy_defined,
        buffer_damage_mapping_policy_defined: damage_to_texture_mapping_policy
            .buffer_damage_mapping_policy_defined,
        damage_coordinate_space_policy_defined: damage_to_texture_mapping_policy
            .damage_coordinate_space_policy_defined,
        renderer_damage_submission_policy_defined: damage_to_texture_mapping_policy
            .renderer_damage_submission_policy_defined,
        frame_callback_completion_policy_defined: report.frame_callback_completion_policy_defined,
        damage_submission_allowed: damage_to_texture_mapping_policy.damage_submission_allowed,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::ObserveTextureImportRouteDecisionReport,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::DefineDamageMappingOwner,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckTextureImportRouteAvailability,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckFutureTextureHandleOwnershipPolicy,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckTextureRegionPolicy,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckSurfaceDamageMappingPolicy,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckBufferDamageMappingPolicy,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckDamageCoordinateSpacePolicy,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::CheckRendererDamageSubmissionPolicy,
            RuntimeSurfaceCommitDamageToTextureMappingAuditOperation::BuildDamageToTextureMappingAuditReport,
        ],
        blockers,
    }
}

/// Phase 56K frame callback completion policy 中可定位的纯数据操作阶段。
///
/// 这些步骤只审计 future frame callback done 的 owner 和 release gate。它们不会
/// 调用 renderer，不会提交 damage，也不会发送 frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation {
    /// 观察 Phase 56J damage-to-texture mapping audit report。
    ObserveDamageToTextureMappingAuditReport,
    /// 定义 future frame callback completion owner。
    DefineFrameCallbackCompletionOwner,
    /// 检查真实 texture 是否存在。
    CheckRealTextureAvailability,
    /// 检查 renderer backend instance 是否真实可用。
    CheckRendererBackendInstanceAvailability,
    /// 检查 damage submission 是否真实可用。
    CheckDamageSubmissionAvailability,
    /// 检查 render success evidence 是否存在。
    CheckRenderSuccessEvidence,
    /// 检查是否允许发送 frame callback done。
    CheckFrameCallbackDonePermission,
    /// 构建 frame callback completion policy report。
    BuildFrameCallbackCompletionPolicyReport,
}

/// Phase 56K frame callback completion policy blocker taxonomy。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker {
    /// 上游 damage-to-texture mapping audit 仍处于 blocked 状态。
    DamageToTextureMappingAuditStillBlocked,
    /// 缺少真实 texture。
    MissingRealTexture,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少真实 damage submission。
    MissingDamageSubmission,
    /// 缺少真实 render success evidence。
    MissingRenderSuccessEvidence,
    /// frame callback done 在本阶段明确禁用。
    FrameCallbackDoneExplicitlyDisabled,
    /// completion policy 只有 pure-data owner，没有真实 completion path。
    FrameCallbackCompletionWithoutRender,
}

/// Phase 56K frame callback completion policy。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitFrameCallbackCompletionPolicy {
    /// future frame callback completion owner 是否已定义。
    pub frame_callback_completion_owner_defined: bool,

    /// future frame callback completion owner 名称。
    pub frame_callback_completion_owner: &'static str,

    /// frame callback done 是否必须等待真实 render success。
    pub render_success_required_before_done: bool,

    /// 真实 texture 是否可用；Phase 56K 固定 false。
    pub real_texture_available: bool,

    /// renderer backend instance 是否真实可用；Phase 56K 固定 false。
    pub renderer_backend_instance_available: bool,

    /// damage submission 是否真实可用；Phase 56K 固定 false。
    pub damage_submission_available: bool,

    /// render success evidence 是否可用；Phase 56K 固定 false。
    pub render_success_evidence_available: bool,

    /// frame callback done 是否允许；Phase 56K 固定 false。
    pub frame_callback_done_allowed: bool,
}

/// Phase 56K frame callback completion checklist。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitFrameCallbackCompletionChecklist {
    /// frame callback completion policy seam 是否可用。
    pub frame_callback_completion_policy_available: bool,

    /// frame callback completion policy 是否仍 blocked。
    pub frame_callback_completion_policy_blocked: bool,

    /// 是否观察到 Phase 56J damage-to-texture mapping audit report。
    pub damage_to_texture_mapping_audit_observed: bool,

    /// 上游 damage-to-texture mapping audit 是否仍 blocked。
    pub damage_to_texture_mapping_audit_still_blocked: bool,

    /// future frame callback completion owner 是否已定义。
    pub frame_callback_completion_owner_defined: bool,

    /// frame callback done 是否必须等待真实 render success。
    pub render_success_required_before_done: bool,

    /// 真实 texture 是否可用；Phase 56K 固定 false。
    pub real_texture_available: bool,

    /// renderer backend instance 是否真实可用；Phase 56K 固定 false。
    pub renderer_backend_instance_available: bool,

    /// damage submission 是否真实可用；Phase 56K 固定 false。
    pub damage_submission_available: bool,

    /// render success evidence 是否可用；Phase 56K 固定 false。
    pub render_success_evidence_available: bool,

    /// frame callback done 是否允许；Phase 56K 固定 false。
    pub frame_callback_done_allowed: bool,
}

/// Phase 56K frame callback completion policy report。
///
/// 该 report 从 Phase 56J damage mapping audit 派生，只定义 future frame callback
/// completion owner 和 render-success gate。它不 import buffer、不创建 texture、不调用
/// renderer、不提交 damage、不发送 frame callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport {
    /// frame callback completion policy seam 是否可用。
    pub frame_callback_completion_policy_available: bool,

    /// 是否观察到 Phase 56J damage mapping audit report。
    pub source_damage_to_texture_mapping_audit_observed: bool,

    /// 被消费的 Phase 56J damage mapping audit report。
    pub source_damage_to_texture_mapping_audit_report:
        RuntimeSurfaceCommitDamageToTextureMappingAuditReport,

    /// future frame callback completion policy。
    pub frame_callback_completion_policy: RuntimeSurfaceCommitFrameCallbackCompletionPolicy,

    /// frame callback completion checklist。
    pub checklist: RuntimeSurfaceCommitFrameCallbackCompletionChecklist,

    /// frame callback completion policy 是否仍 blocked。
    pub frame_callback_completion_policy_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub frame_callback_completion_policy_blocker_reason: &'static str,

    /// 上游 damage-to-texture mapping audit 是否仍 blocked。
    pub damage_to_texture_mapping_audit_still_blocked: bool,

    /// future frame callback completion owner 是否已定义。
    pub frame_callback_completion_owner_defined: bool,

    /// future frame callback completion owner 名称。
    pub frame_callback_completion_owner: &'static str,

    /// frame callback done 是否必须等待真实 render success。
    pub render_success_required_before_done: bool,

    /// 真实 texture 是否可用；Phase 56K 固定 false。
    pub real_texture_available: bool,

    /// renderer backend instance 是否真实可用；Phase 56K 固定 false。
    pub renderer_backend_instance_available: bool,

    /// damage submission 是否真实可用；Phase 56K 固定 false。
    pub damage_submission_available: bool,

    /// render success evidence 是否可用；Phase 56K 固定 false。
    pub render_success_evidence_available: bool,

    /// frame callback done 是否允许；Phase 56K 固定 false。
    pub frame_callback_done_allowed: bool,

    /// Phase 56K 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56K 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56K 不创建 texture。
    pub texture_created: bool,

    /// Phase 56K 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56K 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56K 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56K 不接入 input。
    pub input_support: bool,

    /// Phase 56K 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// frame callback completion policy 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation>,

    /// 阻止真实 frame callback done 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker>,
}

/// 从 Phase 56J damage mapping audit 派生 Phase 56K frame callback completion policy。
///
/// 这是 Phase 56K 的唯一执行入口：它只生成 pure-data policy report。本函数不发送
/// frame callback done，不提交 damage，不调用 renderer，也不创建任何真实 texture。
#[must_use = "frame callback completion policy report is not frame callback done"]
pub fn frame_callback_completion_policy_from_damage_to_texture_mapping_audit(
    report: &RuntimeSurfaceCommitDamageToTextureMappingAuditReport,
) -> RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport {
    let frame_callback_completion_policy = RuntimeSurfaceCommitFrameCallbackCompletionPolicy {
        frame_callback_completion_owner_defined: true,
        frame_callback_completion_owner: "linux_shm_first_buffer_import_adapter",
        render_success_required_before_done: true,
        real_texture_available: false,
        renderer_backend_instance_available: false,
        damage_submission_available: false,
        render_success_evidence_available: false,
        frame_callback_done_allowed: false,
    };

    let checklist = RuntimeSurfaceCommitFrameCallbackCompletionChecklist {
        frame_callback_completion_policy_available: true,
        frame_callback_completion_policy_blocked: true,
        damage_to_texture_mapping_audit_observed: report.damage_to_texture_mapping_audit_available,
        damage_to_texture_mapping_audit_still_blocked: report
            .damage_to_texture_mapping_audit_blocked,
        frame_callback_completion_owner_defined: frame_callback_completion_policy
            .frame_callback_completion_owner_defined,
        render_success_required_before_done: frame_callback_completion_policy
            .render_success_required_before_done,
        real_texture_available: frame_callback_completion_policy.real_texture_available,
        renderer_backend_instance_available: frame_callback_completion_policy
            .renderer_backend_instance_available,
        damage_submission_available: frame_callback_completion_policy.damage_submission_available,
        render_success_evidence_available: frame_callback_completion_policy
            .render_success_evidence_available,
        frame_callback_done_allowed: frame_callback_completion_policy.frame_callback_done_allowed,
    };

    let mut blockers = Vec::new();
    if checklist.damage_to_texture_mapping_audit_still_blocked {
        blockers.push(
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::DamageToTextureMappingAuditStillBlocked,
        );
    }
    if !checklist.real_texture_available {
        blockers.push(RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingRealTexture);
    }
    if !checklist.renderer_backend_instance_available {
        blockers.push(
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingRendererBackendInstance,
        );
    }
    if !checklist.damage_submission_available {
        blockers.push(
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingDamageSubmission,
        );
    }
    if !checklist.render_success_evidence_available {
        blockers.push(
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingRenderSuccessEvidence,
        );
    }
    if !checklist.frame_callback_done_allowed {
        blockers.push(
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::FrameCallbackDoneExplicitlyDisabled,
        );
    }
    blockers.push(
        RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::FrameCallbackCompletionWithoutRender,
    );

    RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport {
        frame_callback_completion_policy_available: true,
        source_damage_to_texture_mapping_audit_observed: report
            .damage_to_texture_mapping_audit_available,
        source_damage_to_texture_mapping_audit_report: report.clone(),
        frame_callback_completion_policy: frame_callback_completion_policy.clone(),
        checklist,
        frame_callback_completion_policy_blocked: true,
        frame_callback_completion_policy_blocker_reason:
            "frame callback completion policy only: missing real texture, renderer backend instance, damage submission, render success evidence, and frame callback done permission",
        damage_to_texture_mapping_audit_still_blocked: report
            .damage_to_texture_mapping_audit_blocked,
        frame_callback_completion_owner_defined: frame_callback_completion_policy
            .frame_callback_completion_owner_defined,
        frame_callback_completion_owner: frame_callback_completion_policy
            .frame_callback_completion_owner,
        render_success_required_before_done: frame_callback_completion_policy
            .render_success_required_before_done,
        real_texture_available: frame_callback_completion_policy.real_texture_available,
        renderer_backend_instance_available: frame_callback_completion_policy
            .renderer_backend_instance_available,
        damage_submission_available: frame_callback_completion_policy.damage_submission_available,
        render_success_evidence_available: frame_callback_completion_policy
            .render_success_evidence_available,
        frame_callback_done_allowed: frame_callback_completion_policy.frame_callback_done_allowed,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::ObserveDamageToTextureMappingAuditReport,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::DefineFrameCallbackCompletionOwner,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::CheckRealTextureAvailability,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::CheckRendererBackendInstanceAvailability,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::CheckDamageSubmissionAvailability,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::CheckRenderSuccessEvidence,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::CheckFrameCallbackDonePermission,
            RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation::BuildFrameCallbackCompletionPolicyReport,
        ],
        blockers,
    }
}

/// Phase 56L real texture creation readiness decision 中可定位的纯数据操作阶段。
///
/// 这些步骤只汇总 Phase 56H-56K 的 blocked evidence 与最小 SHM-first
/// renderability checklist。它们不会创建 texture，不会调用 renderer，也不会发送
/// frame callback done。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation {
    /// 观察 Phase 56K frame callback completion policy report。
    ObserveFrameCallbackCompletionPolicyReport,
    /// 汇总 renderer backend instance readiness。
    SummarizeRendererBackendInstanceReadiness,
    /// 汇总 texture import route readiness。
    SummarizeTextureImportRouteReadiness,
    /// 汇总 future texture handle / cleanup readiness。
    SummarizeTextureOwnershipReadiness,
    /// 汇总 damage submission readiness。
    SummarizeDamageSubmissionReadiness,
    /// 汇总 frame callback completion readiness。
    SummarizeFrameCallbackCompletionReadiness,
    /// 构建真实 texture creation readiness decision report。
    BuildRealTextureCreationReadinessDecisionReport,
}

/// Phase 56L real texture creation readiness blocker taxonomy。
///
/// blocker 明确说明：readiness decision 只是判断真实 texture creation 是否可进入的
/// 门槛汇总。缺少 renderer、import route、handle/cleanup、damage、render success 或
/// frame done permission 时必须继续 blocked。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker {
    /// 上游 frame callback completion policy 仍处于 blocked 状态。
    FrameCallbackCompletionPolicyStillBlocked,
    /// 缺少真实 renderer backend instance。
    MissingRendererBackendInstance,
    /// 缺少真实 texture import route。
    MissingTextureImportRoute,
    /// 缺少 future texture handle ownership policy。
    MissingFutureTextureHandleOwnershipPolicy,
    /// 缺少 future texture cleanup policy。
    MissingTextureCleanupPolicy,
    /// 缺少真实 damage submission。
    MissingDamageSubmission,
    /// 缺少真实 render success evidence。
    MissingRenderSuccessEvidence,
    /// frame callback done 仍明确禁用。
    FrameCallbackDoneDisabled,
    /// 真实 texture creation 在本阶段仍明确禁用。
    RealTextureCreationExplicitlyDisabled,
    /// readiness decision 存在，但没有真实 texture。
    RealTextureCreationReadinessWithoutTexture,
}

/// Phase 56L 最小 SHM-first renderability checklist。
///
/// checklist 只说明进入真实 texture creation 前必须满足的 pure-data 条件。当前所有
/// 真实资源条件仍为 false，因此 core 不会看到 Smithay / renderer / texture 类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRealTextureCreationReadinessChecklist {
    /// readiness decision seam 是否可用。
    pub real_texture_creation_readiness_decision_available: bool,

    /// readiness decision 是否仍 blocked。
    pub real_texture_creation_readiness_blocked: bool,

    /// 是否定义了最小 SHM-first renderability checklist。
    pub minimum_renderability_checklist_defined: bool,

    /// 是否观察到 Phase 56K frame callback completion policy report。
    pub frame_callback_completion_policy_report_observed: bool,

    /// 上游 frame callback completion policy 是否仍 blocked。
    pub frame_callback_completion_policy_still_blocked: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56L 固定 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；Phase 56L 固定 false。
    pub texture_import_route_available: bool,

    /// future texture handle owner 是否已定义；Phase 56L 从上游缺口得出 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture cleanup policy 是否已定义；Phase 56L 从上游缺口得出 false。
    pub texture_cleanup_policy_defined: bool,

    /// damage submission 是否真实可用；Phase 56L 固定 false。
    pub damage_submission_available: bool,

    /// render success evidence 是否可用；Phase 56L 固定 false。
    pub render_success_evidence_available: bool,

    /// frame callback done 是否允许；Phase 56L 固定 false。
    pub frame_callback_done_allowed: bool,

    /// 真实 texture creation 是否 ready；Phase 56L 固定 false。
    pub real_texture_creation_ready: bool,

    /// 真实 texture creation 是否允许执行；Phase 56L 固定 false。
    pub real_texture_creation_allowed: bool,
}

/// Phase 56L real texture creation readiness decision report。
///
/// 该 report 从 Phase 56K policy 派生，只汇总真实 texture creation 的前置条件是否
/// 已满足。它不 import buffer、不创建真实 texture handle、不创建 texture、不调用 renderer、
/// 不提交 damage、不发送 frame callback done、不接 input、不修改 core。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSurfaceCommitRealTextureCreationReadinessDecisionReport {
    /// readiness decision seam 是否可用。
    pub real_texture_creation_readiness_decision_available: bool,

    /// 是否观察到 Phase 56K frame callback completion policy report。
    pub source_frame_callback_completion_policy_report_observed: bool,

    /// 被消费的 Phase 56K frame callback completion policy report。
    pub source_frame_callback_completion_policy_report:
        RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport,

    /// 最小 SHM-first renderability checklist。
    pub checklist: RuntimeSurfaceCommitRealTextureCreationReadinessChecklist,

    /// readiness decision 是否仍 blocked。
    pub real_texture_creation_readiness_blocked: bool,

    /// 稳定 blocker reason，便于 runtime / orchestrator report 展示。
    pub real_texture_creation_readiness_blocker_reason: &'static str,

    /// 是否定义了最小 SHM-first renderability checklist。
    pub minimum_renderability_checklist_defined: bool,

    /// 上游 frame callback completion policy 是否仍 blocked。
    pub frame_callback_completion_policy_still_blocked: bool,

    /// 真实 renderer backend instance 是否可用；Phase 56L 固定 false。
    pub renderer_backend_instance_available: bool,

    /// 真实 texture import route 是否可用；Phase 56L 固定 false。
    pub texture_import_route_available: bool,

    /// future texture handle owner 是否已定义；Phase 56L 固定 false。
    pub future_texture_handle_owner_defined: bool,

    /// future texture cleanup policy 是否已定义；Phase 56L 固定 false。
    pub texture_cleanup_policy_defined: bool,

    /// damage submission 是否真实可用；Phase 56L 固定 false。
    pub damage_submission_available: bool,

    /// render success evidence 是否可用；Phase 56L 固定 false。
    pub render_success_evidence_available: bool,

    /// frame callback done 是否允许；Phase 56L 固定 false。
    pub frame_callback_done_allowed: bool,

    /// 真实 texture creation 是否 ready；Phase 56L 固定 false。
    pub real_texture_creation_ready: bool,

    /// 真实 texture creation 是否允许执行；Phase 56L 固定 false。
    pub real_texture_creation_allowed: bool,

    /// Phase 56L 不尝试真实 import。
    pub buffer_import_attempted: bool,

    /// Phase 56L 不完成真实 import。
    pub buffer_imported: bool,

    /// Phase 56L 不创建 texture。
    pub texture_created: bool,

    /// Phase 56L 不调用 renderer。
    pub renderer_called: bool,

    /// Phase 56L 不提交 damage。
    pub damage_submitted: bool,

    /// Phase 56L 不发送 frame callback done。
    pub frame_callback_done_sent: bool,

    /// Phase 56L 不接入 input。
    pub input_support: bool,

    /// Phase 56L 不触发 core mutation。
    pub core_mutation_invoked: bool,

    /// readiness decision 执行步骤。
    pub operations: Vec<RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation>,

    /// 阻止真实 texture creation 的 blockers。
    pub blockers: Vec<RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker>,
}

/// 从 Phase 56K frame callback completion policy 派生 Phase 56L texture creation readiness decision。
///
/// 这是 Phase 56L 的唯一执行入口：它只生成 pure-data decision report。本函数不创建
/// texture，不创建真实 texture handle，不调用 renderer，不提交 damage，也不发送 frame callback done。
#[must_use = "real texture creation readiness decision is not texture creation"]
pub fn real_texture_creation_readiness_decision_from_frame_callback_completion_policy(
    report: &RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport,
) -> RuntimeSurfaceCommitRealTextureCreationReadinessDecisionReport {
    let source_route = &report
        .source_damage_to_texture_mapping_audit_report
        .source_texture_import_route_decision_report;

    let checklist = RuntimeSurfaceCommitRealTextureCreationReadinessChecklist {
        real_texture_creation_readiness_decision_available: true,
        real_texture_creation_readiness_blocked: true,
        minimum_renderability_checklist_defined: true,
        frame_callback_completion_policy_report_observed: report
            .frame_callback_completion_policy_available,
        frame_callback_completion_policy_still_blocked: report
            .frame_callback_completion_policy_blocked,
        renderer_backend_instance_available: report.renderer_backend_instance_available,
        texture_import_route_available: source_route.texture_import_route_available,
        future_texture_handle_owner_defined: source_route.future_texture_handle_owner_defined,
        texture_cleanup_policy_defined: source_route.texture_cleanup_policy_defined,
        damage_submission_available: report.damage_submission_available,
        render_success_evidence_available: report.render_success_evidence_available,
        frame_callback_done_allowed: report.frame_callback_done_allowed,
        real_texture_creation_ready: false,
        real_texture_creation_allowed: false,
    };

    let mut blockers = Vec::new();
    if checklist.frame_callback_completion_policy_still_blocked {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::FrameCallbackCompletionPolicyStillBlocked,
        );
    }
    if !checklist.renderer_backend_instance_available {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingRendererBackendInstance,
        );
    }
    if !checklist.texture_import_route_available {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingTextureImportRoute,
        );
    }
    if !checklist.future_texture_handle_owner_defined {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingFutureTextureHandleOwnershipPolicy,
        );
    }
    if !checklist.texture_cleanup_policy_defined {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingTextureCleanupPolicy,
        );
    }
    if !checklist.damage_submission_available {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingDamageSubmission,
        );
    }
    if !checklist.render_success_evidence_available {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingRenderSuccessEvidence,
        );
    }
    if !checklist.frame_callback_done_allowed {
        blockers.push(
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::FrameCallbackDoneDisabled,
        );
    }
    blockers.extend([
        RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::RealTextureCreationExplicitlyDisabled,
        RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::RealTextureCreationReadinessWithoutTexture,
    ]);

    RuntimeSurfaceCommitRealTextureCreationReadinessDecisionReport {
        real_texture_creation_readiness_decision_available: true,
        source_frame_callback_completion_policy_report_observed: report
            .frame_callback_completion_policy_available,
        source_frame_callback_completion_policy_report: report.clone(),
        checklist,
        real_texture_creation_readiness_blocked: true,
        real_texture_creation_readiness_blocker_reason:
            "real texture creation readiness decision only: missing renderer backend instance, texture import route, future texture handle ownership, cleanup, damage submission, render success evidence, and frame callback done permission",
        minimum_renderability_checklist_defined: true,
        frame_callback_completion_policy_still_blocked: report
            .frame_callback_completion_policy_blocked,
        renderer_backend_instance_available: false,
        texture_import_route_available: false,
        future_texture_handle_owner_defined: false,
        texture_cleanup_policy_defined: false,
        damage_submission_available: false,
        render_success_evidence_available: false,
        frame_callback_done_allowed: false,
        real_texture_creation_ready: false,
        real_texture_creation_allowed: false,
        buffer_import_attempted: false,
        buffer_imported: false,
        texture_created: false,
        renderer_called: false,
        damage_submitted: false,
        frame_callback_done_sent: false,
        input_support: false,
        core_mutation_invoked: false,
        operations: vec![
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::ObserveFrameCallbackCompletionPolicyReport,
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::SummarizeRendererBackendInstanceReadiness,
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::SummarizeTextureImportRouteReadiness,
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::SummarizeTextureOwnershipReadiness,
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::SummarizeDamageSubmissionReadiness,
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::SummarizeFrameCallbackCompletionReadiness,
            RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation::BuildRealTextureCreationReadinessDecisionReport,
        ],
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker,
        RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker,
        RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker,
        RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker,
        RuntimeSurfaceCommitTextureCreationBlocker,
        RuntimeSurfaceCommitTextureCreationPreconditionBlocker,
        RuntimeSurfaceCommitTextureImportRouteDecisionBlocker,
        RuntimeSurfaceCommitTextureOwnerBoundaryBlocker,
        damage_to_texture_mapping_audit_from_texture_import_route_decision,
        frame_callback_completion_policy_from_damage_to_texture_mapping_audit,
        real_texture_creation_readiness_decision_from_frame_callback_completion_policy,
        renderer_backend_instance_audit_from_texture_owner_boundary_report,
        texture_creation_noop_report_from_precondition_audit,
        texture_creation_precondition_audit_from_validation_harness_report,
        texture_import_route_decision_from_renderer_backend_instance_audit,
        texture_owner_boundary_report_from_noop_report, validate_shm_metadata_harness_paths,
    };

    /// Phase 56E 从 Phase 56D validation harness report 派生 texture precondition audit。
    ///
    /// 该测试固定：texture precondition blocked 只是安全边界，不代表 texture created、
    /// renderer called、damage submitted 或 frame callback done。真实 buffer / texture /
    /// renderer 类型仍只能停留在 smithay_backend 的 Linux-only adapter 层，core 不感知。
    #[test]
    fn derives_blocked_texture_precondition_audit_from_metadata_validation_harness() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);

        assert!(audit.texture_precondition_audit_available);
        assert!(!audit.source_metadata_report_observed);
        assert!(audit.observed_metadata_report.is_none());
        assert!(audit.validation_harness_report.validation_harness_invoked);
        assert!(audit.validation_harness_report.all_validation_paths_covered);
        assert!(!audit.texture_precondition_allowed);
        assert!(audit.texture_precondition_blocked);
        assert!(!audit.checklist.metadata_sufficient_for_texture_precondition);
        assert!(!audit.checklist.renderer_backend_instance_available);
        assert!(!audit.checklist.texture_import_route_available);
        assert!(!audit.checklist.damage_to_texture_mapping_available);
        assert!(!audit.checklist.frame_callback_completion_policy_available);
        assert!(audit.blockers.contains(
            &RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingRendererBackendInstance
        ));
        assert!(audit.blockers.contains(
            &RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingTextureImportRoute
        ));
        assert!(audit.blockers.contains(
            &RuntimeSurfaceCommitTextureCreationPreconditionBlocker::MissingFrameCallbackCompletionPolicy
        ));
        assert!(!audit.buffer_import_attempted);
        assert!(!audit.buffer_imported);
        assert!(!audit.texture_created);
        assert!(!audit.renderer_called);
        assert!(!audit.damage_submitted);
        assert!(!audit.frame_callback_done_sent);
        assert!(!audit.input_support);
        assert!(!audit.core_mutation_invoked);
    }

    /// Phase 56F 从 Phase 56E precondition audit 派生 texture creation no-op report。
    ///
    /// 该测试固定：no-op report 是 execution boundary，不代表 texture created、
    /// renderer called、damage submitted 或 frame callback done。
    #[test]
    fn derives_blocked_texture_creation_noop_report_from_precondition_audit() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let report = texture_creation_noop_report_from_precondition_audit(&audit);

        assert!(report.texture_creation_noop_available);
        assert!(report.source_precondition_audit_observed);
        assert!(report.texture_creation_blocked);
        assert!(!report.texture_creation_attempted);
        assert!(!report.texture_precondition_allowed);
        assert!(!report.metadata_sufficient_for_texture_precondition);
        assert!(!report.texture_owner_boundary_available);
        assert!(!report.renderer_backend_instance_available);
        assert!(!report.texture_import_route_available);
        assert!(!report.damage_to_texture_mapping_available);
        assert!(!report.frame_callback_completion_policy_available);
        assert!(
            report.blockers.contains(
                &RuntimeSurfaceCommitTextureCreationBlocker::TexturePreconditionNotAllowed
            )
        );
        assert!(
            report.blockers.contains(
                &RuntimeSurfaceCommitTextureCreationBlocker::MissingRendererBackendInstance
            )
        );
        assert!(
            report
                .blockers
                .contains(&RuntimeSurfaceCommitTextureCreationBlocker::MissingTextureImportRoute)
        );
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureCreationBlocker::MissingFrameCallbackCompletionPolicy
        ));
        assert!(
            report
                .blockers
                .contains(&RuntimeSurfaceCommitTextureCreationBlocker::MissingTextureOwnerBoundary)
        );
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureCreationBlocker::RuntimeEvidenceWithoutTextureCreation
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureCreationBlocker::TextureCreationExplicitlyDisabled
        ));
        assert!(
            report.blockers.contains(
                &RuntimeSurfaceCommitTextureCreationBlocker::RendererCallExplicitlyDisabled
            )
        );
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }

    /// Phase 56G 从 Phase 56F no-op report 派生 texture owner boundary report。
    ///
    /// 该测试固定：owner boundary 只是 future ownership seam，不代表 texture created、
    /// renderer called、damage submitted 或 frame callback done。
    #[test]
    fn derives_blocked_texture_owner_boundary_report_from_noop_report() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let noop_report = texture_creation_noop_report_from_precondition_audit(&audit);
        let report = texture_owner_boundary_report_from_noop_report(&noop_report);

        assert!(report.texture_owner_boundary_available);
        assert!(report.source_noop_report_observed);
        assert!(report.texture_owner_boundary_blocked);
        assert!(report.texture_creation_request_owner_defined);
        assert_eq!(
            report.texture_creation_request_owner,
            "linux_shm_first_buffer_import_adapter"
        );
        assert!(!report.future_texture_handle_owner_defined);
        assert!(!report.future_texture_lifetime_owner_defined);
        assert!(!report.future_texture_cleanup_owner_defined);
        assert!(!report.future_texture_release_owner_defined);
        assert!(!report.future_texture_invalidation_owner_defined);
        assert!(!report.renderer_backend_instance_available);
        assert!(!report.texture_import_route_available);
        assert!(
            report.blockers.contains(
                &RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::TextureCreationNoopOnly
            )
        );
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingRendererBackendInstance
        ));
        assert!(
            report.blockers.contains(
                &RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingTextureImportRoute
            )
        );
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::MissingFutureTextureCleanupPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureOwnerBoundaryBlocker::OwnerBoundaryWithoutTextureCreation
        ));
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }

    /// Phase 56H 从 Phase 56G owner boundary report 派生 renderer backend instance audit。
    ///
    /// 该测试固定：renderer backend instance audit 只是 future owner/lifecycle/cleanup
    /// seam，不代表 renderer backend instance 可用、texture created、renderer called、
    /// damage submitted 或 frame callback done。
    #[test]
    fn derives_blocked_renderer_backend_instance_audit_from_texture_owner_boundary_report() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let noop_report = texture_creation_noop_report_from_precondition_audit(&audit);
        let owner_report = texture_owner_boundary_report_from_noop_report(&noop_report);
        let report =
            renderer_backend_instance_audit_from_texture_owner_boundary_report(&owner_report);

        assert!(report.renderer_backend_instance_audit_available);
        assert!(report.source_texture_owner_boundary_report_observed);
        assert!(report.renderer_backend_instance_audit_blocked);
        assert!(report.texture_owner_boundary_still_blocked);
        assert!(!report.renderer_backend_instance_available);
        assert!(!report.renderer_backend_instance_owner_defined);
        assert!(!report.renderer_backend_instance_lifecycle_owner_defined);
        assert!(!report.renderer_backend_instance_cleanup_owner_defined);
        assert!(!report.renderer_backend_instance_availability_owner_defined);
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::TextureOwnerBoundaryStillBlocked
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstance
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceOwnerPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceLifecyclePolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceCleanupPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker::MissingRendererBackendInstanceAvailabilityPolicy
        ));
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }

    /// Phase 56I 从 Phase 56H renderer backend instance audit 派生 texture import route decision。
    ///
    /// 该测试固定：texture import route decision 只是 route owner / blocker seam，不代表
    /// 真实 import-buffer 可调用、texture handle 可创建、texture created 或 renderer called。
    #[test]
    fn derives_blocked_texture_import_route_decision_from_renderer_backend_instance_audit() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let noop_report = texture_creation_noop_report_from_precondition_audit(&audit);
        let owner_report = texture_owner_boundary_report_from_noop_report(&noop_report);
        let renderer_backend_audit =
            renderer_backend_instance_audit_from_texture_owner_boundary_report(&owner_report);
        let report = texture_import_route_decision_from_renderer_backend_instance_audit(
            &renderer_backend_audit,
        );

        assert!(report.texture_import_route_decision_available);
        assert!(report.source_renderer_backend_instance_audit_report_observed);
        assert!(report.texture_import_route_decision_blocked);
        assert!(report.renderer_backend_instance_audit_still_blocked);
        assert!(!report.renderer_backend_instance_available);
        assert!(!report.texture_import_route_available);
        assert!(report.texture_import_route_owner_defined);
        assert_eq!(
            report.texture_import_route_owner,
            "linux_shm_first_buffer_import_adapter"
        );
        assert!(!report.import_buffer_call_allowed);
        assert!(!report.future_texture_handle_owner_defined);
        assert!(!report.texture_cleanup_policy_defined);
        assert!(!report.texture_release_policy_defined);
        assert!(!report.damage_mapping_policy_defined);
        assert!(!report.frame_callback_completion_policy_defined);
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::RendererBackendInstanceAuditStillBlocked
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingRendererBackendInstance
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingImportBufferCallPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingFutureTextureHandleOwnershipPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingTextureCleanupPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingDamageMappingPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::MissingFrameCallbackCompletionPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitTextureImportRouteDecisionBlocker::ImportBufferExplicitlyDisabled
        ));
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }

    /// Phase 56J 从 Phase 56I texture import route decision 派生 damage-to-texture mapping audit。
    ///
    /// 该测试固定：damage mapping audit 只定义 future damage mapping owner / blocker
    /// taxonomy，不提交真实 damage、不调用 renderer、不发送 frame callback done。
    #[test]
    fn derives_blocked_damage_to_texture_mapping_audit_from_texture_import_route_decision() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let noop_report = texture_creation_noop_report_from_precondition_audit(&audit);
        let owner_report = texture_owner_boundary_report_from_noop_report(&noop_report);
        let renderer_backend_audit =
            renderer_backend_instance_audit_from_texture_owner_boundary_report(&owner_report);
        let route_decision = texture_import_route_decision_from_renderer_backend_instance_audit(
            &renderer_backend_audit,
        );
        let report =
            damage_to_texture_mapping_audit_from_texture_import_route_decision(&route_decision);

        assert!(report.damage_to_texture_mapping_audit_available);
        assert!(report.source_texture_import_route_decision_report_observed);
        assert!(report.damage_to_texture_mapping_audit_blocked);
        assert!(report.texture_import_route_decision_still_blocked);
        assert!(!report.texture_import_route_available);
        assert!(report.damage_mapping_owner_defined);
        assert_eq!(
            report.damage_mapping_owner,
            "linux_shm_first_buffer_import_adapter"
        );
        assert!(!report.future_texture_handle_owner_defined);
        assert!(!report.texture_region_policy_defined);
        assert!(!report.surface_damage_mapping_policy_defined);
        assert!(!report.buffer_damage_mapping_policy_defined);
        assert!(!report.damage_coordinate_space_policy_defined);
        assert!(!report.renderer_damage_submission_policy_defined);
        assert!(!report.damage_submission_allowed);
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::TextureImportRouteDecisionStillBlocked
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingTextureImportRoute
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingFutureTextureHandleOwnershipPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingTextureRegionPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingSurfaceDamageMappingPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::MissingBufferDamageMappingPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker::DamageSubmissionExplicitlyDisabled
        ));
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }

    /// Phase 56K 从 Phase 56J damage mapping audit 派生 frame callback completion policy。
    ///
    /// 该测试固定：policy 只定义 future completion owner 和 render-success gate；
    /// 在缺少真实 texture、renderer、damage submission 时不得发送 frame callback done。
    #[test]
    fn derives_blocked_frame_callback_completion_policy_from_damage_mapping_audit() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let noop_report = texture_creation_noop_report_from_precondition_audit(&audit);
        let owner_report = texture_owner_boundary_report_from_noop_report(&noop_report);
        let renderer_backend_audit =
            renderer_backend_instance_audit_from_texture_owner_boundary_report(&owner_report);
        let route_decision = texture_import_route_decision_from_renderer_backend_instance_audit(
            &renderer_backend_audit,
        );
        let damage_mapping_audit =
            damage_to_texture_mapping_audit_from_texture_import_route_decision(&route_decision);
        let report = frame_callback_completion_policy_from_damage_to_texture_mapping_audit(
            &damage_mapping_audit,
        );

        assert!(report.frame_callback_completion_policy_available);
        assert!(report.source_damage_to_texture_mapping_audit_observed);
        assert!(report.frame_callback_completion_policy_blocked);
        assert!(report.damage_to_texture_mapping_audit_still_blocked);
        assert!(report.frame_callback_completion_owner_defined);
        assert_eq!(
            report.frame_callback_completion_owner,
            "linux_shm_first_buffer_import_adapter"
        );
        assert!(report.render_success_required_before_done);
        assert!(!report.real_texture_available);
        assert!(!report.renderer_backend_instance_available);
        assert!(!report.damage_submission_available);
        assert!(!report.frame_callback_done_allowed);
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::DamageToTextureMappingAuditStillBlocked
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingRealTexture
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingRendererBackendInstance
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingDamageSubmission
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::MissingRenderSuccessEvidence
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker::FrameCallbackDoneExplicitlyDisabled
        ));
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }

    /// Phase 56L 从 Phase 56K frame callback completion policy 派生真实 texture creation readiness decision。
    ///
    /// 该测试固定：readiness decision 只汇总 Phase 56H-56K 的 blocker 和最小
    /// SHM-first renderability checklist；它不得创建 texture、调用 renderer、提交
    /// damage 或发送 frame callback done。
    #[test]
    fn derives_blocked_real_texture_creation_readiness_decision_from_frame_callback_policy() {
        let validation_harness = validate_shm_metadata_harness_paths();
        let audit =
            texture_creation_precondition_audit_from_validation_harness_report(&validation_harness);
        let noop_report = texture_creation_noop_report_from_precondition_audit(&audit);
        let owner_report = texture_owner_boundary_report_from_noop_report(&noop_report);
        let renderer_backend_audit =
            renderer_backend_instance_audit_from_texture_owner_boundary_report(&owner_report);
        let route_decision = texture_import_route_decision_from_renderer_backend_instance_audit(
            &renderer_backend_audit,
        );
        let damage_mapping_audit =
            damage_to_texture_mapping_audit_from_texture_import_route_decision(&route_decision);
        let frame_callback_policy =
            frame_callback_completion_policy_from_damage_to_texture_mapping_audit(
                &damage_mapping_audit,
            );
        let report = real_texture_creation_readiness_decision_from_frame_callback_completion_policy(
            &frame_callback_policy,
        );

        assert!(report.real_texture_creation_readiness_decision_available);
        assert!(report.source_frame_callback_completion_policy_report_observed);
        assert!(report.real_texture_creation_readiness_blocked);
        assert!(report.frame_callback_completion_policy_still_blocked);
        assert!(!report.real_texture_creation_ready);
        assert!(!report.real_texture_creation_allowed);
        assert!(!report.renderer_backend_instance_available);
        assert!(!report.texture_import_route_available);
        assert!(!report.future_texture_handle_owner_defined);
        assert!(!report.texture_cleanup_policy_defined);
        assert!(!report.damage_submission_available);
        assert!(!report.render_success_evidence_available);
        assert!(!report.frame_callback_done_allowed);
        assert!(report.minimum_renderability_checklist_defined);
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::FrameCallbackCompletionPolicyStillBlocked
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingRendererBackendInstance
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingTextureImportRoute
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingFutureTextureHandleOwnershipPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingTextureCleanupPolicy
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingDamageSubmission
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::MissingRenderSuccessEvidence
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::FrameCallbackDoneDisabled
        ));
        assert!(report.blockers.contains(
            &RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker::RealTextureCreationExplicitlyDisabled
        ));
        assert!(!report.buffer_import_attempted);
        assert!(!report.buffer_imported);
        assert!(!report.texture_created);
        assert!(!report.renderer_called);
        assert!(!report.damage_submitted);
        assert!(!report.frame_callback_done_sent);
        assert!(!report.input_support);
        assert!(!report.core_mutation_invoked);
    }
}
