# Phase 51N Main Baseline Audit

> Audit date: 2026-06-20 (Asia/Shanghai)
>
> Scope: main baseline audit and handoff update only. This document does not
> introduce Phase 52 work or claim a complete compositor runtime.

## 1. Baseline

- Local `main`: `f3406371aaa688dd4b94dd4f52b3fc6970f046b3`.
- `origin/main`: `f3406371aaa688dd4b94dd4f52b3fc6970f046b3`.
- Merge commit: `f340637 Merge pull request #1 from DoubleO0O/accepted/phase51n-runtime-baseline`.
- Accepted Phase 51N-C commit: `e39f51d test(smithay): prove runtime start stop orchestration`.
- Merge relationship: `f340637` has recovered main `9f419f1` and accepted
  commit `e39f51d` as its two parents.
- Main CI: GitHub Actions run `27875588941` completed successfully for
  `f340637`. Its Linux job passed formatting, default tests, `smithay-probe`
  check/tests, and `smithay-linux` check/tests.
- Local worktree was clean at audit start.

The older `SKY_PROJECT_AUDIT.md` and
`PHASE_51A_NESTED_CLIENT_CONNECTION_PLAN.md` remain historical snapshots of
their recorded commits. This file is the current baseline handoff for main.

## 2. Accepted phase chain

Main includes the following accepted chain in order:

| Accepted phase | Commit | Evidence carried into main |
| --- | --- | --- |
| 51G | `b3d11fd` | Nested socket probe flow bridge |
| 51J-B | `43cd377` | Client disconnect callback readiness seam |
| 51I-B | `8b13189` | Inserted-client compile proof seam |
| 51I-C-B | `1283373` | Real accept-to-connected bridge proof |
| 51J-A-B | `856151e` | Real disconnect callback bridge seam |
| 51J-C | `0fb1fb0` | Runtime disconnect callback closes the core client |
| 51K-C | `499e892` | Nested runtime coordinator single-pump lifecycle proof |
| 51L-C | `2101f21` | Bounded nested runtime loop proof |
| 51M-C | `b82e6b7` | Wakeup and interruptible-wait proof |
| 51N-C | `e39f51d` | Start/run/stop orchestration and clean-shutdown proof |

This is an accepted proof chain. It does not imply that every project-level
runtime capability has been enabled.

## 3. Current proven capabilities

The following capabilities are supported by code, tests, and the successful
Linux CI run on main:

- The nested client lifecycle seam carries connected and disconnected facts
  through the existing backend-event/core-command path.
- The Linux-only accept/insert proof can accept a test stream, insert a client,
  retain backend-session mapping, bridge a connected event, and register the
  corresponding core client.
- A real peer close can trigger the disconnect callback, bridge
  `BackendEvent::ClientDisconnected` to `CoreCommand::CloseClient`, remove the
  active mapping, and retain a core tombstone.
- `NestedRuntimeCoordinator` provides one ordered pump across accept/insert,
  connected bridge, one `Display` dispatch, and disconnected bridge.
- `NestedRuntimeLoop` repeatedly invokes the coordinator under a hard
  `max_iterations` bound and returns structured exit/error/validation data.
- The loop has a cloneable stop handle backed by calloop wakeup. Linux proof
  covers interruption of an in-progress wait.
- `NestedRuntimeOrchestrator` owns the bounded loop lifecycle and proves
  `Created -> Started -> Running -> Stopping -> Stopped`, including external
  stop+wakeup and a clean final report.
- The accepted lifecycle proofs assert clean post-operation
  `ValidationReport` state. Coordinator, loop, and orchestrator reports expose
  the corresponding `validation_is_clean` result.
- Main's GitHub Actions Linux job is green, including
  `cargo check --features smithay-linux` and
  `cargo test --features smithay-linux`.

The exact long-running capability remains deliberately conservative:

```text
long_running_loop_available = false
accepts_clients = false
runtime_accept_loop_started = false
protocol_dispatch_started = false
surface_support = false
shell_role_support = false
render_support = false
input_support = false
```

The apparent distinction is intentional: isolated Linux lifecycle proofs and
a bounded orchestration interface exist, but there is no project-level daily
runtime that continuously exposes those facilities as a usable compositor.

## 4. Current non-goals / not implemented

Phase 51N-C does not implement or accept the following:

- real `wl_surface` creation, ownership, commit, buffer, damage, or frame
  lifecycle;
- `xdg-shell`, `xdg_wm_base`, real `xdg_surface`, or `xdg_toplevel` handling;
- registered protocol globals and a long-running real protocol dispatch path;
- a real surface render pipeline or Smithay/GPU renderer;
- keyboard, pointer, seat, focus delivery, or other real input plumbing;
- DRM, GBM, or libinput backends;
- a daily usable compositor runtime.

The one-dispatch lifecycle proof used to observe client disconnect must not be
reported as real protocol dispatch readiness. No protocol globals or surface
resources are registered, and `protocol_dispatch_started` remains `false`.

## 5. Architecture invariants still preserved

- Core state remains behind the existing pure-data event/command seam.
  Smithay client, socket, `Display`, and callback types do not enter core
  interfaces.
- The coordinator is an adapter at the runtime seam. It orders existing
  accept/dispatch/bridge operations but does not directly mutate core
  registries or invent core client identities.
- The bounded loop calls the coordinator interface and adds iteration, stop,
  wakeup, and reporting policy without widening the core interface.
- The orchestrator calls the loop interface and owns lifecycle state. It does
  not create new backend events, core commands, or public core APIs.
- `ValidationReport` remains the shared verification surface after lifecycle
  mutations; readiness reports preserve explicit negative capability fields.
- This audit does not modify `Cargo.toml`, public Rust interfaces, or the CI
  workflow.

## 6. Risk assessment

- `long_running_loop_available` can be misread because a type named
  `NestedRuntimeLoop` exists. The implementation is bounded; the field is
  correctly `false` at the accepted baseline.
- Real accept/insert and disconnect tests can be overstated as a continuously
  available compositor. Project-level `accepts_clients` and
  `runtime_accept_loop_started` remain `false`.
- A successful `Display` dispatch in the lifecycle pump can be overstated as
  protocol support. Globals, resources, surface roles, and long-running
  protocol dispatch are absent.
- `SKY_PROJECT_AUDIT.md` and `PHASE_51A_NESTED_CLIENT_CONNECTION_PLAN.md`
  describe earlier baselines. Reading their status sections without their
  commit/date context will understate current lifecycle proof progress.
- The repository's GitHub default branch remains a separate governance concern;
  Phase work should name `main` or an exact accepted commit explicitly until
  that setting is reviewed.
- Linux-only results must continue to come from GitHub Actions or a real Linux
  host. A macOS `smithay-probe` pass is not a substitute for
  `smithay-linux` verification.

## 7. Recommended next phase

Phase 52 should be limited to surface / xdg-shell admission design and
preparation:

- define the minimal protocol-global and resource-admission seam;
- define ownership and identity mapping for real `wl_surface` and xdg-shell
  objects without leaking Smithay types into core;
- specify lifecycle ordering, validation evidence, capability gates, and a
  narrow Linux proof plan before implementation.

Phase 52 should not expand into render, input, taskbar, widgets, DRM, GBM, or
libinput. It should not describe admission design/preparation as implemented
surface or xdg-shell support.

## 8. Verification commands and results

The audit uses the following local verification matrix:

```text
cargo fmt --check                         PASS
cargo check                               PASS
cargo test                                PASS
cargo check --features smithay-probe      PASS
cargo test --features smithay-probe       PASS
git diff --check                          PASS
```

Linux-only verification is not claimed from the local macOS host. GitHub
Actions run `27875588941` is the evidence for main's `smithay-linux` check and
test results; the run and its Linux Rust validation job completed successfully.

The audit also verified:

```text
git rev-parse HEAD        f3406371aaa688dd4b94dd4f52b3fc6970f046b3
git rev-parse origin/main f3406371aaa688dd4b94dd4f52b3fc6970f046b3
```
