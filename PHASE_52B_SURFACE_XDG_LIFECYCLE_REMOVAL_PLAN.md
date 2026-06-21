# Phase 52B Surface/XDG Lifecycle Removal Plan

## 1. Decision

Phase 52B selects **Route A: readiness/design only**.

The Phase 52A admission ledger is present and the existing core exposes both
`BackendEvent::WindowClosed -> CoreCommand::CloseWindow` and
`BackendEvent::SurfaceClosed -> CoreCommand::CloseSurface`. Those commands are
terminal close operations, however, and cannot express the required independent
XDG unmap lifecycle:

```text
toplevel unmap
-> close core WindowId
-> keep core SurfaceId alive
-> later destroy surface
-> close core SurfaceId
```

`State::close_window` calls `SurfaceRegistry::mark_dead_for_window`, so closing
the window also marks its bound surface dead. A later surface destroy would
therefore target a stale core surface. Implementing ledger removal on top of
that behavior would overstate a terminal window/surface cascade as an XDG
unmap contract.

No Rust implementation is added in this phase. The core seam must be clarified
and explicitly authorized before Route B can be implemented.

## 2. Verified Baseline

- Baseline: `main@4e92eb0`.
- Phase 52A commit: `7d025cf`.
- `SurfaceXdgAdmissionLedger` stores only pure-data adapter identities and core
  IDs.
- Surface admission dispatches `BackendEvent::SurfaceCreated` through
  `CoreRuntimeBridge`.
- Toplevel admission dispatches `BackendEvent::ToplevelMapped` through
  `CoreRuntimeBridge`.
- The ledger has no surface destroy, toplevel unmap, mapping removal, or
  lifecycle tombstone operation.
- There is no real `wl_surface` destroy callback, XDG unmap callback,
  `GlobalDispatch`, or `Dispatch` integration.

## 3. Existing Core Close Semantics

### Window close

`BackendEvent::WindowClosed { window }` translates to
`CoreCommand::CloseWindow(window)`. `State::close_window` performs all of the
following as one terminal operation:

1. removes the window from workspace state;
2. marks the `WindowRegistry` record dead;
3. marks every bound surface dead with
   `SurfaceRegistry::mark_dead_for_window`.

This is suitable for terminal close and disconnect cleanup. It is not suitable
for an XDG toplevel unmap that must leave the underlying surface alive.

### Surface close

`BackendEvent::SurfaceClosed { surface }` translates to
`CoreCommand::CloseSurface(surface)`. `State::close_surface` marks the surface
dead and then closes its bound window. This terminal cascade is compatible
with a future surface-destroy operation, but the Phase 52A ledger does not yet
remove either adapter mapping around that command.

### Missing semantic seam

The core has no operation that detaches a live surface from a window while
closing only the window lifecycle. It also has no `SurfaceRegistry` operation
that clears a `SurfaceId -> WindowId` binding while preserving the surface.

Adding such behavior is a core lifecycle/API decision. It is outside the
authorized Phase 52B file scope and must not be hidden behind a synonymous
adapter-only event.

## 4. Rejected Implementations

### Treat `CloseWindow` as XDG unmap

Rejected because it also kills the bound surface. Removing both ledger
mappings would be a terminal cascade, not the requested two-step
toplevel-unmap then surface-destroy contract.

### Remove only the toplevel ledger mapping after `CloseWindow`

Rejected because the retained adapter-surface mapping would point to a dead
core surface. The ledger would knowingly contain stale state and a later
destroy would be unable to produce a successful core lifecycle transition.

### Add an adapter-only removal path

Rejected because ledger cleanup without the existing
`BackendEvent -> CoreCommand -> CoreRuntimeBridge -> State` seam would bypass
the required mutation path.

### Change core in Phase 52B

Rejected by scope. A distinct unmap/detach semantic may be justified, but it
must be designed and approved as a core lifecycle change rather than smuggled
in as a removal-ledger detail.

## 5. Blockers

1. **Window close is terminal for bound surfaces.** It cannot preserve the
   admitted `SurfaceId` for a later destroy.
2. **No surface-window detach operation exists.** The core cannot represent an
   unmapped toplevel whose protocol surface remains alive.
3. **No accepted transaction contract exists.** The future adapter ledger must
   remove mappings only after the core accepts the lifecycle operation and
   returns a clean `ValidationReport`.
4. **No removal tombstones exist.** Unknown and duplicate removal cannot be
   distinguished after a mapping is deleted unless the ledger records a
   pure-data terminal outcome.

## 6. Capability Truth Table

| Capability | Current value | Evidence |
| --- | --- | --- |
| `surface_lifecycle_removal_contract_available` | `false` | No ledger destroy operation exists. |
| `xdg_toplevel_unmap_contract_available` | `false` | Existing window close also kills the surface. |
| `adapter_surface_removal_available` | `false` | Surface mapping has lookup/admission only. |
| `adapter_toplevel_removal_available` | `false` | Toplevel mapping has lookup/admission only. |
| `window_close_bridge_available` | `true` | Existing terminal `WindowClosed -> CloseWindow` seam is proven. |
| `surface_close_bridge_available` | `true` | Existing terminal `SurfaceClosed -> CloseSurface` seam is proven. |
| `cascade_surface_destroy_available` | `false` | Core cascades terminal close, but the adapter ledger does not coordinate mapping removal. |
| `linux_protocol_lifecycle_compile_boundary_available` | `false` | No Linux protocol lifecycle type boundary was added. |
| `real_wl_surface_destroy_runtime_available` | `false` | No real callback exists. |
| `real_xdg_toplevel_unmap_runtime_available` | `false` | No real callback exists. |
| `protocol_dispatch_started` | `false` | No globals or request dispatch were added. |
| `render_support` | `false` | Out of scope and not implemented. |
| `input_support` | `false` | Out of scope and not implemented. |

The two `*_close_bridge_available` values describe existing terminal core
operations only. They do not imply that the Phase 52B removal contract exists.

## 7. Required Core Decision Before Route B

The next authorized design slice must choose one explicit semantic model:

1. add a distinct toplevel-unmap/detach command that closes the window while
   clearing its surface link and preserving the surface; or
2. redefine `CloseWindow` so it no longer kills bound surfaces, then move all
   required surface cascades into callers such as surface/client close.

The first option is narrower and preserves existing terminal close behavior,
but it requires a genuinely distinct core lifecycle operation rather than a
synonym. The second option changes established behavior and has a larger
regression surface.

After that decision is accepted, Route B can add pure-data intents, structured
unknown/duplicate/stale errors, terminal tombstones, bridge-backed mutation,
and transactional mapping removal to `SurfaceXdgAdmissionLedger` using TDD.

## 8. Verification Matrix

This Route A change is documentation-only. Verification must still cover the
unchanged default and `smithay-probe` builds:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
cargo tree --features smithay-probe \
  | rg -i 'smithay|wayland|udev|libinput|drm|gbm|x11|vulkan' || true
git diff --check
```

`smithay-linux` is not run locally on macOS. GitHub Actions remains the source
of Linux-only verification after this branch is pushed.

## 9. Non-goals

This plan does not claim or add:

- real `wl_surface` destroy runtime;
- real XDG toplevel unmap/close runtime;
- protocol globals, `GlobalDispatch`, or `Dispatch`;
- renderer or visible compositor output;
- keyboard, pointer, or seat input;
- DRM, GBM, or libinput integration;
- a daily usable compositor.
