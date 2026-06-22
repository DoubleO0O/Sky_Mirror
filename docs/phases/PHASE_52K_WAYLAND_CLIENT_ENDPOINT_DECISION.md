# Phase 52K Wayland Client Endpoint Dependency Decision

## 1. Baseline and Scope

- Base: `main@7276557`, including accepted Phase 52J-A.
- Main CI was green before this decision was written.
- Phase 52I-B provides an explicitly initialized, adapter-owned xdg-shell
  global.
- Phase 52J-A proves that the existing server-side insertion seam is not a
  controlled Wayland client/toplevel harness.
- Phase 52K selects **Route C: dependency proposal with an explicit future
  approval gate**.

This phase changes documentation only. It does not modify Cargo, Rust, CI,
protocol globals, dispatch, admission, core state, render, or input behavior.

## 2. Decision

```text
Current client endpoint available: false
Reusable without new dependency: false
Recommend adding wayland-client: true
Recommend adding now: false
Requires explicit user approval before Cargo change: true
Suggested feature gate: smithay-linux only
Requires wl_compositor owner phase before harness: true
Requires controlled dispatch pump phase: true
```

Sky should permit a future direct `wayland-client` dependency because the
repository has no equivalent API capable of encoding requests, maintaining a
client object map, dispatching events, and performing deterministic
roundtrips. That permission is architectural approval only: the Cargo change
must be made and reviewed in a separate phase.

## 3. Current Dependency Evidence

### 3.1 Cargo and the resolved graph

`Cargo.toml` does not directly depend on `wayland-client`. `Cargo.lock` also
contains no `wayland-client` package, so there is no indirect instance that
could accidentally be mistaken for an importable API.

The current Linux stack resolves to:

```text
smithay 0.7.0
wayland-server 0.31.13
wayland-backend 0.3.15
wayland-scanner 0.31.10
wayland-protocols 0.32.12 (server side through Smithay)
```

Smithay 0.7 re-exports `wayland_server`, `wayland_protocols`,
`wayland_protocols_misc`, and `wayland_protocols_wlr`. It does not re-export
`wayland_client`. A transitive crate is not a stable direct dependency contract
in any case.

### 3.2 Server insertion is not a client endpoint

The existing flow is:

```text
UnixStream::pair
-> server stream
-> DisplayHandle::insert_client
-> server-owned Client and ClientData
```

`DisplayHandle::insert_client` accepts the transport on the server side. The
peer `UnixStream` has no generated protocol proxies, request encoder, registry
state, event queue, object map, roundtrip API, or dispatch implementation.
Consequently it cannot bind `xdg_wm_base`, create `wl_surface`, or prove any
client request reached server dispatch.

Keeping the peer file descriptor alive proves transport continuity only:

```text
server-side client insertion
!= protocol-capable client endpoint
!= global bind attempt
!= global bind proof
!= controlled toplevel lifecycle
```

Hand-writing the wire protocol would duplicate the object-ID, framing,
serialization, event, error, and synchronization responsibilities already
provided by wayland-rs. It is not an acceptable no-dependency substitute.

## 4. Proposed Future Dependencies

The first authorized Cargo-only phase should propose these compatible semver
families, then let Cargo resolve and lock the exact patch versions:

```toml
[features]
smithay-linux = [
    "smithay-probe",
    "dep:smithay",
    "dep:wayland-client",
    "dep:wayland-protocols",
]

[target.'cfg(target_os = "linux")'.dependencies]
wayland-client = { version = "0.31", optional = true }
wayland-protocols = { version = "0.32", optional = true, features = ["client"] }
```

This is a proposal, not a Phase 52K Cargo patch. As of this audit,
`wayland-client 0.31.14` uses the same `wayland-backend 0.3.15` and
`wayland-scanner 0.31.10` families already present in the resolved Smithay
stack. A `0.31` requirement follows the one-version rule while avoiding an
unnecessary exact patch pin. The future Cargo phase must inspect the resulting
lockfile before acceptance.

`wayland-client` supplies the controlled `Connection`, event queue, dispatch,
and core protocol proxies, including `wl_registry`, `wl_compositor`, and
`wl_surface`. A direct `wayland-protocols` dependency with the `client` feature
is also required for generated xdg-shell client proxies such as
`xdg_wm_base`, `xdg_surface`, and `xdg_toplevel`.

Both dependencies must remain optional, Linux-targeted, and reachable only
through `smithay-linux`. They must not enter core or make default and
`smithay-probe` builds resolve Smithay/Wayland runtime dependencies.

Official API references:

- <https://docs.rs/wayland-client/0.31.14/wayland_client/struct.Connection.html>
- <https://docs.rs/wayland-protocols/0.32.12/wayland_protocols/xdg/shell/client/>

## 5. Minimum Future Harness Boundary

The safe future data flow is:

```text
UnixStream::pair
-> server half: DisplayHandle::insert_client
-> client half: wayland_client::Connection::from_socket
-> client registry roundtrip
-> bind real wl_compositor and xdg_wm_base globals
-> create wl_surface
-> create xdg_surface
-> create xdg_toplevel
-> bounded client/server dispatch pump
-> observe real new_toplevel callback
-> register adapter identity in a separately owned boundary
```

The harness owner must remain in the Linux-only adapter/test boundary. It may
own client-side proxy and event-queue state, but no `wayland-client`, Smithay,
or protocol object may cross into core.

The dispatch pump must be bounded and report each observation separately:
endpoint constructed, server insertion completed, registry roundtrip
completed, each global bind attempted/confirmed, each object request sent,
server callback observed, and teardown completed. An earlier observation must
never imply a later one.

## 6. Required Protocol Globals and Phase Ordering

A minimal toplevel lifecycle requires at least:

1. `wl_compositor`, to create the prerequisite `wl_surface`;
2. `xdg_wm_base`, to create `xdg_surface` and `xdg_toplevel`;
3. `wl_display`/`wl_registry`, supplied by the core Wayland connection model
   for discovery and synchronization.

`wl_shm`, buffers, seats, outputs, decoration protocols, and rendering are not
required to prove object creation and callback admission. If protocol rules
require an initial commit for the selected proof, that commit must remain a
surface-lifecycle operation and must not be described as rendering.

The `wl_compositor` global and `wl_surface` owner must be implemented in an
independent phase before the controlled harness. Combining them would make it
impossible to distinguish global/owner failures from client-driver failures
and would enlarge rollback scope. Phase 52K does not implement them because
they require server handler, resource ownership, request dispatch, protocol
lifecycle, and Linux CI work beyond a dependency decision.

Recommended phase order:

```text
52K complete: dependency decision only
-> Cargo-only Linux client compile/import seam
-> Linux wl_compositor global and wl_surface owner boundary
-> controlled client connection/registry/bind proof
-> bounded wl_surface -> xdg_surface -> xdg_toplevel lifecycle proof
-> new_toplevel identity registration owner
-> only then consider ledger/core promotion
```

## 7. Existing Architecture Boundaries

The accepted xdg-shell global owner remains
`SmithayWaylandDisplayProbe::initialize_xdg_shell_global`, which obtains the
handle from its own `Display` and stores the resulting `XdgShellState` in its
paired state. This proves global initialization only; it does not prove client
binding or dispatch.

`LinuxXdgShellStateSkeleton::new_toplevel` remains empty. The
`LinuxXdgToplevelIdentityRegistry` can map a real `ToplevelSurface` identity to
adapter-owned IDs, but production code does not yet own registration at the
callback. `toplevel_destroyed` performs read-only lifecycle observation and
does not prove runtime callback execution.

`SurfaceXdgAdmissionLedger::admit_toplevel` and `unmap_toplevel` already use
the existing `BackendEvent -> CoreCommand -> State` seam. They are deliberately
downstream of protocol lifecycle and identity ownership. This phase does not
call them, emit mapped/unmapped events, execute register/detach commands, or
modify `SurfaceRegistry`, `WindowRegistry`, or `Workspace`.

## 8. Capability Truth

Phase 52K changes no runtime capability:

```text
client_harness_available = false
client_bind_attempted = false
client_bound_xdg_shell_global = false
controlled_toplevel_lifecycle_available = false
new_toplevel_identity_registration_owner_available = false
runtime_callback_observed = false
callback_observed = false
ledger_admit_invoked = false
ledger_unmap_invoked = false
core_register_invoked = false
core_detach_invoked = false
protocol_dispatch_started = false
real_xdg_shell_runtime_available = false
render_support = false
input_support = false
```

The decision `Recommend adding wayland-client: true` is not capability
evidence. No bind, protocol request, dispatch, callback, ledger call, or core
mutation occurred in Phase 52K.

## 9. Risks and Acceptance Gates

Future dependency approval carries these risks:

- feature leakage into default or `smithay-probe` builds;
- duplicate or incompatible wayland-rs patch/minor versions;
- accidentally exposing client/protocol types through public adapter exports;
- tests that block because client and server dispatch are not bounded;
- collapsing insertion, bind, request, callback, and identity evidence into a
  single optimistic capability;
- prematurely coupling protocol lifecycle to admission ledger/core mutation.

The Cargo-only phase is accepted only if default/probe dependency trees remain
clean, Linux `smithay-linux` check/tests pass, the lockfile shows one compatible
wayland-rs family, and no harness behavior is added. The following owner and
harness phases require their own explicit approval and rollback boundary.

## 10. Verification

Run locally:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
git diff --check
cargo tree --features smithay-probe
```

Inspect the probe dependency tree for Smithay, Wayland, udev, libinput, DRM,
GBM, X11, and Vulkan. Linux-only `smithay-linux` verification remains GitHub
Actions evidence; macOS local results must not be presented as Linux proof.

## 11. Final Recommendation

Approve the architectural use of direct `wayland-client 0.31` and
`wayland-protocols 0.32` client bindings under the Linux-only `smithay-linux`
feature, but do not modify Cargo in Phase 52K. The next phase should be a
Cargo-only compile/import seam with no client connection or protocol request.
After that, implement `wl_compositor`/`wl_surface` ownership as a separate
server-side phase before attempting any controlled client lifecycle harness.

Ledger/core integration remains prohibited until real lifecycle callback proof
and `new_toplevel` identity registration ownership are independently accepted.
