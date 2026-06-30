# Phase 53M - Orchestrator Live Unmap Report

## Goal

Phase 53M lifts the loop-level live toplevel unmap summary into the nested
runtime orchestrator lifecycle report.

This keeps the mutation ownership from Phase 53K and the loop drain behavior
from Phase 53L unchanged, while making orchestration callers able to inspect
live unmap progress without reaching through the raw loop report.

## Boundary Rules

- The orchestrator still does not read handler state.
- The orchestrator still does not mutate the admission ledger or core state.
- Live unmap mutation remains owned by the runtime admission queue owner.
- The bounded loop remains the only runtime layer that calls the combined
  live admission and live unmap pump.
- The lifecycle report mirrors the loop summary; it does not invent a separate
  unmap source of truth.

## Capability Truth

Proven by this phase:

- `NestedRuntimeLifecycleReport` exposes live unmap drain facts directly.
- The direct orchestrator live unmap summary is exactly the loop summary.
- A Linux proof covers an admitted live toplevel being unmapped and reported at
  orchestrator level.

Not proven by this phase:

- Renderable windows.
- Frame callbacks or commit-driven rendering.
- Input handling.
- Popup handling.
- A full long-running compositor runtime.
