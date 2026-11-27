//! craft-ui-rt's Skia renderer (skia-safe).
//!
//! Each node part's display list is recorded into a Skia picture in the node's local logical
//! coordinates and kept until the node repaints, its opacity changes or an image or surface it
//! shows changes (the analogue of epaint's kept meshes). A window's pixels live in a canvas that
//! outlives frames. A frame clips the canvas to the damage pixels, clears them, and plays back, in
//! paint order, only the pictures whose ink reaches into the damage, each under its node's clip and
//! `scale * translate(origin)`. Every other pixel stays as the last frame drew it.
//!
//! Two backends share that path:
//! - CPU raster ([`SkiaRenderer::new`]): a raster canvas per window; the window gets it copied into
//!   a softbuffer buffer, only the damage when the buffer holds the previous frame.
//! - GPU ([`SkiaRenderer::new_gpu`], feature `gpu`): a Ganesh render target per window, Metal on
//!   macOS and Vulkan on Linux, Windows and Android, each on a device of its own (crate
//!   `craft-ui-render-skia-gpu`). The window gets the canvas copied onto its drawable or swapchain
//!   image.
//!
//! Windows are handed over as craft-ui-rt's `NativeWindow` inside `SurfaceHandle::Native`.
//!
//! App-drawn surfaces ([`craft_ui_rt::SurfaceKey`]) are CPU tiles: the app registers a
//! [`SurfaceProducer`] and tells the renderer which part changed ([`SkiaRenderer::invalidate_surface`]).
//!
//! Text goes through skparagraph with the font bytes craft ships, with explicit fallback faces
//! ([`SkiaText`]).
//!
//! Skia has no `wasm32-unknown-unknown` build (only `wasm32-unknown-emscripten`), so on wasm32 this
//! crate is empty.

#[cfg(not(target_arch = "wasm32"))]
mod images;
#[cfg(not(target_arch = "wasm32"))]
mod picture;
#[cfg(not(target_arch = "wasm32"))]
mod renderer;
#[cfg(not(target_arch = "wasm32"))]
mod surface;
#[cfg(not(target_arch = "wasm32"))]
mod text;
#[cfg(not(target_arch = "wasm32"))]
mod window;

#[cfg(not(target_arch = "wasm32"))]
pub use renderer::{DrawnFrame, SkiaOptions, SkiaRenderer};
#[cfg(not(target_arch = "wasm32"))]
pub use surface::{Ingested, SurfaceProducer, SurfaceSpec};
#[cfg(not(target_arch = "wasm32"))]
pub use text::{FontFace, SkiaText};
#[cfg(not(target_arch = "wasm32"))]
pub use window::to_xrgb;
