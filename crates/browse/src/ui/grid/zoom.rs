//! `TileZoom`, `ZoomStep` and `ThumbnailSlider`: the size of tiles, by steps or continuously.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use craft_ui::edit::EditResponse;
use egui::{AsIdSalt, Ui};

/// A step of the tile size, from Cmd/Ctrl + / − / 0 or the slider's end glyphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoomStep {
    /// Bigger.
    In,
    /// Smaller.
    Out,
    /// Back to the default.
    Reset,
}

/// How a tile size changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TileZoom {
    /// Factors on [`TileZoom::LADDER`], default 1.0 (card grids; PdfCraft's ladder).
    Ladder,
    /// A continuous size in points (bins: FilmCraft 48–400), `step` per key press, `default` for
    /// Reset.
    Range { min: f32, max: f32, step: f32, default: f32 },
}

impl TileZoom {
    /// PdfCraft's zoom ladder: 0.6, 0.8, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0.
    pub const LADDER: [f32; 11] = [0.6, 0.8, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0];

    /// The value after `step` from `value`: the next ladder value above or below (a tolerance of
    /// 0.001, so a value between rungs goes to the next one), clamped at the ends; for `Range`,
    /// `value ± step` clamped; `Reset` gives 1.0 or `default`.
    pub fn step(&self, value: f32, step: ZoomStep) -> f32 {
        // pdfcraft: crates/ui-egui/src/workflows/combine_grid.rs:50 (`ZOOM_STEPS` and the step search below it).
        todo!()
    }

    /// The slider's range: the ladder's ends, or `min..=max`.
    pub fn range(&self) -> RangeInclusive<f32> {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`: Zoom slider 48–400).
        todo!()
    }
}

/// The tile size slider (Bridge's thumbnail slider): a library `Slider` over
/// [`TileZoom::range`] with a small picture glyph at its start and a large one at its end, each
/// an `IconButton` that steps `Out` and `In`. The slider reports the gesture as `EditEvent<f64>`;
/// Esc during the drag restores the size it started from (FilmCraft #580, the library slider's
/// rule). Accessible name from `TileLabels::thumbnail_size`. Ladder steps are the slider's marks.
pub struct ThumbnailSlider {
    id_salt: egui::Id,
    value: f32,
    zoom: TileZoom,
    label: String,
}

impl ThumbnailSlider {
    /// The slider for `value` (the host's current size).
    pub fn new(id_salt: impl AsIdSalt, value: f32, zoom: TileZoom) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`: the zoom slider).
        todo!()
    }

    /// The accessible name (default `TileLabels::thumbnail_size`).
    #[must_use]
    pub fn label(self, label: impl Into<String>) -> Self {
        // new: builder.
        todo!()
    }

    /// Draws the glyphs and the slider; the host applies `Update`, `Commit` and `Set`, and
    /// restores on `Cancel`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        // new: Bridge's slider; composed of `IconButton`, `Slider`, `IconButton`.
        todo!()
    }
}
