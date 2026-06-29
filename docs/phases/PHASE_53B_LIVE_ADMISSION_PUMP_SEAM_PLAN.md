# Phase 53B - Live Admission Pump Seam

## Goal

Phase 53B connects the Phase 53A live callback admission owner to the Phase 52Z
runtime admission drain from inside `NestedRuntimeCoordinator`.

The new coordinator seam performs one bounded sequence:

1. Run the existing nested lifecycle pump.
2. Read a pure-data live admission observation snapshot from the display owner.
3. Ask the live admission owner to enqueue a pending xdg_toplevel admission.
4. Drain one pending admission through the runtime admission queue owner.

## Route

This is a narrow coordinator owner seam, not a render/input/runtime promotion.

- `NestedRealAcceptFlow` still owns the Wayland display.
- The flow exposes only a copyable live admission observation snapshot.
- `NestedRuntimeCoordinator` passes that snapshot to the live admission owner.
- The live admission owner enqueues into the coordinator-owned runtime queue.
- The existing runtime queue owner consumes the pending intent with ledger and
  core `State` ownership.

## Boundary Rules

- Handler state does not hold `State`.
- Handler/display code does not call admission ledger or core registration.
- Adapter IDs are not treated as core `WindowId`.
- The coordinator does not directly mutate workspace, slot, or stack state.
- Render and input remain unsupported.
- The new display mutable accessor is test-only and exists only to build a
  controlled live observation on the flow-owned display.

## Capability Truth

Proven by this phase:

- A live callback/identity observation can be read from the flow-owned display.
- The coordinator can enqueue that observation through the Phase 53A owner.
- The same coordinator pump can drain the queued admission through the existing
  runtime admission queue owner.
- Ledger/core admission still happens only in the runtime queue owner.

Not proven by this phase:

- Full long-running compositor runtime.
- Renderable windows.
- Input handling.
- Real desktop session behavior.

## Verification

Expected verification:

- Default Rust tests for source contracts and non-Linux gates.
- `smithay-probe` check/test for probe-only compatibility.
- Linux `smithay-linux` CI for the live coordinator pump integration test.

