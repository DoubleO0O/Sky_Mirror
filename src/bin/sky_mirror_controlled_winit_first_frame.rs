//! R2 受控 production socket→external SHM client→Winit/EGL/GLES 首帧 runner。
//!
//! 此 binary 只用于有界验证，不是 Sky Mirror 日常入口。Winit target 与 coordinator 在
//! 进程主线程创建；外部 Wayland client 在独立线程提交 256×256 XRGB8888 四象限
//! （红、绿、蓝、白）buffer、damage 与 frame request。成功只代表该窄路径的 controlled
//! proof，不外推 DRM/input/多输出能力。

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
#[path = "../backend/mod.rs"]
mod backend;
#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
#[path = "../core/mod.rs"]
mod core;
#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
#[path = "../smithay_backend/mod.rs"]
mod smithay_backend;

/// 受控 SHM 四象限图案的唯一真源：width、height、stride、pool/file 长度、damage
/// 尺寸与像素回读期望全部由这里派生；单帧与 sustained 路径不得各自硬编码尺寸。
///
/// XRGB8888 小端内存顺序为 B/G/R/X，X 不承载 alpha、统一写 0x00。backing file 长度
/// 必须恰为 `STRIDE * HEIGHT`，否则服务端 metadata 校验或 GLES 回读长度校验会失败。
#[cfg(any(feature = "smithay-linux", test))]
mod shm_pattern {
    /// 图案宽度（像素）；同时用于 wl_shm buffer width 与 damage_buffer 宽度。
    pub(crate) const WIDTH: i32 = 256;
    /// 图案高度（像素）；同时用于 wl_shm buffer height 与 damage_buffer 高度。
    pub(crate) const HEIGHT: i32 = 256;
    /// 每像素字节数：XRGB8888 固定 4。
    const BYTES_PER_PIXEL: i32 = 4;
    /// 每行字节数 = width × 4；同时用作 wl_shm buffer stride。
    pub(crate) const STRIDE: i32 = WIDTH * BYTES_PER_PIXEL;
    /// SHM pool size 与 backing file 长度，恰为 stride × height。
    pub(crate) const BYTE_LEN: i32 = STRIDE * HEIGHT;

    /// 四象限像素（B, G, R, X）：行主序左上红、右上绿、左下蓝、右下白。
    const RED: [u8; 4] = [0x00, 0x00, 0xFF, 0x00];
    const GREEN: [u8; 4] = [0x00, 0xFF, 0x00, 0x00];
    const BLUE: [u8; 4] = [0xFF, 0x00, 0x00, 0x00];
    const WHITE: [u8; 4] = [0xFF, 0xFF, 0xFF, 0x00];

    /// 生成完整四象限图案字节，长度恒为 `BYTE_LEN`（= stride × height）。
    pub(crate) fn bytes() -> Vec<u8> {
        let width = WIDTH as usize;
        let height = HEIGHT as usize;
        let mut data = Vec::with_capacity(BYTE_LEN as usize);
        for y in 0..height {
            for x in 0..width {
                let pixel = match (y < height / 2, x < width / 2) {
                    (true, true) => RED,
                    (true, false) => GREEN,
                    (false, true) => BLUE,
                    (false, false) => WHITE,
                };
                data.extend_from_slice(&pixel);
            }
        }
        data
    }

    /// 由 `bytes()` 派生的 client top-to-bottom RGB 回读期望（B,G,R,X -> R,G,B）。
    ///
    /// 运行时 GLES 回读按同一行主序输出 width × height 个像素；两处门禁直接比较
    /// 完整向量，从而验证整幅图案而不只是个别采样点。
    pub(crate) fn expected_readback_rgb() -> Vec<[u8; 3]> {
        bytes()
            .chunks_exact(4)
            .map(|pixel| [pixel[2], pixel[1], pixel[0]])
            .collect()
    }
}

/// 本次 R2 运行的确切资源残留核对：socket、wayland-server `<socket>.lock` 与 client SHM。
///
/// 该核对只读检查、从不删除任何文件：删除责任始终归属各自 owner（coordinator 关闭
/// socket、wayland-server 关闭 `.lock`、`ShmBackingFile::drop` 删除 SHM）。核对结果只
/// 证明这些**精确文件名**在运行结束时不存在，不按前缀计数，也不外推目录里其它文件
/// （例如宿主自己的 `wayland-1`）的归属。
#[cfg(any(feature = "smithay-linux", test))]
mod residue {
    use std::{fs, path::Path};

    /// 只核对本次运行确切拥有的资源是否回收，绝不按宽泛前缀计数或误认用户已有文件。
    ///
    /// - 目录读取失败（含单项读取失败）必须算失败，不得静默当作“无残留”。
    /// - 只比对本进程 socket 精确文件名、`<socket>.lock` 精确文件名与本客户端 SHM
    ///   精确文件名；`<socket>.lock` 命名必须与 wayland-server 的
    ///   `socket_path.with_extension("lock")` 保持同源，避免两套推导产生漏检。
    /// - `shm_path` 为 `None` 表示本次运行确定创建过 SHM 却没取得其精确路径，必须
    ///   失败关闭（无法核对 ≠ 无残留），不得跳过 SHM 检查。
    /// - 一次调用聚合全部问题后统一返回 `Err`，避免只报告首个残留而掩盖其它残留。
    /// - 从不删除外层 XDG 目录本身，也从不删除不属于本次运行的任何文件。
    ///
    /// Red 基线已由测试驱动到 Green：`.lock` 精确核对、`shm_path=None` 失败关闭、
    /// 全部残留聚合在同一条错误中报告。
    pub(crate) fn ensure_owned_resources_reclaimed(
        runtime_dir: &Path,
        socket_path: &Path,
        shm_path: Option<&Path>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(runtime_dir)
            .map_err(|error| format!("读取 XDG_RUNTIME_DIR 失败: {error}"))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| format!("读取 XDG_RUNTIME_DIR 单项失败: {error}"))?;
            names.push(entry.file_name());
        }
        let mut problems: Vec<String> = Vec::new();

        // 1) socket 精确文件名。
        match socket_path.file_name() {
            None => problems.push("socket 路径缺少文件名".to_owned()),
            Some(socket_name) => {
                if names.iter().any(|name| name.as_os_str() == socket_name) {
                    problems.push(format!("本次运行 socket 仍残留: {}", socket_path.display()));
                }
            }
        }

        // 2) `<socket>.lock` 精确文件名，命名与 wayland-server 的
        //    `socket_path.with_extension("lock")` 同源，避免两套推导产生漏检。
        let lock_path = socket_path.with_extension("lock");
        match lock_path.file_name() {
            None => problems.push("`.lock` 路径缺少文件名".to_owned()),
            Some(lock_name) => {
                if names.iter().any(|name| name.as_os_str() == lock_name) {
                    problems.push(format!(
                        "本次运行 <socket>.lock 仍残留: {}",
                        lock_path.display()
                    ));
                }
            }
        }

        // 3) 本次 client SHM 精确文件名；`None` 表示无法核对，必须失败关闭。
        match shm_path {
            None => {
                problems.push("未取得本次运行 SHM 路径，无法核对 SHM 残留（失败关闭）".to_owned());
            }
            Some(shm_path) => match shm_path.file_name() {
                None => problems.push("SHM 路径缺少文件名".to_owned()),
                Some(shm_name) => {
                    if names.iter().any(|name| name.as_os_str() == shm_name) {
                        problems.push(format!("本次运行 SHM 仍残留: {}", shm_path.display()));
                    }
                }
            },
        }

        if problems.is_empty() {
            Ok(())
        } else {
            // 聚合全部问题一次性报告，避免只报首个残留而掩盖 `.lock`/SHM 残留。
            Err(problems.join("；"))
        }
    }
}

/// 回读不匹配的紧凑诊断：长度、首个差异坐标、行/列/180 度关系、全黑与四角颜色。
///
/// 门禁失败时只输出该摘要，不倾倒整幅 256×256 像素向量；该助手只描述差异，不参与
/// 通过/失败判定，也不证明渲染结果。
#[cfg(any(feature = "smithay-linux", test))]
fn describe_readback_mismatch(actual: &[[u8; 3]], expected: &[[u8; 3]], width: usize) -> String {
    if actual.len() != expected.len() {
        return format!(
            "回读长度不符: actual={} expected={} width={width}",
            actual.len(),
            expected.len()
        );
    }
    if actual == expected {
        return "回读与期望逐像素一致（不匹配另有原因）".to_owned();
    }
    let mut message = String::new();
    if actual.iter().all(|pixel| *pixel == [0x00, 0x00, 0x00]) {
        message.push_str("actual 全黑；");
    }
    if let Some(index) = actual
        .iter()
        .zip(expected.iter())
        .position(|(actual, expected)| actual != expected)
    {
        let (x, y) = if width > 0 {
            (index % width, index / width)
        } else {
            (0, 0)
        };
        message.push_str(&format!(
            "首个差异 index={index} x={x} y={y} actual={:?} expected={:?}；",
            actual[index], expected[index]
        ));
    }
    // 行/列/180 度关系：黑窗口失败时一次性判定“目标是否只是方向反了”。
    if width > 0 && actual.len() % width == 0 {
        let height = actual.len() / width;
        let row_flip = (0..height).all(|y| {
            (0..width).all(|x| actual[y * width + x] == expected[(height - 1 - y) * width + x])
        });
        let col_flip = (0..height).all(|y| {
            (0..width).all(|x| actual[y * width + x] == expected[y * width + (width - 1 - x)])
        });
        let rot180 = (0..height).all(|y| {
            (0..width).all(|x| {
                actual[y * width + x] == expected[(height - 1 - y) * width + (width - 1 - x)]
            })
        });
        if row_flip {
            message.push_str("行序上下翻转；");
        }
        if col_flip {
            message.push_str("列序左右翻转；");
        }
        if rot180 {
            message.push_str("180 度旋转；");
        }
        if !actual.is_empty() && width > 0 && height > 0 && actual.len() == width * height {
            let last_row = (height - 1) * width;
            message.push_str(&format!(
                "actual 四角: 左上={:?} 右上={:?} 左下={:?} 右下={:?}；",
                actual[0],
                actual[width - 1],
                actual[last_row],
                actual[last_row + width - 1]
            ));
        }
    }
    message
}

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
mod controlled_runner {
    use std::{
        fs,
        io::{ErrorKind, Read, Write},
        net::Shutdown,
        os::unix::net::UnixStream,
        os::{fd::AsFd, unix::fs::OpenOptionsExt},
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use calloop::{EventLoop, Interest, Mode, PostAction, generic::Generic};
    use wayland_client::{
        Connection, Dispatch, EventQueue, Proxy, QueueHandle,
        backend::WaylandError,
        protocol::{
            wl_buffer::WlBuffer, wl_callback::WlCallback, wl_compositor::WlCompositor,
            wl_registry::WlRegistry, wl_shm::WlShm, wl_shm_pool::WlShmPool, wl_surface::WlSurface,
        },
    };
    use wayland_protocols::xdg::shell::client::{
        xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
    };

    use crate::{
        core::state::State,
        describe_readback_mismatch, residue, shm_pattern,
        smithay_backend::{
            linux_toplevel_admission_runtime_queue::RuntimeToplevelAdmissionDrainTick,
            nested_runtime_coordinator::{
                NestedRuntimeCoordinator, RuntimeShmRenderAttemptOutcome,
                RuntimeShmRenderAttemptReport,
            },
        },
    };

    const RUN_DEADLINE: Duration = Duration::from_secs(8);
    const PUMP_TIMEOUT: Duration = Duration::from_millis(5);
    const MAX_PUMPS: usize = 1_600;
    const MAX_CLIENT_READINESS_POLLS: usize = 1_600;
    /// 持续可见模式的绝对 deadline／watchdog：显式 stop 前维持，超时即失败退出并清理。
    /// 固定 45s 有界，不引入可中断当前 pump 的硬超时语义之外的第二种超时。
    const SUSTAINED_DEADLINE: Duration = Duration::from_secs(45);
    const SUSTAINED_MAX_PUMPS: usize = 9_000;
    /// client 持有期的单轮有界驱动预算，避免无界等待；外层再按 stop 轮询。
    const CLIENT_HOLD_ROUND: Duration = Duration::from_millis(500);
    /// stdin 控制门单行上限（含换行），与 bounded runner 一致；仅用 std，不新增依赖。
    const STOP_LINE_LIMIT: usize = 128;
    /// teardown 阶段等待 client 返回销毁结果的有界预算；期间服务端 socket 保持可用并持续 pump。
    const TEARDOWN_WAIT: Duration = Duration::from_secs(5);
    /// 确认本次运行确切身份关闭（tombstone、布局移除、基线保持）的有界排空预算。
    const CLOSURE_WAIT: Duration = Duration::from_secs(5);

    /// client SHM backing file 的唯一 owner；drop 删除文件，失败路径同样不会留下残留。
    struct ShmBackingFile {
        file: fs::File,
        path: PathBuf,
    }

    impl ShmBackingFile {
        fn create(runtime_dir: &Path) -> Result<Self, String> {
            let entropy = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| format!("生成 SHM 文件名失败: {error}"))?
                .as_nanos();
            let path = runtime_dir.join(format!("sky-mirror-r2-shm-{entropy}.bin"));
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .map_err(|error| format!("创建 SHM backing file 失败: {error}"))?;
            // 先构造 owner 再写入：任何写入/flush 失败都会经由 Drop 删除文件，
            // 失败退出路径不留下本次运行的 SHM 残留。
            let mut this = Self { file, path };
            // 256×256 XRGB8888 四象限（红、绿、蓝、白）：尺寸/stride 全部由 shm_pattern
            // 派生，文件长度恰为 stride × height。该已知图样用于证明 client buffer 不是
            // 固定背景；写入/flush 失败经 Drop 删除文件，失败路径不留下 SHM 残留。
            let pattern = shm_pattern::bytes();
            if pattern.len() != shm_pattern::BYTE_LEN as usize {
                return Err(format!(
                    "SHM 图样长度 {} 与 stride × height {} 不符",
                    pattern.len(),
                    shm_pattern::BYTE_LEN
                ));
            }
            this.file
                .write_all(&pattern)
                .map_err(|error| format!("写入 SHM 图样失败: {error}"))?;
            this.file
                .flush()
                .map_err(|error| format!("flush SHM 图样失败: {error}"))?;
            Ok(this)
        }
    }

    impl Drop for ShmBackingFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct RegisteredGlobal {
        name: u32,
        version: u32,
    }

    #[derive(Debug, Clone, Copy)]
    enum ClientCallbackKind {
        Registry,
        Frame,
    }

    #[derive(Default)]
    struct ExternalClientState {
        wl_compositor_global: Option<RegisteredGlobal>,
        wl_shm_global: Option<RegisteredGlobal>,
        xdg_wm_base_global: Option<RegisteredGlobal>,
        registry_sync_done: bool,
        wl_surface: Option<WlSurface>,
        shm_pool: Option<WlShmPool>,
        shm_buffer: Option<WlBuffer>,
        xdg_surface: Option<XdgSurface>,
        xdg_toplevel: Option<XdgToplevel>,
        buffer_commit_sent: bool,
        damage_sent: bool,
        frame_requested: bool,
        frame_done: bool,
    }

    impl Dispatch<WlRegistry, ()> for ExternalClientState {
        fn event(
            state: &mut Self,
            _proxy: &WlRegistry,
            event: wayland_client::protocol::wl_registry::Event,
            _data: &(),
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
            match event {
                wayland_client::protocol::wl_registry::Event::Global {
                    name,
                    interface,
                    version,
                } if interface == "wl_compositor" => {
                    state.wl_compositor_global = Some(RegisteredGlobal { name, version });
                }
                wayland_client::protocol::wl_registry::Event::Global {
                    name,
                    interface,
                    version,
                } if interface == "wl_shm" => {
                    state.wl_shm_global = Some(RegisteredGlobal { name, version });
                }
                wayland_client::protocol::wl_registry::Event::Global {
                    name,
                    interface,
                    version,
                } if interface == "xdg_wm_base" => {
                    state.xdg_wm_base_global = Some(RegisteredGlobal { name, version });
                }
                wayland_client::protocol::wl_registry::Event::GlobalRemove { name } => {
                    if state
                        .wl_compositor_global
                        .is_some_and(|global| global.name == name)
                    {
                        state.wl_compositor_global = None;
                    }
                    if state
                        .wl_shm_global
                        .is_some_and(|global| global.name == name)
                    {
                        state.wl_shm_global = None;
                    }
                    if state
                        .xdg_wm_base_global
                        .is_some_and(|global| global.name == name)
                    {
                        state.xdg_wm_base_global = None;
                    }
                }
                _ => {}
            }
        }
    }

    impl Dispatch<XdgWmBase, ()> for ExternalClientState {
        fn event(
            _state: &mut Self,
            proxy: &XdgWmBase,
            event: wayland_protocols::xdg::shell::client::xdg_wm_base::Event,
            _data: &(),
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
            if let wayland_protocols::xdg::shell::client::xdg_wm_base::Event::Ping { serial } =
                event
            {
                proxy.pong(serial);
            }
        }
    }

    impl Dispatch<XdgSurface, ()> for ExternalClientState {
        fn event(
            state: &mut Self,
            proxy: &XdgSurface,
            event: wayland_protocols::xdg::shell::client::xdg_surface::Event,
            _data: &(),
            _connection: &Connection,
            queue_handle: &QueueHandle<Self>,
        ) {
            if let wayland_protocols::xdg::shell::client::xdg_surface::Event::Configure { serial } =
                event
            {
                proxy.ack_configure(serial);
                if !state.buffer_commit_sent {
                    let surface = state
                        .wl_surface
                        .as_ref()
                        .expect("configure 前 client 必须持有 wl_surface");
                    let buffer = state
                        .shm_buffer
                        .as_ref()
                        .expect("configure 前 client 必须持有 shm buffer");
                    surface.attach(Some(buffer), 0, 0);
                    surface.damage_buffer(0, 0, shm_pattern::WIDTH, shm_pattern::HEIGHT);
                    let _frame = surface.frame(queue_handle, ClientCallbackKind::Frame);
                    surface.commit();
                    state.buffer_commit_sent = true;
                    state.damage_sent = true;
                    state.frame_requested = true;
                }
            }
        }
    }

    impl Dispatch<XdgToplevel, ()> for ExternalClientState {
        fn event(
            _state: &mut Self,
            _proxy: &XdgToplevel,
            _event: wayland_protocols::xdg::shell::client::xdg_toplevel::Event,
            _data: &(),
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlCallback, ClientCallbackKind> for ExternalClientState {
        fn event(
            state: &mut Self,
            _proxy: &WlCallback,
            event: wayland_client::protocol::wl_callback::Event,
            callback_kind: &ClientCallbackKind,
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
            if let wayland_client::protocol::wl_callback::Event::Done { .. } = event {
                match callback_kind {
                    ClientCallbackKind::Registry => state.registry_sync_done = true,
                    ClientCallbackKind::Frame => state.frame_done = true,
                }
            }
        }
    }

    wayland_client::delegate_noop!(ExternalClientState: ignore WlCompositor);
    wayland_client::delegate_noop!(ExternalClientState: ignore WlSurface);
    wayland_client::delegate_noop!(ExternalClientState: ignore WlShm);
    wayland_client::delegate_noop!(ExternalClientState: ignore WlShmPool);
    wayland_client::delegate_noop!(ExternalClientState: ignore WlBuffer);

    #[derive(Debug)]
    struct ExternalClientEvidence {
        buffer_commit_sent: bool,
        damage_sent: bool,
        frame_requested: bool,
        frame_done: bool,
    }

    /// 持续模式 client 的收尾方式：区分“传输层突然断开”与“有序协议销毁”。
    ///
    /// 该枚举只描述**客户端动作**；服务端是否真的观察到状态关闭由同一 State 的
    /// tombstone／布局移除／validation 验证单独证明，两者不得互相冒充。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ClientTeardown {
        /// 断连测试：关闭本进程拥有的 UnixStream 双向传输，不发送任何对象 destroy 请求。
        TransportShutdown,
        /// 正常 stop：按 owner 顺序发出 destroy 请求并 flush。
        OrderedDestroy,
    }

    /// 持续模式 client 的完整返回：首帧证据 ＋ 收尾方式。
    struct SustainedClientResult {
        evidence: ExternalClientEvidence,
        teardown: ClientTeardown,
    }

    #[derive(Default)]
    struct ClientReadiness {
        readable_or_error: bool,
        writable_or_error: bool,
    }

    /// 在同一绝对 deadline 内等待 client socket 的可写或错误事件。
    ///
    /// 该 helper 不读取 socket，也不创建 `prepare_read` guard；唯一 reader 仍是下方
    /// `drive_client_until` 中的 guard，避免 write backpressure 分支破坏 Wayland 单 reader
    /// 同步约束。
    ///
    /// `Ok(true)` 表示观察到 writable/error 事件；`Ok(false)` 表示 deadline 内未观察到
    /// （对持有循环是预期空闲，由调用方决定算 idle 还是失败）；`Err` 只表示 poll
    /// dispatch 的真实错误。
    fn wait_for_client_writable(
        write_readiness_loop: &mut EventLoop<ClientReadiness>,
        readiness: &mut ClientReadiness,
        deadline: Instant,
        stage: &str,
    ) -> Result<bool, String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }
        readiness.writable_or_error = false;
        write_readiness_loop
            .dispatch(Some(remaining), readiness)
            .map_err(|error| format!("外部 client {stage} writable readiness 失败: {error}"))?;
        Ok(readiness.writable_or_error)
    }

    /// 只为已经发出的 cleanup request 完成有界 outbound flush；它不尝试 read 或 dispatch
    /// 新事件，因此不会在 client 已开始销毁对象后引入新的 protocol state mutation。
    fn flush_client_until(
        event_queue: &mut EventQueue<ExternalClientState>,
        write_readiness_loop: &mut EventLoop<ClientReadiness>,
        readiness: &mut ClientReadiness,
        deadline: Instant,
        stage: &str,
    ) -> Result<(), String> {
        for _ in 0..MAX_CLIENT_READINESS_POLLS {
            match event_queue.flush() {
                Ok(()) => return Ok(()),
                Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
                    if !wait_for_client_writable(write_readiness_loop, readiness, deadline, stage)?
                    {
                        return Err(format!(
                            "外部 client {stage} deadline 内无 writable fd 事件"
                        ));
                    }
                }
                Err(error) => return Err(format!("外部 client {stage} flush 失败: {error}")),
            }
        }
        Err(format!(
            "外部 client {stage} 超过固定 writable poll 上限 {MAX_CLIENT_READINESS_POLLS}"
        ))
    }

    /// `drive_client_until` 的显式结果，用于区分预期空闲与真实故障。
    ///
    /// - `Completed`：传入谓词已满足（首帧条件或 stop 条件）。
    /// - `Idle`：到达传入 deadline 仍未满足；对持有循环是预期的“无事件空闲等待”，
    ///   对首帧准备阶段由 `drive_client_until_completed` 重新解释为失败。
    ///
    /// poll、dispatch、flush、read 的真实错误一律以 `Err` 上报，不因持有循环而被吞掉。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum DriveOutcome {
        Completed,
        Idle,
    }

    /// 首帧准备阶段专用包装：`Idle`（deadline 内条件未满足）必须是失败，与旧语义
    /// 一致地返回 `Err`；真实错误原样上抛。
    fn drive_client_until_completed(
        event_queue: &mut EventQueue<ExternalClientState>,
        state: &mut ExternalClientState,
        readiness_loop: &mut EventLoop<ClientReadiness>,
        write_readiness_loop: &mut EventLoop<ClientReadiness>,
        readiness: &mut ClientReadiness,
        deadline: Instant,
        stage: &str,
        completed: impl Fn(&ExternalClientState) -> bool,
    ) -> Result<(), String> {
        match drive_client_until(
            event_queue,
            state,
            readiness_loop,
            write_readiness_loop,
            readiness,
            deadline,
            stage,
            completed,
        )? {
            DriveOutcome::Completed => Ok(()),
            DriveOutcome::Idle => Err(format!("外部 client {stage} 超过固定 deadline")),
        }
    }

    /// 只通过绝对 deadline、固定 poll 上限和 fd readiness 驱动 Wayland client。
    ///
    /// `prepare_read` 严格先于 readiness poll，唯一 reader 是其 guard；不会使用
    /// `blocking_dispatch`、无界 roundtrip 或脱离 owner 的后台线程。
    ///
    /// deadline 内无新事件返回 `Ok(DriveOutcome::Idle)`（预期空闲）；只有
    /// poll/dispatch/flush/read 真实错误与固定 poll 上限突破才返回 `Err`。
    fn drive_client_until(
        event_queue: &mut EventQueue<ExternalClientState>,
        state: &mut ExternalClientState,
        readiness_loop: &mut EventLoop<ClientReadiness>,
        write_readiness_loop: &mut EventLoop<ClientReadiness>,
        readiness: &mut ClientReadiness,
        deadline: Instant,
        stage: &str,
        completed: impl Fn(&ExternalClientState) -> bool,
    ) -> Result<DriveOutcome, String> {
        let mut polls = 0usize;
        while !completed(state) {
            if polls >= MAX_CLIENT_READINESS_POLLS {
                return Err(format!(
                    "外部 client {stage} 超过固定 readiness poll 上限 {MAX_CLIENT_READINESS_POLLS}"
                ));
            }
            if Instant::now() >= deadline {
                return Ok(DriveOutcome::Idle);
            }

            match event_queue.flush() {
                Ok(()) => {}
                Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
                    // 发送方向也必须由同一 fd readiness/deadline 驱动；不能以 sleep
                    // 轮询冒充有界 I/O。read 的 guard 尚未创建，故不会破坏单 reader。
                    let writable =
                        wait_for_client_writable(write_readiness_loop, readiness, deadline, stage)?;
                    polls = polls.saturating_add(1);
                    if !writable && Instant::now() >= deadline {
                        return Ok(DriveOutcome::Idle);
                    }
                    continue;
                }
                Err(error) => return Err(format!("外部 client {stage} flush 失败: {error}")),
            }
            event_queue
                .dispatch_pending(state)
                .map_err(|error| format!("外部 client {stage} dispatch_pending 失败: {error}"))?;
            if completed(state) {
                break;
            }

            let Some(read_guard) = event_queue.prepare_read() else {
                polls = polls.saturating_add(1);
                continue;
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(DriveOutcome::Idle);
            }
            readiness.readable_or_error = false;
            readiness_loop
                .dispatch(Some(remaining), readiness)
                .map_err(|error| format!("外部 client {stage} readiness poll 失败: {error}"))?;
            polls = polls.saturating_add(1);
            if !readiness.readable_or_error {
                // 等满 remaining 仍无 fd 事件：deadline 已到即为预期空闲，而不是故障。
                if Instant::now() >= deadline {
                    return Ok(DriveOutcome::Idle);
                }
                continue;
            }
            match read_guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => return Err(format!("外部 client {stage} read 失败: {error}")),
            }
            event_queue
                .dispatch_pending(state)
                .map_err(|error| format!("外部 client {stage} read 后 dispatch 失败: {error}"))?;
        }
        Ok(DriveOutcome::Completed)
    }

    fn run_external_client(
        socket_path: &Path,
        runtime_dir: &Path,
        shm_path_sender: mpsc::Sender<PathBuf>,
    ) -> Result<ExternalClientEvidence, String> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|error| format!("外部 client 连接 production socket 失败: {error}"))?;
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("外部 client 设置非阻塞 socket 失败: {error}"))?;
        let readiness_socket = stream
            .try_clone()
            .map_err(|error| format!("外部 client 克隆 readiness socket 失败: {error}"))?;
        let write_readiness_socket = stream
            .try_clone()
            .map_err(|error| format!("外部 client 克隆 writable readiness socket 失败: {error}"))?;
        let connection = Connection::from_socket(stream)
            .map_err(|error| format!("外部 client 创建 Connection 失败: {error}"))?;
        let mut event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();
        let display = connection.display();
        let registry = display.get_registry(&queue_handle, ());
        let _registry_sync = display.sync(&queue_handle, ClientCallbackKind::Registry);
        let deadline = Instant::now() + RUN_DEADLINE;
        let mut readiness_loop = EventLoop::<ClientReadiness>::try_new()
            .map_err(|error| format!("外部 client 创建 readiness loop 失败: {error}"))?;
        let mut write_readiness_loop = EventLoop::<ClientReadiness>::try_new()
            .map_err(|error| format!("外部 client 创建 writable readiness loop 失败: {error}"))?;
        let mut readiness = ClientReadiness::default();
        let _readiness_source = readiness_loop
            .handle()
            .insert_source(
                Generic::new(readiness_socket, Interest::READ, Mode::Level),
                |event, _socket, readiness| {
                    readiness.readable_or_error |= event.readable || event.error;
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|error| format!("外部 client 注册 readiness source 失败: {error}"))?;
        let _write_readiness_source = write_readiness_loop
            .handle()
            .insert_source(
                Generic::new(write_readiness_socket, Interest::WRITE, Mode::Level),
                |event, _socket, readiness| {
                    readiness.writable_or_error |= event.writable || event.error;
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|error| format!("外部 client 注册 writable readiness source 失败: {error}"))?;
        let mut state = ExternalClientState::default();
        drive_client_until_completed(
            &mut event_queue,
            &mut state,
            &mut readiness_loop,
            &mut write_readiness_loop,
            &mut readiness,
            deadline,
            "registry discovery",
            |state| state.registry_sync_done,
        )?;

        let compositor_global = state
            .wl_compositor_global
            .ok_or("registry 未发现 wl_compositor")?;
        let shm_global = state.wl_shm_global.ok_or("registry 未发现 wl_shm")?;
        let xdg_global = state
            .xdg_wm_base_global
            .ok_or("registry 未发现 xdg_wm_base")?;
        let compositor = registry.bind::<WlCompositor, _, _>(
            compositor_global.name,
            compositor_global.version.min(5),
            &queue_handle,
            (),
        );
        let shm = registry.bind::<WlShm, _, _>(
            shm_global.name,
            shm_global.version.min(1),
            &queue_handle,
            (),
        );
        let xdg_wm_base = registry.bind::<XdgWmBase, _, _>(
            xdg_global.name,
            xdg_global.version.min(7),
            &queue_handle,
            (),
        );
        if !compositor.is_alive() || !shm.is_alive() || !xdg_wm_base.is_alive() {
            return Err("required global bind 后得到 inert proxy".to_owned());
        }

        let backing = ShmBackingFile::create(runtime_dir)?;
        // 立刻把本次运行确切的 SHM 路径交给主线程，失败路径也能做精确残留核对。
        let _ = shm_path_sender.send(backing.path.clone());
        let pool = shm.create_pool(
            backing.file.as_fd(),
            shm_pattern::BYTE_LEN,
            &queue_handle,
            (),
        );
        let buffer = pool.create_buffer(
            0,
            shm_pattern::WIDTH,
            shm_pattern::HEIGHT,
            shm_pattern::STRIDE,
            wayland_client::protocol::wl_shm::Format::Xrgb8888,
            &queue_handle,
            (),
        );
        let surface = compositor.create_surface(&queue_handle, ());
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &queue_handle, ());
        let toplevel = xdg_surface.get_toplevel(&queue_handle, ());
        toplevel.set_title("Sky Mirror R2 controlled SHM client".to_owned());

        state.wl_surface = Some(surface.clone());
        state.shm_pool = Some(pool);
        state.shm_buffer = Some(buffer);
        state.xdg_surface = Some(xdg_surface);
        state.xdg_toplevel = Some(toplevel);
        // 标准 xdg 初始空提交；server configure 到达后 callback 才 attach/damage/frame/commit。
        surface.commit();
        drive_client_until_completed(
            &mut event_queue,
            &mut state,
            &mut readiness_loop,
            &mut write_readiness_loop,
            &mut readiness,
            deadline,
            "configure/frame done",
            |state| state.frame_done,
        )?;

        let evidence = ExternalClientEvidence {
            buffer_commit_sent: state.buffer_commit_sent,
            damage_sent: state.damage_sent,
            frame_requested: state.frame_requested,
            frame_done: state.frame_done,
        };
        // 按 role→xdg_surface→wl_surface→buffer/pool 的 owner 顺序显式发出 cleanup。
        if let Some(toplevel) = state.xdg_toplevel.take() {
            toplevel.destroy();
        }
        if let Some(xdg_surface) = state.xdg_surface.take() {
            xdg_surface.destroy();
        }
        surface.destroy();
        if let Some(buffer) = state.shm_buffer.take() {
            buffer.destroy();
        }
        if let Some(pool) = state.shm_pool.take() {
            pool.destroy();
        }
        flush_client_until(
            &mut event_queue,
            &mut write_readiness_loop,
            &mut readiness,
            deadline,
            "cleanup destroy flush",
        )?;
        Ok(evidence)
    }

    /// 显式停止信号：正常停止（`stop` 行或 EOF）与输入错误（超长行／读取失败）必须区分，
    /// 输入错误绝不能被上报为成功停止。
    enum StopSignal {
        /// 正常停止请求，`origin` 记录来源（`stop` 行或 EOF）。
        Requested { origin: &'static str },
        /// 输入协议/读取错误：调用方必须以失败收场，但仍需先完成资源清理。
        InputError(String),
    }

    /// 有界读取停止信号：单行总长（内容+换行）不超过 `STOP_LINE_LIMIT`，在读取过程中
    /// 即限长，绝不先用 `read_line` 无界缓冲再检查长度。EOF 视为正常停止；
    /// 超长行与读取失败返回 `InputError`。仅用 std，不新增依赖。
    ///
    /// reader 线程的读取依赖该限长在有限输入下返回；若调用方既不发送数据也不关闭
    /// stdin，线程保持阻塞并随进程退出回收（不宣称已 join，也不宣称可强制中止）。
    fn read_stop_signal_bounded() -> StopSignal {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        // 内容上限 = STOP_LINE_LIMIT - 1（为行终止符预留 1 字节）。
        let mut line = vec![0u8; STOP_LINE_LIMIT - 1];
        let mut line_len = 0usize;
        let mut chunk = [0u8; 64];
        loop {
            match handle.read(&mut chunk) {
                Ok(0) => {
                    // EOF：与既有约定一致为正常停止；残留半行按是否为 `stop` 决定来源。
                    let pending = std::str::from_utf8(&line[..line_len])
                        .map(|text| text.trim() == "stop")
                        .unwrap_or(false);
                    return StopSignal::Requested {
                        origin: if pending { "stop" } else { "eof" },
                    };
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => {
                    return StopSignal::InputError(format!("stdin 读取失败: {error}"));
                }
                Ok(read) => {
                    for &byte in &chunk[..read] {
                        if byte == b'\n' {
                            let is_stop = std::str::from_utf8(&line[..line_len])
                                .map(|text| text.trim() == "stop")
                                .unwrap_or(false);
                            if is_stop {
                                return StopSignal::Requested { origin: "stop" };
                            }
                            // 非 stop 行忽略并复位，继续等待；避免误停。
                            line_len = 0;
                            continue;
                        }
                        if line_len >= line.len() {
                            // 超长行：协议错误，区别于正常停止，且不无界缓冲。
                            return StopSignal::InputError(format!(
                                "stdin 停止行超过 {STOP_LINE_LIMIT} 字节上限（含换行）"
                            ));
                        }
                        line[line_len] = byte;
                        line_len += 1;
                    }
                }
            }
        }
    }

    /// 显式 stop 控制门 reader 线程：先发送停止信号、后置位 stop 标志，保证主循环
    /// 观察到 `stop_flag == true` 时信号一定已可被 `try_recv` 取走。
    ///
    /// owner：stop 标志由主线程创建、reader 线程写入、主循环与 client 持有循环读取；
    /// 线程阻塞读取不可取消，主流程超时后收尾退出，线程随进程退出回收（不宣称已 join）。
    fn spawn_stdin_stop_reader(stop_flag: Arc<AtomicBool>, signals: mpsc::Sender<StopSignal>) {
        thread::spawn(move || {
            let signal = read_stop_signal_bounded();
            let _ = signals.send(signal);
            stop_flag.store(true, Ordering::SeqCst);
        });
    }

    /// 持续可见的外部 client：首帧逻辑与单帧一致，首帧后保持对象存活直到明确停止。
    ///
    /// 不创建新 buffer、不替换、不做 surface-tree 合成；只持有首个 256×256 四象限 SHM buffer。
    /// `disconnect_after_present` 为 true 时，首帧成功并完成既有短暂持有后**只关闭传输层**
    /// （对本进程明确拥有的 UnixStream 克隆执行双向 shutdown），不发送任何对象 destroy、
    /// 不 flush destroy，用于验证服务端必须经连接关闭路径清理窗口状态。正常模式下，
    /// 收到 stop 才按 role→xdg_surface→wl_surface→buffer/pool 顺序 cleanup 并 flush。
    /// owner：SHM backing file 由 RAII owner 在返回时删除；Wayland 对象由本线程拥有，
    /// 正常模式显式 destroy，断连模式仅 drop——已核对 wayland-client 0.31.14：Proxy／
    /// Connection 均无 Drop 发送 destroy 的实现，丢弃代理不产生协议请求。
    fn run_external_client_sustained(
        socket_path: &Path,
        runtime_dir: &Path,
        stop_flag: Arc<AtomicBool>,
        overall_deadline: Instant,
        disconnect_after_present: bool,
        shm_path_sender: mpsc::Sender<PathBuf>,
    ) -> Result<SustainedClientResult, String> {
        // 复用单帧的前半段搭建：连接、registry、bind、建池/buffer/surface/toplevel、空提交。
        // 为避免复制大段逻辑，这里先以内联方式重走相同步骤，但首帧后进入持有循环。
        let stream = UnixStream::connect(socket_path)
            .map_err(|error| format!("外部 client 连接 production socket 失败: {error}"))?;
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("外部 client 设置非阻塞 socket 失败: {error}"))?;
        let readiness_socket = stream
            .try_clone()
            .map_err(|error| format!("外部 client 克隆 readiness socket 失败: {error}"))?;
        let write_readiness_socket = stream
            .try_clone()
            .map_err(|error| format!("外部 client 克隆 writable readiness socket 失败: {error}"))?;
        // 断连测试专用：本进程明确拥有的克隆，仅用于对同一 socket 执行双向 shutdown。
        // shutdown(2) 作用于 socket 本身而非单个 fd，即使 calloop source 仍持有其余克隆，
        // 服务端也能确定地观察到 EOF/HUP；仅靠 drop 代理／Connection 无法保证立即断开。
        let disconnect_shutdown_socket = stream.try_clone().map_err(|error| {
            format!("外部 client 克隆 disconnect shutdown socket 失败: {error}")
        })?;
        let connection = Connection::from_socket(stream)
            .map_err(|error| format!("外部 client 创建 Connection 失败: {error}"))?;
        let mut event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();
        let display = connection.display();
        let registry = display.get_registry(&queue_handle, ());
        let _registry_sync = display.sync(&queue_handle, ClientCallbackKind::Registry);
        let mut readiness_loop = EventLoop::<ClientReadiness>::try_new()
            .map_err(|error| format!("外部 client 创建 readiness loop 失败: {error}"))?;
        let mut write_readiness_loop = EventLoop::<ClientReadiness>::try_new()
            .map_err(|error| format!("外部 client 创建 writable readiness loop 失败: {error}"))?;
        let mut readiness = ClientReadiness::default();
        let _readiness_source = readiness_loop
            .handle()
            .insert_source(
                Generic::new(readiness_socket, Interest::READ, Mode::Level),
                |event, _socket, readiness| {
                    readiness.readable_or_error |= event.readable || event.error;
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|error| format!("外部 client 注册 readiness source 失败: {error}"))?;
        let _write_readiness_source = write_readiness_loop
            .handle()
            .insert_source(
                Generic::new(write_readiness_socket, Interest::WRITE, Mode::Level),
                |event, _socket, readiness| {
                    readiness.writable_or_error |= event.writable || event.error;
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|error| format!("外部 client 注册 writable readiness source 失败: {error}"))?;
        let mut state = ExternalClientState::default();
        drive_client_until_completed(
            &mut event_queue,
            &mut state,
            &mut readiness_loop,
            &mut write_readiness_loop,
            &mut readiness,
            overall_deadline,
            "registry discovery",
            |state| state.registry_sync_done,
        )?;
        let compositor_global = state
            .wl_compositor_global
            .ok_or("registry 未发现 wl_compositor")?;
        let shm_global = state.wl_shm_global.ok_or("registry 未发现 wl_shm")?;
        let xdg_global = state
            .xdg_wm_base_global
            .ok_or("registry 未发现 xdg_wm_base")?;
        let compositor = registry.bind::<WlCompositor, _, _>(
            compositor_global.name,
            compositor_global.version.min(5),
            &queue_handle,
            (),
        );
        let shm = registry.bind::<WlShm, _, _>(
            shm_global.name,
            shm_global.version.min(1),
            &queue_handle,
            (),
        );
        let xdg_wm_base = registry.bind::<XdgWmBase, _, _>(
            xdg_global.name,
            xdg_global.version.min(7),
            &queue_handle,
            (),
        );
        if !compositor.is_alive() || !shm.is_alive() || !xdg_wm_base.is_alive() {
            return Err("required global bind 后得到 inert proxy".to_owned());
        }
        let backing = ShmBackingFile::create(runtime_dir)?;
        // 立刻把本次运行确切的 SHM 路径交给主线程，失败路径也能做精确残留核对。
        let _ = shm_path_sender.send(backing.path.clone());
        let pool = shm.create_pool(
            backing.file.as_fd(),
            shm_pattern::BYTE_LEN,
            &queue_handle,
            (),
        );
        let buffer = pool.create_buffer(
            0,
            shm_pattern::WIDTH,
            shm_pattern::HEIGHT,
            shm_pattern::STRIDE,
            wayland_client::protocol::wl_shm::Format::Xrgb8888,
            &queue_handle,
            (),
        );
        let surface = compositor.create_surface(&queue_handle, ());
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &queue_handle, ());
        let toplevel = xdg_surface.get_toplevel(&queue_handle, ());
        toplevel.set_title("Sky Mirror R2 sustained SHM client".to_owned());
        state.wl_surface = Some(surface.clone());
        state.shm_pool = Some(pool);
        state.shm_buffer = Some(buffer);
        state.xdg_surface = Some(xdg_surface);
        state.xdg_toplevel = Some(toplevel);
        surface.commit();
        drive_client_until_completed(
            &mut event_queue,
            &mut state,
            &mut readiness_loop,
            &mut write_readiness_loop,
            &mut readiness,
            overall_deadline,
            "configure/frame done",
            |state| state.frame_done,
        )?;
        // 持有期真实错误（poll/dispatch/flush/read）必须上报；预期 idle/deadline 不算错误。
        let mut hold_error: Option<String> = None;
        if disconnect_after_present {
            // 主动断连场景：持有约 1s 让服务端完成首帧 present，随后主动 cleanup 退出。
            // 服务端必须有界感知断连、排空 unmap/tombstone 并清理，不 hang。
            let hold_until = Instant::now() + Duration::from_secs(1);
            while Instant::now() < hold_until && Instant::now() < overall_deadline {
                let round = (Instant::now() + CLIENT_HOLD_ROUND).min(overall_deadline);
                match drive_client_until(
                    &mut event_queue,
                    &mut state,
                    &mut readiness_loop,
                    &mut write_readiness_loop,
                    &mut readiness,
                    round,
                    "disconnect hold",
                    |_| false,
                ) {
                    // 谓词恒 false 不会 Completed；防御性收束避免空转。
                    Ok(DriveOutcome::Completed) => break,
                    // 预期空闲：单轮 500ms 无事件，不是故障。
                    Ok(DriveOutcome::Idle) => {}
                    Err(error) => {
                        hold_error = Some(error);
                        break;
                    }
                }
            }
        } else {
            // 正常持续可见：保持对象存活，持续响应 ping/configure，直到明确 stop 或超时。
            // 不提交新 buffer，避免触及未授权的替换/release 语义。
            while !stop_flag.load(Ordering::SeqCst) && Instant::now() < overall_deadline {
                let round = (Instant::now() + CLIENT_HOLD_ROUND).min(overall_deadline);
                if round <= Instant::now() {
                    break;
                }
                let stop_clone = stop_flag.clone();
                match drive_client_until(
                    &mut event_queue,
                    &mut state,
                    &mut readiness_loop,
                    &mut write_readiness_loop,
                    &mut readiness,
                    round,
                    "sustained hold",
                    move |_| stop_clone.load(Ordering::SeqCst),
                ) {
                    // stop 条件满足：进入显式销毁。
                    Ok(DriveOutcome::Completed) => break,
                    // 预期空闲：单轮 500ms 无事件，不是故障。
                    Ok(DriveOutcome::Idle) => {}
                    Err(error) => {
                        hold_error = Some(error);
                        break;
                    }
                }
            }
        }
        let evidence = ExternalClientEvidence {
            buffer_commit_sent: state.buffer_commit_sent,
            damage_sent: state.damage_sent,
            frame_requested: state.frame_requested,
            frame_done: state.frame_done,
        };
        if disconnect_after_present {
            // ===== 断连测试：首帧成功且既有短暂持有完成后，只关闭 Wayland client 传输层。 =====
            // 绝不发送 xdg_toplevel/xdg_surface/wl_surface/buffer/pool 的 destroy 请求，
            // 也绝不 flush 任何 destroy；服务端只能经连接关闭路径清理窗口状态。
            // 失败时不退回有序 destroy/flush——否则失败的断连测试会变成假成功。
            //
            // 先执行本进程拥有克隆的双向 shutdown 并检查其错误：shutdown(2) 作用于 socket
            // 本身，服务端可确定观察到 EOF/HUP；仅 drop 代理不产生 destroy（wayland-client
            // 0.31.14 无 Drop 发送实现），但 calloop source 仍持有 fd 克隆，drop Connection
            // 不保证及时对端断开，故不以 drop 冒充 transport disconnect。
            let shutdown_result = disconnect_shutdown_socket.shutdown(Shutdown::Both);
            // 本地 RAII owner（SHM backing file 等）在函数返回时按既有语义清理，不受此分支影响。
            if let Some(error) = hold_error {
                // 持有期真实错误必须上报；即使 shutdown 已执行也不把失败断连包装成成功。
                return Err(format!(
                    "持有期驱动真实错误（非预期 idle，已执行 transport shutdown={}）: {error}",
                    shutdown_result.is_ok()
                ));
            }
            match shutdown_result {
                Ok(()) => {}
                Err(error) => {
                    // shutdown 失败：明确失败，不回退有序 destroy/flush。
                    return Err(format!(
                        "transport shutdown 失败（未发送 destroy，且不回退有序销毁）: {error}"
                    ));
                }
            }
            // 对象仅随作用域 drop：不发 destroy、不 flush；服务端清理由 State 关闭验证单独证明。
            return Ok(SustainedClientResult {
                evidence,
                teardown: ClientTeardown::TransportShutdown,
            });
        }
        // ===== 正常 stop：按 role→xdg_surface→wl_surface→buffer/pool 的 owner 顺序显式发出销毁请求。
        if let Some(toplevel) = state.xdg_toplevel.take() {
            toplevel.destroy();
        }
        if let Some(xdg_surface) = state.xdg_surface.take() {
            xdg_surface.destroy();
        }
        surface.destroy();
        if let Some(buffer) = state.shm_buffer.take() {
            buffer.destroy();
        }
        if let Some(pool) = state.shm_pool.take() {
            pool.destroy();
        }
        // backing 在此 drop 删除 SHM 文件；flush 仅处理已发出 destroy，不过度 read。
        // teardown/flush 错误必须传播，不静默吞掉。
        let flush_deadline = Instant::now() + Duration::from_secs(2);
        let flush_result = flush_client_until(
            &mut event_queue,
            &mut write_readiness_loop,
            &mut readiness,
            flush_deadline,
            "sustained cleanup destroy flush",
        );
        if let Some(error) = hold_error {
            return Err(format!("持有期驱动真实错误（非预期 idle）: {error}"));
        }
        if let Err(error) = flush_result {
            return Err(format!("teardown destroy flush 失败: {error}"));
        }
        Ok(SustainedClientResult {
            evidence,
            teardown: ClientTeardown::OrderedDestroy,
        })
    }

    /// 只读身份基线：`State::new()` 后、外部 client 连接前的 alive 三元组与
    /// workspace 归属快照；关闭验证只认相对该基线的差分与保持性。
    #[derive(Default)]
    struct IdentityBaseline {
        clients: Vec<u64>,
        surfaces: Vec<u64>,
        windows: Vec<u64>,
        workspace_windows: Vec<(u32, Vec<u64>)>,
    }

    /// 从只读 State 快照当前全部 alive client/surface/window 及 workspace 归属。
    fn snapshot_alive_identities(state: &State) -> IdentityBaseline {
        let mut clients: Vec<u64> = state
            .clients
            .records()
            .iter()
            .filter(|record| record.alive)
            .map(|record| record.id)
            .collect();
        clients.sort_unstable();
        let mut surfaces: Vec<u64> = state
            .surfaces
            .records()
            .iter()
            .filter(|record| record.alive)
            .map(|record| record.id)
            .collect();
        surfaces.sort_unstable();
        let mut windows: Vec<u64> = state
            .registry
            .records()
            .iter()
            .filter(|record| record.alive)
            .map(|record| record.id)
            .collect();
        windows.sort_unstable();
        let mut workspace_windows: Vec<(u32, Vec<u64>)> = state
            .compositor
            .workspaces
            .iter()
            .map(|workspace| {
                let mut ids = workspace.window_ids();
                ids.sort_unstable();
                (workspace.id, ids)
            })
            .collect();
        workspace_windows.sort_by_key(|(id, _)| *id);
        IdentityBaseline {
            clients,
            surfaces,
            windows,
            workspace_windows,
        }
    }

    /// 本次运行捕获的确切三元组（present 后动态识别，不硬编码、不数总数）。
    #[derive(Debug, Clone, Copy)]
    struct AdmissionEvidence {
        client: u64,
        surface: u64,
        window: u64,
    }

    /// present 后只读捕获本运行新增的 client/surface/window 三元组：相对基线恰好
    /// 新增一个 alive client，其恰有一个新增 alive surface 关联一个新增 alive window，
    /// 归属相符且 window 已进入 workspace，State 校验干净。任何缺失、歧义或错误归属均为 Err。
    fn capture_present_admission(
        state: &State,
        baseline: &IdentityBaseline,
    ) -> Result<AdmissionEvidence, String> {
        let mut new_clients: Vec<u64> = state
            .clients
            .records()
            .iter()
            .filter(|record| record.alive && !baseline.clients.contains(&record.id))
            .map(|record| record.id)
            .collect();
        new_clients.sort_unstable();
        let [client] = new_clients.as_slice() else {
            return Err(format!(
                "present 后相对基线的新增 alive client 必须恰好一个: {new_clients:?}"
            ));
        };
        let mut new_surfaces: Vec<u64> = state
            .surfaces
            .surfaces_for_client(*client)
            .into_iter()
            .filter(|surface| {
                state.surfaces.is_alive(*surface) && !baseline.surfaces.contains(surface)
            })
            .collect();
        new_surfaces.sort_unstable();
        let [surface] = new_surfaces.as_slice() else {
            return Err(format!(
                "client {client} 的新增 alive surface 必须恰好一个: {new_surfaces:?}"
            ));
        };
        let window = state
            .surfaces
            .window_for_surface(*surface)
            .ok_or_else(|| format!("surface {surface} 必须关联 Core window"))?;
        if baseline.windows.contains(&window) || !state.registry.is_alive(window) {
            return Err(format!(
                "surface {surface} 关联的 window 必须是新增且 alive: {window}"
            ));
        }
        let surface_record = state
            .surfaces
            .get(*surface)
            .ok_or_else(|| format!("surface {surface} 必须仍在 registry"))?;
        if surface_record.client != Some(*client) || surface_record.window != Some(window) {
            return Err(format!(
                "surface {surface} 归属不符: client={:?} window={:?}",
                surface_record.client, surface_record.window
            ));
        }
        if !state
            .compositor
            .workspaces
            .iter()
            .any(|workspace| workspace.window_ids().contains(&window))
        {
            return Err(format!("window {window} 必须已进入 workspace"));
        }
        if !state.validate().is_clean() {
            return Err("admission 捕获时 State 校验必须干净".to_owned());
        }
        Ok(AdmissionEvidence {
            client: *client,
            surface: *surface,
            window,
        })
    }

    /// 用 admission 阶段保存的同一组确切 ID 验证关闭：三类记录仍存在（tombstone
    /// 未被误删）、alive=false、surface→client 归属保持、window 不再被任何 workspace
    /// 的 slot/stack 引用、启动基线窗口与归属保持、State 校验干净。
    ///
    /// 只读；不依赖全局 `is_clean()` 单一信号，也不做“drop 后新建空 State”式伪验证。
    fn verify_identity_closed(
        state: &State,
        baseline: &IdentityBaseline,
        evidence: &AdmissionEvidence,
    ) -> Result<(), String> {
        let client_record = state
            .clients
            .get(evidence.client)
            .ok_or_else(|| "client 记录不得被删除".to_owned())?;
        if client_record.alive {
            return Err(format!("client {} 必须已关闭（仍 alive）", evidence.client));
        }
        let surface_record = state
            .surfaces
            .get(evidence.surface)
            .ok_or_else(|| "surface 记录不得被删除".to_owned())?;
        if surface_record.alive {
            return Err(format!(
                "surface {} 必须已关闭（仍 alive）",
                evidence.surface
            ));
        }
        if surface_record.client != Some(evidence.client) {
            return Err(format!(
                "surface {} 的 client 归属必须保持为 {}，实际 {:?}",
                evidence.surface, evidence.client, surface_record.client
            ));
        }
        let window_record = state
            .registry
            .get(evidence.window)
            .ok_or_else(|| "window 记录不得被删除".to_owned())?;
        if window_record.alive {
            return Err(format!("window {} 必须已关闭（仍 alive）", evidence.window));
        }
        if state
            .compositor
            .workspaces
            .iter()
            .any(|workspace| workspace.window_ids().contains(&evidence.window))
        {
            return Err(format!(
                "window {} 不得残留于任何 workspace 的 slot 或 stack",
                evidence.window
            ));
        }
        for window in &baseline.windows {
            if !state.registry.is_alive(*window) {
                return Err(format!("启动基线 window {window} 必须仍存活"));
            }
        }
        for (workspace_id, expected) in &baseline.workspace_windows {
            let mut actual: Vec<u64> = state
                .compositor
                .workspaces
                .iter()
                .find(|workspace| workspace.id == *workspace_id)
                .map(|workspace| workspace.window_ids())
                .unwrap_or_default();
            actual.sort_unstable();
            if &actual != expected {
                return Err(format!(
                    "workspace {workspace_id} 的基线归属必须保持: 期望 {expected:?} 实际 {actual:?}"
                ));
            }
        }
        if !state.validate().is_clean() {
            return Err("关闭确认时 State 校验必须干净".to_owned());
        }
        Ok(())
    }

    /// 持续循环的观测累积：present、确切身份、render/loop 故障都只记事实，不吞错误。
    #[derive(Default)]
    struct SustainedObservation {
        presented: Option<RuntimeShmRenderAttemptReport>,
        first_present_pump: Option<usize>,
        admission: Option<AdmissionEvidence>,
        admission_error: Option<String>,
        render_failed: Option<String>,
        rejected_seen: usize,
    }

    /// 单次 pump ＋ render/身份观测；pump 错误以 `Err` 返回，由调用方记录后停止推进。
    ///
    /// 首次观察到真实 present 后立刻打印 present 证据行，并从此按 pump 尝试捕获本次
    /// 运行的确切 client/surface/window 身份（关闭验证只认该身份）。
    fn pump_and_observe(
        coordinator: &mut NestedRuntimeCoordinator,
        state: &mut State,
        baseline: &IdentityBaseline,
        pumps: &mut usize,
        obs: &mut SustainedObservation,
    ) -> Result<(), String> {
        *pumps += 1;
        let report = coordinator.pump_once_with_live_toplevel_admission_and_unmap_drain(
            state,
            PUMP_TIMEOUT,
            RuntimeToplevelAdmissionDrainTick::phase52y_default(*pumps as u64),
        );
        if !report.lifecycle_report.errors.is_empty() {
            return Err(format!(
                "coordinator pump 失败: {:?}",
                report.lifecycle_report.errors
            ));
        }
        let render = coordinator.last_shm_render_attempt();
        match render.outcome {
            RuntimeShmRenderAttemptOutcome::Presented => {
                if obs.presented.is_none() {
                    obs.presented = Some(render.clone());
                    obs.first_present_pump = Some(*pumps);
                    println!("R2 sustained first present: pump={}", *pumps);
                }
            }
            RuntimeShmRenderAttemptOutcome::RenderFailed => {
                if obs.render_failed.is_none() {
                    obs.render_failed = Some(
                        render
                            .error
                            .clone()
                            .unwrap_or_else(|| "render failed 无错误文本".to_owned()),
                    );
                }
            }
            RuntimeShmRenderAttemptOutcome::Rejected(_) => {
                // 拒绝/tombstone 路径（如断连后迟到 commit）：计数但不谎称成功。
                obs.rejected_seen = obs.rejected_seen.saturating_add(1);
            }
            RuntimeShmRenderAttemptOutcome::Idle | RuntimeShmRenderAttemptOutcome::Deferred => {}
        }
        if obs.presented.is_some() && obs.admission.is_none() {
            match capture_present_admission(state, baseline) {
                Ok(evidence) => {
                    obs.admission = Some(evidence);
                    obs.admission_error = None;
                    println!(
                        "R2 sustained admission captured: client={} surface={} window={}",
                        evidence.client, evidence.surface, evidence.window
                    );
                }
                Err(error) => obs.admission_error = Some(error),
            }
        }
        Ok(())
    }

    /// 持续可见最小 Green：首帧后维持 nested Winit 窗口直到显式停止。
    ///
    /// 只表示受控 nested 窗口在测试期间维持已提交内容；不宣称 long-running compositor、
    /// production renderer、完整 buffer lifecycle 或桌面可用。失败时不发虚假 frame done。
    ///
    /// 正常 stop：先请求 client 销毁 → 服务端 socket 保持可用并继续处理销毁请求 →
    /// 有界确认本次运行捕获的确切 client/surface/window 已关闭且布局移除 → 关闭
    /// coordinator/socket → 最后 join client 线程（join 无法被强制中止，不宣称硬超时）。
    ///
    /// `--disconnect-test`：client 首帧后只做 transport shutdown（不发 destroy、不 flush），
    /// 主循环仍持续 pump；成功行只在客户端 transport shutdown、服务端 State 关闭验证
    /// （tombstone／归属／布局移除／基线／validation）与本次 socket/SHM 残留核对
    /// 全部通过后打印，两侧证据分开表述。
    /// owner：socket 由 coordinator 拥有，SHM 由 client 线程拥有，output/texture 由
    /// coordinator 内 Winit owner 拥有并保留首 buffer 至 drop。
    fn run_sustained(disconnect_after_present: bool) -> Result<(), Box<dyn std::error::Error>> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or("controlled runner 需要 XDG_RUNTIME_DIR")?;
        if !runtime_dir.is_dir() {
            return Err("XDG_RUNTIME_DIR 必须是已存在目录".into());
        }
        let socket_name = format!("wayland-sky-mirror-r2-{}", std::process::id());
        let socket_path = runtime_dir.join(&socket_name);
        let mut coordinator =
            NestedRuntimeCoordinator::with_production_protocol_bootstrap(&socket_name)?;
        coordinator.initialize_winit_output_on_current_thread()?;
        let mut state = State::new();
        // 启动前只读基线：关闭验证只认本次运行捕获的确切身份与该基线的差分/保持性。
        let baseline = snapshot_alive_identities(&state);
        let stop_flag = Arc::new(AtomicBool::new(false));
        // 正常模式启用 stdin 停止门（区分正常 stop 与输入错误）；断连测试不读 stdin。
        let stop_signals = if disconnect_after_present {
            None
        } else {
            let (signal_sender, signal_receiver) = mpsc::channel();
            spawn_stdin_stop_reader(stop_flag.clone(), signal_sender);
            Some(signal_receiver)
        };
        let (client_sender, client_receiver) = mpsc::channel();
        let (shm_path_sender, shm_path_receiver) = mpsc::channel();
        let client_socket_path = socket_path.clone();
        let client_runtime_dir = runtime_dir.clone();
        let client_stop = stop_flag.clone();
        let overall_deadline = Instant::now() + SUSTAINED_DEADLINE;
        let client_thread = thread::spawn(move || {
            let result = run_external_client_sustained(
                &client_socket_path,
                &client_runtime_dir,
                client_stop,
                overall_deadline,
                disconnect_after_present,
                shm_path_sender,
            );
            let _ = client_sender.send(result);
        });

        let mut obs = SustainedObservation::default();
        let mut client_result: Option<Result<SustainedClientResult, String>> = None;
        let mut loop_error: Option<String> = None;
        let mut input_error: Option<String> = None;
        let mut stop_reason = "unknown";
        let mut pumps = 0usize;
        // ===== 主循环：pump 观测 → 停止信号 → client 结果；真实错误立即记录并停止推进 =====
        while Instant::now() < overall_deadline && pumps < SUSTAINED_MAX_PUMPS {
            if let Err(error) = pump_and_observe(
                &mut coordinator,
                &mut state,
                &baseline,
                &mut pumps,
                &mut obs,
            ) {
                loop_error = Some(error);
                stop_reason = "pump_error";
                break;
            }
            if obs.render_failed.is_some() {
                // 真实 import/render/submit 失败：不发虚假 done，保留错误证据后有界退出。
                stop_reason = "render_failed";
                break;
            }
            if let Some(signals) = &stop_signals {
                match signals.try_recv() {
                    Ok(StopSignal::Requested { origin }) => {
                        // 显式停止：只有已首帧 present 才允许成为成功候选。
                        stop_reason = if obs.presented.is_some() {
                            "stop_requested"
                        } else {
                            "stop_before_present"
                        };
                        println!(
                            "R2 sustained stop accepted: origin={origin} reason={stop_reason} first_present_pump={:?}",
                            obs.first_present_pump
                        );
                        break;
                    }
                    Ok(StopSignal::InputError(message)) => {
                        // 输入协议/读取错误：与正常 stop 严格区分，必须以失败收场。
                        input_error = Some(message.clone());
                        stop_reason = "stop_input_error";
                        println!("R2 sustained stop input error: {message}");
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        input_error =
                            Some("stdin 停止 reader 线程异常结束（未发出信号）".to_owned());
                        stop_reason = "stop_input_error";
                        break;
                    }
                }
            }
            match client_receiver.try_recv() {
                Ok(result) => {
                    let failed = result.is_err();
                    // 断连测试必须以 transport shutdown 收尾；出现有序销毁即为收尾方式不符。
                    let teardown_ok = match &result {
                        Ok(client) => {
                            client.teardown
                                == if disconnect_after_present {
                                    ClientTeardown::TransportShutdown
                                } else {
                                    ClientTeardown::OrderedDestroy
                                }
                        }
                        Err(_) => false,
                    };
                    client_result = Some(result);
                    if disconnect_after_present {
                        // 主动断连测试：client 已返回（客户端动作=transport shutdown 已发出）；
                        // 服务端是否观察到状态关闭由后续 closure 验证单独证明，两者不互相冒充。
                        stop_reason = if failed {
                            "client_disconnect_err"
                        } else if !teardown_ok {
                            // 未按 transport shutdown 收尾（如回退到有序 destroy）不算成功断连。
                            "client_disconnect_wrong_teardown"
                        } else if obs.presented.is_none() {
                            "client_disconnect_before_present"
                        } else {
                            println!(
                                "R2 sustained client transport shutdown sent: first_present_pump={:?}",
                                obs.first_present_pump
                            );
                            "client_disconnect"
                        };
                        break;
                    }
                    if failed {
                        stop_reason = "client_error";
                        break;
                    }
                    if !teardown_ok {
                        // 正常 stop 路径必须保持有序 destroy 收尾，不得被断连收尾替换。
                        stop_reason = "client_wrong_teardown";
                        break;
                    }
                    if stop_flag.load(Ordering::SeqCst) {
                        // reader 线程先 send 后置位：下一轮停止信号必然可取，不在此判定成功。
                        continue;
                    }
                    stop_reason = "client_early_return";
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    loop_error = Some("外部 client 未返回结果即断开".to_owned());
                    stop_reason = "client_channel_disconnect";
                    break;
                }
            }
        }
        if stop_reason == "unknown" {
            if Instant::now() >= overall_deadline {
                stop_reason = "timeout";
            } else if pumps >= SUSTAINED_MAX_PUMPS {
                stop_reason = "max_pumps";
            }
        }
        // ===== 有序 teardown =====
        // 1) 先请求 client 销毁；正常停止、输入错误与超时同样适用，失败路径也必须回收。
        stop_flag.store(true, Ordering::SeqCst);
        let mut teardown_error: Option<String> = None;
        // 2) 服务端 socket 保持可用并继续 pump，有界等待 client 返回销毁结果。
        let teardown_deadline = Instant::now() + TEARDOWN_WAIT;
        while client_result.is_none()
            && Instant::now() < teardown_deadline
            && pumps < SUSTAINED_MAX_PUMPS
        {
            if let Err(error) = pump_and_observe(
                &mut coordinator,
                &mut state,
                &baseline,
                &mut pumps,
                &mut obs,
            ) {
                teardown_error = Some(error);
                break;
            }
            match client_receiver.try_recv() {
                Ok(result) => client_result = Some(result),
                Err(mpsc::TryRecvError::Empty) => {}
                // 线程已结束但无结果：交由下方 join 判定 panic，不再空等。
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
        // 3) 有界排空并确认本次运行的确切 client/surface/window 已关闭（tombstone 保留、
        //    alive=false、归属保持）、布局移除、基线保持、validation 干净；该确认必须发生在
        //    关闭 coordinator/socket 之前，不能只依赖全局 is_clean() 或宽泛文件计数。
        let mut closure_check: Result<(), String> = if obs.admission.is_some() {
            Err("尚未开始关闭确认".to_owned())
        } else if obs.presented.is_some() {
            Err(format!(
                "已 present 但未捕获确切身份: {}",
                obs.admission_error
                    .clone()
                    .unwrap_or_else(|| "未知".to_owned())
            ))
        } else {
            Err("未完成首帧 present，无关闭确认对象".to_owned())
        };
        if let Some(ids) = obs.admission {
            let closure_deadline = Instant::now() + CLOSURE_WAIT;
            loop {
                closure_check = verify_identity_closed(&state, &baseline, &ids);
                if closure_check.is_ok() {
                    break;
                }
                if teardown_error.is_some()
                    || Instant::now() >= closure_deadline
                    || pumps >= SUSTAINED_MAX_PUMPS
                {
                    break;
                }
                if let Err(error) = pump_and_observe(
                    &mut coordinator,
                    &mut state,
                    &baseline,
                    &mut pumps,
                    &mut obs,
                ) {
                    teardown_error = Some(error);
                    break;
                }
            }
        }
        // 4) 关闭确认（尽力且有界）完成后，drop coordinator 关闭 socket。
        drop(coordinator);
        // 5) 最后 join client 线程。join 无法被强制中止：此处不宣称硬超时，只保证
        //    client 侧各阶段都受自身绝对 deadline 与轮次预算约束。
        let client_panicked = client_thread.join().is_err();
        if client_result.is_none() {
            client_result = client_receiver
                .recv_timeout(Duration::from_millis(500))
                .ok();
        }
        // 6) 只核对本次运行确切拥有的 socket/`.lock`/SHM；目录读取失败必须算失败，
        //    不按前缀误认；`shm_path` 缺失在核对内部失败关闭，不静默跳过。
        let shm_path = shm_path_receiver.try_recv().ok();
        let resource_check = residue::ensure_owned_resources_reclaimed(
            &runtime_dir,
            &socket_path,
            shm_path.as_deref(),
        );
        // ===== 统一裁决（受控证明，非 production）：聚合全部失败，任何局部证据
        // 不得外推整体成功；错误路径也必须已在上方完成有界回收。 =====
        let mut failures: Vec<String> = Vec::new();
        if let Some(error) = &loop_error {
            failures.push(format!("主循环错误：{error}"));
        }
        if let Some(error) = &input_error {
            // 输入错误必须与正常 stop 严格区分，绝不能被上报为成功停止。
            failures.push(format!("stdin 停止输入错误：{error}"));
        }
        if let Some(error) = &obs.render_failed {
            failures.push(format!(
                "import/render/submit 失败（无虚假 frame done）：{error}"
            ));
        }
        if let Some(error) = &teardown_error {
            failures.push(format!("teardown 失败：{error}"));
        }
        if client_panicked {
            failures.push("client 线程 panic，join 未取得结果".to_owned());
        }
        match &client_result {
            None => failures.push("join 后仍没有 client 结果".to_owned()),
            Some(Err(error)) => failures.push(format!("外部 client 失败：{error}")),
            Some(Ok(_)) => {}
        }
        // 停止原因门禁：正常模式必须是 present 后的显式 stop；断连测试必须是
        // present 后的确切断连（before_present / err 变体一律不算成功）。
        let expected_stop = if disconnect_after_present {
            "client_disconnect"
        } else {
            "stop_requested"
        };
        if failures.is_empty() && stop_reason != expected_stop {
            failures.push(format!(
                "停止原因不符：stop={stop_reason} 期望={expected_stop} pumps={pumps}"
            ));
        }
        // present 门禁：两种模式都必须先有首帧 present。
        if obs.presented.is_none() {
            failures.push("持续可见未观察到首帧 present".to_owned());
        }
        // 关闭确认：本次运行确切 client/surface/window 的 tombstone 保留、alive=false、
        // 归属保持、布局移除、基线保持与 validation 干净（在关 socket 前已完成）。
        if obs.presented.is_some() {
            if let Err(error) = &closure_check {
                failures.push(format!("身份关闭确认失败：{error}"));
            }
        }
        // 残留核对：只按本进程精确资源名（socket 精确名 + client 上报的精确 SHM 路径），
        // 目录读取失败必须算失败，不按前缀计数误认。
        if let Err(error) = &resource_check {
            failures.push(format!("残留核对失败：{error}"));
        }
        // 闭环不变量：present 且 client Ok 时必须满足既有不变量（含 expected_rgb
        // 读回、frame done 恰好一次与 core clean）。client 证据从 SustainedClientResult 取。
        if let (Some(presented), Some(Ok(client))) = (&obs.presented, &client_result) {
            let client = &client.evidence;
            // 完整 256×256 四象限回读期望：与 SHM 字节同源派生，整幅逐像素比较。
            let expected_rgb = shm_pattern::expected_readback_rgb();
            if !presented.resource_transferred
                || !presented.buffer_imported
                || !presented.texture_drawn
                || !presented.texture_read_back
                || presented.texture_readback_rgb != expected_rgb
                || !presented.backbuffer_submitted
                || !presented.output_damage_submitted
                || presented.frame_callbacks_done != 1
                || !client.buffer_commit_sent
                || !client.damage_sent
                || !client.frame_requested
                || !client.frame_done
                || !state.validate().is_clean()
            {
                // 失败消息只输出布尔标志与回读差异摘要，不倾倒整幅像素向量。
                failures.push(format!(
                    "R2 sustained proof 不满足闭环不变量: flags=transfer:{} import:{} drawn:{} readback:{} submit:{} damage:{} frame_done={} client=commit:{} damage:{} frame_req:{} frame_done:{} core_clean={} readback[{}]",
                    presented.resource_transferred,
                    presented.buffer_imported,
                    presented.texture_drawn,
                    presented.texture_read_back,
                    presented.backbuffer_submitted,
                    presented.output_damage_submitted,
                    presented.frame_callbacks_done,
                    client.buffer_commit_sent,
                    client.damage_sent,
                    client.frame_requested,
                    client.frame_done,
                    state.validate().is_clean(),
                    describe_readback_mismatch(
                        &presented.texture_readback_rgb,
                        &expected_rgb,
                        shm_pattern::WIDTH as usize
                    ),
                ));
            }
        }

        if failures.is_empty() {
            // 全部门禁通过才打印成功行；ids 为本次运行捕获的确切身份。
            let ids = obs.admission.expect("门禁通过必须已捕获本次运行的确切身份");
            let frame_done = obs
                .presented
                .as_ref()
                .map(|report| report.frame_callbacks_done)
                .unwrap_or(0);
            if disconnect_after_present {
                // 成功行严格区分两侧证据：client_transport_shutdown 只表示客户端发出了
                // 双向 shutdown（且未发 destroy，由 client_wrong_teardown 门禁保证）；
                // server_* 字段全部来自同一 State 的关闭验证（tombstone alive=false、
                // 归属保持、布局移除、基线保持、validation 干净）与本次 socket/SHM
                // 残留核对。任一缺失都不打印成功行。
                println!(
                    "R2 sustained disconnect clean: first_present_pump={:?} pumps={pumps} ids=client={} surface={} window={} client_transport_shutdown=true server_state_closed=true server_layout_removed=true server_core_clean=true socket_clean=true lock_clean=true shm_clean=true",
                    obs.first_present_pump, ids.client, ids.surface, ids.window
                );
            } else {
                println!(
                    "R2 sustained visible: pumps={pumps} first_present_pump={:?} stop={stop_reason} rejected_seen={} ids=client={} surface={} window={} client_state_closed=true socket_clean=true lock_clean=true shm_clean=true frame_done={frame_done} core_clean=true",
                    obs.first_present_pump, obs.rejected_seen, ids.client, ids.surface, ids.window
                );
            }
            Ok(())
        } else {
            Err(format!("sustained 裁决失败：{}", failures.join("；")).into())
        }
    }

    pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|arg| arg == "--sustained") {
            let disconnect = args.iter().any(|arg| arg == "--disconnect-test");
            return run_sustained(disconnect);
        }
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or("controlled runner 需要 XDG_RUNTIME_DIR")?;
        if !runtime_dir.is_dir() {
            return Err("XDG_RUNTIME_DIR 必须是已存在目录".into());
        }
        let socket_name = format!("wayland-sky-mirror-r2-{}", std::process::id());
        let socket_path = runtime_dir.join(&socket_name);
        let mut coordinator =
            NestedRuntimeCoordinator::with_production_protocol_bootstrap(&socket_name)?;
        coordinator.initialize_winit_output_on_current_thread()?;
        let mut state = State::new();
        let (client_sender, client_receiver) = mpsc::channel();
        let (shm_path_sender, shm_path_receiver) = mpsc::channel();
        let client_socket_path = socket_path.clone();
        let client_runtime_dir = runtime_dir.clone();
        let client_thread = thread::spawn(move || {
            let result =
                run_external_client(&client_socket_path, &client_runtime_dir, shm_path_sender);
            let _ = client_sender.send(result);
        });

        let deadline = Instant::now() + RUN_DEADLINE;
        let mut presented: Option<RuntimeShmRenderAttemptReport> = None;
        let mut client_result: Option<Result<ExternalClientEvidence, String>> = None;
        let mut loop_error = None;
        let mut pumps = 0usize;
        while Instant::now() < deadline && pumps < MAX_PUMPS {
            pumps += 1;
            let report = coordinator.pump_once_with_live_toplevel_admission_and_unmap_drain(
                &mut state,
                PUMP_TIMEOUT,
                RuntimeToplevelAdmissionDrainTick::phase52y_default(pumps as u64),
            );
            if !report.lifecycle_report.errors.is_empty() {
                loop_error = Some(format!(
                    "coordinator pump 失败: {:?}",
                    report.lifecycle_report.errors
                ));
                break;
            }
            let render = coordinator.last_shm_render_attempt();
            if render.outcome == RuntimeShmRenderAttemptOutcome::Presented {
                presented = Some(render.clone());
            }
            match client_receiver.try_recv() {
                Ok(result) => {
                    let failed = result.is_err();
                    client_result = Some(result);
                    if failed {
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    loop_error = Some("外部 client 未返回结果即断开".to_owned());
                    break;
                }
            }
            if presented.is_some() && client_result.is_some() {
                break;
            }
        }

        // 先 drop server owner 关闭 socket，让任何仍在 protocol read 的 client 确定退出；
        // 随后 join，不允许测试线程脱离。正常路径 client 已收到 frame done 并自行 cleanup。
        drop(coordinator);
        let client_thread_panicked = client_thread.join().is_err();
        if client_result.is_none() {
            client_result = client_receiver.recv_timeout(Duration::from_millis(50)).ok();
        }
        if client_thread_panicked {
            return Err("外部 client thread panic".into());
        }
        if let Some(error) = loop_error {
            return Err(error.into());
        }
        // 残留核对：socket 精确名 + `<socket>.lock` + client 上报的精确 SHM 路径；
        // `shm_path` 缺失（如 client 未跑到建池）在核对内部失败关闭，不静默跳过；
        // 从不核对或删除宿主自己的 `wayland-1`。
        let shm_path = shm_path_receiver.try_recv().ok();
        if let Err(error) = residue::ensure_owned_resources_reclaimed(
            &runtime_dir,
            &socket_path,
            shm_path.as_deref(),
        ) {
            return Err(format!("残留核对失败：{error}").into());
        }
        let presented = presented.ok_or("deadline 内未完成真实 client frame present")?;
        let client = client_result
            .ok_or("外部 client join 后仍没有结果")?
            .map_err(|error| format!("外部 client 失败（cleanup 后返回）: {error}"))?;
        // 完整 256×256 四象限回读期望：与 SHM 字节同源派生，整幅逐像素比较。
        let expected_rgb = shm_pattern::expected_readback_rgb();
        if !presented.resource_transferred
            || !presented.buffer_imported
            || !presented.texture_drawn
            || !presented.texture_read_back
            || presented.texture_readback_rgb != expected_rgb
            || !presented.backbuffer_submitted
            || !presented.output_damage_submitted
            || presented.frame_callbacks_done != 1
            || !client.buffer_commit_sent
            || !client.damage_sent
            || !client.frame_requested
            || !client.frame_done
            || !state.validate().is_clean()
        {
            // 失败消息只输出布尔标志与回读差异摘要，不倾倒整幅像素向量。
            return Err(format!(
                "R2 controlled proof 不满足闭环不变量: flags=transfer:{} import:{} drawn:{} readback:{} submit:{} damage:{} frame_done={} client=commit:{} damage:{} frame_req:{} frame_done:{} core_clean={} readback[{}]",
                presented.resource_transferred,
                presented.buffer_imported,
                presented.texture_drawn,
                presented.texture_read_back,
                presented.backbuffer_submitted,
                presented.output_damage_submitted,
                presented.frame_callbacks_done,
                client.buffer_commit_sent,
                client.damage_sent,
                client.frame_requested,
                client.frame_done,
                state.validate().is_clean(),
                describe_readback_mismatch(
                    &presented.texture_readback_rgb,
                    &expected_rgb,
                    shm_pattern::WIDTH as usize
                ),
            )
            .into());
        }

        println!(
            "R2 controlled SHM frame: pumps={pumps} import={} pixels=verified texture_drawn={} submit={} damage={} frame_done={} socket_clean=true lock_clean=true shm_clean=true",
            presented.buffer_imported,
            presented.texture_drawn,
            presented.backbuffer_submitted,
            presented.output_damage_submitted,
            presented.frame_callbacks_done,
        );
        Ok(())
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    controlled_runner::run()
}

/// default、非 Linux 与 `cargo test` 不创建图形或 protocol resource。
#[cfg(any(not(all(feature = "smithay-linux", target_os = "linux")), test))]
fn main() {
    eprintln!(
        "sky_mirror_controlled_winit_first_frame 需要 Linux smithay-linux feature 并直接运行"
    );
}

/// 256×256 四象限 SHM 图案的字节/布局契约测试（纯数据，不创建任何图形或 protocol 资源）。
///
/// 该测试只证明图案尺寸派生、行主序 BGRX 布局与回读期望一致性；它不证明用户可见、
/// 不证明渲染或 present，也不冒充完整链路验收。
#[cfg(test)]
mod shm_pattern_layout_tests {
    use crate::shm_pattern::{BYTE_LEN, HEIGHT, STRIDE, WIDTH, bytes, expected_readback_rgb};

    /// 准备并生成完整图案字节；断言统一派生的尺寸与四象限行主序布局。
    /// Red 依据：旧 2×2 实现下长度与任一象限颜色都会失败。
    #[test]
    fn quadrant_pattern_bytes_are_256x256_row_major() {
        let bytes = bytes();
        assert_eq!(WIDTH, 256);
        assert_eq!(HEIGHT, 256);
        assert_eq!(STRIDE, WIDTH * 4);
        assert_eq!(BYTE_LEN, STRIDE * HEIGHT);
        assert_eq!(BYTE_LEN, 262_144);
        assert_eq!(bytes.len(), 262_144);
        for y in 0..256usize {
            for x in 0..256usize {
                // 期望值独立硬编码：左上红、右上绿、左下蓝、右下白（B,G,R,X 小端）。
                let expected = match (y < 128, x < 128) {
                    (true, true) => [0x00, 0x00, 0xFF, 0x00],
                    (true, false) => [0x00, 0xFF, 0x00, 0x00],
                    (false, true) => [0xFF, 0x00, 0x00, 0x00],
                    (false, false) => [0xFF, 0xFF, 0xFF, 0x00],
                };
                let offset = y * 1024 + x * 4;
                let actual = [
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ];
                assert_eq!(actual, expected, "像素 (x={x}, y={y}) 的 BGRX 布局不符");
            }
        }
    }

    /// 准备回读期望；断言锚点颜色、行主序（无翻转）且逐像素由同一图案字节派生。
    /// Red 依据：旧 2×2 期望只有 4 个像素且锚点索引越界/不符。
    #[test]
    fn expected_readback_rgb_matches_pattern_bytes() {
        let bytes = bytes();
        let readback = expected_readback_rgb();
        assert_eq!(readback.len(), 65_536);
        assert_eq!(readback[0], [0xFF, 0x00, 0x00]);
        assert_eq!(readback[127], [0xFF, 0x00, 0x00]);
        assert_eq!(readback[128], [0x00, 0xFF, 0x00]);
        assert_eq!(readback[255], [0x00, 0xFF, 0x00]);
        assert_eq!(readback[32_768], [0x00, 0x00, 0xFF]);
        assert_eq!(readback[65_535], [0xFF, 0xFF, 0xFF]);
        for (index, pixel) in readback.iter().enumerate() {
            let offset = index * 4;
            assert_eq!(
                *pixel,
                [bytes[offset + 2], bytes[offset + 1], bytes[offset]],
                "回读索引 {index} 必须由同一行主序字节派生（B,G,R,X -> R,G,B）"
            );
        }
    }
}

/// 残留核对契约测试（只用临时目录里的普通文件，不创建任何图形或 protocol 资源）。
///
/// 该测试只证明“运行结束后按精确文件名核对 socket/`.lock`/SHM”的语义；它不删除任何
/// 真实运行目录、不证明窗口可见、也不冒充完整链路验收。
#[cfg(test)]
mod residue_reclaim_tests {
    use std::{
        fs,
        io::ErrorKind,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::residue::ensure_owned_resources_reclaimed;

    /// 候选路径：`sky-r2-residue-{tag}-{pid}-{attempt}`。
    ///
    /// 路径只由 tag、进程 ID 与候选序号决定，因此测试可以确定性地排他预建某个候选，
    /// 再验证 fixture 不会动那个并非本次调用创建的目录。
    fn candidate_path(tag: &str, attempt: usize) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sky-r2-residue-{tag}-{}-{attempt}",
            std::process::id()
        ))
    }

    /// 准备：唯一命名的临时运行目录；cleanup：`Drop` 删除整个目录，不留测试残留。
    ///
    /// 只有本次调用成功创建的目录才归本 fixture 所有，`Drop` 也只清理那个路径。
    struct TempRuntimeDir(PathBuf);

    impl TempRuntimeDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            Self::create(tag, COUNTER.fetch_add(1, Ordering::Relaxed))
        }

        /// 从 `start_attempt` 起逐个候选名**排他**创建临时目录。
        ///
        /// 只有本次 `fs::create_dir` 成功返回的目录才归本 fixture 所有；`AlreadyExists`
        /// 说明该路径不是本次创建的，只能换下一个候选名重试，绝不 `remove_dir_all`
        /// 预清理任何未经本次调用成功创建的路径。其余 I/O 错误直接失败，不静默降级。
        ///
        /// # Panics
        ///
        /// 预算内候选名全部被占用，或 `create_dir` 返回 `AlreadyExists` 以外的错误时
        /// panic——两者都不得被伪装成“创建成功”。
        fn create(tag: &str, start_attempt: usize) -> Self {
            const ATTEMPTS: usize = 64;
            for offset in 0..ATTEMPTS {
                let path = candidate_path(tag, start_attempt.saturating_add(offset));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("创建临时运行目录失败 {}: {error}", path.display()),
                }
            }
            panic!("连续 {ATTEMPTS} 个候选名都已存在，放弃创建 {tag} 临时运行目录");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRuntimeDir {
        /// 只清理本次 `create_dir` 成功创建的 `self.0`；`self.0` 永远由本 fixture 自己
        /// 创建，因此这里不存在误删他人目录的可能。
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// 本次运行 socket 的精确文件名（与 runner 同构：无扩展名）。
    fn socket_path(dir: &Path) -> PathBuf {
        dir.join("wayland-sky-mirror-r2-424242")
    }

    /// 执行：仅存在宿主自己的 `wayland-1` 别名时必须通过；断言：精确名核对绝不误认。
    /// Red 依据：若按前缀/计数核对，宿主 `wayland-1` 会被误报为本次运行残留。
    #[test]
    fn host_wayland_socket_alias_is_not_flagged() {
        let dir = TempRuntimeDir::new("host-alias");
        fs::write(dir.path().join("wayland-1"), b"host").expect("写入宿主别名失败");
        let socket = socket_path(dir.path());
        let shm = dir.path().join("sky-mirror-r2-shm-424242.bin");
        let result = ensure_owned_resources_reclaimed(dir.path(), &socket, Some(&shm));
        assert_eq!(
            result,
            Ok(()),
            "宿主 wayland-1 不得被标记为本次残留: {result:?}"
        );
    }

    /// 执行：只存在 `<socket>.lock` 时核对必须失败；断言：错误文本点名 lock。
    /// Red 依据：旧核对完全不检查 `.lock`，此时返回 `Ok(())`。
    #[test]
    fn socket_lock_residue_is_reported() {
        let dir = TempRuntimeDir::new("lock");
        let socket = socket_path(dir.path());
        let lock = socket.with_extension("lock");
        fs::write(&lock, b"").expect("写入 .lock 失败");
        let shm = dir.path().join("sky-mirror-r2-shm-424242.bin");
        let error = ensure_owned_resources_reclaimed(dir.path(), &socket, Some(&shm))
            .expect_err(".lock 残留必须核对失败");
        assert!(
            error.contains("lock") && error.contains("wayland-sky-mirror-r2-424242.lock"),
            "错误必须点名残留的 .lock 精确路径，实际: {error}"
        );
    }

    /// 执行：只存在本次 socket 时核对必须失败；断言：错误文本点名 socket。
    #[test]
    fn socket_residue_is_reported() {
        let dir = TempRuntimeDir::new("socket");
        let socket = socket_path(dir.path());
        fs::write(&socket, b"").expect("写入 socket 文件失败");
        let shm = dir.path().join("sky-mirror-r2-shm-424242.bin");
        let error = ensure_owned_resources_reclaimed(dir.path(), &socket, Some(&shm))
            .expect_err("socket 残留必须核对失败");
        assert!(
            error.contains("socket"),
            "错误必须点名 socket，实际: {error}"
        );
    }

    /// 执行：只存在本次 SHM 时核对必须失败；断言：错误文本点名 SHM。
    #[test]
    fn shm_residue_is_reported() {
        let dir = TempRuntimeDir::new("shm");
        let socket = socket_path(dir.path());
        let shm = dir.path().join("sky-mirror-r2-shm-424242.bin");
        fs::write(&shm, [0u8; 16]).expect("写入 SHM 文件失败");
        let error = ensure_owned_resources_reclaimed(dir.path(), &socket, Some(&shm))
            .expect_err("SHM 残留必须核对失败");
        assert!(error.contains("SHM"), "错误必须点名 SHM，实际: {error}");
    }

    /// 执行：目录干净但未取得本次 SHM 路径时核对必须失败；断言：失败关闭而非跳过。
    /// Red 依据：旧实现对 `shm_path=None` 直接跳过 SHM 检查并返回 `Ok(())`。
    #[test]
    fn missing_shm_path_fails_closed() {
        let dir = TempRuntimeDir::new("missing-shm");
        let socket = socket_path(dir.path());
        let error = ensure_owned_resources_reclaimed(dir.path(), &socket, None)
            .expect_err("未取得 SHM 路径必须失败关闭");
        assert!(
            error.contains("SHM"),
            "错误必须点名无法核对 SHM，实际: {error}"
        );
    }

    /// 执行：socket、`.lock` 与 SHM 同时残留；断言：一次错误聚合全部三项。
    /// Red 依据：旧实现在首个 socket 残留处提前返回，`.lock` 与 SHM 不会被报告。
    #[test]
    fn all_residues_are_aggregated_into_one_error() {
        let dir = TempRuntimeDir::new("aggregate");
        let socket = socket_path(dir.path());
        let shm = dir.path().join("sky-mirror-r2-shm-424242.bin");
        fs::write(&socket, b"").expect("写入 socket 文件失败");
        fs::write(socket.with_extension("lock"), b"").expect("写入 .lock 失败");
        fs::write(&shm, [0u8; 16]).expect("写入 SHM 文件失败");
        let error = ensure_owned_resources_reclaimed(dir.path(), &socket, Some(&shm))
            .expect_err("全部残留必须聚合报告");
        assert!(
            error.contains("socket") && error.contains("lock") && error.contains("SHM"),
            "错误必须同时包含 socket/`.lock`/SHM，实际: {error}"
        );
    }

    /// Red：候选目录已存在时 fixture 必须改用下一个候选名，绝不能预删既有目录。
    ///
    /// 准备：按同一命名规则**排他预建**候选目录并放入 sentinel 与原内容（只用
    /// `create_dir`，绝不用 `remove_dir_all` 制造初态）；执行：从同一候选序号创建
    /// fixture，再 `Drop` 它；断言：既有目录、sentinel 与原内容全程存在，fixture 拿到
    /// 的是另一个候选目录，且 `Drop` 只清理本次新建目录。
    /// Red 依据：旧 fixture 在创建前 `remove_dir_all(&path)`，会递归删除这个并非本次
    /// 调用创建的目录 → sentinel 断言失败。
    #[test]
    fn existing_candidate_dir_is_never_pre_deleted() {
        // 准备：取本 tag 下第一个可用候选序号，由本测试排他创建（不删任何既有路径）。
        let tag = "collide";
        let mut attempt = 0usize;
        let existing = loop {
            assert!(
                attempt < 64,
                "候选序号耗尽：同 tag 的残留目录过多，拒绝继续"
            );
            let candidate = candidate_path(tag, attempt);
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == ErrorKind::AlreadyExists => attempt += 1,
                Err(error) => panic!("预建碰撞目录失败 {}: {error}", candidate.display()),
            }
        };
        fs::write(existing.join("sentinel.txt"), b"do-not-delete").expect("写入 sentinel 失败");
        fs::write(existing.join("keep.txt"), b"original").expect("写入原内容失败");

        // 执行：从同一候选序号创建 fixture，它必然与既有目录碰撞。
        let dir = TempRuntimeDir::create(tag, attempt);

        // 断言：既有目录及其内容必须原样存在。
        assert!(
            existing.join("sentinel.txt").exists(),
            "既有目录的 sentinel 必须仍然存在：fixture 不得删除并非本次创建的路径"
        );
        assert_eq!(
            fs::read(existing.join("sentinel.txt")).expect("sentinel 必须可读"),
            b"do-not-delete",
            "既有目录 sentinel 内容不得被改动"
        );
        assert_eq!(
            fs::read(existing.join("keep.txt")).expect("原内容必须可读"),
            b"original",
            "既有目录原内容必须原样保留"
        );
        assert_ne!(
            dir.path(),
            &existing,
            "碰撞时必须改用下一个候选名，而不是占用既有目录"
        );
        assert!(dir.path().exists(), "本次 fixture 必须真正创建自己的目录");

        // 断言：`Drop` 只清理本次新建目录，不动碰撞目录。
        let created = dir.path().to_path_buf();
        drop(dir);
        assert!(!created.exists(), "Drop 必须清理本次新建目录");
        assert!(
            existing.join("sentinel.txt").exists(),
            "Drop 不得删除既有碰撞目录"
        );

        // cleanup：本测试自己创建的目录由本测试在收尾时删除（这不是预清理初态）。
        fs::remove_dir_all(&existing).expect("清理本测试自建的碰撞目录失败");
    }
}

/// Winit present 关键步骤的源码守卫（`linux_winit_output.rs` 在 `cargo test` 下不参与
/// 编译，故用 `include_str!` 守卫其源码文本）。
///
/// 该守卫只证明源码包含必要调用，不证明运行时行为；运行时证据由受控 binary 的目标
/// framebuffer 回读门禁与人工截图验收提供。
#[cfg(test)]
mod winit_present_source_guard {
    /// 相对本 binary 文件路径引入 backend 源码文本。
    const LINUX_WINIT_OUTPUT_SOURCE: &str =
        include_str!("../smithay_backend/linux_winit_output.rs");

    /// 执行：检查 present 路径是否泵送宿主 Winit 事件；断言：源码必须包含
    /// `dispatch_new_events`。
    /// Red 依据：旧源码从不调用 `dispatch_new_events`，宿主窗口系统回调不推进。
    #[test]
    fn present_pumps_winit_host_events() {
        assert!(
            LINUX_WINIT_OUTPUT_SOURCE.contains("dispatch_new_events"),
            "present 路径必须泵送宿主 Winit 事件（dispatch_new_events）"
        );
    }

    /// 执行：检查 present 是否在 submit 前读回目标 framebuffer；断言：源码必须包含
    /// `copy_framebuffer`。
    /// 依据：source texture 回读不能证明目标 framebuffer 已得到期望像素。
    #[test]
    fn present_reads_back_target_framebuffer() {
        assert!(
            LINUX_WINIT_OUTPUT_SOURCE.contains("copy_framebuffer"),
            "present 必须在 submit 前读回目标 framebuffer（copy_framebuffer）"
        );
    }
}

/// 回读不匹配诊断助手的契约测试（纯数据，不创建任何图形或 protocol 资源）。
///
/// 该测试只证明失败消息能定位首个差异、关系（行/列/180 度翻转）与全黑状态；它不
/// 证明渲染结果，也不冒充完整链路验收。
#[cfg(test)]
mod readback_mismatch_tests {
    use crate::describe_readback_mismatch;

    /// 2×2 期望：左上红、右上绿、左下蓝、右下白。
    fn expected_2x2() -> Vec<[u8; 3]> {
        vec![
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0xFF, 0xFF],
        ]
    }

    /// 执行：长度不等的回读；断言：消息必须点名长度不符并给出两侧长度。
    #[test]
    fn reports_length_mismatch_with_both_lengths() {
        let actual = vec![[0x00, 0x00, 0x00]; 3];
        let message = describe_readback_mismatch(&actual, &expected_2x2(), 2);
        assert!(
            message.contains("长度"),
            "必须点名长度不符，实际: {message}"
        );
        assert!(
            message.contains('3') && message.contains('4'),
            "必须给出 actual/expected 长度，实际: {message}"
        );
    }

    /// 执行：actual 为期望的上下翻转；断言：消息包含首个差异坐标与行序翻转关系。
    /// 依据：黑窗口失败时需要一次性判定“目标是否只是方向反了”。
    #[test]
    fn reports_first_diff_coordinates_and_row_flip_relation() {
        let expected = expected_2x2();
        let mut actual = expected.clone();
        actual.swap(0, 2);
        actual.swap(1, 3);
        let message = describe_readback_mismatch(&actual, &expected, 2);
        assert!(
            message.contains("index=0"),
            "必须给出首个差异 index，实际: {message}"
        );
        assert!(
            message.contains("x=0") && message.contains("y=0"),
            "必须给出首个差异坐标，实际: {message}"
        );
        assert!(
            message.contains("行序上下翻转"),
            "必须识别行序翻转，实际: {message}"
        );
    }

    /// 执行：actual 全黑（黑窗口最典型形态）；断言：消息点名全黑。
    #[test]
    fn reports_all_black_actual() {
        let actual = vec![[0x00, 0x00, 0x00]; 4];
        let message = describe_readback_mismatch(&actual, &expected_2x2(), 2);
        assert!(message.contains("全黑"), "必须点名全黑，实际: {message}");
    }

    /// 执行：actual 与期望完全相同；断言：消息说明逐像素一致（不匹配另有原因）。
    #[test]
    fn reports_identical_pixels_as_consistent() {
        let expected = expected_2x2();
        let message = describe_readback_mismatch(&expected, &expected, 2);
        assert!(
            message.contains("一致"),
            "必须说明逐像素一致，实际: {message}"
        );
    }
}
