//! Sky Mirror compositor 核心模块的统一出口。
//!
//! 这里仅声明并导出各个核心子模块，不保存状态，也不实现业务逻辑。
//! 模块之间通过明确的数据边界串联：
//! 输入先转换为 Action，State 统一修改状态，随后依次生成 Layout、Scene 和 RenderFrame。
//! 保持集中导出可以让程序入口只引用 `core`，同时避免各模块自行组装全局状态。

/// 输入意图对应的语义动作定义。
pub mod action;
/// 后端驱动到核心运行时桥接层的抽象接口。
pub mod backend_driver;
/// 后端 / protocol 事件到 CoreCommand 的纯数据适配层。
pub mod backend_event;
/// 后端事件回放测试器，用于纯数据模拟未来 Smithay 事件序列。
pub mod backend_replay;
/// Wayland client 的纯数据占位模型。
pub mod client;
/// 外部系统进入核心状态的统一命令边界。
pub mod command;
/// compositor 更高层组装逻辑的预留模块。
pub mod compositor;
/// 组合 Inspector 与 Validator 的统一诊断接口。
pub mod diagnostics;
/// 基于 calloop 的系统主事件循环。
pub mod event_loop;
/// workspace、slot 和 window 的显式焦点状态。
pub mod focus;
/// 输入事件与 tick-based 临时输入源。
pub mod input;
/// compositor 内部状态的只读调试快照。
pub mod inspector;
/// 未来真实 backend / Smithay 接入核心状态时必须遵守的边界契约。
pub mod integration_contract;
/// 抽象按键、修饰键和快捷键映射。
pub mod keybinding;
/// Workspace 到窗口矩形 placement 的纯布局计算。
pub mod layout;
/// 当前输出尺寸及 resize 状态。
pub mod output;
/// SceneFrame 到 RenderFrame 的规划与 MockRenderer。
pub mod render;
/// 单个后端事件进入核心状态的运行时桥接层。
pub mod runtime_bridge;
/// 布局结果与焦点合成的纯数据场景图。
pub mod scene;
/// workspace、focus、stack 和窗口 ID 的 session 持久化。
pub mod session;
/// compositor 集中状态与 Action 分发入口。
pub mod state;
/// 未来 Wayland/Smithay surface 与 WindowId 的纯数据绑定占位层。
pub mod surface;
/// compositor 核心状态的只读一致性检查器。
pub mod validator;
/// 逻辑窗口注册表与轻量 metadata。
pub mod window;
/// 固定 slot、窗口 stack 和 workspace 数据模型。
pub mod workspace;
