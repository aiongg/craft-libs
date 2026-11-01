//! [`ShapeOptionsBar`]: the Shape tools' options bar cells (task SH-T6).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_color::ui::PaintChip;
use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use super::{ShapeAppearance, ShapeChange, ShapeLabels, ShapePaint, ShapeToolSettings};
use crate::ui::shape_drag::ShapeKind;
use craft_ui::edit::EditEvent;

/// The app's decoration of a paint chip before it shows.
type ChipDecor<'a> = dyn FnMut(ShapePaint, PaintChip<'a>) -> PaintChip<'a> + 'a;

/// What [`ShapeOptionsBar::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app routes each change to the selected layer and to the tool settings"]
pub struct ShapeOptionsOutput {
    /// Each changed cell this pass, as [`ShapePanelOutput::events`](super::ShapePanelOutput::events).
    /// With a shape layer selected the app applies it to the layer and to [`ShapeToolSettings`];
    /// without, to the settings only (PhotoCraft #3347).
    pub events: Vec<EditEvent<ShapeChange>>,
}

/// The cells a Shape tool adds to the options bar, in PhotoCraft's order: "Fill" check box and
/// chip, "Stroke" chip, stroke width (0–288 px), the Stroke style button; then the kind's cell:
/// Rectangle and Rounded Rectangle "Radius" (0–10,000), Polygon and Star "Sides" (3–100), Line
/// "Weight" (1–1,000). Custom Shape's picker is the app's (S6a-presets' `PresetPicker`), added
/// after these cells.
///
/// - The values are the selected shape layer's when one is selected, else the tool settings'
///   (the app passes the right [`ShapeAppearance`]).
/// - Stroke width 0 is no stroke; turning the stroke on at 0 gives
///   [`ShapeChange::ENABLED_STROKE_WIDTH`].
/// - Keyboard: every cell is a Tab stop in the bar's order; captions scrub their fields.
pub struct ShapeOptionsBar<'a> {
    id_salt: IdSalt,
    kind: ShapeKind,
    settings: &'a ShapeToolSettings,
    appearance: &'a ShapeAppearance,
    stroke_open: Option<&'a mut bool>,
    chips: Option<Box<ChipDecor<'a>>>,
    fallbacks: (Srgba, Srgba),
    labels: Option<ShapeLabels>,
}

impl<'a> ShapeOptionsBar<'a> {
    /// The cells for `kind` with the `settings` and `appearance` shown.
    pub fn new(id_salt: impl AsIdSalt, kind: ShapeKind, settings: &'a ShapeToolSettings, appearance: &'a ShapeAppearance) -> Self {
        ShapeOptionsBar {
            id_salt: IdSalt::new(id_salt),
            kind,
            settings,
            appearance,
            stroke_open: None,
            chips: None,
            fallbacks: (Srgba::BLACK, Srgba::WHITE),
            labels: None,
        }
    }

    /// The Stroke style popover's open flag (S7-pen's `StrokeButton`); without it the button is
    /// not shown.
    pub fn stroke_open(mut self, open: &'a mut bool) -> Self {
        self.stroke_open = Some(open);
        self
    }

    /// Decorates each paint chip before it shows (its Gradient, Pattern and Picture editors).
    pub fn chips(mut self, decorate: impl FnMut(ShapePaint, PaintChip<'a>) -> PaintChip<'a> + 'a) -> Self {
        self.chips = Some(Box::new(decorate));
        self
    }

    /// The foreground and background colours a switch to Solid starts from.
    pub fn fallbacks(mut self, fill: Srgba, stroke: Srgba) -> Self {
        self.fallbacks = (fill, stroke);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ShapeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the cells to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> ShapeOptionsOutput {
        // pc: crates/ui-egui/src/tools/shape_options.rs:35-191 `options_bar` (Fill :63, Stroke :69, the
        // width field :71, Radius :100, Sides :113, Weight :121; the edit target :36-52, :173-191).
        // new: `OptionsBar` cells, `PaintChip`, `NumberField`, `craft_draw::ui::stroke::StrokeButton`
        // (its `EditEvent<StrokeStyle>` mapped to `ShapeChange::StrokeStyle`).
        todo!()
    }
}
