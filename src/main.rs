//! Sky Mirror compositor 原型的程序入口。
//!
//! 入口层只负责组装全局状态、尝试恢复会话并启动事件循环。
//! workspace、focus、layout 等业务状态不会在这里直接修改，而是统一交由
//! `State` 和 `CompositorState` 管理。

mod backend;
mod core;
// Feature boundary: 默认构建不编译 Smithay 集成层；probe 与 Linux 资源路径由各自
// feature 选择，程序入口本身不启动或调用这些探针。
#[cfg(any(feature = "smithay-probe", feature = "smithay-linux"))]
mod smithay_backend;

use core::event_loop::EventLoop;

/// 创建 compositor 全局状态并进入主事件循环。
fn main() {
    // 提前输出启动日志，便于区分初始化阶段与后续事件循环日志。
    println!("[Sky Mirror] Starting...");

    // 创建全局根状态。
    //
    // `State::new()` 会建立默认 workspace、窗口注册表以及当前 MVP 使用的测试窗口。
    // 如果随后成功恢复 session，这些纯数据状态会被恢复内容替换。
    let mut state = core::state::State::new();

    // 尝试从固定路径恢复上次会话。
    //
    // 加载失败不是致命错误：首次启动、文件缺失或内容无效时继续使用默认状态，
    // 从而保证 compositor 仍可进入事件循环。
    if let Err(error) = state.load_session("sky_mirror_session.json") {
        println!("[Session] Using default state: {}", error);
    }

    // 创建 calloop 驱动的事件循环。
    //
    // 事件循环创建失败意味着 compositor 无法调度输入和渲染流程，因此直接终止。
    let mut event_loop = EventLoop::new().expect("failed to create compositor event loop");

    // 将唯一的全局可变状态借给事件循环运行。
    //
    // EventLoop 本身只负责调度；业务状态修改仍通过 `State::dispatch_action()` 完成。
    event_loop
        .run(&mut state)
        .expect("compositor event loop failed");
}
