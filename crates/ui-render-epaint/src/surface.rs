//! App-drawn surfaces (a document canvas) inside a display list. The app registers a
//! [`SurfaceHook`] for a [`SurfaceKey`]; where the list has `Item::Surface { key, rect }` the
//! renderer calls the hook's `prepare` and `paint` inside the frame's render pass on the
//! renderer's own wgpu device, so the app's textures and buffers are the ones it draws with.

use std::sync::Arc;

use craft_ui_rt::SurfaceKey;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};
use epaint::PaintCallbackInfo;

/// Where and how the surface is drawn this frame.
pub type SurfaceInfo = PaintCallbackInfo;

/// What `prepare` is told: the surface's rect in window points and the target's scale and size.
#[derive(Clone, Copy, Debug)]
pub struct SurfacePrepare {
    pub rect: emath::Rect,
    pub pixels_per_point: f32,
    pub screen_size_px: [u32; 2],
}

/// What the app does to draw a surface. A frame whose damage is several rects calls the hook once
/// per rect the surface reaches into, each time with that rect as the scissor
/// (`SurfaceInfo::clip_rect`), so `prepare` must be cheap to repeat.
pub trait SurfaceHook: Send + Sync {
    /// Before the pass: upload buffers, render offscreen. Command buffers returned are submitted
    /// before the frame's own.
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        info: &SurfacePrepare,
    ) -> Vec<wgpu::CommandBuffer> {
        let _ = (device, queue, encoder, info);
        Vec::new()
    }

    /// Inside the pass. The viewport is the surface's rect and the scissor the part of it that is
    /// being drawn.
    fn paint(&self, info: &SurfaceInfo, pass: &mut wgpu::RenderPass<'static>);
}

/// The registered hooks.
pub(crate) type Hooks = std::collections::HashMap<SurfaceKey, Arc<dyn SurfaceHook>>;

/// egui-wgpu's callback, forwarding to a hook.
pub(crate) struct Forward(pub Arc<dyn SurfaceHook>, pub emath::Rect);

impl CallbackTrait for Forward {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        _: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let info = SurfacePrepare { rect: self.1, pixels_per_point: screen.pixels_per_point, screen_size_px: screen.size_in_pixels };
        self.0.prepare(device, queue, encoder, &info)
    }

    fn paint(&self, info: PaintCallbackInfo, pass: &mut wgpu::RenderPass<'static>, _: &CallbackResources) {
        self.0.paint(&info, pass);
    }
}
