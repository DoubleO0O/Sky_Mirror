# Phase 55E - Buffer Import Resource Owner Boundary

## Goal

Phase 55E introduces a runtime-owned buffer importer resource owner boundary
after the Phase 55D renderer backend owner shell readiness seam.

The purpose is to prove that the runtime can hand off the minimal pure-data
evidence required by a future real buffer importer without importing client
buffers, creating textures, or calling a renderer.

## Scope

This phase only adds the buffer importer owner boundary and readiness/report
shape.

- Do not import buffers.
- Do not create textures.
- Do not call a renderer.
- Do not submit damage.
- Do not send frame callback done.
- Do not add input support.
- Do not mutate core.
- Do not hand-write `WindowId`.
- Do not claim a renderable window or real compositor runtime.

## Chain Position

Phase 55E consumes the Phase 55D
`RuntimeSurfaceCommitRendererBackendOwnerShellReadinessReport` and produces
`RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport`.

The report preserves the future buffer import handoff evidence:

- adapter surface id;
- surface identity key;
- commit sequence;
- buffer attach/presence/removal evidence;
- damage evidence and damage rect count;
- frame callback request evidence and count;
- renderer backend descriptor evidence;
- registered renderer backend kind.

Multiple commit-derived intents remain FIFO through the bounded loop and
orchestrator reports. This prevents a later buffer importer from relying on a
latest-snapshot view that could drop earlier commit evidence.

## Owner Boundary Truth

Phase 55E makes these owner boundary facts visible:

- `buffer_importer_owner_available = true`
- `buffer_importer_owner_bound = true`
- `renderer_backend_descriptor_evidence_available = true`
- `registered_renderer_backend_kind = Some(SmithayLinux)`

These facts only mean a runtime-owned boundary can observe and preserve the
handoff evidence. They do not mean a Wayland buffer has been imported, a
texture exists, a renderer has run, damage has been submitted, or a frame
callback has completed.

## Capability Truth

The current real capability truth remains:

- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

## Blocked Resource Paths

The report explicitly keeps the following blockers visible:

- missing actual buffer import implementation;
- missing texture creation;
- missing renderer call;
- missing damage submit;
- missing frame callback done.

`buffer_importer_owner_available` is an owner-boundary readiness field. It is
not permission to flip `buffer_imported` to true. A future phase may only do so
after adding the real buffer importer implementation and tests proving the
operation actually happened.

## Next Safe Step

The next phase can add a real buffer import planning/report seam behind this
owner boundary, or add a narrower importer implementation descriptor. It should
still avoid creating textures, calling a renderer, submitting damage, or sending
frame callback done until those resource paths have separate owner boundaries
and verification.
