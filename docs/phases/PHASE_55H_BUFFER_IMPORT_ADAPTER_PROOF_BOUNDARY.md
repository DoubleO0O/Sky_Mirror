# Phase 55H - Buffer Import Adapter Proof Boundary

## Scope

Phase 55H adds a runtime-owned pure-data buffer import adapter proof boundary after the Phase 55G buffer import implementation descriptor seam.

The boundary consumes `RuntimeSurfaceCommitBufferImportImplementationBoundaryReport` and produces `RuntimeSurfaceCommitBufferImportAdapterProofBoundaryReport`. It preserves adapter-owned evidence needed by a future real importer handoff without importing buffers, creating textures, calling a renderer, submitting damage, sending frame callback done, handling input, or mutating core state.

## Chain Position

The Phase 55H seam is downstream of:

- wl_surface.commit FIFO observation
- render-dirty readiness intent
- render operation intent
- renderer backend owner shell readiness
- buffer importer resource owner readiness
- buffer import planning
- buffer import implementation descriptor / adapter boundary

It is upstream of any future real buffer import implementation. It records proof that an adapter/importer handoff can receive the descriptor evidence, but it does not execute the handoff.

## Adapter Proof Truth

When the upstream implementation descriptor is present, the adapter proof report preserves:

- adapter surface id
- surface identity key
- commit sequence
- buffer attach / present / removed evidence
- candidate evidence observed
- actual import required
- importer owner evidence
- renderer backend descriptor evidence
- registered renderer backend kind

The expected report truth for a proof-bearing commit is:

- adapter_proof_boundary_available = true
- adapter_proof_registered = true
- actual_import_required = true

`actual_import_required = true` means a future real importer would need to act on the observed candidate. It does not mean Phase 55H imported a buffer.

## Capability Truth

Phase 55H keeps every real runtime/render/input/core capability false:

- buffer_import_attempted = false
- buffer_imported = false
- texture_created = false
- renderer_called = false
- damage_submitted = false
- frame_callback_done_sent = false
- input_support = false
- core_mutation_invoked = false

The adapter proof is pure data. It does not make a surface renderable, does not create a texture, and does not submit work to a renderer.

## Blocked Paths

The report keeps structured blockers for work that still belongs to later phases:

- missing implementation descriptor
- missing importer owner evidence
- missing renderer backend descriptor evidence
- missing buffer import candidate evidence
- missing actual buffer import
- missing texture creation
- missing renderer call
- missing damage submit
- missing frame callback done

Idle pumps may report the missing implementation descriptor blocker. Plain commits without buffer evidence may report missing candidate evidence while still preserving FIFO ordering.

## Validation Intent

Phase 55H validation should prove:

- multiple adapter proofs are exposed in FIFO commit order
- candidate evidence does not become actual import execution
- actual import required is preserved as pure data
- runtime loop and orchestrator reports expose the adapter proof boundary
- no buffer import, texture creation, renderer call, damage submit, frame callback done, input, or core mutation is claimed

## Next Safe Step

The next safe phase can define the minimal real importer precondition gate. That phase must still avoid real renderer calls unless it explicitly owns the renderer path and keeps frame callback done, damage submit, input, and core mutation truth separated.
