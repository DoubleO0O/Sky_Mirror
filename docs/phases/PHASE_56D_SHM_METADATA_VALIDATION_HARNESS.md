# Phase 56D - SHM Metadata Validation Harness

Phase 56D adds a pure-data validation harness for the Phase 56B SHM metadata
evidence and the Phase 56C blocker taxonomy. It does not import a buffer,
create a texture, call a renderer, submit damage, send frame callback done,
connect input, mutate core state, or enter DRM / GBM / dmabuf, EGL / GLES, or
WGPU paths.

The phase is validation evidence only.

## Scope

Phase 56D is allowed to:

- keep real Smithay / Wayland / `WlBuffer` / SHM / `BufferData` types inside
  `src/smithay_backend` Linux-only adapter code;
- use pure-data fake evidence / controlled reports for validation;
- expose validation harness results through runtime / bounded loop /
  orchestrator summaries;
- preserve the Phase 56B and Phase 56C evidence reports as pure data.

Phase 56D is not allowed to:

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

## Controlled Paths

The validation harness covers:

- no real WlBuffer path;
- non-SHM path;
- metadata unavailable path;
- metadata partially available path;
- metadata insufficient for texture precondition path;
- missing lifetime / cleanup ownership policy path;
- runtime evidence without import execution path.

These paths validate blocker coverage only. They do not require a real client,
do not retain a `WlBuffer`, and do not produce a renderable buffer.

## Capability Truth

Phase 56D keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

Validation harness coverage does not mean buffer import, texture creation, or
renderer admission has occurred.

## Core Isolation

Core still has no Smithay, Wayland, `wl_buffer`, SHM, `BufferData`, buffer,
texture, renderer, dmabuf, EGL, GLES, WGPU, or graphics-resource type
dependency. Core remains limited to abstract domain concepts such as
`WindowId`, `Geometry`, `State`, `Action`, and `Command`.

## Stop Condition

Phase 56D stops after validation harness results are exposed. Phase 56E or any
later phase that enters texture creation preconditions, renderer calls, damage
submit, frame callback done, input, DRM / GBM / dmabuf, WGPU, EGL, or GLES
requires separate user authorization.
