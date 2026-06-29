# Phase 52W: Pending Admission Consumer Owner

## Goal

Move the Phase 52V pending `xdg_toplevel` admission intent from a callback-side
data bridge into an explicit owner consumer that can invoke the existing
`SurfaceXdgAdmissionLedger` admission seam.

## Boundary

- The callback handler still only records pending admission intent.
- The consumer owns removal from `ToplevelAdmissionBridgeQueue`.
- The consumer calls `ledger.admit_surface(...)` and
  `ledger.admit_toplevel(...)` before removing a pending intent.
- Core `WindowId` allocation is still produced by the existing ledger/core
  admission path, not by direct consumer allocation.
- The phase remains Linux-only behind
  `#[cfg(all(feature = "smithay-linux", target_os = "linux"))]`.

## Implementation Shape

1. Add `src/smithay_backend/linux_toplevel_admission_consumer.rs`.
2. Export `PendingToplevelAdmissionConsumerInput`,
   `PendingToplevelAdmissionConsumerReport`,
   `PendingToplevelAdmissionConsumerBlocker`,
   `PendingToplevelAdmissionConsumerOperation`, and
   `consume_pending_toplevel_admission`.
3. Add a narrow `ToplevelAdmissionBridgeQueue::pop_front` API so only the owner
   consumer removes pending intents.
4. Keep the source-contract tests proving the consumer:
   - reads the pending queue,
   - calls the ledger surface and toplevel admission seams,
   - removes the pending intent only after successful ledger admission,
   - does not touch handler state, runtime, render, input, workspace, stack, or
     low-level device code.

## Capability Report

- `ledger_consume_attempted`: true on the success path.
- `ledger_admit_surface_invoked`: true on the success path.
- `ledger_admit_invoked`: true on the success path.
- `core_register_invoked`: true on the success path through the ledger seam.
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
- No real render path.
- No real input path.
- No direct workspace, stack, or window registry mutation.
- No direct Smithay handler mutation of core `State`.

## Verification

- `cargo test pending_admission_consumer_owner -- --nocapture`.
- `cargo fmt --check`.
- `cargo check`.
- `cargo test`.
- `cargo check --features smithay-probe`.
- `cargo test --features smithay-probe`.
- `cargo check --target x86_64-unknown-linux-gnu --features smithay-linux`.
- Optional Linux `cargo test --target x86_64-unknown-linux-gnu --features
  smithay-linux --no-run` is expected to be environment-dependent on macOS
  because it requires a Linux linker/runtime target setup.

## Next Phase

After this consumer owner seam is merged, the next phase can connect the owner
consumer to a controlled Linux runtime tick or pump while preserving the
callback/ledger ownership split.
