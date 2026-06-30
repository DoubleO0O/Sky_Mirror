# Phase 53L - Loop Live Unmap Drain

## Goal

Phase 53L connects the Phase 53K live toplevel unmap owner into the bounded
nested runtime loop. Each loop iteration now uses one lifecycle pump and then
drains both live admission and live unmap observations through runtime owners.

This proves that a controlled admitted live toplevel can be detached by the
bounded loop without moving ledger or core mutation into the Smithay handler.

## Boundary Rules

- The xdg-shell handler still only records adapter-owned observations.
- The display and flow layers still only forward pure data reports.
- The bounded loop does not read handler state directly.
- The bounded loop does not call `SurfaceXdgAdmissionLedger::unmap_toplevel`
  directly.
- Admission and unmap mutation remain owned by the runtime admission queue
  owner, which holds the ledger and receives `&mut State`.
- `stop_when_idle` treats live unmap progress as non-idle, matching the earlier
  live admission backlog behavior.

## Capability Truth

Proven by this phase:

- The bounded loop can run a combined live admission and live unmap pump.
- Loop reports preserve live unmap drain counts separately from live admission
  counts.
- A live destroyed observation can detach the admitted core window during loop
  execution.
- `stop_when_idle` waits through a live unmap progress iteration before exiting
  idle.

Not proven by this phase:

- Orchestrator-level live unmap reporting.
- Renderable windows.
- Frame callbacks or commit-driven rendering.
- Input handling.
- Popup handling.
- A full long-running compositor runtime.
