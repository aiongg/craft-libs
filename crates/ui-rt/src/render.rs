use emath::Rect;

use crate::display::{ImageStore, RgbaImage};
use crate::frame::FrameTree;
use crate::geom::{RectU, SizeU};
use crate::id::WindowId;
use crate::platform::SurfaceHandle;
use crate::text::TextEngine;

/// What one render did, for the bench's prepare column and count checks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderStats {
    /// Display lists in the frame.
    pub draws: u32,
    /// Display lists drawn and converted this frame (meshes tessellated, pictures recorded).
    pub prepared: u32,
    /// Display lists drawn from the backend's cache.
    pub reused: u32,
    pub images_uploaded: u32,
    /// CPU time spent turning display lists into the backend's form for this frame (meshes
    /// tessellated, culled and moved; pictures recorded), in milliseconds. GPU uploads, command
    /// encoding, submission and rasterisation are not in it.
    pub prepare_ms: f64,
}

/// A pending [`Renderer::request_read_back`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReadBackId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub enum ReadBack {
    /// Not done yet; poll again after a later step.
    Pending,
    Ready(RgbaImage),
    /// The window or its surface went before the pixels were read, or the id is unknown.
    Failed,
}

/// Display lists in, pixels out, one target per window. The driver calls, per frame:
/// [`Renderer::sync_images`] once, then [`Renderer::render`] for each window whose pixels changed.
pub trait Renderer {
    type Text: TextEngine;
    fn text(&mut self) -> &mut Self::Text;
    /// Attaches (or re-attaches at a new size or scale) the window's surface.
    fn set_target(&mut self, window: WindowId, target: SurfaceHandle, size_px: SizeU, scale: f32);
    /// Drops the window's surface and keeps the device and caches; the next `set_target` draws
    /// again. Also called when the window closes.
    fn lose_target(&mut self, window: WindowId);
    /// Applies [`ImageStore::changes`]: uploads new images and changed sub-rects, drops removed
    /// ones. Called once per frame before the windows render; images are shared by all windows.
    fn sync_images(&mut self, images: &ImageStore) -> u32;
    /// `damage` is in logical points: no pixel outside it differs from the window's last rendered
    /// frame, so a backend that keeps its pixels draws only inside it ([`crate::damage_pixels`]).
    /// On a new target the runtime sends the whole viewport.
    fn render(&mut self, window: WindowId, frame: &FrameTree<'_>, damage: &[Rect]) -> RenderStats;
    /// Starts reading back `rect` (physical pixels; the whole target when `None`) of the window's
    /// last rendered frame. GPU backends map a buffer asynchronously (the web cannot block).
    fn request_read_back(&mut self, window: WindowId, rect: Option<RectU>) -> ReadBackId;
    fn poll_read_back(&mut self, id: ReadBackId) -> ReadBack;
}
