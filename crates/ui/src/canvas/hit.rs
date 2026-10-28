//! Hit tests in screen space: the distance from the pointer to an overlay's screen geometry, in
//! points, which the overlay compares with its token tolerance (`guide.hit`, `handle.hit`). Lines
//! and bands in a rotated view are tested where they are drawn, which a hit rect can't follow.
//! When two overlays are in reach, the one drawn last takes the press.

use egui::Pos2;

/// The distance from `pointer` to point `p`.
pub fn point(pointer: Pos2, p: Pos2) -> f32 {
    pointer.distance(p)
}

/// The distance from `pointer` to the segment from `a` to `b`.
pub fn segment(pointer: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq <= f32::EPSILON {
        return pointer.distance(a);
    }
    let t = ((pointer - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    pointer.distance(a + ab * t)
}

/// The distance from `pointer` to the polyline through `points`, closed back to the first point
/// when `closed`. Infinite for no points.
pub fn polyline(pointer: Pos2, points: &[Pos2], closed: bool) -> f32 {
    match points {
        [] => f32::INFINITY,
        [p] => point(pointer, *p),
        _ => {
            let open = points.windows(2).map(|w| segment(pointer, w[0], w[1]));
            let last = (closed && points.len() > 2).then(|| segment(pointer, points[points.len() - 1], points[0]));
            open.chain(last).fold(f32::INFINITY, f32::min)
        }
    }
}
