# Sky Mirror 项目历史与文档来源

> 本文件合并已被取代的阶段计划、审计、恢复记录、Codex 环境说明与交接 README。
> 它保存演进与历史证据，不覆盖 PROJECT_GUIDE.md、AGENTS.md、实时源码或 CI 的当前真值。

## 1. 历史边界

Sky Mirror 的原始 .git 曾在跨机转移后丢失。现有仓库于 2026-06-13 从已接受的源码快照重建，首提交 9f419f1 不能被描述为原始项目历史。Phase 45–47R 的来源通过 archive/handoff 中保留的 patch 与 zip 交付物保存；这些归档继续保留，本文件只吸收其 README 说明。

历史先后跨越 Arch Linux、macOS、Windows、Windows + WSL，当前恢复到 Ubuntu。旧绝对路径、旧插件清单、旧工具版本、旧 branch/HEAD、旧 CI 数量和 phase-local readiness 都是当时快照，不能直接用于当前机器或当前 HEAD。

## 2. Phase 45–47：跨平台交接链

七组交接包都基于缺失历史前的 9348b21 Phase 45.6 基线，由 macOS Darwin arm64 生成，要求在 Arch/Linux 上应用并完成 smithay-linux 验证。每一组 zip 只封装同名 patch 与 README，没有额外独特源码；patch 则是累积式源码差异，因此后组大体包含前组内容。

- Phase 45/46：封住 SmithayRuntimeProbe 旧 Linux API 兼容和真实 Display/socket/XDG_RUNTIME_DIR 测试边界；新增 BackendRuntimeReport、capabilities、diagnostics、runtime facade。生成端没有 Linux 验证，明确不声称通过。
- Phase 47M：加入后端中立的 surface lifecycle、registry、结构化错误与测试。只是真实 adapter 之前的纯数据模型，不持有 wl_surface。
- Phase 47N：加入 surface trace runner、执行报告、mock adapter 与场景测试；保证事件只能经 registry apply_event 推进。
- Phase 47O：加入 surface 到 window 的 candidate intent，只表达候选关系，不创建 Core window。
- Phase 47P：加入 window admission preview，预演是否可接纳，不分配真实 workspace/slot。
- Phase 47Q：把 trace、candidate intent、preview 组合成纯数据 admission pipeline，仍不进入 BackendEvent/Core。
- Phase 47R：加入 admission contract golden snapshot，冻结纯数据行为供未来 adapter 防回归；不证明真实 Wayland 时序。

各 README 反复强调 default=[]、Core/backend 不反向依赖 Smithay 层、不允许 early return 静默跳过 Linux 资源测试、supports_real_wayland_surfaces=false、supports_gpu_rendering=false。后续代码已吸收这些模型，并大幅超过当时能力；原交接应用命令和 Arch 包清单仅具历史意义。

## 3. 2026-06-19：恢复后的结构审计

旧 SKY_PROJECT_AUDIT 在 macOS、Phase 49R、dirty handoff 删除背景下检查了 Core、layout、registry、Smithay probe 与 public API。它当时判断 Core 状态机中等成熟，而真实 nested compositor、surface、render、input、DRM 基本未开始；并警告 linux_handler_probe、gate、evidence、matrix 膨胀会替代真实纵向切片。

审计促成或记录了 focused fullscreen、window lifecycle cleanup、validator 与 diagnostics。它对公开字段、双重 surface 模型、ID alias、mock/proof 测试偏差、缺 README 的技术债仍有参考价值；但“不能 accept client／没有 globals／fullscreen 固定 slot 0”等事实已被后续阶段推翻，旧路径、测试数、dirty 状态也失效。

## 4. Phase 51：nested client/runtime 基线

- Phase 51A 先设计最小 nested client connection 切片：Core 已有 client lifecycle seam，未来 Linux callback 必须转为 BackendEvent/CoreCommand；规划 session ID 与 core ClientId 映射、真实 accept/insert、disconnect、validation 与 feature 隔离，禁止借机改 Core/Cargo。
- Phase 51N 审计合并后的 main：client session、socket accept probe、real accept/disconnect bridge、single-pump coordinator、bounded loop、external stop/wakeup、start/run/stop orchestrator 已形成。它证明窄范围 Linux lifecycle，但不等于长时 compositor、surface、render 或 input。

## 5. Phase 52：Surface/XDG 与 admission 演进

- 52A：建立纯数据 adapter identity ledger，让 surface/xdg-toplevel intent 经既有 BackendEvent -> CoreCommand -> State 接纳；拒绝 duplicate、orphan、stale。
- 52C：先补 Core toplevel unmap/detach seam；移除 window 活跃状态并清链接，但保持 surface alive。
- 52B-B：在 ledger 中实现安全 unmap；成功 Core detach 后才移除 toplevel mapping，并以 tombstone 区分重复与未知 identity。
- 52D：文档审计确认当时没有真实 xdg-shell global、handler 或 callback source，因此不伪造 callback API。
- 52E：加入 Linux-only xdg-shell compile seam，证明 Smithay trait/type 形状，不声称 runtime 已启动。
- 52F：选择 Wayland ObjectId 作为稳定 protocol key，建立 adapter-owned toplevel identity registry 与单调 AdapterToplevelId。
- 52G：把 XdgShellHandler::toplevel_destroyed 接到只读 identity lookup observation；不调用 ledger/Core。
- 52H：readiness-only 阶段，记录真实 runtime callback proof 仍缺 global、client、dispatch 与 registration owner。
- 52I：让 Display owner 可显式初始化 xdg_wm_base global，只解决 global owner 前置条件。
- 52J：设计 controlled client toplevel harness，但选择文档路线，不创建 client 或 protocol object。
- 52K：评估并建议 Linux-only wayland-client/wayland-protocols 依赖，保留未来审批门。
- 52L：真正加入 optional client crates 与 compile/import seam；只证明依赖和类型可编译。
- 52M：先以文档确认 wl_compositor owner 所需 Smithay API 与 blocker。
- 52M-B：实现 Linux-only CompositorState owner 和 per-client CompositorClientState seam，但不创建 client/surface。
- 52N：受控 Unix endpoint 上真实 bind wl_compositor；不连接系统 Wayland socket。
- 52O：受控 client 创建 wl_surface，并由 adapter 分配 surface identity；不进 XDG/Core。
- 52P：受控 client bind xdg_wm_base；仍不创建 xdg_surface。
- 52Q：受控创建 xdg_surface，保留 wl_surface 与 XDG role 对应关系。
- 52R：受控创建 xdg_toplevel，并驱动 request/dispatch；不接 ledger/Core。
- 52S：观察真实 new_toplevel callback，只证明 callback 到达。
- 52T：callback 中注册 adapter toplevel identity，把真实 ToplevelSurface 关联到 AdapterSurfaceId；仍不 admit。
- 52U：adapter owner 将已注册 toplevel identity提交到 admission ledger；证明 owner 位置，不等于 production path。
- 52V：对应源码提交建立 live callback 到 pending ledger admission 的 bridge；仓库没有单独 52V 文档，历史由相邻 52U/52W 与 Git 提交补足。
- 52W：pending admission consumer owner 消费 intent，并调用 ledger admission。
- 52X：controlled admission pump 把 producer/consumer 串联；仍不是 production protocol pump。
- 52Y：让 runtime 拥有 pending admission queue。
- 52Z：bounded nested loop drain admission queue，形成 loop report；不宣称长时桌面。

## 6. Phase 53：live admission、backlog 与 unmap

- 53A：将 live new_toplevel observation 的 owner seam 接到 coordinator queue。
- 53B：每次 coordinator pump 可推进 live admission。
- 53C：bounded loop 通过 live admission pump 推进，而不是测试直接调用 consumer。
- 53D：orchestrator 层证明受控 live admission path 可贯穿 start/run/report。
- 53E：在 lifecycle report 暴露 admission run summary；report 不等于新 production API。
- 53F：对重复 callback/identity 做去重，避免重复 admission。
- 53G：证明多个 distinct live admissions 的顺序与独立身份。
- 53H：保存未及时消费的 callback backlog，不丢 observation。
- 53I：只要 backlog 存在，loop 不应错误判 idle。
- 53J：orchestrator 证明 idle 边界也会 drain backlog。
- 53K：live toplevel destroy/unmap owner 调用 ledger unmap。
- 53L：bounded loop drain live unmap。
- 53M：orchestrator report 暴露 live unmap 结果；仍非完整 production lifecycle。

## 7. Phase 54：surface commit 到渲染意图

- 54A：观察 wl_surface.commit，只记录 callback-like evidence，不 render。
- 54B：将 commit observation 放入 backlog，避免异步时序丢失。
- 54C–54Q 没有独立文档，Git 历史显示依次加入 runtime drain、buffer presence、damage、frame callback observation、render-dirty intent/queue、renderer admission、owner/readiness shells、render operation intent/queue 与 execution owner；这些阶段共同把真实 commit 证据变成纯数据渲染工作描述。
- 54R：总审计确认链路只到 readiness/report；缺真实 renderer、buffer owner/import、texture、damage submit 与 frame done，并建议后续只做最小安全切入。

## 8. Phase 55：buffer import 前置链

- 55A 没有独立文档；Git 提交加入 basic render pipeline skeleton，仍无执行。
- 55B：报告当前 backend capability，明确 renderer/import/texture 为 false。
- 55C：renderer backend registration descriptor，只描述未来选择与注册。
- 55D：renderer backend owner shell，只占据 owner 位置，不持有真实 backend。
- 55E：buffer importer resource owner boundary，定义 handoff 与 cleanup 责任。
- 55F：planning report 区分 candidate evidence 与 actual import requirement。
- 55G：implementation descriptor 描述未来 importer adapter 与失败语义。
- 55H：adapter proof boundary 验证 report/handoff 契约，不 import。
- 55I：precondition gate 汇总是否允许进入 import；当前仍 blocked。
- 55J：execution dry-run/no-op 解释如果执行会在哪些 blocker 停止。
- 55K：implementation owner shell 保存 future importer 所需证据。
- 55L：actual attempt record 只记录“是否应尝试”，仍不进行尝试。
- 55M：总审计明确 55E–55L 全是准备链，没有 real buffer import。
- 55N：比较 SHM、dmabuf、EGL/GLES、WGPU 等路线，推荐 SHM-first nested MVP；只是路线建议，要求 Phase 56A 另行授权。

## 9. Phase 56：SHM/texture/renderer 决策到 protocol bootstrap

- 56A：Linux-only SHM-first buffer adapter skeleton，允许引用真实 WlBuffer 类型，但不读取内容、不 import。
- 56B：提取或分类 SHM metadata evidence（offset/size/stride/format 等），仍不 import。
- 56C：细化 unavailable/unsupported/blocked metadata taxonomy。
- 56D：用纯数据 harness 验证 metadata 与 blocker 路径，证明 evidence 不等于执行。
- 56E：审计 texture creation 所需 renderer instance、import route、damage 与 frame policy；全部仍缺。
- 56F：texture creation no-op skeleton；即使到达 execution boundary 也明确阻断。
- 56G：定义未来 texture request、handle 与 cleanup owner，不创建 texture。
- 56H：审计 renderer backend instance 前置条件，仍是 pure-data report。
- 56I：决定未来 texture import route 与 owner，不调用 import API。
- 56J：审计 commit damage 到 texture coordinate/damage submission 的映射责任。
- 56K：定义 frame callback 只有在相应 render 完成后才可 done 的 policy。
- 56L：汇总真实 texture creation readiness，结论仍 blocked。
- 56M：定义真实 renderer backend instance 的未来 owner/lifetime/error 边界。
- 56N：选择 concrete renderer candidate 与 construction route，仍是决策。
- 56O：在 adapter owner 内构造并保存 Smithay DummyRenderer，证明 type/owner/storage/drop；buffer_imported、texture_created、renderer_called、damage_submitted、frame_done、input、core mutation 全为 false。
- 56P：关键转折。production NestedRuntimeOrchestrator::start() 改走 production constructor；严格按 Display -> wl_compositor -> xdg_wm_base -> socket -> calloop source 初始化。外部有界 Wayland client 真实 discovery/bind 两个 globals；server report 不冒充 client observation。加入 deadline、poll cap、scoped join、outer watchdog、kill/reap 与 socket/runtime cleanup。Phase 在 global bootstrap 结束，明确禁止创建 wl_surface/xdg_surface/xdg_toplevel 和所有 render/input/Core mutation。PR #92 后来已合并；原文末尾 push/PR/CI/merge PENDING 是提交前历史状态。
- 56Q：在明确授权的 narrow tracer 中继续真实 external Wayland lifecycle。production client 创建 wl_surface -> xdg_surface -> xdg_toplevel，处理 initial configure/ack 与无 buffer commit；server handler 只记录 callback/identity observation 并发送 configure，coordinator 通过 active session -> Core client bridge 将 admission intent 入队。真实 xdg_toplevel destroy 由 live unmap owner 消费，Core window detach、registry tombstone 成立，同时验证 wl_surface 在 toplevel destroy 后仍存活，随后完成 surface destroy 与 socket cleanup。该证据是 bounded production fact，不等于 long-running compositor；buffer/import/texture/renderer/damage/frame done/input/DRM 仍明确未接入，真实 disconnect callback 的独立证明仍作为边界证据保留。
- R2 SHM-first controlled proof：在 56Q production socket/lifecycle 之上，external client 提交带 damage 与 frame request 的 2×2 XRGB8888 WlBuffer。新增 coordinator 原子 admission owner，在资源仍位于 display FIFO 时验证 source session -> Core client、adapter surface/toplevel、ledger surface/window、live Core identity 与 exact commit token；未知、已死、销毁、交错、重复或迟到身份按单侧／双侧精确回收并留下 tombstone。进程主线程的唯一 Winit/EGL/GLES owner 完成真实 SHM import、metadata→texture dimensions/format cross-check、texture readback、texture draw、backbuffer/output-damage submit；回读红／绿／蓝／白像素与 client 图样一致。独立 completion gate 只在 presentation、identity、damage 与 callback 数量全部一致时发送一次 frame done。client read 与 write 均由独立 fd readiness、同一绝对 deadline 与固定 poll cap 驱动；无论 worker 成功、失败、超时或断连，runner 都先 drop server owner、join client、检查 socket，再返回结果，cleanup destroy request 同样有界 flush。它仍只是单 buffer 的 bounded controlled proof：未接入默认 main，不含长时 buffer replacement/release、surface-tree、完整 damage tracking、input、DRM/KMS、dmabuf 或多输出。

## 10. Codex 环境与恢复文档历史

### 10.1 旧环境规则

CODEX_ENVIRONMENT_RULES 与 CODEX_PHASE_PROMPT_TEMPLATE 固定了 2026-06 macOS 环境：主仓 /Users/double/Code/Sky_Mirror、旧仓 /Users/double/sky_mirror、特定 Codebase Memory 项目、Headroom/RTK、worktree 与插件操作顺序。它们的可复用原则是先核对 main/HEAD/status、MCP canonical root、按 allowlist 工作、Red/Green、Linux 验证与不夸大能力；绝对路径、插件版本、索引名和主机分工现已过时。

### 10.2 Codex rebuild report

CODEX_REBUILD_REPORT 记录 2026-06-24 在 macOS 将活动仓库迁到 /Users/double/Code/Sky_Mirror、重建 Codebase Memory、核对插件/skills/Headroom/RTK/worktree，并把当时最新阶段识别为 52U。它是环境恢复审计，不是产品规格；当前 Ubuntu、Phase 56P 和新的工具链已取代其状态结论。

### 10.3 repository recovery notes

RECOVERY_NOTES 记录 transferred working tree 缺失 .git、预期旧基线 9348b21、Phase 45–47R 内容已存在、Phase 48A 在 Arch 接受，以及重建历史不得冒充原始历史。它当时列出的无 real surface/render/runtime 能力是恢复时快照，后续已有部分 protocol bootstrap，但仍没有可见 renderer/input/desktop。

### 10.4 当前状态文档

旧 docs/ai/PROJECT_STATE 在 2026-07-30 将最近完成阶段定为 56P、候选 56Q NOT APPROVED，并列出 production/controlled/mock/unimplemented 边界。其当前有效内容已规范化进入 PROJECT_GUIDE；“Phase 56P release 仍 PENDING”保留为历史冲突说明。

## 11. 当前结论与后续路线的形成

历史共同说明：项目从纯数据 Core 与大量 proof/readiness，逐步到达真实 production socket/global bootstrap、56Q external XDG 窄生命周期，以及 R2 单帧 SHM import/readback/draw/submit/frame-done 的 bounded controlled proof。默认 main、长时 renderer/resource lifecycle、input 与 DRM 仍断开。继续开发应停止新增不服务纵向切片的 descriptor/report，并分别审批输入、可靠性／长期 buffer release、产品入口和最终 DRM session。规范化当前路线见 PROJECT_GUIDE。

## 12. Source provenance：86/86

下表每行对应一个被本次整合取代并删除的说明文件。目标章节给出其独特信息的归宿；patch 与 zip 不在删除集合中。

| # | 原路径 | 吸收章节 |
|---:|---|---|
| 1 | archive/handoff/phase45_46/phase45_46_handoff_README.txt | 1, 2 |
| 2 | archive/handoff/phase45_47m/phase45_47m_handoff_README.txt | 1, 2 |
| 3 | archive/handoff/phase45_47n/phase45_47n_handoff_README.txt | 1, 2 |
| 4 | archive/handoff/phase45_47o/phase45_47o_handoff_README.txt | 1, 2 |
| 5 | archive/handoff/phase45_47p/phase45_47p_handoff_README.txt | 1, 2 |
| 6 | archive/handoff/phase45_47q/phase45_47q_handoff_README.txt | 1, 2 |
| 7 | archive/handoff/phase45_47r/phase45_47r_handoff_README.txt | 1, 2 |
| 8 | docs/CODEX_ENVIRONMENT_RULES.md | 1, 10.1 |
| 9 | docs/CODEX_PHASE_PROMPT_TEMPLATE.md | 10.1 |
| 10 | docs/CODEX_REBUILD_REPORT.md | 10.2 |
| 11 | docs/ai/PROJECT_STATE.md | 10.4, PROJECT_GUIDE |
| 12 | docs/audit/SKY_PROJECT_AUDIT.md | 3, 11 |
| 13 | docs/phases/PHASE_51A_NESTED_CLIENT_CONNECTION_PLAN.md | 4 |
| 14 | docs/phases/PHASE_51N_MAIN_BASELINE_AUDIT.md | 4 |
| 15 | docs/phases/PHASE_52A_SURFACE_XDG_ADMISSION_PLAN.md | 5 |
| 16 | docs/phases/PHASE_52B_B_SURFACE_XDG_LEDGER_REMOVAL_PLAN.md | 5 |
| 17 | docs/phases/PHASE_52C_CORE_TOPLEVEL_UNMAP_DETACH_PLAN.md | 5 |
| 18 | docs/phases/PHASE_52D_LINUX_XDG_TOPLEVEL_UNMAP_CALLBACK_PLAN.md | 5 |
| 19 | docs/phases/PHASE_52E_LINUX_XDG_SHELL_COMPILE_SEAM_PLAN.md | 5 |
| 20 | docs/phases/PHASE_52F_XDG_TOPLEVEL_IDENTITY_MAPPING_PLAN.md | 5 |
| 21 | docs/phases/PHASE_52G_XDG_LIFECYCLE_CALLBACK_IDENTITY_LOOKUP_PLAN.md | 5 |
| 22 | docs/phases/PHASE_52H_RUNTIME_CALLBACK_OBSERVED_PROOF_PLAN.md | 5 |
| 23 | docs/phases/PHASE_52I_XDG_SHELL_GLOBAL_OWNER_BOUNDARY_PLAN.md | 5 |
| 24 | docs/phases/PHASE_52J_CONTROLLED_CLIENT_TOPLEVEL_HARNESS_PLAN.md | 5 |
| 25 | docs/phases/PHASE_52K_WAYLAND_CLIENT_ENDPOINT_DECISION.md | 5 |
| 26 | docs/phases/PHASE_52L_LINUX_CLIENT_COMPILE_SEAM_PLAN.md | 5 |
| 27 | docs/phases/PHASE_52M_B_LINUX_WL_COMPOSITOR_STATE_OWNER_PLAN.md | 5 |
| 28 | docs/phases/PHASE_52M_LINUX_WL_COMPOSITOR_OWNER_PLAN.md | 5 |
| 29 | docs/phases/PHASE_52N_CONTROLLED_WL_COMPOSITOR_BIND_PROOF_PLAN.md | 5 |
| 30 | docs/phases/PHASE_52O_CONTROLLED_WL_SURFACE_IDENTITY_PROOF_PLAN.md | 5 |
| 31 | docs/phases/PHASE_52P_CONTROLLED_XDG_WM_BASE_BIND_PROOF_PLAN.md | 5 |
| 32 | docs/phases/PHASE_52Q_CONTROLLED_XDG_SURFACE_CREATION_PROOF_PLAN.md | 5 |
| 33 | docs/phases/PHASE_52R_CONTROLLED_XDG_TOPLEVEL_CREATION_PROOF_PLAN.md | 5 |
| 34 | docs/phases/PHASE_52S_NEW_TOPLEVEL_CALLBACK_OBSERVATION_PROOF_PLAN.md | 5 |
| 35 | docs/phases/PHASE_52T_ADAPTER_TOPLEVEL_IDENTITY_REGISTRATION_PLAN.md | 5 |
| 36 | docs/phases/PHASE_52U_LEDGER_ADMISSION_OWNER_PLAN.md | 5 |
| 37 | docs/phases/PHASE_52W_PENDING_ADMISSION_CONSUMER_OWNER_PLAN.md | 5 |
| 38 | docs/phases/PHASE_52X_CONTROLLED_ADMISSION_PUMP_PLAN.md | 5 |
| 39 | docs/phases/PHASE_52Y_RUNTIME_ADMISSION_QUEUE_OWNER_PLAN.md | 5 |
| 40 | docs/phases/PHASE_52Z_NESTED_RUNTIME_ADMISSION_DRAIN_PLAN.md | 5 |
| 41 | docs/phases/PHASE_53A_LIVE_CALLBACK_ADMISSION_OWNER_PLAN.md | 6 |
| 42 | docs/phases/PHASE_53B_LIVE_ADMISSION_PUMP_SEAM_PLAN.md | 6 |
| 43 | docs/phases/PHASE_53C_LOOP_LIVE_ADMISSION_PUMP_PLAN.md | 6 |
| 44 | docs/phases/PHASE_53D_ORCHESTRATOR_LIVE_ADMISSION_PROOF.md | 6 |
| 45 | docs/phases/PHASE_53E_LIVE_ADMISSION_RUN_REPORT.md | 6 |
| 46 | docs/phases/PHASE_53F_LIVE_ADMISSION_CALLBACK_DEDUPE.md | 6 |
| 47 | docs/phases/PHASE_53G_MULTIPLE_LIVE_ADMISSIONS.md | 6 |
| 48 | docs/phases/PHASE_53H_LIVE_OBSERVATION_BACKLOG.md | 6 |
| 49 | docs/phases/PHASE_53I_LOOP_IDLE_LIVE_BACKLOG.md | 6 |
| 50 | docs/phases/PHASE_53J_ORCHESTRATOR_IDLE_BACKLOG.md | 6 |
| 51 | docs/phases/PHASE_53K_LIVE_UNMAP_OWNER.md | 6 |
| 52 | docs/phases/PHASE_53L_LOOP_LIVE_UNMAP_DRAIN.md | 6 |
| 53 | docs/phases/PHASE_53M_ORCHESTRATOR_LIVE_UNMAP_REPORT.md | 6 |
| 54 | docs/phases/PHASE_54A_WL_SURFACE_COMMIT_OBSERVATION.md | 7 |
| 55 | docs/phases/PHASE_54B_WL_SURFACE_COMMIT_BACKLOG.md | 7 |
| 56 | docs/phases/PHASE_54R_RENDER_PIPELINE_READINESS_AUDIT.md | 7 |
| 57 | docs/phases/PHASE_55B_RENDER_BACKEND_CAPABILITY_REPORT.md | 8 |
| 58 | docs/phases/PHASE_55C_RENDERER_BACKEND_REGISTRATION_DESCRIPTOR.md | 8 |
| 59 | docs/phases/PHASE_55D_RENDERER_BACKEND_OWNER_SHELL.md | 8 |
| 60 | docs/phases/PHASE_55E_BUFFER_IMPORT_RESOURCE_OWNER_BOUNDARY.md | 8 |
| 61 | docs/phases/PHASE_55F_BUFFER_IMPORT_PLANNING_REPORT.md | 8 |
| 62 | docs/phases/PHASE_55G_BUFFER_IMPORT_IMPLEMENTATION_DESCRIPTOR.md | 8 |
| 63 | docs/phases/PHASE_55H_BUFFER_IMPORT_ADAPTER_PROOF_BOUNDARY.md | 8 |
| 64 | docs/phases/PHASE_55I_BUFFER_IMPORT_PRECONDITION_GATE.md | 8 |
| 65 | docs/phases/PHASE_55J_BUFFER_IMPORT_EXECUTION_DRY_RUN.md | 8 |
| 66 | docs/phases/PHASE_55K_BUFFER_IMPORT_IMPLEMENTATION_OWNER_SHELL.md | 8 |
| 67 | docs/phases/PHASE_55L_BUFFER_IMPORT_ACTUAL_ATTEMPT_RECORD.md | 8 |
| 68 | docs/phases/PHASE_55M_REAL_BUFFER_IMPORT_BOUNDARY_AUDIT.md | 8 |
| 69 | docs/phases/PHASE_55N_REAL_IMPORT_ROUTE_DECISION_MATRIX.md | 8 |
| 70 | docs/phases/PHASE_56A_SHM_FIRST_BUFFER_IMPORT_ADAPTER_SKELETON.md | 9 |
| 71 | docs/phases/PHASE_56B_SHM_BUFFER_METADATA_EVIDENCE.md | 9 |
| 72 | docs/phases/PHASE_56C_SHM_METADATA_BLOCKER_REFINEMENT.md | 9 |
| 73 | docs/phases/PHASE_56D_SHM_METADATA_VALIDATION_HARNESS.md | 9 |
| 74 | docs/phases/PHASE_56E_TEXTURE_CREATION_PRECONDITION_AUDIT.md | 9 |
| 75 | docs/phases/PHASE_56F_TEXTURE_CREATION_NOOP_SKELETON.md | 9 |
| 76 | docs/phases/PHASE_56G_TEXTURE_OWNER_BOUNDARY.md | 9 |
| 77 | docs/phases/PHASE_56H_RENDERER_BACKEND_INSTANCE_AUDIT.md | 9 |
| 78 | docs/phases/PHASE_56I_TEXTURE_IMPORT_ROUTE_DECISION.md | 9 |
| 79 | docs/phases/PHASE_56J_DAMAGE_TO_TEXTURE_MAPPING_AUDIT.md | 9 |
| 80 | docs/phases/PHASE_56K_FRAME_CALLBACK_COMPLETION_POLICY.md | 9 |
| 81 | docs/phases/PHASE_56L_REAL_TEXTURE_CREATION_READINESS_DECISION.md | 9 |
| 82 | docs/phases/PHASE_56M_REAL_RENDERER_BACKEND_OWNER_BOUNDARY.md | 9 |
| 83 | docs/phases/PHASE_56N_RENDERER_BACKEND_CONCRETE_ROUTE_DECISION.md | 9 |
| 84 | docs/phases/PHASE_56O_RENDERER_BACKEND_CONSTRUCTION_ROUTE_PROOF.md | 9 |
| 85 | docs/phases/PHASE_56P_PRODUCTION_NESTED_PROTOCOL_GLOBAL_BOOTSTRAP.md | 9 |
| 86 | docs/recovery/RECOVERY_NOTES.md | 1, 10.3 |

Coverage：待删除说明文件 86；provenance 条目 86；未映射 0；重复映射 0。

## 13. 原始交接归档审计与删除覆盖

本轮逐个读取了 7 个 patch 和 7 个 zip。7 个 zip 均为标准 Zip archive，内容严格只有同名 patch 与已被本文件吸收的同名 README；每个 zip 内 patch 与外部同名 patch 逐字节一致，没有额外源码、文档、二进制或隐藏文件。因此 zip 的唯一独特历史信息是 README，patch 的源码差异由当前源码、Git 历史和本文件的阶段章节共同覆盖。7 组交接包都在 Darwin arm64（aarch64-apple-darwin）生成，基于缺失历史前的 9348b21 Phase 45.6 基线；生成端没有运行 smithay-linux，不能把它们的 Mac 验证写成 Linux 通过。

逐包覆盖关系如下；表中 14 个路径是本轮获批删除清单，删除前每一项都已完成历史吸收核对：

| 归档包 | patch 独特覆盖 | README 独特事实 | zip 处理 |
|---|---|---|---|
| phase45_46 | Phase 45 Final Seal 的 Display/socket/XDG_RUNTIME_DIR 资源测试与旧 runtime API 兼容；Phase 46 runtime report/capabilities/diagnostics/facade | 9348b21 基线、Darwin 生成、未运行 smithay-linux、Arch/Linux 验收命令和 default=[] 边界 | patch 差异已由第 2 节与当前源码覆盖；zip README 已吸收 |
| phase45_47m | 在 45/46 之上加入 Mac-safe 纯数据 surface lifecycle、registry、结构化错误 | 明确不保存 wl_surface、不接 xdg_toplevel、不进 GPU/compositor；Linux 阻塞未解除 | 同上 |
| phase45_47n | 在 47M 之上加入 surface trace runner、执行报告、mock adapter、scenario | supports_surface_lifecycle_boundary/trace_harness=true，real_wayland/gpu=false；trace 仍只经 registry apply_event | 同上 |
| phase45_47o | 在 47N 之上加入 surface→window candidate intent 纯数据规划 | candidate intent 不进 Core、不构造 BackendEvent/CoreCommand、不修改 workspace/slot/stack/focus | 同上 |
| phase45_47p | 在 47O 之上加入 window admission preview 纯数据预检 | preview 不创建 Core window、不分配真实 workspace/slot；Mac default/probe 验证 131/344 | 同上 |
| phase45_47q | 在 47P 之上加入 surface admission pipeline | pipeline 只编排 trace、candidate intent、preview，不进 Core；Mac probe 验证 383 | 同上 |
| phase45_47r | 在 47Q 之上加入 surface admission contract golden snapshot | golden scenario 只冻结纯数据模型，不代表真实 Wayland 时序；Mac probe 验证 413 | 同上 |

归档删除 provenance（14/14）：

| # | 已删除归档路径 | 覆盖去处 |
|---:|---|---|
| 1 | archive/handoff/phase45_46/phase45_46_handoff.patch | 本节 phase45_46 行及第 2 节 |
| 2 | archive/handoff/phase45_46/phase45_46_handoff.zip | 本节 phase45_46 行；zip 内 README 已吸收 |
| 3 | archive/handoff/phase45_47m/phase45_47m_handoff.patch | 本节 phase45_47m 行及第 2 节 |
| 4 | archive/handoff/phase45_47m/phase45_47m_handoff.zip | 本节 phase45_47m 行；zip 内 README 已吸收 |
| 5 | archive/handoff/phase45_47n/phase45_47n_handoff.patch | 本节 phase45_47n 行及第 2 节 |
| 6 | archive/handoff/phase45_47n/phase45_47n_handoff.zip | 本节 phase45_47n 行；zip 内 README 已吸收 |
| 7 | archive/handoff/phase45_47o/phase45_47o_handoff.patch | 本节 phase45_47o 行及第 2、5 节 |
| 8 | archive/handoff/phase45_47o/phase45_47o_handoff.zip | 本节 phase45_47o 行；zip 内 README 已吸收 |
| 9 | archive/handoff/phase45_47p/phase45_47p_handoff.patch | 本节 phase45_47p 行及第 2、5 节 |
| 10 | archive/handoff/phase45_47p/phase45_47p_handoff.zip | 本节 phase45_47p 行；zip 内 README 已吸收 |
| 11 | archive/handoff/phase45_47q/phase45_47q_handoff.patch | 本节 phase45_47q 行及第 2、5 节 |
| 12 | archive/handoff/phase45_47q/phase45_47q_handoff.zip | 本节 phase45_47q 行；zip 内 README 已吸收 |
| 13 | archive/handoff/phase45_47r/phase45_47r_handoff.patch | 本节 phase45_47r 行及第 2、5 节 |
| 14 | archive/handoff/phase45_47r/phase45_47r_handoff.zip | 本节 phase45_47r 行；zip 内 README 已吸收 |

所有包的共同限制是：不能通过删除 feature、early return、静默跳过或放松断言来掩盖 Linux 资源失败；default feature 必须保持空集；Core/backend 不得依赖这些 runtime facade 或纯数据预览类型；真实 wl_surface、xdg_toplevel、GPU 渲染和输入均未由这些包完成。当前 Ubuntu 的后续验证已由本仓库新鲜 all-features 测试取代历史 Mac 计数，但不改变这些包当时的历史边界。

本轮删除以下 14 个原始交接归档文件；删除后 patch/zip 不再作为当前仓库入口：

- archive/handoff/phase45_46/phase45_46_handoff.patch 与 .zip
- archive/handoff/phase45_47m/phase45_47m_handoff.patch 与 .zip
- archive/handoff/phase45_47n/phase45_47n_handoff.patch 与 .zip
- archive/handoff/phase45_47o/phase45_47o_handoff.patch 与 .zip
- archive/handoff/phase45_47p/phase45_47p_handoff.patch 与 .zip
- archive/handoff/phase45_47q/phase45_47q_handoff.patch 与 .zip
- archive/handoff/phase45_47r/phase45_47r_handoff.patch 与 .zip

这些归档的完整历史信息已由本节及第 2、4、5 节吸收；它们不是当前 main 的应用说明，除非从外部备份专门做历史复原，不应在现仓再次 git apply。
