//! 独立 sync 探测客户端：同一连接完成 registry＋sync#1，经 stdin 门控再做 sync#2。
//!
//! 只做 registry 发现和两次独立 `wl_display.sync` 往返，不创建 surface/toplevel，
//! 不复制测试 harness。两次 callback 用不同标识区分，回包证据以收到对应 callback
//! 为准，不用 poll 数替代。

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
mod probe_client {
    use calloop::{EventLoop, Interest, Mode, PostAction, generic::Generic};
    use std::{
        io::Write,
        os::unix::net::UnixStream,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };
    use wayland_client::{
        Connection, Dispatch, EventQueue, Proxy, QueueHandle,
        backend::WaylandError,
        protocol::{wl_callback::WlCallback, wl_registry::WlRegistry},
    };

    /// sync 回包阶段标识：两次 sync 使用不同标识，互不复用完成标志。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SyncStage {
        First,
        Second,
    }

    /// 客户端协议状态：只记录两次独立回包是否到达。
    #[derive(Default)]
    struct ClientState {
        sync_first_done: bool,
        sync_second_done: bool,
    }

    impl Dispatch<WlRegistry, ()> for ClientState {
        fn event(
            _state: &mut Self,
            _proxy: &WlRegistry,
            _event: wayland_client::protocol::wl_registry::Event,
            _data: &(),
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlCallback, SyncStage> for ClientState {
        fn event(
            state: &mut Self,
            _proxy: &WlCallback,
            event: wayland_client::protocol::wl_callback::Event,
            stage: &SyncStage,
            _connection: &Connection,
            _queue_handle: &QueueHandle<Self>,
        ) {
            if let wayland_client::protocol::wl_callback::Event::Done { .. } = event {
                match stage {
                    SyncStage::First => state.sync_first_done = true,
                    SyncStage::Second => state.sync_second_done = true,
                }
            }
        }
    }

    /// readiness 状态：沿用有界事件驱动，不使用无界 blocking roundtrip。
    #[derive(Default)]
    struct Readiness {
        readable_or_error: bool,
    }

    /// 每个 sync 等待回包的 deadline。
    const SYNC_DEADLINE: Duration = Duration::from_secs(5);
    /// 单次 pump 等待的最大时长。
    const PUMP_TIMEOUT: Duration = Duration::from_millis(5);
    /// readiness poll 固定上限。
    const MAX_POLLS: usize = 1_000;
    /// stdin `continue` 门控的最大等待时长。
    const GATE_DEADLINE: Duration = Duration::from_secs(10);

    type ClientResult<T> = Result<T, String>;

    /// 用不 panic 的写入方式输出协议行并立即 flush。
    fn write_line(line: &str) -> std::io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{line}")?;
        handle.flush()
    }

    /// 有界驱动事件队列直到 `done` 成立；返回实际 poll 数。
    #[allow(clippy::too_many_arguments)]
    fn pump_until(
        event_queue: &mut EventQueue<ClientState>,
        client_state: &mut ClientState,
        readiness_loop: &mut EventLoop<Readiness>,
        readiness: &mut Readiness,
        deadline: Instant,
        stage: &str,
        done: impl Fn(&ClientState) -> bool,
    ) -> ClientResult<usize> {
        let mut polls = 0usize;
        while !done(client_state) {
            if polls >= MAX_POLLS {
                return Err(format!("{stage} 超过 readiness poll 上限"));
            }
            if Instant::now() >= deadline {
                return Err(format!("{stage} 超过 deadline"));
            }
            let flushed = match event_queue.flush() {
                Ok(()) => true,
                Err(WaylandError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    false
                }
                Err(error) => return Err(format!("{stage} flush 失败: {error}")),
            };
            event_queue
                .dispatch_pending(client_state)
                .map_err(|error| format!("{stage} dispatch_pending 失败: {error}"))?;
            if done(client_state) {
                break;
            }
            if !flushed {
                thread::sleep(PUMP_TIMEOUT);
                polls += 1;
                continue;
            }
            let Some(read_guard) = event_queue.prepare_read() else {
                event_queue
                    .dispatch_pending(client_state)
                    .map_err(|error| format!("{stage} 二次 dispatch 失败: {error}"))?;
                polls += 1;
                continue;
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!("{stage} 超过 deadline"));
            }
            readiness_loop
                .dispatch(remaining.min(PUMP_TIMEOUT), readiness)
                .map_err(|error| format!("{stage} readiness 等待失败: {error}"))?;
            polls += 1;
            if readiness.readable_or_error {
                readiness.readable_or_error = false;
                if let Err(error) = read_guard.read() {
                    return Err(format!("{stage} socket 读取失败: {error}"));
                }
            }
        }
        Ok(polls)
    }

    /// 控制行长度上限（字节，含行终止符）；`continue` 远小于上限。
    const MAX_LINE_BYTES: usize = 128;

    /// 按字节有界读取一行控制输入，最多读取上限字节。
    ///
    /// 先 push 再判长：累计字节（含终止符）超过上限立即报错，因此内容至多
    /// 127 字节加换行；正常 `continue` 命令不受影响。
    fn read_bounded_line() -> ClientResult<String> {
        use std::io::Read as _;

        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        let mut buf = Vec::with_capacity(MAX_LINE_BYTES + 1);
        let mut byte = [0u8; 1];
        loop {
            match handle.read(&mut byte) {
                Ok(0) => {
                    if buf.is_empty() {
                        return Err("stdin 遇到 EOF".to_owned());
                    }
                    break;
                }
                Ok(_) => {
                    buf.push(byte[0]);
                    if buf.len() > MAX_LINE_BYTES {
                        return Err("控制行超过长度上限".to_owned());
                    }
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(format!("stdin 读取失败: {error}")),
            }
        }
        String::from_utf8(buf)
            .map(|text| text.trim_end_matches(['\r', '\n']).to_owned())
            .map_err(|_| "控制行非 UTF-8".to_owned())
    }

    /// 有界等待 stdin 精确命令 `continue`；EOF、超时、错文、超长均为失败。
    ///
    /// 只传一条结果，同样使用固定容量同步通道；读取线程不持有连接或队列，
    /// 超时后由进程退出回收，不做无条件 join。
    fn await_continue() -> ClientResult<()> {
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let _ = sender.send(read_bounded_line());
        });
        match receiver.recv_timeout(GATE_DEADLINE) {
            Err(mpsc::RecvTimeoutError::Timeout) => Err("等待 continue 超时".to_owned()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("stdin 读取线程已结束".to_owned()),
            Ok(Err(error)) => Err(error),
            Ok(Ok(line)) => {
                if line == "continue" {
                    Ok(())
                } else {
                    Err(format!("门控命令不匹配: {line:?}"))
                }
            }
        }
    }

    pub(super) fn run(socket_path: &str) -> ClientResult<()> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|error| format!("连接 socket 失败: {error}"))?;
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("设置 nonblocking 失败: {error}"))?;
        let readiness_fd = stream
            .try_clone()
            .map_err(|error| format!("克隆 readiness fd 失败: {error}"))?;
        let connection = Connection::from_socket(stream)
            .map_err(|error| format!("创建 Wayland connection 失败: {error}"))?;
        let mut event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();
        let display = connection.display();
        let mut client_state = ClientState::default();
        let mut readiness_loop = EventLoop::<Readiness>::try_new()
            .map_err(|error| format!("创建 readiness loop 失败: {error}"))?;
        let mut readiness = Readiness::default();
        readiness_loop
            .handle()
            .insert_source(
                Generic::new(readiness_fd, Interest::READ, Mode::Level),
                |event, _, readiness| {
                    readiness.readable_or_error |= event.readable || event.error;
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|error| format!("注册 readiness source 失败: {error}"))?;

        let registry = display.get_registry(&queue_handle, ());
        if !registry.is_alive() {
            return Err("registry proxy 必须存活".to_owned());
        }
        let sync_first = display.sync(&queue_handle, SyncStage::First);
        if !sync_first.is_alive() {
            return Err("sync#1 proxy 必须存活".to_owned());
        }
        event_queue
            .flush()
            .map_err(|error| format!("sync#1 flush 失败: {error}"))?;
        write_line("sync1 armed").map_err(|error| format!("输出 sync1 armed 失败: {error}"))?;
        let first_polls = pump_until(
            &mut event_queue,
            &mut client_state,
            &mut readiness_loop,
            &mut readiness,
            Instant::now() + SYNC_DEADLINE,
            "sync#1",
            |state| state.sync_first_done,
        )?;
        if !client_state.sync_first_done {
            return Err("必须收到 sync#1 的独立回包".to_owned());
        }
        write_line(&format!("sync1 done polls={first_polls}"))
            .map_err(|error| format!("输出 sync1 done 失败: {error}"))?;

        await_continue()?;

        let sync_second = display.sync(&queue_handle, SyncStage::Second);
        if !sync_second.is_alive() {
            return Err("sync#2 proxy 必须存活".to_owned());
        }
        event_queue
            .flush()
            .map_err(|error| format!("sync#2 flush 失败: {error}"))?;
        write_line("sync2 armed").map_err(|error| format!("输出 sync2 armed 失败: {error}"))?;
        let second_polls = pump_until(
            &mut event_queue,
            &mut client_state,
            &mut readiness_loop,
            &mut readiness,
            Instant::now() + SYNC_DEADLINE,
            "sync#2",
            |state| state.sync_second_done,
        )?;
        if !client_state.sync_second_done {
            return Err("必须收到 sync#2 的独立回包".to_owned());
        }
        write_line(&format!("sync2 done polls={second_polls}"))
            .map_err(|error| format!("输出 sync2 done 失败: {error}"))?;
        Ok(())
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
fn main() {
    use std::io::Write as _;

    let socket_path = std::env::args().nth(1);
    let result = match socket_path {
        Some(path) => probe_client::run(&path),
        None => Err("用法: sky_mirror_session_probe_client <socket-path>".to_owned()),
    };
    if let Err(error) = result {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(&mut handle, "sky_mirror_session_probe_client: {error}");
        let _ = handle.flush();
        std::process::exit(1);
    }
}

/// 非 Linux、未启用 smithay-linux 或测试构建不创建客户端资源。
#[cfg(any(not(all(feature = "smithay-linux", target_os = "linux")), test))]
fn main() {
    use std::io::Write as _;

    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(
        &mut handle,
        "sky_mirror_session_probe_client 需要 Linux smithay-linux feature 并直接运行"
    );
    let _ = handle.flush();
    std::process::exit(2);
}
