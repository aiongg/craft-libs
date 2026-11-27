//! Ganesh on Metal (macOS): the system default device and a command queue Skia submits on.
//!
//! A window gets a `CAMetalLayer` on its view (raw-window-metal keeps the layer's bounds and
//! contents scale in step with the view). Presenting copies the kept canvas onto the next drawable
//! with Skia, submits, and presents the drawable from a command buffer committed after Skia's, as
//! skia-safe's Metal example does.
//!
//! GPU time: Metal runs a queue's command buffers in commit order, so two small command buffers
//! committed just before and just after Skia's submission bracket it; the time is the second's
//! `GPUEndTime` minus the first's `GPUStartTime`. Each holds a 4-byte buffer fill, so neither is
//! an empty command buffer the driver might skip.

use craft_ui_rt::SizeU;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CGSize;
use objc2_foundation::NSRange;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLBuffer, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable,
    MTLPixelFormat, MTLResourceOptions,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use raw_window_handle::RawWindowHandle;
use raw_window_metal::Layer;
use skia_safe::gpu::{self, DirectContext, FlushInfo, SurfaceOrigin, SyncCpu, mtl};
use skia_safe::{ColorType, Surface};
use web_time::Instant;

use super::{Keep, Submitted, WindowRef, blit, render_target};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

pub struct Gpu {
    context: DirectContext,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    /// What the timestamp bracket's command buffers fill.
    mark: Retained<ProtocolObject<dyn MTLBuffer>>,
}

impl Gpu {
    pub fn new(_window: Option<&WindowRef>) -> Result<Gpu, String> {
        let device = MTLCreateSystemDefaultDevice().ok_or("no Metal device")?;
        let queue = device.newCommandQueue().ok_or("no Metal command queue")?;
        let mark = device.newBufferWithLength_options(4, MTLResourceOptions::StorageModePrivate).ok_or("no Metal buffer")?;
        let backend = unsafe { mtl::BackendContext::new(Retained::as_ptr(&device) as mtl::Handle, Retained::as_ptr(&queue) as mtl::Handle) };
        let context = gpu::direct_contexts::make_metal(&backend, None).ok_or("Skia could not make a Metal context")?;
        log::info!("skia-gpu: Metal device {}", device.name());
        Ok(Gpu { context, queue, device, mark })
    }

    pub fn surface(&mut self, size: SizeU) -> Option<Surface> {
        render_target(&mut self.context, size)
    }

    pub fn presenter(&mut self, window: WindowRef) -> Option<Presenter> {
        let RawWindowHandle::AppKit(handle) = window.window else {
            log::error!("skia-gpu: the window is not an AppKit window");
            return None;
        };
        let layer = unsafe { Layer::from_ns_view(handle.ns_view) };
        let presenter = Presenter { window: window.keep, layer };
        let ca = presenter.layer();
        ca.setDevice(Some(&*self.device));
        ca.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        // Skia draws into the drawable, which needs it as a regular texture.
        ca.setFramebufferOnly(false);
        Some(presenter)
    }

    /// A committed command buffer with one tiny blit in it.
    fn mark(&self) -> Option<Retained<ProtocolObject<dyn MTLCommandBuffer>>> {
        let buffer = self.queue.commandBuffer()?;
        let blit = buffer.blitCommandEncoder()?;
        blit.fillBuffer_range_value(&self.mark, NSRange::new(0, 4), 0);
        blit.endEncoding();
        buffer.commit();
        Some(buffer)
    }

    pub fn submit(&mut self, canvas: &mut Surface, presenter: Option<&mut Presenter>, timing: bool) -> Submitted {
        // Metal hands out autoreleased objects (command buffers, drawables); the bench loop has
        // no run loop to drain them.
        autoreleasepool(|_| {
            let t = Instant::now();
            if let Some(p) = presenter {
                p.present(&mut self.context, &self.queue, canvas);
                return Submitted { submit_ms: ms(t), gpu_ms: None };
            }
            if let Err(e) = self.context.flush(&FlushInfo::default()) {
                log::error!("skia flush failed: {e:?}");
            }
            let mut submit_ms = ms(t);
            let before = if timing { self.mark() } else { None };
            let t = Instant::now();
            if !self.context.submit(SyncCpu::No) {
                log::error!("skia submit failed");
            }
            submit_ms += ms(t);
            let gpu_ms = before.and_then(|before| {
                let after = self.mark()?;
                after.waitUntilCompleted();
                Some((after.GPUEndTime() - before.GPUStartTime()) * 1000.0)
            });
            Submitted { submit_ms, gpu_ms }
        })
    }
}

/// A window's Metal layer.
pub struct Presenter {
    window: Keep,
    layer: Layer,
}

impl Presenter {
    /// What the window handles borrow from; the window's identity.
    pub fn keep(&self) -> &Keep {
        &self.window
    }

    fn layer(&self) -> &CAMetalLayer {
        // SAFETY: `Layer` wraps a `CAMetalLayer` and lives as long as `self`.
        unsafe { self.layer.as_ptr().cast().as_ref() }
    }

    pub fn resize(&mut self, size: SizeU) {
        self.layer().setDrawableSize(CGSize::new(f64::from(size.width), f64::from(size.height)));
    }

    fn present(&mut self, context: &mut DirectContext, queue: &ProtocolObject<dyn MTLCommandQueue>, canvas: &mut Surface) {
        let ca = self.layer();
        let Some(drawable) = ca.nextDrawable() else { return };
        let size = ca.drawableSize();
        let texture = drawable.texture();
        let info = unsafe { mtl::TextureInfo::new(Retained::as_ptr(&texture) as mtl::Handle) };
        let target = gpu::backend_render_targets::make_mtl((size.width as i32, size.height as i32), &info);
        let Some(mut onto) = gpu::surfaces::wrap_backend_render_target(context, &target, SurfaceOrigin::TopLeft, ColorType::BGRA8888, None, None)
        else {
            log::error!("skia could not wrap the drawable");
            return;
        };
        blit(canvas, &mut onto);
        if let Err(e) = context.flush_and_submit() {
            log::error!("skia flush failed, frame dropped: {e:?}");
            return;
        }
        drop(onto);
        let Some(buffer) = queue.commandBuffer() else { return };
        let drawable: Retained<ProtocolObject<dyn MTLDrawable>> = (&drawable).into();
        buffer.presentDrawable(&drawable);
        buffer.commit();
    }
}
