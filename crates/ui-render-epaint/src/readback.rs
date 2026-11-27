//! Reading a window's canvas back: copy the rect into a buffer, map it asynchronously, and let
//! the caller poll. The web cannot block, so nothing here waits.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use craft_ui_rt::{ReadBack, RectU, RgbaImage, SizeU};
use egui_wgpu::{RenderState, wgpu};

const PENDING: u8 = 0;
const MAPPED: u8 = 1;
const FAILED: u8 = 2;

pub(crate) struct PendingRead {
    buffer: wgpu::Buffer,
    rect: RectU,
    row: u32,
    bgra: bool,
    state: Arc<AtomicU8>,
}

/// Starts reading `rect` (the whole canvas when `None`). `None` when the rect is empty or outside
/// the canvas.
pub(crate) fn start(gpu: &RenderState, canvas: &wgpu::Texture, size: SizeU, rect: Option<RectU>) -> Option<PendingRead> {
    let rect = rect.unwrap_or(RectU::of_size(size));
    if rect.is_empty() || !rect.within(RectU::of_size(size)) {
        return None;
    }
    let (device, queue) = (&gpu.device, &gpu.queue);
    let row = (rect.width() * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("craft-ui-render-epaint read-back"),
        size: u64::from(row) * u64::from(rect.height()),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: canvas,
            mip_level: 0,
            origin: wgpu::Origin3d { x: rect.x0, y: rect.y0, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(rect.height()) },
        },
        wgpu::Extent3d { width: rect.width(), height: rect.height(), depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));
    let state = Arc::new(AtomicU8::new(PENDING));
    let flag = state.clone();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        flag.store(if result.is_ok() { MAPPED } else { FAILED }, Ordering::Release);
    });
    let bgra = matches!(gpu.target_format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
    Some(PendingRead { buffer, rect, row, bgra, state })
}

/// The read's state after giving the device a chance to finish it.
pub(crate) fn poll(gpu: &RenderState, read: &PendingRead) -> ReadBack {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = gpu.device.poll(wgpu::PollType::Poll) {
        log::debug!("read-back poll: {e}");
    }
    #[cfg(target_arch = "wasm32")]
    let _ = gpu;
    match read.state.load(Ordering::Acquire) {
        PENDING => ReadBack::Pending,
        FAILED => ReadBack::Failed,
        _ => {
            let size = SizeU::new(read.rect.width(), read.rect.height());
            let Ok(mapped) = read.buffer.slice(..).get_mapped_range() else {
                return ReadBack::Failed;
            };
            let mut image = RgbaImage::new(size);
            let (row, width) = (read.row as usize, size.width as usize);
            for y in 0..size.height as usize {
                let src = &mapped[y * row..][..width * 4];
                let dst = &mut image.pixels[y * width * 4..][..width * 4];
                for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
                    let [r, g, b] = if read.bgra { [s[2], s[1], s[0]] } else { [s[0], s[1], s[2]] };
                    let a = s[3];
                    // The canvas holds premultiplied colour; the image is straight alpha.
                    let straight = |c: u8| if a == 0 || a == 255 { c } else { (u32::from(c) * 255 / u32::from(a)).min(255) as u8 };
                    d.copy_from_slice(&[straight(r), straight(g), straight(b), a]);
                }
            }
            drop(mapped);
            read.buffer.unmap();
            ReadBack::Ready(image)
        }
    }
}
