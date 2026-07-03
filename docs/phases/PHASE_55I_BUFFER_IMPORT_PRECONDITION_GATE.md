# Phase 55I - Buffer Import Precondition Gate

Phase 55I adds a pure-data precondition gate after the Phase 55H buffer import
adapter proof boundary.

The gate answers one narrow question: whether the runtime has enough recorded
evidence to permit a future real buffer import attempt to be scheduled by a
later owner. It does not perform that attempt.

## Inputs

The gate consumes `RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport`.
It preserves adapter-owned evidence:

- adapter surface id
- commit sequence
- buffer attach evidence
- present buffer evidence
- buffer removal evidence
- candidate evidence
- actual import requirement evidence
- importer owner evidence
- renderer backend descriptor evidence

Candidate evidence remains separate from actual import requirement. A null
attach can be a candidate observation while still keeping
`actual_import_required = false`.

## Gate Truth

The gate can report:

- `import_precondition_gate_available = true`
- `import_preconditions_met = true`
- `future_import_preconditions_met = true`
- `actual_import_required = true`

Those true values mean only that a future owner has enough pure-data evidence to
consider a real import attempt. They do not mean a buffer was imported, a texture
was created, or a renderer was called.

The minimal preconditions are:

- adapter proof registered
- candidate evidence observed
- actual import required
- buffer present
- buffer not removed
- importer owner evidence available
- renderer backend descriptor evidence available

## Capability Truth

Phase 55I keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

The runtime, bounded loop, and orchestrator reports may expose precondition gate
results, but they still report blocked execution resources:

- missing actual buffer import
- missing texture creation
- missing renderer call
- missing damage submit
- missing frame callback done

## Boundary

Phase 55I does not:

- import a Wayland buffer
- create a texture
- call a renderer
- submit damage
- send frame callback done
- connect input
- mutate core state
- construct or guess `WindowId`

Smithay handlers still only produce adapter-owned observations. They do not hold
or mutate `State`.

## Next Safe Step

The next safe phase may introduce an importer execution owner shell or an
explicit real-import attempt gate. It must still keep texture creation, renderer
calls, damage submit, frame callback done, input, and core mutation as separate
capability truths unless that phase explicitly owns and verifies those paths.
