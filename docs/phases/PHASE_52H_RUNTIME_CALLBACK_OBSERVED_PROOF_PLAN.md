# Phase 52H Runtime Callback Observed Proof Plan

## 1. Baseline

- Base: `main@86b0160`, containing accepted Phase 52G-B.
- Phase 52F-B provides the Linux-only `ToplevelSurface` identity key and
  `LinuxXdgToplevelIdentityRegistry` mapping boundary.
- Phase 52G-B wires `XdgShellHandler::toplevel_destroyed` to a read-only
  identity lookup and produces `XdgToplevelLifecycleObservationReport`.
- Main GitHub Actions was green before this phase started.

Phase 52H selects **Route A: readiness/design only**. It adds no Rust behavior,
does not initialize an xdg-shell runtime, and does not promote callback or
protocol capabilities.

## 2. Current Evidence

The accepted callback-like path is:

```text
XdgShellHandler::toplevel_destroyed(ToplevelSurface)
-> observe_toplevel_lifecycle
-> LinuxXdgToplevelIdentityRegistry::lookup_toplevel
-> AdapterToplevelId + AdapterSurfaceId observation
```

This is handler wiring and compile evidence. It proves that the callback method
has a conservative lookup destination if Smithay invokes it. It does not prove
that a running xdg-shell global dispatched a real client lifecycle request or
that the callback executed with a registered identity.

Manual calls to `observe_toplevel_lifecycle`, or direct calls to the handler
method in a test, are explicitly insufficient runtime evidence.

## 3. Why Route B Is Not Available

The current repository cannot honestly construct the Phase 52H-B proof within
the allowed scope:

1. `LinuxXdgShellStateSkeleton::xdg_shell_state` remains `None`.
2. Production code does not call `XdgShellState::new`; therefore no xdg-shell
   global is initialized for a client to bind.
3. `XdgShellHandler::new_toplevel` remains empty. There is no runtime owner that
   registers the new `ToplevelSurface` in
   `LinuxXdgToplevelIdentityRegistry` before a destroy callback.
4. `toplevel_destroyed` only contains handler wiring. Its presence cannot be
   treated as `runtime_callback_observed` evidence.
5. The repository has no controlled xdg client/global/dispatch harness that can
   create and destroy a real toplevel through Smithay's request path.
6. Supplying those pieces expands the task into xdg-shell runtime
   initialization and may require Cargo, CI, or broader runtime changes. Those
   changes are outside Phase 52H's permitted boundary.

Consequently, Phase 52H must not add a helper that accepts a caller-provided
"proof" flag, manually invoke the handler, or infer callback execution from a
successful identity lookup.

## 4. Capability Truth

Phase 52H-A preserves these values:

```text
callback_observed = false
ledger_unmap_invoked = false
core_detach_invoked = false
mapping_removed = false
real_xdg_shell_runtime_available = false
protocol_dispatch_started = false
render_support = false
input_support = false
```

`handler_wired = true` and a manually invoked helper may report
`identity_lookup_invoked = true`, but neither implies
`runtime_callback_observed = true`.

No conclusion in this document changes the existing Phase 52G observation
report or removes an identity mapping.

## 5. Mutation Boundary

Phase 52H-A does not call or modify:

- `SurfaceXdgAdmissionLedger::admit_toplevel`;
- `SurfaceXdgAdmissionLedger::unmap_toplevel`;
- `BackendEvent::ToplevelUnmapped`;
- `CoreCommand::DetachWindowFromSurface`;
- `SurfaceRegistry`, `WindowRegistry`, `Workspace`, or core `State`;
- renderer, input, `SeatHandler`, DRM, GBM, or libinput paths.

Phase 52F's registry remains adapter-owned and retains only lightweight
`ObjectId` identity. Phase 52G's observation remains read-only and does not
store a real `ToplevelSurface`.

## 6. Phase 52H-B Prerequisites

All four prerequisites must exist before Route B can be reconsidered:

1. **xdg-shell global initialization**: an owned `XdgShellState` created through
   `XdgShellState::new` and attached to the same display/handler state used by
   dispatch.
2. **Controlled client/toplevel lifecycle harness**: a Linux-only client that
   binds the real global and creates then destroys a real xdg toplevel through
   protocol requests.
3. **`new_toplevel` identity registration owner**: a production-shaped owner
   that registers the callback's `ToplevelSurface` before lifecycle teardown,
   with explicit duplicate and rollback semantics.
4. **Dispatch-driven destroy callback**: evidence that Smithay request dispatch,
   rather than a direct helper or trait-method call, invoked
   `toplevel_destroyed` and resolved the registered `AdapterToplevelId`.

Only evidence satisfying all four gates may set a proof-context
`runtime_callback_observed` value to true.

## 7. Blockers

- `MissingXdgShellGlobalInitialization`
- `MissingControlledClientToplevelHarness`
- `MissingNewToplevelIdentityRegistrationOwner`
- `MissingDispatchDrivenDestroyCallback`

These are design blockers, not new Rust enum variants in Phase 52H-A.

## 8. Acceptance Boundary

Phase 52H-A is accepted when:

- the absence of real runtime callback proof is explicit;
- handler wiring, helper invocation, and runtime callback observation remain
  distinct concepts;
- all capability and mutation values in Section 4 remain false;
- the four prerequisites for Phase 52H-B are recorded;
- default and `smithay-probe` verification remain green and unpolluted;
- no Rust, Cargo, CI, core, ledger, render, or input file changes are present.

## 9. Verification

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

Inspect the `smithay-probe` dependency tree for Smithay, Wayland, udev,
libinput, DRM, GBM, X11, and Vulkan dependencies. Linux-only `smithay-linux`
verification remains the responsibility of GitHub Actions; macOS results must
not be presented as Linux proof.

## 10. Recommended Next Step

Do not connect lifecycle observation to ledger/core removal next. First scope a
separate Linux-only xdg-shell runtime initialization phase that establishes the
global and controlled client harness without render or input. Then assign
identity registration ownership in `new_toplevel` and prove dispatch-driven
destroy independently. Ledger unmap and core detach remain later, separately
reviewed promotions.
