# Phase 52D Linux XDG Toplevel Unmap Callback Boundary Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:test-driven-development` before adding any Rust behavior. This
> Phase 52D result is route A, so the accepted change is documentation only.

**Goal:** Record the verified blockers that prevent a truthful Linux-only
`xdg_toplevel` unmap callback bridge from being implemented on
`main@14b70e8`, while preserving the accepted Phase 52A/52C/52B-B seams.

**Architecture:** A future real Linux protocol callback must reduce its
Smithay object identity to `AdapterSurfaceId` and `AdapterToplevelId`, then
delegate the complete transaction to
`SurfaceXdgAdmissionLedger::unmap_toplevel`. The current Linux adapter has no
real xdg-shell global, role handler, `Dispatch` implementation, or production
callback source, so this phase must not add a function that can only be called
by tests and then label it a callback boundary.

**Tech stack:** Rust 2024, Smithay 0.7 behind `smithay-linux`, existing
pure-data admission ledger, `CoreRuntimeBridge`, GitHub Actions Linux matrix.

---

## 1. Baseline and evidence

- Baseline: `main@14b70e8`.
- Phase 52A-B is present through `7d025cf`: adapter surface/toplevel identity
  admission and core ID mappings are pure data.
- Phase 52C-B is present through `7f51e40`:
  `BackendEvent::ToplevelUnmapped` translates to
  `CoreCommand::DetachWindowFromSurface`, then
  `State::detach_window_from_surface` removes the window while preserving the
  surface.
- Phase 52B-B is present through `2261403`:
  `SurfaceXdgAdmissionLedger::unmap_toplevel` validates adapter/core mappings,
  invokes the Phase 52C seam, removes only the toplevel mapping, retains the
  surface mapping, and requires clean validation.
- The refreshed codebase-memory graph contains 2,960 nodes and 12,226 edges.
  Its production call trace finds no caller of
  `SurfaceXdgAdmissionLedger::unmap_toplevel`; only unit tests call it.
- Source search finds no production `impl Dispatch`, `impl GlobalDispatch`,
  `XdgShellState`, `ToplevelSurface`, or xdg-shell delegate macro. Matches for
  these names are negative boundary tests that require them to remain absent.
- `SmithayLinuxGlobalDispatchTraitBoundaryReport` and
  `SmithayLinuxDispatchRequestBoundaryReport` are readiness skeletons. They do
  not implement traits, register globals, process requests, or attach a
  handler to the production adapter.

## 2. Route decision

Selected route: **A — readiness/design only**.

Route B is rejected because there is no real Linux xdg-shell callback source
to invoke the bridge. A public function accepting synthetic identities would
only duplicate the pure-data ledger seam and would not prove callback
observation.

Route C is deferred because the current global-dispatch and request-dispatch
preconditions are still explicitly blocked. Introducing an isolated protocol
type alias or inert function would not establish a useful compile seam and
would increase the public Linux-only interface without a production caller.

## 3. Existing accepted lifecycle chain

```text
XdgToplevelUnmapIntent
-> SurfaceXdgAdmissionLedger::unmap_toplevel
-> BackendEvent::ToplevelUnmapped
-> BackendEventTranslator
-> CoreCommand::DetachWindowFromSurface
-> State::detach_window_from_surface
-> SurfaceRegistry::detach_window
-> CompositorState::remove_window
-> WindowRegistry::mark_dead
-> ValidationReport
```

The accepted postconditions remain:

- `AdapterToplevelId -> WindowId` mapping is removed only after success.
- `AdapterSurfaceId -> SurfaceId` mapping is retained.
- `SurfaceId` remains alive and is no longer linked to the dead `WindowId`.
- workspace and focus references to the window are removed.
- a rejected ledger/core operation does not commit ledger removal.

## 4. Missing callback prerequisites

Before route B can be truthful, all of these must exist in one Linux-only
tracer bullet:

1. A real xdg-shell global owned by the Smithay display state.
2. A real protocol handler that receives an xdg-toplevel lifecycle request or
   a precisely defined compositor-side unmap signal.
3. Adapter-owned stable identity mapping from the real protocol resources to
   `AdapterSurfaceId` and `AdapterToplevelId`.
4. A production owner for `SurfaceXdgAdmissionLedger` and the mutable core
   `State` at the callback-to-runtime seam.
5. Ordering rules for map, unmap, destroy, duplicate notification, client
   disconnect, and surface destruction.
6. A Linux integration proof showing the real handler invokes the ledger once
   and propagates its structured result without direct registry mutation.

The callback must not hold a real Smithay/Wayland object in core or in the
ledger. It must not reinterpret xdg-toplevel unmap as `wl_surface` destroy.

## 5. Required future data flow

```text
Linux-only real protocol handler
-> adapter-owned protocol resource identity lookup
-> AdapterSurfaceId + AdapterToplevelId
-> XdgToplevelUnmapIntent
-> SurfaceXdgAdmissionLedger::unmap_toplevel
-> SurfaceXdgRemovalReport | SurfaceXdgRemovalError
```

The ledger remains the deep module at the transaction seam: the future
adapter must not pre-delete mappings, call `State` directly, or reproduce
stale/mismatch/core-detach validation.

## 6. Error and transaction requirements

A future bridge must preserve the ledger's structured rejection for unknown,
duplicate, mismatched, stale, unclean, and core-detach failure cases. It may
add callback-source errors only for failures that occur before an
`XdgToplevelUnmapIntent` can be formed, such as an unknown protocol resource
or missing adapter identity mapping.

Callback observation alone is not success. Success may be reported only when
the returned `SurfaceXdgRemovalReport` confirms mapping removal, retained and
live surface state, and clean validation.

## 7. Capability truth

Available before and after this documentation phase:

- `ledger_toplevel_unmap_contract_available = true`
- `ledger_toplevel_mapping_removal_available = true`
- `ledger_surface_mapping_retained_after_unmap = true`
- `ledger_core_detach_bridge_available = true`
- `ledger_removal_transaction_available = true`

Unavailable and therefore kept false:

- `linux_xdg_toplevel_unmap_boundary_available = false`
- `linux_xdg_toplevel_unmap_compile_boundary_available = false`
- `linux_xdg_toplevel_unmap_bridge_available = false`
- `xdg_unmap_callback_observed = false`
- `ledger_unmap_invoked_from_linux_boundary = false`
- `core_detach_invoked_from_linux_boundary = false`
- `real_xdg_toplevel_unmap_runtime_available = false`
- `real_wl_surface_destroy_runtime_available = false`
- `protocol_dispatch_started = false`
- `render_support = false`
- `input_support = false`

## 8. Verification plan

The documentation-only branch must pass:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
git diff --check
cargo tree --features smithay-probe \
  | rg -i 'smithay|wayland|udev|libinput|drm|gbm|x11|vulkan' || true
```

On macOS, `smithay-linux` is not claimed locally. After push, GitHub Actions
must provide the Linux evidence for:

```bash
cargo check --features smithay-linux
cargo test --features smithay-linux
```

Local verification on 2026-06-21:

- `cargo fmt --check`: pass.
- `cargo check`: pass with existing dead-code warnings only.
- `cargo test`: pass, 245 tests.
- `cargo check --features smithay-probe`: pass with existing warnings only.
- `cargo test --features smithay-probe`: pass, 527 tests.
- `git diff --check`: pass.
- The `smithay-probe` dependency-tree filter returned no Smithay, Wayland,
  udev, libinput, DRM, GBM, X11, or Vulkan matches.
- Local `smithay-linux` was intentionally not run on macOS; GitHub Actions is
  the required source for that result.

## 9. Recommended next phase

Do not add a standalone synthetic unmap callback wrapper. The next eligible
slice should first establish one minimal, Linux-only xdg-shell global and
request-handler compile seam with all runtime capability fields still false.
Only after that seam has a real production event source should a later phase
bridge one observed lifecycle signal into the existing ledger transaction.

Renderer, input, DRM, GBM, libinput, frame callbacks, and daily-usable
compositor claims remain outside this sequence.
