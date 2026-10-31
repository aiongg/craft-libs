//! The geometry of a gradient line on a canvas: where a colour stop sits on it, the location under
//! a point, the part under the pointer and the 45° snap of an end. Pure functions, so an app's own
//! canvas code and [`GradientOverlay`](crate::ui::gradient::GradientOverlay) agree on where things are.
//!
//! A stop's location `t` (0..=1) is the stop's own. With `reverse` it is drawn at `1 − t` from the
//! start (PhotoCraft's `u_of`), and every location read back from a point is turned back the same
//! way.

use egui::Pos2;
use kurbo::{Point, Vec2};

use crate::ui::gradient::GradientStops;

/// Midpoint diamonds show only between stops farther apart than this, as the stop bar's do.
pub const MIN_MIDPOINT_GAP: f32 = 0.001;

/// A gradient's line between its start and end handles, in document units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientLine {
    pub start: Point,
    pub end: Point,
}

/// What [`GradientOverlay`](crate::ui::gradient::GradientOverlay) edits: the line and the stops on it.
#[derive(Clone, Debug, PartialEq)]
pub struct LineGradient {
    pub line: GradientLine,
    pub stops: GradientStops,
}

/// The shape a gradient's style adds round the line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GradientExtent {
    /// Only the line (linear).
    #[default]
    None,
    /// A circle of the line's length round the start (radial, angle).
    Circle,
    /// A square with the end as one corner, centred on the start (diamond).
    Square,
    /// The line mirrored through the start, dashed (reflected).
    Mirror,
}

/// A part of the line a pointer or key takes. Indices are into the colour row's stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinePart {
    /// The line between the handles.
    Line,
    Start,
    End,
    Stop(usize),
    /// The diamond between stop `i` and stop `i + 1`.
    Midpoint(usize),
}

/// Where on the line, as a fraction from the start, location `t` is drawn.
fn along(t: f32, reverse: bool) -> f32 {
    if reverse { 1.0 - t } else { t }
}

/// The document point where location `t` sits on `line`.
pub fn at(line: &GradientLine, t: f32, reverse: bool) -> Point {
    line.start + (line.end - line.start) * f64::from(along(t, reverse))
}

/// The location of the point on `line` nearest `p`, clamped to 0..=1, and the distance from `p` to
/// that point (document units). A zero-length line projects to location 0.
pub fn project(line: &GradientLine, p: Point, reverse: bool) -> (f32, f64) {
    let d = line.end - line.start;
    let len_sq = d.hypot2();
    let u = if len_sq > 0.0 { ((p - line.start).dot(d) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
    let nearest = line.start + d * u;
    (along(u as f32, reverse), nearest.distance(p))
}

/// `moved` turned about `fixed` onto the nearest multiple of 45° counted from `base_deg`, keeping
/// its distance from `fixed`. Angles are those of the document's axes (y down).
pub fn snap45(fixed: Point, moved: Point, base_deg: f64) -> Point {
    let d = moved - fixed;
    let len = d.hypot();
    if len == 0.0 {
        return moved;
    }
    let base = base_deg.to_radians();
    let step = std::f64::consts::FRAC_PI_4;
    let angle = base + ((d.atan2() - base) / step).round() * step;
    fixed + Vec2::from_angle(angle) * len
}

/// The location of the midpoint diamond after colour stop `i`: its stop's location plus the
/// midpoint's share of the way to the next; `None` for the last stop and for stops no farther
/// apart than [`MIN_MIDPOINT_GAP`].
pub fn midpoint_location(stops: &GradientStops, i: usize) -> Option<f32> {
    let (a, b) = (stops.color.get(i)?, stops.color.get(i + 1)?);
    (b.location - a.location > MIN_MIDPOINT_GAP).then_some(a.location + a.midpoint * (b.location - a.location))
}

/// The screen point where location `t` sits on the screen line `start` to `end`.
pub fn at_screen(start: Pos2, end: Pos2, t: f32, reverse: bool) -> Pos2 {
    start + (end - start) * along(t, reverse)
}

/// The part of the screen line `start` to `end` under `pointer`, within `tolerance` points: an end
/// handle, the start handle, the nearest colour stop, the nearest midpoint, then the line. An end
/// handle takes a press over a stop under it. Of equal distances the part drawn last wins, the
/// later stop. Midpoints count only with `midpoints`.
pub fn hit(start: Pos2, end: Pos2, stops: &GradientStops, reverse: bool, midpoints: bool, pointer: Pos2, tolerance: f32) -> Option<LinePart> {
    let near = |p: Pos2| pointer.distance(p) <= tolerance;
    if near(end) {
        return Some(LinePart::End);
    }
    if near(start) {
        return Some(LinePart::Start);
    }
    // `min_by` keeps the first of equal items, so the later part is tried first.
    let nearest = |points: Vec<(usize, Pos2)>| {
        points
            .into_iter()
            .rev()
            .map(|(i, p)| (i, pointer.distance(p)))
            .filter(|&(_, d)| d <= tolerance)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    };
    let stop_points = stops.color.iter().enumerate().map(|(i, s)| (i, at_screen(start, end, s.location, reverse))).collect();
    if let Some(i) = nearest(stop_points) {
        return Some(LinePart::Stop(i));
    }
    if midpoints {
        let points = (0..stops.color.len()).filter_map(|i| midpoint_location(stops, i).map(|t| (i, at_screen(start, end, t, reverse)))).collect();
        if let Some(i) = nearest(points) {
            return Some(LinePart::Midpoint(i));
        }
    }
    (craft_ui::canvas::hit::segment(pointer, start, end) <= tolerance).then_some(LinePart::Line)
}

#[cfg(test)]
mod tests;
