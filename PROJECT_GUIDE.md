# Sky Mirror 项目指南

> 当前真值入口。最后按 main@c6f1fd6 与 Ubuntu 26.04.1 本地验证整理。
> 历史演进与旧文档来源见 PROJECT_HISTORY.md；代理工程约束以 AGENTS.md 为准。

本文件是 Sky Mirror 的唯一项目主入口。后续项目说明、提交说明、当前状态和未来改动规划优先集中写入本文件；PROJECT_HISTORY.md 仅作为不可丢失的历史证据、旧文档来源映射与演进背景，不承担日常主入口职责。

## 1. 项目定位

Sky Mirror 是使用 Rust 与 Smithay 构建的 Linux Wayland 合成器／窗口管理器项目。它尝试用 X、Y、Z 三轴组织窗口：workspace 表达工作空间，固定 slot 表达平面位置，stack 表达同一位置中的深度。当前代码已经形成较完整的纯数据窗口状态机，并建立了一条受 feature 控制的 Linux nested runtime 证明路径，但还不是可见、可日常使用或可替代桌面会话的合成器。

当前仓库只有一个 binary package，版本 0.1.0，使用 Rust 2024 edition。项目目前面向继续开发和验证，而非最终用户发布：没有稳定安装包、配置格式、桌面会话文件、用户手册或兼容性承诺。

## 2. 真实性分级

阅读代码、报告和测试时必须使用以下分级，不能从局部证据外推产品能力：

- production fact：真实路径已经连接，资源、生命周期和失败语义在该窄范围内成立。
- controlled proof：受控测试环境中的真实或半真实证明，只覆盖明确测试的链路。
- callback observation：只证明回调被观察到，不证明后续状态或业务结果完成。
- report、descriptor、readiness：纯数据描述、决策或准备条件，不代表动作发生。
- skeleton：结构、接口或 owner 位置存在，但关键资源或执行仍未接通。
- mock：替身行为，只能证明调用契约或编排。
- unimplemented：目标能力尚无实现。

MockRenderer、DummyRenderer、InputSimulator、source guard、回调计数、报告字段、编译通过、单元测试绿色和 CI 绿色都不能称为 production compositor 能力。

## 3. 当前真实能力

### 3.1 已成立的代码能力

- Core 是以纯数据为主的集中状态机，表达 workspace、四个 slot、slot 内 stack、focus、Fullscreen／Split／Grid、逻辑 output size，以及 client、surface、window registry。
- 平台事件的标准状态入口已经定义为 BackendEvent -> CoreCommand -> State；CoreRuntimeBridge 负责翻译、执行和命令后验证。
- client 关闭可级联关闭其 surface 与 window；toplevel detach 可结束 window 而保留底层 surface；registry 保留 alive=false 的诊断 tombstone。
- session JSON 可保存 workspace、slot、stack、focus 与下一个 window ID，并恢复为纯数据状态。
- Linux 内部 production nested constructor 能依次创建 Display、初始化 wl_compositor 和 xdg_wm_base、绑定 Wayland socket、注册 calloop source，并接受／插入 client。
- Phase 56P 的有界外部客户端测试曾真实发现并 bind wl_compositor 与 xdg_wm_base；当前 Ubuntu 全矩阵也已重新通过。

### 3.2 仅受控证明或骨架

- wl_surface、xdg_surface、xdg_toplevel 的创建、new_toplevel 回调、identity registration、admission、unmap、commit／buffer／damage／frame-callback observation 主要存在于 controlled harness、callback observation 或 report 链路。
- adapter 有 ObjectId 到 AdapterSurfaceId／AdapterToplevelId 的映射与 tombstone 模型，但 production public path 尚未完成外部 toplevel 到 Core window 的全生命周期闭环。
- Linux adapter 能构造并持有 Smithay DummyRenderer，只证明 concrete type、owner、storage 与 drop cleanup seam；它不创建可显示 texture，也不绘制。
- 默认 main 使用三个 mock window、InputSimulator 和 MockRenderer 日志输出。它与 production nested orchestrator 尚未汇合。

### 3.3 尚未实现

- main 启动 production nested session。
- production external XDG create／map／unmap／destroy／disconnect 到 adapter、ledger、Core 的完整闭环。
- owned WlBuffer 生命周期、真实 SHM import、texture、render target、renderer call、damage submission、frame done。
- 真实 keyboard、pointer、touch 输入。
- DRM／KMS、GBM、dmabuf、EGL／GLES、Vulkan／WGPU、libinput、udev、seat、login/session、多输出与 XWayland。
- 可承诺的 session schema、原子保存与崩溃恢复。
- 最终用户配置、安装、启动、安全退出和故障排查体验。

## 4. 架构地图

当前 binary 路径：

    main
      -> State::new + load_session
      -> core::EventLoop
           InputSimulator -> InputEvent -> Action -> State
           State -> LayoutEngine -> SceneBuilder -> RenderPlanner -> MockRenderer

feature-gated Linux nested 路径：

    NestedRuntimeOrchestrator
      -> NestedRuntimeLoop
        -> NestedRuntimeCoordinator
          -> NestedRealAcceptFlow
            -> Smithay Display + Wayland socket + calloop source
            -> wl_compositor + xdg_wm_base
            -> client/session mapping
            -> adapter observation / queues / ledger
               -> BackendEvent -> CoreRuntimeBridge -> CoreCommand -> State

边界原则：Core 不持有 Smithay、Wayland 或 Linux 对象；backend 不得直接修改 workspace、slot、stack、focus 或 registry；ObjectId、adapter ID、session ID 与 core ID 必须显式转换；资源创建、登记、注销、销毁、断连与补偿必须有唯一 owner。

## 5. Cargo feature 分层

- default：空 feature 集。编译 Core、旧原型入口与默认可见纯数据边界，不引入 Smithay crate。
- smithay-probe：加入后端中立的纯数据事件、driver、runtime、scenario、surface lifecycle／trace／admission 等证明层，不应引入平台资源。
- smithay-linux：包含 smithay-probe，并启用 Smithay 0.7、wayland-client、wayland-protocols 与 Smithay renderer_test；仅允许 Linux。
- smithay-backend：smithay-linux 的兼容别名，不是独立实现。

Smithay 关闭默认 features，只启用 wayland_frontend；renderer_test 只用于 DummyRenderer proof，当前没有真实图形后端。

## 6. Ubuntu 开发环境

本项目已在 Ubuntu 26.04.1 LTS x86_64 上恢复并验证。建议使用 Rust 官方 rustup 的最新 stable，而不是 Ubuntu 的旧版 Rust 包。

已验证基线：

- rustup 1.29.1
- rustc／cargo 1.98.1 stable
- rustfmt 1.9.0-stable
- clippy 0.1.98
- GCC／G++ 15.2、GNU Make 4.4.1、build-essential
- libxkbcommon-dev 1.13.1
- VS Code rust-lang.rust-analyzer 0.3.3041

最小系统依赖：

    sudo apt-get update
    sudo apt-get install --yes --no-install-recommends build-essential libxkbcommon-dev

Wayland socket 测试必须使用短、存在且权限为 0700 的 XDG_RUNTIME_DIR。过长路径会触发 Unix SUN_LEN 限制，这属于 harness 环境错误，不能伪装成代码 Red。例如：

    export XDG_RUNTIME_DIR=/tmp/smrt-test
    install -d -m 700 "$XDG_RUNTIME_DIR"

## 7. 构建与验证

在仓库根执行，不要直接运行 binary 来做健康检查：

    cargo fmt --check
    cargo check --locked
    cargo test --locked
    cargo check --locked --features smithay-probe
    cargo test --locked --features smithay-probe
    cargo check --locked --features smithay-linux
    cargo test --locked --features smithay-linux
    cargo check --locked --all-features
    cargo test --locked --all-features
    cargo clippy --locked --all-features --tests

当前 Ubuntu 新鲜结果：default 352 tests、smithay-probe 634 tests、smithay-linux 884 tests、all-features 884 tests，均为 0 failed、0 ignored。Clippy 完成但有 111 条非致命 warning；不要把 warning 当作顺手清理授权。

CI 位于 .github/workflows/ci.yml，在 ubuntu-latest 安装 stable Rust 与 libxkbcommon-dev，创建短 runtime 目录，然后运行 fmt、default、probe 和 Linux check/test。CI 配置本身是独立自动化真值，不由本文替代。

## 8. 运行限制

当前不要把 cargo run 当成 compositor 验收：main 会启动旧的无限事件循环，读取或生成 sky_mirror_session.json，周期性产生模拟输入，并将 RenderFrame 打到日志；它不启动 production nested runtime，也不会显示窗口。开发验证不得接管用户桌面。真正的 nested session 或 DRM session 必须在独立、受控、有超时和明确退出机制的环境中设计。

## 9. 当前阶段与风险

最近完成能力阶段是 Phase 56P，基线提交 21916d9；当前 main HEAD c6f1fd6 在其后只增加 AGENTS.md 与项目状态文档，没有产品代码变化。Phase 56Q 只是候选，尚未批准。

主要风险按优先级为：

1. 用户路径断裂：main 与 production nested orchestrator 是两条未汇合路径。
2. production XDG lifecycle 尚无 create／destroy／disconnect 完整闭环。
3. 渲染与输入均未实现，无法产生可见、可操作桌面。
4. 多层 identity 与 owner 若处理不严会造成错绑、重复 cleanup 或幽灵窗口。
5. session 直接写相对 JSON，缺 schema/version、原子替换和完整语义校验。
6. 大量 report／readiness／proof 造成认知负担；测试数量远大于真实集成覆盖。
7. 关键 Core 字段仍较公开，部分边界靠约定而非类型系统强制。
8. 默认三个 workspace，但快捷键含 Super+4 -> ID 3，第四项当前无目标 workspace。

## 10. 恢复路线

- R0：Ubuntu Rust／native 依赖／rust-analyzer 与完整矩阵恢复。已完成。
- R1／Phase 56Q：production nested public path 的真实外部 toplevel lifecycle tracer bullet。
- R2：SHM-first 的 owned buffer、texture、render、damage、frame done 最小可见闭环。
- R3：nested keyboard／pointer 输入与焦点、workspace、slot、stack、layout 交互。
- R4：多客户端可靠性、协议错误、session 安全、README／运行说明与产品化。
- R5：nested 可用后再设计 DRM/KMS、GBM、libinput、seat/session、多输出与 XWayland。

## 11. 下一步审批门槛

推荐的第一个代码阶段是 R1／Phase 56Q：让真实外部 client 通过 NestedRuntimeOrchestrator::start() 的 production 路径依次创建 wl_surface、xdg_surface、xdg_toplevel；将 session 解析为 core ClientId；只经 BackendEvent -> CoreRuntimeBridge -> CoreCommand -> State 注册 window；在 xdg_toplevel destroy 后 retire adapter identity、ledger unmap、Core detach/dead，同时保持底层 wl_surface 存活，直到 surface destroy 或 client disconnect。

开始前必须明确批准：goal、精确 allowlist/denylist、正确 Red、最小 Green、timeout/watchdog、cleanup 与 destroy 语义。建议沿用现有 ID 类型，先不夹带 newtype 重构。该阶段必须继续禁止 buffer、import、texture、renderer、damage、frame done、input 与 DRM。

## 12. 文档权威顺序

1. 实时 Git、源码与 CI。
2. AGENTS.md 的长期工程规则。
3. 本文件的规范化当前状态。
4. PROJECT_HISTORY.md 的演进、旧证据与来源映射。
5. archive/handoff 中保留的 patch／zip 原始交付归档。

历史测试只证明当时提交和环境；每个新阶段必须取得本阶段的新鲜验证。
