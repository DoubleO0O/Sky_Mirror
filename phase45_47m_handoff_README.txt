Sky Mirror Phase 45/46/47M Arch/Linux 验证交接说明
====================================================

一、基线信息

当前 Git HEAD：
9348b21 chore: establish Phase 45.6 baseline

生成交接包的主机：
macOS / Darwin arm64
Rust host：aarch64-apple-darwin

本交接包包含：

1. Phase 45 Final Seal：旧 SmithayRuntimeProbe Linux API 兼容与 Linux 资源测试封板。
2. Phase 46：BackendRuntimeReport、capabilities、diagnostics 运行时门面。
3. Phase 47M：Mac-safe 的纯 Rust surface 生命周期模型、结构化错误和测试。

Phase 47M 只代表后端中立的纯数据生命周期边界已经存在，不等于真实 Wayland
surface 支持。它没有保存 wl_surface，没有接入 xdg_toplevel，也没有进入 GPU
渲染或真实 compositor 主循环。

二、为什么没有运行 smithay-linux

交接包在 macOS / Darwin arm64 上生成。smithay-linux 依赖 Linux、Wayland 和
XDG_RUNTIME_DIR 等系统资源，不能在该主机上完成真实验证。因此生成端没有运行：

cargo check --features smithay-linux
cargo test --features smithay-linux

Phase 47 的 Linux 阻塞仍未解除。只有在 Arch/Linux 上完成下述验证后，才允许
进入 Phase 47B/48 的真实 Smithay adapter 工作。

三、在 Arch/Linux 上应用补丁

请在干净或可控的 Sky Mirror 工作区中执行：

git checkout 9348b21
git apply --check phase45_47m_handoff.patch
git apply phase45_47m_handoff.patch

应用后必须确认以下文件存在：

src/smithay_backend/runtime_facade.rs
src/smithay_backend/surface_lifecycle.rs

并确认 Cargo.toml 中 default = [] 保持不变。

四、Arch/Linux 必须运行的验证命令

cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
cargo check --features smithay-linux
cargo test --features smithay-linux

smithay-linux 测试必须真实执行 Display、socket 和 XDG_RUNTIME_DIR 相关资源路径。
不允许通过 early return、静默跳过或放松断言掩盖失败。

五、验收标准

1. 上述全部命令通过。
2. SmithayRuntimeProbe 旧 Linux API 继续可编译。
3. SmithayLinuxRuntimeProbe 和 BackendRuntimeReport 的 Linux 转换可编译。
4. runtime_facade.rs 与 surface_lifecycle.rs 均被补丁完整应用。
5. Linux Display/socket 测试真实执行。
6. core/backend 不依赖 smithay_backend 的 runtime facade 或 surface lifecycle 类型。
7. supports_surface_lifecycle_boundary 为 true。
8. supports_real_wayland_surfaces 继续为 false。
9. supports_gpu_rendering 继续为 false。
10. default = [] 不变。

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

如果确实必须修改 Cargo.toml，只允许修复必要的 feature/dependency 边界，并且
default = []、smithay-probe、smithay-linux 都必须保留。不得删除测试，不得削弱
断言，不得为了通过验证提前实现真实 surface adapter。

七、验证成功后的下一步

只有 Arch/Linux 上的 smithay-linux 编译和测试全部通过，Phase 47 阻塞才可以
解除。随后才允许进入 Phase 47B/48，设计真实 Smithay 回调到纯数据生命周期事件、
BackendEvent、BackendDriverRunner 和 CoreRuntimeBridge 的适配路径。
