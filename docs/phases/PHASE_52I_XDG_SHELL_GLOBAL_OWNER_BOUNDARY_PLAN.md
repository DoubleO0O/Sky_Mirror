# Phase 52I XDG Shell Global Owner Boundary Plan

## 1. Baseline

- Base: `main@048322a`, including accepted Phase 52H-A.
- Phase 52E provides the Linux-only xdg-shell handler and dispatch trait shape.
- Phase 52F provides adapter-owned `ToplevelSurface` identity mapping.
- Phase 52G provides read-only lifecycle observation wiring.
- Phase 52H records that runtime callback proof still lacks four prerequisites.

Phase 52I selects **Route B: Linux-only global initialization owner boundary**
and resolves only the first Phase 52H prerequisite.

## 2. Owner Architecture

`SmithayWaylandDisplayProbe` already owns both halves that must remain paired:

```text
Display<LinuxXdgShellStateSkeleton>
+ LinuxXdgShellStateSkeleton
```

It is therefore the public global owner. Its explicit initialization method
obtains `self.display.handle()` and passes that handle to a crate-private state
method. External callers cannot inject a `DisplayHandle` belonging to a
different display.

The initialization path is:

```text
SmithayWaylandDisplayProbe::initialize_xdg_shell_global
-> self.display.handle()
-> LinuxXdgShellStateSkeleton::initialize_xdg_shell_global
-> XdgShellState::new::<LinuxXdgShellStateSkeleton>
-> owner stores XdgShellState
-> LinuxXdgShellGlobalInitReport
```

`SmithayWaylandDisplayProbe::new` deliberately does not call this path. Existing
probe construction remains global-free until a caller explicitly opts in.

## 3. Initialization Transaction

Smithay 0.7 constructs `XdgShellState` synchronously and does not return a
fallible result. Sky Mirror first checks the owner state, constructs the complete
value, and only then stores it in `Option<XdgShellState>`.

Duplicate initialization returns
`LinuxXdgShellGlobalInitError::AlreadyInitialized` before mutation. The original
state remains owned and queryable; no second global is registered.

Readiness queries are read-only and derive initialization truth from whether the
owner currently holds `XdgShellState`.

## 4. Capability Truth

After successful explicit initialization, these fields are true:

```text
global_owner_available = true
xdg_shell_state_new_invoked = true
xdg_shell_global_initialized = true
xdg_shell_state_owned = true
```

The following remain false:

```text
client_harness_available = false
new_toplevel_registration_owner_available = false
callback_observed = false
ledger_unmap_invoked = false
core_detach_invoked = false
protocol_dispatch_started = false
real_xdg_shell_runtime_available = false
render_support = false
input_support = false
```

Successful `XdgShellState::new` means only that the paired Linux owner created
and retained the global state. It does not mean protocol dispatch started, a
client bound the global, callback observation occurred, the xdg-shell runtime is
usable, or a compositor is available.

## 5. Preserved Boundaries

Phase 52I does not:

- create a controlled client/toplevel harness;
- register identity in `new_toplevel`;
- trigger or simulate `toplevel_destroyed`;
- call `SurfaceXdgAdmissionLedger::admit_toplevel` or `unmap_toplevel`;
- emit `BackendEvent::ToplevelUnmapped`;
- execute `CoreCommand::DetachWindowFromSurface`;
- modify core registries, workspace, render, input, DRM, GBM, or libinput;
- introduce `SeatHandler`.

The Phase 52E `xdg_popup` path remains fail-closed. Global initialization does
not add popup runtime or seat/input dependencies.

## 6. Remaining Blockers

After initialization succeeds, the report retains:

- `MissingControlledClientHarness`;
- `MissingNewToplevelRegistrationOwner`;
- `MissingDispatchDrivenCallbackProof`.

Before explicit initialization it additionally reports
`MissingExplicitInitialization`.

## 7. Verification Contract

Default and `smithay-probe` builds must remain free of Smithay/Wayland/Linux
graphics-stack dependencies. Source contract tests enforce the Linux feature
gate, explicit owner call site, absence of constructor auto-initialization, and
absence of ledger/core/input calls in the new path.

Linux CI additionally compiles and tests the real `DisplayHandle`,
`XdgShellState::new`, successful ownership, conservative report, and duplicate
initialization rejection.

## 8. Recommended Next Phase

The next phase should implement a controlled Linux-only client/toplevel
lifecycle harness. It should prove that a client can bind the initialized global
without yet connecting lifecycle callbacks to ledger or core mutation.

Identity registration ownership in `new_toplevel` and dispatch-driven destroy
proof remain later, separately reviewed promotions.
