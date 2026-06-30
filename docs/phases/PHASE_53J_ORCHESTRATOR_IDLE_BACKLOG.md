# Phase 53J - Orchestrator Idle Backlog Proof

## Goal

Phase 53J proves that the runtime orchestrator inherits the bounded loop's
live-admission-aware idle behavior.

Phase 53I fixed the loop so `stop_when_idle` does not exit while a live
admission pump made enqueue or consume progress. This phase verifies the same
behavior through the orchestrator start/run lifecycle and final report.

## Boundary Rules

- The orchestrator still only owns lifecycle state and delegates runtime work to
  `NestedRuntimeLoop`.
- The orchestrator does not read display handler state directly.
- The orchestrator does not enqueue admission intents directly and does not
  touch ledger/core outside the loop/coordinator path.
- The final lifecycle report mirrors the loop's live admission summary.
- This phase does not add renderer support, input handling, popup handling,
  frame callbacks, or a full desktop runtime claim.

## Capability Truth

Proven by this phase:

- An orchestrator run with `stop_when_idle` can drain a two-observation live
  admission backlog before exiting idle.
- The orchestrator final report preserves the loop live admission summary.
- The orchestrator remains a lifecycle wrapper over the bounded loop.

Not proven by this phase:

- A full long-running compositor event loop.
- Real user-driven multi-window desktop behavior.
- Renderable windows.
- Input handling.
