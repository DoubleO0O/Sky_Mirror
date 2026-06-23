# Codex Environment Rules for Sky / Sky Mirror

## 1. Purpose

本文件是 Sky / Sky Mirror 的 Codex 执行环境规范，用来约束索引、worktree、skills、RTK/Headroom、验证和报告方式。它不是产品功能文档，也不改变项目 runtime capability。

## 2. Repository

- Main repo path: `/Users/double/sky_mirror`
- GitHub repo: `DoubleO0O/Sky_Mirror`
- Main branch: `main`

## 3. MCP Rules

- 只使用主项目 MCP 索引：`Users-double_sky_mirror` / `Users-double-sky_mirror`，以 `list_projects` 实际返回的 project name 为准。
- 主项目 root path 必须是 `/Users/double/sky_mirror`。
- 禁止索引 `/Users/double/.config/superpowers/worktrees/...` 下的任何 worktree。
- 在 worktree 内开发时，MCP 查询仍使用主项目索引；worktree 当前修改用 `git diff`、`rg`、文件读取确认。
- `index_status` 参数必须是 project name，不是 filesystem path。
- 错误示例：`index_status /Users/double/sky_mirror`
- 正确示例：`index_status Users-double-sky_mirror`
- 需要重建索引时，优先使用 `index_repository(repo_path="/Users/double/sky_mirror", persistence=false)`，避免写入仓库 artifact。

## 4. RTK / Headroom Rules

- `rtk` 可能不在 `PATH`。
- 优先使用 `/Users/double/.headroom/bin/rtk`。
- 当 `rtk` 不支持复杂参数、过滤输出影响判断、或包装环境缺少必要命令时，允许 fallback 到原生命令或 `/usr/bin/...`。
- fallback 必须在报告中说明，包括失败命令和替代方式。
- 不伪造 RTK 使用；没有使用就写没有使用。

## 5. Skills Rules

默认必读：

- `using-agent-skills`
- `using-superpowers`
- `using-git-worktrees`
- `git-workflow-and-versioning`
- `test-driven-development`
- `verification-before-completion`
- `source-driven-development`

设计/API 变更时再读：

- `api-and-interface-design`
- `spec-driven-development`
- `documentation-and-adrs`
- `context-engineering`

CI/故障时再读：

- `gh-fix-ci`
- `ci-cd-and-automation`
- `systematic-debugging`
- `debugging-and-error-recovery`

不要假设存在：

- `find-skills`
- `rust-skills`
- `codebase-design`

如果这些 skill 在当前 skill 路径中不存在，报告必须写清：`find-skills / rust-skills / codebase-design 未在当前 skill 路径中发现，未伪造使用。`

## 6. Worktree Rules

- 不在 `main` 上开发功能或阶段实现。
- 每个 Phase 使用独立 worktree 和独立分支。
- worktree 不进入 MCP 索引。
- 已合并进 `main` 且工作区 clean 的 worktree 可以清理。
- 未合并 worktree 必须保留。
- 合并状态不确定、工作区不干净、或分支仍在当前进行中时必须保留。
- 本轮默认不删除远程分支；删除远程已合并分支需要用户明确确认。

## 7. Feature Gate Rules

- `core` 不依赖 Smithay、Wayland 或 Linux graphics stack。
- Linux-only API 必须使用 `#[cfg(all(feature = "smithay-linux", target_os = "linux"))]`。
- `default` / `smithay-probe` 不暴露 Linux runtime types。
- controlled proof 只能证明受控前置条件和观察结果，不能自动升级为 real runtime capability。

## 8. Reporting Rules

每轮最终报告必须说明：

- MCP 项目列表、root path、nodes/edges、`index_status`；
- skills 使用情况和缺失 skill；
- RTK / Headroom 使用情况与 fallback；
- Git branch、commit、push、`git status -sb`；
- 测试命令、结果、失败或跳过原因；
- 未完成项、风险、保留 worktree 和原因。

## 9. What Not To Do

- 不索引 worktree。
- 不伪造 MCP 状态。
- 不伪造 RTK 使用。
- 不夸大 capability。
- 不把 controlled proof 写成 real runtime。
- 不提前做 render、input、taskbar、widgets。
- 不删除 `/Users/double/sky_mirror` 主仓库。
- 不删除未合并 worktree。
- 不删除未合并远程分支。
- 不修改 `Cargo.toml`、`Cargo.lock`、Rust 业务代码或 CI，除非当前任务明确要求。
