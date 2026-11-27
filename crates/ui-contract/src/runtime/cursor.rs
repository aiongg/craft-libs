//! The cursor service (8.5): requests of a pass collect here, the end hook picks the winner and records it, and a
//! painted winner reaches the window as its cursor image where the window shows one.

use crate::runtime::Shift;

use egui::Rect;
use std::any::Any;
use std::sync::Arc;

/// The largest side elsewhere: winit's limit; AppKit sets none on an `NSCursor` image.
pub(crate) const MAX_SIDE: u16 = 2048;

/// A cursor image in straight (not premultiplied) RGBA, row by row, its hotspot on the centre pixel.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CursorBitmap {
    /// Straight RGBA, row by row.
    pub rgba: Arc<[u8]>,
    /// Width and height in pixels; odd, so the hotspot lies on the centre pixel at every scale.
    pub size: [u16; 2],
    /// The hotspot in pixels from the top left.
    pub hotspot: [u16; 2],
}

/// Where a cursor's footprint may show: a layer and a clip in screen space. A brush tip outline is a mark on its
/// canvas, not a pointer, so it shows only inside the canvas's view and under every layer above the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CursorRegion {
    /// The layer the requester paints on.
    pub layer: craft_ui_rt::Layer,
    /// The part of the screen the footprint may cover.
    pub clip: Rect,
}

impl Shift for CursorRegion {
    fn shifted(&self, by: &crate::runtime::RecordShift) -> Self {
        CursorRegion { layer: self.layer, clip: crate::runtime::Clip::new(self.clip).shifted(by).rect() }
    }

    fn snapped(&self, to: &Self, within: f32) -> Self {
        CursorRegion { layer: self.layer, clip: self.clip.snapped(&to.clip, within) }
    }
}

impl CursorRegion {
    /// The region of `clip` on `layer`.
    pub fn new(layer: craft_ui_rt::Layer, clip: Rect) -> Self {
        CursorRegion { layer, clip }
    }
}

/// A painted pointer the window can take as its cursor image: drawn at a scale, compared with the last one handed over.
pub(crate) trait CursorArt: Any + Clone + PartialEq + Send + Sync {
    /// The image at `scale` device pixels per point, `None` where the art has none at that size.
    fn rasterise(&self, scale: f32, max_side: u16) -> Option<CursorBitmap>;
}
