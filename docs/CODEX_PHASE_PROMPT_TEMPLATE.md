# Codex Phase Prompt Template

## 0. Read Environment Rules

Read:

- docs/CODEX_ENVIRONMENT_RULES.md
- docs/CODEX_PHASE_PROMPT_TEMPLATE.md

Use `/Users/double/Code/Sky_Mirror` as the active main repo unless a future environment rebuild changes the rule.

## 1. Baseline Check

Commands:

- `git switch main`
- `git pull --ff-only origin main`
- `git status -sb`
- `gh run list --branch main --limit 8`

Confirm:

- main is clean.
- HEAD equals origin/main.
- latest main CI is green.
- the required previous Phase is merged.

## 2. MCP Rule

Use native `mcp__codebase_memory` tools first.

Use only the main project from `list_projects`:

- project: `Users-double-Code-Sky_Mirror`
- root_path: `/Users/double/Code/Sky_Mirror`

Never index worktree paths.

Never index `/Users/double/sky_mirror` unless a future rebuild explicitly makes it the active repo again.

Use project name, not path, for `index_status`.

If codebase-memory-mcp is unavailable, report that and continue with git, `rg`, and file reads.

## 3. Goal

Single phase goal.

```text
Phase XX goal:
- ...
```

## 4. Non-goals

Explicitly list what not to do.

```text
Non-goals:
- ...
- ...
```

## 5. Allowed Files

Explicit file list.

```text
Allowed files:
- ...
```

Only modify these files.

## 6. Forbidden Files

Cargo/core/CI/main forbidden unless explicitly authorized.

```text
Forbidden files:
- Cargo.toml
- Cargo.lock
- .github/workflows/*
- src/core/*
- src/backend/*
- src/smithay_backend/*
- src/main.rs
```

Add phase-specific forbidden files as needed.

## 7. Feature Gate Requirements

Linux-only APIs gated under:

```rust
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
```

Default and `smithay-probe` builds must not expose Linux runtime types.

## 8. TDD Plan

RED:

- Add the smallest failing test or source-contract check first.
- Confirm it fails for the intended reason.

GREEN:

- Make the smallest implementation that satisfies the test.
- Avoid unrelated refactors.

VERIFY:

- Run cargo fmt/check/test/default/probe.
- Use GitHub Actions if Linux feature work is involved.

For pure documentation phases, TDD can be marked not applicable, but verification commands still run where the environment supports them.

## 9. Verification Commands

Run locally:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
git diff --check
git status -sb
git diff --stat
```

When Linux-only Smithay code is involved, also rely on GitHub Actions for smithay-linux check/test unless the local Linux target and dependencies are explicitly ready.

## 10. Final Report Format

Use this structure:

```markdown
## 1. MCP / Environment
- project:
- root_path:
- index_status:
- worktree index:
- environment blocker:

## 2. True Capability
- current Phase:
- capability proven:
- capability not proven:

## 3. Files
- modified:
- forbidden untouched:

## 4. API
- added:
- changed:
- removed:

## 5. Tests
- cargo fmt --check:
- cargo check:
- cargo test:
- cargo check --features smithay-probe:
- cargo test --features smithay-probe:
- git diff --check:

## 6. Git
- branch:
- commit:
- pushed:
- status:

## 7. Risks
- ...

## 8. Next Step
- ...
```
