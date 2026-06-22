# Phase 52M Linux wl_compositor Owner Boundary Plan

## 1. Baseline and Decision

- Base: `main@16971bf`, including accepted Phase 52L-B commit `c80c8b3`.
- Phase 52I-B owns an explicitly initialized xdg-shell global on
  `SmithayWaylandDisplayProbe`.
- Phase 52L-B exposes Linux-only Wayland client types but creates no client
  connection, event queue, bind, or lifecycle harness.
- Phase 52M selects **Route A: Smithay compositor API blocker docs-only**.

This phase adds no Rust behavior. It does not initialize `wl_compositor`, own
`CompositorState`, observe `WlSurface`, implement client binding, or enter
ledger/core mutation.

## 2. Smithay 0.7 API Evidence

Smithay 0.7 provides the required server-side protocol implementation under
its existing `wayland_frontend` feature:

- `CompositorState::new::<D>(&DisplayHandle)` registers version 5
  `wl_compositor` and version 1 `wl_subcompositor` globals;
- `CompositorState::new_v6::<D>` selects compositor version 6;
- `delegate_compositor!(D)` supplies the required global and resource dispatch
  delegation for compositor, subcompositor, surfaces, regions, and callbacks;
- `D` must implement `CompositorHandler`;
- each server-side client must own a `CompositorClientState` reachable through
  its `ClientData`;
- the handler must return its owned `CompositorState` and implement surface
  `commit` handling.

Official references:

- <https://docs.rs/smithay/0.7.0/smithay/wayland/compositor/index.html>
- <https://docs.rs/smithay/0.7.0/smithay/macro.delegate_compositor.html>

The API itself does not require `SeatHandler`, renderer, input, DRM, GBM, or
libinput. Those systems remain unrelated to this owner boundary.

## 3. Existing Owner Topology

The current display/state pairing is:

```text
SmithayWaylandDisplayProbe
├── Display<LinuxXdgShellStateSkeleton>
└── LinuxXdgShellStateSkeleton
    └── Option<XdgShellState>
```

The xdg-shell owner is safe because its explicit initializer obtains the
`DisplayHandle` from the paired display, constructs `XdgShellState`, and stores
it in that display's state. Duplicate initialization is rejected before
mutation.

The existing server client insertion path uses:

```text
NestedClientDataOwner
├── NestedClientSessionId
└── NestedClientCallbackEventQueue
```

It does not own Smithay's `CompositorClientState`.

## 4. Why Route B Is Blocked in the Approved File Scope

A correct compositor owner cannot be added only to
`wayland_display.rs`, `mod.rs`, and a new module.

### 4.1 Display state storage is missing

`CompositorHandler::compositor_state` must return `&mut CompositorState` from
the actual display state. The display state is `LinuxXdgShellStateSkeleton`,
but that type has no compositor field. Adding transactional storage requires a
change to `src/smithay_backend/linux_xdg_shell.rs`, which is outside the Phase
52M allowed modification list.

Storing `CompositorState` in `SmithayWaylandDisplayProbe` alone does not satisfy
the trait: dispatch is parameterized by the display state, not by the outer
owner object.

### 4.2 Per-client compositor state is missing

`CompositorHandler::client_compositor_state` returns a reference tied to the
Wayland `Client`. Smithay requires that state to live in the client's
`ClientData` for correct isolation and cleanup. The accepted insertion owner,
`NestedClientDataOwner`, has no `CompositorClientState` field.

Adding that field and preserving session/disconnect behavior requires a change
to `src/smithay_backend/client_insert.rs`, also outside the approved file list.

### 4.3 Unsafe substitutes are rejected

Phase 52M must not use any of these shortcuts:

- a global/static `CompositorClientState` shared by all clients;
- a leaked or fabricated reference to satisfy the handler lifetime;
- an `unreachable!`/panic implementation for client lookup;
- a second standalone `Display` that separates `wl_compositor` from the
  accepted xdg-shell owner;
- a new client-data type that the existing insertion path never installs.

These approaches may compile, but they do not form a coherent owner boundary.
Some would panic when the first client binds; a second display would prevent a
future controlled client from seeing compositor and xdg-shell globals on the
same server connection.

## 5. Route B Authorization and Design Prerequisites

Route B can be reconsidered after explicitly authorizing this narrow expansion:

1. Modify `linux_xdg_shell.rs` so `LinuxXdgShellStateSkeleton` transactionally
   owns `Option<CompositorState>` alongside `Option<XdgShellState>`.
2. Modify `client_insert.rs` so `NestedClientDataOwner` owns a
   `CompositorClientState` while preserving session identity and disconnect
   callbacks.
3. Implement `CompositorHandler` for the actual display state and use
   `delegate_compositor!(LinuxXdgShellStateSkeleton)`.
4. Add an explicit `SmithayWaylandDisplayProbe::initialize_wl_compositor_global`
   method that supplies only its own display handle.
5. Define `new_surface`, `commit`, and `destroyed` as conservative callback
   boundaries that neither allocate adapter/core IDs nor claim callback
   observation without runtime dispatch.
6. Reject duplicate initialization before mutation and retain the original
   `CompositorState`.
7. Keep all APIs under
   `cfg(all(feature = "smithay-linux", target_os = "linux"))` and expose only
   pure-data readiness reports.

No Cargo, core, backend, main, or CI change is required for that future slice.

## 6. Future Route B Capability Semantics

After a successful future explicit initializer, these facts may become true:

```text
global_owner_available = true
compositor_state_new_invoked = true
wl_compositor_global_initialized = true
wl_compositor_state_owned = true
wl_surface_owner_boundary_available = true
```

Even then, the following must remain independently proven:

```text
wl_compositor_global_initialized
!= client_bound_wl_compositor

wl_surface_owner_boundary_available
!= wl_surface_created_or_observed
!= surface_identity_registry_available
!= core_register_invoked

CompositorState owned
!= real_compositor_runtime_available
```

Route C must remain separate until a real client-dispatched `WlSurface` object
can be observed and converted to an adapter-owned identity without touching
core.

## 7. Phase 52M-A Capability Truth

Because this phase is documentation-only, all new compositor-owner facts remain
false:

```text
global_owner_available = false
compositor_state_new_invoked = false
wl_compositor_global_initialized = false
wl_compositor_state_owned = false
wl_surface_owner_boundary_available = false
wl_surface_identity_registry_available = false
client_connection_created = false
event_queue_created = false
registry_bind_attempted = false
client_bound_wl_compositor = false
client_harness_available = false
xdg_surface_lifecycle_available = false
xdg_toplevel_lifecycle_available = false
new_toplevel_identity_registration_owner_available = false
ledger_admit_invoked = false
ledger_unmap_invoked = false
core_register_invoked = false
core_detach_invoked = false
protocol_dispatch_started = false
real_compositor_runtime_available = false
real_xdg_shell_runtime_available = false
render_support = false
input_support = false
```

The accepted Phase 52I xdg-shell global owner and Phase 52L client type compile
seam remain unchanged. Neither implies a compositor global exists.

## 8. Preserved Boundaries

Phase 52M-A does not:

- call `CompositorState::new` or `new_v6`;
- invoke `delegate_compositor!` or implement `CompositorHandler`;
- alter `LinuxXdgShellStateSkeleton` or `NestedClientDataOwner`;
- create `wayland_client::Connection`, event queue, registry, or global bind;
- observe, store, commit, destroy, or identify a real `WlSurface`;
- create `xdg_surface` or `xdg_toplevel` lifecycle;
- call `SurfaceXdgAdmissionLedger`, emit mapped/unmapped events, or execute
  core register/detach commands;
- modify `SurfaceRegistry`, `WindowRegistry`, `Workspace`, render, input, DRM,
  GBM, or libinput.

## 9. Verification

Run locally:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
git diff --check
```

Verify that the `smithay-probe` dependency tree remains free of Smithay and
Wayland runtime dependencies. Linux `smithay-linux` check/tests remain GitHub
Actions evidence; macOS results must not be presented as Linux proof.

## 10. Recommended Next Phase

Request a narrowly expanded Phase 52M-B authorization covering only:

- `linux_xdg_shell.rs` compositor-state ownership;
- `client_insert.rs` per-client `CompositorClientState` ownership;
- `linux_wl_compositor.rs`, `wayland_display.rs`, and `mod.rs` delegation,
  explicit initialization, reports, and tests.

That phase must still prohibit client connection creation, registry bind,
surface identity mapping, xdg lifecycle, ledger/core, render, and input. Route C
should follow only after B is accepted and a separate runtime surface-observation
proof is approved.
