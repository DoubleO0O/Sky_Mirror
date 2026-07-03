# Phase 55F - Buffer Import Planning Report

## Goal

Phase 55F introduces a runtime-owned buffer import planning/report seam after
the Phase 55E buffer importer resource owner boundary.

The purpose is to prove that the runtime can derive a pure-data future buffer
import plan from commit evidence and renderer backend descriptor evidence
without importing client buffers, creating textures, or calling a renderer.

## Scope

This phase only adds the buffer import planning/report shape.

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

Phase 55F consumes the Phase 55E
`RuntimeSurfaceCommitBufferImportResourceOwnerReadinessReport` and produces
`RuntimeSurfaceCommitBufferImportPlanningReport`.

The planning report preserves:

- adapter surface id;
- surface identity key;
- commit sequence;
- buffer attach/presence/removal evidence;
- whether buffer import is a future candidate;
- whether buffer import would be required by the future importer;
- damage evidence and damage rect count;
- frame callback request evidence and count;
- renderer backend descriptor evidence;
- registered renderer backend kind.

Multiple commit-derived intents remain FIFO through the bounded loop and
orchestrator reports. A commit with buffer attach/presence evidence can produce
a future import-required plan while a following plain commit stays in the same
FIFO report with no import requirement.

## Planning Truth

Phase 55F makes these planning facts visible:

- `buffer_import_plan_available = true`
- `buffer_import_plan_built = true`
- `buffer_import_candidate_observed = true`
- `buffer_import_required = true` only when candidate evidence represents a present,
  non-removed buffer. A null attach/remove observation remains candidate evidence,
  but it is not counted as requiring a real buffer import.

These facts only mean a pure-data plan exists for a commit that carries buffer
attach/presence evidence. They do not mean the buffer was imported, converted
to a texture, rendered, damaged, or frame-completed.

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

For commits without buffer attach/presence evidence, the report can also mark
the missing buffer import candidate. That is not an error; it is the pure-data
plan saying no future buffer import is required for that commit.

## Next Safe Step

The next phase can add a narrow importer implementation descriptor or a real
buffer import adapter boundary behind this planning report. It should still
avoid creating textures, calling a renderer, submitting damage, or sending
frame callback done until those resource paths have separate owner boundaries
and verification.
