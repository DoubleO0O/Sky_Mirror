# Phase 53F - Live Admission Callback Dedupe

## Goal

Phase 53F prevents a single live `new_toplevel` callback observation from being
enqueued more than once across bounded loop iterations.

The loop may observe the same display-owned callback snapshot on more than one
iteration. The runtime coordinator must treat callback sequence identity as
runtime-owned admission progress, so repeated snapshots do not create duplicate
pending admissions.

## Boundary Rules

- The display owner still only exposes the latest pure-data observation.
- The coordinator owns dedupe state because it owns the runtime admission queue.
- The live admission owner may query and mark coordinator-owned callback
  sequence progress, but it does not read or mutate core registries.
- The loop and orchestrator routes remain unchanged.
- This phase does not claim full long-running compositor runtime, render, input,
  or real desktop readiness.

## Capability Truth

Proven by this phase:

- One repeated callback observation across two loop iterations is enqueued once.
- The second observation is handled without duplicate core surface/window
  admission.
- The run summary reports two owner invocations but only one enqueue and one
  consumed admission.

Not proven by this phase:

- Multiple real clients.
- Multiple distinct live toplevel callbacks in one running desktop session.
- Renderable windows.
- Input handling.
