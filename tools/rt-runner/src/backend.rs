//! The renderers a run can use.

use craft_ui_contract::fonts::{I18nScripts, UiFonts};
use craft_ui_render_epaint::{EpaintOptions, EpaintRenderer};

/// epaint's tessellator on wgpu, with craft's default faces, in the options that make runs repeat.
pub fn epaint() -> EpaintRenderer {
    let fonts = UiFonts::new(&I18nScripts).stack().flat_definitions();
    EpaintRenderer::create_blocking(EpaintOptions::predictable(), fonts, None).expect("the epaint renderer needs a wgpu adapter (lavapipe will do)")
}

/// Skia's CPU raster.
#[cfg(feature = "skia")]
pub fn skia() -> craft_ui_render_skia::SkiaRenderer {
    craft_ui_render_skia::SkiaRenderer::new(Default::default())
}

/// Skia on the GPU (Vulkan on this box).
#[cfg(feature = "skia-gpu")]
pub fn skia_gpu() -> craft_ui_render_skia::SkiaRenderer {
    craft_ui_render_skia::SkiaRenderer::new_gpu(Default::default(), None).expect("a Vulkan or Metal device")
}
