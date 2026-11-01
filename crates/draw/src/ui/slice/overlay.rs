//! [`SliceOverlay`]: the slices drawn over the document (task SH-T4).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::Painter;

use super::{SliceItem, SliceStyle};
use craft_ui::canvas::CanvasView;
use craft_ui::localized::LocaleNumbers;

/// The slices drawn over the document, shown while a Slice tool is active and, without the tools,
/// when View › Show › Slices is on and the document has slices (the app draws it itself then).
///
/// - User and layer slices: a `slice.outline` (1 pt) line inside the edge in the slice colour
///   ([`colour`](Self::colour), `overlay.slice` by default); the selected slice
///   `slice.selected_outline` (2 pt) in `overlay.slice_selected`.
/// - Auto slices (only with [`auto`](Self::auto)): dashed `slice.auto_dash` / `slice.auto_gap`
///   (3 / 3) at `slice.auto_alpha` (0.7) of the slice colour.
/// - Badges ([`numbers`](Self::numbers)): the number "NN" (two digits, the locale's digits; layer
///   slices "NN ▪") in the caption role on a `slice.badge_radius` rounded rect at
///   `slice.badge_offset` from the top-left corner, padded `slice.badge_pad`; only on a slice
///   larger on screen than `slice.badge_min` (18 × 12). Fill: `overlay.slice_auto` (auto),
///   `overlay.slice_selected` (selected), the slice colour (others); text `overlay.slice_text`.
/// - Document geometry never mirrors right to left.
#[derive(Clone, Copy, Debug)]
pub struct SliceOverlay<'a> {
    items: &'a [SliceItem],
    selected: Option<usize>,
    colour: Option<Srgba>,
    numbers: bool,
    auto: bool,
}

impl<'a> SliceOverlay<'a> {
    /// The overlay of `items`, numbers and auto slices shown.
    pub fn new(items: &'a [SliceItem]) -> Self {
        SliceOverlay { items, selected: None, colour: None, numbers: true, auto: true }
    }

    /// The selected slice's number.
    pub fn selected(mut self, number: Option<usize>) -> Self {
        self.selected = number;
        self
    }

    /// The slice colour preference; `overlay.slice` when `None`.
    pub fn colour(mut self, colour: Option<Srgba>) -> Self {
        self.colour = colour;
        self
    }

    /// Show Slice Numbers.
    pub fn numbers(mut self, numbers: bool) -> Self {
        self.numbers = numbers;
        self
    }

    /// Draw auto slices (the tools on and Hide Auto Slices off).
    pub fn auto(mut self, auto: bool) -> Self {
        self.auto = auto;
        self
    }

    /// Draws the slices through `view`.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &SliceStyle, numbers: &LocaleNumbers) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:242-303 `draw_overlay` (the colour :249, auto
        // dashed 3/3 at × 0.7 :264, outlines :266-269, the badge rule `sr.width() > 18.0 &&
        // sr.height() > 12.0` :285, the badge at (2, 2) grown (6, 2) :292, 2 pt radius :300, text at
        // (3, 1) :301).
        todo!()
    }
}
