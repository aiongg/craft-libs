//! Ganesh on the GPU for rt-render-skia: one backend per platform, behind one small interface.
//!
//! - `Gpu::new` makes the device and Skia's `DirectContext`; `Gpu::surface` makes the kept
//!   offscreen render target the renderer draws into, as it draws into the CPU raster surface.
//! - `Gpu::submit` flushes the frame's GPU work and submits it. Offscreen, with timing on, it
//!   brackets Skia's submission with GPU timestamps of its own and waits for them; in a window it
//!   copies the canvas onto the next drawable or swapchain image and presents that.
//!
//! The bracket is the GPU time of everything Skia submitted for the frame (uploads, clears, draws)
//! plus two tiny command buffers. Skia's own command buffers are not reachable through skia-safe,
//! so timestamps go into command buffers submitted on the same queue just before and just after
//! Skia's.
//!
//! Backends: Metal on macOS, Vulkan on Linux, Windows and Android. On other targets the crate is
//! empty.

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
use std::any::Any;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
use std::sync::Arc;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
use craft_ui_rt::SizeU;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
use skia_safe::{AlphaType, BlendMode, ColorType, ImageInfo, Paint, SamplingOptions, Surface, gpu};

#[cfg(target_os = "macos")]
mod metal;
#[cfg(target_os = "macos")]
pub use metal::{Gpu, Presenter};

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "android"))]
mod vulkan;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "android"))]
pub use vulkan::{Gpu, Presenter};

/// What a window's raw handles borrow from: the platform keeps the window alive while a presenter holds this,
/// and compares windows by it.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
pub type Keep = Arc<dyn Any + Send + Sync>;

/// A window to present to: its raw handles and what keeps them valid.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
pub struct WindowRef {
    pub window: RawWindowHandle,
    pub display: RawDisplayHandle,
    pub keep: Keep,
}

/// CPU and GPU time of one frame's submission.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
pub struct Submitted {
    /// CPU time of the flush (command recording) and the submit, plus the present in a window.
    /// The bracket's own submissions and the wait for the timestamps are not in it.
    pub submit_ms: f64,
    pub gpu_ms: Option<f64>,
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
/// The kept canvas: a premultiplied RGBA render target the context owns.
fn render_target(context: &mut gpu::DirectContext, size: SizeU) -> Option<Surface> {
    let info = ImageInfo::new((size.width as i32, size.height as i32), ColorType::RGBA8888, AlphaType::Premul, None);
    gpu::surfaces::render_target(context, gpu::Budgeted::Yes, &info, None, gpu::SurfaceOrigin::TopLeft, None, false, None)
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android"))]
/// Copies the canvas onto a drawable of the same size, pixel for pixel.
fn blit(canvas: &mut Surface, onto: &mut Surface) {
    let mut paint = Paint::default();
    paint.set_blend_mode(BlendMode::Src);
    canvas.draw(onto.canvas(), (0, 0), SamplingOptions::default(), Some(&paint));
}
