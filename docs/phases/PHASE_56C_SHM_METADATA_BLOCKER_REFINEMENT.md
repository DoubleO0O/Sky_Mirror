# Phase 56C - SHM Metadata Unsupported / Blocker Refinement

Phase 56C refines the SHM-first nested MVP metadata blocker taxonomy from
Phase 56B. It does not import a buffer, create a texture, call a renderer,
submit damage, send frame callback done, connect input, mutate core state, or
enter DRM / GBM / dmabuf, EGL / GLES, or WGPU paths.

The phase is blocker evidence only.

## Scope

Phase 56C is allowed to:

- keep real Smithay / Wayland / `WlBuffer` / SHM / `BufferData` types inside
  `src/smithay_backend` Linux-only adapter code;
- refine unavailable / unsupported / blocked metadata reasons;
- expose refined blocker evidence through runtime / bounded loop /
  orchestrator summaries;
- preserve the Phase 56B metadata evidence report as pure data.

Phase 56C is not allowed to:

- import a buffer;
- create a texture;
- call a renderer;
- submit damage;
- send frame callback done;
- connect input;
- mutate core;
- enter DRM / GBM / dmabuf;
- enter EGL / GLES;
- enter WGPU;
- add a Cargo dependency.

## Refined Blocker Taxonomy

The Linux-only adapter and runtime reports distinguish:

- no real WlBuffer available;
- WlBuffer available but not SHM;
- SHM-like candidate but missing safe Smithay metadata accessor;
- metadata observable but insufficient for texture precondition;
- missing buffer lifetime / cleanup ownership policy;
- runtime report only has evidence, not import execution.

These blockers are stop signs for future phases. They are not evidence that a
buffer has been imported or that a surface is renderable.

## Capability Truth

Phase 56C keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

Metadata blocker evidence does not mean buffer import, texture creation, or
renderer admission has occurred.

## Core Isolation

Core still has no Smithay, Wayland, `wl_buffer`, SHM, `BufferData`, buffer,
texture, renderer, dmabuf, EGL, GLES, WGPU, or graphics-resource type
dependency. Core remains limited to abstract domain concepts such as
`WindowId`, `Geometry`, `State`, `Action`, and `Command`.

## Stop Condition

Phase 56C stops after refined metadata blocker evidence is exposed. Phase 56D
or any later phase that enters texture creation preconditions, renderer calls,
damage submit, frame callback done, input, DRM / GBM / dmabuf, WGPU, EGL, or
GLES requires separate user authorization.
