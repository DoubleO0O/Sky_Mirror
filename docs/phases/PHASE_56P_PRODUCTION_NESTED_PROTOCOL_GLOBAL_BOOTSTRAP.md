# Phase 56P — Production Nested Protocol Global Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development`
> or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox
> (`- [ ]`) syntax for tracking.

**Goal:** 让 `NestedRuntimeOrchestrator::start()` 的 production nested socket 在对外可连接前，
按固定顺序初始化 `wl_compositor` 与 `xdg_wm_base`，并用真实外部 Wayland client 的有界
registry roundtrip 证明两个 globals 可发现、可 bind。

**Architecture:** 保留所有既有 probe/controlled constructor，由 `real_accept_flow.rs`
新增 production-only bootstrap seam；coordinator、loop 只做 additive forwarding，
orchestrator 的 `start()` 有意改选 production seam。bootstrap report 是独立的
`pub(crate)` 纯数据快照，绝不改写 Phase 51 readiness，也不把 client-side 观察伪装成
server 自动知道的事实。

**Tech Stack:** Rust、Smithay 0.7、wayland-server 0.31、wayland-client 0.31、
wayland-protocols 0.32、calloop（Smithay re-export 0.14）、WSL Linux。

## Global Constraints

- Phase 56P 最终 allowlist 为以下 6 个文件：
  - `src/smithay_backend/real_accept_flow.rs`
  - `src/smithay_backend/nested_runtime_coordinator.rs`
  - `src/smithay_backend/nested_runtime_loop.rs`
  - `src/smithay_backend/nested_runtime_orchestrator.rs`
  - `src/smithay_backend/mod.rs`（仅限 Source Guard Alignment 的两个既有
    `#[cfg(test)]` source guard）
  - `docs/phases/PHASE_56P_PRODUCTION_NESTED_PROTOCOL_GLOBAL_BOOTSTRAP.md`
- 禁止修改 `src/core/**`、`src/backend/**`、`src/main.rs`、
  `src/smithay_backend/wayland_display.rs`、
  `src/smithay_backend/linux_wl_surface_identity.rs`、
  `src/smithay_backend/linux_shm_buffer_import_adapter.rs`、`Cargo.toml`、`Cargo.lock`、
  `.github/workflows/**` 和任何 allowlist 外文件。
- 除上述明确批准的 test-only `mod.rs` source guard 外，若实现不能在上述 6 文件内完成，
  立即停止，不得自动扩围。
- 保持既有 public API 签名、返回类型、错误 enum 形状和 lifecycle state 语义。
- `NestedRuntimeOrchestrator::start()` 的 production protocol 可见性按本阶段目标有意改变；
  不得表述成“保持所有 public 行为不变”。
- 固定顺序是 `Display → wl_compositor → xdg_wm_base → 两者成功 → socket bind → calloop source`。
- 严格 TDD：先看到测试以正确原因失败，再写最小 production 实现。
- 不新增依赖，不改 feature 名称，不让 Smithay/Wayland 类型进入 core。
- buffer import、texture、renderer、damage、frame callback、input、core mutation 必须为 false。
- 不进入 `WlBuffer` handoff、buffer import、texture creation、renderer call、damage 或
  frame callback 后续 phase。

---

## 1. 已批准的兼容边界

本阶段保持不变的是：

- 既有 public function/method 签名；
- 既有返回类型；
- `NestedRuntimeOrchestratorError` 等 public error enum 的 variant 形状；
- `Created → Started`、`Created → Failed` 等 lifecycle state 语义；
- `NestedRealAcceptFlow::with_socket_name`、
  `NestedRuntimeCoordinator::with_socket_name`、
  `NestedRuntimeLoop::with_socket_name` 的 probe/controlled 语义；
- Phase 51 `NestedRuntimeOrchestratorReadinessReport` 的字段和值语义。

本阶段有意改变的是：

- `NestedRuntimeOrchestrator::start()` 不再选择旧 probe constructor；
- production start 成功后，socket 上的 registry 会公开 `wl_compositor` 与
  `xdg_wm_base`；
- 外部 Wayland client 可发现并 bind 这两个 globals。

旧 Phase 51 readiness report 只是历史 phase-local snapshot，不是 Phase 56P 当前
聚合真值。它继续保守地返回当时的字段值，本阶段不得修改它来“追平”新事实。

## 2. 文件职责

### `real_accept_flow.rs`

- 拥有私有 production bootstrap helper；
- 定义结构化、私有 bootstrap error；
- 定义并持有 `pub(crate)` 纯数据 `ProductionProtocolBootstrapReport`；
- 在 socket 暴露前完成两个 globals；
- 绑定 socket 后、注册 calloop source 前，把
  `socket_bound_after_bootstrap` 更新为真实值；
- protocol dispatch 后 flush client，保证真实 registry roundtrip 能在有界时间内完成；
- 测试初始化顺序、两阶段错误、重复初始化、exactly-once 语义和失败清理。

### `nested_runtime_coordinator.rs`

- 保留旧 constructor；
- 新增 production constructor forwarding；
- 通过私有 common constructor 避免复制整套 owner 初始化；
- 只读转发 bootstrap report，不触碰 admission/render/core 语义。

### `nested_runtime_loop.rs`

- 保留旧 constructor；
- 新增 production constructor forwarding；
- 只读转发 bootstrap report；
- 不改变 bounded loop、stop 或 wakeup 语义。

### `nested_runtime_orchestrator.rs`

- `start()` 改选 production constructor；
- 保持原 start report、error mapping 和 lifecycle transition；
- 添加 `pub(crate)` report getter；
- 用唯一 socket、client thread、bounded server run、channel、scoped join 与 outer process
  watchdog 做真实外部测试；
- server report 的 external 两项始终为 false；client-side 真实观察只写入 test-local
  `ExternalProtocolRoundtripEvidence`，绝不回写或合并到 server report。

### 本文档

- 记录设计、Red/Green 证据、验证矩阵、最终 capability truth 和明确 deferred scope。

## 3. 精确接口

### 3.1 独立 report

`real_accept_flow.rs` 新增以下 crate-private 纯数据类型；所有字段均为 bool，不持有
Display、socket、source、client、proxy、buffer 或 renderer：

```rust
#[must_use = "production protocol bootstrap report 必须按 server/client 观察边界解释"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ProductionProtocolBootstrapReport {
    pub(crate) bootstrap_attempted: bool,
    pub(crate) wl_compositor_initialized: bool,
    pub(crate) xdg_wm_base_initialized: bool,
    pub(crate) socket_bound_after_bootstrap: bool,
    pub(crate) external_registry_discovered_both_globals: bool,
    pub(crate) external_client_bound_both_globals: bool,
    pub(crate) buffer_import_attempted: bool,
    pub(crate) buffer_imported: bool,
    pub(crate) texture_created: bool,
    pub(crate) renderer_called: bool,
    pub(crate) damage_submitted: bool,
    pub(crate) frame_callback_done_sent: bool,
    pub(crate) input_support: bool,
    pub(crate) core_mutation_invoked: bool,
}
```

生产 constructor 只能填写 server 自己知道的四项：

- `bootstrap_attempted = true`
- `wl_compositor_initialized = true`
- `xdg_wm_base_initialized = true`
- `socket_bound_after_bootstrap = true`

external registry/client 两项在 server snapshot 中保持 false，因为 server 不能推断
client 看到了什么。真实 client 的发现和 bind 仅记录在测试私有的
`ExternalProtocolRoundtripEvidence`，绝不写回 production getter，也不构造混合 server/client
真值的 aggregate report。其余八个 deferred 字段始终显式为 false：
`buffer_import_attempted`、`buffer_imported`、`texture_created`、`renderer_called`、
`damage_submitted`、`frame_callback_done_sent`、`input_support`、`core_mutation_invoked`。

### 3.2 结构化内部错误

`real_accept_flow.rs` 新增私有错误，不扩展任何 public error enum：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionProtocolBootstrapError {
    WlCompositor {
        source: LinuxWlCompositorGlobalInitError,
    },
    XdgWmBase {
        source: LinuxXdgShellGlobalInitError,
    },
}
```

它实现稳定的 `Display` 和 `std::error::Error`。映射规则固定为：

- `initialize_wl_compositor_global()` 的错误只映射到 `WlCompositor`；
- `initialize_xdg_shell_global()` 的错误只映射到 `XdgWmBase`；
- 两个现有 source 的 `AlreadyInitialized` 保留在 variant 中，因此重复初始化可做结构化断言；
- orchestrator 外层仍映射成现有 `StartFailed { message }`，不新增 public variant。

### 3.3 flow constructor

新增：

```rust
pub(crate) fn with_production_protocol_bootstrap(
    name: &str,
) -> Result<Self, Box<dyn std::error::Error>>;

pub(crate) fn production_protocol_bootstrap_report(
    &self,
) -> Option<ProductionProtocolBootstrapReport>;
```

私有 helper 固定为：

```rust
fn bootstrap_production_protocol_globals(
    display: &mut SmithayWaylandDisplayProbe,
) -> Result<ProductionProtocolBootstrapReport, ProductionProtocolBootstrapError>;

fn with_production_protocol_display(
    name: &str,
    display: SmithayWaylandDisplayProbe,
) -> Result<NestedRealAcceptFlow, Box<dyn std::error::Error>>;
```

旧 `with_socket_name` 仍创建不含 globals 的 Display，并让 report 为 `None`。
production constructor 创建 Display 后只调用一次
`with_production_protocol_display`；后者只调用一次
`bootstrap_production_protocol_globals`。这里的 exactly-once 是控制流性质：
若 helper 被第二次调用，现有 global API 会以 `AlreadyInitialized` 拒绝，production
constructor 将不能成功；不增加任何“调用次数”字段或虚构计数器。

### 3.4 forwarding chain

三层统一新增同名 crate-private constructor/getter：

```rust
pub(crate) fn with_production_protocol_bootstrap(
    name: &str,
) -> Result<Self, Box<dyn std::error::Error>>;

pub(crate) fn production_protocol_bootstrap_report(
    &self,
) -> Option<ProductionProtocolBootstrapReport>;
```

`NestedRuntimeOrchestrator::start()` 只把：

```rust
NestedRuntimeLoop::with_socket_name(&self.config.socket_name)
```

替换为：

```rust
NestedRuntimeLoop::with_production_protocol_bootstrap(&self.config.socket_name)
```

成功/失败分支、start report 和 lifecycle transition 原样保留。

## 4. Socket 暴露与 owner 生命周期

production owner 的唯一合法顺序：

1. `SmithayWaylandDisplayProbe::new()` 创建并拥有 Display；
2. `initialize_wl_compositor_global()` 成功；
3. `initialize_xdg_shell_global()` 成功；
4. 两个 `Ok` report 均确认 global initialized；
5. 创建 insert boundary；
6. `SmithayWaylandSocketProbe::with_name()` 创建并绑定唯一 socket；
7. report 的 `socket_bound_after_bootstrap` 变为 true；
8. socket 转成 `ListeningSocketSource`；
9. 创建 calloop event loop；
10. 注册 source；
11. 全部成功后才返回持有 Display/socket source/event loop 的 flow。

任一 bootstrap 错误发生在第 6 步之前，所以没有可连接的半启动 socket，也没有
calloop source。测试用的 `with_production_protocol_display` 消费 Display；返回 Err
时局部 owner 自动 drop。socket/source 后续失败同样由 Rust drop 清理，禁止泄漏或
全局缓存这些 owner。

## 5. 有界外部 Wayland client 证明

测试常量固定为：

```rust
const PRODUCTION_PROTOCOL_TEST_TIMEOUT: Duration = Duration::from_secs(5);
const PRODUCTION_PROTOCOL_PUMP_TIMEOUT: Duration = Duration::from_millis(5);
const PRODUCTION_PROTOCOL_MAX_PUMPS: usize = 1_000;
const PRODUCTION_PROTOCOL_CLIENT_MAX_READINESS_POLLS: usize = 1_000;
const PRODUCTION_PROTOCOL_OUTER_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(6);
const PRODUCTION_PROTOCOL_WATCHDOG_KILL_REAP_TIMEOUT: Duration = Duration::from_secs(1);
const PRODUCTION_PROTOCOL_WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PRODUCTION_PROTOCOL_WATCHDOG_MAX_POLLS: usize = 1_500;
```

最终实现的测试必须：

1. outer watchdog 为每个 child 创建唯一 `XDG_RUNTIME_DIR`，child 在其中使用 basename
   `phase56p-external-protocol.sock`；无 child-role 的 helper fallback 才使用
   `unique_socket_name("production-protocol-registry-roundtrip")`；
2. `start()` 成功后从 `XDG_RUNTIME_DIR` 拼出真实 socket path；
3. 创建 client-start channel 和 client-result channel；
4. client thread 通过 `UnixStream::connect` 连接该真实 socket；
5. 对 `UnixStream` 调用 `set_nonblocking(true)`；总边界由 absolute deadline 与 poll cap 提供；
6. 用 `Connection::from_socket` 后，以手动、非阻塞 event-queue driver 驱动 registry 与
   bind 阶段；不把最终路径描述为同步 `registry_queue_init`；
7. 通过 fd readiness 驱动有限次 dispatch/flush，分别发现并 bind `WlCompositor` 与
   `XdgWmBase`；
8. 完成 registry 与 bind 的独立手动同步阶段；不把最终路径描述为同步
   `event_queue.roundtrip`；
9. client 在成功和失败两条路径都发送结构化结果并调用 stop handle wakeup；
10. server 用 `max_iterations = 1_000`、`pump_timeout = 5ms`、
    `stop_when_idle = false` 做 bounded run；
11. 所有 `recv` 使用 `recv_timeout`；
12. inner bounded protocol 通过 channel `recv_timeout`、scoped join、absolute deadline 与
    readiness poll cap 完成；
13. 外层 `current_exe` process watchdog 以有界 `try_wait` 在 6 秒 deadline 内监视 child，
    超时后发出 kill，并在 1 秒 bounded reap window 内回收；
14. `ProtocolChildTerminationGuard` 的 RAII Drop 也执行有界 kill/reap（含有限重试），
    并覆盖 silent-peer、SIGKILL、socket/runtime-directory cleanup 回归。

禁止：

- `loop {}` 没有 deadline；
- 无界 `sleep`；
- 直接调用没有 timeout 前置条件的 `join()`；
- 修改进程级 `WAYLAND_DISPLAY`；
- 调用 `display_mut_for_controlled_toplevel_registration`；
- 创建 `wl_surface`、`xdg_surface`、`xdg_toplevel`；
- buffer、render、damage、frame callback、input 或 core mutation。

## Task 1: Flow bootstrap、错误与 server report

**Files:**

- Modify: `src/smithay_backend/real_accept_flow.rs`
- Test: `src/smithay_backend/real_accept_flow.rs` 内部 `#[cfg(test)]` module

**Interfaces:**

- Consumes: `SmithayWaylandDisplayProbe::{new,initialize_wl_compositor_global,
  initialize_xdg_shell_global}` 和既有 socket/source constructor。
- Produces: `ProductionProtocolBootstrapReport`、
  `NestedRealAcceptFlow::with_production_protocol_bootstrap` 与只读 getter。

本任务必须按以下精确 contract 实现，不能省略或重命名字段：

```rust
#[must_use = "production protocol bootstrap report 必须按 server/client 观察边界解释"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ProductionProtocolBootstrapReport {
    pub(crate) bootstrap_attempted: bool,
    pub(crate) wl_compositor_initialized: bool,
    pub(crate) xdg_wm_base_initialized: bool,
    pub(crate) socket_bound_after_bootstrap: bool,
    pub(crate) external_registry_discovered_both_globals: bool,
    pub(crate) external_client_bound_both_globals: bool,
    pub(crate) buffer_import_attempted: bool,
    pub(crate) buffer_imported: bool,
    pub(crate) texture_created: bool,
    pub(crate) renderer_called: bool,
    pub(crate) damage_submitted: bool,
    pub(crate) frame_callback_done_sent: bool,
    pub(crate) input_support: bool,
    pub(crate) core_mutation_invoked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionProtocolBootstrapError {
    WlCompositor {
        source: LinuxWlCompositorGlobalInitError,
    },
    XdgWmBase {
        source: LinuxXdgShellGlobalInitError,
    },
}

fn bootstrap_production_protocol_globals(
    display: &mut SmithayWaylandDisplayProbe,
) -> Result<ProductionProtocolBootstrapReport, ProductionProtocolBootstrapError>;

fn with_production_protocol_display(
    name: &str,
    display: SmithayWaylandDisplayProbe,
) -> Result<NestedRealAcceptFlow, Box<dyn std::error::Error>>;

impl NestedRealAcceptFlow {
    pub(crate) fn with_production_protocol_bootstrap(
        name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>>;

    pub(crate) fn production_protocol_bootstrap_report(
        &self,
    ) -> Option<ProductionProtocolBootstrapReport>;
}
```

- [x] **Step 1: 写 compositor-stage Red test**

  先初始化 compositor，再调用尚不存在的私有 bootstrap helper。期望结构化
  `WlCompositor { source: AlreadyInitialized }`，并断言 xdg 仍未初始化，从而证明
  compositor 是第一阶段且失败会短路。

- [x] **Step 2: 写 xdg-stage Red test**

  先初始化 xdg，再调用 helper。期望 compositor 在 helper 内成功初始化，随后返回
  `XdgWmBase { source: AlreadyInitialized }`。这同时证明顺序不是 xdg-first。

- [x] **Step 3: 写 duplicate/exactly-once Red test**

  对 fresh Display 调用 helper 一次成功，第二次调用得到结构化 compositor duplicate
  error；production constructor 成功本身证明 constructor 没有调用 helper 两次。
  断言中不出现 invocation count。

- [x] **Step 4: 写 failure cleanup Red test**

  把预先初始化 xdg 的 Display 按值交给
  `with_production_protocol_display`，期望 Err；断言唯一 socket path 不存在；随后用
  同名 fresh production constructor 成功启动并 drop，再断言 path 不存在。该测试同时
  覆盖失败 Display owner drop、socket 未暴露和 source 未残留。

- [x] **Step 5: 运行 Red**

  Run:

  ```text
  cargo test --locked --features smithay-linux production_protocol_bootstrap -- --nocapture
  ```

  Expected: FAIL，因为 production report/helper/constructor 尚不存在；修正拼写或测试
  harness 错误，直到失败只指向缺失的 Phase 56P contract。

- [x] **Step 6: 写最小 flow 实现**

  实现第 3 节的 report/error/helper/constructor/getter；旧 constructor 走 common
  assembly 且 report 为 `None`。`dispatch_wayland_clients_once` 在成功 dispatch 后
  调用现有 `flush_clients_once`，不改变返回类型。

  bootstrap helper 的完整赋值规则固定为：

  ```rust
  let compositor = display
      .initialize_wl_compositor_global()
      .map_err(|source| ProductionProtocolBootstrapError::WlCompositor { source })?;
  let xdg = display
      .initialize_xdg_shell_global()
      .map_err(|source| ProductionProtocolBootstrapError::XdgWmBase { source })?;

  Ok(ProductionProtocolBootstrapReport {
      bootstrap_attempted: true,
      wl_compositor_initialized: compositor.wl_compositor_global_initialized,
      xdg_wm_base_initialized: xdg.xdg_shell_global_initialized,
      socket_bound_after_bootstrap: false,
      external_registry_discovered_both_globals: false,
      external_client_bound_both_globals: false,
      buffer_import_attempted: false,
      buffer_imported: false,
      texture_created: false,
      renderer_called: false,
      damage_submitted: false,
      frame_callback_done_sent: false,
      input_support: false,
      core_mutation_invoked: false,
  })
  ```

  socket 创建成功后、`into_source` 和 `insert_source` 之前，只对 `Some(report)` 执行：

  ```rust
  report.socket_bound_after_bootstrap = true;
  ```

  dispatch/flush 固定为：

  ```rust
  let dispatched = self.display.dispatch_clients_once()?;
  self.display.flush_clients_once()?;
  Ok(dispatched)
  ```

- [x] **Step 7: 运行目标 Green**

  Run:

  ```text
  cargo test --locked --features smithay-linux production_protocol_bootstrap -- --nocapture
  ```

  Expected: flow 级新测试全部 PASS；无新 warning。

- [x] **Step 8: 提交 Task 1**

  ```text
  git add src/smithay_backend/real_accept_flow.rs
  git commit -m "feat: add phase 56p production protocol bootstrap flow"
  ```

## Task 2: Production constructor chain 与真实外部 roundtrip

**Files:**

- Modify: `src/smithay_backend/nested_runtime_coordinator.rs`
- Modify: `src/smithay_backend/nested_runtime_loop.rs`
- Modify: `src/smithay_backend/nested_runtime_orchestrator.rs`
- Test: `src/smithay_backend/nested_runtime_orchestrator.rs` 内部 `#[cfg(test)]` module

**Interfaces:**

- Consumes: Task 1 的 production flow constructor/report getter。
- Produces: coordinator/loop/orchestrator forwarding getter，production `start()`，
  以及与 server getter 严格分离的 test-local external-client evidence。

三层必须使用以下同名 crate-private contract：

```rust
pub(crate) fn with_production_protocol_bootstrap(
    name: &str,
) -> Result<Self, Box<dyn std::error::Error>>;

pub(crate) fn production_protocol_bootstrap_report(
    &self,
) -> Option<ProductionProtocolBootstrapReport>;
```

orchestrator 的唯一行为替换是：

```rust
match NestedRuntimeLoop::with_production_protocol_bootstrap(&self.config.socket_name) {
    Ok(runtime_loop) => {
        let socket_name = runtime_loop.socket_name().to_owned();
        self.runtime_loop = Some(runtime_loop);
        self.state = NestedRuntimeLifecycleState::Started;
        Ok(NestedRuntimeStartReport {
            previous_state,
            state: self.state,
            started: true,
            socket_name,
        })
    }
    Err(error) => {
        self.state = NestedRuntimeLifecycleState::Failed;
        Err(NestedRuntimeOrchestratorError::StartFailed {
            message: error.to_string(),
        })
    }
}
```

外部测试使用固定边界：

```rust
const PRODUCTION_PROTOCOL_TEST_TIMEOUT: Duration = Duration::from_secs(5);
const PRODUCTION_PROTOCOL_PUMP_TIMEOUT: Duration = Duration::from_millis(5);
const PRODUCTION_PROTOCOL_MAX_PUMPS: usize = 1_000;
const PRODUCTION_PROTOCOL_CLIENT_MAX_READINESS_POLLS: usize = 1_000;
const PRODUCTION_PROTOCOL_OUTER_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(6);
const PRODUCTION_PROTOCOL_WATCHDOG_KILL_REAP_TIMEOUT: Duration = Duration::from_secs(1);
const PRODUCTION_PROTOCOL_WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PRODUCTION_PROTOCOL_WATCHDOG_MAX_POLLS: usize = 1_500;
```

- [x] **Step 1: 先写外部 client Red test**

  第一版测试只使用既有 orchestrator start/run/stop API 和 test-local Wayland client，
  不先引用尚不存在的 report getter。运行当前 production start，期望 client 返回
  registry 缺少 `wl_compositor` 或 `xdg_wm_base` 的结构化失败；超时、panic、连接失败
  都不是正确 Red，必须先修正 bounded harness。

- [x] **Step 2: 运行并确认正确 Red**

  Run:

  ```text
  cargo test --locked --features smithay-linux \
    production_orchestrator_external_registry_roundtrip -- --nocapture
  ```

  Expected: FAIL，client 已连接真实唯一 socket，但 registry bind 明确报告缺失 target
  global；inner protocol/channel 路径有界退出，outer process watchdog、socket cleanup 均受
  固定 deadline 和 poll cap 约束。

- [x] **Step 3: 补齐 report/compatibility Red assertions**

  在同一 test module 增加 production report getter、server snapshot 与 test-local external
  client evidence 的分离断言；同时保留既有 Phase 51 readiness false 断言，证明没有改写
  历史 snapshot。

- [x] **Step 4: 写最小 forwarding 实现**

  coordinator 复用现有私有 with_flow 完成 flow 与其 owner 的组装；production
  forwarding 仅负责创建已完成 bootstrap 的 flow 并交给该 helper。loop 的 legacy
  with_socket_name 与 production with_production_protocol_bootstrap constructors
  分别创建对应 coordinator，并各自组装相同的 stop_handle/coordinator owner
  生命周期边界。上述私有 helper 与组装形状属于实现细节，不构成 public contract。
  orchestrator start() 仅切换到 production constructor，并新增 crate-private
  report getter；不新增 capability，也不改变已验证的 bootstrap 顺序或兼容性结论。

- [x] **Step 5: 运行目标 Green**

  Run:

  ```text
  cargo test --locked --features smithay-linux \
    production_orchestrator_external_registry_roundtrip -- --nocapture
  cargo test --locked --features smithay-linux \
    runtime_orchestrator_start_transitions_state -- --nocapture
  cargo test --locked --features smithay-linux \
    real_accepted_client_connected_event_registers_core_client -- --nocapture
  ```

  Expected: external registry/bind PASS；既有 lifecycle 与旧 flow constructor tests PASS。

- [x] **Step 6: 验证 server/client capability truth 边界**

  server `ProductionProtocolBootstrapReport` 仅前四项为 true，external discovery/bind
  仍为 false；测试私有 `ExternalProtocolRoundtripEvidence` 单独证明 discovery/bind 为 true。
  buffer import、texture、renderer、damage、frame callback、input、core mutation 全部 false。

- [x] **Step 7: 提交 Task 2**

  实际提交链及职责：

  - `7db9239`：production constructor chain。
  - `01f2db1`：有界、非阻塞 client registry/bind roundtrip。
  - `1c28786`：outer process watchdog。
  - `f3d68af`：silent-peer 与 RAII termination guard。

## Task 3: 文档、完整矩阵与 allowlist 审计

**Files:**

- Modify: `docs/phases/PHASE_56P_PRODUCTION_NESTED_PROTOCOL_GLOBAL_BOOTSTRAP.md`

**Interfaces:**

- Consumes: Task 1/2 的测试输出、最终 diff 和 capability report。
- Produces: phase 完成记录、验证表、PR body 事实来源。

- [x] **Step 1: 更新本文档执行证据**

  在“执行记录”填写实际 Red 原因、Green test counts、矩阵结果、commit 和 CI URL；
  不把未运行命令写成 PASS。

- [x] **Step 2: 运行完整矩阵**

  在 WSL worktree 中设置独立 `CARGO_TARGET_DIR`，依次执行：

  ```text
  git diff --check
  cargo fmt --check
  cargo check --locked
  cargo test --locked
  cargo check --locked --features smithay-probe
  cargo test --locked --features smithay-probe
  cargo check --locked --features smithay-linux
  cargo test --locked --features smithay-linux
  cargo clippy --locked --all-targets --features smithay-linux
  ```

- [x] **Step 3: 审计文件范围**

  Run:

  ```text
  git diff --name-only main...HEAD
  git status --short
  ```

  Expected: 只出现 Global Constraints 的 6 个文件；`Cargo.toml`、`Cargo.lock` 和禁止
  路径无 diff。

- [ ] **Step 4: whole-branch spec review 与 quality review（PENDING）**

  reviewer 必须检查固定顺序、owner drop、错误映射、hard timeout、旧 readiness 语义、
  public API/error enum 兼容、false capability 和中文注释。Critical/Important finding
  修复后重新跑覆盖测试并 re-review。

- [x] **Step 5: documentation commit**

  本提交只纳入本文档；此前代码提交不 amend、不重写。

- [ ] **Step 6: push、PR、CI、ready、merge 与 main CI（PENDING）**

  push `codex/phase56p-production-nested-protocol-global-bootstrap`，创建 draft PR。PR body
  必须包含 Goal、Current boundary、Implemented、Explicitly deferred、Capability truth、
  Files changed、Tests、Public API compatibility、Feature gate、Risks、Follow-up。等待全部
  GitHub Actions checks 结束；CI red 时先读取失败 job/step/log 并报告根因，不盲目重跑。

## 验收标准

- production `start()` 不调用 test-only mutable display accessor；
- 两个 globals 在 socket bind 前完成；
- compositor、xdg、duplicate 三类错误均是结构化内部错误；
- bootstrap 失败无 Display/socket/source 残留，同名干净启动可成功；
- production constructor 没有 invocation counter，且成功路径只调用一次 bootstrap；
- 外部 client 真实发现并 bind 两个 globals；
- 所有 wait、server run 和 join 有硬 timeout；
- 失败路径清理唯一 socket；
- Phase 51 readiness 仍是历史 snapshot，未被改写；
- public API 签名、返回类型、错误 enum 形状、lifecycle state 语义不变；
- buffer/import/texture/renderer/damage/frame/input/core mutation 全部 false；
- 完整矩阵通过，allowlist 外无 diff；
- phase 在 globals bootstrap 结束，不跨入任何 buffer/render 后续能力。

## 执行记录

状态：Task 1/2 已完成并有审计证据；Task 3 的最终矩阵、范围审计、hash 审计、资源审计和
本文档提交已完成。下列尚未执行的 release 门禁必须保持 **PENDING**，不得写为 PASS：
whole-branch spec review、whole-branch quality review、push、PR、CI、ready、merge、main CI。

### 已完成提交

Task 1：

- `f0f97a4059470ce69bc14992db3ebb3632637cdc` — `feat: add phase 56p production protocol bootstrap flow`
- `00d6519625f9a94fbebfc3f86147f7c7c0c4fc9e` — `fix: release socket source on insertion failure`
- `dccffb15bdab65516e58581ad1f3b2fd562b09f2` — `fix: preserve insert boundary assembly order`

Task 1 的 socket-source ownership review 发现 `insert_source` 失败时 `InsertError` 可继续
持有 source；第二个提交在返回错误前显式释放 source。后续修复恢复 common assembly 的
insert-boundary-before-socket 顺序。最终 bootstrap target `5/5`、flow module `13/13` 通过；
Task 1 规格 review 的 Critical/Important/Minor 均为 0。

Task 2：

- `7db9239136602482dea2a379ae739c93e9788352` — `feat: route production protocol bootstrap through nested runtime`
- `01f2db1c6e471363776fe89c347bc751cfbba79a` — `test: bound production protocol client roundtrips`
- `1c28786e417c827ba6ede33b1054ad8d48182939` — `test: watchdog nested protocol client scenario`
- `f3d68afe714dd85ce99dfec185a4263be14f1770` — `test: guard silent protocol client watchdog`

### Task 2 TDD、Green 与 watchdog 证据

首次正式执行在正确 Red 之前曾发生测试夹具编译错误；该错误已披露且没有被伪造为 Red。修正
fixture 和有界 harness 后，正式 Red 为真实 client 已连接唯一 socket、但 registry 明确缺少
`wl_compositor` 或 `xdg_wm_base`，命令以 exit 1 在 0.05 秒结束。随后 production constructor
链路使外部 registry/bind Green：outer `1/1`、child `1/1`，bounded server pump 为 `3/1000`，
registry/bind readiness polls 各为 `1`。

不得将历史改写为‘第一次运行就是正确 Red’。

最终 client 使用手动非阻塞 event-queue driver 与 fd readiness；所有操作受 absolute deadline
与 poll caps 约束。client thread 采用 scoped join，outer `current_exe` watchdog 以有界
`try_wait` 轮询并在 deadline 后 kill/reap。最终 watchdog 记录正常 child `child_polls=14`、
`timed_out=false`，并覆盖 SIGKILL、RAII Drop、socket 与 runtime-directory 精确清理。

最终重新验证：`production_protocol_` `13/13`、coordinator `13/13`、loop `22/22`、
orchestrator `29/29`；`cargo fmt --all -- --check`、
`cargo clippy --locked --features smithay-linux --tests --quiet` 以及
`git diff --check dccffb1..HEAD` 均为 exit 0。独立规格 review 为 Critical/Important/Minor
`0/0/0`；代码质量 review 为 `0/0/2`。以下两项为 Minor、非阻塞、本阶段不修复的 test debt：

1. `set_permissions` 失败可能留下空 test runtime directory。
2. watchdog 测试的启动、cleanup 与断言 scaffolding 有重复。

用户批准的历史流程偏差豁免，不影响当前代码技术验收。

### Server truth、external evidence 与 deferred scope

production server getter 只报告：`bootstrap_attempted=true`、
`wl_compositor_initialized=true`、`xdg_wm_base_initialized=true`、
`socket_bound_after_bootstrap=true`；它的
`external_registry_discovered_both_globals=false` 与
`external_client_bound_both_globals=false`。真实 client 的 discovery/bind 仅由测试私有
`ExternalProtocolRoundtripEvidence` 证明为 true，绝不回填 server report。Phase 51 readiness
仍是历史 phase-local snapshot。

以下 deferred capabilities 全部仍为 false：`buffer_import_attempted`、`buffer_imported`、
`texture_created`、`renderer_called`、`damage_submitted`、`frame_callback_done_sent`、
`input_support`、`core_mutation_invoked`。本阶段没有 buffer handoff、texture、renderer、damage、
frame callback、input 或 core mutation。

production `start()` 有意公开两个 globals；public signatures、return types、error enum shapes
与 lifecycle meanings 均保持兼容。

### Task 3 and release status

#### 最终矩阵与范围审计

九项最终矩阵均已 PASS。权威聚合记录为
`D:\\Sky_mirror_setup\\logs\\phase56p-task3-final-matrix-729aaa9\\matrix-summary-v4-corrected.json`，
SHA-256：`4CFB8AEC497089BD22597E724C771C86EB69A03755B7325C376C43DD01E7CE17`。

最终范围审计、五文件 hash、四个 production 文件 hash 和资源审计均为 PASS。
仓库内 `target` 已移动（非删除）至
`D:\\Sky_mirror_setup\\quarantine\\phase56p-task3\\repo-target-20260728-163210`。

`729aaa97c6e2273b21517558e509b0bdbdf84505` 的 Source Guard Alignment 仅修改
`src/smithay_backend/mod.rs` 内两个 test-only source guard。最终 allowlist 为 6 个文件：
`real_accept_flow.rs`、`nested_runtime_coordinator.rs`、`nested_runtime_loop.rs`、
`nested_runtime_orchestrator.rs`、test-only `mod.rs`，以及本阶段文档。

审计中保留的 harness 历史包括：非登录 WSL 未加载 Rust PATH、WSL Git 不能解析 Windows
linked worktree、AWK/read 元数据解析、长 Unix socket 路径，以及旧汇总聚合器失败；
这些均不是代码或测试失败。最终记录使用正确 Rust 环境、Windows Git、短运行时路径和
corrected summary；不存在无边界的同步协议驱动。

- Task 3 final matrix：**PASS**。
- Task 3 scope/hash/resource audit：**PASS**。
- documentation commit：本提交完成。
- whole-branch spec review：**PENDING**。
- whole-branch quality review：**PENDING**。
- push：**PENDING**。
- PR：**PENDING**。
- CI：**PENDING**。
- ready：**PENDING**。
- merge：**PENDING**。
- main CI：**PENDING**。
