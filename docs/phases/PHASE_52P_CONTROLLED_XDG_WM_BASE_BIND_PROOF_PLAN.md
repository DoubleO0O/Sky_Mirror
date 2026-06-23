# Phase 52P Controlled xdg_wm_base Bind Proof

## 1. Baseline and Route

- Base: `main@d6c298e`, including accepted Phase 52O-B commit `47f2e08`.
- Local `main` and `origin/main` matched; the five latest main CI runs were green.
- codebase-memory-mcp was restored, checked with project name
  `Users-double-sky_mirror`, and reindexed non-persistently at this baseline.
- This phase selects **Route B: controlled xdg_wm_base bind proof**.

No Cargo, core, backend, main, CI, xdg surface/toplevel lifecycle, ledger,
renderer, input, DRM, GBM, or libinput change is part of this phase.

## 2. Existing Ownership and Harness Evidence

The accepted chain already provides:

    Phase 52I-B
      SmithayWaylandDisplayProbe explicitly initializes and owns XdgShellState

    Phase 52M-B
      the same display/state pair explicitly owns CompositorState

    Phase 52N-B
      controlled UnixStream endpoint, client insertion, Connection,
      registry discovery, wl_compositor bind, bounded dispatch/flush

    Phase 52O-B
      controlled wl_surface creation and adapter-only surface identity

Phase 52P reuses the controlled endpoint/driver pattern but creates no surface.
Both xdg-shell and wl_compositor owners must be initialized before endpoint
creation; missing owners produce structured blockers without partial mutation.

## 3. Locked API Evidence

The locked client dependency already imports:

    wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase

The locked wayland-client 0.31.14 API provides:

    registry_queue_init<State>(&Connection)
    GlobalList::bind<I, State, U>()
    EventQueue::roundtrip(&mut State)

The locked xdg-shell protocol defines `xdg_wm_base` as a global interface.
Binding it creates only the manager proxy. `get_xdg_surface` is a separate
request and is deliberately absent from Phase 52P.

The protocol also defines ping/pong. Smithay 0.7 sends a ping only when the
compositor explicitly calls `ShellClient::send_ping`; global bind does not make
that call. Phase 52P sends no ping and therefore does not claim a ping/pong
boundary. If a later phase sends ping, it must implement and test pong
separately rather than silently extending this proof.

Sources:

- local locked source: `wayland-client-0.31.14/src`
- local locked protocol: `wayland-protocols-0.32.12/.../xdg-shell.xml`
- local locked source: `smithay-0.7.0/src/wayland/shell/xdg/mod.rs`

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
            ├── bind XdgWmBase
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
    client_bound_xdg_wm_base
    protocol_dispatch_started

These facts remain false:

    wl_surface_created
    adapter_surface_identity_allocated
    xdg_surface_create_attempted
    xdg_surface_created
    xdg_toplevel_create_attempted
    xdg_toplevel_created
    xdg_surface_lifecycle_available
    xdg_toplevel_lifecycle_available
    new_toplevel_callback_observed
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

    xdg_wm_base bound != xdg_surface created
    xdg_wm_base bound != xdg_toplevel lifecycle
    bounded bind dispatch != real xdg-shell runtime
    registry bind != core window registration

## 6. Preserved Boundaries

- No `get_xdg_surface` or `get_toplevel` request.
- No `WlSurface`, `XdgSurface`, or `XdgToplevel` client object in the proof.
- No `new_toplevel` callback observation.
- No admission ledger, BackendEvent, CoreCommand, SurfaceRegistry,
  WindowRegistry, Workspace, or WindowId access.
- No render, input, seat, DRM, GBM, or libinput behavior.
- Public reports/errors contain only pure data.

## 7. Feature Isolation

The module, real Wayland/xdg client types, and re-exports are available only
under:

    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]

Default and `smithay-probe` validate source boundaries but do not compile or
expose the controlled bind API.

## 8. TDD and Verification

The TDD RED step added default-visible source boundary tests first. They failed
because the 52P module/export did not exist. The minimum owner checks,
controlled endpoint, dual global bind, bounded roundtrip, conservative report,
and Linux behavior tests then made the boundary tests pass.

Local verification:

    cargo fmt --check
    cargo check
    cargo test
    cargo check --features smithay-probe
    cargo test --features smithay-probe
    git diff --check

Linux CI must additionally pass:

    cargo check --features smithay-linux
    cargo test --features smithay-linux

macOS local results are not presented as Linux evidence.

## 9. Next Boundary

The next phase may separately prove `wl_surface -> xdg_surface` creation only
after defining its adapter ownership and lifecycle contract. It must still stop
before xdg_toplevel, ledger/core admission, render, or input unless those are
authorized as independent phases.
