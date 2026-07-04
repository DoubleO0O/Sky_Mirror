# Phase 55N - Real Import Route Decision Matrix

Phase 55N prepares the real buffer import route decision for Phase 56A. It is a
decision matrix and non-executing adapter contract phase only. It does not
implement real buffer import, select a backend for execution, create textures,
call a renderer, submit damage, send frame callback done, connect input, or
mutate core state.

This is a recommendation, not an implementation.

## Current Chain Overview

The Phase 55 buffer import preparation chain currently contains:

- 55E: buffer importer resource owner boundary / handoff seam
- 55F: buffer import planning/report seam
- 55G: buffer import implementation descriptor / adapter boundary
- 55H: buffer import adapter proof boundary
- 55I: buffer import precondition gate
- 55J: buffer import execution dry-run / no-op guard
- 55K: buffer import implementation owner shell / actual import owner boundary
- 55L: actual buffer import attempt admission / record pure-data seam
- 55M: real buffer import boundary audit / blocker taxonomy

Those phases are still:

- pure-data
- readiness
- descriptor
- proof
- gate
- dry-run
- owner shell
- record
- audit

They are not:

- real buffer import
- real texture creation
- real renderer call
- real damage submit
- real frame callback done

Phase 55N keeps this line intact. A route decision matrix can make Phase 56A
smaller and safer, but it does not make any buffer imported or renderable.

## Capability Truth

Phase 55N keeps the current truth table unchanged:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

Route selection evidence, adapter contract availability, and recommended
Phase 56A direction must not flip these fields.

## Route Decision Matrix

### Route A: SHM-first nested MVP route

Target fit: nested Wayland MVP where the first goal is proving a minimal
`wl_buffer` import adapter boundary inside the nested backend.

Nested MVP feasibility: highest. Shared-memory buffers are the smallest path to
prove a Wayland client buffer can cross the future import adapter boundary
without immediately committing to DRM, GBM, dmabuf negotiation, or GPU-specific
renderer resources.

Linux DRM/GBM migration: moderate. SHM-first does not solve scanout or optimal
GPU paths, but it keeps the first import skeleton understandable and leaves a
clean upgrade path to dmabuf / GBM / EGL / GLES later.

Smithay type intrusion risk: lowest if real Smithay buffer and shm types remain
inside the smithay_backend / Linux-only adapter layer.

Feature gate complexity: lowest. Phase 56A can likely reuse the existing
`smithay-linux` gate for Linux-only nested adapter work.

Testing complexity: lowest. It can be covered first by source-contract,
compile-time Linux CI, and narrow adapter result tests before texture creation
or renderer calls exist.

Renderer/texture dependency: minimal. The route can keep texture creation and
renderer calls absent while proving the importer adapter shape.

Short-term implementation cost: lowest.

Long-term maintainability: good if the SHM path remains an MVP route and the
contract names dmabuf / GBM / EGL / GLES extension points.

Main blockers:

- real Smithay buffer type admission is not yet authorized;
- SHM handler state and buffer lifecycle ownership still need an explicit
  Linux-only owner;
- texture creation and renderer calls must remain separate phases.

### Route B: dmabuf route

Target fit: efficient Linux compositor path, possible future zero-copy or
lower-copy client buffer import.

Nested MVP feasibility: lower than SHM-first. dmabuf requires more Linux
graphics negotiation and stronger backend assumptions before the runtime can
even prove a simple imported buffer result.

Linux DRM/GBM migration: high. dmabuf is closer to the future native graphics
path than SHM-first.

Smithay type intrusion risk: high unless dmabuf types are strictly isolated in
smithay_backend / Linux-only adapter modules.

Feature gate complexity: high. The route likely needs more precise Linux-only
gates and backend-specific compile coverage.

Testing complexity: high. Realistic tests need Linux CI, buffer format
handling, failure cases, and eventually renderer integration.

Renderer/texture dependency: high. dmabuf becomes meaningful only when paired
with texture creation and renderer import support.

Short-term implementation cost: high.

Long-term maintainability: strong if introduced after the SHM-first contract
has proven the ownership shape.

Main blockers:

- backend choice is not authorized;
- dmabuf import and format negotiation are not represented by the current
  runtime contract;
- texture creation and renderer call paths are still false.

### Route C: EGL/GLES/GBM route

Target fit: native Linux rendering stack and eventual compositor-grade GPU
resource path.

Nested MVP feasibility: low for the next phase. It couples buffer import,
graphics context ownership, texture creation, renderer calls, and platform
resource setup too early.

Linux DRM/GBM migration: highest. This route is closest to a future native
session / DRM / GBM stack.

Smithay type intrusion risk: very high unless real graphics handles are
contained in gated smithay_backend glue.

Feature gate complexity: very high. It would likely need precise target,
feature, system library, and runtime environment gates.

Testing complexity: very high. Meaningful tests need Linux graphics libraries,
runtime device assumptions, and careful CI separation.

Renderer/texture dependency: very high. EGL/GLES/GBM is mostly about creating
and using real renderer and texture resources.

Short-term implementation cost: very high.

Long-term maintainability: good only after narrower importer, texture, and
renderer owner boundaries exist.

Main blockers:

- real backend selection is not authorized;
- texture creation is still false;
- renderer calls are still false;
- DRM/GBM ownership is outside the current nested MVP minimum.

### Route D: WGPU route

Target fit: future cross-platform renderer architecture or higher-level GPU
abstraction path.

Nested MVP feasibility: moderate to low. WGPU can be attractive long term, but
it may not map cleanly to Smithay's immediate buffer import and Linux compositor
ownership requirements.

Linux DRM/GBM migration: uncertain. The route may abstract over GPU APIs, but
the project still needs Linux Wayland buffer import and compositor ownership.

Smithay type intrusion risk: high if WGPU resource types cross into core.

Feature gate complexity: high. It would introduce a new dependency and more
backend-specific gates, which Phase 55N explicitly does not allow.

Testing complexity: high. Reliable CI would require GPU abstraction coverage or
mocked device paths.

Renderer/texture dependency: high. WGPU route is not useful without real device
and texture ownership decisions.

Short-term implementation cost: high.

Long-term maintainability: potentially good if selected as the renderer layer
after the import boundary is proven.

Main blockers:

- no new Cargo dependency is allowed in Phase 55N;
- WGPU backend strategy is not authorized;
- real texture and renderer ownership are still separate missing resources.

### Route E: hybrid staged route

Target fit: long-term staged compositor roadmap.

Nested MVP feasibility: high if the first stage is SHM-first nested MVP route
and later stages add dmabuf, then EGL/GLES/GBM or WGPU as explicit decisions.

Linux DRM/GBM migration: high over time. It avoids prematurely forcing native
graphics complexity into the nested MVP.

Smithay type intrusion risk: manageable if every stage keeps real resource
types inside smithay_backend / Linux-only adapter modules.

Feature gate complexity: moderate over time and low in the immediate next
phase.

Testing complexity: manageable because each route extension gets its own
source-contract, Linux CI, and runtime tests.

Renderer/texture dependency: staged. Phase 56A can avoid texture creation and
renderer calls while later phases own those resources explicitly.

Short-term implementation cost: low for the first stage.

Long-term maintainability: best if every stage preserves capability truth and
does not collapse import, texture, renderer, damage, and frame callback work
into one change.

Main blockers:

- Phase 56A still needs explicit user authorization;
- each later route upgrade needs its own backend decision and capability truth
  transition.

## Recommended Route

The default recommendation is:

Phase 56A should use SHM-first nested MVP route / non-DRM-first minimal path.

This recommendation is scoped to Phase 55N. It is not a real implementation,
not a real backend selection for execution, and not permission to import a
buffer. Real implementation must wait for explicit user authorization for
Phase 56A.

The SHM-first recommendation is preferred because it:

- better matches a nested Wayland MVP;
- avoids entering DRM / GBM / dmabuf complexity before the first import adapter
  skeleton exists;
- makes `wl_buffer` to importer adapter skeleton validation easier;
- keeps core pure by confining real buffer details to smithay_backend;
- leaves a staged upgrade path to dmabuf, GBM, EGL, GLES, or WGPU;
- can keep texture creation, renderer calls, damage submit, frame callback done,
  input, and core mutation disabled in Phase 56A.

## Non-executing Adapter Contract

Phase 55N defines this contract in documentation only. It does not add a
production struct and does not execute the contract.

### input evidence

- adapter surface id
- commit sequence
- buffer presence evidence
- buffer candidate evidence
- actual import required
- precondition gate evidence
- execution dry-run evidence
- implementation owner shell evidence

### output evidence

- route selected
- adapter contract available
- real importer missing
- execution allowed = false
- buffer_import_attempted = false
- buffer_imported = false

The contract is useful only as a future handoff checklist. It is not a real
importer implementation and does not prove a real imported buffer.

## Smithay And Renderer Type Boundary

Real Smithay, Wayland, renderer, buffer, texture, dmabuf, EGL, GLES, GBM, and
WGPU types must stay in smithay_backend / Linux-only adapter / glue layers.

Core must not depend on those real resource types. Core may only see abstract
domain concepts such as:

- `WindowId`
- `Geometry`
- `State`
- `Layout`
- `Action`
- `Command`

Smithay handlers must continue to produce adapter-owned observations and must
not directly hold or mutate `State`.

## Phase 56A Minimum Safe Entry Point

Phase 56A: minimal SHM-first buffer import adapter skeleton

Phase 56A can be the first phase that starts a real resource boundary, but only
after explicit user authorization. Its minimum safe target should be:

- establish a Linux-only / nested-only SHM importer adapter skeleton;
- read or express `wl_shm` buffer import capability;
- generate an import attempt result;
- keep texture creation disabled at first;
- keep renderer calls disabled at first;
- keep core unaware of real buffer handles and real Smithay resources;
- preserve separate blockers for texture creation, renderer call, damage submit,
  and frame callback done.

Phase 55N does not enter this implementation.

## User Decisions Required Before Phase 56A

Before Phase 56A starts, the user must decide:

1. Is SHM-first accepted as the Phase 56A nested MVP route?
2. May Phase 56A introduce real Smithay buffer types inside the Linux-only
   adapter layer?
3. Is Phase 56A limited to nested backend only, without DRM / GBM?
4. Should Phase 56A still forbid texture creation and renderer calls?
5. Should Phase 56A reuse `smithay-linux`, or introduce a narrower feature gate?

Until those answers are explicit, the execution state is blocked.

Required stop condition: waiting for user authorization on Phase 56A.
