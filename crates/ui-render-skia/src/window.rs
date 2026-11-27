//! The platform's window as the renderer sees it, and the CPU backend's presenter.

use std::num::NonZeroU32;
use std::sync::Arc;

use craft_ui_rt::{RectU, SizeU};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use skia_safe::{ColorType, Surface};

/// What a platform's window type provides: raw handles, kept valid while the renderer holds the
/// `Arc`.
pub trait WindowHandles: HasWindowHandle + HasDisplayHandle + Send + Sync + std::any::Any {}

impl<T: HasWindowHandle + HasDisplayHandle + Send + Sync + std::any::Any> WindowHandles for T {}

/// The object a native platform puts in `SurfaceHandle::Native(Rc<dyn Any>)` for this renderer
/// (`Rc::new(NativeWindow::new(window))`).
#[derive(Clone)]
pub struct NativeWindow(pub Arc<dyn WindowHandles>);

impl NativeWindow {
    pub fn new<W: WindowHandles>(window: Arc<W>) -> NativeWindow {
        NativeWindow(window)
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn window_ref(&self) -> Option<craft_ui_render_skia_gpu::WindowRef> {
        let keep: craft_ui_render_skia_gpu::Keep = self.0.clone();
        Some(craft_ui_render_skia_gpu::WindowRef {
            window: self.0.window_handle().ok()?.as_raw(),
            display: self.0.display_handle().ok()?.as_raw(),
            keep,
        })
    }
}

/// The CPU backend's window: softbuffer takes the canvas each frame.
pub(crate) struct SoftPresenter {
    pub window: NativeWindow,
    surface: softbuffer::Surface<Arc<dyn WindowHandles>, Arc<dyn WindowHandles>>,
    // The surface borrows the connection the context holds, so it stays alive with it.
    _context: softbuffer::Context<Arc<dyn WindowHandles>>,
}

impl SoftPresenter {
    pub fn new(window: NativeWindow) -> Option<SoftPresenter> {
        let context = softbuffer::Context::new(window.0.clone()).map_err(|e| log::error!("softbuffer context: {e}")).ok()?;
        let surface = softbuffer::Surface::new(&context, window.0.clone()).map_err(|e| log::error!("softbuffer surface: {e}")).ok()?;
        Some(SoftPresenter { window, surface, _context: context })
    }

    pub fn resize(&mut self, size: SizeU) -> bool {
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return false;
        };
        self.surface.resize(w, h).map_err(|e| log::error!("softbuffer resize: {e}")).is_ok()
    }

    /// Copies `canvas` into the window's buffer and presents it. When the buffer still holds the
    /// previous frame (age 1) only `damage` is converted.
    pub fn present(&mut self, canvas: &mut Surface, size: SizeU, damage: &[RectU]) {
        let Some(pixmap) = canvas.peek_pixels() else { return };
        let (Some(src), row) = (pixmap.bytes(), pixmap.row_bytes()) else { return };
        let rgba = pixmap.color_type() == ColorType::RGBA8888;
        let mut buffer = match self.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                log::debug!("frame skipped: {e}");
                return;
            }
        };
        let width = size.width as usize;
        let full = [RectU::of_size(size)];
        let partial = buffer.age() == 1;
        for r in if partial { damage } else { &full[..] } {
            to_xrgb(src, row, rgba, &mut buffer, width, *r);
        }
        let result = if partial {
            let rects: Vec<softbuffer::Rect> = damage
                .iter()
                .filter_map(|r| Some(softbuffer::Rect { x: r.x0, y: r.y0, width: NonZeroU32::new(r.width())?, height: NonZeroU32::new(r.height())? }))
                .collect();
            buffer.present_with_damage(&rects)
        } else {
            buffer.present()
        };
        if let Err(e) = result {
            log::debug!("present failed: {e}");
        }
    }
}

/// Converts `rect` of a premultiplied 8-bit canvas (`src`, `row` bytes per row; BGRA, or RGBA when
/// `rgba`) into softbuffer's 0RGB words (`dst`, `width` words per row).
pub fn to_xrgb(src: &[u8], row: usize, rgba: bool, dst: &mut [u32], width: usize, rect: RectU) {
    for y in rect.y0 as usize..rect.y1 as usize {
        let line = &src[y * row + rect.x0 as usize * 4..y * row + rect.x1 as usize * 4];
        let out = &mut dst[y * width + rect.x0 as usize..y * width + rect.x1 as usize];
        for (d, s) in out.iter_mut().zip(line.chunks_exact(4)) {
            let (r, g, b) = if rgba { (s[0], s[1], s[2]) } else { (s[2], s[1], s[0]) };
            *d = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
        }
    }
}
