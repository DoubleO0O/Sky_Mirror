Sky Mirror Phase 45/46 Arch/Linux 验证交接说明
================================================

1. 当前基线

当前 Git HEAD：

9348b21 chore: establish Phase 45.6 baseline

交接 patch 基于该提交生成，包含：

- Phase 45 Final Seal 的 Smithay runtime 兼容 API 与 Linux 资源测试加固。
- Phase 46 的 BackendRuntimeReport、capabilities、diagnostics 和 runtime facade。
- 未跟踪文件 src/smithay_backend/runtime_facade.rs 的完整新增内容。

2. 当前 host OS

生成交接包的主机：

Darwin 25.5.0 arm64
Rust host: aarch64-apple-darwin

3. 为什么没有运行 smithay-linux

smithay-linux 只允许在 Linux 上启用。当前 macOS 构建会由项目自己的
compile_error! 明确拒绝，因此这里没有运行：

cargo check --features smithay-linux
cargo test --features smithay-linux

本交接包不声称 Linux 验证已经通过。

4. 在 Arch/Linux 上应用 patch

请在干净的 Sky Mirror 仓库中执行：

git checkout 9348b21
git apply --check phase45_46_handoff.patch
git apply phase45_46_handoff.patch

应用前应确认工作区没有会与 patch 冲突的本地修改。

5. Arch/Linux 系统依赖

如系统尚未安装必要依赖，可优先执行：

sudo pacman -S --needed base-devel pkgconf wayland wayland-protocols libxkbcommon libinput libudev-zero mesa libglvnd

不要通过删除 smithay-linux feature 或放宽测试来绕过系统依赖错误。

6. Arch/Linux 必须运行的验证命令

依次执行：

cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
cargo check --features smithay-linux
cargo test --features smithay-linux

最终验收前再完整执行：

cargo fmt
cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe
cargo check --features smithay-linux
cargo test --features smithay-linux

7. 验收标准

- 所有命令通过。
- default feature 仍为 []。
- SmithayRuntimeProbe 旧 Linux API 继续编译：
  with_socket_name、from_parts、bootstrap_mode、socket_name_string。
- SmithayLinuxRuntimeProbe 可以编译并运行测试。
- Display、socket 和 XDG_RUNTIME_DIR 资源测试真实执行。
- BackendRuntimeReport、BackendRuntimeCapabilities 和
  BackendRuntimeDiagnostic 在 smithay-linux 下编译。
- From<&SmithayLinuxRuntimeProbe> 的 Linux 条件转换编译并通过测试。
- 测试中没有 early return、let Ok(...) else 或接受错误后继续通过的模式。
- src/core 和 src/backend 不依赖 smithay_backend runtime facade 类型。
- 所有状态事件继续通过 BackendDriverRunner。

8. 如果验证失败，允许修改的范围

优先只修改：

- src/smithay_backend/runtime.rs
- src/smithay_backend/bootstrap.rs
- src/smithay_backend/linux_runtime.rs
- src/smithay_backend/runtime_facade.rs
- src/smithay_backend/mod.rs
- 与 Smithay Linux runtime 直接相关的测试

原则上不要修改：

- Cargo.toml
- src/core/*
- src/backend/*
- src/main.rs

若 Cargo.toml 确实需要修复 Linux feature/dependency 边界，必须说明原因，
并保持 default = []。不得删除测试、削弱断言或用静默跳过掩盖失败。

9. 验证成功后的下一步

只有 cargo test --features smithay-linux 在真实 Arch/Linux 上通过后，
才解除 Phase 47 阻塞并进入 Phase 47B，开始设计 surface_lifecycle.rs。

Phase 47B 仍不得改变 core workspace、slot、stack、focus 语义，也不得绕过
BackendDriverRunner。
