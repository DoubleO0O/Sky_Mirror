# Phase 53C - Bounded Loop Live Admission Pump

## Goal

Phase 53C connects the Phase 53B coordinator live admission pump into
`NestedRuntimeLoop::run_for_iterations`.

Each bounded loop iteration now executes the coordinator path that:

1. Runs the existing nested lifecycle pump.
2. Reads the flow-owned display's live toplevel admission observation snapshot.
3. Enqueues the observation through the live admission owner.
4. Drains one pending xdg_toplevel admission through the runtime admission queue owner.

## Route

This phase keeps the public bounded-loop report shape stable. The loop stores the
coordinator `lifecycle_report` in the existing `pump_reports` vector, while the
live admission enqueue and drain remain owned by the coordinator and runtime
queue owner.

The loop still does not directly read or mutate core registries, admission
ledger state, workspace slots, or handler state.

## Capability Truth

Proven by this phase:

- The bounded loop calls the live admission pump on each iteration.
- A controlled live xdg_toplevel observation can be admitted through the loop.
- The existing lifecycle report accounting remains stable for orchestrator users.

Not proven by this phase:

- Full long-running compositor runtime.
- Renderable windows.
- Input handling.
- Real desktop session behavior.

## Verification

Expected verification:

- Default Rust tests for non-Linux source contracts.
- Linux `smithay-linux` test compile for the live loop admission proof.
- GitHub Linux CI for running the Linux-only proof.
