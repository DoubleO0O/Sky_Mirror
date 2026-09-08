# Sky Mirror Agent 工程规则

## 1. 项目定位

- Sky Mirror 是以 Rust 与 Smithay 构建的 Linux Wayland 合成器 / 窗口管理器项目。
- 空间模型使用 X / Y / Z 三轴，并以 workspace、slot、stack 组织窗口状态。
- 当前工程由纯数据 Core、受 feature gate 控制的证明路径，以及部分 Linux runtime 组成。
- Core 的职责是表达状态、命令和可验证的不变量，不持有平台资源。
- 测试结果、报告字段、mock 行为和回调观测都不是产品能力。
- 对外描述必须区分“已经存在的代码结构”与“用户可见、可投入生产的能力”。

## 2. 真实性分级

- production fact：真实产品路径已接通，资源、生命周期和失败语义均成立。
- controlled proof：受控环境中的窄范围证明，只证明明确覆盖的链路。
- callback observation：只证明回调被观察到，不证明完整业务结果。
- report / descriptor / readiness：描述状态或准备条件，不代表动作已经完成。
- skeleton：保留结构、接口或控制流，但关键能力仍未接通。
- mock：由替身提供行为，只能证明调用契约或测试编排。
- unimplemented：尚无满足目标的实现。
- 报告、注释、提交说明和 PR 描述必须使用上述分级表达真实状态。
- 禁止把 MockRenderer、DummyRenderer、source guard、回调观测、报告字段、编译通过、单元测试通过、CI 绿色、readiness 或 descriptor 称为 production。
- 证据只能支持其直接覆盖的结论，不得从局部证明外推整体能力。

## 3. 架构边界

- Core 必须保持平台中立，不得持有 Smithay、Wayland 或 Linux 资源。
- 平台事件沿 `BackendEvent -> CoreCommand -> State` 方向进入 Core。
- ObjectId、adapter ID、session ID 与 core ID 必须明确区分并显式转换。
- backend 不得直接修改 workspace、slot、stack、focus 或 registry。
- validator 只负责验证，不等于 rollback，也不得被描述为事务保证。
- owner、cleanup、tombstone 的职责和状态转换必须显式表达。
- 资源创建、注册、注销、销毁和断连必须有唯一责任方。
- bridge / adapter 负责跨边界映射，不得绕过 Core 建立第二条状态真相。
- 错误路径必须保留身份一致性，并说明哪些状态已提交、哪些需要补偿。

## 4. 高风险审批

- 修改 `src/core`、`src/backend`、`src/main.rs` 前必须获得明确审批。
- 修改 Cargo 文件、features、CI 或依赖前必须获得明确审批。
- 接入真实 buffer handoff、import、texture、renderer、damage 或 frame done 前必须获得明确审批。
- 接入真实 input、图形栈、libinput、udev、seat 或 session 前必须获得明确审批。
- 新增会修改 production Core 的路径前必须获得明确审批。
- 扩大公共 API、allowlist 或可执行范围前必须获得明确审批。
- 高风险范围不清楚、审批措辞含糊或证据冲突时，立即停止并询问。
- 审批只覆盖明确列出的目标，不自动延伸到相邻重构或清理。

## 5. Skills 与 MCP

- 开始任务前发现真正适用的 Skills；流程 Skill 必须先于实现 Skill。
- 只使用完成当前任务所需的最小工具集，不因工具可用而扩大范围。
- Codebase Memory 只有在 canonical root、branch 与 HEAD 全部一致时才可使用。
- 任一索引条件不一致即视为 STALE，禁止据此作当前代码结论。
- 代码发现优先顺序为：symbol、call graph、snippet、所需文件片段、完整小文件。
- 禁止为了探索而读取超大完整文件。
- 禁止重复扫描 `smithay_backend` 或反复读取未变化的同一代码。
- 图工具证据不足时，才针对字符串、配置或非代码文件使用窄范围文本搜索。

## 6. Context 与 Token

- 同一 HEAD 下已经读取且未变化的内容不得无目的重读。
- 调查记录至少包含文件、symbol、结论和结论失效条件。
- 成功测试只保留命令、退出码和摘要。
- 失败测试只保留定位问题所需的最小错误、上下文和复现条件。
- 不输出完整日志、完整源码、冗长历史或与决策无关的工具回显。
- 稳定规则写入 `AGENTS.md`，规范化动态事实写入 `PROJECT_GUIDE.md`，历史演进与来源映射写入 `PROJECT_HISTORY.md`。
- 可重复且稳定的工作流在后续获得审批后沉淀为 Skill。
- 不得把本地缓存、工具内部状态或临时运行状态写入 Git 状态文档。

## 7. 开发工作流

- 标准顺序是：确认 baseline。
- 明确最小 scope。
- 列出 allow / deny 边界。
- 建立能够因目标缺失而失败的正确 Red。
- 实现最小 Green。
- 运行针对性验证。
- 审查 diff。
- 获批后运行更广验证。
- 进行独立审查。
- 更新项目状态。
- 最后才进入 commit、PR 与 CI。
- timeout、socket、环境缺失或 harness 故障不得伪装成 Red。
- 失败后不得静默重跑；必须说明失败原因和重新运行的依据。
- 不顺手清理 warning，不做无关重构，不并行推进多条高风险链路。
- 完成声明必须由本轮新鲜、完整且与声明相匹配的验证支撑。

## 8. 中文注释

- public、重要 `pub(crate)`、trait 和 enum 应有准确的中文文档注释。
- owner、registry、ledger、bridge、adapter、constructor、cleanup、destroy、disconnect、rollback 必须解释职责。
- ownership、lifetime、ID、callback、error、deadline、watchdog、kill、reap、tombstone、feature gate 等密集逻辑应使用中文说明。
- 测试应标明准备、执行、断言、Red / Green 依据和 cleanup。
- 注释重点解释为什么、约束、风险与资源责任，而不是机械复述代码。
- 禁止把 proof、report、mock 或 CI 结果注释成 production 能力。
- 注释必须随语义变化更新；失真的旧注释应视为缺陷。

## 9. Git 与环境

- 未获审批不得 reset、clean、force push 或删除未知文件。
- 修改前检查 branch、HEAD、status 与 `origin/main`。
- 新阶段只能在获得审批后创建 worktree。
- Windows Git 负责 Git 与 worktree 管理；WSL / Linux 负责 Smithay 相关验证。
- 不得为修复环境错误而改 production 代码。
- 不得夹带 EOL、`core.autocrlf` 或 `.gitattributes` 变更。
- EOL 策略必须作为独立议题单独审批。
- 不得擅自创建分支、提交、push 或 PR。

## 10. 状态入口

- 开始工作前读取 `PROJECT_GUIDE.md`。
- 具体阶段细节与已删除旧文档的来源映射以 `PROJECT_HISTORY.md` 为补充证据。
- `AGENTS.md` 只承载长期稳定规则，不写当前 HEAD、测试数量或临时 blocker。
- 状态文档与 Git、代码或权威 CI 冲突时，以实时事实为准并标记状态文档过期。
- 过时的阶段文档不得覆盖实时 PR、CI 和代码事实。
- 状态发生变化时，只更新被新证据影响的动态条目。
