# Phase 56I - Texture Import Route Decision

## Authorization

Phase 56I is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56H.

This phase establishes a texture import route decision as pure data only. It does
not wait for separate renderer, texture, damage, frame callback, input, or
DRM/GBM authorization.

## Phase 56H to Phase 56I Relationship

Phase 56H produced a renderer backend instance audit report from the Phase 56G
texture owner boundary report. Phase 56I consumes that renderer backend instance
audit report and derives a texture import route decision report.

The texture import route decision defines the future owner for the WlBuffer to
renderer texture route and records the missing import call, TextureId ownership,
cleanup, release, damage mapping, and frame callback policies. It does not create
a renderer backend instance, create a texture, call a renderer, submit damage,
send frame callback done, connect input, mutate core, create `TextureId`, or
execute `ImportAll::import_buffer`.

## Implementation Content

Phase 56I adds:

- `RuntimeSurfaceCommitTextureImportRouteDecisionOperation`
- `RuntimeSurfaceCommitTextureImportRouteDecisionBlocker`
- `RuntimeSurfaceCommitTextureImportRoutePolicy`
- `RuntimeSurfaceCommitTextureImportRouteDecisionChecklist`
- `RuntimeSurfaceCommitTextureImportRouteDecisionReport`
- `texture_import_route_decision_from_renderer_backend_instance_audit`

The report is FIFO-preserving because each pump produces one texture import route
decision report from the corresponding Phase 56H renderer backend instance audit
report.

## Capability Truth

These values remain false in Phase 56I:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56I also records:

- texture_import_route_decision_available = true
- texture_import_route_decision_blocked = true
- texture_import_route_available = false
- texture_import_route_owner_defined = true
- import_buffer_call_allowed = false
- texture_id_owner_defined = false
- texture_cleanup_policy_defined = false
- texture_release_policy_defined = false
- damage_mapping_policy_defined = false
- frame_callback_completion_policy_defined = false

These values are route decision and blocker evidence. They are not real import
capability, not texture capability, and not renderable window capability.

## Blocker Taxonomy

Phase 56I uses the following blocker taxonomy:

- renderer_backend_instance_audit_still_blocked
- missing_renderer_backend_instance
- missing_import_buffer_call_policy
- missing_texture_id_ownership_policy
- missing_texture_cleanup_policy
- missing_texture_release_policy
- missing_damage_mapping_policy
- missing_frame_callback_completion_policy
- import_buffer_explicitly_disabled
- texture_import_route_decision_without_import

The default blocker reason is that the route still lacks a real renderer backend
instance, import_buffer call policy, TextureId ownership, cleanup, release,
damage mapping, and frame callback completion policy.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- texture_import_route_decision_invocations
- texture_import_route_decision_reports
- texture_import_route_decision_available
- texture_import_route_decision_blocked
- texture_import_route_renderer_backend_instance_audit_report_observed
- texture_import_route_renderer_backend_instance_audit_still_blocked
- texture_import_route_renderer_backend_instance_available
- texture_import_route_available
- texture_import_route_owner_defined
- texture_import_route_import_buffer_call_allowed
- texture_import_route_texture_id_owner_defined
- texture_import_route_texture_cleanup_policy_defined
- texture_import_route_texture_release_policy_defined
- texture_import_route_damage_mapping_policy_defined
- texture_import_route_frame_callback_completion_policy_defined
- texture_import_route_missing_renderer_backend_instance
- texture_import_route_missing_import_buffer_call_policy
- texture_import_route_missing_texture_id_ownership_policy
- texture_import_route_missing_texture_cleanup_policy
- texture_import_route_missing_texture_release_policy
- texture_import_route_missing_damage_mapping_policy
- texture_import_route_missing_frame_callback_completion_policy
- texture_import_route_import_buffer_explicitly_disabled
- texture_import_route_decision_without_import
- texture_import_route_buffer_import_attempted
- texture_import_route_buffer_imported
- texture_import_route_texture_created
- texture_import_route_renderer_called
- texture_import_route_damage_submitted
- texture_import_route_frame_callback_done_sent
- texture_import_route_input_support
- texture_import_route_core_mutation_invoked

The report is not bounded loop progress. It is visibility for a blocked seam.

## Feature Gate

The implementation remains in the existing `smithay-linux` and Linux-target
Smithay backend path. Default and `smithay-probe` builds continue to avoid real
Linux renderer resources.

No new dependency is added in this phase.

## Smithay / Wayland / Linux Type Boundary

Real Smithay / Wayland buffer and SHM metadata type boundaries remain restricted
to `src/smithay_backend/linux_shm_buffer_import_adapter.rs` and related
Linux-only smithay_backend glue.

Core must not import or name Smithay, Wayland, `wl_buffer`, `WlBuffer`,
`BufferData`, texture, renderer, EGL, GLES, WGPU, DRM, GBM, dmabuf,
`ImportAll::import_buffer`, or `TextureId` types. Core remains limited to
abstract concepts such as `WindowId`, `Geometry`, `State`, `Layout`, `Action`,
and `Command`.

## Test Strategy

Phase 56I is covered by:

- a unit test deriving a blocked texture import route decision report from a
  Phase 56H renderer backend instance audit report;
- a source-contract test proving the document, adapter report, runtime loop,
  and orchestrator summary expose the decision while keeping capability truth
  false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- A texture import route owner can be mistaken for real import route
  availability. Capability truth fields prevent that.
- A pure-data decision can be mistaken for bounded loop progress. The runtime
  summary exposes the report but does not include it in progress detection.
- Naming `ImportAll::import_buffer` and `TextureId` in documentation must remain
  descriptive only; production code must not call or construct either.

## Phase 56J Suggestions

Phase 56J should consume the Phase 56I texture import route decision report and
define the future TextureId ownership / cleanup / release policy:

1. define the owner responsible for future TextureId handles;
2. define cleanup and release ordering for imported textures;
3. keep `ImportAll::import_buffer` disabled while real renderer backend instance,
   damage mapping, and frame callback completion policy are missing;
4. continue as pure data unless all preconditions are proven.

## Stop Condition

Phase 56I stops after the texture import route decision report is exposed through
runtime / bounded loop / orchestrator summaries and CI is green. In No-Brake Goal
Mode, continue directly into Phase 56J after merge and green main CI.
