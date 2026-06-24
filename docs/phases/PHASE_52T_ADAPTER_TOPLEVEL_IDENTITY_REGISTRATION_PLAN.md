# Phase 52T: Adapter Toplevel Identity Registration Plan

## Goal

Route B: prove that a controlled `xdg_surface.get_toplevel` request reaches
`XdgShellHandler::new_toplevel`, and that the Linux adapter registers a pure
data `AdapterToplevelId` in its adapter-owned toplevel identity registry.

## Boundary

- `new_toplevel` callback observed is not the same as ledger admission.
- `AdapterToplevelId` is not core `WindowId`.
- Adapter identity registration is not a renderable window.
- The proof uses a bounded in-process Unix stream pair; it is not a real
  compositor runtime.
- Smithay/Wayland objects remain in `src/smithay_backend`; core receives no
  Smithay types.

## Implementation Shape

1. Keep the controlled client path from Phase 52S: bind `wl_compositor`, create
   `wl_surface`, bind `xdg_wm_base`, create `xdg_surface`, then create
   `xdg_toplevel`.
2. In `LinuxXdgShellStateSkeleton::new_toplevel`, record the callback count and
   immediately derive adapter identity:
   - `LinuxXdgToplevelIdentityRegistry::key_for_toplevel(surface)` extracts the
     stable toplevel key.
   - `surface.wl_surface()` is observed through the existing adapter surface
     identity registry.
   - `LinuxXdgToplevelIdentityRegistry::register` allocates
     `AdapterToplevelId`.
3. Store only the pure data registration observation. Do not store
   `ToplevelSurface`.
4. Report adapter identity registration as true while keeping ledger/core/window,
   render, input, and real runtime capability false.

## Non-Goals

- No `SurfaceXdgAdmissionLedger::admit_toplevel`.
- No `SurfaceXdgAdmissionLedger::unmap_toplevel`.
- No `BackendEvent::ToplevelMapped` or `BackendEvent::ToplevelUnmapped`.
- No `CoreCommand::RegisterWindowForSurface` or
  `CoreCommand::DetachWindowFromSurface`.
- No core `SurfaceRegistry`, `WindowRegistry`, workspace, or `WindowId`
  allocation.
- No renderer, seat/input, DRM/GBM/libinput, or real socket runtime claim.

## Verification

- macOS local checks cover default and `smithay-probe`.
- Linux target typecheck covers `smithay-linux`.
- Full Linux runtime tests remain CI-owned because macOS cannot execute Linux
  target binaries.
