# Phase 56G - Texture Owner Boundary

## Authorization

Phase 56G is authorized as the next SHM-first nested MVP checkpoint after
Phase 56F.

This phase does not enter Phase 56H. It establishes a Texture Owner Boundary
as pure data only.

## Phase 56F to Phase 56G Relationship

Phase 56F produced a texture creation blocker / no-op report from the texture
creation precondition audit. Phase 56G consumes that no-op report and derives a
texture owner boundary report.

The Texture Owner Boundary defines who owns a future texture creation request,
who must eventually own a future texture handle or id lifecycle, and which
cleanup / release / invalidation policies are still missing. It does not create
a texture, call a renderer, submit damage, send frame callback done, connect
input, mutate core, or execute `ImportAll::import_buffer`.

## Capability Truth

These values remain false in Phase 56G:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

Phase 56G also records:

- texture_owner_boundary_available = true
- texture_owner_boundary_blocked = true
- texture_creation_request_owner_defined = true
- future_texture_handle_owner_defined = false
- future_texture_lifetime_owner_defined = false
- future_texture_cleanup_owner_defined = false
- future_texture_release_owner_defined = false
- future_texture_invalidation_owner_defined = false
- renderer_backend_instance_available = false
- texture_import_route_available = false

These values are ownership and blocker evidence. They are not real texture
resources, not renderer capability, and not renderable window capability.

## Texture Owner Boundary Semantics

The Phase 56G report fields are pure data:

- texture_owner_boundary_available: the owner boundary report can be produced.
- texture_owner_boundary_blocked: real execution remains blocked.
- texture_creation_request_owner_defined: the future request owner is named.
- future_texture_handle_owner_defined: ownership of a future texture handle or
  `TextureId` is not defined.
- future_texture_lifetime_owner_defined: future lifetime ownership is not
  defined.
- future_texture_cleanup_owner_defined: future cleanup ownership is not
  defined.
- future_texture_release_owner_defined: future release ownership is not
  defined.
- future_texture_invalidation_owner_defined: future invalidation ownership is
  not defined.
- renderer_backend_instance_available: no real renderer backend instance is
  available.
- texture_import_route_available: no route exists from SHM evidence to a
  texture object.

The boundary can name the owner of a future texture creation request without
owning a real texture handle and without creating a texture.

## Texture Owner Boundary Blocker Taxonomy

Phase 56G uses the following blocker taxonomy:

- texture_creation_noop_only
- missing_texture_owner_boundary
- missing_future_texture_handle_policy
- missing_future_texture_lifetime_policy
- missing_future_texture_cleanup_policy
- missing_future_texture_release_policy
- missing_future_texture_invalidation_policy
- missing_renderer_backend_instance
- missing_texture_import_route
- runtime_evidence_without_texture_ownership
- owner_boundary_without_texture_creation

The default blocker reason is that Phase 56G can define the owner boundary for
a future request, but it still lacks future handle ownership, lifetime,
cleanup, release, invalidation, renderer backend instance, and texture import
route policies.

## Report Fields

Runtime / bounded loop / orchestrator reports expose:

- texture_owner_boundary_invocations
- texture_owner_boundary_reports
- texture_owner_boundary_available
- texture_owner_boundary_blocked
- texture_owner_texture_creation_request_owner_defined
- texture_owner_future_texture_handle_owner_defined
- texture_owner_future_texture_lifetime_owner_defined
- texture_owner_future_texture_cleanup_owner_defined
- texture_owner_future_texture_release_owner_defined
- texture_owner_future_texture_invalidation_owner_defined
- texture_owner_renderer_backend_instance_available
- texture_owner_texture_import_route_available
- texture_owner_texture_creation_noop_only
- texture_owner_missing_texture_owner_boundary
- texture_owner_missing_future_texture_handle_policy
- texture_owner_missing_future_texture_lifetime_policy
- texture_owner_missing_future_texture_cleanup_policy
- texture_owner_missing_future_texture_release_policy
- texture_owner_missing_future_texture_invalidation_policy
- texture_owner_missing_renderer_backend_instance
- texture_owner_missing_texture_import_route
- texture_owner_runtime_evidence_without_texture_ownership
- texture_owner_boundary_without_texture_creation
- texture_owner_buffer_import_attempted
- texture_owner_buffer_imported
- texture_owner_texture_created
- texture_owner_renderer_called
- texture_owner_damage_submitted
- texture_owner_frame_callback_done_sent
- texture_owner_input_support
- texture_owner_core_mutation_invoked

The report is FIFO-preserving because each pump produces one owner boundary
report from the corresponding Phase 56F texture creation no-op report.

## Smithay Type Boundary

Real Smithay / Wayland buffer and SHM metadata type boundaries remain
restricted to `src/smithay_backend/linux_shm_buffer_import_adapter.rs` and
related Linux-only smithay_backend glue.

Core must not import or name Smithay, Wayland, `wl_buffer`, `WlBuffer`,
`BufferData`, texture, renderer, EGL, GLES, WGPU, DRM, GBM, dmabuf,
`ImportAll::import_buffer`, or `TextureId` types. Core remains limited to
abstract concepts such as `WindowId`, `Geometry`, `State`, `Layout`, `Action`,
and `Command`.

## Phase 56H Suggestions

Phase 56H requires separate user authorization. Possible directions are:

1. define future texture handle / id ownership policy as pure data;
2. define future texture cleanup / release / invalidation policy as pure data;
3. audit renderer backend instance availability;
4. audit texture import route requirements.

Default recommendation: Phase 56H should define the future texture handle and
cleanup ownership policy without creating a texture. Real texture creation,
renderer calls, damage submit, or frame callback done still require separate
authorization.

## Stop Condition

Phase 56G stops after the Texture Owner Boundary report is exposed through
runtime / bounded loop / orchestrator summaries and CI is green. Do not enter
Phase 56H without explicit authorization.
