Sky Mirror Phase 45/46/47M/47N/47O Arch/Linux 验证交接说明
============================================================

一、基线信息

当前 Git HEAD：
9348b21 chore: establish Phase 45.6 baseline

生成交接包的主机：
macOS / Darwin arm64
Rust host：aarch64-apple-darwin

本交接包包含：

1. Phase 45 Final Seal：SmithayRuntimeProbe 旧 Linux API 兼容和资源测试封板。
2. Phase 46：BackendRuntimeReport、capabilities、diagnostics 运行时门面。
3. Phase 47M：纯 Rust surface 生命周期模型、注册表和结构化错误。
4. Phase 47N：surface trace runner、执行报告、mock adapter 和场景测试。
5. Phase 47O：surface 到 window candidate 的纯数据意图规划层。

Phase 47M/47N/47O 只建立后端中立的纯数据模型、trace 和候选意图，不等于真实
Wayland surface 支持。它们没有保存 wl_surface，没有接入 xdg_toplevel，也没有
进入真实 compositor 或 GPU 渲染路径。

Window candidate intent 没有进入 core，没有转换成 CoreCommand，没有调用
BackendDriverRunner，也没有修改 workspace、slot、stack 或 focus。

当前能力声明必须保持：

supports_surface_lifecycle_boundary = true
supports_surface_trace_harness = true
supports_surface_window_intent_planning = true
supports_real_wayland_surfaces = false
supports_gpu_rendering = false

二、为什么没有运行 smithay-linux

交接包在 macOS / Darwin arm64 上生成。smithay-linux 依赖 Linux、Wayland、
XDG_RUNTIME_DIR 和相关系统资源，不能在当前主机完成真实编译与资源测试。
因此生成端没有运行：

cargo check --features smithay-linux
cargo test --features smithay-linux

Phase 47 的 Linux 阻塞仍未解除。

三、在 Arch/Linux 上应用补丁

请在干净或可控的 Sky Mirror 工作区中执行：

git checkout 9348b21
git apply --check phase45_47o_handoff.patch
git apply phase45_47o_handoff.patch

应用后必须确认以下文件存在：

src/smithay_backend/runtime_facade.rs
src/smithay_backend/surface_lifecycle.rs
src/smithay_backend/surface_trace.rs
src/smithay_backend/surface_window_intent.rs

同时确认 Cargo.toml 中 default = [] 保持不变。

四、Arch/Linux 必须运行的验证命令

cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
cargo check --features smithay-linux
cargo test --features smithay-linux

smithay-linux 测试必须真实执行 Display、socket 和 XDG_RUNTIME_DIR 资源路径。
不允许通过 early return、静默跳过、删除测试或放松断言掩盖失败。

五、验收标准

1. 上述全部命令通过。
2. SmithayRuntimeProbe 旧 Linux API 继续可编译。
3. SmithayLinuxRuntimeProbe 和 BackendRuntimeReport 的 Linux 转换可编译。
4. runtime_facade.rs、surface_lifecycle.rs、surface_trace.rs 和
   surface_window_intent.rs 完整应用。
5. Linux Display/socket 测试真实执行。
6. Trace runner 仍只通过 BackendSurfaceRegistry::apply_event 推进状态。
7. Window intent planner 只读取最终记录，不重放 trace，不调用 core。
8. core/backend 不依赖 runtime facade、surface lifecycle、surface trace 或
   surface window intent 类型。
9. supports_surface_lifecycle_boundary 为 true。
10. supports_surface_trace_harness 为 true。
11. supports_surface_window_intent_planning 为 true。
12. supports_real_wayland_surfaces 为 false。
13. supports_gpu_rendering 为 false。
14. default = [] 不变。

六、smithay-linux 失败时允许修改的范围

优先只允许修改：

src/smithay_backend/runtime.rs
src/smithay_backend/bootstrap.rs
src/smithay_backend/linux_runtime.rs
src/smithay_backend/runtime_facade.rs
src/smithay_backend/mod.rs
src/smithay_backend/wayland_display.rs
src/smithay_backend/wayland_socket.rs

原则上不允许修改：

Cargo.toml
src/core/*
src/backend/*
src/main.rs
src/smithay_backend/surface_lifecycle.rs
src/smithay_backend/surface_trace.rs
src/smithay_backend/surface_window_intent.rs

如果确实必须修改 Cargo.toml，只允许修复必要 feature/dependency 边界，并保持
default = []、smithay-probe、smithay-linux 和兼容别名不变。不得删除测试，
不得削弱断言，也不得借验证修复提前实现真实 surface adapter 或把候选意图送入
core。

七、验证成功后的下一步

只有 Arch/Linux 上 smithay-linux 的编译和测试全部通过，Phase 47 阻塞才可以
解除。成功后才允许进入真实 Smithay adapter 阶段，把平台回调转换为既有
BackendSurfaceLifecycleEvent，并单独设计候选意图到 BackendEvent/CoreCommand
边界；该边界仍必须经过 BackendDriverRunner 和 CoreRuntimeBridge。
