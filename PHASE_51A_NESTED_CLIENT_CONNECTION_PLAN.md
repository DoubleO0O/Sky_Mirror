# Phase 51A：Nested Wayland Client Connection Minimum Slice Plan

> 文档时间：2026-06-19，Asia/Shanghai
>
> 审计基线：`843674a925ecdade15f11170f0ac7355d92918da`
>
> 性质：设计与实施计划；本文件不表示真实 client accept 已实现。

## 1. 当前状态

Sky Mirror 当前已经有一条完整的纯数据 core client lifecycle seam，但 Linux Smithay 侧仍是明确的 probe / skeleton：

```text
纯数据测试事件
    -> BackendEvent
    -> BackendEventTranslator
    -> CoreCommand
    -> State
    -> ClientRegistry
    -> ValidationReport
```

当前基线具备以下事实：

- `BackendEvent::ClientConnected` / `ClientDisconnected` 已存在；
- `CoreCommand::RegisterClient` / `CloseClient` 已存在；
- `State` 是协调 `ClientRegistry` 以及 client 关闭级联的入口；
- `CoreRuntimeBridge` 已通过 `State::handle_command_with_validation` 返回命令后的 `ValidationReport`；
- `wayland_display.rs` 可以构造真实 `Display<SmithayWaylandState>` 并取得 `DisplayHandle`；
- `wayland_socket.rs` 可以构造并真实绑定 `ListeningSocketSource`，但没有把它加入 event loop；
- `linux_adapter.rs` 明确报告 `accepts_clients = false`，其 client-session ledger 只记录 unsupported observation，不来自 socket accept；
- `src/smithay_backend/client_session.rs` 当前不存在；
- 当前能力仍是 probe / skeleton，不是可用 compositor runtime。

官方 Smithay 0.7 文档确认：`ListeningSocketSource` 是可加入 calloop 的 `EventSource`，callback 提供 client `UnixStream`，随后应调用 `DisplayHandle::insert_client`。`Display` 的协议请求处理则依赖 `dispatch_clients` 与对应 `Dispatch` 实现。因此“接受连接”可以先于 wl_surface、xdg-shell 和 renderer 独立切出，但不能被描述成完整协议支持：

- [Smithay 0.7 ListeningSocketSource](https://smithay.github.io/smithay/smithay/wayland/socket/struct.ListeningSocketSource.html)
- [Smithay socket module example](https://smithay.github.io/smithay/smithay/wayland/socket/index.html)
- [wayland_server DisplayHandle](https://smithay.github.io/wayland-rs/wayland_server/struct.DisplayHandle.html)
- [wayland_server ClientData](https://docs.rs/wayland-server/latest/wayland_server/backend/trait.ClientData.html)

## 2. 已有能力

### 2.1 Core client lifecycle

已有能力，不需要重复新增 enum variant：

```rust
BackendEvent::ClientConnected {
    client: Option<ClientId>,
    kind: ClientKind,
    name: Option<String>,
}

BackendEvent::ClientDisconnected {
    client: ClientId,
}

CoreCommand::RegisterClient {
    client: Option<ClientId>,
    kind: ClientKind,
    name: Option<String>,
}

CoreCommand::CloseClient(ClientId)
```

现有翻译关系已经正确表达两层语义：

- backend event 表示外部连接或断开事实；
- core command 表示对核心状态的变更请求；
- `BackendEventTranslator` 是纯函数，不读写 `State`；
- `CommandHandler` 只通过 `State::register_client*` / `State::close_client` 修改状态；
- `State::close_client` 会级联收束属于该 client 的 surface、window、workspace 与 focus 生命周期。

### 2.2 ClientRegistry

已有能力：

- 自动分配：`ClientRegistry::register_client`；
- 外部 ID：`ClientRegistry::register_client_with_id`；
- tombstone：`mark_dead` 保留记录并设置 `alive = false`；
- 查询：`get`、`records`、`is_alive`、`next_id`；
- 显式外部 ID 会推进 `next_id`，避免后续自动分配直接冲突；
- 重复显式 ID 会被拒绝，不覆盖原记录。

当前 `ClientId` 是 `u64` type alias，不是 newtype。`SurfaceId` 也是 `u64` type alias；语义隔离目前依靠字段、variant 与 seam，而不是编译期 newtype 隔离。Phase 51B–51E 不顺带迁移 ID 类型，以免扩大公共接口改动。

### 2.3 Surface / client validation

已有能力：

- alive client 没有 surface 是合法中间状态；
- alive surface 指向缺失 client 会产生 `SurfaceReferencesMissingClient`；
- alive surface 指向 dead client 会产生 `SurfaceReferencesDeadClient`；
- dead surface 指向 dead client 可以作为 tombstone 历史保留；
- client 正常断开级联后，validator 能确认状态重新 clean；
- client registry 的 `next_id` 不大于已知最大 ID 时会产生 warning。

### 2.4 Runtime diagnostics

已有能力：

- `CoreRuntimeBridge::handle_backend_event` 返回 `RuntimeEventResult`；
- `RuntimeEventResult` 包含原始 event、翻译后的 command、command result 和 post-command `ValidationReport`；
- `BackendEventReplayer` 使用相同的 `State::handle_command_with_validation` seam；
- `DebugBundle` 组合 `Inspector` snapshot 与 `ValidationReport`；
- 现有 inspector 测试覆盖 client records，因此 client registry 变化可以进入 debug snapshot；Phase 51B 应补一条从 client connection public path 到 `DebugBundle` 的封板断言。

## 3. 缺口

| 能力 | 当前判断 | 缺口 |
|---|---|---|
| Wayland `Display` 构造 | 已有 | 仍未用于真实 client runtime |
| `DisplayHandle` 获取 | 已有 | 尚未用于 `insert_client` |
| Wayland socket 创建/绑定 | 已有 | socket 未加入 event loop |
| `ListeningSocketSource` | 已有真实对象 | 没有 callback / accept path |
| client accept | 缺口 | 没有接收 `UnixStream`，没有 `insert_client` |
| backend client session identity | 缺口 | 现有 skeleton observation ID 不是真实 accepted client 映射 |
| backend session -> core `ClientId` | 缺口 | 没有映射表和 exactly-once 规则 |
| connected event -> core runtime | core 路径已有，adapter 接线缺失 | 没有真实 callback 产出的 event |
| disconnected callback -> core runtime | core 路径已有，adapter 接线缺失 | 没有把 `ClientData::disconnected` 变成纯数据事件 |
| protocol dispatch / globals | 明确不支持 | 不属于本最小切片 |
| wl_surface / xdg-shell / renderer | 明确不支持 | 不属于 Phase 51B–51E |

`linux_adapter.rs` 中已有的 client-session ledger 不能当作 accept 实现：源码明确说明 observation 不来自 socket accept，固定 outcome 是 `RejectedUnsupported`，并且 capabilities 固定 `accepts_clients = false`。

## 4. 最小切片边界

推荐把真实连接路径拆成四个可独立验收的小步：

```text
Phase 51B  封板现有 core client lifecycle
    ↓
Phase 51C  建立 Smithay-free adapter session event / identity seam
    ↓
Phase 51D  Linux-only socket accept probe，产生 adapter session event
    ↓
Phase 51E  session event -> BackendEvent -> RuntimeBridge -> ClientRegistry
```

最小成功链路：

```text
ListeningSocketSource callback receives UnixStream
    -> DisplayHandle::insert_client
    -> NestedClientSessionEvent::Connected(adapter_session_id)
    -> adapter/core coordinator
    -> BackendEvent::ClientConnected { client: None, ... }
    -> CoreRuntimeBridge
    -> CoreCommand::RegisterClient { client: None, ... }
    -> State / ClientRegistry allocates ClientId
    -> coordinator stores adapter_session_id -> ClientId
    -> RuntimeEventResult.validation is clean
```

本切片的“真实”只表示真实 socket 被绑定、真实 client stream 被接受并插入 Wayland display。它不表示：

- client 已获得任何 protocol global；
- compositor 已 dispatch 任意 wl_surface / xdg-shell request；
- client 能创建窗口或提交 buffer；
- renderer、frame callback、input 或 Linux session 已完成。

### 4.1 三种 ClientId 方案

#### 方案 A：core 分配 ClientId，adapter 保存映射（推荐）

- connected event 使用现有 `client: None`；
- `ClientRegistry` 自动生成 core `ClientId`；
- coordinator 从 `CommandResult::ClientRegistered` 取得 ID；
- adapter 保存 `adapter_session_id -> ClientId`；
- disconnect 时通过映射生成 `BackendEvent::ClientDisconnected`。

优点：复用现有接口，不与 core allocator 竞争，不泄漏 Smithay identity。代价：连接结果返回后必须可靠写入映射，并处理“刚连接就断开”的事件顺序。

#### 方案 B：adapter 分配 u64，再走显式 ID 注册

现有 `client: Some(id)` 和 `register_client_with_id` 已能避免简单冲突，但 adapter 与 core 必须共享 ID 分配规则；重启、恢复和并发连接会增加契约面。当前没有必要采用。

#### 方案 C：直接把 Smithay / wayland-backend ClientId 当 core ClientId

拒绝。Smithay 的 client identity 是后端对象，应保持 opaque；直接镜像会把外部库类型或其生命周期约束泄漏到 core，也无法保证稳定数值转换。

## 5. ClientId / BackendEvent / CoreCommand 设计

### 5.1 不新增重复的 core variants

Phase 51B–51E 应复用现有 variants，不再新增同义的 `ClientAccepted`、`AddClient` 或 `RemoveClient`。

语义固定如下：

```text
BackendEvent::ClientConnected       外部连接事实
CoreCommand::RegisterClient         核心状态变更请求
State::register_client              唯一 registry mutation seam

BackendEvent::ClientDisconnected    外部断开事实
CoreCommand::CloseClient            核心状态变更请求
State::close_client                 唯一 lifecycle cascade seam
```

### 5.2 新增 adapter session identity，而不是复用 core ID

建议 Phase 51C 在 Smithay-free 模块中定义一个仅代表 backend connection instance 的 newtype：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NestedClientSessionId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedClientSessionEvent {
    Connected { session: NestedClientSessionId },
    Disconnected { session: NestedClientSessionId },
}
```

该类型不是 core `ClientId`，也不是 `wayland_backend::ClientId`。它由 adapter 自己分配，用来关联 connection callback、`ClientData` 和 core mapping。

### 5.3 映射规则

- 一次成功 `insert_client` 最多产生一个 `Connected(session)`；
- coordinator 只有在 core 返回 `ClientRegistered { registered: true, client }` 后才能写入映射；
- 重复 connected session 返回结构化错误，不重复注册 core client；
- unknown disconnected session 返回结构化错误或 ignored-with-diagnostic，不得伪造 `ClientId`；
- disconnect 成功提交 core 后移除 active mapping；
- tombstone 由 core registries 保存，adapter mapping 只表示当前活跃连接；
- 所有事件按 adapter observation sequence 处理，避免 disconnect 越过 connect。

不需要新增 `ClientRegistry::register_external` / `register_generated`：现有 `register_client` 与 `register_client_with_id` 已分别覆盖两种语义。若未来命名歧义持续存在，可单独做兼容性重命名计划，但不属于最小切片。

## 6. Smithay 类型隔离

硬约束：

```text
UnixStream
ListeningSocketSource
wayland_server::Client
wayland_backend::ClientId
ClientData
DisconnectReason
Display / DisplayHandle
Smithay protocol helper state
```

以上类型只能存在于 `src/smithay_backend/**` 的 Linux adapter/runtime 实现内，不得进入：

```text
src/core/client.rs
src/core/backend_event.rs
src/core/command.rs
src/core/state.rs
src/core/runtime_bridge.rs
src/core/validator.rs
```

推荐 seam：

```text
Smithay objects
    -> Linux accept adapter
    -> NestedClientSessionEvent (pure data)
    -> coordinator mapping
    -> BackendEvent (pure core input fact)
```

`ClientData::disconnected` 不应直接借用或修改 core `State`。它只把 `NestedClientSessionEvent::Disconnected` 写入一个受控队列/channel，由 runtime coordinator 在正常事件顺序中调用 `CoreRuntimeBridge`。

`linux_adapter.rs` 当前有生产代码测试禁止 `crate::core`、`BackendEvent`、`DisplayHandle`、`calloop` 和 `accept(`。Phase 51D 不应破坏该 skeleton 的封板语义；真实 accept 应放在新的 Linux-only module，或先经单独审批调整这一封板测试。推荐新增模块，保留 skeleton 作为保守 capability report。

## 7. Validation integration

连接路径必须使用：

```rust
CoreRuntimeBridge::handle_backend_event(state, event)
```

该入口已经调用：

```rust
State::handle_command_with_validation(command)
```

因此每次 connected / disconnected 都能立即得到 post-command `ValidationReport`，不需要 adapter 再调用一次 `State::validate()`。

连接验收：

- result 是 `CommandResult::ClientRegistered { registered: true, client }`；
- `state.clients.is_alive(client)` 为 true；
- `RuntimeEventResult.validation.is_clean()` 为 true；
- `state.debug_bundle()` 的 snapshot 能看到新 client；
- client 没有 surface 时仍然 clean。

断开验收：

- result 是 `CommandResult::ClientClosed`；
- client tombstone 保留且 `alive = false`；
- owned surface / window 由 `State::close_client` 级联；
- `RuntimeEventResult.validation.is_clean()` 为 true。

非法关系验收：

- surface 指向不存在 client：`SurfaceReferencesMissingClient`；
- alive surface 指向 dead client：`SurfaceReferencesDeadClient`；
- validation 只报告，不自动修复、拒绝或回滚状态。

## 8. 后续 Phase 51B–51E 规划

### Phase 51B：Core client lifecycle command sealing

**目标**

用 public seam 的 TDD 测试封板现有 connected/disconnected 能力，确认无需新增 core enum variant 或生产实现。

**允许修改文件**

```text
src/core/backend_event.rs
src/core/command.rs
src/core/runtime_bridge.rs
src/core/backend_replay.rs
```

只有 failing test 证明现有 public seam 有缺口时，才申请最小修改 `src/core/state.rs`、`src/core/client.rs`、`src/core/validator.rs`；不得预先修改。

**禁止修改文件**

```text
src/smithay_backend/**
src/backend/**
Cargo.toml
Cargo.lock
.github/workflows/**
RECOVERY_NOTES.md
SKY_PROJECT_AUDIT.md
```

**实现步骤**

1. 先运行现有 client lifecycle 测试，记录哪些行为已经由 public seam 覆盖。
2. 添加 `ClientConnected { client: None }` characterization test，断言返回 generated ID、client alive、validation clean、debug bundle 包含 client。若测试直接通过，说明这是已有能力封板，不伪造 RED，也不修改生产代码。
3. 添加 `ClientDisconnected` 无 surface characterization test，断言 tombstone 与 clean validation；已有行为直接通过时保持 tests-only。
4. 对尚未存在的 observable contract（例如 coordinator 需要的 result shape）才执行严格 RED -> GREEN：先确认因缺失行为失败，再写最小生产改动。
5. 添加 connect -> surface owner -> disconnect 的单条 tracer-bullet 回放，断言级联与每步 validation。
6. REFACTOR：只在 GREEN 后消除测试 setup 重复，不新增行为。

**测试要求**

- 新增或改变的生产行为严格执行 RED -> GREEN；已有行为的 tests-only 封板明确标记为 characterization test；
- 测试只走 `CoreRuntimeBridge` / `BackendEventReplayer` public seam；
- 不直接测试 private helper；
- 运行完整 Mac 安全矩阵。

**Git 检查要求**

- 开始：`git status --short`、`git rev-parse HEAD`、`git diff --cached --name-only`；
- 完成：`git status --short`、`git diff --stat`、限定文件 diff、`git diff --check`；
- stage 前确认只有允许文件；不 push、不 merge。

**Feature gate 约束**

- core tests 在 default features 下运行；
- 同一测试还必须在 `smithay-probe` 矩阵保持通过；
- 不启用 `smithay-linux` 于 Mac。

**Public API 兼容要求**

- 保留现有 `BackendEvent` / `CoreCommand` variants 与字段；
- 保留 `State::handle_command` 和 `handle_command_with_validation`；
- 不迁移 `ClientId` type alias。

**验收标准**

- 连接、断开、级联三条 public path 有稳定测试；
- generated `ClientId` 能从 `CommandResult` 被 coordinator 消费；
- 所有 post-command validation clean；
- 无 Smithay 类型进入 core。

**风险**

- 当前很多行为已经存在，错误地追求“新增代码”会制造重复接口；本 Phase 可以是 tests-only seal。

### Phase 51C：Adapter client session boundary

**目标**

建立 Smithay-free 的 adapter session identity、事件与映射接口，为真实 socket callback 和 core `ClientId` 解耦。

**允许修改文件**

```text
src/smithay_backend/client_session.rs        # 新增
src/smithay_backend/mod.rs
src/smithay_backend/runtime_facade.rs        # 仅在需要暴露保守诊断时
```

开始实现前额外只读复核现有 `client_event.rs` 与 `client_id.rs`；若已有等价深模块，复用而不是新增重复模块。

**禁止修改文件**

```text
src/core/**
src/backend/**
src/smithay_backend/linux_adapter.rs
Cargo.toml
Cargo.lock
.github/workflows/**
```

**实现步骤**

1. RED：测试 `NestedClientSessionId` 与 `Connected` / `Disconnected` 事件保持稳定、可比较且不包含 Smithay/core对象。
2. GREEN：实现最小 pure-data newtype 与 event enum。
3. RED：测试 session map 在首次 bind 后可查询 `ClientId`，重复 bind 被拒绝，unknown disconnect 不伪造 ID。
4. GREEN：实现小接口映射；内部集合选择以稳定、可测试为先，不暴露容器类型。
5. RED/GREEN：测试 disconnect 取出并移除 active mapping，重复 disconnect 返回结构化缺失结果。
6. 增加 production-source boundary test，禁止 `smithay::`、`wayland_server`、`DisplayHandle`、`Client`、`UnixStream`。

**测试要求**

- 一次只写一个 observable behavior 的 test；
- 不 mock core internals；
- map 测试只通过公开接口；
- default core 与 `smithay-probe` 全矩阵通过。

**Git 检查要求**

- 开始和完成使用 Phase 51B 同样的 status / HEAD / cached / diff-check 门禁；
- 新文件未 stage 时使用普通 diff 无法显示内容，检查时使用 `git diff --no-index /dev/null <file>` 或在用户批准 stage 后再看 cached diff；
- 不把其他 probe 文件顺手 stage。

**Feature gate 约束**

- `client_session.rs` 应在 `smithay-probe` 下编译，以便 Mac 测试纯数据 seam；
- 不依赖 Smithay crate，不使用 Linux-only API。

**Public API 兼容要求**

- 不改变现有 `SmithayLinuxAdapterClientSessionId` / ledger 行为；
- 不改变 `BackendEvent` / `CoreCommand`；
- 新接口保持最小，mapping 容器私有。

**验收标准**

- session identity 与 core `ClientId` 明确为两种类型/语义；
- exactly-once bind、lookup、remove 行为可测试；
- 模块在 Mac 的 `smithay-probe` 下通过；
- source boundary test 证明无 Smithay 类型。

**风险**

- `client_event.rs` / `client_id.rs` 可能已经存在可复用能力；实现前必须复核，避免平行抽象。
- 新 session ID 与现有 skeleton observation ID 容易混淆；文档与命名必须区分“accepted connection identity”和“unsupported ledger observation”。

### Phase 51D：Nested socket accept probe

**目标**

在 Linux-only 模块中把 `ListeningSocketSource` 加入最小 event loop，接受一个真实 Unix client stream，调用 `DisplayHandle::insert_client`，并只产生 `NestedClientSessionEvent::Connected`。

**允许修改文件**

```text
src/smithay_backend/nested_client_accept.rs  # 新增，推荐
src/smithay_backend/mod.rs
src/smithay_backend/wayland_display.rs       # 仅新增最小内部资源访问 seam
src/smithay_backend/wayland_socket.rs        # 仅新增安全的 ownership transfer seam
src/smithay_backend/runtime_facade.rs        # 仅更新 truthful capability/diagnostic
```

如果现有 `bootstrap.rs` 已经是唯一资源所有者，应在实现前申请把该文件加入允许范围，并在 bootstrap 内建立 deep module；不得复制绑定第二套 Display/socket。

**禁止修改文件**

```text
src/core/**
src/backend/**
src/smithay_backend/linux_adapter.rs
Cargo.toml
Cargo.lock
.github/workflows/**
```

**实现步骤**

1. RED（Linux）：创建唯一 socket 名，连接真实 `UnixStream`，pump 一次后期望收到一个 connected adapter event；现状应因 accept probe 不存在而失败。
2. GREEN：新增 Linux-only accept module，拥有或安全借用 Display handle、socket source、event loop handle 与事件 sender。
3. 在 socket callback 中先分配 adapter session ID，构造只保存 session ID / sender 的 `ClientData`，再调用 `DisplayHandle::insert_client`；只有 insert 成功才发布 connected event。
4. RED/GREEN：insert 失败时返回结构化 error/diagnostic，不发布 connected event，不创建 core 状态。
5. RED/GREEN：两个连接产生两个不同 session ID，事件顺序稳定。
6. 保持 `registers_protocol_globals = false`、`dispatches_protocol_events = false`、`supports_real_wayland_surfaces = false`、`supports_gpu_rendering = false`。

**测试要求**

- Linux test 必须真实绑定 socket 并真实连接，不可用纯模型假装 accept；
- 测试使用临时有效 `XDG_RUNTIME_DIR` 或 CI 提供的真实目录，并验证清理；
- connected proof 只断言 stream 被插入 display 与 event 被发布，不断言 wl_surface / xdg-shell；
- Mac 只跑 default + `smithay-probe`；Linux 由 GitHub Actions 跑 `smithay-linux`。

**Git 检查要求**

- 开始确认 Phase 51C commit 与 clean tree；
- 完成检查只包含新增 accept module 和批准的最小门面文件；
- `git diff --check` 必须通过；
- 不 stage 运行时 socket、日志或临时目录。

**Feature gate 约束**

```rust
#[cfg(all(feature = "smithay-linux", target_os = "linux"))]
```

- 不让 `smithay-probe` 引入 Smithay / calloop / Unix-only 类型；
- 优先使用现有依赖面；若编译证明缺少 event-loop dependency，停止并单独申请 Cargo 变更，不在本 Phase 偷加依赖。

**Public API 兼容要求**

- 保留现有 `SmithayWaylandDisplayProbe` / `SmithayWaylandSocketProbe` 构造入口；
- 新 ownership seam 应尽量 `pub(crate)`；
- 不把 `DisplayHandle` 或 `Client` re-export 到 core-facing facade。

**验收标准**

- Linux CI 日志证明真实 socket connect -> callback -> `insert_client` -> one connected event；
- capability 报告只把“accept probe 可用”作为单独能力，不提升 surface/renderer 能力；
- 无 global、surface、xdg-shell、renderer 实现。

**风险**

- `XDG_RUNTIME_DIR` 权限和并行 socket 名冲突；
- event loop pump 时序导致脆弱测试，必须使用有界等待而非无限阻塞；
- `DisplayHandle::insert_client` 与 `ClientData` 的版本接口必须由当前 lockfile 的 Linux 编译证实；
- accept 成功与 event 发布之间的错误处理必须避免 ghost core client。

### Phase 51E：Client connection runtime bridge

**目标**

把 accepted session event 接入现有 core seam，完成真实 connection -> `ClientRegistry` -> clean `ValidationReport` 的最小端到端证明。

**允许修改文件**

```text
src/smithay_backend/client_session.rs
src/smithay_backend/nested_client_runtime.rs # 新增 coordinator，推荐
src/smithay_backend/nested_client_accept.rs
src/smithay_backend/mod.rs
src/smithay_backend/runtime_facade.rs
```

仅当 Phase 51B 证明 core 缺口仍存在时，另行批准对应 `src/core/**` 文件；默认不修改 core。

**禁止修改文件**

```text
src/backend/**
Cargo.toml
Cargo.lock
.github/workflows/**
真实 wl_surface / xdg-shell / renderer 相关文件
```

**实现步骤**

1. RED（跨平台 pure test）：coordinator 收到 `Connected(session)` 后，断言它提交现有 `BackendEvent::ClientConnected { client: None, kind: WaylandPlaceholder, name: None }`，返回 generated `ClientId` 并保存 mapping。
2. GREEN：实现 coordinator，唯一状态修改调用为 `CoreRuntimeBridge::handle_backend_event`。
3. RED/GREEN：重复 connected session 返回结构化错误，不产生第二个 core client。
4. RED/GREEN：已映射 `Disconnected(session)` 转为 `BackendEvent::ClientDisconnected`，移除 active mapping，返回 clean validation。
5. RED/GREEN：unknown disconnect 不提交伪造 core event，并产生可诊断结果。
6. Linux integration：真实 socket client 连接，pump accept probe，drain connected event，经 coordinator 后断言 `ClientRegistry` alive、runtime validation clean、DebugBundle 可见。
7. 可选 Linux follow-up：如果当前最小 display pump 能可靠触发 `ClientData::disconnected`，再增加真实断开证明；若不能，不把 synthetic disconnect 伪装为真实断开，明确留给下一 Phase。

**测试要求**

- coordinator 的绝大多数行为在 `smithay-probe` 下用纯数据事件测试；
- Linux 只保留一条真实 connection tracer bullet，减少环境不稳定面；
- 对 connect、duplicate、unknown disconnect、normal disconnect 分别执行 RED -> GREEN；
- 完整运行 default、`smithay-probe` 和 GitHub Actions `smithay-linux` 矩阵。

**Git 检查要求**

- 开始确认 Phase 51D commit 与 clean tree；
- 完成限定 diff 到批准文件，执行 `git diff --check`；
- stage 后检查 `git diff --cached --name-status`，任何额外文件都停止；
- 不 push、不 merge，等待用户检查。

**Feature gate 约束**

- mapping/coordinator pure logic 放在 `smithay-probe`；
- 真实 socket integration 仅在 `smithay-linux + Linux`；
- core 始终不依赖任一 Smithay feature。

**Public API 兼容要求**

- 复用现有 core event/command/result；
- 保留旧 probe / skeleton public reports；
- 新 runtime report 为 additive，不把原来的 `ProbeOnly` 静默改成完整 runtime；
- `State` 仍是唯一 ClientRegistry mutator seam。

**验收标准**

```text
real nested socket connection
    -> accepted session event
    -> BackendEvent::ClientConnected
    -> CoreCommand::RegisterClient
    -> State / ClientRegistry alive record
    -> RuntimeEventResult.validation clean
    -> DebugBundle reflects client
```

同时 capability 仍明确：无 protocol globals、无 wl_surface、无 xdg-shell、无 renderer。

**风险**

- accept event 与 disconnect event 的竞态；
- coordinator mapping 写入失败后的 client cleanup；
- adapter event channel 满载或关闭；
- integration test 只能证明最小连接链，不能证明协议 request 时序。

## 9. 测试策略

每个后续 Phase 的基础矩阵：

```bash
cargo fmt --check
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
```

TDD 规则：

1. 一次只写一个 observable behavior 测试；
2. 先运行并确认因缺失行为而失败；
3. 写最小实现；
4. 运行目标测试确认通过；
5. 运行相关模块测试；
6. 最后运行完整矩阵；
7. 只在 GREEN 后重构；
8. 测试通过 public interface，不验证 private container 或调用次数。

Linux-only 矩阵：

```bash
cargo check --features smithay-linux
cargo test --features smithay-linux
```

- Mac 本地不强制运行 `smithay-linux`；
- Linux 验证交给 GitHub Actions；
- 真实 nested runtime 必须由 Linux CI log、artifact 或 Linux 主机测试证明；
- 51D/51E 的 Linux test 必须证明真实 socket/stream 路径，不能只跑纯数据 scenario；
- `XDG_RUNTIME_DIR` 必须存在且是目录；测试 socket 名必须唯一并在结束时清理。

建议按测试金字塔控制环境成本：

```text
大量：core / coordinator pure-data unit and integration tests
少量：smithay-probe boundary/source tests
一条：smithay-linux real socket connection tracer bullet
```

## 10. 风险

1. **能力夸大**：Display/socket 构造和 client insert 不等于 compositor 可用；capability 字段必须保持细粒度。
2. **ID 混淆**：core `ClientId`、adapter session ID、wayland-backend ClientId 是三种 identity，禁止数值强转替代映射。
3. **竞态**：client 可能在 coordinator 完成 mapping 前断开；adapter event 必须有稳定顺序和 exactly-once 处理。
4. **ghost record**：`insert_client`、事件发布、core 注册任一步失败，都可能使两侧状态不同步；每步需结构化结果。
5. **callback 所有权**：`ClientData::disconnected` 不能持有 `&mut State`；应用 channel/queue 解耦 callback 与 core mutation。
6. **现有 skeleton 冲突**：`linux_adapter.rs` 有封板测试禁止 accept/calloop/core 依赖，真实路径应新增模块而不是悄悄放宽测试。
7. **API 版本漂移**：官方文档显示的 `DisplayHandle::insert_client` / `ClientData` 接口必须以项目锁定版本在 Linux CI 编译为准。
8. **环境不稳定**：`XDG_RUNTIME_DIR`、Unix socket 权限、并行测试名称和清理会影响 CI。
9. **type alias 安全性**：当前 core ID 都是整数 alias，编译器不能阻止误传；本切片通过 newtype adapter session 与 seam 测试降低风险，不扩大到全局迁移。
10. **范围膨胀**：client 连接后没有 globals 是预期状态；不得因 client 无法创建窗口而提前进入 xdg-shell、wl_surface 或 renderer。

## 11. 不做事项

Phase 51A 以及本计划的 51B–51E 最小切片不实现：

```text
真实 wl_surface
真实 xdg_toplevel / xdg-shell global
surface map / unmap
buffer attach / commit
renderer / GPU / frame callback
keyboard / pointer / seat
DRM / GBM / libinput / udev
真实 Linux session
完整 Smithay handler 集合
UI / 胶囊任务栏 / 桌面组件 / onboarding / 窗口记忆
```

也不做以下架构捷径：

- 不把 Smithay / Wayland 类型放入 core；
- 不让 backend 直接修改 `ClientRegistry`；
- 不复制已有 `BackendEvent` / `CoreCommand` variants；
- 不把 probe scenario 当成真实 socket 证据；
- 不把 accepted connection 当成 surface 或 window；
- 不在 Mac 强行运行 `smithay-linux`；
- 不在未获得用户批准时修改 Cargo、CI、审计文档或进入 Phase 51B。
