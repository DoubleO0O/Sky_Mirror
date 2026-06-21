# Sky Mirror 项目体检与结构梳理报告

审计时间：2026-06-19（Asia/Shanghai）  
审计方式：只读扫描、Git/Cargo 安全检查、源码静态分析；除本报告外未修改项目文件。  
审计主机：macOS Darwin 25.5.0，arm64；Rust/Cargo 1.96.0。  

## 0. Codex 当前可用能力盘点

### 可访问项目根目录

- 当前工作目录：`/Users/double/sky_mirror`
- Git 顶层目录：`/Users/double/sky_mirror`
- 文件系统权限允许读取整个项目；本轮按用户要求只新增本报告，不修改源码、不自动修复、不提交 Git。

### 当前可用工具能力

- 本地终端：执行 shell、Git、Cargo、`rg`、`find`、`sed`、`awk` 等命令，可读取退出码和完整错误。
- 文件能力：读取项目文件、查看图片、用补丁方式新增/修改文件；本轮只使用补丁新增 `SKY_PROJECT_AUDIT.md`。
- 工程协作：内部任务计划、长任务等待、Git 状态检查、可发现的线程/自动化/插件能力。
- 网络与外部资料：Web 搜索和页面读取可用；本轮结论均可由本地仓库验证，因此未使用互联网。
- 视觉与办公文档：图片生成/编辑、DOCX、PDF、PPTX、电子表格处理能力可用；本轮未使用。
- 浏览器/UI：Codex 内置浏览器、Chrome、macOS Computer Use 插件能力可用；本轮无需使用。
- 子代理能力在运行环境中可见，但工具规则要求用户明确授权后才能启动；本轮没有启动子代理。

### 当前可读取的本地 skills / plugins

能够读取，已确认以下目录：

- `/Users/double/.agents/skills`
- `/Users/double/.codex/skills/.system`
- `/Users/double/.codex/plugins/cache`

本地 agent skills：

- `codebase-design`
- `diagnosing-bugs`
- `domain-modeling`
- `find-skills`
- `grill-with-docs`
- `handoff`
- `improve-codebase-architecture`
- `rust-skills`
- `tdd`
- `to-issues`
- `to-prd`

Codex system skills：

- `imagegen`
- `openai-docs`
- `plugin-creator`
- `skill-creator`
- `skill-installer`

已安装插件及其 skills：

- Browser：`control-in-app-browser`
- Chrome：`control-chrome`
- Computer Use：`computer-use`
- GitHub：`github`、`gh-address-comments`、`gh-fix-ci`、`yeet`
- Superpowers：`using-superpowers`、`brainstorming`、`writing-plans`、`dispatching-parallel-agents`、`executing-plans`、`subagent-driven-development`、`systematic-debugging`、`test-driven-development`、`verification-before-completion`、`requesting-code-review`、`receiving-code-review`、`using-git-worktrees`、`finishing-a-development-branch`、`writing-skills`
- Documents / PDF / Presentations / Spreadsheets：对应的文档、PDF、演示文稿和电子表格 skills

本轮实际使用了 `using-superpowers`、`writing-plans`（仅内部清单，未写计划文件）、`codebase-design`、`rust-skills` 和 `verification-before-completion`。`dispatching-parallel-agents` 的说明已读取，但因没有用户明确授权代理而未启动代理。

## 1. 当前工程状态

### Git 基线

- 项目根目录：`/Users/double/sky_mirror`
- 当前分支：`phase49r-dispatch-request-boundary-preconditions`
- 当前 HEAD：`49f7d6ebe2266df6d5b3b52b0368b2e3e10cce11`
- HEAD 提交：`feat: add Phase 49R dispatch request boundary preconditions`
- Git **不 clean**。
- 没有检测到源码 modified；有 **21 个已跟踪文件被删除**，均为 Phase 45–47 的 handoff patch/zip/README。
- `git diff --stat`：21 files changed，约 26,573 行删除；zip 为二进制删除。
- Cargo 检查前后 `git status --short` 一致，说明这些删除不是本轮命令造成的。

被删除的文件组：

- `phase45_46_handoff.{patch,zip}` 与 `phase45_46_handoff_README.txt`
- `phase45_47m_handoff.*`
- `phase45_47n_handoff.*`
- `phase45_47o_handoff.*`
- `phase45_47p_handoff.*`
- `phase45_47q_handoff.*`
- `phase45_47r_handoff.*`

### 最近 20 个 commit

```text
49f7d6e (HEAD -> phase49r-dispatch-request-boundary-preconditions, tag: phase49r-dispatch-request-boundary-preconditions) feat: add Phase 49R dispatch request boundary preconditions
21cbbae (tag: phase49q-global-dispatch-trait-boundary-preconditions, phase49q-global-dispatch-trait-boundary-preconditions) feat: add Phase 49Q global dispatch trait boundary preconditions
5059a68 (tag: phase49p-global-registration-promotion-preconditions, phase49p-global-registration-promotion-preconditions) feat: add Phase 49P global registration promotion preconditions
8adef0e (tag: phase49o-display-handle-internal-ownership-evidence, phase49o-display-handle-internal-ownership-evidence) feat: add Phase 49O display handle internal ownership evidence
e1d2369 (tag: phase49n-display-handle-public-api-evidence, phase49n-display-handle-public-api-evidence) feat: add Phase 49N display handle public API evidence
dfe2515 (tag: phase49m-display-handle-internal-gate, phase49m-display-handle-internal-gate) feat: add Phase 49M display handle internal access gate
c14bc0c (tag: phase49l-bind-shape-final-seal, phase49l-bind-shape-final-seal) feat: add Phase 49L bind shape final seal
30ab0aa (tag: phase49k-display-handle-policy, phase49k-display-handle-policy) feat: add Phase 49K display handle access policy
d3b9fd2 (tag: phase49j-bind-handler-state-model, phase49j-bind-handler-state-model) feat: add Phase 49J bind handler state model
a576c6a (tag: phase49i-bind-global-data-model, phase49i-bind-global-data-model) feat: add Phase 49I bind global data model
c9cfdc0 (tag: phase49h-bind-global-resource-identity, phase49h-bind-global-resource-identity) feat: add Phase 49H bind global resource identity model
22ed316 (tag: phase49g-bind-client-identity-model, phase49g-bind-client-identity-model) feat: add Phase 49G bind client identity model
d1bcef8 (tag: phase49f-global-dispatch-bind-shape, phase49f-global-dispatch-bind-shape) feat: add Phase 49F global dispatch bind shape probe
00eb5db (tag: phase49e-handler-reduction-plan, phase49e-requirement-reduction-plan) feat: add Phase 49E handler reduction plan
0cfa93d (tag: docs-comment-audit, docs-whole-repo-comment-audit) docs: audit and improve code comments
f82d296 (tag: phase49d-handler-requirement-matrix, phase49d-handler-requirement-matrix) feat: add Phase 49D handler requirement matrix
9763774 (tag: phase49c-inert-handler-compile-probe, phase49c-inert-handler-compile-probe) feat: add Phase 49C inert handler compile probe
fc36ce7 (tag: phase49b-inert-global-handler-boundary, phase49b-inert-global-handler-boundary) feat: add Phase 49B inert global handler boundary
e7b08ce (tag: phase49a-guarded-global-registration, phase49a-guarded-global-registration) feat: add Phase 49A guarded global registration feasibility
9b1ee41 (tag: phase48i-activation-attempt-ledger, tag: phase48-final-seal, phase48i-activation-attempt-ledger) feat: add Phase 48I activation attempt ledger
```

### 是否适合继续开发

代码层面可以继续分析和做隔离开发：默认构建、默认测试和 `smithay-probe` 测试通过。但当前工作树不适合直接提交或合并，必须先确认 21 个 handoff 删除是否是有意清理。否则下一次普通提交很容易把大批历史交付物删除混入功能变更。

另一个 Git 风险来自 `RECOVERY_NOTES.md`：仓库是在原 `.git` 丢失后重建，当前历史不能被当作完整原始历史。针对旧 public API 的历史判断只能基于恢复后的提交和当前源码，无法保证覆盖恢复前的全部调用方。

## 2. 项目目录结构总览

### 规模与顶层结构

- 跟踪的 Rust 文件：56 个
- `src` 总行数：约 30,085 行
- `src/core`：约 8,743 行
- `src/backend`：约 51 行
- `src/smithay_backend`：约 21,245 行，占源码约 70.6%
- 最大文件：`linux_handler_probe.rs` 7,146 行、`linux_adapter.rs` 3,604 行
- 顶层只有 `.gitignore`、Cargo 文件、`RECOVERY_NOTES.md`、`src`、`target`，以及本报告。
- `tests/`、`examples/`、`docs/`、`scripts/`、`.github/` 均不存在；测试全部内嵌在模块的 `#[cfg(test)]` 中。
- `target/` 已存在，为本地构建产物，不是源码。

### `src/main.rs`

单一 binary 入口。它创建 `State`、尝试读取 `sky_mirror_session.json`、创建 calloop `EventLoop` 并进入循环。入口没有启动 `smithay_backend`；即使启用 feature，Smithay 模块也只是被编译，主程序没有把它组装进实际运行路径。

### `src/core`

- `workspace.rs`：四固定 slot、`SlotContent`、`Stack`、`LayoutMode`、`WindowId`。
- `focus.rs`：`FocusState { workspace, slot, window }`。
- `state.rs`：`CompositorState` 和根 `State`；协调 workspace、focus、output、registry、surface、client 生命周期。
- `action.rs`：用户语义动作。
- `input.rs` / `keybinding.rs`：抽象输入事件、按键映射与 tick 模拟输入源。
- `command.rs`：外部系统进入核心的 `CoreCommand` / `CommandResult` / `CommandHandler`。
- `backend_event.rs`：纯数据 `BackendEvent` 到 `CoreCommand` 的翻译。
- `runtime_bridge.rs` / `backend_driver.rs`：单事件桥接和 backend driver seam。
- `backend_replay.rs`：纯数据事件序列回放器。
- `layout.rs`：无状态布局计算。
- `scene.rs`：placement 与 focus 合成为 scene。
- `render.rs`：render plan 和 `MockRenderer`。
- `client.rs` / `surface.rs` / `window.rs`：三个纯数据 registry。
- `session.rs`：workspace/focus/stack 的 Serde 会话镜像。
- `inspector.rs` / `validator.rs` / `diagnostics.rs`：只读快照、一致性验证和诊断包。
- `integration_contract.rs`：外部 backend 允许入口与禁止直接访问区域的纯数据契约说明。
- `event_loop.rs`：calloop 调度、模拟输入、mock render。
- `output.rs`：纯数据输出尺寸。
- `compositor.rs`：仅保留的高层组装占位模块。

### `src/backend`

- `drm.rs`：只有 `DrmBackend` 空结构和打印日志的 `new/init`，不打开 DRM 设备。
- `egl.rs`：空模块说明，没有 EGL display/context/surface。
- `input.rs`：空模块说明，没有 libinput/udev/seat。

目录名像真实系统 backend，但实现完全是 stub。

### `src/smithay_backend`

可分为四组：

1. 纯数据事件适配：`action_event`、`client_event`、`output_event`、`surface_event`、`toplevel_event`、ID allocator、`driver`、`runtime`、`scenario`。
2. 纯数据 surface 预演：`surface_lifecycle`、`surface_trace`、`surface_window_intent`、`window_admission_preview`、`surface_admission_pipeline`、`surface_admission_contract`。
3. Linux 资源探针：`wayland_display`、`wayland_socket`、`bootstrap`、`linux_runtime`。
4. Linux skeleton/evidence：`linux_adapter`、`linux_handler_probe`、`runtime_facade`。

当前项目更像“核心状态机 + 大量 Smithay 接入前契约/证据”的原型，不是可运行 compositor。目录规模已经明显向 probe/evidence 倾斜：Linux handler 的前置条件、readiness、blocker、ledger 类型很多，但真实 handler、client accept、protocol dispatch 为零。

## 3. Cargo / Feature / 依赖分析

### Package / workspace

- package：`sky_mirror 0.1.0`
- Rust edition：2024
- target：只有 `src/main.rs` 一个 binary；没有 `lib.rs`
- 没有显式 `[workspace]`。Cargo metadata 将单一 package 视为 workspace root 和唯一 member。
- 未声明 `rust-version`、license、description、repository、README 等 package metadata。

### 直接依赖

- `calloop = 0.13`，锁定 0.13.0：默认运行循环。
- `serde = 1` + derive，锁定 1.0.228：session 数据。
- `serde_json = 1`，锁定 1.0.150：session JSON。
- Linux target 下 optional `smithay = 0.7`，锁定 0.7.0；关闭 Smithay default features，仅启用 `wayland_frontend`。

### Features

```toml
default = []
smithay-probe = []
smithay-linux = ["smithay-probe", "dep:smithay"]
smithay-backend = ["smithay-linux"]
```

- 默认 feature 为空，不编译 `smithay_backend`。
- `smithay-probe` 本身不拉取 Smithay crate，只打开纯数据模块。
- `smithay-linux` 是 additive feature：包含 `smithay-probe` 并启用 Linux-only Smithay 依赖。
- `smithay-backend` 是旧名称兼容别名，转发到 `smithay-linux`。
- 非 Linux 上启用 `smithay-linux` 会主动 `compile_error!`，错误清晰。

### 边界判断

- 没有 Linux-only Smithay、udev、libinput、DRM、GBM、X11、Vulkan 依赖被放进默认构建。
- Smithay 只出现在 `target.'cfg(target_os = "linux")'.dependencies` 且 optional，feature gate 总体正确。
- `smithay` 的 `wayland_frontend` 会带入 Wayland server/protocol、xkbcommon、drm-fourcc 等传递依赖，但这只发生在 Linux feature 解析中，并不等于项目已使用 DRM/GPU。
- `smithay-probe` check 产生 176 个 dead-code warnings，说明 feature 打开的公共面很大，但 binary 入口不消费这些类型。大量 `pub` 主要服务模块内测试，不是被运行时使用的深模块 interface。
- 只有 binary target，当前所谓 public API 并不是可供外部 crate 正常依赖的 library API；但它仍是仓库内 feature、测试和未来拆 library 时的兼容面。
- 缺少 CI 配置；本轮 macOS 无法验证 Linux-only 源码。历史 `RECOVERY_NOTES.md` 记载 Phase 47 在 Arch 通过，但不能替代当前 HEAD 的 Linux CI。

## 4. core 层结构分析

### State / CompositorState

- `CompositorState` 持有 `DrmBackend` stub、`Vec<Workspace>`、当前 workspace ID、`FocusState`、`OutputState` 和 `running`。
- `State` 再持有 `CompositorState`、`WindowRegistry`、`SurfaceRegistry`、`ClientRegistry`。
- `State::new()` 默认创建 3 个 workspace 和 3 个 mock window；这是演示初态，不是空 compositor 初态。
- session 只保存 workspace/focus/stack/next window ID；backend、output 和 metadata 不保存。

### Workspace / Slot / Stack / FocusState

- 每个 workspace 始终有 `[Slot; 4]`。
- `SlotContent = Empty | Single(WindowId) | Stack(Stack)`。
- 前四个窗口依次占 slot 0–3；第五个及以后固定进入 slot 0 stack。
- stack push 会把新窗口设为 active；remove 会修正索引并在剩 1 个窗口时降级为 Single。
- `Workspace::slot_window()` 是统一可见性入口，Stack 只返回 active window。
- `FocusState` 显式保存 workspace/slot/window，`CompositorState::refresh_focus()` 负责一致性修复。

### Action / Command / InputEvent

- `InputEvent -> Action -> State::dispatch_action()` 路径存在。
- 外部事实走 `BackendEvent -> BackendEventTranslator -> CoreCommand -> State::handle_command()`。
- `CommandHandler` 协调 client/surface/window 注册、绑定、关闭和诊断。
- `CloseClient` 实现会级联关闭 surface/window/workspace 引用；但 `CoreCommand::CloseClient` 的一处文档仍写“只标记 client dead，不自动关闭 surface/window”，与实现和其他文档不一致，属于注释/API 契约漂移风险。

### RuntimeBridge / BackendEvent

- `CoreRuntimeBridge::handle_backend_event()` 固定执行翻译、命令、验证。
- `BackendDriver` trait 只产出纯数据 `BackendEvent`，不接触 `State`。
- `BackendDriverRunner` 是真实 backend 应接入的 seam；当前 Smithay probe 遵守它。

### core 纯净度判断

- `src/core` 生产代码没有导入 `smithay`、Wayland server、udev、libinput、GBM、X11、Vulkan 或真实 DRM 类型。
- domain 模块（workspace/layout/scene/registry/command）基本是纯 Rust 数据与状态机。
- 但整个 `core` 目录并非完全纯净：
  - `core::state` 直接依赖 `crate::backend::drm::DrmBackend`，即便该类型只是 stub，也使 domain 根状态知道 backend 实现。
  - `core::event_loop` 直接依赖 `calloop` 并持有 `MockRenderer`、`InputSimulator`。
- 因此更准确的结论是：**核心业务模型纯净，`core` 目录的运行时组装层不纯净，seam 尚未按目录/模块彻底分离。**

### backend 泄漏和直接修改风险

- 当前 Smithay 生产代码没有直接写 `State.compositor.workspaces`、slot 或 stack；状态变化走 `BackendEvent/CoreCommand/State`。
- 搜到的 Smithay 对 `state.compositor`、`state.registry`、`state.surfaces` 直接访问主要在测试断言中，是只读验证。
- 风险仍存在，因为 `State`、`CompositorState` 的关键字段和 `current_workspace_mut()` 是 `pub`；`CoreIntegrationContract` 只是纯数据说明，编译器并不强制禁止 backend 绕过它。
- `ClientId`、`SurfaceId`、`WindowId` 在 core 中都是 `u64` type alias，不是 newtype，编译器无法阻止三种 ID 被误传。

## 5. layout / scene / render pipeline 分析

当前链路确实存在：

```text
Workspace / State
  -> LayoutEngine::compute_workspace
  -> SceneBuilder::build
  -> RenderPlanner::from_scene
  -> RenderFrame
  -> MockRenderer::render
```

### 布局规则

- Fullscreen：只读取 slot 0，窗口占满输出；slot 0 空时返回空 placement。
- Split：slot 0 左半、slot 1 右半；奇数宽度余数给右侧。空 slot 不产生 placement，但不会重新压缩位置。
- Grid：slot 0/1/2/3 对应左上/右上/左下/右下；奇数宽高余数给右列/下行。
- Empty workspace：placement、scene nodes、render commands 都为空，不 panic。
- Stack：所有布局统一调用 `slot_window()`，只显示 active window。
- 最大可见数符合 0/1/2/4 模型。

### Fullscreen 是否 focus-aware

否。`LayoutEngine` 只接收 `Workspace` 和 `OutputSize`，完全不知道 `FocusState`；Fullscreen 被测试明确固定为 slot 0。若用户在 Split/Grid 下把焦点移到 slot 1–3，再切回 Fullscreen，`set_current_layout()` 只调用 `refresh_focus()`，它会保留当前 occupied slot，因此可能出现：

- 焦点仍指向 slot 1–3 的窗口；
- Fullscreen 实际只绘制 slot 0；
- Scene 中没有任何节点匹配 focused window。

这是当前 layout/focus 契约中最明确的功能缺口。需要先决定产品语义：Fullscreen 是“固定 slot 0”还是“当前 focused slot”；项目背景倾向后者。

### Scene / render

- `SceneBuilder` 给 focused node `z_index = 10`，其他为 0，并按 z-index 排序。
- `RenderPlanner` 只生成 `DrawWindow` 纯数据命令；`State` 再附加 WindowRegistry metadata。
- `MockRenderer` 只 `println!`，没有 buffer、damage、frame callback、texture、Smithay renderer、OpenGL/Vulkan/wgpu。
- 当前没有真实 GPU render，也没有真实 Wayland surface 内容进入 RenderFrame。

## 6. registry / lifecycle 分析

### ClientRegistry

- 自动或指定 `ClientId` 注册；保存 kind、alive、可选 name。
- 可 mark dead、查询、列举、维护 next ID。
- 不保存真实 client、socket、credentials 或 Smithay `Client`。

### SurfaceRegistry

- 自动或指定 `SurfaceId` 注册；保存可选 `ClientId`、可选 `WindowId`、`SurfaceRole`、alive。
- 支持 bind client、bind window、按 client/window 批量 mark dead、查询双向关系中的部分方向。
- `SurfaceRole` 可以表达 `XdgToplevel`、`XdgPopup`、`LayerShell`、`Unknown`，但只是枚举标签。
- 不保存 `WlSurface`、xdg resource、commit state、buffer、damage 或 configure serial。

### WindowRegistry

- 是 core `WindowId` 的唯一正常分配者；保存 title/app_id/kind/alive。
- 支持 session restore 的 mock metadata 补齐和 next ID 修正。
- 关闭后保留 tombstone metadata，便于诊断。
- 不保存真实 surface handle；绑定关系在 SurfaceRegistry。

### ID 关系

```text
ClientId (连接)
  -> SurfaceRecord.client
SurfaceId (协议 surface 占位)
  -> SurfaceRecord.window
WindowId (核心逻辑窗口)
  -> Workspace Slot/Stack + WindowRegistry metadata
```

关系模型清楚，但 type alias 不提供编译期隔离；真实对象到这些 ID 的 adapter-owned map 尚不存在。

### 生命周期

- core 可表达 client connect/disconnect、surface create/bind/close、toplevel map 为 window、window close。
- client disconnect 会级联 surface/window/workspace；记录保留为 dead。
- `smithay_backend::surface_lifecycle` 另有一套纯数据 `BackendSurfaceRegistry`，能表达 Created/Configured/Mapped/Unmapped/Destroyed 和 tombstone，并被 trace/intent/preview pipeline 使用。
- 这套 backend lifecycle 与 core `SurfaceRegistry` 是两套模型，目前通过预演/契约联系，而非真实运行时统一映射。若继续扩展，存在双状态源和同步规则重复的风险。

### 真实协议接入判断

- 能表达“未来 xdg_toplevel 的纯数据事实”，不能管理真实 xdg_toplevel。
- 没有真实 `WlSurface` / xdg_toplevel 对象接入。
- 没有 map/unmap/commit callback；只有手工推入描述符和 mock trace。

## 7. Smithay 接入状态分析

### 各关键模块

- `runtime.rs`：纯数据事件队列、ID allocator、driver；不依赖 Smithay crate。
- `runtime_facade.rs`：只读能力/诊断报告；明确 real surface 和 GPU 为 false。
- `wayland_display.rs`：Linux 下真实构造 `wayland_server::Display<SmithayWaylandState>` 并可取 `DisplayHandle`。
- `wayland_socket.rs`：Linux 下真实构造 `ListeningSocketSource` 并读取 socket 名。
- `bootstrap.rs`：同时持有 Display probe 和 socket probe，但不连接二者。
- `linux_runtime.rs`：组合 bootstrap 与纯数据 runtime；事件仍是预置队列。
- `linux_adapter.rs`：资源所有权、生命周期、pump 计数、global plan、blocked ledger；明确 skeleton only。
- `linux_handler_probe.rs`：只定义 inert type、requirement matrix、bind shape、precondition/evidence 报告。它只导入 `linux_adapter` 的纯数据 enum，没有实现 Smithay `GlobalDispatch` / `Dispatch`，没有 delegate 宏，没有 `create_global`。

### 明确回答

- 是否有真实 Wayland Display：**有资源对象**，仅 Linux feature 下可构造；未进入主程序和调度循环。
- 是否有真实 ListeningSocket：**有资源对象**，仅创建/持有/读取名称；未加入 calloop。
- 是否能接受 client connection：**不能**。没有 accept、`insert_client`、client dispatch/flush。
- 是否注册 xdg-shell global：**不能**。只有 Compositor/Shm/XdgWmBase 的字符串计划和 skeleton ledger，真实注册数固定为 0。
- 是否能创建/管理 `wl_surface`：**不能**。只有纯数据 surface ID/registry/trace。
- 是否能 map xdg_toplevel：**不能处理真实对象**。只能把手工 descriptor 转成 `BackendEvent::ToplevelMapped`。
- 是否能 render real surface：**不能**。RenderFrame 只有逻辑 ID 和矩形，renderer 是日志 mock。
- 当前是否只是 probe / skeleton：**是**。项目自己的 capability 字段也固定声明 `accepts_clients=false`、`registers_protocol_globals=false`、`dispatches_protocol_events=false`、`supports_real_wayland_surfaces=false`、`supports_gpu_rendering=false`。

### 离 nested compositor MVP 的缺口

至少需要一个真实 Linux 纵向切片：

1. 将 Display、ListeningSocketSource 和运行时 state 放入同一个可调度 runtime。
2. 把 socket source 插入 calloop，accept stream 并 `insert_client`。
3. 建立最小 `wl_compositor`、`wl_shm`、`xdg_wm_base` protocol state/global。
4. 实现所需 `GlobalDispatch` / `Dispatch` / Smithay handler 与 delegate 宏。
5. 在 adapter 内维护真实 client/resource/WlSurface/xdg_toplevel 到 core ID 的映射。
6. 将 create/map/unmap/destroy/metadata/commit 回调翻译为既有 `BackendEvent`，只通过 core command seam 修改状态。
7. 实际 dispatch clients、flush、处理 lifecycle 和错误。
8. 把真实 surface tree/buffer 进入 renderer，发 frame callback；nested MVP 可先使用嵌套窗口/软件或 Smithay renderer，不应先跳到 DRM/GBM。
9. 接入最小 seat/keyboard/pointer，仍转换到 InputEvent/Action。

## 8. 测试状态

| 命令 | 结果 | 说明 |
|---|---:|---|
| `cargo fmt --check` | PASS，exit 0 | 无输出，格式检查通过 |
| `cargo test` | PASS，exit 0 | 131 passed，0 failed，0 ignored；6 组 dead-code warnings |
| `cargo check --features smithay-probe` | PASS，exit 0 | 编译通过；产生 176 个 dead-code warnings |
| `cargo test --features smithay-probe` | PASS，exit 0 | 413 passed，0 failed，0 ignored；6 组 warnings |
| `cargo check --features smithay-linux` | FAIL，exit 101 | macOS 命中项目主动 `compile_error!`；属于平台限制，不是当前目录错误、feature 缺失或已证明的代码错误 |
| `cargo test --features smithay-linux` | FAIL，exit 101 | 同上，Linux-only 测试未编译、未运行 |

源码中静态统计到 476 个 `#[test]`，0 个 `#[ignore]`。在当前 feature 结构下，可推断 63 个为 Linux feature 增量测试（476 - 413），但本轮没有在 Linux 上实际执行它们，因此不能报告为通过。

当前机器只安装 `aarch64-apple-darwin` Rust target，没有 Linux cross target，也没有 `XDG_RUNTIME_DIR`。Linux 测试使用 `assert_runtime_dir()`，缺少运行时目录会明确 panic/fail，而不是提前 return；因此没有发现“环境不足被 skip 却看起来 pass”的代码路径。风险在于：当前没有 CI 文件，本轮也无法确认当前 HEAD 在真实 Linux 上仍通过。

默认和 probe 测试非常多，但大部分验证的是纯数据模型、报告字段、源码字符串中“不包含某 API”以及 blocker/evidence 一致性。测试数量不能等价为真实 compositor 集成成熟度。

## 9. Public API 风险

指定的四个 API 当前都存在于 `SmithayRuntimeProbe`，但只在 `smithay-linux + Linux` 下编译：

```rust
SmithayRuntimeProbe::with_socket_name
SmithayRuntimeProbe::from_parts
SmithayRuntimeProbe::bootstrap_mode
SmithayRuntimeProbe::socket_name_string
```

当前兼容设计：

- `SmithayRuntimeProbe` 在 Linux feature 下保留 `compatibility_bootstrap: Option<SmithayBootstrapProbe>`。
- `with_socket_name` 和 `from_parts(bootstrap, driver)` 保留旧式构造语义。
- `bootstrap_mode` 和 `socket_name_string` 保留旧式查询。
- 新 `SmithayLinuxRuntimeProbe` 也有同名方法，但其 `from_parts(bootstrap, runtime)` 第二参数类型不同。
- `linux_runtime` 有测试 `smithay_runtime_legacy_linux_api_remains_available`，`runtime_facade` 也覆盖 legacy runtime 转报告。

风险判断：

- 当前没有删除/重命名这四个方法，已有 compatibility wrapper。
- 但同名 API 分布在 `SmithayRuntimeProbe` 与 `SmithayLinuxRuntimeProbe`，非常容易在迁移时误把“相似名称”当作“相同签名”。
- 它们只在 Linux cfg 下存在，macOS 的 `smithay-probe` 构建无法编译验证这些兼容方法。
- 仓库没有 library target，外部 crate 当前无法把它当稳定库 API 使用；仓库内已跟踪调用方主要是 Linux tests/facade。未知的恢复前调用方或外部脚本：**不确定**。
- Git 是恢复仓库，`git log -S` 只能追到恢复基线 `9f419f1`，无法证明更早历史中没有其他旧签名。

建议保留这些 wrapper，等真实 Linux runtime 稳定后再标记 deprecated；不要直接删除。若未来增加 `lib.rs`，应先明确哪一个 runtime 类型是正式 interface，并在 Linux CI 编译 legacy API 测试。

## 10. 当前真实阶段判断

| 维度 | 判断 |
|---|---|
| 纯 core 状态机成熟度 | 中等偏高：slot/stack/focus/action/command/lifecycle/validator 较完整，但字段过度公开、ID 为 alias、core/runtime seam 混放 |
| layout engine 成熟度 | 中等：Fullscreen/Split/Grid 和奇数尺寸规则明确；缺 focus-aware Fullscreen、多输出、gap/border/scale/damage |
| registry 成熟度 | 中等：纯数据生命周期和级联较完整；没有真实对象映射，且 backend/core 有两套 surface 模型 |
| Smithay probe 成熟度 | 作为 probe 很高：模型、报告、负能力声明和测试很丰富 |
| nested compositor 成熟度 | 很低：没有真实 client accept、global、dispatch、surface callback |
| real Wayland surface 接入成熟度 | 未开始（0 条真实 surface 路径） |
| render pipeline 成熟度 | 规划层初步完成；执行层仍是日志 mock，没有 surface/GPU |
| input pipeline 成熟度 | 语义映射完成一部分；设备层只有 tick simulator |
| Linux DRM/GBM/libinput 后端成熟度 | 未开始；backend 文件是空壳/stub，Smithay 仅启用 wayland_frontend |

```text
当前项目真实阶段：纯 core 原型较完整，Smithay 接入仍处于 Display/socket 资源探针 + 大量纯数据契约/evidence skeleton 阶段。
不是：可接受 Wayland client 的 nested compositor，也不是具备真实 surface、输入或 GPU/DRM 输出的 compositor。
已经完成：固定 slot/stack/focus 状态机、Action/Command/BackendEvent seam、布局/scene/render plan、纯数据 registries/lifecycle、Display/socket 构造探针、较大规模纯数据测试。
尚未完成：socket accept/insert_client、protocol globals/handlers/dispatch、真实 wl_surface/xdg_toplevel 映射、真实 render、真实输入、DRM/GBM/libinput 后端。
最大风险：继续用 phase/precondition/evidence 增加“已准备”的表面积，却迟迟没有真实 Linux 纵向切片；同时 21 个 tracked 删除和缺失 Linux CI 会让集成状态不可靠。
```

## 11. 下一步建议，但不要写代码

### 先处理工程基线

1. 先确认 21 个 handoff 文件删除是保留、恢复还是单独提交，避免混入后续功能提交。
2. 建立至少 Linux + macOS probe 的 CI。Linux job 必须设置独立 `XDG_RUNTIME_DIR`，执行本报告中的完整 feature 矩阵。
3. 不要把 `RECOVERY_NOTES.md` 的历史 Arch 结果当当前 HEAD 的验证证据。

### 收束 core/runtime seam

在真实 Smithay 接线前，只处理会阻塞纵向切片的少数边界问题：

- 明确 Fullscreen 是 focused slot 还是固定 slot 0，并让 focus/layout 契约一致。
- 统一 workspace 数量：当前默认只有 ID 0–2，但 Super+4 映射到不存在的 ID 3。
- 决定是否把 `ClientId` / `SurfaceId` / `WindowId` 改为 newtype，以降低错绑风险。
- 将 calloop/EventLoop 和 DrmBackend stub 从纯 domain 模块 seam 中分离，或至少避免 `CompositorState` 直接拥有具体 backend。
- 收紧关键字段可见性，让“backend 不得直接改 workspace/registry”由编译器而非纯数据契约报告保证。
- 明确 core `SurfaceRegistry` 与 backend `BackendSurfaceRegistry` 的职责：前者应是核心逻辑视图，后者应是 adapter 内真实协议生命周期映射；不要继续复制第三套状态机。

这些问题应小范围收束，不应演变成长期重构。

### 停止继续堆 probe / evidence / precondition

建议停止继续新增 Phase 49 风格的 evidence、gate、seal、matrix、ledger，除非它直接服务下一条真实可执行路径。`linux_handler_probe.rs` 已达 7,146 行，最近 20 个提交几乎全部是 handler/global 的前置条件证据，但仍没有一个 Smithay handler trait 实现。继续扩展会增加 public surface、测试维护成本和认知负担，却不提高 compositor 能力。

### 进入 nested Wayland client connection

应该进入，而且应采用最小 tracer-bullet：

1. Linux runtime 真正拥有并调度 Display + socket。
2. 接受一个 client 并插入 Display；先只证明连接/断开事件能经 BackendEvent 到 core ClientRegistry。
3. 注册最小 globals 和 handler；随后让一个测试 client 创建 `wl_surface` + `xdg_toplevel`。
4. 把真实 resource identity 映射到 adapter-owned ID，再提交既有 BackendEvent；禁止 callback 直接改 State。
5. 先得到真实 map/unmap/destroy 和一个可见 nested surface，再扩展 renderer/input。

### 现在不应做的功能

- 不应先做 DRM/KMS、GBM、libinput、udev、多 GPU、XWayland、Vulkan。
- 不应继续扩展复杂 layout 动画、配置系统或 IPC，真实 client/surface 还没有接通。
- 不应把纯数据 `WouldCreateWindow`、global plan、pump tick 或 capability report 当作 runtime 完成度。

推荐下一阶段目标：**在 Linux 上完成“真实 socket accept → insert client → 最小 globals → 一个真实 xdg_toplevel map/unmap → BackendEvent/CoreCommand → core registry/workspace”的纵向闭环；渲染可在下一小步接入。**

## 12. 给 ChatGPT 架构规划用的摘要

```text
Sky Mirror 当前根目录 /Users/double/sky_mirror，分支 phase49r-dispatch-request-boundary-preconditions，HEAD 49f7d6ebe2266df6d5b3b52b0368b2e3e10cce11。Git 不 clean：21 个已跟踪的 Phase 45–47 handoff patch/zip/README 被删除，约 26,573 行删除；测试前后状态一致，删除不是本轮造成。仓库是 .git 丢失后的恢复历史，恢复前 API 历史不完整。

Cargo 是单 package、单 binary、Rust 2024；default=[]，smithay-probe=[]，smithay-linux=[smithay-probe, dep:smithay]，smithay-backend 是兼容别名。默认依赖 calloop/serde/serde_json；Smithay 0.7 只在 Linux target、optional、wayland_frontend feature 下启用，没有 Linux 图形栈进入默认构建。

本轮结果：cargo fmt --check PASS；cargo test 131 passed/0 failed/0 ignored；cargo check --features smithay-probe PASS（176 dead-code warnings）；cargo test --features smithay-probe 413 passed/0 failed/0 ignored；两条 smithay-linux check/test 在 macOS 按项目 compile_error! 失败，exit 101，当前 HEAD 的 Linux 编译/测试未验证。源码静态有 476 个 #[test]、0 ignored；Linux 测试使用 assert_runtime_dir，不会因环境缺失静默 skip。

主要模块：core 有 Workspace/4 Slots/Stack/Focus/Action/CoreCommand/BackendEvent/State、LayoutEngine、SceneBuilder、RenderPlanner、Client/Surface/Window registries、validator/inspector；backend 只有 DRM/EGL/input stub；smithay_backend 有纯数据 driver/runtime/lifecycle/trace/intent/preview pipeline，以及 Linux Display、ListeningSocket、bootstrap、adapter skeleton、handler evidence。

真实阶段：core 状态机和纯数据测试较成熟，Smithay 仍是资源探针/skeleton。Linux 下能构造真实 Display 和 ListeningSocketSource，但未加入 calloop、不能 accept/insert client、没有 protocol globals、GlobalDispatch/Dispatch、真实 wl_surface/xdg_toplevel、真实 render、真实 input、DRM/GBM/libinput。Render pipeline 最终是 MockRenderer 日志。

已完成：固定 4-slot/stack 模型、Action/Command/BackendEvent seam、纯数据 layout/scene/render plan、registry lifecycle、Display/socket 构造探针。未完成：nested compositor 的全部真实协议执行路径。明确缺口：Fullscreen 固定 slot 0、并不 focus-aware；默认只有 3 个 workspace 但 Super+4 指向 ID 3；core::state 直接持有 DrmBackend stub，core::event_loop 依赖 calloop；关键字段 public，边界主要靠约定；core/backend 有两套 surface lifecycle 模型。

最大风险：继续增加 probe/evidence/precondition 表面积而不做真实纵向切片；linux_handler_probe.rs 已 7,146 行，最近提交几乎全是前置条件证据。建议先处理 Git 删除和少量 core seam 决策，然后停止新增证据层，进入 Linux 真实 socket accept -> insert_client -> wl_compositor/wl_shm/xdg_wm_base globals/handlers -> 一个真实 xdg_toplevel map/unmap -> BackendEvent/CoreCommand -> core 的闭环。nested 成功前不要先做 DRM/GBM/libinput/Vulkan。
```
