# Phase 52G XDG Lifecycle Callback Identity Lookup Plan

## 1. Baseline

- Base: `main@626d911`, containing accepted Phase 52F.
- Phase 52E provides the Linux-only `XdgShellHandler` compile seam.
- Phase 52F provides stable `ObjectId` keys and the adapter-owned
  `LinuxXdgToplevelIdentityRegistry`.
- The admission ledger and core detach path exist, but Phase 52G does not call
  either one.

## 2. Selected Route

Phase 52G selects **Route B: Linux-only callback observation to identity lookup
boundary**.

Smithay's real `XdgShellHandler::toplevel_destroyed(ToplevelSurface)` method is
the lifecycle handler seam. It can safely pass a borrowed surface to a helper
that extracts its key and performs a read-only registry lookup.

## 3. Observation Flow

```text
XdgShellHandler::toplevel_destroyed(ToplevelSurface)
-> observe_toplevel_lifecycle
-> LinuxXdgToplevelIdentityRegistry::lookup_toplevel
-> LinuxXdgToplevelIdentityKey(ObjectId)
-> AdapterToplevelId + AdapterSurfaceId
-> XdgToplevelLifecycleObservationReport
```

`LinuxXdgShellStateSkeleton` stores the most recent report. It does not store
the `ToplevelSurface` argument. Lookup does not remove or alter the Phase 52F
mapping.

## 4. Callback Truth

The handler method is wired to the observation helper, but Sky Mirror still
does not initialize `XdgShellState`, register an xdg-shell global, or run real
protocol dispatch. Therefore wiring and compile evidence do not prove a real
runtime callback invocation.

Both readiness and per-invocation reports conservatively keep
`callback_observed = false`. A later phase must provide runtime evidence before
that capability can change.

## 5. Structured Results

Successful lookup reports:

- lifecycle signal;
- resolved `AdapterToplevelId`;
- mapped `AdapterSurfaceId`;
- identity lookup attempted and resolved.

Structured failures distinguish:

- unavailable or unstable identity source;
- handler state without registry ownership;
- unknown identity;
- removed/tombstoned identity;
- mismatched expected `AdapterSurfaceId`;
- unexpected registry lookup rejection;
- forbidden ledger or core mutation attempts.

Observation failure does not change active mapping or tombstone counts.

## 6. Mutation Boundary

The Linux observation module does not import or call:

- `SurfaceXdgAdmissionLedger`;
- `BackendEvent`;
- `CoreCommand`;
- core `State`;
- registry `remove`;
- `SeatHandler`.

Consequently, `ledger_unmap_invoked` and `core_detach_invoked` remain false.
Phase 52E popup handling remains fail-closed.

## 7. Capability Truth

Established:

- Linux handler-to-observation wiring;
- `ToplevelSurface` identity extraction and read-only registry lookup;
- `AdapterToplevelId` resolution report;
- unknown, tombstone, source, and surface-mismatch reporting;
- last-observation ownership in `LinuxXdgShellStateSkeleton`.

Still false:

- `callback_observed`;
- `ledger_unmap_invoked`;
- `core_detach_invoked`;
- `real_xdg_shell_runtime_available`;
- `protocol_dispatch_started`;
- `render_support`;
- `input_support`.

## 8. Remaining Blockers

- `MissingRealCallbackObservation`
- `MissingRegisteredIdentityRuntimeProof`
- `MissingLedgerCallerOwnership`
- `MissingXdgShellRuntime`

## 9. Verification

Default and `smithay-probe` tests cover the pure-data report, structured
failures, transaction invariants, exact Linux gates, and source-level mutation
prohibitions. Linux GitHub Actions is authoritative for the real Smithay
handler and `ToplevelSurface` type path.

## 10. Recommended Next Phase

The next phase should first prove a running xdg-shell lifecycle callback and a
registered identity at that callback. Ledger ownership and any unmap call must
remain a separate promotion with explicit transactional tests. Render, input,
DRM, GBM, and libinput remain out of scope.
