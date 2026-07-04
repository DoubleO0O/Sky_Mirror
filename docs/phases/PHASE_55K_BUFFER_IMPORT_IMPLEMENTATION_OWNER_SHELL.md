# Phase 55K - Buffer Import Implementation Owner Shell

Phase 55K adds a runtime-owned buffer import implementation owner shell after
the Phase 55J execution dry-run / no-op guard.

The owner shell answers one narrow question: can the runtime preserve the
handoff evidence at the owner boundary where a future real importer
implementation would live? It does not perform the import attempt.

## Inputs

The owner shell consumes
`RuntimeSurfaceCommitBufferImportExecutionDryRunReport`. It preserves evidence
from the upstream chain:

- adapter surface id
- commit sequence
- buffer attach evidence
- present buffer evidence
- buffer removal evidence
- candidate evidence
- actual import requirement evidence
- implementation descriptor evidence
- adapter proof evidence
- precondition gate evidence
- execution dry-run evidence
- importer owner evidence
- renderer backend descriptor evidence

Candidate evidence remains separate from actual import requirement. A null
attach/remove observation can be a candidate observation while still keeping
`actual_import_required = false`.

## Owner Shell Truth

Phase 55K makes these owner shell facts visible:

- `implementation_owner_shell_available = true`
- `real_importer_implementation_available = false`
- `actual_import_attempt_admitted = false`
- `actual_import_attempt_blocked = true`

These facts mean the runtime reached a pure-data implementation owner boundary
and intentionally stopped before real resource work.

When `actual_import_required = false`, the owner shell can report a blocked
no-op because no real import is required for that commit. It must still keep
`buffer_import_attempted = false`.

When `actual_import_required = true`, the owner shell remains blocked because
there is no real buffer import implementation in Phase 55K. The blocker must
include missing real buffer import implementation, and
`buffer_import_attempted = false` still holds.

## Capability Truth

Phase 55K keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

The runtime, bounded loop, and orchestrator reports may expose implementation
owner shell results, but they still report blocked execution resources:

- no actual import required for null attach/remove or plain commits
- missing real buffer import implementation when an actual import would be
  required
- missing texture creation
- missing renderer call
- missing damage submit
- missing frame callback done

## Boundary

Phase 55K does not:

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

The next safe phase may introduce a narrower actual import attempt record or a
real importer implementation placeholder behind this owner shell. Texture
creation, renderer calls, damage submit, frame callback done, input, and core
mutation must stay separated unless that phase explicitly owns and verifies
those paths.
