# Phase XX Prompt Template

Use this template for future Sky / Sky Mirror Codex phases. Keep the prompt short, explicit, and phase-specific. Do not index worktrees.

## 0. Baseline Check

Run first:

```bash
git switch main
git pull --ff-only origin main
git status -sb
gh run list --branch main --limit 8
```

Confirm:

- `main` is clean.
- `origin/main` is current.
- Latest `main` CI is green.
- Phase preconditions are merged before starting.

## 1. Environment Rules

Follow `docs/CODEX_ENVIRONMENT_RULES.md`.

Required summary:

- MCP uses only the main project for `/Users/double/sky_mirror`.
- Never index `/Users/double/.config/superpowers/worktrees/...`.
- Worktree diffs are checked with git/file reads, not a worktree MCP index.
- Use `/Users/double/.headroom/bin/rtk` when available.
- Report any RTK fallback honestly.
- Read only the minimal applicable skills.

## 2. Goal

State the one goal for this phase:

```text
Phase XX goal:
- ...
```

## 3. Strict Non-goals

List what must not happen:

```text
Non-goals:
- ...
- ...
```

Do not promote controlled proof into broader runtime capability unless the phase explicitly requires and verifies it.

## 4. Allowed Files

```text
Allowed files:
- ...
```

Only modify these files.

## 5. Forbidden Files

```text
Forbidden files:
- Cargo.toml
- Cargo.lock
- .github/workflows/ci.yml
- src/core/**
- ...
```

Add phase-specific forbidden files as needed.

## 6. Feature Gate Requirements

For Linux-only work:

```rust
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
```

Default and `smithay-probe` builds must not expose Linux runtime types. Keep pure-data core boundaries intact.

## 7. TDD Plan

RED:

- Write the smallest failing test or source-contract check first.
- Confirm it fails for the intended reason.

GREEN:

- Make the smallest implementation that satisfies the test.
- Avoid unrelated refactors.

VERIFY:

- Run the full local matrix.
- Use Linux CI for `smithay-linux` runtime/protocol proof when required.

For pure documentation phases, TDD may be documented as not applicable, but verification commands still run.

## 8. Verification Commands

Run locally:

```bash
cargo fmt --check
cargo check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
git diff --check
```

When the phase touches Linux-only Smithay code, also rely on GitHub Actions for:

```text
smithay-linux check/test
```

If a local command cannot run, report the exact blocker.

## 9. Git Requirements

- Use a dedicated worktree branch.
- Do not commit directly on `main`.
- Commit message:

```text
<type>: <short phase description>
```

- Push the branch.
- Do not merge `main`.
- Do not create a PR unless explicitly requested.

## 10. Final Report Format

Use this structure:

```markdown
## 1. MCP 使用情况
- project:
- root_path:
- index_status:
- worktree 索引:

## 2. RTK 使用情况
- headroom:
- rtk:
- fallback:

## 3. Skills 使用情况
- 默认必读:
- 额外读取:
- 未发现:

## 4. 当前真实阶段判断
- ...

## 5. 完成类型
- ...

## 6. 修改文件
- ...

## 7. 新增 API
- ...

## 8. feature gate
- ...

## 9. capability / blocker
- ...

## 10. 测试结果
- ...

## 11. Git 状态
- branch:
- commit:
- pushed:
- status:

## 12. 风险
- ...

## 13. 下一步
- ...
```
