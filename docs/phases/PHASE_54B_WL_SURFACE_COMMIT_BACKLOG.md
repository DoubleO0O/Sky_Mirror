# Phase 54B - wl_surface Commit Backlog

## Goal

Phase 54B preserves multiple `wl_surface.commit` observations as a FIFO backlog
owned by the adapter/display layer.

Phase 54A proved that a controlled commit reaches the Smithay handler and can
be resolved to an adapter-only surface identity. This phase prevents later
runtime owners from losing commit events when more than one commit arrives
before consumption.

## Boundary Rules

- The backlog stores only pure-data adapter commit observations or structured
  adapter identity errors.
- `WlSurface` remains inside the Smithay handler/display owner.
- The display owner exposes `take_next_wl_surface_commit_observation`.
- The handler still does not inspect buffers, damage, or frame callbacks.
- The handler still does not call the admission ledger or core state.
- No render or input capability is claimed.

## Capability Truth

Proven by this phase:

- Multiple controlled `wl_surface.commit` observations are preserved.
- Commit observations are consumed in FIFO order.
- FIFO entries retain adapter surface identity and commit sequence.

Not proven by this phase:

- Buffer-backed surfaces.
- Damage tracking.
- Frame callback request or delivery.
- Renderable windows.
- xdg role commit semantics.
- Input handling.
- A full long-running compositor runtime.
