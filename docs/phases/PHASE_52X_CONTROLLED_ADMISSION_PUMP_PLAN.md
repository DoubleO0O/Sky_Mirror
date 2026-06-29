# Phase 52X: Controlled Admission Pump

## Goal

Connect the controlled `new_toplevel` adapter identity report to the pending
admission bridge and owner consumer in one explicit pump step.

## Boundary

- The pump accepts an `AdapterToplevelIdentityRegistrationReport` that was
  produced by the existing controlled Linux proof.
- The pump builds a `LiveToplevelAdmissionBridgeInput` from that report.
- The bridge produces a pending admission intent.
- The pump owns a `ToplevelAdmissionBridgeQueue` for the tick and passes that
  queue to `consume_pending_toplevel_admission`.
- Ledger admission and core `WindowId` allocation still happen only through the
  existing `SurfaceXdgAdmissionLedger` seam.
- The production pump does not create Wayland clients, dispatch protocol
  requests, or touch handler state.

## Implementation Shape

1. Add `src/smithay_backend/linux_toplevel_admission_pump.rs`.
2. Export `ControlledToplevelAdmissionPumpInput`,
   `ControlledToplevelAdmissionPumpReport`,
   `ControlledToplevelAdmissionPumpBlocker`,
   `ControlledToplevelAdmissionPumpOperation`, and
   `pump_controlled_toplevel_admission`.
3. Keep the module Linux-only behind
   `#[cfg(all(feature = "smithay-linux", target_os = "linux"))]`.
4. Preserve a source-contract test proving the production pump:
   - reads a controlled registration report,
   - builds a live callback bridge input,
   - creates an owner queue,
   - queues the pending admission intent,
   - invokes the Phase 52W owner consumer,
   - does not use handler state, client harness, workspace/stack mutation, or
     low-level device code.

## Capability Report

- `bridge_input_created`: true on the success path.
- `pending_admission_intent_created`: true on the success path.
- `pending_admission_consumed`: true on the success path.
- `ledger_consume_attempted`: true on the success path.
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
- No new real compositor event loop.
- No direct Wayland client/server creation in production pump code.
- No direct Smithay handler mutation of core `State`.
- No render path.
- No input path.

## Verification

- `cargo test controlled_admission_pump -- --nocapture`.
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

After this pump seam is merged, the next phase can move from a report-driven
controlled pump toward a runtime-owned pending admission queue that is drained
from the nested runtime coordinator tick.
