//! [`SliceOverlay`]: the slices drawn over the document (task SH-T4).

use craft_theme::Srgba;
use egui::{Color32, Painter, Rect, Shape, Stroke, StrokeKind, pos2, vec2};
use kurbo::Rect as DocRect;

use super::{SliceItem, SliceOrigin, SliceStyle};
use craft_ui::canvas::CanvasView;
use craft_ui::localized::LocaleNumbers;

/// A layer slice's mark after its number, as a fraction of the number's height.
const LAYER_MARK: f32 = 0.45;

/// The slices drawn over the document, shown while a Slice tool is active and, without the tools,
/// when View › Show › Slices is on and the document has slices (the app draws it itself then).
///
/// - User and layer slices: a `slice.outline` (1 pt) line inside the edge in the slice colour
///   ([`colour`](Self::colour), `overlay.slice` by default); the selected slice
///   `slice.selected_outline` (2 pt) in `overlay.slice_selected`.
/// - Auto slices (only with [`auto`](Self::auto)): dashed `slice.auto_dash` / `slice.auto_gap`
///   (3 / 3) at `slice.auto_alpha` (0.7) of the slice colour.
/// - Badges ([`numbers`](Self::numbers)): the number "NN" (two digits, the locale's digits; layer
///   slices "NN" and a small square) in the caption role on a `slice.badge_radius` rounded rect at
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
        let colour = self.colour.map_or(style.colour, Color32::from);
        // Auto slices first (dashed, underneath), then user and layer slices.
        for auto_pass in [true, false] {
            if auto_pass && !self.auto {
                continue;
            }
            for item in self.items.iter().filter(|i| (i.origin == SliceOrigin::Auto) == auto_pass) {
                let selected = self.selected == Some(item.number);
                let quad = view.quad(item.rect);
                let screen = Rect::from_points(&quad);
                if auto_pass {
                    let stroke = Stroke::new(style.outline, colour.gamma_multiply(style.auto_alpha));
                    let points = [quad[0], quad[1], quad[2], quad[3], quad[0]];
                    painter.add(Shape::dashed_line(&points, stroke, style.auto_dash.0, style.auto_dash.1));
                } else {
                    outline(painter, view, item.rect, Stroke::new(style.outline, colour));
                }
                if selected {
                    outline(painter, view, item.rect, Stroke::new(style.selected_outline, style.selected));
                }
                if self.numbers && screen.width() > style.badge_min.x && screen.height() > style.badge_min.y {
                    let layer = item.origin == SliceOrigin::Layer;
                    let fill = if auto_pass {
                        style.auto_fill
                    } else if selected {
                        style.selected
                    } else {
                        colour
                    };
                    let galley = painter.layout_no_wrap(number_text(numbers, item.number), style.badge_font.clone(), style.text);
                    // The layer mark is drawn, not typed: the installed fonts have no "▪".
                    let mark = if layer { galley.size().y * LAYER_MARK } else { 0.0 };
                    let gap = if layer { galley.size().y * LAYER_MARK } else { 0.0 };
                    let size = vec2(galley.size().x + gap + mark, galley.size().y) + style.badge_pad * 2.0;
                    let badge = Rect::from_min_size(screen.left_top() + vec2(style.badge_offset, style.badge_offset), size);
                    painter.rect_filled(badge, style.badge_radius, fill);
                    let text_at = badge.min + style.badge_pad;
                    if layer {
                        let centre = pos2(text_at.x + galley.size().x + gap + mark / 2.0, text_at.y + galley.size().y / 2.0);
                        painter.rect_filled(Rect::from_center_size(centre, vec2(mark, mark)), 0.0, style.text);
                    }
                    painter.galley(text_at, galley, style.text);
                }
            }
        }
    }
}

/// A slice's number as two digits in the locale's digits ("03").
pub(super) fn number_text(numbers: &LocaleNumbers, number: usize) -> String {
    let text = numbers.format(number as f64, 0..=0);
    if number < 10 { format!("{}{text}", numbers.format(0.0, 0..=0)) } else { text }
}

/// `rect`'s outline inside its edge: a plain rectangle on an axis-aligned view, the turned quad
/// otherwise.
pub(super) fn outline(painter: &Painter, view: &CanvasView, rect: DocRect, stroke: Stroke) {
    let quad = view.quad(rect);
    if view.axis_aligned() {
        painter.rect_stroke(Rect::from_points(&quad), 0.0, stroke, StrokeKind::Inside);
    } else {
        painter.add(Shape::closed_line(quad.to_vec(), stroke));
    }
}
