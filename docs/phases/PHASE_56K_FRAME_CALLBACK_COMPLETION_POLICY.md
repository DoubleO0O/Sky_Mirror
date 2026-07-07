# Phase 56K - Frame Callback Completion Policy

## Authorization

Phase 56K is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56J.

This phase establishes a frame callback completion policy as pure data only. It
does not wait for separate texture, renderer, damage, input, or DRM/GBM
authorization.

## Phase 56J to Phase 56K Relationship

Phase 56J produced a damage-to-texture mapping audit from the Phase 56I texture
import route decision report. Phase 56K consumes that damage mapping audit and
derives a frame callback completion policy report.

The frame callback completion policy defines the future owner for completing
pending frame callbacks and the rule that callback completion requires real
render success. It does not create a texture, call a renderer, submit damage,
send frame callback done, connect input, mutate core, create a texture handle,
or execute a buffer import route.

## Implementation Content

Phase 56K adds:

- `RuntimeSurfaceCommitFrameCallbackCompletionPolicyOperation`
- `RuntimeSurfaceCommitFrameCallbackCompletionPolicyBlocker`
- `RuntimeSurfaceCommitFrameCallbackCompletionPolicy`
- `RuntimeSurfaceCommitFrameCallbackCompletionChecklist`
- `RuntimeSurfaceCommitFrameCallbackCompletionPolicyReport`
- `frame_callback_completion_policy_from_damage_to_texture_mapping_audit`

The report is FIFO-preserving because each pump produces one frame callback
completion policy report from the corresponding Phase 56J damage mapping audit
report.

## Capability Truth

These values remain false in Phase 56K:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56K also records:

- frame_callback_completion_policy_available = true
- frame_callback_completion_policy_blocked = true
- frame_callback_completion_owner_defined = true
- render_success_required_before_done = true
- real_texture_available = false
- renderer_backend_instance_available = false
- damage_submission_available = false
- render_success_evidence_available = false
- frame_callback_done_allowed = false

These values are policy and blocker evidence. They are not real frame callback
completion capability, not renderer capability, and not renderable window
capability.

## Blocker Taxonomy

Phase 56K uses the following blocker taxonomy:

- damage_to_texture_mapping_audit_still_blocked
- missing_real_texture
- missing_renderer_backend_instance
- missing_damage_submission
- missing_render_success_evidence
- frame_callback_done_explicitly_disabled
- frame_callback_completion_without_render

The default blocker reason is that the route still lacks a real texture,
renderer backend instance, damage submission, render success evidence, and
permission to send frame callback done.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- frame_callback_completion_policy_invocations
- frame_callback_completion_policy_reports
- frame_callback_completion_policy_available
- frame_callback_completion_policy_blocked
- frame_callback_policy_damage_mapping_audit_observed
- frame_callback_policy_damage_mapping_audit_still_blocked
- frame_callback_completion_owner_defined
- frame_callback_policy_render_success_required_before_done
- frame_callback_policy_real_texture_available
- frame_callback_policy_renderer_backend_instance_available
- frame_callback_policy_damage_submission_available
- frame_callback_policy_render_success_evidence_available
- frame_callback_done_allowed
- frame_callback_policy_missing_real_texture
- frame_callback_policy_missing_renderer_backend_instance
- frame_callback_policy_missing_damage_submission
- frame_callback_policy_missing_render_success_evidence
- frame_callback_policy_done_explicitly_disabled
- frame_callback_policy_without_render
- frame_callback_policy_buffer_import_attempted
- frame_callback_policy_buffer_imported
- frame_callback_policy_texture_created
- frame_callback_policy_renderer_called
- frame_callback_policy_damage_submitted
- frame_callback_policy_frame_callback_done_sent
- frame_callback_policy_input_support
- frame_callback_policy_core_mutation_invoked

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
`wl_buffer`, `WlBuffer`, `BufferData`, texture, renderer, EGL, GLES, WGPU, DRM,
GBM, dmabuf, or real renderer damage submission types. Core remains limited to
abstract concepts such as `WindowId`, `Geometry`, `State`, `Layout`, `Action`,
and `Command`.

## Test Strategy

Phase 56K is covered by:

- a unit test deriving a blocked frame callback completion policy report from a
  Phase 56J damage-to-texture mapping audit report;
- a source-contract test proving the document, adapter report, runtime loop,
  and orchestrator summary expose the policy while keeping capability truth
  false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- A completion policy owner can be mistaken for permission to send frame
  callback done. Capability truth fields prevent that.
- A pure-data policy can be mistaken for bounded loop progress. The runtime
  summary exposes the report but does not include it in progress detection.
- Frame callback completion must be tied to real render success. Phase 56K
  records the missing render success evidence instead of guessing.

## Phase 56L Suggestions

Phase 56L should consume the Phase 56K frame callback completion policy report
and define the next renderability checkpoint:

1. define the minimum blocked renderability checklist for SHM-first surfaces;
2. keep frame callback done disabled until real texture, renderer, damage, and
   render success paths are proven;
3. continue as pure data unless all preconditions are proven.

## Stop Condition

Phase 56K stops after the frame callback completion policy report is exposed
through runtime / bounded loop / orchestrator summaries and CI is green. In
No-Brake Goal Mode, continue directly into Phase 56L after merge and green main
CI.
