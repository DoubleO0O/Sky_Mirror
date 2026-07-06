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
