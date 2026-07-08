# Phase 56N - Renderer Backend Concrete Route Decision

## Authorization

Phase 56N is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56M.

This phase selects a renderer backend concrete route candidate as pure data
only. It does not wait for separate renderer, texture, damage, frame callback,
input, DRM, GBM, EGL, GLES, or WGPU authorization.

## Phase 56M to Phase 56N Relationship

Phase 56M produced a renderer backend owner boundary and kept renderer backend
creation blocked. Phase 56N consumes that owner boundary and records the next
minimal route decision: which candidate should be proven by a later Linux-target
renderer backend skeleton.

The concrete route decision is not renderer construction. It does not construct
a renderer backend instance, import a buffer, create a `TextureId`, create a
texture, call a renderer, submit damage, send frame callback done, connect
input, mutate core, or execute a Smithay `ImportAll::import_buffer` route.

## Implementation Content

Phase 56N adds:

- `RuntimeSurfaceCommitRendererBackendConcreteRouteDecisionOperation`
- `RuntimeSurfaceCommitRendererBackendConcreteRouteDecisionBlocker`
- `RuntimeSurfaceCommitRendererBackendConcreteRouteDecisionReport`
- `renderer_backend_concrete_route_decision_from_owner_boundary`

The report is FIFO-preserving because each pump produces one concrete route
decision report from the corresponding Phase 56M renderer backend owner boundary
report.

## Capability Truth

These values remain false in Phase 56N:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56N also records:

- renderer_backend_concrete_route_decision_available = true
- renderer_backend_concrete_route_decision_blocked = true
- renderer_backend_concrete_type_candidate_defined = true
- renderer_backend_concrete_type_compiled = false
- renderer_backend_construction_route_available = false
- renderer_backend_runtime_storage_available = false
- renderer_backend_cleanup_policy_available = false
- render_target_binding_available = false
- renderer_backend_construction_allowed = false
- renderer_backend_instance_created = false

These values are route-decision evidence. They are not real renderer backend
capability, not texture capability, and not renderable window capability.

## Concrete Route Candidate

The selected pure-data candidate is
`smithay_linux_nested_renderer_candidate`.

The candidate remains blocked until a later phase proves the concrete renderer
backend type compiles on the Linux target, defines a construction route, owns
runtime storage, owns cleanup, and binds a render target.

## Blocker Taxonomy

Phase 56N uses the following blocker taxonomy:

- renderer_backend_owner_boundary_still_blocked
- missing_concrete_renderer_backend_type_compile_proof
- missing_renderer_backend_construction_route
- missing_renderer_backend_runtime_storage
- missing_renderer_backend_cleanup_policy
- missing_render_target_binding
- renderer_backend_construction_explicitly_disabled
- concrete_route_decision_without_backend_instance

The default blocker reason is that the route has a candidate but lacks compile
proof, construction route, runtime storage, cleanup policy, render target
binding, and a real backend instance.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- renderer_backend_concrete_route_decision_invocations
- renderer_backend_concrete_route_decision_reports
- renderer_backend_concrete_route_decision_available
- renderer_backend_concrete_route_decision_blocked
- renderer_backend_concrete_route_owner_boundary_observed
- renderer_backend_concrete_route_owner_boundary_still_blocked
- renderer_backend_concrete_type_candidate_defined
- renderer_backend_concrete_type_compiled
- renderer_backend_concrete_route_construction_route_available
- renderer_backend_concrete_route_runtime_storage_available
- renderer_backend_concrete_route_cleanup_policy_available
- renderer_backend_concrete_route_render_target_binding_available
- renderer_backend_concrete_route_construction_allowed
- renderer_backend_instance_created
- renderer_backend_concrete_route_missing_compile_proof
- renderer_backend_concrete_route_missing_construction_route
- renderer_backend_concrete_route_missing_runtime_storage
- renderer_backend_concrete_route_missing_cleanup_policy
- renderer_backend_concrete_route_missing_render_target_binding
- renderer_backend_concrete_route_construction_explicitly_disabled
- renderer_backend_concrete_route_without_backend_instance
- renderer_backend_concrete_route_buffer_import_attempted
- renderer_backend_concrete_route_buffer_imported
- renderer_backend_concrete_route_texture_created
- renderer_backend_concrete_route_renderer_called
- renderer_backend_concrete_route_damage_submitted
- renderer_backend_concrete_route_frame_callback_done_sent
- renderer_backend_concrete_route_input_support
- renderer_backend_concrete_route_core_mutation_invoked

The report is not bounded loop progress. It is visibility for a blocked seam.

## Feature Gate

The implementation remains in the existing `smithay-linux` and Linux-target
Smithay backend path. Default and `smithay-probe` builds continue to avoid real
Linux renderer resources.

No new dependency is added in this phase.

## Smithay / Wayland / Linux Type Boundary

Real Smithay / Wayland buffer, callback, renderer, output, and render target
type boundaries remain restricted to `src/smithay_backend` Linux-only glue.

Core must not import or name Smithay, Wayland, `wl_callback`, `WlCallback`,
`wl_buffer`, `WlBuffer`, `BufferData`, `TextureId`, texture, renderer, EGL,
GLES, WGPU, DRM, GBM, dmabuf, or real renderer damage submission types. Core
remains limited to abstract concepts such as `WindowId`, `Geometry`, `State`,
`Layout`, `Action`, and `Command`.

## Test Strategy

Phase 56N is covered by:

- a source-contract test proving the document, adapter report, coordinator,
  runtime loop, and orchestrator summary expose the decision while keeping
  capability truth false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- A concrete route candidate can be mistaken for a compiled renderer backend
  type. The report keeps `renderer_backend_concrete_type_compiled = false`.
- A route decision can be mistaken for renderer backend construction. The report
  keeps `renderer_backend_instance_created = false` and
  `renderer_backend_construction_allowed = false`.
- The report can be mistaken for bounded loop progress. The runtime summary
  exposes the report but does not include it in progress detection.

## Phase 56O Suggestions

Phase 56O should consume the Phase 56N concrete route decision and add the
minimal Linux-target renderer backend construction skeleton only if the concrete
route can compile behind `smithay-linux`; otherwise it should record the exact
compile blocker and continue with the next smallest route proof.

## Stop Condition

Phase 56N stops after the renderer backend concrete route decision report is
exposed through runtime / bounded loop / orchestrator summaries and CI is
green. In No-Brake Goal Mode, continue directly into Phase 56O after merge and
green main CI.
