# Phase 55L - Buffer Import Actual Attempt Record

Phase 55L adds a runtime-owned actual buffer import attempt admission / record
seam after the Phase 55K implementation owner shell.

The record answers one narrow question: when the runtime reaches the future
actual import attempt boundary, can it preserve the admission decision and the
upstream owner shell evidence without performing a real import attempt?

## Inputs

The recorder consumes
`RuntimeSurfaceCommitBufferImportImplementationOwnerShellReport`. It preserves:

- adapter surface evidence through the observed owner shell report
- commit sequence evidence through the observed owner shell report
- buffer attach evidence
- present buffer evidence
- buffer removal evidence
- candidate evidence
- actual import requirement evidence
- implementation owner shell evidence
- execution dry-run evidence
- importer owner evidence
- renderer backend descriptor evidence

Candidate evidence remains separate from actual import requirement. A null
attach/remove observation can still be recorded while keeping
`actual_import_required = false`.

## Attempt Record Truth

Phase 55L makes these actual attempt record facts visible:

- `actual_attempt_record_available = true`
- `actual_attempt_recorded = true`
- `actual_attempt_admission_checked = true`
- `actual_attempt_admitted = false`
- `actual_attempt_blocked = true`

These facts mean the runtime reached the pure-data actual attempt admission
record boundary and intentionally stopped before touching Wayland buffers.

When `actual_import_required = false`, the record is blocked because no actual
import is required. When `actual_import_required = true`, the record is blocked
because Phase 55L still has no real buffer import implementation and the
upstream owner shell did not admit an attempt.

## Capability Truth

Phase 55L keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

The runtime, bounded loop, and orchestrator reports may expose actual attempt
records, but they still report blocked execution resources:

- missing actual attempt admission
- no actual import required for null attach/remove or plain commits
- missing real buffer import implementation when an actual import would be
  required
- missing texture creation
- missing renderer call
- missing damage submit
- missing frame callback done

## Boundary

Phase 55L does not:

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

The next safe phase may introduce a real importer implementation placeholder
behind the actual attempt admission record. It must still avoid texture
creation, renderer calls, damage submit, frame callback done, input, and core
mutation unless that later phase explicitly owns and verifies those paths.
