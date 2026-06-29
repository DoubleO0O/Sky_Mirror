# Phase 52Y: Runtime Admission Queue Owner

## Goal

Introduce a runtime-owned pending toplevel admission queue owner that holds both
the queue and the admission ledger, then drains one pending intent per tick
through the Phase 52W consumer.

## Boundary

- The owner holds `ToplevelAdmissionBridgeQueue`.
- The owner holds `SurfaceXdgAdmissionLedger`.
- Runtime drain calls `consume_pending_toplevel_admission`.
- Successful drain advances `next_core_surface_id`.
- This phase does not wire the owner into `NestedRuntimeCoordinator`.
- This phase does not create Wayland clients, dispatch protocol requests, or
  touch Smithay handler state.

## Implementation Shape

1. Add `src/smithay_backend/linux_toplevel_admission_runtime_queue.rs`.
2. Export `RuntimeToplevelAdmissionQueueOwner`,
   `RuntimeToplevelAdmissionDrainTick`,
   `RuntimeToplevelAdmissionEnqueueReport`,
   `RuntimeToplevelAdmissionDrainReport`,
   `RuntimeToplevelAdmissionQueueBlocker`, and
   `RuntimeToplevelAdmissionQueueOperation`.
3. Keep the module Linux-only behind
   `#[cfg(all(feature = "smithay-linux", target_os = "linux"))]`.
4. Add source-contract tests proving production code:
   - owns queue and ledger together,
   - drains through the Phase 52W consumer,
   - advances `next_core_surface_id` only after successful consumption,
   - does not use handler state, client harness, workspace/stack mutation, or
     low-level device code.

## Capability Report

- `runtime_queue_owned`: true.
- `runtime_ledger_owned`: true.
- `drain_invoked`: true when a tick drain is attempted.
- `ledger_consume_attempted`: true on a non-empty successful drain.
- `pending_admission_consumed`: true on the success path.
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
- No direct `NestedRuntimeCoordinator` integration yet.
- No new Wayland client/server harness in production code.
- No direct Smithay handler mutation of core `State`.
- No render path.
- No input path.

## Verification

- `cargo test runtime_admission_queue_owner -- --nocapture`.
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

After this owner is merged, the next phase can add a narrow integration seam
from `NestedRuntimeCoordinator` into this runtime-owned admission queue while
preserving the existing accept/dispatch/disconnect lifecycle ordering.
