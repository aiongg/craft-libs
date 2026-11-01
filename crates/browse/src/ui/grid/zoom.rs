//! `TileZoom`, `ZoomStep` and `ThumbnailSlider`: the size of tiles, by steps or continuously.

use std::ops::RangeInclusive;

use craft_ui::controls::{ControlSize, IconButton, Slider};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::icons::Icon;
use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _};

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
/// rule). Words from `TileLabels` (`.labels(..)`, else the language of the `Ui`). Ladder steps are the slider's marks.
pub struct ThumbnailSlider {
    id_salt: IdSalt,
    value: f32,
    zoom: TileZoom,
    labels: Option<TileLabels>,
    width: Option<f32>,
}

impl ThumbnailSlider {
    fn marks() -> [f64; 11] {
        TileZoom::LADDER.map(f64::from)
    }

    /// The slider for `value` (the host's current size).
    pub fn new(id_salt: impl AsIdSalt, value: f32, zoom: TileZoom) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`: the zoom slider).
        Self { id_salt: IdSalt::new(id_salt), value, zoom, labels: None, width: None }
    }

    /// The widths of the whole control, end buttons included, in `ui`: the natural one (the
    /// slider's `slider.width`) and the least (half of it). A host that is short of room gives
    /// [`width`](Self::width) a value between them, or leaves the control out.
    pub fn extent(ui: &Ui) -> (f32, f32) {
        // new: footers fit the control to the room they have.
        let tokens = Tokens::of(ui);
        let fixed = 2.0 * (IconButton::style(&tokens, ControlSize::Md).side + ui.spacing().item_spacing.x);
        let slider = Slider::style(&tokens, ControlSize::Md).width;
        (fixed + slider, fixed + slider / 2.0)
    }

    /// The width of the whole control, end buttons included (default: natural, see
    /// [`extent`](Self::extent)).
    #[must_use]
    pub fn width(self, width: f32) -> Self {
        // new: builder.
        Self { width: Some(width), ..self }
    }

    /// The words: the slider's name and the two end buttons' (default `TileLabels::of(ui)`).
    #[must_use]
    pub fn labels(self, labels: &TileLabels) -> Self {
        // new: builder.
        Self { labels: Some(labels.clone()), ..self }
    }

    /// Draws the glyphs and the slider; the host applies `Update`, `Commit` and `Set`, and
    /// restores on `Cancel`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        // new: Bridge's slider; composed of `IconButton`, `Slider`, `IconButton`.
        let labels = self.labels.clone().unwrap_or_else(|| TileLabels::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let range = self.zoom.range();
        let (low, high) = (*range.start(), *range.end());
        let mut step = None;
        let mut response = None;
        ui.horizontal(|ui| {
            let out = ui
                .add(IconButton::new(Icon::GridSmall, labels.smaller.as_str()).size(ControlSize::Md).enabled(self.value > low).id_salt((id, "out")))
                .clicked();
            if out {
                step = Some(ZoomStep::Out);
            }
            let mut slider = Slider::new(id, labels.thumbnail_size.as_str(), self.value, range.clone())
                .size(ControlSize::Md)
                .default_value(self.zoom.step(self.value, ZoomStep::Reset));
            if let Some(total) = self.width {
                let ends = 2.0 * (IconButton::style(&Tokens::of(ui), ControlSize::Md).side + ui.spacing().item_spacing.x);
                slider = slider.width((total - ends).max(0.0));
            }
            slider = match self.zoom {
                TileZoom::Ladder => slider.marks(&Self::marks()),
                TileZoom::Range { step, .. } => slider.step(step),
            };
            response = Some(slider.show(ui));
            let more = ui
                .add(IconButton::new(Icon::Grid, labels.larger.as_str()).size(ControlSize::Md).enabled(self.value < high).id_salt((id, "in")))
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
