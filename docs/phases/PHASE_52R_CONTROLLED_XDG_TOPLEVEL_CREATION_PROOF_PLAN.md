# Phase 52R Controlled xdg_toplevel Creation Proof

## 1. Baseline and Route

- Base: `main@0836c19`, merge of Phase 52Q-B PR #20.
- Phase 52Q-B commit `a1601eb` is now contained in `main`.
- Local `main` and `origin/main` matched before the 52R worktree was created.
- Main CI was green before implementation.
- codebase-memory-mcp was checked with project name `Users-double-sky_mirror`;
  only the main project index was used.
- This phase selects **Route B: controlled xdg_toplevel creation proof**.

No Cargo, core, backend, main, CI, adapter toplevel identity registration,
ledger, renderer, input, DRM, GBM, or libinput change is part of this phase.

## 2. Existing Ownership and Harness Evidence

The accepted chain already provides:

    Phase 52N-B
      controlled endpoint, client insertion, Connection, registry discovery,
      wl_compositor bind, bounded dispatch/flush

    Phase 52O-B
      controlled wl_surface creation and adapter-only surface identity

    Phase 52P-B
      controlled xdg_wm_base bind

    Phase 52Q-B
      controlled xdg_surface creation without toplevel, ledger, or core

Phase 52R extends the same controlled client proof by calling
`xdg_surface.get_toplevel`. Both xdg-shell and wl_compositor owners must be
initialized before endpoint creation; missing owners produce structured
blockers without partial mutation.

## 3. Locked API Evidence

The locked client dependency and protocol crates provide the pieces used by this
phase:

    wayland_client::Connection::from_socket
    wayland_client::globals::registry_queue_init
    wayland_client::globals::GlobalList::bind
    wayland_client::EventQueue::roundtrip
    wl_compositor::WlCompositor::create_surface
    xdg_wm_base::XdgWmBase::get_xdg_surface
    xdg_surface::XdgSurface::get_toplevel
    xdg_toplevel::XdgToplevel

The local `wayland-client-0.31.14` simple window example demonstrates:

    let xdg_surface = wm_base.get_xdg_surface(base_surface, qh, ());
    let toplevel = xdg_surface.get_toplevel(qh, ());

`get_toplevel` creates a client protocol object. In this phase it is not
interpreted as window admission, adapter toplevel identity registration, or a
renderable runtime window.

Sources:

- local locked source: `wayland-client-0.31.14/src`
- local locked source/protocol: `wayland-protocols-0.32.12`
- local locked source: `smithay-0.7.0/src`

## 4. Controlled Proof Topology

    SmithayWaylandDisplayProbe
    ├── initialized XdgShellState
    ├── initialized CompositorState
    └── controlled UnixStream server endpoint
        └── NestedClientInsertCompileBoundary
            └── NestedClientDataOwner

    controlled UnixStream client endpoint
    └── Connection::from_socket
        └── registry_queue_init
            ├── bind WlCompositor
            ├── create wl_surface
            ├── bind XdgWmBase
            ├── get_xdg_surface(wl_surface)
            ├── get_toplevel()
            └── bounded roundtrip

The owner thread dispatches and flushes the paired display until the client
roundtrip completes or the five-second proof deadline expires. No real system
Wayland session socket is opened.

## 5. Capability Truth

On a successful controlled proof, these facts are true:

    server_xdg_shell_global_owner_available
    server_xdg_shell_global_initialized
    server_wl_compositor_owner_available
    server_wl_compositor_initialized
    controlled_endpoint_created
    server_client_inserted
    client_connection_created
    event_queue_created
    registry_roundtrip_completed
    registry_bind_attempted
    client_bound_wl_compositor
    wl_surface_create_attempted
    wl_surface_created
    server_surface_observed
    adapter_surface_identity_allocated
    client_bound_xdg_wm_base
    xdg_surface_create_attempted
    xdg_surface_created
    xdg_toplevel_create_attempted
    xdg_toplevel_created
    protocol_dispatch_started

These facts remain false:

    new_toplevel_callback_observed
    adapter_toplevel_identity_registered
    ledger_admit_invoked
    ledger_unmap_invoked
    core_register_invoked
    core_detach_invoked
    window_id_allocated
    render_support
    input_support
    real_compositor_runtime_available
    real_xdg_shell_runtime_available

Therefore:

    xdg_toplevel_created != WindowId allocated
    xdg_toplevel_created != adapter toplevel identity registered
    xdg_toplevel_created != ledger/core admission
    bounded dispatch != real compositor runtime
    adapter surface identity != adapter toplevel identity
    protocol object creation != renderable window

## 6. Preserved Boundaries

- No adapter toplevel identity registration.
- No admission ledger admit or unmap.
- No BackendEvent, CoreCommand, SurfaceRegistry, WindowRegistry, Workspace, or
  WindowId access.
- No render, input, seat, DRM, GBM, or libinput behavior.
- Public reports/errors contain only pure data.

## 7. Feature Isolation

The module, real Wayland/xdg client types, and re-exports are available only
under:

    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]

Default and `smithay-probe` validate source boundaries but do not compile or
expose the controlled xdg_toplevel API.

## 8. TDD and Verification

The TDD RED step added default-visible source boundary tests first. They failed
because the 52R module did not exist. The minimum Linux-only module, conservative
public report, controlled endpoint/client flow, xdg_surface creation request,
xdg_toplevel creation request, and boundary tests then made the source tests
pass.

Local verification:

    cargo fmt --check
    cargo check
    cargo test
    cargo check --features smithay-probe
    cargo test --features smithay-probe
    git diff --check
    cargo tree --features smithay-probe | rg -i 'smithay|wayland|udev|libinput|drm|gbm|x11|vulkan' || true
    cargo tree --target x86_64-unknown-linux-gnu --features smithay-linux | rg -i 'smithay|wayland|udev|libinput|drm|gbm|x11|vulkan' || true

Linux CI must additionally pass:

    cargo check --features smithay-linux
    cargo test --features smithay-linux

macOS local results are not presented as Linux evidence.

## 9. Next Boundary

The next phase may separately prove callback observation or adapter toplevel
identity registration. It must still stop before ledger/core admission, render,
input, or real daily-use runtime unless those are authorized as independent
phases.
