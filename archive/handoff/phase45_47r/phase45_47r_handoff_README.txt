Sky Mirror Phase 45-47R Arch/Linux 验证交接说明

一、基线信息

当前 HEAD：
9348b21 chore: establish Phase 45.6 baseline

当前生成环境：
Darwin arm64 / aarch64-apple-darwin

本交接包包含：
- Phase 45 Final Seal
- Phase 46 runtime facade
- Phase 47M pure surface lifecycle
- Phase 47N surface trace / mock adapter
- Phase 47O surface window candidate intent
- Phase 47P window admission preview
- Phase 47Q surface admission pipeline
- Phase 47R surface admission contract snapshot

二、Mac 验证状态

当前 Mac 环境已通过：
- cargo fmt --check
- cargo build
- cargo test：131 passed
- cargo check --features smithay-probe
- cargo test --features smithay-probe：413 passed

没有运行 smithay-linux。该 feature 依赖真实 Linux/Wayland 系统环境和原生依赖，
Darwin 不能替代 Arch/Linux 验证，因此 Phase 47 的 Linux 阻塞仍未解除。

三、能力和契约边界

Phase 47M/47N/47O/47P/47Q/47R 不等于真实 Wayland surface 支持。
supports_real_wayland_surfaces 仍为 false。
supports_gpu_rendering 仍为 false。

Contract、pipeline、preview 和 intent 都没有进入 core，没有构造 BackendEvent 或
CoreCommand，也没有调用 BackendDriverRunner。
Window admission preview 不创建 core window，不分配真实 workspace 或 slot。

Phase 47R golden scenarios 固定的是当前纯数据模型行为，用于防止后续适配器破坏
既有语义；这些快照不代表真实 Wayland 协议时序、wl_surface 或 xdg_toplevel 已经
完成验证。

四、在 Arch/Linux 上应用 patch

请在 Sky Mirror 仓库根目录执行：

git checkout 9348b21
git apply --check phase45_47r_handoff.patch
git apply phase45_47r_handoff.patch

应用后确认以下文件存在：
- src/smithay_backend/runtime_facade.rs
- src/smithay_backend/surface_lifecycle.rs
- src/smithay_backend/surface_trace.rs
- src/smithay_backend/surface_window_intent.rs
- src/smithay_backend/window_admission_preview.rs
- src/smithay_backend/surface_admission_pipeline.rs
- src/smithay_backend/surface_admission_contract.rs

五、Arch/Linux 必须执行的验证

cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
cargo check --features smithay-linux
cargo test --features smithay-linux

验收要求：
- 所有命令通过。
- smithay-linux 在真实 Linux 主机完成编译和测试。
- Linux Display、socket 和 XDG_RUNTIME_DIR 测试真实执行。
- 不允许使用 early return 或静默跳过掩盖失败。
- default = [] 保持不变。
- core/backend 不依赖 smithay_backend 的 runtime 或 surface 契约类型。
- supports_real_wayland_surfaces 继续为 false。
- supports_gpu_rendering 继续为 false。
- 未开始真实 wl_surface / xdg_toplevel adapter。

六、Linux 验证失败时允许修复的范围

优先只允许修改：
- src/smithay_backend/runtime.rs
- src/smithay_backend/bootstrap.rs
- src/smithay_backend/linux_runtime.rs
- src/smithay_backend/runtime_facade.rs
- src/smithay_backend/mod.rs
- src/smithay_backend/wayland_display.rs
- src/smithay_backend/wayland_socket.rs

如果失败直接涉及纯模型、管线或契约在 Linux 下的编译，才允许最小修改：
- src/smithay_backend/surface_lifecycle.rs
- src/smithay_backend/surface_trace.rs
- src/smithay_backend/surface_window_intent.rs
- src/smithay_backend/window_admission_preview.rs
- src/smithay_backend/surface_admission_pipeline.rs
- src/smithay_backend/surface_admission_contract.rs

原则上不得修改：
- Cargo.toml
- src/core/*
- src/backend/*
- src/main.rs

确实必须修改 Cargo.toml 时，必须说明 Linux 原生依赖或 feature 边界原因，
并保持 default = []、smithay-probe 和 smithay-linux 不被删除。

七、后续门槛

只有 Arch/Linux 上的 smithay-linux 完整验证通过后，才能进入真实 Smithay adapter
阶段。成功前不得声明 Phase 47 阻塞解除，也不得把当前纯模型 contract 或 pipeline
描述为真实 Wayland surface 生命周期支持。
