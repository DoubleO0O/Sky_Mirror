//! 独立、有界、无图形的 nested session runner。
//!
//! 本 binary 是 `NestedRuntimeOrchestrator::run_next_batch` 的首个非测试调用方。
//! 它只负责真实 session 启动、调用方驱动的有限批次、预算停止与资源释放，不内嵌
//! 客户端线程，不创建测试窗口，不接入 buffer/render/input/DRM。验收客户端由独立
//! 进程承担，本程序仅向 stdout 输出实际 socket 路径供其连接。

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
mod session_runner {
    use std::{
        fs,
        io::Write,
        os::unix::{ffi::OsStrExt, fs::DirBuilderExt},
        path::{Path, PathBuf},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use crate::{
        core::state::State,
        smithay_backend::{
            nested_runtime_loop::{NestedRuntimeLoopConfig, NestedRuntimeLoopExitReason},
            nested_runtime_orchestrator::{
                NestedRuntimeLifecycleState, NestedRuntimeOrchestrator,
                NestedRuntimeOrchestratorConfig,
            },
        },
    };

    /// 正常批次上限：至少执行两批，至多三批。
    const MAX_NORMAL_BATCHES: usize = 3;
    /// 每批 coordinator pump 上限。
    const BATCH_ITERATIONS: usize = 200;
    /// 每轮 pump 等待 accept source 的上限。
    const PUMP_TIMEOUT: Duration = Duration::from_millis(5);
    /// session 总预算：只在批次之间检查单调时钟，不能中断正在执行的批次，
    /// 也不保证程序必定在此时限内退出。
    const SESSION_BUDGET: Duration = Duration::from_secs(8);
    /// 独占子目录名碰撞时的有限换名重试上限。
    const MAX_DIR_RETRIES: u32 = 16;
    /// Unix socket 路径（含 NUL 的 SUN_LEN）上限；按字节比较。
    const MAX_SOCKET_PATH_BYTES: usize = 108;
    /// 本次运行在独占子目录内的 socket 文件名。
    const SOCKET_FILE_NAME: &str = "session.sock";
    /// bind 会在 socket 旁创建的 lock 文件名（wayland-server 约定）。
    const LOCK_FILE_NAME: &str = "session.lock";

    /// runner 失败文本：调用方只据此决定非零退出，不做 panic 收尾。
    type RunnerResult<T> = Result<T, String>;

    /// 在 XDG 目录下原子创建本次独占的短名称 0700 子目录。
    ///
    /// 使用带 mode 的单次创建，不存在先建后改权限的窗口；名碰撞只做有限换名重试。
    fn create_exclusive_subdir(runtime_dir: &Path) -> RunnerResult<PathBuf> {
        for _ in 0..MAX_DIR_RETRIES {
            let entropy = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| format!("生成子目录名时间熵失败: {error}"))?
                .as_nanos();
            let subdir = runtime_dir.join(format!(
                "sky-mirror-session-{}-{entropy}",
                std::process::id()
            ));
            match fs::DirBuilder::new().mode(0o700).create(&subdir) {
                Ok(()) => return Ok(subdir),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("创建独占子目录失败: {error}"));
                }
            }
        }
        Err(format!(
            "独占子目录名在 {MAX_DIR_RETRIES} 次重试内持续碰撞，停止"
        ))
    }

    /// 只清理本次独占子目录内的已知 socket/lock 文件；未知内容一律保留。
    ///
    /// 返回具体清理错误列表；调用方必须把它们并入最终失败结果，不能静默吞掉。
    fn cleanup_known_socket_files(subdir: &Path) -> Vec<String> {
        let mut failures = Vec::new();
        for file_name in [SOCKET_FILE_NAME, LOCK_FILE_NAME] {
            let path = subdir.join(file_name);
            if path.exists() {
                if let Err(error) = fs::remove_file(&path) {
                    failures.push(format!("已知文件 {file_name} 清理失败: {error}"));
                }
            }
        }
        failures
    }

    /// 仅当子目录为空时删除；非空则保留并报告，不递归删除。
    fn remove_subdir_if_empty(subdir: &Path, notes: &mut Vec<String>) -> RunnerResult<()> {
        let mut entries =
            fs::read_dir(subdir).map_err(|error| format!("读取自建子目录失败: {error}"))?;
        if entries.next().is_some() {
            notes.push("自建子目录含未知内容，保留不删除".to_owned());
            return Err("自建子目录非空，已保留并报告".to_owned());
        }
        fs::remove_dir(subdir).map_err(|error| format!("删除空自建子目录失败: {error}"))
    }

    /// 用不 panic 的写入方式把 socket 路径行送到 stdout 并立即 flush。
    ///
    /// 资源仍存活，输出失败必须作为失败交给共同收尾路径，不能提前 return 或 panic。
    fn write_socket_line(socket_path: &Path) -> std::io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "session socket: {}", socket_path.display())?;
        handle.flush()
    }

    /// 用不 panic 的写入方式输出最终摘要并立即 flush。
    fn write_summary_line(summary: &str) -> std::io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{summary}")?;
        handle.flush()
    }

    pub(super) fn run() -> RunnerResult<()> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| "需要 XDG_RUNTIME_DIR".to_owned())?;
        if !runtime_dir.is_absolute() || !runtime_dir.is_dir() {
            return Err("XDG_RUNTIME_DIR 必须为已存在的绝对目录".to_owned());
        }

        let subdir = create_exclusive_subdir(&runtime_dir)?;
        // 子目录创建之后的所有退出路径都必须经过本函数末尾的共同清理。
        let mut notes: Vec<String> = Vec::new();
        let mut cleanup_failures: Vec<String> = Vec::new();
        let mut failure: Option<String> = None;

        let subdir_name = match subdir.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_owned(),
            None => {
                failure = Some("自建子目录名必须是 UTF-8".to_owned());
                String::new()
            }
        };
        // 相对 socket 名（含子目录）：wayland bind 会 join 到 XDG 下，无需改接口或环境。
        let socket_name = format!("{subdir_name}/{SOCKET_FILE_NAME}");
        let socket_path = runtime_dir.join(&socket_name);
        let lock_path = subdir.join(LOCK_FILE_NAME);
        if failure.is_none() && socket_path.as_os_str().as_bytes().len() >= MAX_SOCKET_PATH_BYTES {
            failure = Some("完整 socket 路径达到 Unix SUN_LEN 上限，停止".to_owned());
        }

        let mut orchestrator = None;
        let mut state: Option<State> = None;
        let mut stop_consumed = false;
        let mut batches = 0usize;
        let mut pumps = 0usize;

        if failure.is_none() {
            let mut resource_orchestrator =
                NestedRuntimeOrchestrator::new(NestedRuntimeOrchestratorConfig {
                    socket_name,
                    loop_config: NestedRuntimeLoopConfig {
                        max_iterations: BATCH_ITERATIONS,
                        pump_timeout: PUMP_TIMEOUT,
                        stop_when_idle: false,
                        continue_after_error: false,
                    },
                });
            let resource_state = State::new();
            match resource_orchestrator.start() {
                Err(error) => {
                    // 先释放 owner，再清理由本次可能已绑定的已知文件与自建目录。
                    drop(resource_orchestrator);
                    drop(resource_state);
                    cleanup_failures.extend(cleanup_known_socket_files(&subdir));
                    let subdir_outcome = remove_subdir_if_empty(&subdir, &mut notes);
                    if let Err(error) = subdir_outcome {
                        cleanup_failures.push(error);
                    }
                    let message = if cleanup_failures.is_empty() {
                        format!("orchestrator start 失败: {error:?}")
                    } else {
                        format!(
                            "orchestrator start 失败: {error:?}; cleanup_failures={cleanup_failures:?}"
                        )
                    };
                    return Err(message);
                }
                Ok(_start_report) => {}
            }
            // start 成功后才输出 socket 路径；输出失败只记录，继续进入共同收尾。
            if let Err(error) = write_socket_line(&socket_path) {
                failure = Some(format!("输出 socket 路径失败: {error}"));
            }
            orchestrator = Some(resource_orchestrator);
            state = Some(resource_state);
        }

        let started_at = Instant::now();
        if let (Some(orchestrator), Some(state)) = (orchestrator.as_mut(), state.as_mut()) {
            while failure.is_none()
                && batches < MAX_NORMAL_BATCHES
                && started_at.elapsed() < SESSION_BUDGET
            {
                match orchestrator.run_next_batch(state) {
                    Err(error) => {
                        failure = Some(format!("batch 方法返回 Err: {error:?}"));
                        break;
                    }
                    Ok(report) => {
                        pumps += report.iterations_run;
                        let exit_reason = report.exit_reason;
                        let report_errors = !report.errors.is_empty();
                        let report_clean = report.validation_is_clean;
                        drop(report);
                        if exit_reason == NestedRuntimeLoopExitReason::Error
                            || report_errors
                            || !report_clean
                        {
                            failure = Some(format!(
                                "batch 报告失败: exit={exit_reason:?} errors={report_errors} clean={report_clean}"
                            ));
                            break;
                        }
                        match orchestrator.state() {
                            NestedRuntimeLifecycleState::Started => {
                                batches += 1;
                            }
                            NestedRuntimeLifecycleState::Stopped => {
                                // 普通批次已消费停止或中断并自行终结：直接收尾，不再调用 batch。
                                stop_consumed = matches!(
                                    exit_reason,
                                    NestedRuntimeLoopExitReason::StopRequested
                                        | NestedRuntimeLoopExitReason::Interrupted
                                );
                                break;
                            }
                            other => {
                                failure = Some(format!(
                                    "batch 正常退出但 session 状态非 Started/Stopped: {other:?}"
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 预算耗尽且仍 Started：请求停止，再用一批消费停止；其他终态直接释放。
        if failure.is_none()
            && orchestrator.as_ref().is_some_and(|orchestrator| {
                orchestrator.state() == NestedRuntimeLifecycleState::Started
            })
        {
            match orchestrator.as_mut() {
                None => {
                    failure = Some("session owner 在停止阶段缺失".to_owned());
                }
                Some(orchestrator) => match orchestrator.stop_handle() {
                    Err(error) => {
                        failure = Some(format!("预算耗尽后获取 stop handle 失败: {error:?}"));
                    }
                    Ok(handle) => {
                        handle.request_stop_and_wakeup();
                        match state.as_mut() {
                            None => {
                                failure = Some("State owner 在停止消费阶段缺失".to_owned());
                            }
                            Some(state) => match orchestrator.run_next_batch(state) {
                                Err(error) => {
                                    failure = Some(format!("停止消费批次返回 Err: {error:?}"));
                                }
                                Ok(report) => {
                                    let exit_reason = report.exit_reason;
                                    let report_errors = !report.errors.is_empty();
                                    let report_clean = report.validation_is_clean;
                                    let iterations = report.iterations_run;
                                    drop(report);
                                    pumps += iterations;
                                    let stopped_at_end = orchestrator.state()
                                        == NestedRuntimeLifecycleState::Stopped;
                                    let stop_reason_ok = matches!(
                                        exit_reason,
                                        NestedRuntimeLoopExitReason::StopRequested
                                            | NestedRuntimeLoopExitReason::Interrupted
                                    );
                                    if stop_reason_ok
                                        && iterations == 0
                                        && !report_errors
                                        && report_clean
                                        && stopped_at_end
                                    {
                                        stop_consumed = true;
                                    } else {
                                        failure = Some(format!(
                                            "停止消费批次未满足 StopRequested/Interrupted+0 iterations+无错误+validation 干净+Stopped: exit={exit_reason:?} iterations={iterations} errors={report_errors} clean={report_clean} state={:?}",
                                            orchestrator.state()
                                        ));
                                    }
                                }
                            },
                        }
                    }
                },
            }
        }

        let final_state = orchestrator
            .as_ref()
            .map_or(NestedRuntimeLifecycleState::Created, |orchestrator| {
                orchestrator.state()
            });
        drop(orchestrator);
        drop(state);
        finish(
            &subdir,
            &socket_path,
            &lock_path,
            failure,
            final_state,
            batches,
            pumps,
            stop_consumed,
            notes,
            cleanup_failures,
        )
    }

    /// 共同收尾：释放 owner、核验并清理由本次创建的资源、输出摘要并汇总失败。
    ///
    /// 原失败与清理失败必须同时上报；清理成功不能把原失败改写为成功。
    #[allow(clippy::too_many_arguments)]
    fn finish(
        subdir: &Path,
        socket_path: &Path,
        lock_path: &Path,
        mut failure: Option<String>,
        final_state: NestedRuntimeLifecycleState,
        batches: usize,
        pumps: usize,
        stop_consumed: bool,
        mut notes: Vec<String>,
        mut cleanup_failures: Vec<String>,
    ) -> RunnerResult<()> {
        // 先释放 owner，再核验 socket/lock 是否已由 owner 释放而消失。
        // orchestrator/state 在前述作用域已 drop；这里只做文件事实核验。
        let socket_gone = !socket_path.exists();
        let lock_gone = !lock_path.exists();
        if !socket_gone || !lock_gone {
            let residual = "owner 释放后 socket 或 lock 仍残留".to_owned();
            notes.push(residual.clone());
            if failure.is_none() {
                failure = Some(residual);
            }
            // 继续清理本次自建目录内的已知残留，但绝不清除原失败语义。
            cleanup_failures.extend(cleanup_known_socket_files(subdir));
        }
        match remove_subdir_if_empty(subdir, &mut notes) {
            Ok(()) => {}
            Err(error) => {
                cleanup_failures.push(error);
            }
        }

        let summary = format!(
            "session runner: batches={batches} pumps={pumps} stop_consumed={stop_consumed} final_state={final_state:?} socket_gone={socket_gone} lock_gone={lock_gone} failure={failure:?} cleanup_failures={cleanup_failures:?} notes={notes:?}"
        );
        if let Err(error) = write_summary_line(&summary) {
            cleanup_failures.push(format!("输出最终摘要失败: {error}"));
        }

        if failure.is_none() {
            failure = if final_state == NestedRuntimeLifecycleState::Stopped {
                None
            } else {
                Some(format!("最终状态非 Stopped: {final_state:?}"))
            };
        }

        match (failure, cleanup_failures.is_empty()) {
            (None, true) => Ok(()),
            (Some(message), true) => Err(message),
            (None, false) => Err(format!("cleanup_failures={cleanup_failures:?}")),
            (Some(message), false) => {
                Err(format!("{message}; cleanup_failures={cleanup_failures:?}"))
            }
        }
    }
}

#[cfg(all(feature = "smithay-linux", target_os = "linux", not(test)))]
fn main() {
    use std::io::Write as _;

    if let Err(error) = session_runner::run() {
        // 最终 stderr 输出使用不 panic 的写入方式。
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(&mut handle, "sky_mirror_bounded_session_runner: {error}");
        let _ = handle.flush();
        std::process::exit(1);
    }
}

/// 非 Linux、未启用 smithay-linux 或测试构建不创建 session 资源。
#[cfg(any(not(all(feature = "smithay-linux", target_os = "linux")), test))]
fn main() {
    use std::io::Write as _;

    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(
        &mut handle,
        "sky_mirror_bounded_session_runner 需要 Linux smithay-linux feature 并直接运行"
    );
    let _ = handle.flush();
    std::process::exit(2);
}
