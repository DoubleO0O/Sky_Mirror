# Phase 56M - Real Renderer Backend Owner Boundary

## Authorization

Phase 56M is authorized by No-Brake Goal Mode as the next SHM-first nested MVP
checkpoint after Phase 56L.

This phase defines the real renderer backend owner boundary as pure data only.
It does not wait for separate renderer, texture, damage, frame callback, input,
DRM, GBM, EGL, GLES, or WGPU authorization.

## Phase 56L to Phase 56M Relationship

Phase 56L produced a real texture creation readiness decision and kept real
texture creation blocked. Phase 56M consumes that decision and defines the
minimum owner boundary for the future renderer backend instance that real
texture creation will require.

The owner boundary is not renderer creation. It does not construct a renderer
backend instance, import a buffer, create a `TextureId`, create a texture, call
a renderer, submit damage, send frame callback done, connect input, mutate core,
or execute a Smithay `ImportAll::import_buffer` route.

## Implementation Content

Phase 56M adds:

- `RuntimeSurfaceCommitRendererBackendOwnerBoundaryOperation`
- `RuntimeSurfaceCommitRendererBackendOwnerBoundaryBlocker`
- `RuntimeSurfaceCommitRendererBackendOwnerBoundaryPolicy`
- `RuntimeSurfaceCommitRendererBackendOwnerBoundaryReport`
- `renderer_backend_owner_boundary_from_real_texture_creation_readiness_decision`

The report is FIFO-preserving because each pump produces one renderer backend
owner boundary report from the corresponding Phase 56L readiness decision
report.

## Capability Truth

These values remain false in Phase 56M:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56M also records:

- renderer_backend_owner_boundary_available = true
- renderer_backend_owner_boundary_blocked = true
- real_texture_creation_readiness_still_blocked = true
- renderer_backend_owner_defined = true
- renderer_backend_lifecycle_owner_defined = true
- renderer_backend_cleanup_owner_defined = true
- renderer_backend_error_owner_defined = true
- renderer_backend_availability_owner_defined = true
- minimal_renderer_path_selected = true
- renderer_backend_instance_available = false
- renderer_backend_concrete_type_selected = false
- renderer_backend_construction_route_available = false
- renderer_backend_runtime_storage_available = false
- renderer_backend_cleanup_implemented = false
- render_target_binding_available = false
- renderer_backend_creation_allowed = false

These values are owner-boundary evidence. They are not real renderer backend
capability, not texture capability, and not renderable window capability.

## Minimal Renderer Route

The selected pure-data route label is `smithay_linux_nested_renderer`, owned by
`linux_nested_renderer_backend_owner`.

The route remains blocked until later phases define and compile the concrete
renderer backend type, construction route, runtime storage, cleanup
implementation, render target binding, and creation policy.

## Blocker Taxonomy

Phase 56M uses the following blocker taxonomy:

- real_texture_creation_readiness_still_blocked
- missing_renderer_backend_instance
- missing_renderer_backend_concrete_type
- missing_renderer_backend_construction_route
- missing_renderer_backend_runtime_storage
- missing_renderer_backend_cleanup_implementation
- missing_render_target_binding
- renderer_backend_creation_explicitly_disabled
- renderer_backend_owner_boundary_without_instance

The default blocker reason is that the route has an owner boundary but still
lacks a concrete renderer type, construction route, runtime storage, cleanup
implementation, render target binding, and real backend instance.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- renderer_backend_owner_boundary_invocations
- renderer_backend_owner_boundary_reports
- renderer_backend_owner_boundary_available
- renderer_backend_owner_boundary_blocked
- renderer_backend_owner_boundary_readiness_decision_observed
- renderer_backend_owner_boundary_readiness_still_blocked
- renderer_backend_owner_defined
- renderer_backend_lifecycle_owner_defined
- renderer_backend_cleanup_owner_defined
- renderer_backend_error_owner_defined
- renderer_backend_availability_owner_defined
- renderer_backend_minimal_renderer_path_selected
- renderer_backend_owner_boundary_instance_available
- renderer_backend_concrete_type_selected
- renderer_backend_construction_route_available
- renderer_backend_runtime_storage_available
- renderer_backend_cleanup_implemented
- renderer_backend_render_target_binding_available
- renderer_backend_creation_allowed
- renderer_backend_owner_boundary_missing_instance
- renderer_backend_owner_boundary_missing_concrete_type
- renderer_backend_owner_boundary_missing_construction_route
- renderer_backend_owner_boundary_missing_runtime_storage
- renderer_backend_owner_boundary_missing_cleanup_implementation
- renderer_backend_owner_boundary_missing_render_target_binding
- renderer_backend_owner_boundary_creation_explicitly_disabled
- renderer_backend_owner_boundary_without_instance
- renderer_backend_owner_boundary_buffer_import_attempted
- renderer_backend_owner_boundary_buffer_imported
- renderer_backend_owner_boundary_texture_created
- renderer_backend_owner_boundary_renderer_called
- renderer_backend_owner_boundary_damage_submitted
- renderer_backend_owner_boundary_frame_callback_done_sent
- renderer_backend_owner_boundary_input_support
- renderer_backend_owner_boundary_core_mutation_invoked

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

Phase 56M is covered by:

- a unit test deriving a blocked renderer backend owner boundary from a Phase
  56L real texture creation readiness decision report;
- a source-contract test proving the document, adapter report, coordinator,
  runtime loop, and orchestrator summary expose the owner boundary while
  keeping capability truth false;
- the standard default, `smithay-probe`, and Linux-target check/test matrix.

## Known Risks

- An owner boundary can be mistaken for a real renderer backend instance. The
  report keeps `renderer_backend_instance_available = false` and
  `renderer_backend_creation_allowed = false`.
- A selected renderer route label can be mistaken for a compiled renderer type.
  The report keeps `renderer_backend_concrete_type_selected = false`.
- The report can be mistaken for bounded loop progress. The runtime summary
  exposes the report but does not include it in progress detection.

## Phase 56N Suggestions

Phase 56N should consume the Phase 56M owner boundary and define the minimal
renderer backend concrete type / construction route decision. It should remain
pure data unless the Linux-target construction route is proven by compile-time
evidence and a cleanup policy.

## Stop Condition

Phase 56M stops after the renderer backend owner boundary report is exposed
through runtime / bounded loop / orchestrator summaries and CI is green. In
No-Brake Goal Mode, continue directly into Phase 56N after merge and green main
CI.
