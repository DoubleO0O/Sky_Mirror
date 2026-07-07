# Phase 56J - Damage-to-Texture Mapping Audit

## Authorization

Phase 56J is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56I.

This phase establishes a damage-to-texture mapping audit as pure data only. It
does not wait for separate damage, renderer, texture, frame callback, input, or
DRM/GBM authorization.

## Phase 56I to Phase 56J Relationship

Phase 56I produced a texture import route decision report from the Phase 56H
renderer backend instance audit. Phase 56J consumes that texture import route
decision report and derives a damage-to-texture mapping audit report.

The damage-to-texture mapping audit defines the future owner for mapping commit
damage, surface damage, and buffer damage into a future texture region. It does
not create a texture, call a renderer, submit damage, send frame callback done,
connect input, mutate core, create a texture handle, or execute a buffer import
route.

## Implementation Content

Phase 56J adds:

- `RuntimeSurfaceCommitDamageToTextureMappingAuditOperation`
- `RuntimeSurfaceCommitDamageToTextureMappingAuditBlocker`
- `RuntimeSurfaceCommitDamageToTextureMappingPolicy`
- `RuntimeSurfaceCommitDamageToTextureMappingChecklist`
- `RuntimeSurfaceCommitDamageToTextureMappingAuditReport`
- `damage_to_texture_mapping_audit_from_texture_import_route_decision`

The report is FIFO-preserving because each pump produces one damage mapping
audit report from the corresponding Phase 56I texture import route decision
report.

## Capability Truth

These values remain false in Phase 56J:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56J also records:

- damage_to_texture_mapping_audit_available = true
- damage_to_texture_mapping_audit_blocked = true
- damage_mapping_owner_defined = true
- texture_import_route_available = false
- future_texture_handle_owner_defined = false
- texture_region_policy_defined = false
- surface_damage_mapping_policy_defined = false
- buffer_damage_mapping_policy_defined = false
- damage_coordinate_space_policy_defined = false
- renderer_damage_submission_policy_defined = false
- damage_submission_allowed = false

These values are audit and blocker evidence. They are not real damage
submission capability, not renderer capability, and not renderable window
capability.

## Blocker Taxonomy

Phase 56J uses the following blocker taxonomy:

- texture_import_route_decision_still_blocked
- missing_texture_import_route
- missing_future_texture_handle_ownership_policy
- missing_texture_region_policy
- missing_surface_damage_mapping_policy
- missing_buffer_damage_mapping_policy
- missing_damage_coordinate_space_policy
- missing_renderer_damage_submission_policy
- missing_frame_callback_completion_policy
- damage_submission_explicitly_disabled
- damage_mapping_without_texture

The default blocker reason is that the route still lacks a real texture import
route, future texture handle ownership, future texture region policy, surface
and buffer damage mapping policies, coordinate-space conversion policy, renderer
damage submission policy, and frame callback completion policy.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- damage_to_texture_mapping_audit_invocations
- damage_to_texture_mapping_audit_reports
- damage_to_texture_mapping_audit_available
- damage_to_texture_mapping_audit_blocked
- damage_mapping_texture_import_route_decision_report_observed
- damage_mapping_texture_import_route_decision_still_blocked
- damage_mapping_texture_import_route_available
- damage_mapping_future_texture_handle_owner_defined
- damage_mapping_owner_defined
- damage_mapping_texture_region_policy_defined
- damage_mapping_surface_damage_mapping_policy_defined
- damage_mapping_buffer_damage_mapping_policy_defined
- damage_mapping_coordinate_space_policy_defined
- damage_mapping_renderer_damage_submission_policy_defined
- damage_mapping_frame_callback_completion_policy_defined
- damage_submission_allowed
- damage_mapping_missing_texture_import_route
- damage_mapping_missing_future_texture_handle_ownership_policy
- damage_mapping_missing_texture_region_policy
- damage_mapping_missing_surface_damage_mapping_policy
- damage_mapping_missing_buffer_damage_mapping_policy
- damage_mapping_missing_coordinate_space_policy
- damage_mapping_missing_renderer_damage_submission_policy
- damage_mapping_missing_frame_callback_completion_policy
- damage_mapping_damage_submission_explicitly_disabled
- damage_mapping_without_texture
- damage_mapping_buffer_import_attempted
- damage_mapping_buffer_imported
- damage_mapping_texture_created
- damage_mapping_renderer_called
- damage_mapping_damage_submitted
- damage_mapping_frame_callback_done_sent
- damage_mapping_input_support
- damage_mapping_core_mutation_invoked

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
`BufferData`, texture, renderer, EGL, GLES, WGPU, DRM, GBM, dmabuf, or real
renderer damage submission types. Core remains limited to abstract concepts such
as `WindowId`, `Geometry`, `State`, `Layout`, `Action`, and `Command`.

## Test Strategy

Phase 56J is covered by:

- a unit test deriving a blocked damage-to-texture mapping audit report from a
  Phase 56I texture import route decision report;
- a source-contract test proving the document, adapter report, runtime loop,
  and orchestrator summary expose the audit while keeping capability truth
  false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- A damage mapping owner can be mistaken for real damage submission capability.
  Capability truth fields prevent that.
- A pure-data audit can be mistaken for bounded loop progress. The runtime
  summary exposes the report but does not include it in progress detection.
- Surface damage and buffer damage use different coordinate spaces. Phase 56J
  records the missing coordinate-space policy instead of guessing.

## Phase 56K Suggestions

Phase 56K should consume the Phase 56J damage-to-texture mapping audit report
and define the frame callback completion policy:

1. define the owner responsible for pending frame callback completion;
2. define the rule that callbacks are completed only after real render success;
3. keep frame callback done disabled while real texture, renderer, and damage
   submission paths are missing;
4. continue as pure data unless all preconditions are proven.

## Stop Condition

Phase 56J stops after the damage-to-texture mapping audit report is exposed
through runtime / bounded loop / orchestrator summaries and CI is green. In
No-Brake Goal Mode, continue directly into Phase 56K after merge and green main
CI.
