# Phase 52J Controlled Client Toplevel Harness Plan

## 1. Baseline

- Base: `main@dbe45c2`, including accepted Phase 52I-B.
- Phase 52I provides an explicitly initialized xdg-shell global and a paired
  `SmithayWaylandDisplayProbe` owner.
- The existing nested socket path can insert the server half of a Unix stream
  into a Wayland `Display` and observe connection/disconnection lifecycle.
- Phase 52J selects **Route A: readiness/design only**.

This phase adds no Rust behavior. It does not create a Wayland protocol client,
bind a global, create a toplevel, dispatch a request, or enter ledger/core
mutation.

## 2. Evidence Levels

The following operations are distinct and must not be collapsed into one
capability claim:

```text
server-side stream insertion
!= protocol-capable client endpoint
!= client bind of xdg_wm_base
!= wl_surface creation
!= xdg_surface / xdg_toplevel lifecycle
!= new_toplevel identity registration
!= callback observed proof
```

`UnixStream::pair` plus `DisplayHandle::insert_client` proves only that the
Wayland server backend accepted a server-side transport and owns `ClientData`.
The peer stream has no protocol encoder, object map, event queue, global
registry handling, or generated xdg-shell client proxies. It cannot honestly be
reported as a controlled client harness.

Directly invoking `XdgShellHandler::new_toplevel`,
`XdgShellHandler::toplevel_destroyed`, or an observation helper would bypass
Wayland request dispatch and is therefore not harness or callback evidence.

## 3. Why Route B Is Not Available

The repository cannot build a real controlled client/toplevel harness inside
the current Phase 52J boundary:

1. `Cargo.toml` has no direct `wayland-client` dependency.
2. Smithay 0.7 does not re-export `wayland-client`; its `wayland_frontend`
   re-exports provide server and protocol definitions only.
3. Existing `UnixStream::pair` plus `DisplayHandle::insert_client` is
   server-side insertion, not an endpoint capable of sending Wayland requests.
4. The repository has no real `wl_compositor` global. Existing compositor
   global descriptions are planned-only and cannot create `wl_surface`.
5. An initialized xdg-shell global alone cannot drive the required
   `wl_surface -> xdg_surface -> xdg_toplevel` lifecycle.
6. `XdgShellHandler::new_toplevel` remains empty and has no adapter identity
   registration owner.
7. Hand-writing a Wayland wire encoder or temporarily implementing
   `wl_compositor` would expand this phase into protocol runtime and surface
   admission work. It would also create a fragile replacement for generated
   client bindings. Both paths exceed the approved scope.

Adding `wayland-client`, changing CI, or introducing a compositor global
requires explicit authorization in a later phase. Phase 52J must not smuggle
those changes in through a test-only helper.

## 4. Capability Truth

Phase 52J-A keeps every harness/runtime/mutation field conservative:

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

The accepted Phase 52I facts remain narrower:

```text
xdg_shell_global_initialized = true after explicit owner initialization
xdg_shell_state_owned = true after explicit owner initialization
```

Those facts do not imply any Phase 52J capability is true.

## 5. Preserved Boundaries

Phase 52J-A does not:

- modify Cargo, Cargo.lock, CI, core, backend, or `main.rs`;
- add a client crate or raw protocol encoder;
- register `wl_compositor` or create a real `wl_surface`;
- call `SurfaceXdgAdmissionLedger::admit_toplevel` or `unmap_toplevel`;
- emit `BackendEvent::ToplevelMapped` or `ToplevelUnmapped`;
- execute `CoreCommand::RegisterWindowForSurface` or
  `DetachWindowFromSurface`;
- store a real `ToplevelSurface` in an adapter registry;
- add popup runtime, `SeatHandler`, input, render, DRM, GBM, or libinput.

The Phase 52E popup path remains fail-closed. Phase 52F identity storage and
Phase 52G read-only observation semantics are unchanged.

## 6. Prerequisites for Phase 52J-B

Route B can be reconsidered only after all of the following decisions and
boundaries are explicit:

1. Decide whether adding a direct `wayland-client` dependency is allowed, or
   identify an existing dependency that exposes a real reusable client
   endpoint. A server-side `Client` is not a substitute.
2. Define the Linux-only `wl_compositor` global and `wl_surface` ownership
   boundary without entering core admission.
3. Define a controlled client request driver that performs registry discovery,
   global binding, request encoding, event handling, synchronization, and
   deterministic teardown.
4. Define the `xdg_surface` and `xdg_toplevel` lifecycle driver, including the
   required `wl_surface` prerequisite and protocol ordering.
5. Define the `new_toplevel` identity registration owner, duplicate policy, and
   rollback semantics independently from ledger/core mutation.
6. Define the minimum dispatch loop/event-pump boundary needed to move requests
   between the controlled client and server owner, with bounded completion and
   structured failure reporting.
7. Preserve a hard prohibition on ledger/core calls until lifecycle proof and
   identity registration ownership are completed and reviewed as separate
   phases.

## 7. Required Proof for a Future Harness

A future B implementation must distinguish at least these observations:

- client endpoint created;
- server endpoint inserted;
- registry roundtrip completed;
- xdg-shell bind attempted;
- xdg-shell bind confirmed by dispatch evidence;
- `wl_surface` created through a real compositor global;
- `xdg_surface` and `xdg_toplevel` requests dispatched;
- `new_toplevel` callback reached;
- identity registration succeeded;
- teardown request dispatched;
- callback observation occurred.

No later observation may be inferred solely from an earlier one. In particular,
a connected transport or initialized global cannot be used as proof of bind,
toplevel lifecycle, identity registration, or callback execution.

## 8. Verification

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
GBM, X11, and Vulkan. Linux-only `smithay-linux` verification remains the
responsibility of GitHub Actions; macOS results must not be presented as Linux
proof.

## 9. Recommended Next Step

Before implementing a harness, request a focused dependency/surface-runtime
decision covering prerequisites 1 and 2. If a real client dependency or
`wl_compositor` global remains out of scope, keep Phase 52J at Route A rather
than adding a synthetic report that resembles runtime evidence.

Once authorized, implement the controlled client endpoint and compositor
surface prerequisite before touching `new_toplevel`. Ledger admission and core
mutation remain later, independently reviewed promotions.
