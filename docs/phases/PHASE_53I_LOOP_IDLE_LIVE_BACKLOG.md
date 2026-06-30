# Phase 53I - Loop Idle Live Backlog Drain

## Goal

Phase 53I prevents the bounded nested runtime loop from treating a pump as idle
when that pump made live toplevel admission progress.

Phase 53H added FIFO backlog ownership for live `new_toplevel` observations.
This phase closes the loop-level idle gap: `stop_when_idle` must not exit after
the first lifecycle-idle pump if that same pump consumed a pending live
admission observation.

## Boundary Rules

- The bounded loop still exits on true idle, stop, error, interrupt, or
  `max_iterations`.
- Live admission progress is a loop-level observation derived from the existing
  coordinator pump report.
- The loop does not read display handler state directly.
- The loop does not enqueue admission intents directly and does not touch
  ledger/core outside the coordinator pump.
- This phase does not add renderer support, input handling, popup handling,
  frame callbacks, or a full desktop runtime claim.

## Capability Truth

Proven by this phase:

- `stop_when_idle` does not exit while live admission enqueue/consume progress
  is happening.
- A live observation backlog can be drained across bounded loop iterations.
- The loop exits as `Idle` only after a later pump has no lifecycle activity and
  no live admission progress.

Not proven by this phase:

- A full long-running compositor event loop.
- Real user-driven multi-window desktop behavior.
- Renderable windows.
- Input handling.
