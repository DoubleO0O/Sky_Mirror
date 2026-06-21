# Phase 52F XDG Toplevel Identity Mapping Plan

## 1. Baseline

- Base: `main@56fbd7a`, which contains accepted Phase 52E at merge commit
  `650481c`.
- Phase 52E provides a Linux-only xdg-shell global/request-handler compile
  seam and a future `ToplevelSurface` hook location.
- The accepted admission ledger already maps `AdapterToplevelId` to core
  `WindowId`, but no Smithay protocol identity owns that adapter ID yet.

## 2. Selected Route

Phase 52F selects **Route B: pure adapter-owned identity mapping**.

Wayland server `ObjectId` is cloneable, hashable, and distinguishes protocol
objects even when clients later reuse the same numerical protocol ID. It is
therefore a stable Linux-only registry key. The registry never stores
`ToplevelSurface` itself.

## 3. Ownership Boundary

```text
Smithay ToplevelSurface reference
-> xdg_toplevel Resource::id()
-> LinuxXdgToplevelIdentityKey(ObjectId)
-> LinuxXdgToplevelIdentityRegistry
-> AdapterToplevelId + AdapterSurfaceId mapping
```

`LinuxXdgShellStateSkeleton` owns the registry beside `XdgShellState`, making
the future callback owner explicit. Phase 52F does not call the registry from
`new_toplevel` or `toplevel_destroyed`; the ownership location is established
without claiming callback observation.

## 4. Transaction Rules

- New identities receive monotonically allocated, nonzero
  `AdapterToplevelId` values.
- Active duplicate identities are rejected without replacing the original
  mapping.
- A duplicate identity naming a different `AdapterSurfaceId` is rejected as a
  structured surface mismatch.
- Unknown lookup is distinct from stale lookup.
- Successful removal retires both the protocol identity key and its
  `AdapterToplevelId`.
- Tombstoned identities and retired adapter IDs cannot be reused.
- All rejection checks run before mapping/tombstone mutation.

## 5. Core and Ledger Boundary

Phase 52F does not import or call `SurfaceXdgAdmissionLedger`, `BackendEvent`,
`CoreCommand`, or core `State`. Mapping/removal in this registry is adapter
identity bookkeeping only. It does not admit or unmap a toplevel in core.

The future eligible lifecycle path remains:

```text
real toplevel lifecycle callback
-> lookup LinuxXdgToplevelIdentityKey
-> AdapterToplevelId
-> explicitly owned SurfaceXdgAdmissionLedger call
-> existing BackendEvent / CoreCommand / State seam
```

## 6. Capability Truth

Established:

- stable `ObjectId` identity source
- adapter-owned mapping registry
- `AdapterToplevelId` allocation and lookup
- duplicate, mismatch, unknown, stale, tombstone, and ID-reuse rejection
- explicit registry ownership in the Linux xdg-shell state skeleton

Still false:

- `ledger_unmap_invoked`
- `callback_observed`
- `real_xdg_shell_runtime_available`
- `protocol_dispatch_started`
- `render_support`
- `input_support`

No xdg-shell global is initialized. Popup remains fail-closed, and no
`SeatHandler`, renderer, DRM, GBM, or libinput support is added.

## 7. Remaining Blockers

- `MissingProductionLifecycleBridge`
- `MissingLedgerCallerOwnership`
- `MissingRealCallbackObservation`
- `MissingXdgShellGlobalInitialization`

## 8. Verification

Default and `smithay-probe` builds exercise the pure registry and source-level
feature/capability gates. Linux GitHub Actions is authoritative for compiling
the real Smithay `ToplevelSurface`, `Resource`, and `ObjectId` API path.

## 9. Recommended Next Phase

The next phase may design one narrowly owned lifecycle bridge that receives a
real callback, looks up the existing mapping, and prepares a ledger operation.
It must separately prove callback observation and ledger ownership before any
core mutation is enabled. Runtime global registration, render, and input remain
out of scope.
