# Codex Environment Rules for Sky / Sky Mirror

## 1. Purpose

这是 Codex 重新安装后的执行环境规范，不是产品功能文档。它记录当前可验证的仓库、MCP、plugins、skills、RTK/Headroom、worktree 和报告规则，避免 future phases 依赖旧聊天记录或旧索引状态。

## 2. Repository

- Main repo path: /Users/double/Code/Sky_Mirror
- Old repo path /Users/double/sky_mirror is not the active repo.
- GitHub repo: DoubleO0O/Sky_Mirror
- Main branch: main
- MCP must index only /Users/double/Code/Sky_Mirror.

## 3. MCP Rules

- 只索引主项目 /Users/double/Code/Sky_Mirror。
- MCP project name: `Users-double-Code-Sky_Mirror`。
- MCP root_path: `/Users/double/Code/Sky_Mirror`。
- MCP status: `ready`。
- 当前索引规模：3942 nodes, 14563 edges, 13500416 size_bytes。
- project name 以 `list_projects` 实际返回为准；当前必须只有 `Users-double-Code-Sky_Mirror`。
- 禁止索引 /Users/double/sky_mirror。
- 禁止索引 /Users/double/.config/superpowers/worktrees/...。
- `index_status` 使用 project name，不使用 path。
- 优先使用原生 `mcp__codebase_memory` 工具：`list_projects`、`index_status`、`search_code`、`search_graph`、`get_code_snippet`。
- worktree 开发时 MCP 仍查主项目。
- worktree 当前改动用 `git diff`、`rg`、文件读取确认。
- 如果 codebase-memory-mcp / memory-mcp 工具不可用，必须报告不可用，不能伪造 `list_projects`、`delete_project`、`index_repository` 或 `index_status` 结果。

## 4. RTK / Headroom Rules

- `headroom` 当前可见路径：/Users/double/.local/bin/headroom。
- `rtk` 当前不在 PATH。
- `/Users/double/.headroom/bin/rtk` 当前存在且可执行。
- `rtk` 不支持复杂命令时允许 fallback 到原生命令。
- fallback 必须报告。
- 不伪造 rtk 使用；没有使用 rtk 就写没有使用。

## 5. Skills Rules

默认可用 / 常用必读 skills：

- `using-agent-skills`
- `source-driven-development`
- `test-driven-development`
- `git-workflow-and-versioning`
- `documentation-and-adrs`
- `code-review-and-quality`

设计/API 时再读：

- `api-and-interface-design`
- `codebase-design`
- `spec-driven-development`
- `documentation-and-adrs`
- `context-engineering`

CI/故障时再读：

- `gh-fix-ci`
- `ci-cd-and-automation`
- `systematic-debugging`
- `debugging-and-error-recovery`

Superpowers plugin skills 当前可见：

- `using-superpowers`
- `using-git-worktrees`
- `verification-before-completion`
- `systematic-debugging`
- `test-driven-development`

不存在或不应假设存在：

- `find-skills`
- `rust-skills`

## 6. Plugin Rules

当前 config.toml enabled plugins：

- `browser@openai-bundled`
- `chrome@openai-bundled`
- `computer-use@openai-bundled`
- `documents@openai-primary-runtime`
- `pdf@openai-primary-runtime`
- `spreadsheets@openai-primary-runtime`
- `presentations@openai-primary-runtime`
- `template-creator@openai-primary-runtime`

当前 cache 中可见但未在 config.toml enabled 列表中的 curated plugins：

- `github@openai-curated-remote` version 0.1.5
- `superpowers@openai-curated-remote` version 5.1.4

MCP / memory 状态：

- codebase-memory-mcp 已修复并启用。
- 原生 `mcp__codebase_memory` 工具已在新 Codex 会话中验证可用。
- 已验证工具：`list_projects`、`index_status`、`search_code`、`search_graph`、`get_code_snippet`。
- 只剩一个 MCP 项目：`Users-double-Code-Sky_Mirror`。
- 当前 root_path：`/Users/double/Code/Sky_Mirror`。
- 当前 status：`ready`。
- 不得把 Codex App thread/project 工具误报为 codebase-memory-mcp。

## 7. Worktree Rules

- 不在 main 上开发功能。
- 每个 Phase 用独立 worktree 或独立分支。
- worktree 不进入 MCP 索引。
- 未合并 worktree 必须保留。
- 删除 worktree 前必须确认 clean + merged。
- 本次新主仓 `/Users/double/Code/Sky_Mirror` 当前只发现主 worktree。
- 旧路径 `/Users/double/sky_mirror` 不再作为主仓使用，不因其 dirty 状态阻塞新主仓任务。

## 8. Sky Architecture Rules

- core 不依赖 Smithay / Wayland / Linux graphics stack。
- Smithay 类型只在 smithay_backend / adapter / glue 层。
- backend 不直接改 workspace / slot / stack。
- 状态变化走 Action / Command / State。
- 不提前做 render/input/taskbar/widgets。
- Linux-only API 必须使用 `#[cfg(all(feature = "smithay-linux", target_os = "linux"))]`。
- controlled proof 不能写成 real runtime capability，除非有对应阶段目标和验证。

## 9. Reporting Rules

- 报告 MCP 状态。
- 报告 skills/plugins/RTK 状态。
- 报告 Git 状态。
- 报告测试结果。
- 报告未完成项和风险。
- 报告是否只修改允许文件。
- 报告是否修改 Rust / Cargo / CI；环境文档任务中应为否。
