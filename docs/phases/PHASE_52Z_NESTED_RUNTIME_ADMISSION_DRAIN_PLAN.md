# Phase 52Z: Nested Runtime Admission Drain

## Goal

Wire the runtime-owned pending toplevel admission queue owner into
`NestedRuntimeCoordinator` behind an explicit admission drain pump.

## Boundary

- `NestedRuntimeCoordinator` owns a `RuntimeToplevelAdmissionQueueOwner`.
- Existing `pump_once` behavior remains unchanged.
- A new combined pump runs the existing accept/dispatch/disconnect lifecycle
  first, then drains one pending toplevel admission intent.
- Pending admission still enters through an explicit enqueue method; this phase
  does not yet wire live Smithay `new_toplevel` callbacks into the queue.
- Ledger admission and core `WindowId` allocation still happen only through the
  existing `SurfaceXdgAdmissionLedger` and Phase 52W consumer seam.

## Implementation Shape

1. Extend `src/smithay_backend/nested_runtime_coordinator.rs` so
   `NestedRuntimeCoordinator` owns a `RuntimeToplevelAdmissionQueueOwner`.
2. Add `NestedRuntimeAdmissionPumpReport`, which carries the existing
   `NestedRuntimePumpReport` and the runtime admission drain report.
3. Add `enqueue_pending_toplevel_admission`.
4. Add `pump_once_with_toplevel_admission_drain`.
5. Add read-only inspection helpers for pending count, next core surface ID,
   surface mapping, and toplevel mapping.
6. Preserve source-contract tests proving the production coordinator:
   - uses the runtime queue owner,
   - exposes an enqueue seam,
   - runs lifecycle before admission drain,
   - does not use handler state, client harness, workspace/stack mutation, or
     low-level device code for the new admission drain seam.

## Capability Report

- `lifecycle_report`: preserves the existing nested lifecycle pump report.
- `admission_drain_report`: reports one runtime admission queue drain attempt.
- `runtime_queue_owned`: true through the Phase 52Y owner.
- `runtime_ledger_owned`: true through the Phase 52Y owner.
- `pending_admission_consumed`: true when a queued intent is admitted.
- `ledger_admit_surface_invoked`: true on the success path.
- `ledger_admit_invoked`: true on the success path.
- `core_register_invoked`: true through the ledger/core admission seam.
- `window_id_allocated`: true when the ledger returns a core window.
- `handler_state_touched`: false.
- `ledger_bypassed`: false.
- `render_support`: false.
- `input_support`: false.
- `real_compositor_runtime_available`: false.
- `real_xdg_shell_runtime_available`: false.

## Non-Goals

- No `src/core/*` changes.
- No `src/backend/*` changes.
- No `Cargo.toml`, `Cargo.lock`, or CI changes.
- No automatic live callback enqueue from handler state.
- No new real compositor event loop.
- No direct Wayland client/server creation in the admission drain code.
- No direct Smithay handler mutation of core `State`.
- No render path.
- No input path.

## Verification

- `cargo test nested_runtime_coordinator_admission_drain_source_uses_runtime_queue_owner -- --nocapture`.
- `cargo fmt --check`.
- `cargo check`.
- `cargo test`.
- `cargo check --features smithay-probe`.
- `cargo test --features smithay-probe`.
- `cargo check --target x86_64-unknown-linux-gnu --features smithay-linux`.
- Optional Linux `cargo test --target x86_64-unknown-linux-gnu --features
  smithay-linux --no-run` is environment-dependent on macOS because it requires
  a Linux linker/runtime target setup.

## Next Phase

After this coordinator drain seam is merged, the next phase can connect live
`new_toplevel` callback ownership to the coordinator's runtime admission queue.
That should keep handler code on the callback/identity side of the boundary and
continue routing ledger/core admission through the runtime queue owner.
