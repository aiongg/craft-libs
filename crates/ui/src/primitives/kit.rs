//! The content-paint kit (5.4): the recurring shapes of document and plot painting, read from tokens, so an
//! allowlisted content painter draws them through one implementation. The shape functions are pure; the
//! `ContentPainterCx` methods add their shapes to its painter.

use super::content::ContentPainterCx;
use super::draw::PaintShape;
use super::rule::{Rule, RuleKind};
use super::surface::Surface;
use crate::style::{Corners, SurfaceKind, UiTokens, rgba_color, rule_look, surface_style};
use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};
use std::f32::consts::TAU;

/// Which edge of a rect a tick row sits on, and so which way its marks point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TickEdge {
    /// Marks rise from the bottom edge, positions run along x.
    Bottom,
    /// Marks hang from the top edge, positions run along x.
    Top,
    /// Marks grow from the left edge, positions run along y.
    Left,
    /// Marks grow from the right edge, positions run along y.
    Right,
}

/// One tick: where it sits along its edge and whether it is a major one.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Tick {
    /// The position along the edge, in points from the rect's origin on that axis.
    pub at: f32,
    /// Major ticks are longer than minor ones.
    pub major: bool,
}

impl Tick {
    /// A tick at `at`.
    pub const fn new(at: f32, major: bool) -> Self {
        Tick { at, major }
    }
}

/// The transparency checkerboard over `rect`, drawn where `clip` shows it: the light colour fills it and the dark
/// cells sit on top, from the rect's top-left corner, with cells of `size.checker`.
pub fn checker(tokens: &UiTokens, rect: Rect, clip: Rect) -> Vec<Shape> {
    let visible = rect.intersect(clip);
    if visible.width() <= 0.0 || visible.height() <= 0.0 {
        return Vec::new();
    }
    let side = tokens.size.checker.max(f32::EPSILON);
    let (light, dark) = (rgba_color(tokens.color.overlay.checker_light), rgba_color(tokens.color.overlay.checker_dark));
    let mut shapes = vec![Shape::rect_filled(visible, 0.0, light)];
    let cells = |from: f32, to: f32, origin: f32| {
        let at = |edge: f32, round: fn(f32) -> f32| round((edge - origin) / side).max(0.0) as usize;
        at(from, f32::floor)..at(to, f32::ceil)
    };
    for row in cells(visible.top(), visible.bottom(), rect.top()) {
        for column in cells(visible.left(), visible.right(), rect.left()).filter(|column| (column + row) % 2 == 0) {
            let min = rect.min + vec2(column as f32 * side, row as f32 * side);
            let cell = Rect::from_min_size(min, Vec2::splat(side)).intersect(visible);
            shapes.push(Shape::rect_filled(cell, 0.0, dark));
        }
    }
    shapes
}

/// The grid lines over `rect` that `clip` shows, every `spacing` points from the rect's top-left corner, as grid
/// rules. Lines closer than a device pixel are drawn a pixel apart.
pub fn grid_lines(tokens: &UiTokens, rect: Rect, spacing: Vec2, clip: Rect, pixels_per_point: f32) -> Vec<Shape> {
    let visible = rect.intersect(clip);
    let look = rule_look(tokens, RuleKind::Grid);
    let mut shapes = Vec::new();
    if visible.width() < 0.0 || visible.height() < 0.0 {
        return shapes;
    }
    let pixel = 1.0 / pixels_per_point.max(f32::EPSILON);
    if spacing.x > 0.0 {
        let step = spacing.x.max(pixel);
        let mut n = ((visible.left() - rect.left()) / step).ceil().max(0.0);
        while rect.left() + n * step <= visible.right() {
            let x = rect.left() + n * step;
            shapes.extend(Rule::shapes(&look, pos2(x, visible.top()), pos2(x, visible.bottom()), pixels_per_point));
            n += 1.0;
        }
    }
    if spacing.y > 0.0 {
        let step = spacing.y.max(pixel);
        let mut n = ((visible.top() - rect.top()) / step).ceil().max(0.0);
        while rect.top() + n * step <= visible.bottom() {
            let y = rect.top() + n * step;
            shapes.extend(Rule::shapes(&look, pos2(visible.left(), y), pos2(visible.right(), y), pixels_per_point));
            n += 1.0;
        }
    }
    shapes
}

/// The tick marks of `ticks` on one edge of `rect` that `clip` shows: minor ones `gap.tight` long, major ones
/// `gap.inline`.
pub fn ticks(tokens: &UiTokens, rect: Rect, edge: TickEdge, ticks: &[Tick], clip: Rect, pixels_per_point: f32) -> Vec<Shape> {
    let look = rule_look(tokens, RuleKind::Edge);
    let (minor, major) = (tokens.space.gap.tight, tokens.space.gap.inline);
    ticks
        .iter()
        .filter_map(|tick| {
            let length = if tick.major { major } else { minor };
            let (from, to) = match edge {
                TickEdge::Bottom => (pos2(rect.left() + tick.at, rect.bottom()), pos2(rect.left() + tick.at, rect.bottom() - length)),
                TickEdge::Top => (pos2(rect.left() + tick.at, rect.top()), pos2(rect.left() + tick.at, rect.top() + length)),
                TickEdge::Left => (pos2(rect.left(), rect.top() + tick.at), pos2(rect.left() + length, rect.top() + tick.at)),
                TickEdge::Right => (pos2(rect.right(), rect.top() + tick.at), pos2(rect.right() - length, rect.top() + tick.at)),
            };
            clip.intersects(Rect::from_two_pos(from, to)).then(|| Rule::shapes(&look, from, to, pixels_per_point))
        })
        .flatten()
        .collect()
}

/// The line that marks a shape on any image: a `thick` halo under a `medium` line, in the overlay colours.
pub fn outline(tokens: &UiTokens, shape: &PaintShape) -> Vec<Shape> {
    let (halo, line) = (rgba_color(tokens.color.overlay.halo), rgba_color(tokens.color.overlay.line));
    let (thick, medium) = (tokens.stroke.thick, tokens.stroke.medium);
    let mut shapes = stroked(shape, Stroke::new(thick, halo));
    shapes.extend(stroked(shape, Stroke::new(medium, line)));
    shapes
}

/// Marching ants along `points` (closed when `closed`): dashes of the marquee colour with the gaps filled in the
/// halo colour, shifted by `phase` points along the path.
pub fn marching_ants(tokens: &UiTokens, points: &[Pos2], closed: bool, phase: f32) -> Vec<Shape> {
    let (light, dark) = (rgba_color(tokens.color.overlay.marquee), rgba_color(tokens.color.overlay.halo));
    let (dash, gap) = (tokens.stroke.dash, tokens.stroke.dash_gap);
    let mut path = points.to_vec();
    if closed && let Some(first) = path.first().copied() {
        path.push(first);
    }
    let width = tokens.stroke.medium;
    // The dark pass swaps dash and gap lengths and starts one dash later, so its dashes are the light pass's gaps.
    let mut shapes = Shape::dashed_line_with_offset(&path, Stroke::new(width, dark), &[gap], &[dash], phase + gap);
    shapes.extend(Shape::dashed_line_with_offset(&path, Stroke::new(width, light), &[dash], &[gap], phase));
    shapes
}

/// The insertion mark of a drop from `from` to `to`.
pub fn drop_mark(tokens: &UiTokens, from: Pos2, to: Pos2, pixels_per_point: f32) -> Vec<Shape> {
    Rule::shapes(&rule_look(tokens, RuleKind::DropMark), from, to, pixels_per_point)
}

/// The points of the ellipse with centre `center` and conjugate half-axes `axes` (a rotated or skewed circle is
/// a pair of axes), counter-clockwise from the first axis, with a count that keeps the chord error under a
/// fraction of a point at any size. The first point is not repeated at the end.
pub fn ellipse(center: Pos2, axes: [Vec2; 2]) -> Vec<Pos2> {
    // Chord error of a regular n-gon on radius r is r (1 - cos(pi / n)); n = pi / acos(1 - e / r) keeps it under e.
    const MAX_CHORD_ERROR: f32 = 0.1;
    const MIN_POINTS: usize = 16;
    const MAX_POINTS: usize = 720;
    let radius = axes[0].length().max(axes[1].length());
    let count = if radius <= MAX_CHORD_ERROR {
        MIN_POINTS
    } else {
        ((std::f32::consts::PI / (1.0 - MAX_CHORD_ERROR / radius).acos()).ceil() as usize).clamp(MIN_POINTS, MAX_POINTS)
    };
    (0..count)
        .map(|i| {
            let angle = TAU * i as f32 / count as f32;
            center + axes[0] * angle.cos() + axes[1] * angle.sin()
        })
        .collect()
}

fn stroked(shape: &PaintShape, stroke: Stroke) -> Vec<Shape> {
    match shape {
        PaintShape::Rect(rect, radius) => vec![Shape::rect_stroke(*rect, *radius, stroke, egui::StrokeKind::Middle)],
        PaintShape::Circle(center, radius) => vec![Shape::circle_stroke(*center, *radius, stroke)],
        PaintShape::Polygon(points) => vec![Shape::closed_line(points.clone(), stroke)],
        PaintShape::Path(points) => vec![Shape::line(points.clone(), stroke)],
    }
}

impl ContentPainterCx<'_> {
    fn ppp(&self) -> f32 {
        self.painter.pixels_per_point()
    }

    /// Paints the transparency checkerboard over `rect`.
    pub fn checker(&self, rect: Rect) {
        self.painter.extend(checker(self.tokens, rect, self.painter.clip_rect()));
    }

    /// Paints the well of a plot over `rect` and returns the rect inside its edge, where the plot draws.
    pub fn plot_frame(&self, rect: Rect) -> Rect {
        let style = surface_style(self.tokens, SurfaceKind::Well);
        self.painter.extend(Surface::shapes(&style, None, rect, Corners::None, self.direction, self.ppp()));
        rect.shrink(self.tokens.stroke.thin)
    }

    /// Paints grid lines every `spacing` points over `rect`.
    pub fn grid_lines(&self, rect: Rect, spacing: Vec2) {
        self.painter.extend(grid_lines(self.tokens, rect, spacing, self.painter.clip_rect(), self.ppp()));
    }

    /// Paints tick marks on one edge of `rect`.
    pub fn ticks(&self, rect: Rect, edge: TickEdge, marks: &[Tick]) {
        self.painter.extend(ticks(self.tokens, rect, edge, marks, self.painter.clip_rect(), self.ppp()));
    }

    /// Paints a halo-and-line outline of `shape`.
    pub fn outline(&self, shape: &PaintShape) {
        self.painter.extend(outline(self.tokens, shape));
    }

    /// Paints marching ants along `points`.
    pub fn marching_ants(&self, points: &[Pos2], closed: bool, phase: f32) {
        self.painter.extend(marching_ants(self.tokens, points, closed, phase));
    }

    /// Paints the insertion mark of a drop.
    pub fn drop_mark(&self, from: Pos2, to: Pos2) {
        self.painter.extend(drop_mark(self.tokens, from, to, self.ppp()));
    }

    /// The colour a plot reads grid and tick ink in.
    pub fn grid_color(&self) -> Color32 {
        rgba_color(self.tokens.color.overlay.grid)
    }
}
