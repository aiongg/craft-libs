//! The ruler's geometry and drawing.

use craft_ui::canvas::{RulerStyle, RulerUnits, Rulers, TickRule, crisp};
use egui::{Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use super::TabAlign;

/// A decimal stop's dot sits this share of the foot along, and this share of the stem up
/// (WordCraft `ruler_tabs.rs:101`: 0.7 of the foot, 3.5 of a 5 stem).
const DOT_ALONG: f32 = 0.7;
const DOT_UP: f32 = 0.7;
/// The dot's radius, as a share of the marker stroke's width (WordCraft `ruler_tabs.rs:101`: 1.1
/// beside a 1.5 stroke).
const DOT_RADIUS: f32 = 0.75;
/// A bar stop's stem, as a share of a stop's (WordCraft `ruler_tabs.rs:105`: 7 against 5).
const BAR_STEM: f32 = 1.4;
/// The grip across a table column marker: three vertical lines this far apart (points), each
/// this far long on either side of its middle (WordCraft `table_ruler.rs:47-50`: three
/// lines 3 apart, 1.5 either way).
const GRIP_LINES: [f32; 3] = [-1.0, 0.0, 1.0];
const GRIP_PITCH: f32 = 3.0;
const GRIP_HALF: f32 = 1.5;

/// Where the column sits on the strip: positions are points from the column's start edge, which
/// is the strip's right end in a right-to-left paragraph.
#[derive(Clone, Copy, Debug)]
pub(super) struct Geometry {
    pub strip: Rect,
    /// Screen points per point.
    pub scale: f32,
    pub right_to_left: bool,
    pub width: f32,
}

impl Geometry {
    /// The screen x of the distance `d` from the start edge.
    pub(super) fn x(&self, d: f32) -> f32 {
        if self.right_to_left { self.strip.max.x - d * self.scale } else { self.strip.min.x + d * self.scale }
    }

    /// The distance from the start edge at the screen x.
    pub(super) fn at(&self, x: f32) -> f32 {
        if self.right_to_left { (self.strip.max.x - x) / self.scale } else { (x - self.strip.min.x) / self.scale }
    }

    /// The part of the strip over the column.
    pub(super) fn column(&self) -> Rect {
        let (a, b) = (self.x(0.0), self.x(self.width));
        Rect::from_x_y_ranges(a.min(b)..=a.max(b), self.strip.y_range()).intersect(self.strip)
    }

    /// The direction of increasing distance on screen: 1 or -1.
    pub(super) fn sign(&self) -> f32 {
        if self.right_to_left { -1.0 } else { 1.0 }
    }
}

/// The marker sizes.
#[derive(Clone, Copy, Debug)]
pub(super) struct Shapes {
    /// Half the width of a marker.
    pub half: f32,
    /// The width of a table column marker.
    column: f32,
    /// The height of a triangle.
    tip: f32,
    /// The height of the left indent's box.
    bar: f32,
    /// The height of a stop's stem.
    stem: f32,
    /// The length of a stop's foot.
    foot: f32,
    /// The side of the tab selector's box.
    pub selector: f32,
}

const MARKER_HALF: f32 = 4.5;
const MARKER_COLUMN: f32 = 7.0;
const MARKER_TIP: f32 = 5.0;
const MARKER_BAR: f32 = 4.0;
const MARKER_STEM: f32 = 5.0;
const MARKER_FOOT: f32 = 4.0;
const MARKER_SELECTOR: f32 = 14.0;

impl Shapes {
    pub(super) fn of() -> Self {
        Shapes {
            half: MARKER_HALF,
            column: MARKER_COLUMN,
            tip: MARKER_TIP,
            bar: MARKER_BAR,
            stem: MARKER_STEM,
            foot: MARKER_FOOT,
            selector: MARKER_SELECTOR,
        }
    }

    fn triangle(&self, a: Pos2, b: Pos2, c: Pos2, fill: Color32, stroke: Stroke) -> Shape {
        Shape::convex_polygon(vec![a, b, c], fill, stroke)
    }

    /// The first-line marker's box: a triangle pointing down from the strip's top.
    pub(super) fn first_line(&self, strip: Rect, x: f32) -> Rect {
        Rect::from_min_size(pos2(x - self.half, strip.min.y), vec2(2.0 * self.half, self.tip))
    }

    /// The hanging marker's box: a triangle pointing up from the left box.
    pub(super) fn hanging(&self, strip: Rect, x: f32) -> Rect {
        let base = strip.max.y - self.bar;
        Rect::from_min_max(pos2(x - self.half, base - self.tip), pos2(x + self.half, base))
    }

    pub(super) fn left_box(&self, strip: Rect, x: f32) -> Rect {
        Rect::from_min_max(pos2(x - self.half, strip.max.y - self.bar), pos2(x + self.half, strip.max.y))
    }

    /// The right marker's box: a triangle pointing up from the strip's bottom.
    pub(super) fn right(&self, strip: Rect, x: f32) -> Rect {
        Rect::from_min_max(pos2(x - self.half, strip.max.y - self.tip), pos2(x + self.half, strip.max.y))
    }

    /// The box around a stop's glyph, standing on the strip's bottom.
    pub(super) fn stop(&self, strip: Rect, x: f32) -> Rect {
        let reach = self.foot.max(self.half);
        Rect::from_min_max(pos2(x - reach, strip.max.y - self.stem), pos2(x + reach, strip.max.y))
    }

    /// The first-line, hanging and right triangles and the left box.
    pub(super) fn indents(&self, painter: &Painter, style: &RulerStyle, strip: Rect, at: [f32; 3]) {
        let [first, left, right] = at;
        let stroke = style.marker;
        let fill = style.fill;
        let top = self.first_line(strip, first);
        painter.add(self.triangle(top.left_top(), top.right_top(), pos2(first, top.max.y), fill, stroke));
        let hanging = self.hanging(strip, left);
        painter.add(self.triangle(hanging.left_bottom(), hanging.right_bottom(), pos2(left, hanging.min.y), fill, stroke));
        painter.rect(self.left_box(strip, left), 0.0, fill, stroke, egui::StrokeKind::Inside);
        let end = self.right(strip, right);
        painter.add(self.triangle(end.left_bottom(), end.right_bottom(), pos2(right, end.min.y), fill, stroke));
    }

    /// The glyph of a stop of `kind` standing on `base` at `x`; a right-to-left paragraph mirrors
    /// the foot.
    pub(super) fn glyph(&self, painter: &Painter, x: f32, base: f32, kind: TabAlign, right_to_left: bool, stroke: Stroke) {
        let foot = if right_to_left { -self.foot } else { self.foot };
        let stem = |height: f32| painter.line_segment([pos2(x, base - height), pos2(x, base)], stroke);
        let along = |a: f32, b: f32| painter.line_segment([pos2(x + a, base), pos2(x + b, base)], stroke);
        match kind {
            TabAlign::Left => {
                stem(self.stem);
                along(0.0, foot);
            }
            TabAlign::Right => {
                stem(self.stem);
                along(-foot, 0.0);
            }
            TabAlign::Center => {
                stem(self.stem);
                along(-self.foot, self.foot);
            }
            TabAlign::Decimal => {
                stem(self.stem);
                along(-self.foot, self.foot);
                painter.circle_filled(pos2(x + foot * DOT_ALONG, base - self.stem * DOT_UP), stroke.width * DOT_RADIUS, stroke.color);
            }
            TabAlign::Bar => {
                stem(self.stem * BAR_STEM);
            }
        }
    }
}

/// The margin shade over the strip and the ruler's fill over the column.
pub(super) fn background(painter: &Painter, style: &RulerStyle, g: &Geometry) {
    painter.rect_filled(g.strip, 0.0, style.margin_shade);
    painter.rect_filled(g.column(), 0.0, style.fill);
}

/// The ticks and numbers along the strip from the column's start edge, from the kernel's tick
/// rule. Numbers that would run into a `keep_clear` rect are left out.
pub(super) fn ticks(painter: &Painter, style: &RulerStyle, g: &Geometry, units: &RulerUnits, rule: TickRule<'_>, keep_clear: &[Rect]) {
    for tick in Rulers::ticks(units, rule, style, f64::from(g.scale), span(g, units)) {
        let x = g.x((tick.value / units.per_doc) as f32);
        let bottom = g.strip.bottom();
        crisp(painter, pos2(x, bottom - tick.length), pos2(x, bottom), style.tick);
        if let Some(label) = tick.label {
            number(painter, style, g, x, &label, keep_clear);
        }
    }
}

/// The strip's span in ruler units from the column's start edge.
fn span(g: &Geometry, units: &RulerUnits) -> std::ops::RangeInclusive<f64> {
    let (a, b) = (g.at(g.strip.min.x), g.at(g.strip.max.x));
    (f64::from(a.min(b)) * units.per_doc)..=(f64::from(a.max(b)) * units.per_doc)
}

/// The box of a table column marker at screen x `x`: `MARKER_COLUMN` wide, the strip's depth
/// tall.
pub(super) fn column_marker(shapes: &Shapes, strip: Rect, x: f32) -> Rect {
    Rect::from_min_max(pos2(x - shapes.column / 2.0, strip.min.y), pos2(x + shapes.column / 2.0, strip.max.y))
}

/// A table column marker: a box with a grip of three short lines across it.
pub(super) fn column_grip(painter: &Painter, style: &RulerStyle, marker: Rect) {
    painter.rect(marker, 0.0, style.margin_shade, style.marker, egui::StrokeKind::Inside);
    let middle = marker.center();
    for k in GRIP_LINES {
        let at = pos2(middle.x + k * GRIP_PITCH, middle.y);
        painter.line_segment([at - vec2(0.0, GRIP_HALF), at + vec2(0.0, GRIP_HALF)], style.marker);
    }
}

fn number(painter: &Painter, style: &RulerStyle, g: &Geometry, x: f32, label: &str, keep_clear: &[Rect]) {
    let galley = painter.layout_no_wrap(label.to_owned(), style.font.clone(), style.label);
    let (align, at) = if g.right_to_left {
        (Align2::RIGHT_TOP, pos2(x - style.label_pad, g.strip.top()))
    } else {
        (Align2::LEFT_TOP, pos2(x + style.label_pad, g.strip.top()))
    };
    let rect = align.anchor_size(at, galley.size());
    if !keep_clear.iter().any(|clear| clear.intersects(rect)) {
        painter.galley(rect.min, galley, style.label);
    }
}
