# Phase 52O Controlled wl_surface Identity Proof

## 1. Baseline and Route

- Base: `main@e86694c`, including accepted Phase 52N-B commit `0976083`.
- Local `main` and `origin/main` matched; the five latest main CI runs were green.
- This phase selects **Route B: controlled wl_surface creation + adapter-owned identity**.

No Cargo, core, backend, main, CI, xdg lifecycle, ledger, renderer, input, DRM,
GBM, or libinput change is part of this phase.

## 2. Locked API Evidence

The locked wayland-client 0.31.14 example creates a surface with:

    let surface = compositor.create_surface(qh, ());

The locked Smithay 0.7.0 `CompositorHandler` defines:

    fn new_surface(&mut self, surface: &WlSurface)

Smithay documents that a surface observed at this hook has no role or attached
data and cannot yet be rendered. The server-side `Resource::id()` returns an
`ObjectId`; the existing project identity pattern uses that backend identity as
an adapter-internal key rather than relying on a reusable numeric protocol ID.

Sources:

- local locked source: `wayland-client-0.31.14/examples/simple_window.rs`
- local locked protocol: `wayland-client-0.31.14/wayland.xml`
- local locked source: `smithay-0.7.0/src/wayland/compositor/mod.rs`
- local locked source: `wayland-server-0.31.x/src/lib.rs`

The docs.rs pages were attempted during source review but were unavailable from
the current web environment; implementation therefore follows the exact locked
crate sources and examples above.

## 3. Controlled Proof Topology

    UnixStream::pair
    ├── server endpoint
    │   └── NestedClientInsertCompileBoundary
    │       └── NestedClientDataOwner
    └── client endpoint
        └── Connection::from_socket
            └── registry_queue_init
                └── bind WlCompositor
                    └── create_surface
                        └── bounded roundtrip

The owner thread dispatches and flushes the paired server display until the
client roundtrip completes or the five-second proof deadline expires. It never
connects to a system Wayland session socket.

## 4. Adapter Identity Boundary

`LinuxXdgShellStateSkeleton::new_surface` sends the real server `WlSurface` only
to `LinuxWlSurfaceIdentityRegistry`. The registry:

1. uses the resource `ObjectId` only as an internal deduplication key;
2. allocates a monotonic pure-data `ProtocolObjectId`;
3. returns `AdapterSurfaceId` plus `SurfaceIdentityKey`;
4. returns the existing mapping on duplicate observation;
5. never exposes `ObjectId` in the public report.

`AdapterSurfaceId` and `SurfaceIdentityKey` are adapter identities. Neither is a
core `SurfaceId`, neither is written to `SurfaceRegistry`, and no `WindowId` is
allocated.

## 5. Capability Truth

On a successful controlled proof, these facts are true:

    server_wl_compositor_owner_available
    server_wl_compositor_initialized
    controlled_endpoint_created
    server_client_inserted
    client_connection_created
    event_queue_created
    registry_roundtrip_completed
    client_bound_wl_compositor
    wl_surface_create_attempted
    wl_surface_created
    server_surface_observed
    adapter_surface_identity_allocated
    surface_identity_key_available
    protocol_dispatch_started

These facts remain false:

    client_bound_xdg_wm_base
    xdg_surface_created
    xdg_toplevel_created
    xdg_surface_lifecycle_available
    xdg_toplevel_lifecycle_available
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

    wl_surface created != xdg_surface created
    new_surface observed != committed or renderable
    AdapterSurfaceId allocated != core SurfaceId registered
    controlled bounded dispatch != real compositor runtime

## 6. Structured Failure Semantics

- Missing server owner is rejected before endpoint creation with
  `MissingServerWlCompositorOwner`.
- A failed client compositor bind is classified as
  `MissingClientWlCompositorBind`; create_surface is unreachable on that path.
- I/O and client protocol failures identify their operation.
- Missing server observation and adapter identity errors are explicit.
- The bounded driver reports timeout/thread failures instead of hanging.

## 7. Feature Isolation

The module, real Smithay/Wayland types, display accessors, handler storage, and
re-exports are all compiled only under:

    #[cfg(all(feature = "smithay-linux", target_os = "linux"))]

Default and `smithay-probe` validate source boundaries but do not expose or
compile the controlled surface API. No real protocol type enters core.

## 8. TDD and Verification

The TDD RED step added default-visible source tests first. They failed because
the Linux module and export did not exist. Route B then added the minimum
controlled client request, server observation, adapter identity registry, pure
report, and Linux behavior tests.

The verification matrix is:

    cargo fmt --check
    cargo check
    cargo test
    cargo check --features smithay-probe
    cargo test --features smithay-probe
    git diff --check

Linux-only compilation and behavior are verified by GitHub Actions with
`cargo check --features smithay-linux` and
`cargo test --features smithay-linux`; macOS local results are not presented as
Linux evidence.

## 9. Next Boundary

The adapter registry intentionally retains the observed identity after the
controlled client disconnects. Surface destruction/removal and stale-identity
retirement are not proven in Phase 52O and remain a lifecycle risk for a later,
separately scoped boundary.

The next phase may design a controlled xdg-shell bind/lifecycle driver only as
a separate boundary. It must not silently connect this adapter surface identity
to admission ledger/core, and it must remain separate from render/input.
