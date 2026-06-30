# Phase 54A - wl_surface Commit Observation

## Goal

Phase 54A turns the server-side `CompositorHandler::commit` callback from a
no-op into an adapter-owned observation seam.

The handler records which already-observed adapter surface committed, using
only pure data. This is the first step toward commit-driven rendering, but it
does not inspect buffers, damage, frame callbacks, or renderer state.

## Boundary Rules

- `WlSurface` remains inside the Smithay handler/display owner.
- Commit observation reuses the adapter surface identity created by
  `new_surface`.
- Unknown commits are recorded as structured adapter identity errors.
- The handler does not call the admission ledger.
- The handler does not call core state or `CoreRuntimeBridge`.
- The proof client only calls `create_surface` and `commit`.
- No buffer attach, damage, frame callback, xdg role, render, or input support
  is claimed.

## Capability Truth

Proven by this phase:

- A controlled `wl_surface.commit` request reaches the server commit handler.
- The commit callback is resolved to the existing adapter-only surface identity.
- The display owner can expose the latest pure-data commit observation.

Not proven by this phase:

- Buffer-backed surfaces.
- Damage tracking.
- Frame callback request or delivery.
- Renderable windows.
- xdg role commit semantics.
- Input handling.
- A full long-running compositor runtime.
