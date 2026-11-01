//! [`ShapePanel`]: a shape layer's Appearance and Shape sections in the Properties panel (task
//! SH-T6).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_color::ui::PaintChip;
use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use super::{LiveShape, ShapeAppearance, ShapeChange, ShapeLabels};
use craft_ui::edit::EditEvent;

/// Which paint a chip edits, for [`ShapePanel::chips`] and [`ShapeOptionsBar::chips`](super::ShapeOptionsBar::chips).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapePaint {
    Fill,
    Stroke,
}

/// The app's decoration of a paint chip before it shows: the Gradient, Pattern and Picture tabs.
type ChipDecor<'a> = dyn FnMut(ShapePaint, PaintChip<'a>) -> PaintChip<'a> + 'a;

/// What [`ShapePanel::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app turns each event into a document command"]
pub struct ShapePanelOutput {
    /// Each changed field this pass, in drawing order: a scrub or drag is `Begin`, `Update`,
    /// `Commit` (one undo step per gesture, replacing PhotoCraft's `coalesce` keys); a chip tab, a
    /// typed value or an arrow step is `Set`.
    pub events: Vec<EditEvent<ShapeChange>>,
}

/// A shape layer's Properties sections, as PhotoCraft's `shape_properties`.
///
/// - Appearance (collapsible `SectionHeader`, id `appearance`): Fill chip, Stroke chip, stroke
///   width (`NumberField` 0–288 px) and the Stroke style button (S7-pen's `StrokeButton`, enabled
///   while the width is over 0, its popover open flag from [`stroke_open`](Self::stroke_open)).
///   Stroke width 0 is no stroke; turning a stroke on at 0 gives
///   [`ShapeChange::ENABLED_STROKE_WIDTH`]; an arrow step that clamps at 0 reports nothing.
/// - Shape (collapsible, id `liveShape`), by the [`LiveShape`]: W and H (1–300,000 px, whole
///   pixels) for a rect, ellipse or polygon; a rect's "Corner radius" (0–100,000, sets all four)
///   then Top Left, Top Right, Bottom Left, Bottom Right rows (PhotoCraft's row order, mapped to
///   the radii order TL, TR, BR, BL); a polygon's Corner radius, Sides (3–100) and Star ratio
///   (1–100 %, stored 0.01–1); a line's Weight (1–10,000).
/// - A locked layer ([`ShapeAppearance::locked`]) disables the whole panel.
/// - Keyboard: every field and chip is a Tab stop in reading order; sections fold with their
///   headers. Right to left the rows mirror; the corner names stay.
pub struct ShapePanel<'a> {
    id_salt: IdSalt,
    shape: &'a LiveShape,
    appearance: &'a ShapeAppearance,
    stroke_open: Option<&'a mut bool>,
    chips: Option<Box<ChipDecor<'a>>>,
    fallbacks: (Srgba, Srgba),
    sections: (bool, bool),
    labels: Option<ShapeLabels>,
}

impl<'a> ShapePanel<'a> {
    /// The panel for `shape` with `appearance`.
    pub fn new(id_salt: impl AsIdSalt, shape: &'a LiveShape, appearance: &'a ShapeAppearance) -> Self {
        ShapePanel {
            id_salt: IdSalt::new(id_salt),
            shape,
            appearance,
            stroke_open: None,
            chips: None,
            fallbacks: (Srgba::BLACK, Srgba::WHITE),
            sections: (true, true),
            labels: None,
        }
    }

    /// The Stroke style popover's open flag, kept by the app between passes (S7-pen's
    /// `StrokeButton::show(ui, &mut open)`). Without it the button is not shown.
    pub fn stroke_open(mut self, open: &'a mut bool) -> Self {
        self.stroke_open = Some(open);
        self
    }

    /// Decorates each paint chip before it shows (its Gradient, Pattern and Picture editors).
    pub fn chips(mut self, decorate: impl FnMut(ShapePaint, PaintChip<'a>) -> PaintChip<'a> + 'a) -> Self {
        self.chips = Some(Box::new(decorate));
        self
    }

    /// The colours a switch to Solid starts from when a chip has none: the app's foreground (fill)
    /// and background (stroke). Black and white by default.
    pub fn fallbacks(mut self, fill: Srgba, stroke: Srgba) -> Self {
        self.fallbacks = (fill, stroke);
        self
    }

    /// Which sections show; both by default (an app may show one, or a custom path's layer only
    /// Appearance).
    pub fn sections(mut self, appearance: bool, shape: bool) -> Self {
        self.sections = (appearance, shape);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ShapeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ShapePanelOutput {
        // pc: crates/ui-egui/src/tools/vector_ui/options.rs:156-312 `shape_properties` (Appearance at
        // :178, the live shape at :233, fields :232-300, corner rows :265, the stroke width rule
        // :126-128, the gesture keys :142-153).
        // new: `SectionHeader` (collapsible), `PaintChip`, `NumberField`, S7-pen's `StrokeButton`
        // (needs: PE craft_draw::ui::stroke::StrokeButton).
        todo!()
    }
}
