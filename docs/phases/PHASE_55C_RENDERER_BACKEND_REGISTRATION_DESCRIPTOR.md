# Phase 55C - Renderer Backend Registration Descriptor

## Goal

Phase 55C introduces a runtime-owned renderer backend registration descriptor
seam after the Phase 55B render backend capability report.

The purpose is to prove that the runtime can register a narrow renderer backend
descriptor as pure data before any real renderer resource path is enabled.

## Scope

This phase only registers the descriptor/report shape.

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

Phase 55C consumes the Phase 55B
`RuntimeSurfaceCommitRenderBackendCapabilityReport` and produces
`RuntimeSurfaceCommitRendererBackendRegistrationReport`.

The report preserves:

- adapter surface id;
- surface identity key;
- commit sequence;
- buffer attach/presence/removal evidence;
- damage evidence and damage rect count;
- frame callback request evidence and count.

Multiple commit-derived intents remain FIFO through the bounded loop and
orchestrator reports.

## Descriptor Truth

Phase 55C makes these descriptor facts visible:

- `renderer_backend_registration_owner_available = true`
- `renderer_backend_descriptor_available = true`
- `renderer_backend_registered = true`
- `registered_renderer_backend_kind = Some(SmithayLinux)`

These facts only mean a runtime-owned descriptor exists. They do not mean a
real renderer object, buffer importer, texture path, damage submit path, or
frame callback completion path exists.

## Capability Truth

The current real capability truth remains:

- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

## Missing Real Resource Paths

The report explicitly keeps the following blockers visible:

- missing buffer import;
- missing texture creation;
- missing renderer call;
- missing damage submit;
- missing frame callback done.

## Next Safe Step

The next phase can attach a renderer backend owner shell to this descriptor and
continue reporting readiness as pure data. It should still avoid importing
client buffers, creating textures, or calling a renderer until those resource
paths have separate owner boundaries and tests.
