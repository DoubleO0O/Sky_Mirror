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
