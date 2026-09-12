//! Linux-only nested Winit/EGL/GLES 首帧输出 owner。
//!
//! 此模块只为 R2 建立真实 target 的唯一 owner，并导入已由 coordinator 原子授权的
//! SHM buffer、回读 texture、绘制与提交。它不决定 ledger/Core identity，不发送 frame
//! done、不处理 input，也不修改 Core；所有结果均仅为 controlled proof，不是 production
//! compositor。

use std::error::Error;

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Color32F, ExportMem, Frame, ImportMemWl, Renderer, Texture, TextureMapping,
            element::{Id, Kind, texture::TextureRenderElement},
            gles::GlesRenderer,
            utils::draw_render_elements,
        },
        winit::{self, WinitEventLoop, WinitGraphicsBackend},
    },
    utils::{Buffer as BufferCoord, Physical, Rectangle, Transform},
};

use super::linux_xdg_shell::PendingShmBufferResource;

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
    /// bind、GLES render/clear/finish 或 EGL submit 任一步失败都会原样返回；drop 此
    /// owner 即完成唯一 cleanup，函数不改动 Core，因此没有 Core 补偿。
    pub(crate) fn present_first_frame(
        &mut self,
    ) -> Result<NestedWinitFirstFrameReport, Box<dyn Error>> {
        let size = self.backend.window_size();
        let damage: Rectangle<i32, Physical> = Rectangle::from_size(size);
        {
            let (renderer, mut framebuffer) = self.backend.bind()?;
            let mut frame = renderer.render(&mut framebuffer, size, Transform::Flipped180)?;
            frame.clear(Color32F::new(0.08, 0.12, 0.2, 1.0), &[damage])?;
            let _ = frame.finish()?;
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

    /// 导入一个已由 coordinator 原子验证过的 SHM buffer，并呈现到真实 Winit/EGL/GLES target。
    ///
    /// 调用方必须在转移 `PendingShmBufferResource` 前确认 source session、adapter
    /// surface/toplevel、admission ledger 和 live Core window；本 owner 不会重新猜测这些
    /// identity。任一 import/render/submit 失败均返回错误且不发送 frame done；resource
    /// 由调用方的失败路径回收，不会留在 display FIFO 中。
    pub(crate) fn present_validated_shm_buffer(
        &mut self,
        resource: &PendingShmBufferResource,
    ) -> Result<NestedWinitShmPresentReport, Box<dyn Error>> {
        let output_size = self.backend.window_size();
        let output_damage: Rectangle<i32, Physical> = Rectangle::from_size(output_size);
        let buffer_damage: Rectangle<i32, BufferCoord> =
            Rectangle::from_size((resource.metadata.width, resource.metadata.height).into());

        let readback_rgb = {
            let (renderer, mut framebuffer) = self.backend.bind()?;
            let texture = renderer.import_shm_buffer(&resource.buffer, None, &[buffer_damage])?;
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
            let mapping = renderer.copy_texture(&texture, buffer_damage, Fourcc::Xrgb8888)?;
            let mapping_flipped = mapping.flipped();
            let mapped = renderer.map_texture(&mapping)?;
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
            let mut frame =
                renderer.render(&mut framebuffer, output_size, Transform::Flipped180)?;
            frame.clear(Color32F::new(0.0, 0.0, 0.0, 1.0), &[output_damage])?;
            draw_render_elements::<GlesRenderer, _, _>(
                &mut frame,
                1.0,
                &[element],
                &[output_damage],
            )?;
            let _ = frame.finish()?;
            readback_rgb
        };
        self.backend.submit(Some(&[output_damage]))?;
        self.retained_source_buffers.push(resource.buffer.clone());

        Ok(NestedWinitShmPresentReport {
            client_buffer_imported: true,
            client_texture_drawn: true,
            client_texture_read_back: true,
            client_texture_readback_rgb: readback_rgb,
            backbuffer_submitted: true,
            output_damage_submitted: true,
            source_buffer_retained: true,
            client_frame_done_sent: false,
        })
    }

    /// 返回 event loop 是否仍由此 owner 持有，仅供生命周期测试。
    ///
    /// 这不是 event dispatch 或 input 支持，调用不会消费 Winit event。
    pub(crate) const fn event_loop_is_owned(&self) -> bool {
        let _ = &self.event_loop;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{NestedWinitOutputOwner, normalize_xrgb8888_readback_rgb};

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
