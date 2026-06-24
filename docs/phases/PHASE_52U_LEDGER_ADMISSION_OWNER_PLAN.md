# Phase 52U: Ledger Admission Owner from Adapter Toplevel Identity

## Goal

Prove that `SurfaceXdgAdmissionLedger::admit_toplevel` can be called as the
admission owner for an adapter-owned toplevel identity, producing a core
`WindowId` through the existing `BackendEvent::ToplevelMapped` seam.

## Boundary

- `AdapterToplevelId` admission produces `WindowId`, but the phase does not
  manually allocate or pre-claim `WindowId`.
- `admit_toplevel` internally calls `CoreRuntimeBridge::handle_backend_event`
  with `BackendEvent::ToplevelMapped`, which triggers the existing core
  `State::handle_command` allocation path — so `WindowId` is a byproduct of
  the existing core seam, not a new capability claim.
- The proof uses pure data (no real Wayland connection, no controlled client
  roundtrip) to isolate the admission owner boundary.
- Core `State`, `SurfaceXdgAdmissionLedger`, and `BackendEvent` remain in
  `src/smithay_backend` access; `src/core` is not modified.

## Implementation Shape

1. New file `src/smithay_backend/linux_ledger_admission_owner.rs`.
2. Pure-data function `adapter_ledger_admission_report` that:
   - Creates a fresh `SurfaceXdgAdmissionLedger` and `State`.
   - Calls `ledger.admit_surface(state, ...)`.
   - Calls `ledger.admit_toplevel(state, ...)`.
   - Verifies `SurfaceXdgAdmissionReport::ToplevelAdmitted` contains a valid
     `core_window`.
   - Verifies `ledger.{toplevel,surface}_mapping` consistency.
3. All capability flags except admission-related remain false.

## Non-Goals

- No `WindowKind` that implies real protocol runtime or render capability.
- No modification to `src/core/*`.
- No modification to `Cargo.toml`, `Cargo.lock`, or CI.
- No real Wayland socket or controlled client harness.
- No handler `new_toplevel` mutation (Phase 52T identity registration
  remains the handler pathway; admission ownership is proven independently).

## Verification

- `cargo fmt --check` passes.
- `cargo check`, `cargo test` pass.
- `cargo check --features smithay-probe`, `cargo test --features smithay-probe` pass.
- `cargo check --target x86_64-unknown-linux-gnu --features smithay-linux` passes
  (typecheck only, macOS cannot link/run Linux binaries).
- Full Linux CI run covers `cargo test --features smithay-linux` on Linux.
