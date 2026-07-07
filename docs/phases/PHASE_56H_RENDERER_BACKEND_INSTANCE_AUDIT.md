# Phase 56H - Renderer Backend Instance Audit

## Authorization

Phase 56H is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56G.

This phase establishes a renderer backend instance audit as pure data only. It
does not wait for separate renderer, texture, damage, frame callback, input, or
DRM/GBM authorization.

## Phase 56G to Phase 56H Relationship

Phase 56G produced a texture owner boundary report from the Phase 56F texture
creation no-op report. Phase 56H consumes that texture owner boundary report
and derives a renderer backend instance audit report.

The renderer backend instance audit clarifies whether a real renderer backend
instance exists and records the missing owner / lifecycle / cleanup /
availability policies. It does not create a renderer backend instance, create a
texture, call a renderer, submit damage, send frame callback done, connect
input, mutate core, create `TextureId`, or execute `ImportAll::import_buffer`.

## Implementation Content

Phase 56H adds:

- `RuntimeSurfaceCommitRendererBackendInstanceAuditOperation`
- `RuntimeSurfaceCommitRendererBackendInstanceAuditBlocker`
- `RuntimeSurfaceCommitRendererBackendInstancePolicy`
- `RuntimeSurfaceCommitRendererBackendInstanceAuditChecklist`
- `RuntimeSurfaceCommitRendererBackendInstanceAuditReport`
- `renderer_backend_instance_audit_from_texture_owner_boundary_report`

The report is FIFO-preserving because each pump produces one renderer backend
instance audit report from the corresponding Phase 56G texture owner boundary
report.

## Capability Truth

These values remain false in Phase 56H:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56H also records:

- renderer_backend_instance_audit_available = true
- renderer_backend_instance_audit_blocked = true
- texture_owner_boundary_still_blocked = true
- renderer_backend_instance_available = false
- renderer_backend_instance_owner_defined = false
- renderer_backend_instance_lifecycle_owner_defined = false
- renderer_backend_instance_cleanup_owner_defined = false
- renderer_backend_instance_availability_owner_defined = false

These values are audit and blocker evidence. They are not real renderer
capability, not texture capability, and not renderable window capability.

## Blocker Taxonomy

Phase 56H uses the following blocker taxonomy:

- texture_owner_boundary_still_blocked
- missing_renderer_backend_instance
- missing_renderer_backend_instance_owner_policy
- missing_renderer_backend_instance_lifecycle_policy
- missing_renderer_backend_instance_cleanup_policy
- missing_renderer_backend_instance_availability_policy
- renderer_backend_instance_without_texture_creation

The default blocker reason is that the route still lacks a real renderer
backend instance and the owner, lifecycle, cleanup, and availability policies
needed to make that instance safe to use.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- renderer_backend_instance_audit_invocations
- renderer_backend_instance_audit_reports
- renderer_backend_instance_audit_available
- renderer_backend_instance_audit_blocked
- renderer_backend_instance_texture_owner_boundary_report_observed
- renderer_backend_instance_texture_owner_boundary_still_blocked
- renderer_backend_instance_available
- renderer_backend_instance_owner_defined
- renderer_backend_instance_lifecycle_owner_defined
- renderer_backend_instance_cleanup_owner_defined
- renderer_backend_instance_availability_owner_defined
- renderer_backend_instance_missing_renderer_backend_instance
- renderer_backend_instance_missing_owner_policy
- renderer_backend_instance_missing_lifecycle_policy
- renderer_backend_instance_missing_cleanup_policy
- renderer_backend_instance_missing_availability_policy
- renderer_backend_instance_without_texture_creation
- renderer_backend_instance_buffer_import_attempted
- renderer_backend_instance_buffer_imported
- renderer_backend_instance_texture_created
- renderer_backend_instance_renderer_called
- renderer_backend_instance_damage_submitted
- renderer_backend_instance_frame_callback_done_sent
- renderer_backend_instance_input_support
- renderer_backend_instance_core_mutation_invoked

The report is not bounded loop progress. It is visibility for a blocked seam.

## Feature Gate

The implementation remains in the existing `smithay-linux` and Linux-target
Smithay backend path. Default and `smithay-probe` builds continue to avoid real
Linux renderer resources.

No new dependency is added in this phase.

## Smithay / Wayland / Linux Type Boundary

Real Smithay / Wayland buffer and SHM metadata type boundaries remain
restricted to `src/smithay_backend/linux_shm_buffer_import_adapter.rs` and
related Linux-only smithay_backend glue.

Core must not import or name Smithay, Wayland, `wl_buffer`, `WlBuffer`,
`BufferData`, texture, renderer, EGL, GLES, WGPU, DRM, GBM, dmabuf,
`ImportAll::import_buffer`, or `TextureId` types. Core remains limited to
abstract concepts such as `WindowId`, `Geometry`, `State`, `Layout`, `Action`,
and `Command`.

## Test Strategy

Phase 56H is covered by:

- a unit test deriving a blocked renderer backend instance audit report from a
  Phase 56G texture owner boundary report;
- a source-contract test proving the document, adapter report, runtime loop,
  and orchestrator summary expose the audit while keeping capability truth
  false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- A renderer backend instance audit can be mistaken for real renderer backend
  availability. Capability truth fields prevent that.
- A pure-data audit can be mistaken for bounded loop progress. The runtime
  summary exposes the report but does not include it in progress detection.
- Existing older renderer backend owner shell reports do not mean a real
  renderer backend instance exists for texture import.

## Phase 56I Suggestions

Phase 56I should consume the Phase 56H renderer backend instance audit report
and define the texture import route decision / blocker taxonomy:

1. state that `ImportAll::import_buffer` is the real WlBuffer to renderer
   texture route;
2. define the texture import route owner;
3. keep `import_buffer` disabled while renderer backend instance, TextureId
   ownership, cleanup / release, damage mapping, and frame callback policy are
   missing;
4. continue as pure data unless all preconditions are proven.

## Stop Condition

Phase 56H stops after the renderer backend instance audit report is exposed
through runtime / bounded loop / orchestrator summaries and CI is green. In
No-Brake Goal Mode, continue directly into Phase 56I after merge and green main
CI.
