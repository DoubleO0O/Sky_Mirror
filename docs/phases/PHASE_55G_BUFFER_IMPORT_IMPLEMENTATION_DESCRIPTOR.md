# Phase 55G - Buffer Import Implementation Descriptor

## Goal

Phase 55G introduces a runtime-owned buffer import implementation descriptor /
adapter boundary after the Phase 55F buffer import planning report.

The purpose is to describe and register the minimum pure-data boundary required
by a future real buffer importer without importing client buffers, creating
textures, or calling a renderer.

## Scope

This phase only adds a pure-data implementation descriptor and boundary report.

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

Phase 55G consumes the Phase 55F
`RuntimeSurfaceCommitBufferImportPlanningReport` and produces
`RuntimeSurfaceCommitBufferImportImplementationBoundaryReport`.

The descriptor preserves the future importer boundary evidence:

- adapter surface id;
- surface identity key;
- commit sequence;
- buffer attach/presence/removal evidence;
- candidate evidence observed;
- actual import required;
- importer owner evidence;
- renderer backend descriptor evidence;
- registered renderer backend kind.

Multiple planning reports remain FIFO through the bounded loop and orchestrator
reports. A null attach/remove commit can produce candidate evidence while
keeping actual import required false. This prevents the future importer from
confusing removal evidence with a present buffer that must be imported.

## Descriptor Truth

Phase 55G makes these descriptor facts visible:

- `implementation_descriptor_available = true`
- `implementation_descriptor_registered = true`
- `candidate_evidence_observed = true`
- `actual_import_required = true` only when Phase 55F says a present,
  non-removed buffer would need future import.

These facts only mean a pure-data descriptor has been registered for a future
importer boundary. They do not mean a Wayland buffer was imported or that a
texture/render path exists.

## Capability Truth

The current real capability truth remains:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

## Candidate Evidence Truth

`candidate_evidence_observed` records that a commit carried evidence that a
future importer boundary must understand. It is not actual import execution.

For null attach/remove evidence:

- `candidate_evidence_observed = true`
- `actual_import_required = false`
- `buffer_import_attempted = false`
- `buffer_imported = false`

For commits without buffer attach/presence/removal evidence:

- `candidate_evidence_observed = false`
- `actual_import_required = false`
- `buffer_import_attempted = false`
- `buffer_imported = false`

## Blocked Resource Paths

The report explicitly keeps the following blockers visible:

- missing actual buffer import implementation;
- missing texture creation;
- missing renderer call;
- missing damage submit;
- missing frame callback done.

`implementation_descriptor_registered` is an adapter-boundary descriptor field.
It is not permission to flip `buffer_imported`, `texture_created`, or
`renderer_called` to true.

## Next Safe Step

The next phase can add a narrow real buffer import adapter proof boundary or a
more specific importer implementation planning seam. It should still avoid
creating textures, calling a renderer, submitting damage, or sending frame
callback done until those resource paths have separate owner boundaries and
verification.
