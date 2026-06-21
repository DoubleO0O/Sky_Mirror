Sky Mirror Phase 45-47P Arch/Linux 验证交接说明
================================================

一、交接基线

当前 HEAD：

9348b21 chore: establish Phase 45.6 baseline

生成交接包的主机：

Darwin arm64
Rust host: aarch64-apple-darwin

本交接包包含当前工作区相对上述 HEAD 的以下阶段：

1. Phase 45 Final Seal
   保留 SmithayRuntimeProbe 旧 Linux API，并确保 Linux runtime 资源测试不使用
   early return 假通过。

2. Phase 46 runtime facade
   新增 BackendRuntimeReport、BackendRuntimeCapabilities 和结构化 runtime diagnostics。

3. Phase 47M pure surface lifecycle
   新增 Mac-safe 的 surface 生命周期纯数据模型。

4. Phase 47N surface trace / mock adapter
   新增纯数据 trace runner、scenario 和 mock adapter。

5. Phase 47O surface window candidate intent
   新增从 surface 最终记录到窗口候选意图的纯数据规划层。

6. Phase 47P window admission preview
   新增从窗口候选意图到未来接纳动作的纯数据预检层，并新增
   supports_window_admission_preview capability。

二、为什么没有运行 smithay-linux

本交接包在 macOS / Darwin arm64 上生成。smithay-linux 明确为 Linux-only feature，
需要 Wayland、libudev 等 Linux 原生依赖，因此没有在本机运行：

cargo check --features smithay-linux
cargo test --features smithay-linux

这不是 Linux 验证通过。Phase 47 仍被 Arch/Linux 真实编译与测试阻塞。

三、重要边界声明

1. Phase 47M/47N/47O/47P 都是纯数据或纯预检层，不等于真实 Wayland surface 支持。
2. supports_real_wayland_surfaces 仍为 false。
3. supports_gpu_rendering 仍为 false。
4. window candidate intent 没有进入 core。
5. window admission preview 没有创建 core window。
6. window admission preview 没有分配真实 workspace 或 slot。
7. 没有接入 wl_surface、xdg_toplevel 或真实 Smithay callback。
8. 所有既有状态事件仍必须经过 BackendDriverRunner / CoreRuntimeBridge 路径。
9. default Cargo feature 仍为空。

四、在 Arch/Linux 上应用补丁

请在干净或可控的 Sky Mirror 仓库中执行：

git checkout 9348b21
git apply --check phase45_47p_handoff.patch
git apply phase45_47p_handoff.patch

应用后确认以下文件存在：

test -f src/smithay_backend/runtime_facade.rs
test -f src/smithay_backend/surface_lifecycle.rs
test -f src/smithay_backend/surface_trace.rs
test -f src/smithay_backend/surface_window_intent.rs
test -f src/smithay_backend/window_admission_preview.rs

同时确认关键 API：

rg 'BackendRuntimeReport' src/smithay_backend
rg 'BackendSurfaceRegistry' src/smithay_backend
rg 'BackendSurfaceTrace' src/smithay_backend
rg 'BackendWindowCandidateIntent' src/smithay_backend
rg 'BackendWindowAdmissionPreviewAction|WindowAdmissionPreviewPlanner' src/smithay_backend

五、Arch/Linux 必须运行的验证

先运行跨平台与纯数据 probe 基线：

cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe

随后必须运行 Linux 专属验证：

cargo check --features smithay-linux
cargo test --features smithay-linux

不要跳过 smithay-linux，不要使用 early return 掩盖 Display、socket 或
XDG_RUNTIME_DIR 失败。

如 Arch Linux 缺少系统依赖，可按实际错误安装。常见预备依赖为：

sudo pacman -S --needed base-devel pkgconf wayland wayland-protocols libxkbcommon libinput libudev-zero mesa libglvnd

不要通过删除 dependency、feature 或测试来绕过原生依赖问题。

六、Linux 验收标准

1. cargo fmt --check 通过。
2. cargo build 通过。
3. cargo test 通过。
4. cargo check --features smithay-probe 通过。
5. cargo test --features smithay-probe 通过。
6. cargo check --features smithay-linux 通过。
7. cargo test --features smithay-linux 通过。
8. SmithayRuntimeProbe 旧 Linux API 继续可编译。
9. Linux Display/socket 测试真实运行。
10. runtime facade 在 Linux feature 下可编译。
11. surface lifecycle、trace、candidate intent 和 admission preview 保持纯数据边界。
12. core/backend 不依赖 smithay_backend 的 preview 类型。
13. supports_real_wayland_surfaces 保持 false。
14. supports_gpu_rendering 保持 false。
15. default = [] 保持不变。

七、Linux 失败时允许修改的范围

优先只允许修改：

src/smithay_backend/runtime.rs
src/smithay_backend/bootstrap.rs
src/smithay_backend/linux_runtime.rs
src/smithay_backend/runtime_facade.rs
src/smithay_backend/mod.rs
src/smithay_backend/wayland_display.rs
src/smithay_backend/wayland_socket.rs

如果失败与纯数据阶段的跨 feature 编译直接相关，可最小修改：

src/smithay_backend/surface_lifecycle.rs
src/smithay_backend/surface_trace.rs
src/smithay_backend/surface_window_intent.rs
src/smithay_backend/window_admission_preview.rs

原则上禁止修改：

Cargo.toml
src/core/*
src/backend/*
src/main.rs

如果 Cargo.toml 确实必须修改，只允许修复必要的 feature/dependency 边界，
必须说明原因，并保持 default = []、smithay-probe 和 smithay-linux。

不得删除测试、削弱断言、增加静默跳过或 early return。

八、成功后的下一步

只有 Arch/Linux 上的全部 smithay-linux 命令通过后，才允许解除 Phase 47
阻塞并进入真实 Smithay adapter 阶段。

即使 Linux 验证通过，也应继续保持：

Backend callback
  -> BackendEvent
  -> BackendDriverRunner
  -> CoreRuntimeBridge
  -> CoreCommand
  -> State

不得让真实 adapter 直接修改 State、workspace、slot、stack 或 focus。

九、Mac 侧封板结果

生成本包前已在 Darwin arm64 上通过：

cargo fmt
cargo fmt --check
cargo build
cargo test
cargo check --features smithay-probe
cargo test --features smithay-probe

测试基线：

default: 131 passed
smithay-probe: 344 passed

Mac 侧验证不能替代 Arch/Linux smithay-linux 验证。
