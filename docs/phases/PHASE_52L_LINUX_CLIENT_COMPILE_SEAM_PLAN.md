# Phase 52L Linux Client Compile Seam Plan

## 1. Baseline and Route

- Base: `main@7d1dd2c`, including accepted Phase 52K-C.
- Phase 52K approved a future Linux-only `wayland-client` dependency but did
  not change Cargo or create a client endpoint.
- Phase 52L selects **Route B: Cargo-only Linux client compile/import seam**.

This phase proves dependency resolution and client-side type imports only. It
does not create a Wayland connection, event queue, registry bind, controlled
harness, compositor global, surface owner, or lifecycle bridge.

## 2. Cargo and Feature Boundary

The two optional dependencies remain under the existing Linux target section:

```toml
wayland-client = { version = "0.31", optional = true }
wayland-protocols = { version = "0.32", features = ["client"], optional = true }
```

Only `smithay-linux` enables them:

```text
smithay-linux
-> smithay-probe
-> smithay
-> wayland-client
-> wayland-protocols/client
```

`default` and `smithay-probe` do not include either `dep:` feature. The Rust
module and its re-export are additionally guarded by:

```rust
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
```

This keeps client/protocol types out of core and out of non-Linux public API
surfaces. Dependency-tree checks must confirm the declarative boundary rather
than inferring it from source gates alone.

## 3. Compile/Import Seam

`linux_wayland_client_endpoint.rs` privately imports:

- `wayland_client::Connection`;
- core client proxies for `wl_registry`, `wl_compositor`, and `wl_surface`;
- xdg-shell client proxies for `xdg_wm_base`, `xdg_surface`, and
  `xdg_toplevel`.

The report function references those types through `std::any::type_name` so
the compiler must resolve them without constructing any protocol object. No
third-party client type is stored in core or exposed as a report field.

```text
dependency available
+ type import compiles
!= Connection created
!= event queue created
!= registry/global bind attempted
!= client harness available
!= protocol dispatch started
```

## 4. Public API

The Linux-only adapter surface adds:

- `WaylandClientEndpointCompileBlocker`;
- `LinuxWaylandClientEndpointCompileReport`;
- `linux_wayland_client_endpoint_compile_report`.

All fields are pure data. The API is additive and gated; no existing public API
is changed or removed.

## 5. Capability Truth

The following compile facts become true under Linux `smithay-linux`:

```text
wayland_client_dependency_available = true
wayland_protocols_client_feature_available = true
linux_client_imports_compile = true
xdg_wm_base_client_type_available = true
xdg_surface_client_type_available = true
xdg_toplevel_client_type_available = true
```

Runtime and mutation facts remain false:

```text
runtime_connection_created = false
event_queue_created = false
registry_bind_attempted = false
client_harness_available = false
client_bind_attempted = false
client_bound_xdg_shell_global = false
controlled_toplevel_lifecycle_available = false
new_toplevel_identity_registration_owner_available = false
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

## 6. Preserved Boundaries

Phase 52L does not:

- call `Connection::connect_to_env`, `connect_to_name`, or `from_socket`;
- construct `Connection`, `EventQueue`, or client proxy instances;
- discover or bind `wl_registry`, `wl_compositor`, or `xdg_wm_base`;
- register `wl_compositor` or own a real `wl_surface`;
- implement `wl_surface -> xdg_surface -> xdg_toplevel` lifecycle;
- call `SurfaceXdgAdmissionLedger::admit_toplevel` or `unmap_toplevel`;
- emit mapped/unmapped backend events or execute register/detach core commands;
- change core, backend, main, CI, render, input, DRM, GBM, or libinput.

## 7. TDD and Verification

The RED step adds default-visible characterization tests that require:

- the module and re-export to have the exact Linux-only gate;
- Cargo to expose both dependencies only through the approved feature;
- the production module to contain the expected imports and conservative
  report fields;
- the production module to contain none of the forbidden runtime or
  ledger/core calls.

Those tests initially fail because the module and source file do not exist.
The GREEN implementation adds only the dependency/import seam needed to satisfy
them. Linux-only module tests then verify imported xdg types and all false
runtime fields under GitHub Actions.

Required local matrix:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
git diff --check
```

Dependency trees must show no Wayland/Smithay runtime dependencies under
`smithay-probe`, and one compatible wayland-rs family under Linux
`smithay-linux`. Linux CI owns `smithay-linux` check/test evidence; macOS local
results are not Linux proof.

## 8. Next Phase

The next safe phase is the Linux `wl_compositor` global and `wl_surface` owner
boundary. It should remain separate from the controlled client harness so
server-side ownership and request handling can be proved independently.

Only after that owner boundary is accepted should a later phase construct a
controlled `Connection`, event queue, registry roundtrip, and bind proof.
Ledger/core promotion remains later still, after real lifecycle callback and
identity registration ownership are independently proven.
