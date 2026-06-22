# Phase 52M-B Linux wl_compositor State Owner Seam

## 1. Baseline and Route

- Base: main@0ccac3d, including accepted Phase 52M-A commit 69a83de.
- Main and origin/main matched and main CI was green before implementation.
- This phase selects **Route B: Linux-only state owner + per-client data seam**.

The phase creates no Wayland client connection, event queue, registry bind,
controlled client harness, xdg surface/toplevel lifecycle, ledger/core
mutation, renderer, input, DRM, GBM, or libinput integration.

## 2. Smithay 0.7 Ownership Evidence

The locked Smithay 0.7.0 source defines:

    CompositorState::new<D>(&DisplayHandle) -> CompositorState

    CompositorHandler:
      compositor_state(&mut self) -> &mut CompositorState
      client_compositor_state(&self, &Client) -> &CompositorClientState
      commit(&mut self, &WlSurface)

new_surface, new_subsurface, and destroyed have default implementations.
delegate_compositor!(D) supplies global/resource dispatch for compositor,
subcompositor, surface, region, callback, and subsurface protocol objects.

Smithay requires each downstream ClientData implementation to own its own
CompositorClientState so it can be cleaned up with that client.

References:

- <https://docs.rs/smithay/0.7.0/smithay/wayland/compositor/index.html>
- <https://docs.rs/smithay/0.7.0/smithay/macro.delegate_compositor.html>
- local locked source: smithay-0.7.0/src/wayland/compositor/mod.rs
- local locked example: smithay-0.7.0/examples/compositor.rs

The API does not require SeatHandler, render, input, DRM, GBM, or libinput.

## 3. Owner Topology

    SmithayWaylandDisplayProbe
    ├── Display<LinuxXdgShellStateSkeleton>
    └── LinuxXdgShellStateSkeleton
        ├── Option<XdgShellState>
        └── Option<CompositorState>

    NestedClientDataOwner
    ├── NestedClientSessionId
    ├── NestedClientCallbackEventQueue
    └── CompositorClientState

The public initializer lives on SmithayWaylandDisplayProbe. It obtains the
handle from its own display and passes it to the crate-private state operation,
so callers cannot inject a mismatched display handle through the new API.

Duplicate initialization is rejected before mutation with
LinuxWlCompositorGlobalInitError::AlreadyInitialized; the original
CompositorState remains owned.

## 4. Handler and Dispatch Boundary

LinuxXdgShellStateSkeleton implements the minimal CompositorHandler:

- compositor_state returns the state created by explicit initialization;
- client_compositor_state reads the per-client state from the existing
  NestedClientDataOwner;
- commit is intentionally a no-op boundary.

The two required owner invariants are:

1. compositor dispatch cannot occur before its global has been explicitly
   initialized;
2. clients admitted through the existing insertion seam always install
   NestedClientDataOwner.

Invariant assertions are not fake handler state: no static state, leaked
reference, fabricated client data, or mismatched display is used.

delegate_compositor!(LinuxXdgShellStateSkeleton) only proves server-side
dispatch wiring. It does not prove a client request or callback was dispatched.

## 5. Capability Truth

After successful explicit initialization:

    global_owner_available = true
    compositor_state_new_invoked = true
    wl_compositor_global_initialized = true
    wl_compositor_state_owned = true
    compositor_handler_available = true
    delegate_compositor_wired = true
    per_client_compositor_state_available = true
    wl_surface_owner_boundary_available = true

These facts remain false:

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

Therefore:

    wl_compositor global initialized
    != client bound wl_compositor

    CompositorHandler available
    != protocol dispatch started

    WlSurface handler boundary
    != surface observed
    != AdapterSurfaceId allocated
    != core SurfaceId admitted

    CompositorState owned
    != real compositor runtime available

## 6. Feature Isolation

The module, Smithay types, methods, and re-exports are visible only under:

    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]

No Smithay/Wayland type enters src/core. Default and smithay-probe use
source-level boundary tests only and do not expose the Linux owner API.

## 7. TDD and Verification

The TDD RED step added default-visible source boundary tests first. Both tests
failed because the Linux module/owner seam did not exist. The minimal owner,
data, handler, delegation, and report changes then made those tests pass.

Linux-only behavior tests cover:

- explicit initialization;
- structured duplicate rejection;
- retained state ownership;
- per-client compositor state ownership;
- handler/global/surface dispatch trait wiring;
- conservative runtime capability fields.

The final local matrix and Linux CI result are recorded in the phase handoff
report and codebase-memory-mcp ADR after execution.

## 8. Next Boundary

The next phase may add a controlled client bind proof only after explicitly
choosing the client connection/event queue/registry driver. It must remain
separate from xdg lifecycle, surface identity admission, ledger/core mutation,
render, and input.
