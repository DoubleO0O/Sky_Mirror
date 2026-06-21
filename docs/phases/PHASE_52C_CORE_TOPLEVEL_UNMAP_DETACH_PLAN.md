# Phase 52C Core Toplevel Unmap/Detach Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development and execute each task in RED/GREEN order.

**Goal:** Add a pure-data core seam that unmaps one `(SurfaceId, WindowId)` pair, removes the window from active core state, clears the link, and keeps the surface alive.

**Architecture:** `BackendEvent::ToplevelUnmapped` translates to `CoreCommand::DetachWindowFromSurface`; `State::detach_window_from_surface` owns validation and coordinated mutation. `SurfaceRegistry::detach_window` only clears an exact link and never changes surface liveness. Existing `CloseWindow` remains the terminal window-plus-surface cascade.

**Tech Stack:** Rust, existing core registries, `CoreRuntimeBridge`, unit tests.

---

## 1. Baseline and selected design

The baseline is `main@4e92eb0`. Phase 52A admission is present; Phase 52B-A is
on its own unmerged branch and is used only as design context.

Selected route: **B, core pure-data detach seam**.

Rejected alternatives:

- Changing `CloseWindow` to preserve surfaces would alter established terminal
  close and client-disconnect behavior.
- Mutating registries from the adapter would bypass
  `BackendEvent -> CoreCommand -> CoreRuntimeBridge -> State`.

The term "removed WindowId" means that its registry tombstone is no longer
alive and no workspace/focus path references it. The historical record remains
for diagnostics, matching current registry conventions.

## 2. Public interface

The implementation adds:

```rust
SurfaceRegistry::detach_window(surface: SurfaceId, window: WindowId) -> bool

State::detach_window_from_surface(
    surface: SurfaceId,
    window: WindowId,
) -> Result<DetachWindowFromSurfaceResult, DetachWindowFromSurfaceError>

CoreCommand::DetachWindowFromSurface { surface: SurfaceId, window: WindowId }
BackendEvent::ToplevelUnmapped { surface: SurfaceId, window: WindowId }
```

`DetachWindowFromSurfaceError` distinguishes unknown/dead surface,
unknown/dead window, and a mismatched surface-window link. Rejection happens
before mutation.

## 3. TDD tasks

### Task 1: Exact registry unlink

**Files:**

- Modify and test: `src/core/surface.rs`

- [x] Add tests proving an exact link is cleared while the surface stays alive.
- [x] Add a mismatch test proving the original link is unchanged.
- [x] Run `cargo test surface_registry_detach_window` and observe RED because
      `detach_window` does not exist.
- [x] Implement the minimal exact-match `detach_window` method.
- [x] Run the focused tests and observe GREEN.

### Task 2: Coordinated State detach

**Files:**

- Modify and test: `src/core/state.rs`

- [x] Add tests for successful detach, workspace cleanup, focus refresh,
      surface liveness, unknown IDs, mismatch, duplicate detach, and preserved
      terminal `close_window` behavior.
- [x] Run `cargo test detach_toplevel` and observe RED because the State seam
      and result types do not exist.
- [x] Add `DetachWindowFromSurfaceResult`,
      `DetachWindowFromSurfaceError`, and `State::detach_window_from_surface`.
- [x] Preflight all IDs/link state before mutation; then clear the link, remove
      workspace references, and mark only the window dead.
- [x] Run focused State tests and observe GREEN.

### Task 3: Event/command/runtime bridge

**Files:**

- Modify and test: `src/core/backend_event.rs`
- Modify and test: `src/core/command.rs`
- Modify and test: `src/core/runtime_bridge.rs`

- [x] Add translation and public-seam tests for `ToplevelUnmapped`.
- [x] Run focused tests and observe RED because the variants do not exist.
- [x] Add the event, command, structured command result, and handler arm.
- [x] Run focused tests and observe GREEN with a clean `ValidationReport`.

### Task 4: Verification and capability record

**Files:**

- Update: `PHASE_52C_CORE_TOPLEVEL_UNMAP_DETACH_PLAN.md`

- [x] Record that the core detach seam, link detach, workspace cleanup, and
      focus refresh are available.
- [x] Keep real protocol/runtime/render/input capabilities false.
- [x] Run the full default and `smithay-probe` verification matrix.
- [x] Review `git diff` and confirm only Phase 52C files changed.

Git handoff commits and pushes this scoped change, then verifies GitHub Actions
without merging main.

## 4. Capability boundary

On successful implementation these become true:

- `core_toplevel_detach_seam_available`
- `window_detach_keeps_surface_alive`
- `surface_window_link_detach_available`
- `workspace_window_cleanup_available`
- `focus_refresh_after_detach_available`

These remain false:

- `real_xdg_toplevel_unmap_runtime_available`
- `real_wl_surface_destroy_runtime_available`
- `protocol_dispatch_started`
- `render_support`
- `input_support`

This phase does not modify the Phase 52A adapter ledger and does not implement
Phase 52B-B mapping removal.

## 5. Implemented safety semantics

- Unknown/dead surface and unknown/dead window requests return structured
  errors before mutation.
- A mismatched surface-window pair returns its current binding without
  changing either registry.
- Duplicate detach returns `WindowNotAlive` because the first successful
  detach leaves a diagnostic window tombstone.
- A window still referenced by another live surface is rejected before
  mutation, preventing a clean state from gaining an alive-surface to
  dead-window reference.
- Successful detach clears only the exact link, removes all workspace
  references, refreshes focus through `CompositorState::remove_window`, and
  marks only the window dead.
