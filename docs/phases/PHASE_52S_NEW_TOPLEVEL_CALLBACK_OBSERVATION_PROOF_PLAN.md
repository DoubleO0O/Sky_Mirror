# Phase 52S: Controlled New Toplevel Callback Observation Proof Plan

## Goal

Phase 52S proves that a controlled Linux-only client path can drive
`xdg_surface.get_toplevel` far enough for the server-side xdg-shell handler to
observe `new_toplevel`.

The proof remains deliberately narrow:

- it records only pure data callback count and sequence;
- it does not store the Smithay protocol object;
- it does not register adapter toplevel identity;
- it does not call the admission ledger or core;
- it does not allocate a core window identity;
- it does not add render, input, seat, DRM, GBM, or libinput support.

## Preconditions

The proof requires both server-side globals to be explicitly initialized:

- xdg-shell global owner via `SmithayWaylandDisplayProbe`;
- `wl_compositor` owner via the same paired display/state owner.

Missing owners return structured blockers. The proof does not connect to a
system Wayland socket; it creates a controlled Unix stream pair and inserts only
the server endpoint through the existing nested client insertion seam.

## Route

The client route follows the existing controlled creation ladder:

1. create a `wayland-client` connection from the controlled socket;
2. initialize registry discovery with `registry_queue_init`;
3. bind `wl_compositor`;
4. create `wl_surface`;
5. bind `xdg_wm_base`;
6. create `xdg_surface`;
7. call `xdg_surface.get_toplevel`;
8. run a bounded dispatch/flush loop and a client roundtrip.

The server handler increments `new_toplevel_callback_count` and stores the last
observation sequence. No Smithay toplevel object is persisted.

## Success Evidence

A successful report sets:

- `new_toplevel_callback_expected = true`;
- `new_toplevel_callback_observed = true`;
- `new_toplevel_callback_count > 0`;
- `protocol_dispatch_started = true`.

The same report keeps these boundaries false:

- adapter toplevel identity registration;
- admission ledger admit/unmap;
- core register/detach;
- core window identity allocation;
- render/input support;
- real compositor or xdg-shell runtime availability.

## Feature Gate

All public API for this proof is guarded by:

```rust
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
```

Default builds keep only source-boundary tests visible. Linux CI is responsible
for compiling and executing the controlled runtime proof.

## Next Boundary

The next phase may decide whether the observed callback should be converted into
an adapter-owned toplevel identity. That must be a separate promotion step with
its own ownership rules and must not be inferred from this observation proof.
