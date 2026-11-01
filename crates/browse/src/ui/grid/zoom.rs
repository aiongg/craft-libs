//! `TileZoom`, `ZoomStep` and `ThumbnailSlider`: the size of tiles, by steps or continuously.

use std::ops::RangeInclusive;

use craft_ui::controls::{ControlSize, IconButton, Slider};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::icons::Icon;
use egui::{AsIdSalt, Ui};

use super::TileLabels;

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
        const TOLERANCE: f32 = 0.001;
        match *self {
            TileZoom::Ladder => {
                let ladder = Self::LADDER;
                let (first, last) = (ladder[0], ladder[ladder.len() - 1]);
                match step {
                    ZoomStep::Reset => 1.0,
                    ZoomStep::In => ladder.iter().copied().find(|s| *s > value + TOLERANCE).unwrap_or(last),
                    ZoomStep::Out => ladder.iter().rev().copied().find(|s| *s < value - TOLERANCE).unwrap_or(first),
                }
            }
            TileZoom::Range { min, max, step: by, default } => match step {
                ZoomStep::Reset => default.clamp(min, max),
                ZoomStep::In => (value + by).clamp(min, max),
                ZoomStep::Out => (value - by).clamp(min, max),
            },
        }
    }

    /// The slider's range: the ladder's ends, or `min..=max`.
    pub fn range(&self) -> RangeInclusive<f32> {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`: Zoom slider 48–400).
        match *self {
            TileZoom::Ladder => Self::LADDER[0]..=Self::LADDER[Self::LADDER.len() - 1],
            TileZoom::Range { min, max, .. } => min..=max,
        }
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
    fn marks() -> [f64; 11] {
        TileZoom::LADDER.map(f64::from)
    }

    /// The slider for `value` (the host's current size).
    pub fn new(id_salt: impl AsIdSalt, value: f32, zoom: TileZoom) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`: the zoom slider).
        Self { id_salt: egui::Id::NULL.with(egui::IdSalt::new(id_salt)), value, zoom, label: TileLabels::default().thumbnail_size }
    }

    /// The accessible name (default `TileLabels::thumbnail_size`).
    #[must_use]
    pub fn label(self, label: impl Into<String>) -> Self {
        // new: builder.
        Self { label: label.into(), ..self }
    }

    /// Draws the glyphs and the slider; the host applies `Update`, `Commit` and `Set`, and
    /// restores on `Cancel`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        // new: Bridge's slider; composed of `IconButton`, `Slider`, `IconButton`.
        let labels = TileLabels::default();
        let range = self.zoom.range();
        let (low, high) = (*range.start(), *range.end());
        let mut step = None;
        let mut response = None;
        ui.horizontal(|ui| {
            let out = ui
                .add(
                    IconButton::new(Icon::GridSmall, labels.smaller.as_str())
                        .size(ControlSize::Md)
                        .enabled(self.value > low)
                        .id_salt((self.id_salt, "out")),
                )
                .clicked();
            if out {
                step = Some(ZoomStep::Out);
            }
            let mut slider = Slider::new(self.id_salt, self.label.as_str(), self.value, range.clone())
                .size(ControlSize::Md)
                .default_value(self.zoom.step(self.value, ZoomStep::Reset));
            slider = match self.zoom {
                TileZoom::Ladder => slider.marks(&Self::marks()),
                TileZoom::Range { step, .. } => slider.step(step),
            };
            response = Some(slider.show(ui));
            let more = ui
                .add(
                    IconButton::new(Icon::Grid, labels.larger.as_str())
                        .size(ControlSize::Md)
                        .enabled(self.value < high)
                        .id_salt((self.id_salt, "in")),
                )
                .clicked();
            if more {
                step = Some(ZoomStep::In);
            }
        });
        let mut out = response.expect("the slider is drawn");
        if let Some(step) = step {
            let modifiers = ui.input(|i| i.modifiers);
            out.event = Some(EditEvent::set(f64::from(self.zoom.step(self.value, step)), modifiers));
        }
        out
    }
}
