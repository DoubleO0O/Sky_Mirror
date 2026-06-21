# Phase 52E Linux XDG-Shell Compile Seam Plan

## 1. Baseline

- Base: `main@5d8ca70`.
- Phase 52A-B provides the pure-data surface/XDG admission ledger.
- Phase 52C-B provides `BackendEvent::ToplevelUnmapped` through
  `CoreCommand::DetachWindowFromSurface` to core detach semantics.
- Phase 52B-B provides duplicate/orphan/stale-safe ledger unmap behavior.
- Phase 52D-A records that no production xdg-shell global, request handler, or
  toplevel lifecycle callback source existed at that baseline.

## 2. Selected Route

Phase 52E selects **Route B: Linux-only xdg-shell compile seam**.

Smithay 0.7 exposes `XdgShellState`, `XdgShellHandler`, and
`delegate_xdg_shell!`. The existing display probe already provides the narrow
place to install an internal handler-state owner, without changing the public
`SmithayWaylandState` shape or any core API.

## 3. Compile Boundary

The boundary is available only under:

```rust
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
```

The display's private state is now `LinuxXdgShellStateSkeleton`, which combines
the unchanged public `SmithayWaylandState` with an `Option<XdgShellState>`.
The option remains `None`, preserving the old public state shape while placing
future protocol ownership next to the display. Smithay delegation macros
establish the real `GlobalDispatch` and non-popup request `Dispatch` type
implementations, while the `XdgShellHandler` implementation identifies where
future callbacks belong. Popup gets only a fail-closed compile handler because
Smithay's full xdg-shell macro requires `SeatHandler`; Phase 52E must not invent
input/seat state merely to satisfy that transitive bound.

Phase 52E deliberately does **not** call `XdgShellState::new`. That call would
create the `xdg_wm_base` global and would cross from compile proof into runtime
initialization. Therefore no xdg-shell global is registered and no real request
can reach the handler in this phase.

## 4. Future Identity and Lifecycle Path

The future eligible path is:

```text
Smithay ToplevelSurface lifecycle signal
-> adapter-owned object identity lookup
-> AdapterToplevelId
-> SurfaceXdgAdmissionLedger::unmap_toplevel
-> existing BackendEvent / CoreCommand / State seam
```

The real Smithay object must remain in the Linux adapter. It must never enter
core. `toplevel_destroyed` is only the future hook location; Phase 52E does not
derive `AdapterToplevelId`, call the ledger, or mutate core.

## 5. Capability Truth

The following compile facts are true:

- Linux xdg-shell module exists.
- xdg-shell global dispatch type boundary compiles on Linux CI.
- xdg-shell request dispatch type boundary compiles on Linux CI.
- `ToplevelSurface` lifecycle handler has a future adapter identity hook point.

The following remain false:

- `xdg_unmap_callback_observed`
- `ledger_unmap_invoked_from_linux_boundary`
- `real_xdg_shell_runtime_available`
- `protocol_dispatch_started`
- `render_support`
- `input_support`

This phase does not provide `wl_surface` lifecycle admission, a usable
xdg-shell runtime, rendering, keyboard/pointer/seat, DRM, GBM, or libinput.

## 6. Remaining Blockers

- `MissingGlobalInitialization`: no `XdgShellState::new` call and no real
  `xdg_wm_base` global registration.
- `MissingAdapterToplevelIdentityMapping`: no mapping from `ToplevelSurface` to
  `AdapterToplevelId`.
- `MissingToplevelLifecycleBridge`: no production lifecycle callback forwards
  to the pure-data ledger.
- `MissingLedgerCallerOwnership`: no Linux runtime owner currently holds and
  invokes the ledger from protocol callbacks.
- `MissingPopupSeatHandlerBoundary`: Smithay's normal popup delegation requires
  `SeatHandler`; this phase intentionally does not implement input/seat.

## 7. Verification Strategy

Default and `smithay-probe` builds verify exact feature gates, source-level
absence of core/ledger mutation, and conservative capability values. Linux
GitHub Actions is authoritative for real Smithay trait/macro compilation and
the `smithay-linux` test matrix. A macOS checkout must not claim that local
Linux-only verification ran.

## 8. Recommended Next Phase

The next phase should design and prove the adapter-owned identity mapping from
Smithay `ToplevelSurface` to `AdapterToplevelId`, including stale object and
ownership rules. Only after that mapping has a clear owner should a narrowly
scoped lifecycle callback invoke the existing ledger unmap seam. Global runtime
startup, render, input, and hardware backends remain separate later phases.
