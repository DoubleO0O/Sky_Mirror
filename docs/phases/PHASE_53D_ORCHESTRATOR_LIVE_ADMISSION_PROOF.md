# Phase 53D - Orchestrator Live Admission Proof

## Goal

Phase 53D proves that `NestedRuntimeOrchestrator::run` reaches the Phase 53C
bounded loop live admission pump without changing the production orchestrator
route.

The orchestrator still owns lifecycle state only. It starts the existing
`NestedRuntimeLoop`, calls `run_for_iterations`, and turns the loop report into a
final lifecycle report.

## Route

The proof creates a controlled xdg_toplevel observation on the loop-owned
coordinator display after orchestrator start, then runs the orchestrator for one
bounded iteration.

Expected route:

1. `NestedRuntimeOrchestrator::start` creates the loop.
2. The test creates a controlled live xdg_toplevel observation through test-only
   loop accessors.
3. `NestedRuntimeOrchestrator::run` calls `NestedRuntimeLoop::run_for_iterations`.
4. The loop calls the live admission pump and drains one pending admission.
5. The orchestrator reports clean shutdown without directly reading or mutating
   core registries.

## Boundary Rules

- No production orchestrator admission API is added.
- The new accessors are test-only.
- Orchestrator production code still does not touch handler state, admission
  ledger state, core registries, workspace slots, render, or input.
- The proof does not claim full long-running compositor runtime readiness.

## Capability Truth

Proven by this phase:

- Start/run orchestration can carry a controlled live xdg_toplevel admission
  through the loop path.
- The final lifecycle report remains clean after live admission.

Not proven by this phase:

- Full long-running compositor runtime.
- Renderable windows.
- Input handling.
- Real desktop session behavior.
