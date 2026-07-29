# Sky Mirror 项目状态

## 元数据

- 记录时间：2026-07-30 00:02:29 +08:00
- 仓库：DoubleO0O/Sky_Mirror
- 分支：main
- HEAD：21916d9af92ef94dc1bc8b69baf5d17636179c4a
- origin/main：21916d9af92ef94dc1bc8b69baf5d17636179c4a
- 工作区：创建本文档前为 clean
- 最近完成阶段：Phase 56P
- 候选阶段：Phase 56Q
- 候选阶段权限：NOT APPROVED
- 本轮本地 Cargo 验证：NOT RUN

## 失效条件

- HEAD 或 `origin/main` 移动后，本状态必须重新核对。
- 相关源码存在未提交修改后，本状态不能作为干净基线。
- 阶段状态发生变化后，本状态必须更新。
- 依赖、Cargo features 或构建边界发生变化后，相关结论失效。
- 权威 PR 或 CI 状态发生变化后，历史证据必须重新分类。
- 使用 Codebase Memory 前，必须在当前工作区重新验证 canonical root、branch 和 HEAD。
- 本机工具链、索引和 EOL 状态不由本项目状态文档持久化。

## 当前工程位置

- 当前形态是纯数据 Core + feature-gated proof + 部分 Linux nested runtime。
- 它不是可见合成器。
- 它不是 production desktop session。
- 当前 `main` 不会启动 production nested runtime。
- 当前状态只允许按证据描述已有链路，不允许外推桌面可用性。

## 当前代码与 production nested runtime 可证明事实

- Core 是平台中立的纯数据状态机，已表达 workspace、slot、stack 与 focus 状态。
- 平台事件存在 `BackendEvent -> CoreCommand -> State` 的命令路径。
- client、surface、window registry 是纯数据结构及对应状态处理。
- 已有内部 production nested constructor。
- nested 初始化顺序可在 socket 公开前建立 `wl_compositor` 与 `xdg_wm_base`。
- 当前 `main` 仍不启动 production nested runtime。
- 上述能力不代表已有可见桌面或真实渲染输出。

## 权威历史测试和外部客户端证据

- PR #92 已合并，合并提交为当前记录的 HEAD。
- `main` CI 运行 30459769481 的历史结果为 success。
- external registry discovery 提供了外部客户端历史证据。
- external bind 曾绑定 `wl_compositor` 与 `xdg_wm_base` 两个 globals。
- bounded protocol harness、watchdog 与 socket cleanup 有历史测试证据。
- 九项矩阵和相关测试数量只属于历史证据。
- 上述历史证据本轮均未重新运行。
- external client evidence 不能写回 server runtime report。
- 这些证据不能用于宣称当前 `main` 已成为 production compositor。
- 本轮未运行 Cargo 测试，不得把历史 CI 绿色表述为当前本地验证通过。

## 受控证明、骨架与 Mock

- XDG object、callback 与 admission 属于 controlled proof。
- callback observation 只证明回调被观察到。
- SHM metadata / readiness 只表达元数据和准备条件。
- DummyRenderer 只提供 owner proof。
- MockRenderer 只提供测试替身行为。
- InputSimulator 只提供受控输入模拟。
- DRM 当前是日志 stub。
- JSON session 仅具有有限可靠性，不能视为生产会话恢复。
- proof、skeleton、mock、report 与 readiness 均不得描述为 production。

## 尚未实施

- `main` 启动 production nested session。
- production external XDG lifecycle 闭环。
- 真实 client owner 传播。
- destroy identity retire。
- owned `WlBuffer` 生命周期。
- 真实 buffer import 与 texture。
- 真实 renderer 与 damage。
- 真实 frame done。
- 真实 input。
- DRM、GBM、dmabuf、EGL、GLES、Vulkan 或 WGPU 图形链路。
- libinput、udev、seat 或 session 集成。

## Phase 56Q 候选

- 状态：NOT APPROVED。
- 当前不得创建 Phase 56Q 分支或 worktree。
- 当前不得为 Phase 56Q 编写 Red、运行测试或开始实现。
- 候选目标是让外部客户端通过 `NestedRuntimeOrchestrator::start()` 的 production public path 依次创建 `wl_surface -> xdg_surface -> xdg_toplevel`。
- client session 通过现有 session bridge 解析为 core `ClientId`。
- window 只通过 `BackendEvent -> CoreRuntimeBridge -> CoreCommand -> State` 标准链路注册。
- `xdg_toplevel` destroy 后，adapter toplevel identity 进入 retire / tombstone。
- `xdg_toplevel` destroy 后，ledger 执行 unmap，core window 执行 detach / dead。
- 仅销毁 `xdg_toplevel` 时，底层 `wl_surface` 继续存活。
- `wl_surface` 只有在独立 surface destroy 或 client disconnect 时才进入相应关闭流程。
- buffer、import、texture、renderer、damage、frame done 和 input 继续保持未实现。

## 当前阻塞项

1. Phase 56Q 的目标、allowlist、Red / Green 和风险边界尚未审批。
2. Phase 56P 文档中的 release 状态仍是历史 PENDING。

## 下一步安全顺序

1. 用户审阅并明确批准本次两个基础文档进入 Git。
2. 单独审批 Phase 56Q 的目标和高风险边界。
3. 获批后重新确认 baseline 与 allow / deny 范围。
4. 仅在审批后按 Q1-Q5 创建 worktree 并建立正确 Red。

## 使用说明

- 开始新任务时先核对本文件的失效条件。
- 实时 Git、代码、权威 PR 与 CI 事实优先于本文件。
- 发生冲突时标记本文档 STALE，不得用旧结论覆盖实时证据。
- 阶段细节继续以 `docs/phases` 下的文档为补充，不在此复制完整历史。
