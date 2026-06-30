# Phase 53G - Multiple Live Admission Proof

## Goal

Phase 53G proves that one coordinator/display can admit two distinct live
`new_toplevel` callback observations in sequence.

Phase 53F made duplicate callback observations idempotent. This phase verifies
that the same dedupe state does not reject a later callback with a different
sequence, adapter surface identity, and adapter toplevel identity.

## Boundary Rules

- The coordinator remains the owner of live admission queue state and callback
  sequence dedupe state.
- The proof uses the existing controlled adapter identity registration harness.
- Each live observation still enters core only through the live admission owner,
  runtime admission queue, pending admission consumer, ledger, and core command
  seams.
- This phase does not add a renderer, input path, protocol event loop upgrade,
  frame callbacks, or a daily-use desktop claim.

## Capability Truth

Proven by this phase:

- A first callback observation can be enqueued, drained, and mapped into
  ledger/core.
- A second distinct callback observation on the same coordinator/display can be
  enqueued, drained, and mapped into ledger/core.
- Runtime core surface allocation advances once per consumed admission.
- The Phase 53F duplicate-observation guard does not reject distinct callback
  sequences.

Not proven by this phase:

- A full long-running compositor event loop.
- Real user-driven multi-window desktop behavior.
- Renderable windows.
- Input handling.
