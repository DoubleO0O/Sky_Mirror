# Phase 52A Surface/XDG Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use test-driven development and execute each red/green step in order.

**Goal:** Add a pure-data adapter identity ledger that admits surface and xdg-toplevel observations through the existing `BackendEvent -> CoreCommand -> State` seam.

**Architecture:** A default-visible `surface_xdg_admission` module owns only mappings from adapter identities to core `SurfaceId`/`WindowId`. It validates duplicate, orphan, and stale observations before dispatching existing backend events through `CoreRuntimeBridge`; it never stores protocol objects or mutates registries directly.

**Tech Stack:** Rust 2024, existing core runtime bridge, unit tests, existing Cargo feature matrix.

---

## File responsibilities

- Create `src/smithay_backend/surface_xdg_admission.rs`: pure-data identities, intents, mapping ledger, structured reports/errors, conservative readiness report, and unit tests.
- Modify `src/smithay_backend/mod.rs`: compile and re-export the pure-data module in default, `smithay-probe`, and `smithay-linux` builds without adding dependencies.
- Create `PHASE_52A_SURFACE_XDG_ADMISSION_PLAN.md`: record the approved design, invariants, TDD sequence, and verification matrix.

## Task 1: Identity and readiness contract

- [ ] Add failing tests for nonzero identity round-trips, production-source dependency isolation, and conservative readiness fields.
- [ ] Run `cargo test surface_xdg_admission -- --nocapture` and confirm failure because the module/types do not exist.
- [ ] Add `ProtocolObjectId`, `AdapterSurfaceId`, `AdapterToplevelId`, `SurfaceXdgAdmissionBlocker`, `SurfaceXdgAdmissionReadinessReport`, and `surface_xdg_admission_readiness_report` with Chinese public documentation.
- [ ] Keep `real_wl_surface_runtime_available`, `real_xdg_toplevel_runtime_available`, `protocol_dispatch_started`, `render_support`, and `input_support` false.
- [ ] Run the focused tests and confirm they pass.

## Task 2: Surface admission

- [ ] Add failing tests proving `admit_surface` dispatches `BackendEvent::SurfaceCreated`, stores `AdapterSurfaceId -> SurfaceId`, returns a clean `ValidationReport`, and rejects duplicate adapter identities without registering a second core surface.
- [ ] Run the focused tests and confirm the expected missing-interface failures.
- [ ] Add `SurfaceAdmissionIntent`, surface mapping records, `SurfaceXdgAdmissionReport`, `SurfaceXdgAdmissionError`, and `SurfaceXdgAdmissionLedger::admit_surface`.
- [ ] Match `CommandResult::SurfaceRegistered`; insert the mapping only after `registered=true`.
- [ ] Run focused and default tests.

## Task 3: Toplevel admission

- [ ] Add failing tests proving `admit_toplevel` dispatches `BackendEvent::ToplevelMapped`, stores `AdapterToplevelId -> WindowId`, and links the mapped core surface to the returned core window.
- [ ] Add failing tests for duplicate toplevel, orphan adapter surface, and stale/dead core surface rejection; assert rejected operations do not add core windows.
- [ ] Run focused tests and confirm failures are caused by missing toplevel behavior.
- [ ] Add `XdgToplevelAdmissionIntent`, toplevel mapping records, and `admit_toplevel` with pre-dispatch duplicate/orphan/stale checks.
- [ ] Match `CommandResult::WindowRegisteredForSurface` and insert the mapping only after `bound=true`.
- [ ] Run focused and default tests.

## Task 4: Feature isolation and final verification

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo check` and `cargo test`.
- [ ] Run `cargo check --features smithay-probe` and `cargo test --features smithay-probe`.
- [ ] Run `cargo tree --features smithay-probe | rg -i 'smithay|wayland|udev|libinput|drm|gbm|x11|vulkan' || true` and inspect every match rather than treating grep output as a failure by itself.
- [ ] Run `git diff --check`, `git status -sb`, `git diff --stat`, and `git diff`.
- [ ] Confirm only the three approved files changed and no real protocol, dispatch, renderer, input, or Linux runtime code exists.
- [ ] Commit with `feat(smithay): add surface xdg admission contract`.
- [ ] Push the Phase 52A branch and use GitHub Actions as the `smithay-linux` verification source.
