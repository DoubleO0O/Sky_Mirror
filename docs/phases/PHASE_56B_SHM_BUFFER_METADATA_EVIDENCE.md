# Phase 56B - SHM Buffer Metadata Evidence

Phase 56B extends the SHM-first nested MVP route with Linux-only SHM buffer
metadata evidence and taxonomy. It does not import a buffer, create a texture,
call a renderer, submit damage, send frame callback done, connect input, mutate
core state, or enter DRM / GBM / dmabuf, EGL / GLES, or WGPU paths.

The phase is metadata evidence only.

## Scope

Phase 56B is allowed to:

- keep real Smithay / Wayland / `WlBuffer` / SHM types inside
  `src/smithay_backend` Linux-only adapter code;
- define SHM buffer metadata evidence for kind, offset, size, stride, and
  format;
- use Smithay's SHM metadata access boundary only in Linux-only adapter code;
- map unavailable / unsupported blocker cases into pure-data runtime reports;
- expose the metadata evidence report through runtime / bounded loop /
  orchestrator summaries.

Phase 56B is not allowed to:

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

## Metadata Taxonomy

The Linux-only adapter defines:

- `LinuxShmBufferMetadataEvidence`
- `LinuxShmBufferMetadataKind`
- `RuntimeSurfaceCommitShmBufferMetadataReport`
- `RuntimeSurfaceCommitShmBufferMetadataBlocker`

Metadata evidence can express:

- whether the buffer is SHM-managed;
- whether metadata is available;
- whether metadata was observed;
- offset;
- width;
- height;
- stride;
- format;
- unavailable / unsupported blocker state.

If no concrete `WlBuffer` is available in the runtime chain, the report must
stay unavailable and blocked. The runtime path must not invent metadata from
commit observations.

## Capability Truth

Phase 56B keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

Metadata evidence does not mean the buffer has been imported or is renderable.

## Core Isolation

Core still has no Smithay, Wayland, `wl_buffer`, SHM, buffer, texture, renderer,
dmabuf, EGL, GLES, WGPU, or graphics-resource type dependency. Core remains
limited to abstract domain concepts such as `WindowId`, `Geometry`, `State`,
`Action`, and `Command`.

## Stop Condition

Phase 56B stops after metadata evidence / taxonomy is exposed. Phase 56C or any
later phase that attempts texture creation, renderer calls, damage submit, frame
callback done, input, DRM / GBM / dmabuf, WGPU, EGL, or GLES requires separate
user authorization.
