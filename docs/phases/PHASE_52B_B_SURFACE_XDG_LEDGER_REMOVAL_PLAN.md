# Phase 52B-B Surface/XDG Ledger Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development and execute each task in RED/GREEN order.

**Goal:** Add a pure-data `SurfaceXdgAdmissionLedger` operation that unmaps an admitted toplevel through the accepted Phase 52C core detach seam, removes only the adapter toplevel mapping, and retains the live surface mapping.

**Architecture:** `XdgToplevelUnmapIntent` identifies the admitted adapter surface/toplevel pair. The ledger validates all adapter/core mappings, invokes `BackendEvent::ToplevelUnmapped` through `CoreRuntimeBridge`, and commits ledger removal only after an exact successful core result with clean validation. A pure-data tombstone distinguishes duplicate unmap from an identity that was never admitted.

**Tech Stack:** Rust, `BTreeMap`/`BTreeSet`, existing `CoreRuntimeBridge`, unit tests.

---

## 1. Baseline and route

- Baseline: `main@d216ed4`.
- Phase 52A admission ledger is present.
- Phase 52C `ToplevelUnmapped -> DetachWindowFromSurface` seam is present.
- The ledger has no removal operation before this phase.

Selected route: **B, pure-data ledger removal contract**.

Rejected alternatives:

- Direct registry mutation would bypass the accepted core seam.
- Reusing terminal `CloseWindow` would kill the surface and violate XDG unmap
  semantics.
- Adding a second detach command would duplicate Phase 52C behavior.

## 2. Public interface

```rust
pub struct XdgToplevelUnmapIntent {
    pub adapter_toplevel: AdapterToplevelId,
    pub adapter_surface: AdapterSurfaceId,
}

pub fn SurfaceXdgAdmissionLedger::unmap_toplevel(
    &mut self,
    state: &mut State,
    intent: XdgToplevelUnmapIntent,
) -> Result<SurfaceXdgRemovalReport, SurfaceXdgRemovalError>
```

`SurfaceXdgRemovalError` distinguishes unknown and duplicate toplevels,
unknown/mismatched adapter surfaces, stale core surface/window mappings,
core-link mismatch, unclean pre-state, rejected core detach, and unexpected
post-detach results.

`SurfaceXdgRemovalReport` preserves the removed mapping and full
`RuntimeEventResult` as evidence. It reports that the toplevel mapping was
removed while the adapter surface mapping and core surface remain live.

## 3. Transaction semantics

1. Read and validate ledger mappings without mutation.
2. Validate core surface/window/link state and require a clean pre-state.
3. Dispatch `BackendEvent::ToplevelUnmapped` through `CoreRuntimeBridge`.
4. Require an exact `CommandResult::ToplevelDetached { result: Ok(_) }`, clean
   post-validation, live/unlinked surface, and dead window.
5. Only then remove `AdapterToplevelId -> WindowId` and the corresponding
   surface-to-toplevel index, and record the toplevel tombstone.

Any rejection before step 5 leaves every ledger map unchanged. The adapter
surface mapping is never removed by toplevel unmap.

## 4. TDD tasks

### Task 1: Successful ledger unmap

**Files:**

- Modify and test: `src/smithay_backend/surface_xdg_admission.rs`

- [x] Add failing tests for bridge invocation, toplevel mapping removal,
      retained surface mapping, live core surface, dead core window, clean
      workspace/focus, and clean validation.
- [x] Run `cargo test ledger_unmap_toplevel` and observe RED because the intent,
      report, error, and method do not exist.
- [x] Add the minimal pure-data types and successful transactional path.
- [x] Run focused tests and observe GREEN.

### Task 2: Structured rejection and tombstones

**Files:**

- Modify and test: `src/smithay_backend/surface_xdg_admission.rs`

- [x] Add failing tests for unknown/duplicate toplevel, mismatched adapter
      surface, stale core surface/window, core-link mismatch, and core detach
      rejection with unchanged ledger mappings.
- [x] Add a private removed-toplevel tombstone set and pre-dispatch validation.
- [x] Run focused rejection tests and observe GREEN.

### Task 3: Capability facade and verification

**Files:**

- Modify: `src/smithay_backend/surface_xdg_admission.rs`
- Modify: `src/smithay_backend/mod.rs`
- Update: `PHASE_52B_B_SURFACE_XDG_LEDGER_REMOVAL_PLAN.md`

- [x] Add a conservative lifecycle readiness report and default facade exports.
- [x] Prove pure-data/default/probe isolation and keep real runtime/render/input
      capabilities false.
- [x] Run the full local verification matrix and inspect the final diff.

## 5. Capability boundary

Implemented by this phase:

- `ledger_toplevel_unmap_contract_available`
- `ledger_toplevel_mapping_removal_available`
- `ledger_surface_mapping_retained_after_unmap`
- `ledger_core_detach_bridge_available`
- `ledger_removal_transaction_available`

Still unavailable:

- `real_xdg_toplevel_unmap_runtime_available`
- `real_wl_surface_destroy_runtime_available`
- `protocol_dispatch_started`
- `render_support`
- `input_support`

This phase does not implement a real protocol callback and does not remove or
destroy an admitted adapter surface.
