# Phase 55M - Real Buffer Import Boundary Audit

Phase 55M audits the Phase 55E through Phase 55L buffer import readiness chain.
It is a transition and blocker taxonomy phase only. It does not implement real
buffer import.

No real buffer import has happened.

## Phase 55E-55L Audit

Phase 55E established the buffer importer resource owner boundary / handoff
seam. It is pure-data owner readiness and blocked-report evidence. It does not
own a Wayland buffer and does not import anything.

Phase 55F established the buffer import planning report. It separates candidate
evidence from actual import requirement evidence. Candidate evidence is not an
actual import request and is not a successful import.

Phase 55G established the buffer import implementation descriptor / adapter
boundary. The descriptor records the future implementation shape and adapter
boundary evidence. It is not a Smithay importer implementation.

Phase 55H established the buffer import adapter proof boundary. The proof
preserves adapter evidence and blocker state. It does not prove that a real
buffer can be imported.

Phase 55I established the buffer import precondition gate. The gate decides
whether evidence is sufficient to approach the future import boundary. It does
not cross that boundary.

Phase 55J established the buffer import execution dry-run / no-op guard. The
dry-run records what would be needed while forcing execution to stay no-op.

Phase 55K established the buffer import implementation owner shell. The shell
shows where a future implementation owner can live. It is still a shell and has
no real importer implementation behind it.

Phase 55L established the actual buffer import attempt admission / record
seam. The record preserves admission decisions and upstream owner shell
evidence. It intentionally keeps actual attempt admission blocked and does not
perform a real import attempt.

The chain is therefore pure-data, readiness, dry-run, and record plumbing. It
is not a real buffer import pipeline. Future reports must preserve that
distinction: shell / record / dry-run reports are not real import.

## Capability Truth

Phase 55M keeps the current truth table unchanged:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

These fields must not be flipped by owner shell availability, adapter proof,
precondition success, dry-run records, or actual attempt records.

Do not claim renderable window.
Do not claim real compositor runtime ready.

## Real Resource Boundary

Real buffer import will require actual Smithay and renderer resource ownership,
not just runtime evidence. The future implementation must decide and verify at
least these resource categories:

- Wayland buffer identity and lifetime, including `wl_buffer::WlBuffer`.
- Smithay buffer lifecycle hooks, including `BufferHandler`.
- A renderer backend owner that can call a real `Renderer`.
- Renderer-owned `Texture` or backend-specific texture resources.
- Buffer import routes such as shared memory, `Dmabuf`, `EGL`, `GLES`, `GBM`,
  or `WGPU`, depending on the chosen backend.
- A damage submit path that remains separate from damage observation evidence.
- A frame callback done path that remains separate from frame callback request
  observation evidence.

Smithay and renderer resource types must remain in src/smithay_backend. Real
types belong in a Linux-only adapter layer or another explicitly gated adapter
module. They must not leak into `src/core`.

The core remains abstract. It may continue to process abstract domain concepts
such as `WindowId`, `Geometry`, `State`, `Action`, and `Command`. It must not
receive Smithay handles, renderer handles, textures, dmabufs, EGL objects, GLES
objects, WGPU objects, or raw Wayland buffer objects.

Smithay handlers must continue to produce adapter-owned observations and must
not directly hold or mutate `State`.

## Blocker Taxonomy

Phase 55M names the blocker classes that must stay visible until real resource
ownership is deliberately introduced:

- `MissingAttemptAdmission`: the runtime has not admitted an actual import
  attempt.
- `MissingRealBufferImportImplementation`: no real buffer importer
  implementation exists behind the owner shell.
- `MissingTextureCreation`: no real texture creation path exists after import.
- `MissingRendererCall`: no real renderer call path exists.
- `MissingDamageSubmit`: no real damage submit path exists.
- `MissingFrameCallbackDone`: no real frame callback done path exists.

Additional architectural blockers remain in force:

- Smithay or renderer resource types crossing into `src/core`.
- A Smithay handler directly holding or mutating `State`.
- Handwritten or guessed `WindowId` values.
- Treating candidate evidence as actual import required.
- Treating actual import required as actual import executed.
- Treating shell/readiness/proof/dry-run/record evidence as real import.
- Declaring a renderable window before a real imported buffer, texture, and
  renderer call exist.
- Declaring real compositor runtime readiness before buffer import, texture
  creation, renderer call, damage submit, frame callback done, input, and core
  mutation boundaries are all explicitly owned and verified.

## Minimum Safe Cut-In

Phase 55N can remain non-executing if it only introduces a narrower adapter
contract for a future real importer implementation and keeps all capability
truth fields false. It should not choose a backend or import route unless the
phase explicitly owns that decision.

Phase 56A should be the earliest point for a real resource boundary if the next
step chooses an actual Smithay/renderer backend strategy. Before that work
starts, stop and require a user decision about the real backend and import
route, such as shared memory first, dmabuf first, EGL/GLES/GBM, or WGPU.

Stop before choosing a real backend.

Any phase after Phase 55M must keep the current audit visible so that
implementation descriptors, owner shells, dry-runs, and records are not
mistaken for real buffer import.
