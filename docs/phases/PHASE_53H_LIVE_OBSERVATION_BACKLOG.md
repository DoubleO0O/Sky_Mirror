# Phase 53H - Live Observation Backlog

## Goal

Phase 53H preserves multiple live `new_toplevel` callback observations that
arrive before the coordinator admission pump consumes them.

Phase 53G proved that distinct callbacks can be admitted sequentially when each
callback is pumped before the next callback arrives. This phase closes the
backlog gap: the display owner now keeps callback observations in FIFO order,
and the coordinator consumes one observation per live admission pump.

## Boundary Rules

- The xdg-shell handler still records only pure-data callback sequence and
  adapter identity registration observations.
- The display owner owns the pending live observation backlog.
- The accept flow exposes an owned `take_next_live_toplevel_admission_observation`
  seam so the coordinator can release the display borrow before enqueueing.
- The coordinator still owns live admission queue state and callback sequence
  dedupe state.
- Existing latest snapshot accessors remain for conservative fallback and
  duplicate-observation handling.
- This phase does not add renderer support, input handling, popup handling,
  frame callbacks, or a full desktop runtime claim.

## Capability Truth

Proven by this phase:

- Two live callback observations can arrive before any live admission pump.
- The first pump consumes the first callback observation, not the latest one.
- The second pump consumes the second callback observation.
- Each consumed observation can enqueue and drain into a distinct core surface.
- The display owner uses FIFO backlog semantics for pending live observations.

Not proven by this phase:

- A full long-running compositor event loop.
- Real user-driven multi-window desktop behavior.
- Renderable windows.
- Input handling.
