# Phase 52N Controlled wl_compositor Bind Proof

## 1. Baseline and Route

- Base: main@c13db89, including Phase 52M-B commit 5ec667f.
- Main and origin/main matched and main CI was green before implementation.
- This phase selects Route B: controlled client wl_compositor bind proof.

The proof is Linux-only and test-safe. It never connects to a real system
Wayland session socket.

## 2. API Evidence

The locked wayland-client 0.31.14 source provides:

    Connection::from_socket(UnixStream)
    Connection::new_event_queue<State>()
    registry_queue_init<State>(&Connection)
    GlobalList::bind<I, State, U>()
    EventQueue::roundtrip(&mut State)

registry_queue_init creates an event queue, requests the registry, and performs
an initial synchronized roundtrip. The server must therefore dispatch incoming
requests and flush outgoing global/sync events while the client waits.

References:

- <https://docs.rs/wayland-client/0.31.14/wayland_client/struct.Connection.html>
- <https://docs.rs/wayland-client/0.31.14/wayland_client/globals/fn.registry_queue_init.html>
- <https://docs.rs/wayland-client/0.31.14/wayland_client/globals/struct.GlobalList.html>
- locked local sources under wayland-client-0.31.14/src.

## 3. Controlled Endpoint Topology

    UnixStream::pair
    ├── server endpoint
    │   └── NestedClientInsertCompileBoundary
    │       └── NestedClientDataOwner
    │           └── CompositorClientState
    └── client endpoint
        └── Connection::from_socket
            └── registry_queue_init
                └── GlobalList::bind<WlCompositor>

The server-side wl_compositor global must be explicitly initialized before the
proof starts. An uninitialized owner returns a structured
MissingServerWlCompositorOwner blocker before creating any endpoint.

## 4. Dispatch Driver

The client runs in a bounded helper thread because registry roundtrip blocks
until the server processes the request. The owner thread repeatedly:

1. dispatches pending server client requests;
2. flushes outgoing server events;
3. waits briefly for the client result;
4. stops on success, structured failure, or a five-second deadline.

The client:

1. creates Connection from the controlled stream;
2. creates the registry event queue;
3. completes initial registry discovery;
4. binds only WlCompositor;
5. completes a second roundtrip proving the bind request was processed.

protocol_dispatch_started=true means only that this bounded proof driver ran.
It does not describe a long-running compositor runtime.

## 5. Capability Truth

On successful proof:

    server_wl_compositor_owner_available = true
    server_wl_compositor_initialized = true
    per_client_compositor_state_available = true
    controlled_endpoint_created = true
    server_client_inserted = true
    client_connection_created = true
    event_queue_created = true
    registry_roundtrip_started = true
    registry_roundtrip_completed = true
    registry_bind_attempted = true
    client_bound_wl_compositor = true
    client_harness_available = true
    protocol_dispatch_started = true

The following remain false:

    client_bound_xdg_wm_base = false
    wl_surface_created = false
    xdg_surface_lifecycle_available = false
    xdg_toplevel_lifecycle_available = false
    ledger_admit_invoked = false
    ledger_unmap_invoked = false
    core_register_invoked = false
    core_detach_invoked = false
    real_compositor_runtime_available = false
    real_xdg_shell_runtime_available = false
    render_support = false
    input_support = false

Therefore:

    controlled Connection
    != system Wayland session connection

    client bound wl_compositor
    != wl_surface created
    != xdg_wm_base bound
    != xdg lifecycle

    bounded protocol dispatch proof
    != real compositor runtime

## 6. Preserved Boundaries

- No Cargo, core, backend, main, or CI change.
- No XdgWmBase client type or bind in the proof module.
- No WlSurface type or create_surface request.
- No adapter/core surface identity.
- No ledger, BackendEvent, CoreCommand, workspace, render, input, DRM, GBM, or
  libinput access.
- All real client/server types remain under smithay-linux plus Linux target
  gating.
- The public report and errors contain only pure data.

## 7. TDD and Verification

Default-visible source tests were added first and failed because the controlled
bind module did not exist. The minimum Linux-only endpoint, client, registry,
bind, report, and server flush driver were then implemented.

Linux tests cover:

- structured rejection without initialized server owner;
- controlled endpoint and server insertion;
- client connection and event queue creation;
- registry discovery and completed bind roundtrip;
- wl_compositor-only bind;
- conservative xdg/surface/core/render/input fields.

Final local and Linux CI results are recorded in the handoff report and
codebase-memory-mcp ADR.

## 8. Next Boundary

Any future wl_surface creation proof must be a separate phase. It must define
adapter-owned surface identity before considering core admission and must not
silently expand into xdg-shell, render, or input.
