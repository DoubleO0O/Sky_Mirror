# Phase 56L - Real Texture Creation Readiness Decision

## Authorization

Phase 56L is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56K.

This phase establishes a real texture creation readiness decision as pure data
only. It does not wait for separate texture, renderer, damage, frame callback,
input, or DRM/GBM authorization.

## Phase 56K to Phase 56L Relationship

Phase 56K produced a frame callback completion policy report from the Phase 56J
damage-to-texture mapping audit. Phase 56L consumes that policy and summarizes
the Phase 56H through Phase 56K blockers before any real texture creation can
be attempted.

The readiness decision is not texture creation. It does not import a buffer,
create a `TextureId`, create a texture, call a renderer, submit damage, send
frame callback done, connect input, mutate core, or execute a Smithay
`ImportAll::import_buffer` route.

## Implementation Content

Phase 56L adds:

- `RuntimeSurfaceCommitRealTextureCreationReadinessDecisionOperation`
- `RuntimeSurfaceCommitRealTextureCreationReadinessDecisionBlocker`
- `RuntimeSurfaceCommitRealTextureCreationReadinessChecklist`
- `RuntimeSurfaceCommitRealTextureCreationReadinessDecisionReport`
- `real_texture_creation_readiness_decision_from_frame_callback_completion_policy`

The report is FIFO-preserving because each pump produces one readiness decision
report from the corresponding Phase 56K frame callback completion policy
report.

## Capability Truth

These values remain false in Phase 56L:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56L also records:

- real_texture_creation_readiness_decision_available = true
- real_texture_creation_readiness_blocked = true
- minimum_renderability_checklist_defined = true
- real_texture_creation_ready = false
- real_texture_creation_allowed = false
- renderer_backend_instance_available = false
- texture_import_route_available = false
- future_texture_handle_owner_defined = false
- texture_cleanup_policy_defined = false
- damage_submission_available = false
- render_success_evidence_available = false
- frame_callback_done_allowed = false

These values are readiness and blocker evidence. They are not real texture
capability, not renderer capability, and not renderable window capability.

## Minimum Renderability Checklist

The SHM-first surface remains non-renderable until all of the following are
true in a later phase:

- renderer backend instance is real and available;
- texture import route is real and available;
- future texture handle ownership is defined;
- future texture cleanup policy is defined;
- damage submission path is available;
- render success evidence is available;
- frame callback done is allowed only after render success.

## Blocker Taxonomy

Phase 56L uses the following blocker taxonomy:

- frame_callback_completion_policy_still_blocked
- missing_renderer_backend_instance
- missing_texture_import_route
- missing_future_texture_handle_ownership_policy
- missing_texture_cleanup_policy
- missing_damage_submission
- missing_render_success_evidence
- frame_callback_done_disabled
- real_texture_creation_explicitly_disabled
- real_texture_creation_readiness_without_texture

The default blocker reason is that the route still lacks a renderer backend
instance, texture import route, future texture handle ownership, cleanup policy,
damage submission, render success evidence, and permission to send frame
callback done.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- real_texture_creation_readiness_decision_invocations
- real_texture_creation_readiness_decision_reports
- real_texture_creation_readiness_decision_available
- real_texture_creation_readiness_blocked
- real_texture_creation_minimum_renderability_checklist_defined
- real_texture_creation_frame_callback_policy_observed
- real_texture_creation_frame_callback_policy_still_blocked
- real_texture_creation_renderer_backend_instance_available
- real_texture_creation_texture_import_route_available
- real_texture_creation_future_texture_handle_owner_defined
- real_texture_creation_texture_cleanup_policy_defined
- real_texture_creation_damage_submission_available
- real_texture_creation_render_success_evidence_available
- real_texture_creation_frame_callback_done_allowed
- real_texture_creation_ready
- real_texture_creation_allowed
- real_texture_creation_missing_renderer_backend_instance
- real_texture_creation_missing_texture_import_route
- real_texture_creation_missing_future_texture_handle_ownership_policy
- real_texture_creation_missing_texture_cleanup_policy
- real_texture_creation_missing_damage_submission
- real_texture_creation_missing_render_success_evidence
- real_texture_creation_frame_callback_done_disabled
- real_texture_creation_explicitly_disabled
- real_texture_creation_readiness_without_texture
- real_texture_creation_buffer_import_attempted
- real_texture_creation_buffer_imported
- real_texture_creation_texture_created
- real_texture_creation_renderer_called
- real_texture_creation_damage_submitted
- real_texture_creation_frame_callback_done_sent
- real_texture_creation_input_support
- real_texture_creation_core_mutation_invoked

The report is not bounded loop progress. It is visibility for a blocked seam.

## Feature Gate

The implementation remains in the existing `smithay-linux` and Linux-target
Smithay backend path. Default and `smithay-probe` builds continue to avoid real
Linux renderer resources.

No new dependency is added in this phase.

## Smithay / Wayland / Linux Type Boundary

Real Smithay / Wayland buffer, callback, and renderer type boundaries remain
restricted to `src/smithay_backend/linux_shm_buffer_import_adapter.rs` and
related Linux-only smithay_backend glue.

Core must not import or name Smithay, Wayland, `wl_callback`, `WlCallback`,
`wl_buffer`, `WlBuffer`, `BufferData`, `TextureId`, texture, renderer, EGL,
GLES, WGPU, DRM, GBM, dmabuf, or real renderer damage submission types. Core
remains limited to abstract concepts such as `WindowId`, `Geometry`, `State`,
`Layout`, `Action`, and `Command`.

## Test Strategy

Phase 56L is covered by:

- a unit test deriving a blocked real texture creation readiness decision from
  a Phase 56K frame callback completion policy report;
- a source-contract test proving the document, adapter report, runtime loop,
  and orchestrator summary expose the decision while keeping capability truth
  false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- A readiness decision can be mistaken for permission to create a texture.
  Capability truth fields prevent that.
- A minimum renderability checklist can be mistaken for renderability. The
  report records missing renderer, import route, damage, render success, and
  frame callback permission separately.
- The report can be mistaken for bounded loop progress. The runtime summary
  exposes the report but does not include it in progress detection.

## Phase 56M Suggestions

Phase 56M should consume the Phase 56L readiness decision and define the next
minimal renderer backend owner boundary for real texture creation. It should
continue as pure data unless every precondition is proven by real Linux target
evidence.

## Stop Condition

Phase 56L stops after the real texture creation readiness decision report is
exposed through runtime / bounded loop / orchestrator summaries and CI is
green. In No-Brake Goal Mode, continue directly into Phase 56M after merge and
green main CI.
