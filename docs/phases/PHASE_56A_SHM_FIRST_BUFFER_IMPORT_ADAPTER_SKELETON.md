# Phase 56A - SHM-first Buffer Import Adapter Skeleton

Phase 56A starts the SHM-first nested MVP route selected by Phase 55N. It adds a
Linux-only adapter skeleton that can name the real Smithay `wl_buffer` boundary,
but it still does not import buffers, create textures, call a renderer, submit
damage, send frame callback done, connect input, or mutate core state.

The phase is a resource-boundary skeleton, not a render pipeline.

## Scope

Phase 56A is allowed to:

- keep the implementation inside `smithay_backend` and the existing
  `smithay-linux` Linux gate;
- expose a minimal SHM-first adapter skeleton;
- reference Smithay / Wayland `WlBuffer` types only in Linux-only adapter code;
- preserve the Phase 55E-55L buffer import evidence chain;
- generate evidence-only, blocked, unsupported, and no-texture reports.

Phase 56A is not allowed to:

- import a buffer;
- call `with_buffer_contents`;
- create a texture;
- call a renderer;
- submit damage;
- send frame callback done;
- connect input;
- mutate core;
- enter DRM, GBM, dmabuf, WGPU, EGL, or GLES;
- add a Cargo dependency.

## Adapter Boundary

The new adapter boundary is:

- `LinuxShmFirstBufferImportAdapterSkeleton`
- `LinuxShmBufferTypeBoundaryEvidence`
- `RuntimeSurfaceCommitShmFirstBufferImportAdapterReport`

The adapter consumes `RuntimeSurfaceCommitBufferImportActualAttemptRecord`,
which remains the Phase 55L pure-data record. The new report preserves:

- adapter surface id through the observed actual-attempt record;
- commit sequence through the observed actual-attempt record;
- buffer attach evidence;
- buffer present evidence;
- buffer removed evidence;
- actual import required evidence;
- renderer backend descriptor evidence;
- registered renderer backend kind.

The adapter can report that the SHM-first route is selected and that the
Linux-only `WlBuffer` type boundary exists. Runtime reports that do not yet
carry a concrete `WlBuffer` keep `shm_buffer_type_boundary_observed = false`.

## Capability Truth

Phase 56A keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

Narrow SHM-first fields are allowed:

- `shm_buffer_adapter_available = true`
- `shm_buffer_import_route_selected = true`
- `shm_buffer_import_execution_blocked = true`
- `shm_buffer_type_boundary_observed = true` only when a Linux-only adapter
  function is given a real `WlBuffer` reference.

Those fields do not mean `buffer_imported`, `texture_created`, or
`renderer_called`.

## Blockers

Phase 56A reports explicit blockers instead of executing:

- missing actual attempt record;
- missing `WlBuffer` type boundary observation;
- missing SHM buffer access evidence;
- no actual import required;
- texture creation forbidden in Phase 56A;
- renderer call forbidden in Phase 56A;
- damage submit forbidden in Phase 56A;
- frame callback done forbidden in Phase 56A;
- DRM / GBM / dmabuf forbidden in Phase 56A.

## Core Isolation

Core remains unaware of Smithay, Wayland, `wl_buffer`, buffer, texture, and
renderer resource types. Real Smithay resource names stay inside
`src/smithay_backend` Linux-only adapter/glue code.

Core may continue to handle only abstract domain concepts such as `WindowId`,
`Geometry`, `State`, `Action`, and `Command`.

## Stop Condition

Phase 56A stops at the SHM-first adapter skeleton. Phase 56B or any later phase
that attempts texture creation, renderer calls, damage submit, frame callback
done, input, DRM / GBM / dmabuf, WGPU, EGL, or GLES requires separate user
authorization.
