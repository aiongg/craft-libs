//! Stories rendered to images offscreen: craft-ui-render-epaint on a headless window, read back as pixels. The wgpu
//! adapter prefers a software one, so it runs without a display or a GPU.
//!
//! A [`Harness`](crate::contract::rt::harness::Harness) built with [`renderer`] shows the window; [`image_of`] reads its
//! last frame, and [`Harness::into_renderer`](crate::contract::rt::harness::Harness::into_renderer) gives the renderer
//! back for the next story, so one device serves a whole run.

use crate::contract::rt::harness::Harness;
use craft_ui_render_epaint::{EpaintOptions, EpaintRenderer};

pub use craft_ui_render_epaint::EpaintRenderer as Renderer;
pub use image::RgbaImage;

/// A renderer on the preferred adapter, with the options of egui's snapshot renderer (no dithering, predictable
/// texture filtering) and the suite's default faces.
///
/// # Panics
/// When wgpu finds no adapter.
pub fn renderer() -> EpaintRenderer {
    #[allow(clippy::expect_used, reason = "a snapshot cannot be taken without an adapter")]
    EpaintRenderer::create_blocking(EpaintOptions::predictable(), crate::fonts::deflated_default_definitions(), None)
        .expect("a wgpu adapter (a software one will do)")
}

/// The window's last frame as an image, in physical pixels, over a transparent ground.
///
/// # Panics
/// When the renderer cannot read the window back (see [`Harness::render`]).
pub fn image_of<S>(harness: &mut Harness<S, EpaintRenderer>) -> Result<RgbaImage, String> {
    let frame = harness.render();
    RgbaImage::from_raw(frame.size.width, frame.size.height, frame.pixels).ok_or_else(|| "the read-back has the wrong length".to_owned())
}
