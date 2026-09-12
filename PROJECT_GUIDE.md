# Sky Mirror 项目指南

> 当前真值入口。动态代码基线以 Git 的当前 HEAD 与 CI 为准；本指南按 Ubuntu 26.04.1 环境整理。
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
- Linux 内部 production nested constructor 能依次创建 Display、初始化 wl_compositor、wl_shm 和 xdg_wm_base、绑定 Wayland socket、注册 calloop source，并接受／插入 client；wl_shm global 的存在不代表 buffer import 或 render 已发生。
- Phase 56P/56Q 的有界外部客户端测试真实发现并 bind wl_compositor、wl_shm 与 xdg_wm_base；当前 narrow production path 还能创建 wl_surface、xdg_surface、xdg_toplevel，完成 initial configure/ack/commit，并经同一 session mapping admission 到 Core，再消费 toplevel unmap；本轮 `smithay-linux` 与 `all-features` 新鲜验证均已通过。

### 3.2 仅受控证明或骨架

- wl_surface、xdg_surface、xdg_toplevel 的创建、configure/ack、commit、new_toplevel 回调、identity registration、同会话 admission 与 toplevel unmap 已在 bounded production socket tracer 中成立。
- adapter 有 ObjectId 到 AdapterSurfaceId／AdapterToplevelId 的映射与 tombstone 模型；narrow production path 已完成外部 toplevel 到 Core window 的 create/admit/unmap 窄闭环，但仍不是 long-running compositor，也未把 disconnect cleanup 与 surface/resource lifecycle 宣称为完整产品能力。
- R2 受控 binary 在进程主线程持有真实 Winit/EGL/GLES target，外部 client 从同一 production socket 提交 2×2 XRGB8888 SHM buffer。coordinator 在转移 WlBuffer 前重新核对 session、adapter surface/toplevel、admission ledger、live Core surface/window 与 exact commit token；随后完成真实 SHM import、texture 内存回读、texture draw、Winit backbuffer submit 与 output damage submit。回读的红／绿／蓝／白像素与 client 原始图样逐像素一致；只有 presentation、identity、damage 与 callback 数量全部匹配时才发送一次 frame done。该结果是 bounded controlled proof，不是 production desktop。
- Linux adapter 能构造并持有 Smithay DummyRenderer，只证明 concrete type、owner、storage 与 drop cleanup seam；它不创建可显示 texture，也不绘制。
- 默认 main 使用三个 mock window、InputSimulator 和 MockRenderer 日志输出。它与 production nested orchestrator 尚未汇合。

### 3.3 尚未实现

- main 启动 production nested session。
- production external XDG 的长时 create／map／unmap／destroy／disconnect 到 adapter、ledger、Core 的完整产品闭环（当前只有 bounded tracer 的 create/configure/admit/unmap 与独立真实 disconnect callback proof）。
- 长时 owned WlBuffer 替换／release、surface-tree 合成、任意尺寸与变换、精确 per-surface damage tracking、持续帧调度及 production main 中的可见输出；当前 R2 只覆盖单个受控 SHM 首帧，成功 source buffer 保留到 output owner drop。
- 真实 keyboard、pointer、touch 输入。
- DRM／KMS、GBM、dmabuf、Vulkan／WGPU、libinput、udev、seat、login/session、多输出与 XWayland；EGL／GLES 仅用于 R2 nested Winit controlled target，未接入 DRM 或默认 main。
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
            -> wl_compositor + wl_shm + xdg_wm_base
            -> client/session mapping
            -> adapter observation / queues / ledger
               -> BackendEvent -> CoreRuntimeBridge -> CoreCommand -> State
          -> R2 atomic SHM admission owner
             -> Winit/EGL/GLES import + readback + draw + submit
             -> success + damage + exact callback completion gate

边界原则：Core 不持有 Smithay、Wayland 或 Linux 对象；backend 不得直接修改 workspace、slot、stack、focus 或 registry；ObjectId、adapter ID、session ID 与 core ID 必须显式转换；资源创建、登记、注销、销毁、断连与补偿必须有唯一 owner。

## 5. Cargo feature 分层

- default：空 feature 集。编译 Core、旧原型入口与默认可见纯数据边界，不引入 Smithay crate。
- smithay-probe：加入后端中立的纯数据事件、driver、runtime、scenario、surface lifecycle／trace／admission 等证明层，不应引入平台资源。
- smithay-linux：包含 smithay-probe，并启用 Smithay 0.7、wayland-client、wayland-protocols、Smithay renderer_test 与 backend_winit；仅允许 Linux。
- smithay-backend：smithay-linux 的兼容别名，不是独立实现。

Smithay 关闭默认 features；显式启用 wayland_frontend、renderer_test 与 backend_winit。renderer_test 仍只用于 DummyRenderer proof；backend_winit 为 R2 受控路径提供 nested Winit/EGL/GLES target，不代表 DRM/KMS 图形后端或默认产品入口已成立。

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

历史基线（本轮未重跑）：default 352 tests、smithay-probe 634 tests。

本轮 Ubuntu 新鲜结果：default 353、smithay-probe 635、smithay-linux 908、all-features 908 tests；default/probe 均为 0 failed、0 ignored，Linux/all-features 均为 907 passed、0 failed、1 ignored（该 ignored 项是 Winit 必须在进程主线程创建的 unit test，真实 main-thread controlled binary 已单独执行）。已完成相应 `cargo check --locked`、`cargo fmt --check`、`git diff --check` 与 `cargo clippy --locked --all-features --tests`；Clippy 以退出码 0 完成并报告 113 条既有非致命 warning，不把 warning 当作顺手清理授权。

CI 位于 .github/workflows/ci.yml，在 ubuntu-latest 安装 stable Rust 与 libxkbcommon-dev，创建短 runtime 目录，然后运行 fmt、default、probe 和 Linux check/test。CI 配置本身是独立自动化真值，不由本文替代。

## 8. 运行限制

当前不要把 cargo run 当成 compositor 验收：main 会启动旧的无限事件循环，读取或生成 sky_mirror_session.json，周期性产生模拟输入，并将 RenderFrame 打到日志；它不启动 production nested runtime，也不会显示窗口。开发验证不得接管用户桌面。真正的 nested session 或 DRM session 必须在独立、受控、有超时和明确退出机制的环境中设计。

## 9. 当前阶段与风险

最近完成能力阶段是 Phase 56Q 的 narrow production tracer；它建立在 Phase 56P 之上，但不等于 long-running compositor。当前状态以实时 Git、源码和新鲜 all-features 验证为准。

主要风险按优先级为：

1. 用户路径断裂：main 与 production nested orchestrator 是两条未汇合路径。
2. production XDG lifecycle 仍不是长时 create／destroy／disconnect 完整产品闭环；当前只覆盖 bounded tracer 与独立 disconnect callback proof。
3. R2 只证明受控单帧 nested SHM 可见链路；默认 main、长时渲染、buffer release 与输入仍未实现，尚无可投入使用的桌面。
4. 多层 identity 与 owner 若处理不严会造成错绑、重复 cleanup 或幽灵窗口。
5. session 直接写相对 JSON，缺 schema/version、原子替换和完整语义校验。
6. 大量 report／readiness／proof 造成认知负担；测试数量远大于真实集成覆盖。
7. 关键 Core 字段仍较公开，部分边界靠约定而非类型系统强制。
8. 默认三个 workspace，但快捷键含 Super+4 -> ID 3，第四项当前无目标 workspace。

## 10. 恢复路线

- R0：Ubuntu Rust／native 依赖／rust-analyzer 与完整矩阵恢复。已完成。
- R1／Phase 56Q：production nested public path 的真实外部 toplevel lifecycle tracer bullet（已完成窄切片，后续仍需扩大可靠性与长时语义）。
- R2：SHM-first 的 owned buffer、texture readback、render、damage、frame done 最小可见闭环（已完成 bounded controlled proof；未接入默认 main 或长时资源调度）。
- R3：nested keyboard／pointer 输入与焦点、workspace、slot、stack、layout 交互。
- R4：多客户端可靠性、协议错误、session 安全、README／运行说明与产品化。
- R5：nested 可用后再设计 DRM/KMS、GBM、libinput、seat/session、多输出与 XWayland。

## 11. 下一步审批门槛

R1／Phase 56Q 与 R2 bounded controlled proof 已完成。R2 只授权并证明一个外部 XRGB8888 SHM 首帧：资源转移前执行 exact token/session/ledger/Core 原子校验，失败路径按 FIFO 精确回收并 tombstone；成功路径做 GLES import/readback/draw、Winit submit、output damage 与一次 gated frame done，最后 drop owner、join client 并清理 socket/SHM backing file。

下一代码阶段若进入 R3 input、扩大 R2 为长时 renderer／buffer release，或接入默认 main，均必须分别明确批准 goal、精确 allowlist/denylist、正确 Red、最小 Green、timeout/watchdog、cleanup 与 destroy 语义；R2 审批不自动覆盖 input、DRM、dmabuf、多输出、公共 API 或相邻重构。

## 12. 文档权威顺序

1. 实时 Git、源码与 CI。
2. AGENTS.md 的长期工程规则。
3. 本文件的规范化当前状态。
4. PROJECT_HISTORY.md 的演进、旧证据与来源映射。
5. archive/handoff 中保留的 patch／zip 原始交付归档。

历史测试只证明当时提交和环境；每个新阶段必须取得本阶段的新鲜验证。
