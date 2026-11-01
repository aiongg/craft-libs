//! [`MockImage`]: a generated picture for the workspace stories (task ST-WS1).
//!
//! Original art drawn by code, so the stories show a photo-like image without a file: a gradient
//! sky, two hills, a lake, a sun disc and a few hard-edged shapes (a house, a fence) whose edges
//! give the selection and fill tools something to find. 1600 × 1067 by default (3:2).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task ST-WS1")]

use egui::ColorImage;

use craft_ui::canvas::{PixelRect, RasterSource};

/// The generated picture as a `RasterSource`. Pixels are computed on read (no buffer), so any
/// size costs nothing until shown. [`set_revision`](Self::set_revision) lets a story say "the
/// document changed" (a workspace closes or re-reads on it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MockImage {
    size: [u32; 2],
    revision: u64,
}

impl Default for MockImage {
    fn default() -> Self {
        Self { size: [1600, 1067], revision: 1 }
    }
}

impl MockImage {
    pub(crate) fn new(size: [u32; 2]) -> Self {
        Self { size, revision: 1 }
    }

    pub(crate) fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    /// The colour at document pixel `(x, y)`, sRGBA unmultiplied: sky, sun, hills, lake, shapes.
    pub(crate) fn pixel(&self, x: f64, y: f64) -> [u8; 4] {
        // new: original art for the stories.
        todo!()
    }
}

impl RasterSource for MockImage {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        // new: samples `pixel` at each texel's centre.
        todo!()
    }
}
