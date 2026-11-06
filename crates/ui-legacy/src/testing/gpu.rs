//! A wgpu renderer for kittest harnesses on the process's one device.

use std::sync::{Arc, LazyLock};

use egui::mutex::RwLock;
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use egui_wgpu::{RenderState, Renderer, RendererOptions};

/// A wgpu test renderer of its own, on a device shared by the whole process and never dropped.
///
/// Tests run on parallel threads, and Vulkan-Loader before 1.4.350 crashes when one thread creates
/// or destroys an instance or device while another names an object (KhronosGroup/Vulkan-Loader#1866).
/// Pass it to [`HarnessBuilder::renderer`](egui_kittest::HarnessBuilder::renderer) in place of
/// kittest's default renderer, which creates a device per harness.
pub fn wgpu_renderer() -> WgpuTestRenderer {
    static DEVICE: LazyLock<RenderState> = LazyLock::new(|| create_render_state(default_wgpu_setup(), RendererOptions::PREDICTABLE));
    // Every egui context uses the same managed texture ids, so each harness gets its own renderer.
    let renderer = Renderer::new(&DEVICE.device, DEVICE.target_format, RendererOptions::PREDICTABLE);
    WgpuTestRenderer::from_render_state(RenderState { renderer: Arc::new(RwLock::new(renderer)), ..DEVICE.clone() })
}
