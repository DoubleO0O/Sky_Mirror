//! Linux-only nested Winit/EGL/GLES 首帧输出 owner。
//!
//! 此模块只为 R2 建立真实 target 的唯一 owner，并导入已由 coordinator 原子授权的
//! SHM buffer、回读 texture、绘制与提交。它不决定 ledger/Core identity，不发送 frame
//! done、不处理 input，也不修改 Core；所有结果均仅为 controlled proof，不是 production
//! compositor。

use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};

use smithay::{
    backend::{
        allocator::Fourcc,
        egl::ffi::egl as egl_ffi,
        renderer::{
            Color32F, ExportMem, Frame, ImportMemWl, Renderer, Texture, TextureMapping,
            element::{Id, Kind, texture::TextureRenderElement},
            gles::{GlesFrame, GlesRenderer, ffi as gles_ffi},
            utils::draw_render_elements,
        },
        winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    reexports::winit::platform::pump_events::PumpStatus,
    utils::{Buffer as BufferCoord, Physical, Rectangle, Transform},
};

use super::linux_xdg_shell::PendingShmBufferResource;

/// present 前泵送宿主 Winit 事件的有界预算：等待至少一次窗口配置（`Resized`）以推进
/// 宿主窗口系统回调；超时后仍继续呈现，最终由目标 framebuffer 回读门禁兜底判定。
const WINIT_PRESENT_PUMP_BUDGET: Duration = Duration::from_millis(150);
/// 泵送轮次之间的短暂休眠，避免在预算内忙等烧满 CPU。
const WINIT_PRESENT_PUMP_INTERVAL: Duration = Duration::from_millis(2);

/// 一次受控首帧的可审计结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NestedWinitFirstFrameReport {
    /// Winit window / EGL surface / GLES renderer owner 已创建。
    pub target_created: bool,
    /// Winit window 的物理尺寸。
    pub target_size: (i32, i32),
    /// renderer 已成功 bind 到 target。
    pub renderer_bound: bool,
    /// 首帧清屏已由真实 GLES renderer 完成。
    pub first_frame_cleared: bool,
    /// backbuffer 已经提交给 Winit/EGL target。
    pub backbuffer_submitted: bool,
    /// 本切片显式不处理 client buffer。
    pub client_buffer_imported: bool,
    /// 本切片显式不发送 client frame done。
    pub client_frame_done_sent: bool,
}

/// 一次真实 SHM import 与 Winit target 呈现的最小受控证据。
///
/// 该报告只证明已验证 resource 被 GLES import、回读、绘制并提交；并不声明完整 damage
/// tracking、surface-tree 合成、buffer release 或长期 frame callback 调度已实现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NestedWinitShmPresentReport {
    /// 已由当前 GLES renderer 导入 SHM buffer。
    pub client_buffer_imported: bool,
    /// 已以真实 texture render element 绘制到 Winit/EGL framebuffer。
    pub client_texture_drawn: bool,
    /// 已从刚导入的 GLES texture 做真实内存回读。
    pub client_texture_read_back: bool,
    /// 按 client top-to-bottom 顺序归一化后的逐像素 RGB；不包含未定义的 X 通道。
    ///
    /// 该值来自**绘制之后、submit 之前**对目标 framebuffer 元素区域的读回，present 内
    /// 已强制它与 source texture 回读逐像素相等，因此它代表最终 target 的输出像素，
    /// 而不是仅能证明导入内容的 source texture 回读。
    pub client_texture_readback_rgb: Vec<[u8; 3]>,
    /// 已提交 Winit/EGL backbuffer。
    pub backbuffer_submitted: bool,
    /// 仅使用本次完整 output damage，不假称 client damage tracking 已实现。
    pub output_damage_submitted: bool,
    /// 资源仍由 output owner 保持，满足 importer 对 buffer lifetime 的要求。
    pub source_buffer_retained: bool,
    /// 此步骤不发送 frame done，必须由之后的成功完成门单独授权。
    pub client_frame_done_sent: bool,
}

/// Winit output 的唯一资源 owner。
///
/// `backend` 持有 Winit window、EGL display/context/surface 与 `GlesRenderer`；
/// `event_loop` 与 window 必须共同存活。当前尚未连接 production runtime loop，故 drop
/// 是唯一 cleanup owner：owner drop 时释放这些资源，且不触碰 client/Core 状态。
pub(crate) struct NestedWinitOutputOwner {
    backend: WinitGraphicsBackend<GlesRenderer>,
    event_loop: WinitEventLoop,
    /// 已成功 import 的 source buffer 必须和 texture 至少同寿命。此 MVP 只保留到 output
    /// owner drop；替换/显式 release 策略不在本切片范围内。
    retained_source_buffers: Vec<smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer>,
}

/// 将 GLES `Xrgb8888` mapping 归一化为 client top-to-bottom RGB 像素。
///
/// FourCC 的 little-endian 内存顺序是 B/G/R/X；X 不承载 alpha 语义，不能纳入相等性。
/// mapping 长度必须精确匹配，避免截断或尾随数据仍被错误记为已验证。
fn normalize_xrgb8888_readback_rgb(
    mapped: &[u8],
    width: i32,
    height: i32,
    flipped: bool,
) -> Option<Vec<[u8; 3]>> {
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;
    let expected_len = width.checked_mul(height)?.checked_mul(4)?;
    if width == 0 || height == 0 || mapped.len() != expected_len {
        return None;
    }

    let mut rgb = Vec::with_capacity(width * height);
    for output_row in 0..height {
        // `flipped=true` 表示 mapping 相对 GL 的 lower-left 坐标已经翻转，即内存第一行
        // 是 client top row；未翻转 mapping 才需要在这里逆转行序。
        let mapped_row = if flipped {
            output_row
        } else {
            height - 1 - output_row
        };
        for column in 0..width {
            let offset = (mapped_row * width + column) * 4;
            rgb.push([mapped[offset + 2], mapped[offset + 1], mapped[offset]]);
        }
    }
    Some(rgb)
}

/// 目标 framebuffer 中“logical 左上角元素”对应的 `glReadPixels` 区域。
///
/// 投影把 logical `(0,0)` 映射到 NDC 顶边，即 GL 坐标 `y = output_h`（GL 以左下为原点）；
/// 因此位于 logical 左上角、尺寸 `element` 的绘制内容对应
/// `x ∈ [0, element_w)`、`y ∈ [output_h - element_h, output_h)`。元素大于目标或任一
/// 尺寸为零时返回 `None`，调用方必须据此失败，绝不读回越界或无关区域。
fn target_readback_region(
    output_size: (u32, u32),
    element_size: (u32, u32),
) -> Option<Rectangle<i32, BufferCoord>> {
    let (output_width, output_height) = output_size;
    let (element_width, element_height) = element_size;
    if element_width == 0
        || element_height == 0
        || element_width > output_width
        || element_height > output_height
    {
        return None;
    }
    let gl_y = i32::try_from(output_height - element_height).ok()?;
    let width = i32::try_from(element_width).ok()?;
    let height = i32::try_from(element_height).ok()?;
    Some(Rectangle::from_loc_and_size((0, gl_y), (width, height)))
}

/// 描述两幅回读之间的紧凑关系：长度、首个差异坐标、行/列/180 度翻转与全黑状态。
///
/// 只输出统计结论、不复制像素数据，用于 present 失败时一次性判定“目标是黑、还是只是
/// 方向反了”，替代倾倒整幅像素的诊断。
fn readback_relation(actual: &[[u8; 3]], expected: &[[u8; 3]], width: usize) -> String {
    if actual.len() != expected.len() {
        return format!(
            "回读长度不符: actual={} expected={} width={width}",
            actual.len(),
            expected.len()
        );
    }
    if width == 0 || actual.len() % width != 0 {
        return format!("回读长度 {} 无法按 width={width} 解析", actual.len());
    }
    let height = actual.len() / width;
    let mut parts = Vec::new();
    match actual.iter().zip(expected).position(|(a, b)| a != b) {
        None => parts.push("逐像素完全一致".to_owned()),
        Some(index) => parts.push(format!(
            "首个差异 index={index} (x={}, y={}) actual={:?} expected={:?}",
            index % width,
            index / width,
            actual[index],
            expected[index]
        )),
    }
    if actual.iter().all(|pixel| *pixel == [0, 0, 0]) {
        parts.push("actual 全黑".to_owned());
    }
    let rows_equal = |row: usize, other: usize| {
        (0..width).all(|column| actual[row * width + column] == expected[other * width + column])
    };
    let columns_equal = |left: usize, right: usize| {
        (0..height).all(|row| actual[row * width + left] == expected[row * width + right])
    };
    if (0..height).all(|row| rows_equal(row, height - 1 - row)) {
        parts.push("行序上下翻转".to_owned());
    }
    if (0..width).all(|column| columns_equal(column, width - 1 - column)) {
        parts.push("列序左右翻转".to_owned());
    }
    if (0..height).all(|row| {
        (0..width).all(|column| {
            actual[row * width + column]
                == expected[(height - 1 - row) * width + width - 1 - column]
        })
    }) {
        parts.push("180 度旋转".to_owned());
    }
    parts.join("；")
}

/// present 失败诊断：`actual` 固定是目标 framebuffer 回读，`expected` 固定是 source
/// texture 回读。
///
/// 两侧语义必须与门禁比较一致（`target_rgb != readback_rgb`）：黑 target 必须被报告为
/// “actual 全黑”，否则“目标没有图案”会被误读成“source 内容异常”，把坐标/绘制问题
/// 引向错误方向。
fn target_mismatch_diagnostic(
    target_rgb: &[[u8; 3]],
    readback_rgb: &[[u8; 3]],
    width: usize,
) -> String {
    readback_relation(target_rgb, readback_rgb, width)
}

/// 从已按 client top-to-bottom 归一化的全帧回读中取出一个矩形子区域。
///
/// `rect` 为 `(x, y, width, height)`，行序与全帧一致。区域越界或帧长度不匹配时返回
/// `None`：调用方只能据此放弃比较，绝不裁剪、补齐或越界读取。
fn sub_rect_pixels(
    frame_rgb: &[[u8; 3]],
    frame_width: usize,
    frame_height: usize,
    rect: (usize, usize, usize, usize),
) -> Option<Vec<[u8; 3]>> {
    let (x, y, width, height) = rect;
    let expected_len = frame_width.checked_mul(frame_height)?;
    if frame_width == 0 || frame_height == 0 || frame_rgb.len() != expected_len {
        return None;
    }
    if width == 0
        || height == 0
        || x.checked_add(width)? > frame_width
        || y.checked_add(height)? > frame_height
    {
        return None;
    }
    let mut pixels = Vec::with_capacity(width * height);
    for row in y..y + height {
        let start = row * frame_width + x;
        pixels.extend_from_slice(&frame_rgb[start..start + width]);
    }
    Some(pixels)
}

/// 全目标 framebuffer 的紧凑摘要：非黑像素计数、包围盒、与预期区域的关系，以及
/// 图案在**实际出现位置**与 source 的方向/颜色关系。
///
/// 位置结论分两档：逐像素比较确认包围盒内容来自 source（一致或可达的轴向翻转/旋转）时
/// 才称“图案”位于区域内/外；尚未确认内容匹配时只称“非黑像素包围盒”位于区域内/外，
/// 避免把无关杂色误报成预期图案。
///
/// 区域回读为黑时，仅凭该区域无法区分“图案画到别处”和“target 根本没有图案”；本函数
/// 扫描整帧非黑像素给出位置证据。坐标约定：
/// - `frame_rgb` 已按 client top-to-bottom 归一化（第 0 行是屏幕最上方）；
/// - `region` 是 `target_readback_region` 给出的 GL 左下原点坐标，比较前先换算成
///   top-down 行区间 `[h - (y + rh), h - y)`。
///
/// 只输出计数与包围盒，不倾倒像素；方向判定复用 `readback_relation`，其覆盖
/// 逐像素一致、行/列翻转与 180 度旋转——GLES 投影只可能产生这些轴向翻转，转置需要
/// 交换轴，不在该渲染路径的可达变换内。
///
/// 帧长度与 `width × height` 不符时返回“摘要不可用”：失败路径保持关闭，绝不基于
/// 不完整数据给出通过结论。
fn target_frame_summary(
    frame_rgb: &[[u8; 3]],
    width: usize,
    height: usize,
    region: Rectangle<i32, BufferCoord>,
    source_rgb: &[[u8; 3]],
    source_width: usize,
) -> String {
    let expected_len = width.checked_mul(height);
    if width == 0 || height == 0 || expected_len != Some(frame_rgb.len()) {
        return format!(
            "全帧摘要不可用: 长度 {} 无法匹配 {width}x{height}",
            frame_rgb.len()
        );
    }

    let region_x = usize::try_from(region.loc.x).unwrap_or(0);
    let region_gl_y = usize::try_from(region.loc.y).unwrap_or(0);
    let region_width = usize::try_from(region.size.w).unwrap_or(0);
    let region_height = usize::try_from(region.size.h).unwrap_or(0);
    // GL 左下原点 → client top-down 行号：GL y ∈ [ry, ry+rh) 对应 top-down
    // 行 [h-(ry+rh), h-ry)；region 越界时区间退化，只会把结论判为“区域之外”。
    let region_top = height.saturating_sub(region_gl_y + region_height);
    let region_bottom = height.saturating_sub(region_gl_y);

    let total = frame_rgb.len();
    let mut non_black = 0usize;
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for (index, pixel) in frame_rgb.iter().enumerate() {
        if *pixel == [0, 0, 0] {
            continue;
        }
        non_black += 1;
        let x = index % width;
        let y = index / width;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let source_non_black = source_rgb
        .iter()
        .filter(|pixel| **pixel != [0, 0, 0])
        .count();

    if non_black == 0 {
        return format!(
            "全帧摘要: 非黑像素 0/{total}（source 非黑 {source_non_black}），图案未出现在目标 \
             framebuffer 的任何位置（target 根本没有图案）"
        );
    }

    let box_width = max_x - min_x + 1;
    let box_height = max_y - min_y + 1;
    let source_height = if source_width > 0 {
        source_rgb.len() / source_width
    } else {
        0
    };
    // 包围盒由本帧像素推导，帧长度与边界已在入口校验，尺寸相符时子区域读取必然成功；
    // 因此 `None` 只可能意味着包围盒尺寸与 source 不符。
    let relation = if source_width > 0 && (box_width, box_height) == (source_width, source_height) {
        sub_rect_pixels(
            frame_rgb,
            width,
            height,
            (min_x, min_y, box_width, box_height),
        )
        .map(|at_box| readback_relation(&at_box, source_rgb, source_width))
    } else {
        None
    };
    // 位置措辞必须先由逐像素比较确认包围盒内容确实来自 source：只有 `readback_relation`
    // 判出“逐像素完全一致”或 GLES 投影可达的轴向翻转/180 度旋转时，才允许称“图案”；
    // 只有首差或全黑说明内容与 source 无关，此时只能说“非黑像素”，避免把任意杂色误报成
    // 四象限图案。标记取自 `readback_relation` 的固定输出，即使其措辞日后变化，退化方向
    // 也只是不再称“图案”，不会放大结论。
    const CONFIRMED_SOURCE_RELATIONS: [&str; 4] = [
        "逐像素完全一致",
        "行序上下翻转",
        "列序左右翻转",
        "180 度旋转",
    ];
    let pattern_confirmed = relation.as_deref().is_some_and(|text| {
        CONFIRMED_SOURCE_RELATIONS
            .iter()
            .any(|marker| text.contains(marker))
    });

    let mut summary = format!(
        "全帧摘要: 非黑像素 {non_black}/{total}（source 非黑 {source_non_black}）包围盒 \
         x=[{min_x},{max_x}] y=[{min_y},{max_y}]（client top-down）; 预期区域 \
         x=[{region_x},{}] y=[{region_top},{region_bottom})",
        region_x.saturating_add(region_width)
    );
    let inside = min_x >= region_x
        && max_x.saturating_add(1) <= region_x.saturating_add(region_width)
        && min_y >= region_top
        && max_y.saturating_add(1) <= region_bottom;
    summary.push_str(if pattern_confirmed {
        if inside {
            "; 图案落在预期区域内"
        } else {
            "; 图案画到预期区域之外"
        }
    } else if inside {
        "; 非黑像素包围盒位于预期区域内"
    } else {
        "; 非黑像素包围盒位于预期区域之外"
    });
    if non_black == total {
        summary.push_str("（整帧非黑：清屏色不是纯黑时包围盒不承载位置信息）");
    }

    if let Some(relation) = relation {
        summary.push_str(&format!("; 实际位置与 source 关系: {relation}"));
    } else {
        summary.push_str(&format!(
            "; 包围盒尺寸 {box_width}x{box_height} 与图案 {source_width}x{source_height} 不符，\
             无法判定方向"
        ));
    }
    summary
}

/// R2 target 黑帧最小运行时诊断开关：只有显式 `SKY_MIRROR_R2_TARGET_DIAG=1` 才开启。
///
/// 未开启时 present 的导入、绘制、逐像素门禁、submit、frame done 授权与清理路径都与
/// 原实现一致；开启后也只追加尺寸、布尔值、GL 错误码与 1 个像素，不倾倒整幅像素。
fn r2_target_diag_enabled() -> bool {
    std::env::var("SKY_MIRROR_R2_TARGET_DIAG")
        .map(|value| value == "1")
        .unwrap_or(false)
}

/// 排空当前上下文的 GL error 队列并返回错误码。
///
/// `glGetError` 每次只取出最早的一条并同时清掉它，因此必须循环读取；上限用于在驱动
/// 持续报错时避免 present 路径死循环。返回空 `Vec` 表示没有 pending error（全为
/// `GL_NO_ERROR`），不会把“无错误”和“读取失败”混为一谈。
fn diag_drain_gl_errors(gl: &gles_ffi::Gles2) -> Vec<u32> {
    let mut codes = Vec::new();
    for _ in 0..8 {
        // SAFETY: 调用点始终位于仍活跃的 `GlesFrame` 内，上下文已由 `render()` 置为
        // current；`glGetError` 只读错误队列，不修改任何 GL 状态。
        let code = unsafe { gl.GetError() };
        if code == gles_ffi::NO_ERROR {
            break;
        }
        codes.push(code);
    }
    codes
}

/// 把 GL 错误码渲染成紧凑文本：`0` 表示无错误，否则用 `0xXXXX` 以 `+` 连接。
fn diag_format_gl_errors(codes: &[u32]) -> String {
    if codes.is_empty() {
        return "0".to_owned();
    }
    codes
        .iter()
        .map(|code| format!("0x{code:04X}"))
        .collect::<Vec<_>>()
        .join("+")
}

/// 在仍活跃的 `GlesFrame` 上排空并返回 GL 错误码。
///
/// 必须走 `GlesFrame::with_context`：它不重新绑定 EGL surface；`GlesRenderer::with_context`
/// 会 `make_current`，在当前时机可能切到没有 surface 的上下文。
fn diag_frame_gl_errors(frame: &mut GlesFrame<'_, '_>) -> Result<Vec<u32>, String> {
    frame
        .with_context(diag_drain_gl_errors)
        .map_err(|error| format!("GlesFrame::with_context 失败: {error}"))
}

/// 在 `frame.finish()` 与 `copy_framebuffer` 重新绑定之前，从预期红色象限内部读取
/// 1 个 `GL_BACK` 像素，返回“原始通道值 + 读取错误码”的紧凑证据。
///
/// 用途是区分“绘制阶段 target 已经是黑”和“绘制完成后被重绑/回读破坏”，因此只能在
/// `GlesFrame` 仍活跃时执行：`finish()` 会关闭 SCISSOR/BLEND，`copy_framebuffer` 会
/// 重新 `make_current` 并改写 `GL_READ_BUFFER` 与 `GL_PIXEL_PACK_BUFFER`。
///
/// 读取前保存、读取后恢复 `GL_READ_BUFFER` 和 `GL_PIXEL_PACK_BUFFER_BINDING`；不改写
/// framebuffer 绑定（只记录 `GL_READ_FRAMEBUFFER_BINDING`），不改 viewport、scissor、
/// blend 或 alignment——`RGBA + UNSIGNED_BYTE` 在默认 4 字节 `PACK_ALIGNMENT` 下天然
/// 对齐。任何一步不可靠都会返回 `跳过(原因)` 而不是写入，且错误码原样保留，绝不把
/// “调用返回”当成读取成功。
fn diag_read_back_pixel(frame: &mut GlesFrame<'_, '_>, x: i32, y: i32) -> String {
    if x < 0 || y < 0 {
        return format!("跳过(坐标 ({x},{y}) 为负)");
    }

    // 第一段只读状态、不写状态：只要查询不干净就整项放弃，避免在无法可靠恢复时改写 GL。
    let (version, read_framebuffer, read_buffer, pack_buffer, state_errors) = match frame
        .with_context(|gl| {
            // SAFETY: `GL_VERSION` 返回实现持有的 NUL 结尾静态字符串。
            let version_ptr = unsafe { gl.GetString(gles_ffi::VERSION) };
            let version = if version_ptr.is_null() {
                None
            } else {
                Some(
                    // SAFETY: 见上，指针非空且由 GL 保证 NUL 结尾。
                    unsafe { std::ffi::CStr::from_ptr(version_ptr.cast()) }
                        .to_string_lossy()
                        .into_owned(),
                )
            };
            let mut read_framebuffer = 0i32;
            let mut read_buffer = 0i32;
            let mut pack_buffer = 0i32;
            // SAFETY: 三个调用各自只写入调用方持有的 4 字节整数。
            unsafe {
                gl.GetIntegerv(gles_ffi::READ_FRAMEBUFFER_BINDING, &mut read_framebuffer);
                gl.GetIntegerv(gles_ffi::READ_BUFFER, &mut read_buffer);
                gl.GetIntegerv(gles_ffi::PIXEL_PACK_BUFFER_BINDING, &mut pack_buffer);
            }
            (
                version,
                read_framebuffer,
                read_buffer,
                pack_buffer,
                diag_drain_gl_errors(gl),
            )
        }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };
    let Some(version) = version else {
        return "跳过(GL_VERSION 读取失败)".to_owned();
    };
    // `glReadBuffer` / `GL_READ_BUFFER` / pixel pack buffer 都是 GLES3 入口；上下文低于
    // 3.0 时这些符号可能根本没有加载，直接调用会落到空函数指针，必须先按版本守卫。
    if !version.contains("OpenGL ES 3") && !version.contains("OpenGL ES 4") {
        return format!("跳过(GL 非 ES3: {version})");
    }
    if !state_errors.is_empty() {
        return format!(
            "跳过(状态查询错误 {})",
            diag_format_gl_errors(&state_errors)
        );
    }

    let (rgba, read_errors, restore_errors) = match frame.with_context(|gl| {
        let mut rgba = [0u8; 4];
        // SAFETY: 只有先解绑 pixel pack buffer，`glReadPixels` 的末参才是客户端指针；
        // 读取源显式设为 `GL_BACK`，与当前 Surface target 的默认读取面一致。
        unsafe {
            if pack_buffer != 0 {
                gl.BindBuffer(gles_ffi::PIXEL_PACK_BUFFER, 0);
            }
            gl.ReadBuffer(gles_ffi::BACK);
            gl.ReadPixels(
                x,
                y,
                1,
                1,
                gles_ffi::RGBA,
                gles_ffi::UNSIGNED_BYTE,
                rgba.as_mut_ptr().cast(),
            );
        }
        let read_errors = diag_drain_gl_errors(gl);
        // SAFETY: 恢复的正是上一段读到的原值，且只动这两个绑定。
        unsafe {
            gl.ReadBuffer(read_buffer as u32);
            if pack_buffer != 0 {
                gl.BindBuffer(gles_ffi::PIXEL_PACK_BUFFER, pack_buffer as u32);
            }
        }
        let restore_errors = diag_drain_gl_errors(gl);
        (rgba, read_errors, restore_errors)
    }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };

    format!(
        "[{}, {}, {}, {}] err(read)={} err(restore)={} rb={} pk={}",
        rgba[0],
        rgba[1],
        rgba[2],
        rgba[3],
        diag_format_gl_errors(&read_errors),
        diag_format_gl_errors(&restore_errors),
        read_framebuffer,
        pack_buffer,
    )
}

/// 诊断采样的坐标换算：算出预期红色象限中心的 GL 坐标。
///
/// 坐标与 `target_readback_region` 同源：元素位于 logical `(0,0)`，GL 区域左下角是
/// `(0, output_height - element_height)`；红色象限是 source 的 client 左上象限，而 GL
/// 区域内 client 第 0 行位于最高 y，所以中心为
/// `x = element_width / 4`、`y = region_y + element_height - 1 - element_height / 4`。
/// 任一换算或越界失败都返回跳过原因，绝不读取未定义区域。
fn diag_red_quadrant_center(
    output_width: i32,
    output_height: i32,
    element_width: u32,
    element_height: u32,
) -> Result<(i32, i32), String> {
    let (Ok(output_width), Ok(output_height)) =
        (u32::try_from(output_width), u32::try_from(output_height))
    else {
        return Err("跳过(output 尺寸无法转换)".to_owned());
    };
    if element_width == 0
        || element_height == 0
        || element_width > output_width
        || element_height > output_height
    {
        return Err("跳过(元素区域无法落在目标内)".to_owned());
    }
    let region_y = output_height - element_height;
    let gl_x = element_width / 4;
    let gl_y = region_y + element_height - 1 - element_height / 4;
    if gl_x >= output_width || gl_y >= output_height {
        return Err("跳过(象限中心坐标越界)".to_owned());
    }
    let (Ok(gl_x), Ok(gl_y)) = (i32::try_from(gl_x), i32::try_from(gl_y)) else {
        return Err("跳过(象限中心坐标无法转换)".to_owned());
    };
    Ok((gl_x, gl_y))
}

/// 诊断采样入口：在仍活跃的 `GlesFrame` 上，从预期红色象限中心读取 1 个 `GL_BACK`
/// 像素，并以 `label` 为前缀记录像素 RGBA、GL 错误码与恢复状态。
///
/// 清屏后与纹理绘制后两次采样复用同一坐标换算与 `diag_read_back_pixel` 的同一套状态
/// 保存/恢复逻辑；`label` 只用于在日志里区分采样时机，不改变读取行为。坐标换算失败
/// 时输出跳过原因，读取函数无法安全保存或恢复状态时也只返回跳过原因。
fn diag_quadrant_pixel_note(
    frame: &mut GlesFrame<'_, '_>,
    label: &str,
    output_width: i32,
    output_height: i32,
    element_width: u32,
    element_height: u32,
) -> String {
    let (gl_x, gl_y) = match diag_red_quadrant_center(
        output_width,
        output_height,
        element_width,
        element_height,
    ) {
        Ok(point) => point,
        Err(reason) => return format!("{label}={reason}"),
    };
    format!("{label}={}", diag_read_back_pixel(frame, gl_x, gl_y))
}

/// 诊断专用：在同一仍活跃的 `GlesFrame` 上执行一次原生 `glClear` 正向控制。
///
/// Smithay 0.7 的 `GlesFrame::clear` 走 `Disable(BLEND)` + `draw_solid` + `Enable(BLEND)`，
/// **不是** 原生 `glClear`，因此仅凭 `clear_px` 为黑无法区分“solid 绘制没落到 target”和
/// “framebuffer／读取路径本身有问题”。本控制只在 `SKY_MIRROR_R2_TARGET_DIAG=1` 时执行，
/// 用原生 `gl.Clear(GL_COLOR_BUFFER_BIT)` 建立一条不经过 Smithay 绘制管线的对照。
///
/// 只改写 `GL_COLOR_CLEAR_VALUE` 这一项状态：先保存，临时设为饱和青色 `(0,1,1,1)`，
/// 记录调用前、调用后、恢复后三段 GL error code，再恢复原值并回读确认。framebuffer、
/// scissor、color mask 与 draw/read buffer 选择值只查询不改写——原生 `glClear` 正是受
/// 这些状态约束，实际值必须附进文本，否则“没清到采样点”会被误读成清屏失败。
///
/// 任一查询、恢复或版本守卫不可靠都返回 `跳过(原因)` 而不写入；返回值只进诊断文本，
/// 不参与严格像素门禁，也不改变后续任何渲染语义。
fn diag_native_gl_clear_note(frame: &mut GlesFrame<'_, '_>) -> String {
    // 阶段一：只读版本与 clear color；查询不干净就整项放弃，绝不在无法恢复时改写 GL。
    let (version, saved_clear, query_errors) = match frame.with_context(|gl| {
        // SAFETY: `GL_VERSION` 返回实现持有的 NUL 结尾静态字符串。
        let version_ptr = unsafe { gl.GetString(gles_ffi::VERSION) };
        let version = if version_ptr.is_null() {
            None
        } else {
            // SAFETY: 见上，指针非空且由 GL 保证 NUL 结尾。
            Some(
                unsafe { std::ffi::CStr::from_ptr(version_ptr.cast()) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        let mut saved_clear = [0.0f32; 4];
        // SAFETY: 只写入调用方持有的 16 字节。
        unsafe { gl.GetFloatv(gles_ffi::COLOR_CLEAR_VALUE, saved_clear.as_mut_ptr()) };
        (version, saved_clear, diag_drain_gl_errors(gl))
    }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };
    let Some(version) = version else {
        return "跳过(GL_VERSION 读取失败)".to_owned();
    };
    // 本控制的观测手段是 `diag_quadrant_pixel_note`，它依赖 ES3 的 `glReadBuffer(GL_BACK)`
    // 与 pixel pack buffer 入口；上下文低于 3.0 时控制无法被观测，直接跳过更诚实。
    if !version.contains("OpenGL ES 3") && !version.contains("OpenGL ES 4") {
        return format!("跳过(GL 非 ES3: {version})");
    }
    if !query_errors.is_empty() {
        return format!(
            "跳过(clear color 查询错误 {})",
            diag_format_gl_errors(&query_errors)
        );
    }

    // 阶段二：只查询原生清屏会受约束的 framebuffer / scissor / color mask，不改写它们。
    // 查询失败只让对应字段降级为 `?err(...)`，不阻止清屏本身。
    let extras = match frame.with_context(|gl| {
        let mut draw_fb = 0i32;
        let mut scissor_box = [0i32; 4];
        // SAFETY: 两个调用各自只写入调用方持有的整数缓冲。
        unsafe {
            gl.GetIntegerv(gles_ffi::DRAW_FRAMEBUFFER_BINDING, &mut draw_fb);
            gl.GetIntegerv(gles_ffi::SCISSOR_BOX, scissor_box.as_mut_ptr());
        }
        let scissor_on = unsafe { gl.IsEnabled(gles_ffi::SCISSOR_TEST) } != 0;
        let frame_errors = diag_drain_gl_errors(gl);
        let mut color_mask = [0u8; 4];
        // SAFETY: 只写入调用方持有的 4 字节。
        unsafe { gl.GetBooleanv(gles_ffi::COLOR_WRITEMASK, color_mask.as_mut_ptr()) };
        let mask_errors = diag_drain_gl_errors(gl);
        (
            draw_fb,
            scissor_on,
            scissor_box,
            color_mask,
            frame_errors,
            mask_errors,
        )
    }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };
    let (draw_fb, scissor_on, scissor_box, color_mask, frame_errors, mask_errors) = extras;
    // 缓冲选择只读查询：`draw_framebuffer` 绑定相同并不代表 draw/read buffer 的选择值指向
    // 同一个面——原生 `glClear` 写入 `DRAW_BUFFER0`，而 `diag_read_back_pixel` 固定从
    // `GL_BACK` 读取；两者选择值不同就会表现为“清屏执行成功但回读全黑”。注意单像素日志里
    // 的 `rb=` 是 read framebuffer binding，不是这里的 read buffer 选择值。
    //
    // 两项各自查询、各自排空错误队列，任一失败只让其字段降级为 `?err(...)`，其余诊断继续；
    // 全程只读，不调用 `glDrawBuffer`／`glReadBuffer`，不改写任何 GL 状态。
    let (draw_buffer, draw_buffer_errors) = match frame.with_context(|gl| {
        let mut draw_buffer = 0i32;
        // SAFETY: 只写入调用方持有的 4 字节。
        unsafe { gl.GetIntegerv(gles_ffi::DRAW_BUFFER0, &mut draw_buffer) };
        (draw_buffer, diag_drain_gl_errors(gl))
    }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };
    let (read_buffer, read_buffer_errors) = match frame.with_context(|gl| {
        let mut read_buffer = 0i32;
        // SAFETY: 只写入调用方持有的 4 字节。
        unsafe { gl.GetIntegerv(gles_ffi::READ_BUFFER, &mut read_buffer) };
        (read_buffer, diag_drain_gl_errors(gl))
    }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };
    let state_note = {
        let mut parts = Vec::new();
        if frame_errors.is_empty() {
            parts.push(format!(
                "dfb={draw_fb} scissor={} box=[{},{},{},{}]",
                u8::from(scissor_on),
                scissor_box[0],
                scissor_box[1],
                scissor_box[2],
                scissor_box[3],
            ));
        } else {
            parts.push(format!(
                "state=?err({})",
                diag_format_gl_errors(&frame_errors)
            ));
        }
        if mask_errors.is_empty() {
            parts.push(format!(
                "mask={}{}{}{}",
                u8::from(color_mask[0] != 0),
                u8::from(color_mask[1] != 0),
                u8::from(color_mask[2] != 0),
                u8::from(color_mask[3] != 0),
            ));
        } else {
            parts.push(format!(
                "mask=?err({})",
                diag_format_gl_errors(&mask_errors)
            ));
        }
        if draw_buffer_errors.is_empty() {
            parts.push(format!("drawbuf=0x{draw_buffer:04X}"));
        } else {
            parts.push(format!(
                "drawbuf=?err({})",
                diag_format_gl_errors(&draw_buffer_errors)
            ));
        }
        if read_buffer_errors.is_empty() {
            parts.push(format!("readbuf=0x{read_buffer:04X}"));
        } else {
            parts.push(format!(
                "readbuf=?err({})",
                diag_format_gl_errors(&read_buffer_errors)
            ));
        }
        parts.join(" ")
    };

    // 阶段三：执行控制。只改写 clear color；调用前、调用后各排空一次错误队列，
    // 随后恢复原值并回读确认，回读本身失败也算“无法安全恢复”。
    let (pre_errors, clear_errors, restore_errors, back_clear, back_errors) = match frame
        .with_context(|gl| {
            let pre_errors = diag_drain_gl_errors(gl);
            // SAFETY: 只改写 clear color；framebuffer、scissor、color mask 保持 renderer
            // 当前值，采样点是否被覆盖由阶段二记录的状态解释。
            unsafe {
                gl.ClearColor(0.0, 1.0, 1.0, 1.0);
                gl.Clear(gles_ffi::COLOR_BUFFER_BIT);
            }
            let clear_errors = diag_drain_gl_errors(gl);
            // SAFETY: 恢复的正是阶段一读到的原值，且只动 clear color。
            unsafe {
                gl.ClearColor(
                    saved_clear[0],
                    saved_clear[1],
                    saved_clear[2],
                    saved_clear[3],
                );
            }
            let restore_errors = diag_drain_gl_errors(gl);
            let mut back_clear = [0.0f32; 4];
            // SAFETY: 只写入调用方持有的 16 字节。
            unsafe { gl.GetFloatv(gles_ffi::COLOR_CLEAR_VALUE, back_clear.as_mut_ptr()) };
            let back_errors = diag_drain_gl_errors(gl);
            (
                pre_errors,
                clear_errors,
                restore_errors,
                back_clear,
                back_errors,
            )
        }) {
        Ok(values) => values,
        Err(error) => return format!("跳过(GlesFrame::with_context 失败: {error})"),
    };
    if !restore_errors.is_empty() {
        // 恢复失败意味着 clear color 可能仍被改写：必须把实际残留值显式写进诊断，
        // 不能只报一个“跳过”让状态漂移不可见。
        return format!(
            "跳过(clear color 恢复错误 {} back=[{},{},{},{}])",
            diag_format_gl_errors(&restore_errors),
            back_clear[0],
            back_clear[1],
            back_clear[2],
            back_clear[3],
        );
    }
    if !back_errors.is_empty() {
        return format!(
            "跳过(clear color 恢复回读错误 {})",
            diag_format_gl_errors(&back_errors)
        );
    }
    let restored = (0..4).all(|index| (saved_clear[index] - back_clear[index]).abs() <= 1e-6);
    format!(
        "gl_clear=[{},{},{},{}] err(pre)={} err(clear)={} err(restore)={} \
         clr_back=[{},{},{},{}] clr_restore={} {}",
        saved_clear[0],
        saved_clear[1],
        saved_clear[2],
        saved_clear[3],
        diag_format_gl_errors(&pre_errors),
        diag_format_gl_errors(&clear_errors),
        diag_format_gl_errors(&restore_errors),
        back_clear[0],
        back_clear[1],
        back_clear[2],
        back_clear[3],
        u8::from(restored),
        state_note,
    )
}

/// Winit 两条输出绘制路径共用的默认 framebuffer draw buffer 初始化。
///
/// OpenGL ES 3.0 规范允许默认 framebuffer 的 `GL_DRAW_BUFFER0` 初始为 `GL_NONE`：此时
/// clear 与绘制调用仍会成功返回（GL 错误码为 0），但不会写入任何颜色面，目标回读只会
/// 得到全黑。Winit 的 EGL surface 正是绑定到默认 framebuffer
/// （`DRAW_FRAMEBUFFER_BINDING == 0`），因此必须显式把 `GL_BACK` 选为颜色写入目标。
/// 本函数是目标 surface 的必要初始化，不受 `SKY_MIRROR_R2_TARGET_DIAG` 控制；任一前置
/// 条件或设置后校验失败都以包含实测值的错误失败关闭，绝不返回“已绘制”报告。
///
/// 调用时机：`renderer.render()` 之后、任何 clear/draw 之前，且 `GlesFrame` 仍活跃。
/// `GlesFrame::with_context` 不会重新 make_current，GL 上下文与 EGL surface 因此保持
/// `render()` 刚建立的绑定。`target_surface` 必须是在 `backend.bind()` 之前保存的
/// EGL surface handle——`bind()` 的可变借用存活期间不允许再借用 backend。
///
/// 前置校验只读：EGL current draw/read surface 必须都等于目标 surface，且 draw framebuffer
/// 必须是默认 framebuffer（0）；不满足时不强行设置 `GL_BACK`，只用实测值报错。设置后必须
/// 确认没有 GL 错误且 `DRAW_BUFFER0 == GL_BACK` 才继续。本函数不改写 `GL_READ_BUFFER`；
/// 成功时返回紧凑诊断文本（原选择值 / 设置后值 / EGL surface 是否匹配），供调用点记录。
fn configure_winit_default_draw_buffer(
    frame: &mut GlesFrame<'_, '_>,
    target_surface: egl_ffi::types::EGLSurface,
) -> Result<String, String> {
    // 阶段一：只读查询 draw framebuffer 与 draw buffer 现值。查询本身失败或留下 GL 错误时
    // 状态不可信，直接失败关闭，避免在未知状态下改写 GL。
    let (draw_framebuffer, draw_buffer_before, state_errors) = frame
        .with_context(|gl| {
            let mut draw_framebuffer = 0i32;
            let mut draw_buffer_before = 0i32;
            // SAFETY: 两个调用各自只写入调用方持有的 4 字节；`with_context` 不重新绑定
            // surface，上下文已由 `render()` 置为 current。
            unsafe {
                gl.GetIntegerv(gles_ffi::DRAW_FRAMEBUFFER_BINDING, &mut draw_framebuffer);
                gl.GetIntegerv(gles_ffi::DRAW_BUFFER0, &mut draw_buffer_before);
            }
            (
                draw_framebuffer,
                draw_buffer_before,
                diag_drain_gl_errors(gl),
            )
        })
        .map_err(|error| {
            format!("Winit 目标 draw buffer 初始化失败：GlesFrame::with_context 失败: {error}")
        })?;

    // EGL current surface 是只读身份查询，逐项与 bind() 前保存的目标 handle 比较。
    // SAFETY: `GetCurrentSurface` 只读取当前线程的 EGL draw/read 绑定，不改变任何状态。
    let (current_draw, current_read) = unsafe {
        (
            egl_ffi::GetCurrentSurface(egl_ffi::DRAW as egl_ffi::types::EGLint),
            egl_ffi::GetCurrentSurface(egl_ffi::READ as egl_ffi::types::EGLint),
        )
    };
    let egl_matches = current_draw == target_surface && current_read == target_surface;
    let measured = format!(
        "dfb={draw_framebuffer} draw0=0x{draw_buffer_before:04X} egl_match={} \
         egl_draw={current_draw:p} egl_read={current_read:p} target={target_surface:p}",
        u8::from(egl_matches),
    );

    if !state_errors.is_empty() {
        return Err(format!(
            "Winit 目标 draw buffer 初始化失败：状态查询 GL 错误 {}；{measured}",
            diag_format_gl_errors(&state_errors)
        ));
    }
    if draw_framebuffer != 0 {
        return Err(format!(
            "Winit 目标 draw buffer 初始化失败：DRAW_FRAMEBUFFER_BINDING 非 0（不是默认 \
             framebuffer），不为 FBO 强行设置 GL_BACK；{measured}"
        ));
    }
    if !egl_matches {
        return Err(format!(
            "Winit 目标 draw buffer 初始化失败：当前 EGL draw/read surface 不是 Winit 目标 \
             surface；{measured}"
        ));
    }

    // 阶段二：先排空历史错误，再设置 `GL_BACK`；随后分别校验设置错误、设置后查询错误与
    // `DRAW_BUFFER0` 实际值，三者任一不满足都失败关闭。
    let (pre_errors, draw_buffer_after, set_errors, verify_errors) = frame
        .with_context(|gl| {
            let pre_errors = diag_drain_gl_errors(gl);
            // SAFETY: `GL_BACK` 是 GLES 3.0 为默认 framebuffer 定义的颜色写入目标；只改写
            // draw buffer 选择，不改 framebuffer、scissor、color mask 或 read buffer。
            let back = [gles_ffi::BACK];
            unsafe { gl.DrawBuffers(1, back.as_ptr()) };
            let set_errors = diag_drain_gl_errors(gl);
            let mut draw_buffer_after = 0i32;
            // SAFETY: 只写入调用方持有的 4 字节。
            unsafe { gl.GetIntegerv(gles_ffi::DRAW_BUFFER0, &mut draw_buffer_after) };
            (
                pre_errors,
                draw_buffer_after,
                set_errors,
                diag_drain_gl_errors(gl),
            )
        })
        .map_err(|error| {
            format!("Winit 目标 draw buffer 初始化失败：GlesFrame::with_context 失败: {error}")
        })?;

    if !set_errors.is_empty() {
        return Err(format!(
            "Winit 目标 draw buffer 初始化失败：glDrawBuffers(GL_BACK) GL 错误 {}，设置后 \
             draw0=0x{draw_buffer_after:04X}；{measured}",
            diag_format_gl_errors(&set_errors)
        ));
    }
    if !verify_errors.is_empty() {
        return Err(format!(
            "Winit 目标 draw buffer 初始化失败：设置后 DRAW_BUFFER0 查询 GL 错误 {}，设置后 \
             draw0=0x{draw_buffer_after:04X}；{measured}",
            diag_format_gl_errors(&verify_errors)
        ));
    }
    if draw_buffer_after != gles_ffi::BACK as i32 {
        return Err(format!(
            "Winit 目标 draw buffer 初始化失败：设置后 DRAW_BUFFER0=0x{draw_buffer_after:04X} \
             （期望 GL_BACK=0x{:04X}），颜色写入仍被抑制；{measured}",
            gles_ffi::BACK
        ));
    }

    Ok(format!(
        "draw_target=0x{draw_buffer_before:04X}->0x{draw_buffer_after:04X} egl_match=1 \
         dfb=0 err(pre)={} err(set)={}",
        diag_format_gl_errors(&pre_errors),
        diag_format_gl_errors(&set_errors),
    ))
}

/// present 阶段错误随附的短诊断快照。
///
/// 只记录已经真实完成的观测：尚未走到的步骤保持 `None`，渲染为 `unavailable`，避免把
/// “未执行”误读成“已通过”。仅在 `SKY_MIRROR_R2_TARGET_DIAG=1` 时填充，并只被错误路径
/// 读取；诊断关闭时不改变任何错误值与控制流。
#[derive(Default)]
struct R2PresentStageDiag {
    /// draw buffer 初始化 helper 成功返回的证据。
    draw_target: Option<String>,
    /// `renderer.render` 之后、原生 clear 之前的 GL error code。
    gl_pre: Option<String>,
    /// Smithay `frame.clear` 之后的 GL error code。
    gl_clear: Option<String>,
    /// `draw_render_elements` 之后的 GL error code。
    gl_draw: Option<String>,
    /// 原生 `glClear` 正向控制的紧凑结果。
    native_clear: Option<String>,
    /// 原生 `glClear` 后红色象限中心单像素采样。
    gl_clear_px: Option<String>,
    /// Smithay `frame.clear` 后同一坐标的单像素采样。
    clear_px: Option<String>,
    /// 纹理绘制后同一坐标的单像素采样。
    back_px: Option<String>,
    /// 重新绑定 Winit surface 后、submit 前同一坐标的单像素采样。
    pre_submit_px: Option<String>,
}

impl R2PresentStageDiag {
    /// 按固定字段顺序渲染短诊断；未采集字段一律显示 `unavailable`，不省略不猜测。
    fn render(&self) -> String {
        let field = |value: &Option<String>| value.as_deref().unwrap_or("unavailable").to_owned();
        format!(
            "draw_target={}; gl@pre={}; gl@clear={}; gl@draw={}; native_clear={}; \
             gl_clear_px={}; clear_px={}; back_px={}; pre_submit_px={}",
            field(&self.draw_target),
            field(&self.gl_pre),
            field(&self.gl_clear),
            field(&self.gl_draw),
            field(&self.native_clear),
            field(&self.gl_clear_px),
            field(&self.clear_px),
            field(&self.back_px),
            field(&self.pre_submit_px),
        )
    }
}

/// 诊断模式下的阶段错误包装。
///
/// 诊断关闭时与原有 `?` 完全一致地返回原始错误（同一 `From<E> for Box<dyn Error>` 转换），
/// 不改变错误值与控制流；诊断开启时只在错误文本前附加阶段标签与已收集的短诊断，绝不
/// 吞掉错误、重试或继续执行到成功报告。`collected` 只在错误路径读取。
fn stage_error<T, E>(
    diag: bool,
    stage: &str,
    collected: &R2PresentStageDiag,
    result: Result<T, E>,
) -> Result<T, Box<dyn Error>>
where
    E: Error + 'static,
{
    match result {
        Ok(value) => Ok(value),
        Err(error) if diag => Err(format!(
            "R2 stage[{stage}] 失败: {error}; 已收集: {}",
            collected.render()
        )
        .into()),
        Err(error) => Err(error.into()),
    }
}

/// 把一次 `diag_frame_gl_errors` 采样渲染为阶段诊断字段值。
///
/// `None`（诊断关闭或采样未执行）固定渲染为 `unavailable`；采样自身不可靠时保留其
/// `跳过(原因)` 文本，不把“查询失败”伪装成“无错误”。
fn diag_gl_stage_note(value: &Option<Result<Vec<u32>, String>>) -> String {
    match value {
        Some(Ok(codes)) => diag_format_gl_errors(codes),
        Some(Err(reason)) => reason.clone(),
        None => "unavailable".to_owned(),
    }
}

impl NestedWinitOutputOwner {
    /// 创建 nested Winit/EGL/GLES target，但不执行 client 协议或 Core mutation。
    ///
    /// # Errors
    ///
    /// 宿主显示系统、Winit window、EGL 或 GLES 初始化失败时原样返回；局部成功资源会
    /// 在返回前释放，调用者不需要补偿任何 Core state。
    pub(crate) fn new() -> Result<Self, Box<dyn Error>> {
        let (backend, event_loop) = winit::init::<GlesRenderer>()?;
        Ok(Self {
            backend,
            event_loop,
            retained_source_buffers: Vec::new(),
        })
    }

    /// 在真实 Winit/EGL target 上提交一次固定背景首帧。
    ///
    /// 全 target 矩形只作为 EGL submit damage，不能外推为 client surface damage。
    ///
    /// # Errors
    ///
    /// bind、默认 framebuffer 的 GL_BACK draw buffer 初始化、GLES render/clear/finish 或
    /// EGL submit 任一步失败都会原样返回；drop 此 owner 即完成唯一 cleanup，函数不改动
    /// Core，因此没有 Core 补偿。
    pub(crate) fn present_first_frame(
        &mut self,
    ) -> Result<NestedWinitFirstFrameReport, Box<dyn Error>> {
        let size = self.backend.window_size();
        let damage: Rectangle<i32, Physical> = Rectangle::from_size(size);
        // 诊断关闭时本快照不被读取；开启时只用于给错误附加阶段上下文。
        let diag = r2_target_diag_enabled();
        let mut stage_diag = R2PresentStageDiag::default();
        // `bind()` 的可变借用存活期间不能再借用 backend，因此必须在 bind 前保存目标
        // EGL surface handle，供 render() 后的 current surface 身份校验使用。
        let target_surface = self.backend.egl_surface().get_surface_handle();
        {
            let (renderer, mut framebuffer) = self.backend.bind()?;
            let mut frame = stage_error(
                diag,
                "first-frame renderer.render",
                &stage_diag,
                renderer.render(&mut framebuffer, size, Transform::Flipped180),
            )?;
            // 默认 framebuffer 可能以 GL_NONE draw buffer 抑制颜色写入；必须在 clear 前
            // 显式选择 GL_BACK，失败则直接返回错误而不是提交一帧未绘制的画面。
            let draw_target_note = configure_winit_default_draw_buffer(&mut frame, target_surface)?;
            if diag {
                stage_diag.draw_target = Some(draw_target_note);
            }
            stage_error(
                diag,
                "first-frame frame.clear",
                &stage_diag,
                frame.clear(Color32F::new(0.08, 0.12, 0.2, 1.0), &[damage]),
            )?;
            let _ = stage_error(
                diag,
                "first-frame frame.finish",
                &stage_diag,
                frame.finish(),
            )?;
        }
        self.backend.submit(Some(&[damage]))?;

        Ok(NestedWinitFirstFrameReport {
            target_created: true,
            target_size: (size.w, size.h),
            renderer_bound: true,
            first_frame_cleared: true,
            backbuffer_submitted: true,
            client_buffer_imported: false,
            client_frame_done_sent: false,
        })
    }

    /// 有界泵送宿主 Winit 事件，直到观察到 `Resized`（窗口完成一次配置）或预算耗尽。
    ///
    /// winit 要求外部循环周期性 `dispatch_new_events` 才会推进宿主窗口系统回调，否则
    /// 已提交内容可能停留在黑窗口。本方法只消费宿主事件、不修改 Core/backend 状态；
    /// 窗口在 present 前关闭（`PumpStatus::Exit` 或 `CloseRequested`）按错误返回。
    ///
    /// 这是 present 内的一次性泵送：本 owner 尚未接入长期 runtime loop，present 之后
    /// 不再周期泵送，属于本切片的已知限制，不宣称 input 或长期窗口交互能力。
    ///
    /// # Errors
    ///
    /// 窗口已关闭时返回错误；预算耗尽但未观察到 `Resized` 不算错误，之后的目标
    /// framebuffer 回读门禁负责判定内容是否真正绘制到 target。
    fn pump_winit_events_until_configured(&mut self) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + WINIT_PRESENT_PUMP_BUDGET;
        loop {
            let mut resized = false;
            let mut close_requested = false;
            let status = self.event_loop.dispatch_new_events(|event| match event {
                WinitEvent::Resized { .. } => resized = true,
                WinitEvent::CloseRequested => close_requested = true,
                _ => {}
            });
            if matches!(status, PumpStatus::Exit(_)) || close_requested {
                return Err("Winit 窗口在 present 前已关闭（event loop 退出）".into());
            }
            if resized || Instant::now() >= deadline {
                return Ok(());
            }
            thread::sleep(WINIT_PRESENT_PUMP_INTERVAL);
        }
    }

    /// 导入一个已由 coordinator 原子验证过的 SHM buffer，并呈现到真实 Winit/EGL/GLES target。
    ///
    /// 调用方必须在转移 `PendingShmBufferResource` 前确认 source session、adapter
    /// surface/toplevel、admission ledger 和 live Core window；本 owner 不会重新猜测这些
    /// identity。任一默认 framebuffer draw buffer 初始化、import、render 或 submit 失败
    /// 均返回错误且不发送 frame done；resource 由调用方的失败路径回收，不会留在 display
    /// FIFO 中。
    ///
    /// 呈现顺序：有界泵送宿主事件 → 导入并回读 source texture → 建立 render 帧并显式
    /// 选择默认 framebuffer 的 `GL_BACK` 颜色目标 → 绘制 → **从目标 framebuffer 读回
    /// 元素区域并与 source 逐像素比对** → submit → 再泵一次宿主事件。
    /// 只有目标回读与 source 一致时才返回成功，从而让“最终 target 输出”而非仅
    /// “导入内容”进入上层门禁。
    pub(crate) fn present_validated_shm_buffer(
        &mut self,
        resource: &PendingShmBufferResource,
    ) -> Result<NestedWinitShmPresentReport, Box<dyn Error>> {
        // 先推进宿主窗口系统回调（configure/绘制呈现路径），再读取 window_size，
        // 使后续绘制区域基于最近一次配置的物理尺寸。
        self.pump_winit_events_until_configured()?;

        // R2 target 黑帧最小运行时诊断：仅显式 `SKY_MIRROR_R2_TARGET_DIAG=1` 时收集；
        // 未开启时下面所有 `diag*` 分支都不执行，绘制、门禁、submit 与清理语义不变。
        let diag = r2_target_diag_enabled();
        // 阶段短诊断快照：只在诊断开启时填充、只在错误路径读取；未完成字段渲染为
        // `unavailable`，不改变任何错误值与控制流。
        let mut stage_diag = R2PresentStageDiag::default();

        let output_size = self.backend.window_size();
        let output_damage: Rectangle<i32, Physical> = Rectangle::from_size(output_size);
        let buffer_damage: Rectangle<i32, BufferCoord> =
            Rectangle::from_size((resource.metadata.width, resource.metadata.height).into());

        // 同 present_first_frame：`bind()` 前保存目标 EGL surface handle，render() 后用它
        // 确认 current draw/read surface 正是该 Winit target。
        let target_surface = self.backend.egl_surface().get_surface_handle();
        let final_rgb = {
            let (renderer, mut framebuffer) = self.backend.bind()?;
            // 诊断 1：同一次 `bind()` 内的窗口与 framebuffer 真实尺寸。
            // `bind()` 返回的借用活跃期间无法再调用 `&self` 的 `window_size()`（可变借用
            // 与不可变借用冲突），因此窗口读数取自紧邻 `bind()` 之前、中间没有任何窗口或
            // GL 操作的同一次 `window_size()` 调用；framebuffer 读数取自 `bind()` 返回后
            // 立即取值。两者只用于判定尺寸是否相等，不用于把结果写成“已绘制成功”。
            let diag_sizes = if diag {
                let framebuffer_size = framebuffer.size();
                let same =
                    output_size.w == framebuffer_size.w && output_size.h == framebuffer_size.h;
                Some((
                    same,
                    format!(
                        "win={}x{} fb={}x{} same={}",
                        output_size.w,
                        output_size.h,
                        framebuffer_size.w,
                        framebuffer_size.h,
                        u8::from(same)
                    ),
                ))
            } else {
                None
            };
            let diag_sizes_same = diag_sizes.as_ref().map(|(same, _)| *same);
            let texture = stage_error(
                diag,
                "SHM texture import",
                &stage_diag,
                renderer.import_shm_buffer(&resource.buffer, None, &[buffer_damage]),
            )?;
            let expected_width = u32::try_from(resource.metadata.width)
                .map_err(|_| "已验证 SHM metadata width 无法转换为 texture width")?;
            let expected_height = u32::try_from(resource.metadata.height)
                .map_err(|_| "已验证 SHM metadata height 无法转换为 texture height")?;
            if texture.width() != expected_width
                || texture.height() != expected_height
                || texture.format() != Some(Fourcc::Xrgb8888)
            {
                return Err("GLES imported texture 与已验证 XRGB8888 metadata 不一致".into());
            }
            let mapping = stage_error(
                diag,
                "source texture copy",
                &stage_diag,
                renderer.copy_texture(&texture, buffer_damage, Fourcc::Xrgb8888),
            )?;
            let mapping_flipped = mapping.flipped();
            let mapped = stage_error(
                diag,
                "source texture map",
                &stage_diag,
                renderer.map_texture(&mapping),
            )?;
            let readback_rgb = normalize_xrgb8888_readback_rgb(
                mapped,
                resource.metadata.width,
                resource.metadata.height,
                mapping_flipped,
            )
            .ok_or("GLES XRGB8888 texture readback 长度与已验证 SHM metadata 不一致")?;
            let element = TextureRenderElement::from_static_texture(
                Id::from(&resource.buffer),
                renderer.context_id(),
                (0.0, 0.0),
                texture,
                1,
                Transform::Normal,
                Some(1.0),
                None,
                None,
                None,
                Kind::Unspecified,
            );
            let mut frame = stage_error(
                diag,
                "renderer.render",
                &stage_diag,
                renderer.render(&mut framebuffer, output_size, Transform::Flipped180),
            )?;
            // 诊断 3：清屏前基线、清屏后、纹理绘制后的 GL error code，全部在仍活跃的
            // `GlesFrame` 上读取；改用 `renderer.with_context` 会重新 make_current，
            // 可能切到没有 surface 的上下文，因此禁止。
            let diag_err_pre = diag.then(|| diag_frame_gl_errors(&mut frame));
            if diag {
                stage_diag.gl_pre = Some(diag_gl_stage_note(&diag_err_pre));
            }
            // 目标 surface 的必要初始化，不受诊断开关控制：必须在任何 clear/draw（包括
            // 下面的原生 glClear 控制）之前把默认 framebuffer 的颜色目标设为 GL_BACK。
            // 初始化失败直接返回错误，绝不继续生成“已绘制”报告。
            let draw_target_note = configure_winit_default_draw_buffer(&mut frame, target_surface)?;
            if diag {
                stage_diag.draw_target = Some(draw_target_note.clone());
            }
            // 诊断 6：原生 `glClear` 正向控制，必须在 Smithay `frame.clear`（`draw_solid`
            // 绘制）之前执行，才能作为一条不经 Smithay 绘制管线的对照；紧随其后在同一个
            // 仍活跃的 `GlesFrame`、同一坐标读取 `gl_clear_px`。仅诊断模式执行。
            let diag_native_clear = diag.then(|| diag_native_gl_clear_note(&mut frame));
            if diag {
                stage_diag.native_clear = diag_native_clear.clone();
            }
            let diag_gl_clear_pixel = diag.then(|| {
                diag_quadrant_pixel_note(
                    &mut frame,
                    "gl_clear_px",
                    output_size.w,
                    output_size.h,
                    expected_width,
                    expected_height,
                )
            });
            if diag {
                stage_diag.gl_clear_px = diag_gl_clear_pixel.clone();
            }
            // 诊断模式下临时把清屏色改为饱和洋红：清屏色与 source 图案颜色必然不同，
            // clear_px/back_px 两次同点采样因此能区分“清屏没有写入 target”和“纹理绘制
            // 没有写入 target”。正常模式继续纯黑清屏，严格逐像素门禁、错误路径、submit、
            // frame done 授权与清理语义都保持原样。
            let clear_color = if diag {
                Color32F::new(1.0, 0.0, 1.0, 1.0)
            } else {
                Color32F::new(0.0, 0.0, 0.0, 1.0)
            };
            stage_error(
                diag,
                "frame.clear",
                &stage_diag,
                frame.clear(clear_color, &[output_damage]),
            )?;
            let diag_err_clear = diag.then(|| diag_frame_gl_errors(&mut frame));
            if diag {
                stage_diag.gl_clear = Some(diag_gl_stage_note(&diag_err_clear));
            }
            // 诊断 5：清屏后立刻在同一仍活跃的 `GlesFrame` 上采样，作为纹理绘制前的
            // “target 已有可见像素”基线；与 back_px 同一坐标，两次读取之间只相差一次
            // `draw_render_elements`。
            let diag_clear_pixel = diag.then(|| {
                diag_quadrant_pixel_note(
                    &mut frame,
                    "clear_px",
                    output_size.w,
                    output_size.h,
                    expected_width,
                    expected_height,
                )
            });
            if diag {
                stage_diag.clear_px = diag_clear_pixel.clone();
            }
            // 诊断 2：保留 `draw_render_elements` 的返回值，`None` 表示 `render_damage`
            // 为空、元素根本没被绘制，`Some(n)` 给出实际参与绘制的 damage 数量。
            let drawn_damage = stage_error(
                diag,
                "draw_render_elements",
                &stage_diag,
                draw_render_elements::<GlesRenderer, _, _>(
                    &mut frame,
                    1.0,
                    &[element],
                    &[output_damage],
                ),
            )?;
            let diag_err_draw = diag.then(|| diag_frame_gl_errors(&mut frame));
            if diag {
                stage_diag.gl_draw = Some(diag_gl_stage_note(&diag_err_draw));
            }
            // 诊断 4：必须在 `frame.finish()` 与 `copy_framebuffer` 重新绑定之前读取，
            // 否则无法区分“绘制时已经黑”和“后续重绑/回读才变黑”。
            let diag_note = if diag {
                let gl_note = |value: &Option<Result<Vec<u32>, String>>| match value {
                    None => "-".to_owned(),
                    Some(Ok(codes)) => diag_format_gl_errors(codes),
                    Some(Err(reason)) => reason.clone(),
                };
                let sizes_note = diag_sizes
                    .as_ref()
                    .map(|(_, text)| text.clone())
                    .unwrap_or_else(|| "尺寸未采集".to_owned());
                let draw_note = match &drawn_damage {
                    None => "draw=None".to_owned(),
                    Some(damage) => format!("draw=Some({})", damage.len()),
                };
                let native_clear_note =
                    diag_native_clear.unwrap_or_else(|| "gl_clear=-".to_owned());
                let gl_clear_pixel_note =
                    diag_gl_clear_pixel.unwrap_or_else(|| "gl_clear_px=-".to_owned());
                let clear_pixel_note = diag_clear_pixel.unwrap_or_else(|| "clear_px=-".to_owned());
                let pixel_note = diag_quadrant_pixel_note(
                    &mut frame,
                    "back_px",
                    output_size.w,
                    output_size.h,
                    expected_width,
                    expected_height,
                );
                stage_diag.back_px = Some(pixel_note.clone());
                format!(
                    "{sizes_note}; {draw_target_note}; {draw_note}; gl@pre={} gl@clear={} \
                     gl@draw={}; {native_clear_note}; {gl_clear_pixel_note}; {clear_pixel_note}; \
                     {pixel_note}",
                    gl_note(&diag_err_pre),
                    gl_note(&diag_err_clear),
                    gl_note(&diag_err_draw),
                )
            } else {
                String::new()
            };
            let diag_part = if diag_note.is_empty() {
                String::new()
            } else {
                format!("; R2 diag: {diag_note}")
            };
            let _ = stage_error(diag, "frame.finish", &stage_diag, frame.finish())?;
            // 绘制已进入 target framebuffer、submit 之前：读回元素所在的最终目标区域。
            // source texture 回读只证明导入内容，不能证明目标 framebuffer 已被绘制，
            // 因此这里必须以 target 像素作为最终输出证据（否则黑窗口会误报成功）。
            let output_dims = (
                u32::try_from(output_size.w).map_err(|_| "output 宽度无法转换为 u32")?,
                u32::try_from(output_size.h).map_err(|_| "output 高度无法转换为 u32")?,
            );
            let region = target_readback_region(output_dims, (expected_width, expected_height))
                .ok_or("绘制元素大于目标 framebuffer，无法读回最终输出")?;
            let target_mapping = stage_error(
                diag,
                "target copy_framebuffer",
                &stage_diag,
                renderer.copy_framebuffer(&framebuffer, region, Fourcc::Xrgb8888),
            )?;
            let target_mapped = stage_error(
                diag,
                "target map_texture",
                &stage_diag,
                renderer.map_texture(&target_mapping),
            )?;
            // glReadPixels 的区域行序自 GL 底部向上：区域首行是元素的 logical 底行，
            // 所以必须按未翻转归一化；不能沿用 mapping.flipped()，它恒为 true。
            let target_rgb = normalize_xrgb8888_readback_rgb(
                target_mapped,
                resource.metadata.width,
                resource.metadata.height,
                false,
            )
            .ok_or("目标 framebuffer XRGB8888 回读长度与已验证 SHM metadata 不一致")?;
            if target_rgb != readback_rgb {
                let mismatch = target_mismatch_diagnostic(
                    &target_rgb,
                    &readback_rgb,
                    usize::try_from(expected_width).unwrap_or(0),
                );
                // 门禁失败时补一次全帧回读：区域为黑本身无法区分“图案画到别处”和
                // “target 根本没有图案”，必须用整帧非黑像素的包围盒给出位置证据。
                // 摘要只是 best-effort：它自身失败时降级为“摘要不可用”，绝不吞掉上面
                // 这条不一致门禁消息；只输出计数与包围盒，不倾倒像素。失败仍返回 Err。
                let summary = (|| -> Result<String, String> {
                    let full_region: Rectangle<i32, BufferCoord> =
                        Rectangle::from_size((output_size.w, output_size.h).into());
                    let full_mapping = stage_error(
                        diag,
                        "full-frame copy_framebuffer",
                        &stage_diag,
                        renderer.copy_framebuffer(&framebuffer, full_region, Fourcc::Xrgb8888),
                    )
                    .map_err(|error| format!("全帧回读失败: {error}"))?;
                    let full_mapped = stage_error(
                        diag,
                        "full-frame map_texture",
                        &stage_diag,
                        renderer.map_texture(&full_mapping),
                    )
                    .map_err(|error| format!("全帧 mapping 失败: {error}"))?;
                    let full_rgb = normalize_xrgb8888_readback_rgb(
                        full_mapped,
                        output_size.w,
                        output_size.h,
                        false,
                    )
                    .ok_or_else(|| {
                        "目标 framebuffer 全帧 XRGB8888 回读长度与 window_size 不一致".to_owned()
                    })?;
                    Ok(target_frame_summary(
                        &full_rgb,
                        usize::try_from(output_size.w).unwrap_or(0),
                        usize::try_from(output_size.h).unwrap_or(0),
                        region,
                        &readback_rgb,
                        usize::try_from(expected_width).unwrap_or(0),
                    ))
                })()
                .unwrap_or_else(|error| format!("全帧摘要不可用: {error}"));
                return Err(format!(
                    "目标 framebuffer 与 source texture 回读不一致（final target 未得到期望像素）: {mismatch}; {summary}{diag_part}"
                )
                .into());
            }
            // 诊断开启且尺寸不等时绝不把结果写成“已绘制成功”：即使逐像素门禁意外通过，
            // 也必须以尺寸不一致证据返回错误。诊断未开启时 `diag_sizes_same` 恒为
            // `None`，本分支不存在，行为与原实现一致。
            if diag_sizes_same == Some(false) {
                return Err(
                    format!("R2 target 诊断：window_size 与 framebuffer.size 不等，不记为已绘制成功{diag_part}")
                        .into(),
                );
            }

            // source/target 严格逐像素门禁已经通过，但 `map_texture` 会把 EGL context
            // make-current 到无 surface 状态。重新经同一个 Winit surface 建立短 frame，
            // 只恢复 current surface 与 GL 状态，不 clear、不 draw，以免覆盖已验证像素。
            let mut submit_frame = stage_error(
                diag,
                "Winit submit rebind renderer.render",
                &stage_diag,
                renderer.render(&mut framebuffer, output_size, Transform::Flipped180),
            )?;
            let rebind_target_note = stage_error(
                diag,
                "Winit submit rebind target verification",
                &stage_diag,
                configure_winit_default_draw_buffer(&mut submit_frame, target_surface)
                    .map_err(std::io::Error::other),
            )?;
            if diag {
                stage_diag.draw_target = Some(format!(
                    "{}; pre-submit {rebind_target_note}",
                    stage_diag.draw_target.as_deref().unwrap_or("unavailable"),
                ));
            }

            // 该采样沿用绘制后的同一 GL 坐标，并必须与已记录的 back_px 完全一致；任何
            // 跳过、GL 读/恢复错误、非默认 read framebuffer 或像素变化都失败关闭。
            let pre_submit_sample_error = if diag {
                let note = diag_quadrant_pixel_note(
                    &mut submit_frame,
                    "pre_submit_px",
                    output_size.w,
                    output_size.h,
                    expected_width,
                    expected_height,
                );
                stage_diag.pre_submit_px = Some(note.clone());
                let sample = note.strip_prefix("pre_submit_px=");
                let sample_succeeded = sample.is_some_and(|sample| {
                    sample.starts_with('[')
                        && sample.contains("] err(read)=0 err(restore)=0 rb=0 pk=")
                });
                let drawn_pixel = stage_diag
                    .back_px
                    .as_deref()
                    .and_then(|drawn| drawn.strip_prefix("back_px="))
                    .and_then(|drawn| drawn.split_once(']'))
                    .map(|(pixel, _)| pixel);
                let before_submit_pixel = sample
                    .and_then(|sample| sample.split_once(']'))
                    .map(|(pixel, _)| pixel);
                let same_as_drawn_pixel = drawn_pixel
                    .zip(before_submit_pixel)
                    .is_some_and(|(drawn, before_submit)| drawn == before_submit);
                (!sample_succeeded || !same_as_drawn_pixel).then(|| {
                    std::io::Error::other(format!(
                        "重绑后的 pre_submit_px 采样失败或与 back_px 不一致: {note}"
                    ))
                })
            } else {
                None
            };
            let _ = stage_error(
                diag,
                "Winit submit rebind frame.finish",
                &stage_diag,
                submit_frame.finish(),
            )?;
            if let Some(error) = pre_submit_sample_error {
                stage_error(diag, "Winit submit pre_submit_px", &stage_diag, Err(error))?;
            }
            target_rgb
        };
        stage_error(
            diag,
            "Winit backend.submit / eglSwapBuffers",
            &stage_diag,
            self.backend.submit(Some(&[output_damage])),
        )?;
        // 提交后再泵一次，让宿主收到本次提交相关的窗口事件；不改变任何资源归属，
        // 也不影响 socket/`.lock`/SHM 的清理责任。
        let _ = self.event_loop.dispatch_new_events(|_| {});
        self.retained_source_buffers.push(resource.buffer.clone());

        Ok(NestedWinitShmPresentReport {
            client_buffer_imported: true,
            client_texture_drawn: true,
            client_texture_read_back: true,
            client_texture_readback_rgb: final_rgb,
            backbuffer_submitted: true,
            output_damage_submitted: true,
            source_buffer_retained: true,
            client_frame_done_sent: false,
        })
    }

    /// 返回 event loop 是否仍由此 owner 持有，仅供生命周期测试。
    ///
    /// 这不是 event dispatch 或 input 支持：present 路径内的有界泵送由
    /// `pump_winit_events_until_configured` 负责，本函数调用本身不会消费 Winit event。
    pub(crate) const fn event_loop_is_owned(&self) -> bool {
        let _ = &self.event_loop;
        true
    }
}

#[cfg(test)]
mod tests {
    use smithay::utils::Rectangle;

    use super::{
        NestedWinitOutputOwner, normalize_xrgb8888_readback_rgb, readback_relation,
        sub_rect_pixels, target_frame_summary, target_mismatch_diagnostic, target_readback_region,
    };

    /// Red：GLES mapping 标记 flipped 时必须恢复为 client 的逐行顺序，并忽略 X 通道；
    /// 这样受控 runner 比对的是实际四个 RGB 像素而不是 renderer/report 布尔值。
    #[test]
    fn xrgb_readback_normalizes_vertical_flip_and_preserves_rgb_pixels() {
        // flipped mapping 顺序为 top row（红、绿）后 bottom row（蓝、白），每像素 B/G/R/X。
        let mapped = [
            0x00, 0x00, 0xFF, 0x7C, 0x00, 0xFF, 0x00, 0x7D, 0xFF, 0x00, 0x00, 0x7A, 0xFF, 0xFF,
            0xFF, 0x7B,
        ];

        assert_eq!(
            normalize_xrgb8888_readback_rgb(&mapped, 2, 2, true),
            Some(vec![
                [0xFF, 0x00, 0x00],
                [0x00, 0xFF, 0x00],
                [0x00, 0x00, 0xFF],
                [0xFF, 0xFF, 0xFF]
            ])
        );
        assert_eq!(
            normalize_xrgb8888_readback_rgb(&mapped, 2, 2, false),
            Some(vec![
                [0x00, 0x00, 0xFF],
                [0xFF, 0xFF, 0xFF],
                [0xFF, 0x00, 0x00],
                [0x00, 0xFF, 0x00]
            ])
        );
    }

    /// Red：长度不等于 width×height×4 的 mapping 必须拒绝，不能截断后仍声称像素已验证。
    #[test]
    fn xrgb_readback_rejects_truncated_or_oversized_mapping() {
        assert_eq!(normalize_xrgb8888_readback_rgb(&[0; 15], 2, 2, false), None);
        assert_eq!(normalize_xrgb8888_readback_rgb(&[0; 17], 2, 2, false), None);
    }

    /// 目标 framebuffer 回读区域契约：元素位于 logical 左上角时，GL（左下原点）区域
    /// 必须是 `y ∈ [output_h - element_h, output_h)`；元素大于目标时必须拒绝。
    /// Red 依据：旧实现没有该函数，调用点无法编译。
    #[test]
    fn target_readback_region_maps_top_left_element_to_gl_coordinates() {
        assert_eq!(
            target_readback_region((1280, 800), (256, 256)),
            Some(Rectangle::from_loc_and_size((0, 544), (256, 256))),
            "logical 左上 256×256 必须映射到 GL y∈[544,800)"
        );
        assert_eq!(
            target_readback_region((256, 256), (256, 256)),
            Some(Rectangle::from_loc_and_size((0, 0), (256, 256))),
            "元素铺满目标时区域即整个目标"
        );
        assert_eq!(
            target_readback_region((100, 100), (256, 256)),
            None,
            "元素大于目标必须拒绝，不能读回越界区域"
        );
        assert_eq!(
            target_readback_region((256, 256), (0, 256)),
            None,
            "零尺寸元素必须拒绝"
        );
    }

    /// 回读关系诊断契约：长度、首个差异坐标、行/列/180 度关系必须可区分。
    /// Red 依据：旧实现没有该函数，present 失败只能倾倒整幅像素。
    #[test]
    fn readback_relation_classifies_length_diff_and_transforms() {
        let expected = vec![
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0xFF, 0xFF],
        ];

        let row_flipped = vec![expected[2], expected[3], expected[0], expected[1]];
        let message = readback_relation(&row_flipped, &expected, 2);
        assert!(
            message.contains("行序上下翻转"),
            "必须识别行序翻转，实际: {message}"
        );

        let column_flipped = vec![expected[1], expected[0], expected[3], expected[2]];
        let message = readback_relation(&column_flipped, &expected, 2);
        assert!(
            message.contains("列序左右翻转"),
            "必须识别列序翻转，实际: {message}"
        );

        let rotated = vec![expected[3], expected[2], expected[1], expected[0]];
        let message = readback_relation(&rotated, &expected, 2);
        assert!(
            message.contains("180"),
            "必须识别 180 度旋转，实际: {message}"
        );

        let message = readback_relation(&expected[..2], &expected, 2);
        assert!(
            message.contains("长度"),
            "必须识别长度不符，实际: {message}"
        );

        let mut diff = expected.clone();
        diff[3] = [0x01, 0x02, 0x03];
        let message = readback_relation(&diff, &expected, 2);
        assert!(
            message.contains("index=3") && message.contains("x=1") && message.contains("y=1"),
            "必须给出首个差异 index 与坐标，实际: {message}"
        );
    }

    /// Red：失败诊断的 `actual` 必须是目标 framebuffer 回读、`expected` 必须是 source
    /// texture 回读，与门禁比较 `target_rgb != readback_rgb` 的两侧语义一致。
    ///
    /// 准备：source 是 2×2 纯红图案，目标区域分别是全黑与含非黑像素的两种回读；执行：
    /// 按（target=actual, source=expected）生成诊断；断言：黑必须记为 actual 全黑、首个
    /// 差异两侧颜色不得互换。Red 依据：旧调用点把两侧反接，黑 target 会被写成
    /// “actual 是红、expected 是黑”，把“目标没有图案”误报成“source 异常”。
    #[test]
    fn target_mismatch_diagnostic_labels_target_as_actual() {
        let source = vec![[0xFF, 0x00, 0x00]; 4];

        // 情形一：目标区域全黑（“target 根本没有图案”的直接观测）。
        let black_target = vec![[0x00, 0x00, 0x00]; 4];
        let message = target_mismatch_diagnostic(&black_target, &source, 2);
        assert!(
            message.contains("actual 全黑"),
            "全黑的是目标，必须记为 actual 全黑，实际: {message}"
        );
        assert!(
            message.contains("actual=[0, 0, 0]") && message.contains("expected=[255, 0, 0]"),
            "首个差异必须 actual=目标、expected=source，实际: {message}"
        );

        // 情形二：目标有内容但与 source 不同，用于确认两侧没有被静默互换。
        let colored_target = vec![[0x01, 0x02, 0x03]; 4];
        let message = target_mismatch_diagnostic(&colored_target, &source, 2);
        assert!(
            message.contains("actual=[1, 2, 3]") && message.contains("expected=[255, 0, 0]"),
            "actual 必须是目标像素、expected 必须是 source 像素，实际: {message}"
        );
    }

    /// 全帧摘要契约：区域回读为黑时必须区分“图案画到别处”“target 根本没有图案”和
    /// “位置正确但方向反了”，只输出计数/包围盒与关系结论，不倾倒像素。
    /// Red 依据：旧实现没有该函数，present 失败只有区域级诊断，无法定位图案。
    #[test]
    fn target_frame_summary_separates_wrong_location_from_missing_pattern() {
        // 准备：2×2 四象限图案（红/绿/蓝/白），4×4 目标，元素位于 logical 左上 →
        // GL 区域 (0,2,2,2)，换算成 client top-down 行 [0,2)。
        let source = vec![
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0xFF, 0xFF],
        ];
        let region = Rectangle::from_loc_and_size((0, 2), (2, 2));
        let black = vec![[0x00, 0x00, 0x00]; 16];

        // 执行一：整帧全黑 → 必须给出“图案未出现在任何位置”。
        let summary = target_frame_summary(&black, 4, 4, region, &source, 2);
        assert!(
            summary.contains("非黑像素 0/16")
                && summary.contains("未出现在目标 framebuffer 的任何位置"),
            "全黑 target 必须判定为没有任何图案，实际: {summary}"
        );

        // 执行二：图案落在预期区域 → 必须判定在区域内且方向一致。
        let mut in_place = black.clone();
        in_place[0] = source[0];
        in_place[1] = source[1];
        in_place[4] = source[2];
        in_place[5] = source[3];
        let summary = target_frame_summary(&in_place, 4, 4, region, &source, 2);
        assert!(
            summary.contains("图案落在预期区域内") && summary.contains("逐像素完全一致"),
            "预期区域内的图案必须判定为一致，实际: {summary}"
        );

        // 执行三：图案位于预期区域之外（top-down 行 2..3、列 2..3）。
        let mut elsewhere = black.clone();
        elsewhere[10] = source[0];
        elsewhere[11] = source[1];
        elsewhere[14] = source[2];
        elsewhere[15] = source[3];
        let summary = target_frame_summary(&elsewhere, 4, 4, region, &source, 2);
        assert!(
            summary.contains("图案画到预期区域之外")
                && summary.contains("x=[2,3]")
                && summary.contains("y=[2,3]"),
            "错位图案必须给出实际包围盒，实际: {summary}"
        );

        // 执行四：位置正确但上下翻转 → 必须识别方向，而不是只报“不一致”。
        let mut row_flipped = black.clone();
        row_flipped[0] = source[2];
        row_flipped[1] = source[3];
        row_flipped[4] = source[0];
        row_flipped[5] = source[1];
        let summary = target_frame_summary(&row_flipped, 4, 4, region, &source, 2);
        assert!(
            summary.contains("图案落在预期区域内") && summary.contains("行序上下翻转"),
            "方向翻转必须在实际位置被识别，实际: {summary}"
        );
    }

    /// Red：与 source 图案无关的单个非黑像素，绝不能被摘要写成“图案画到别处”。
    /// 准备：4×4 全黑目标，只在预期区域之外（top-down 行 3、列 3）放一个与 2×2 四象限
    /// 图案无关的非黑像素；执行：全帧摘要；断言：位置结论只能是“非黑像素包围盒位于
    /// 区域之外”，尺寸诊断保留；Red 依据：旧实现仅凭非黑像素包围盒位置就断言“图案画到
    /// 预期区域之外”，任意杂色都会触发该结论；cleanup：纯数据，无资源。
    #[test]
    fn target_frame_summary_does_not_mistake_unrelated_pixel_for_pattern() {
        // 准备：source 是红/绿/蓝/白四象限图案，杂色像素与其无任何关系。
        let source = vec![
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0xFF, 0xFF],
        ];
        let region = Rectangle::from_loc_and_size((0, 2), (2, 2));
        let mut frame = vec![[0x00, 0x00, 0x00]; 16];
        frame[15] = [0x12, 0x34, 0x56];

        // 执行：全帧摘要必须基于像素比较，而不是仅凭包围盒位置下结论。
        let summary = target_frame_summary(&frame, 4, 4, region, &source, 2);

        // 断言一：内容未与 source 比对通过时，禁止出现任何“图案”位置结论。
        assert!(
            !summary.contains("图案画到预期区域之外") && !summary.contains("图案落在预期区域内"),
            "与 source 无关的杂色不得被写成图案，实际: {summary}"
        );
        // 断言二：仍须如实报告非黑像素包围盒在预期区域之外。
        assert!(
            summary.contains("非黑像素包围盒位于预期区域之外"),
            "必须报告非黑像素包围盒的位置，实际: {summary}"
        );
        // 断言三：尺寸诊断不能因为措辞调整而丢失。
        assert!(
            summary.contains("包围盒尺寸 1x1 与图案 2x2 不符"),
            "包围盒与 source 尺寸不符的诊断必须保留，实际: {summary}"
        );
    }

    /// 全帧摘要的长度与子区域读取必须拒绝不匹配输入，不能裁剪后仍给出结论。
    #[test]
    fn target_frame_summary_and_sub_rect_reject_bad_geometry() {
        let summary = target_frame_summary(
            &[[0x00; 3]; 15],
            4,
            4,
            Rectangle::from_loc_and_size((0, 2), (2, 2)),
            &[[0xFF; 3]; 4],
            2,
        );
        assert!(
            summary.contains("全帧摘要不可用"),
            "长度不匹配必须拒绝，实际: {summary}"
        );

        let frame = vec![[0x00; 3]; 16];
        assert_eq!(
            sub_rect_pixels(&frame, 4, 4, (2, 2, 3, 3)),
            None,
            "越界子区域必须拒绝"
        );
        assert_eq!(
            sub_rect_pixels(&frame, 4, 4, (0, 0, 0, 2)),
            None,
            "零尺寸子区域必须拒绝"
        );
        assert_eq!(
            sub_rect_pixels(&[[0x00; 3]; 15], 4, 4, (0, 0, 2, 2)),
            None,
            "帧长度不匹配必须拒绝"
        );
        assert_eq!(
            sub_rect_pixels(&frame, 4, 4, (1, 1, 2, 2)).map(|pixels| pixels.len()),
            Some(4),
            "合法子区域必须返回完整像素"
        );
    }

    /// R2 Green：创建真实 Winit/EGL/GLES target，并 bind/clear/submit 一帧固定背景。
    ///
    /// 准备：宿主桌面可创建 Winit target；执行：唯一 owner 构造并提交首帧；断言仅覆盖
    /// 这三步；cleanup：scope 末尾 drop owner，不留下 socket、client、buffer 或 Core mutation。
    #[test]
    #[ignore = "Winit 0.30 event loop 必须在进程主线程创建；受控 binary 覆盖真实执行"]
    fn controlled_winit_target_binds_clears_and_submits_one_first_frame() {
        let mut owner = NestedWinitOutputOwner::new()
            .expect("受控桌面环境必须能创建 Winit/EGL/GLES 首帧 target");
        assert!(owner.event_loop_is_owned());

        let report = owner
            .present_first_frame()
            .expect("真实 Winit/EGL/GLES target 必须能提交受控首帧");

        assert!(report.target_created);
        assert!(report.target_size.0 > 0);
        assert!(report.target_size.1 > 0);
        assert!(report.renderer_bound);
        assert!(report.first_frame_cleared);
        assert!(report.backbuffer_submitted);
        assert!(!report.client_buffer_imported);
        assert!(!report.client_frame_done_sent);
    }
}
