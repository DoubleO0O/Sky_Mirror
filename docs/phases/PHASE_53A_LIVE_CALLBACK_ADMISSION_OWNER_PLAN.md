# Phase 53A: Live Callback Admission Owner

## Goal

Connect the live `new_toplevel` callback observation path to the
runtime-owned coordinator admission queue.

## Boundary

- `SmithayWaylandDisplayProbe` remains the owner of live callback and adapter
  identity observations.
- The new live admission owner reads the latest display-owned observation.
- The owner converts the observation through the Phase 52V pending admission
  bridge.
- The Phase 52V bridge's `MissingLedgerOwner` and `MissingStateOwner` blockers
  remain valid for the bridge layer, but Phase 53A treats them as the expected
  handoff to the coordinator owner when a pending admission intent exists.
- The owner enqueues the pending admission intent into
  `NestedRuntimeCoordinator`.
- The existing Phase 52Z coordinator drain remains responsible for ledger and
  core admission.
- Handler code remains on the callback and identity side of the boundary.

## Implementation Shape

1. Add the Linux-only `linux_live_toplevel_admission_owner` module.
2. Add `enqueue_live_toplevel_admission_from_display`.
3. Read `last_new_toplevel_callback_observation_sequence` from the display
   owner.
4. Read `last_adapter_toplevel_identity_registration_observation` from the
   display owner.
5. Build `LiveToplevelAdmissionBridgeInput` from the registered adapter
   surface and toplevel identities.
6. Call `live_toplevel_admission_bridge_report`.
7. Enqueue the resulting pending admission intent through
   `NestedRuntimeCoordinator::enqueue_pending_toplevel_admission`.
8. Return a pure data report that records the observation, bridge, enqueue, and
   conservative unsupported runtime capabilities.

## Capability Report

- `new_toplevel_callback_observed`: true when the display owner has recorded a
  live callback observation.
- `adapter_toplevel_identity_registered`: true when the adapter identity
  registration observation is successful.
- `bridge_input_created`: true when the owner can build the Phase 52V input.
- `pending_admission_intent_created`: true when the Phase 52V bridge returns a
  pending admission intent.
- `coordinator_enqueue_invoked`: true only when the owner enqueues into the
  coordinator queue.
- `handler_state_touched`: false.
- `ledger_admit_invoked`: false.
- `core_register_invoked`: false.
- `window_id_allocated`: false.
- `render_support`: false.
- `input_support`: false.
- `real_compositor_runtime_available`: false.
- `real_xdg_shell_runtime_available`: false.

## Non-Goals

- No `src/core/*` changes.
- No `src/backend/*` changes.
- No `Cargo.toml`, `Cargo.lock`, or CI changes.
- No handler-owned ledger admission.
- No direct core state mutation in the live admission owner.
- No direct Wayland client harness creation in the live admission owner.
- No Wayland dispatch or flush in the live admission owner.
- No render path.
- No input path.
- No full nested compositor runtime.

## Verification

- `cargo test live_toplevel_admission_owner -- --nocapture`.
- `cargo fmt --check`.
- `cargo check`.
- `cargo test`.
- `cargo check --features smithay-probe`.
- `cargo test --features smithay-probe`.
- `cargo check --target x86_64-unknown-linux-gnu --features smithay-linux`.
- `cargo check --tests --target x86_64-unknown-linux-gnu --features smithay-linux`.
- Optional Linux `cargo test --target x86_64-unknown-linux-gnu --features
  smithay-linux --no-run` is environment-dependent on macOS because it requires
  a Linux linker/runtime target setup.

## Next Phase

After this owner seam is merged, the next phase can move from an explicit owner
call toward a bounded nested runtime path that invokes the live callback
admission owner at the correct pump point while preserving the same queue,
ledger, and core ownership boundaries.
