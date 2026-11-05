//! The ruler's geometry and drawing.

use craft_ui::canvas::{RulerStyle, RulerUnits, crisp};
use craft_ui::theme::UiTokens;
use egui::{Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use super::TabAlign;

/// Ticks drawn at most (a ruler zoomed far out).
const MAX_TICKS: i64 = 20_000;

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

/// The marker sizes (`ruler.marker_*`).
#[derive(Clone, Copy, Debug)]
pub(super) struct Shapes {
    /// Half the width of a marker.
    half: f32,
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

impl Shapes {
    pub(super) fn of(tokens: &UiTokens) -> Self {
        let r = &tokens.component.ruler;
        Shapes { half: r.marker_half, tip: r.marker_tip, bar: r.marker_bar, stem: r.marker_stem, foot: r.marker_foot, selector: r.marker_selector }
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
                painter.circle_filled(pos2(x + foot * 0.7, base - self.stem * 0.7), stroke.width * 0.75, stroke.color);
            }
            TabAlign::Bar => {
                stem(self.stem * 1.4);
            }
        }
    }
}

/// The margin shade over the strip and the ruler's fill over the column.
pub(super) fn background(painter: &Painter, style: &RulerStyle, g: &Geometry) {
    painter.rect_filled(g.strip, 0.0, style.margin_shade);
    painter.rect_filled(g.column(), 0.0, style.fill);
}

/// The ticks and numbers along the strip from the column's start edge. Numbers that would run
/// into a `keep_clear` rect are left out.
pub(super) fn ticks(painter: &Painter, style: &RulerStyle, g: &Geometry, units: &RulerUnits, keep_clear: &[Rect]) {
    let (step, subdivisions) = units.step(f64::from(g.scale), style.step_min);
    let minor = step / f64::from(subdivisions);
    if !(minor.is_finite() && minor > 0.0 && units.per_doc.is_finite() && units.per_doc > 0.0) {
        return;
    }
    let (a, b) = (g.at(g.strip.min.x), g.at(g.strip.max.x));
    let (lo, hi) = (f64::from(a.min(b)) * units.per_doc, f64::from(a.max(b)) * units.per_doc);
    let subdivisions = i64::from(subdivisions);
    // One labelled step beyond each end: a number starts beside its tick.
    let (first, last) = ((lo / minor).floor() as i64 - subdivisions, (hi / minor).ceil() as i64 + subdivisions);
    if last - first > MAX_TICKS {
        return;
    }
    for m in first..=last {
        let x = g.x((m as f64 * minor / units.per_doc) as f32);
        let rem = m.rem_euclid(subdivisions);
        let major = rem == 0;
        let len = if major {
            style.size
        } else if subdivisions % 2 == 0 && rem == subdivisions / 2 {
            style.tick_mid
        } else {
            style.tick_minor
        };
        let bottom = g.strip.bottom();
        crisp(painter, pos2(x, bottom - len), pos2(x, bottom), style.tick);
        if major {
            number(painter, style, g, x, &units.tick_label((m / subdivisions) as f64 * step, step), keep_clear);
        }
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
