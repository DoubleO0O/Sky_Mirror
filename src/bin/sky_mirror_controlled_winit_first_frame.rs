//! R2 受控 production socket→external SHM client→Winit/EGL/GLES 首帧 runner。
//!
//! 此 binary 只用于有界验证，不是 Sky Mirror 日常入口。Winit target 与 coordinator 在
//! 进程主线程创建；外部 Wayland client 在独立线程提交 2×2 XRGB8888 buffer、damage 与
//! frame request。成功只代表该窄路径的 controlled proof，不外推 DRM/input/多输出能力。

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
#[path = "../backend/mod.rs"]
mod backend;
#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
#[path = "../core/mod.rs"]
mod core;
#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
#[path = "../smithay_backend/mod.rs"]
mod smithay_backend;

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
mod controlled_runner {
    use std::{
        fs,
        io::{ErrorKind, Write},
        os::unix::net::UnixStream,
        os::{fd::AsFd, unix::fs::OpenOptionsExt},
        path::{Path, PathBuf},
        sync::mpsc,
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
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .map_err(|error| format!("创建 SHM backing file 失败: {error}"))?;
            // 2×2 XRGB8888：红、绿、蓝、白。该已知图样用于证明 client buffer 不是固定背景。
            file.write_all(&[
                0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF,
                0xFF, 0x00,
            ])
            .map_err(|error| format!("写入 SHM 图样失败: {error}"))?;
            file.flush()
                .map_err(|error| format!("flush SHM 图样失败: {error}"))?;
            Ok(Self { file, path })
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
                    surface.damage_buffer(0, 0, 2, 2);
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
    fn wait_for_client_writable(
        write_readiness_loop: &mut EventLoop<ClientReadiness>,
        readiness: &mut ClientReadiness,
        deadline: Instant,
        stage: &str,
    ) -> Result<(), String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "外部 client {stage} writable wait 超过固定 deadline"
            ));
        }
        readiness.writable_or_error = false;
        write_readiness_loop
            .dispatch(Some(remaining), readiness)
            .map_err(|error| format!("外部 client {stage} writable readiness 失败: {error}"))?;
        if !readiness.writable_or_error {
            return Err(format!(
                "外部 client {stage} deadline 内无 writable fd 事件"
            ));
        }
        Ok(())
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
                    wait_for_client_writable(write_readiness_loop, readiness, deadline, stage)?;
                }
                Err(error) => return Err(format!("外部 client {stage} flush 失败: {error}")),
            }
        }
        Err(format!(
            "外部 client {stage} 超过固定 writable poll 上限 {MAX_CLIENT_READINESS_POLLS}"
        ))
    }

    /// 只通过绝对 deadline、固定 poll 上限和 fd readiness 驱动 Wayland client。
    ///
    /// `prepare_read` 严格先于 readiness poll，唯一 reader 是其 guard；不会使用
    /// `blocking_dispatch`、无界 roundtrip 或脱离 owner 的后台线程。
    fn drive_client_until(
        event_queue: &mut EventQueue<ExternalClientState>,
        state: &mut ExternalClientState,
        readiness_loop: &mut EventLoop<ClientReadiness>,
        write_readiness_loop: &mut EventLoop<ClientReadiness>,
        readiness: &mut ClientReadiness,
        deadline: Instant,
        stage: &str,
        completed: impl Fn(&ExternalClientState) -> bool,
    ) -> Result<(), String> {
        let mut polls = 0usize;
        while !completed(state) {
            if polls >= MAX_CLIENT_READINESS_POLLS {
                return Err(format!(
                    "外部 client {stage} 超过固定 readiness poll 上限 {MAX_CLIENT_READINESS_POLLS}"
                ));
            }
            if Instant::now() >= deadline {
                return Err(format!("外部 client {stage} 超过固定 deadline"));
            }

            match event_queue.flush() {
                Ok(()) => {}
                Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
                    // 发送方向也必须由同一 fd readiness/deadline 驱动；不能以 sleep
                    // 轮询冒充有界 I/O。read 的 guard 尚未创建，故不会破坏单 reader。
                    wait_for_client_writable(write_readiness_loop, readiness, deadline, stage)?;
                    polls = polls.saturating_add(1);
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
                return Err(format!("外部 client {stage} 超过固定 deadline"));
            }
            readiness.readable_or_error = false;
            readiness_loop
                .dispatch(Some(remaining), readiness)
                .map_err(|error| format!("外部 client {stage} readiness poll 失败: {error}"))?;
            polls = polls.saturating_add(1);
            if !readiness.readable_or_error {
                return Err(format!("外部 client {stage} deadline 内无 fd 事件"));
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
        Ok(())
    }

    fn run_external_client(
        socket_path: &Path,
        runtime_dir: &Path,
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
        drive_client_until(
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
        let pool = shm.create_pool(backing.file.as_fd(), 16, &queue_handle, ());
        let buffer = pool.create_buffer(
            0,
            2,
            2,
            8,
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
        drive_client_until(
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

    pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
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
        let client_socket_path = socket_path.clone();
        let client_runtime_dir = runtime_dir.clone();
        let client_thread = thread::spawn(move || {
            let result = run_external_client(&client_socket_path, &client_runtime_dir);
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
        if socket_path.exists() {
            return Err("coordinator drop 后 production socket 仍残留".into());
        }
        if client_thread_panicked {
            return Err("外部 client thread panic".into());
        }
        if let Some(error) = loop_error {
            return Err(error.into());
        }
        let presented = presented.ok_or("deadline 内未完成真实 client frame present")?;
        let client = client_result
            .ok_or("外部 client join 后仍没有结果")?
            .map_err(|error| format!("外部 client 失败（cleanup 后返回）: {error}"))?;
        let expected_rgb = vec![
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0xFF, 0xFF],
        ];
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
            return Err(format!(
                "R2 controlled proof 不满足闭环不变量: render={presented:?}, client={client:?}, core_clean={}",
                state.validate().is_clean()
            )
            .into());
        }

        println!(
            "R2 controlled SHM frame: pumps={pumps} import={} pixels=verified texture_drawn={} submit={} damage={} frame_done={} socket_clean=true",
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
