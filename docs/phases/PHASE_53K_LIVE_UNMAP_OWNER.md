# Phase 53K - Live Toplevel Unmap Owner

## Goal

Phase 53K proves the reverse lifecycle path after live toplevel admission:
a live `toplevel_destroyed` observation can be drained by the runtime owner and
submitted to `SurfaceXdgAdmissionLedger::unmap_toplevel`.

This closes the first controlled admit/unmap pair at the owner boundary without
moving core state mutation into the Smithay handler.

## Boundary Rules

- The xdg-shell handler only records adapter-owned lifecycle observations.
- The handler does not hold `State`.
- The handler does not call `SurfaceXdgAdmissionLedger::unmap_toplevel`.
- The runtime admission owner remains the mutation boundary because it owns the
  admission ledger and receives `&mut State` from the coordinator.
- Unmap goes through the existing ledger and core detach seam.
- Adapter surface mapping is retained after unmap; adapter toplevel mapping is
  removed.

## Capability Truth

Proven by this phase:

- A runtime owner can convert a destroyed lifecycle observation into
  `XdgToplevelUnmapIntent`.
- The owner can call `SurfaceXdgAdmissionLedger::unmap_toplevel`.
- The existing core detach seam removes the core window while keeping the core
  surface alive.
- A coordinator-level proof can admit a live toplevel and then detach it through
  the live unmap drain path.

Not proven by this phase:

- Renderable windows.
- Frame callbacks or commit-driven rendering.
- Input handling.
- Popup handling.
- A full long-running compositor runtime.
