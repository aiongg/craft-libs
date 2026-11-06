//! [`SliceOverlay`]: the slices drawn over the document.

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
/// - User and layer slices: a `outline` (1 pt) line inside the edge in the slice colour
///   ([`color`](Self::color), the slice colour by default); the selected slice
///   `selected_outline` (2 pt) in the selected slice colour.
/// - Auto slices (only with [`auto`](Self::auto)): dashed `auto_dash` /
///   (3 / 3) at `auto_alpha` (0.7) of the slice colour.
/// - Badges ([`numbers`](Self::numbers)): the number "NN" (two digits, the locale's digits; layer
///   slices "NN" and a small square) in the caption role on a `badge_radius` rounded rect at
///   `badge_offset` from the top-left corner, padded `badge_pad`; only on a slice
///   larger on screen than `badge_min` (18 × 12). Fill: the auto slice colour (auto),
///   the selected slice colour (selected), the slice colour (others); white text.
/// - Document geometry never mirrors right to left.
#[derive(Clone, Copy, Debug)]
pub struct SliceOverlay<'a> {
    items: &'a [SliceItem],
    selected: Option<usize>,
    color: Option<Srgba>,
    numbers: bool,
    auto: bool,
}

impl<'a> SliceOverlay<'a> {
    /// The overlay of `items`, numbers and auto slices shown.
    pub fn new(items: &'a [SliceItem]) -> Self {
        SliceOverlay { items, selected: None, color: None, numbers: true, auto: true }
    }

    /// The selected slice's number.
    pub fn selected(mut self, number: Option<usize>) -> Self {
        self.selected = number;
        self
    }

    /// The slice colour preference; the slice colour when `None`.
    pub fn color(mut self, color: Option<Srgba>) -> Self {
        self.color = color;
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
        let color = self.color.map_or(style.color, Color32::from);
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
                    let stroke = Stroke::new(style.outline, color.gamma_multiply(style.auto_alpha));
                    let points = [quad[0], quad[1], quad[2], quad[3], quad[0]];
                    painter.add(Shape::dashed_line(&points, stroke, style.auto_dash.0, style.auto_dash.1));
                } else {
                    outline(painter, view, item.rect, Stroke::new(style.outline, color));
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
                        color
                    };
                    let galley = painter.layout_no_wrap(number_text(numbers, item.number), style.badge_font.clone(), style.text);
                    // pc: `slice_ui.rs:287` types "{:02} ▪"; the installed fonts have no "▪" (U+25AA) and egui
                    // panics on a missing glyph, so the mark is a square painted at `LAYER_MARK` of the number's height.
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
