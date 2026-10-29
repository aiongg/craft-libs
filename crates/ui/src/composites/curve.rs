//! [`Curve`]: the value a [`CurveEditor`](super::CurveEditor) edits, and the point rules it edits
//! by (PhotoCraft's `point_curve`, shared by Curves, Camera Raw and the contour editor).

use crate::edit::parts::Rebase;

/// A tone curve in levels (`0..=levels`, 255 for 8-bit tones): input across, output up.
#[derive(Clone, Debug)]
pub enum Curve {
    /// Control points `[input, output]`, sorted by input, whole levels, the first and last the
    /// curve's end points. The host interpolates them (its sampler).
    Points(Vec<[f32; 2]>),
    /// A drawn curve (pencil): one output level per input level, drawn as straight segments.
    Map(Vec<f32>),
}

/// Curves are equal when every level is, a NaN equal to another: the host's points, and the
/// levels a drawn curve took from the host's sampler, may hold one, and the edit machinery
/// compares values to tell a change from none ([`Gesture`](crate::edit::Gesture)).
impl PartialEq for Curve {
    fn eq(&self, other: &Self) -> bool {
        let same = |a: &f32, b: &f32| a == b || (a.is_nan() && b.is_nan());
        match (self, other) {
            (Curve::Points(a), Curve::Points(b)) => a.len() == b.len() && a.iter().flatten().zip(b.iter().flatten()).all(|(a, b)| same(a, b)),
            (Curve::Map(a), Curve::Map(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b)),
            _ => false,
        }
    }
}

/// A graph's event moves, adds or removes points from the curve it was drawn from: a held one
/// moves the levels it changed, or adds or removes its point, on the curve of its turn
/// ([`edit::parts`](crate::edit::parts)). When that leaves the inputs out of order, or the curve
/// changed in any other way (a drawn curve, points the host added), the event's curve stands.
impl Rebase for Curve {
    fn rebase(self, base: &Curve, now: &Curve) -> Curve {
        if base == now {
            return self;
        }
        let (Curve::Points(points), Curve::Points(base), Curve::Points(now)) = (&self, base, now) else {
            return self;
        };
        if base.len() != now.len() {
            return self;
        }
        let level = |a: f32, b: f32| a == b || (a.is_nan() && b.is_nan());
        let same = |a: &[f32; 2], b: &[f32; 2]| level(a[0], b[0]) && level(a[1], b[1]);
        // The first point the event changed, and whether the rest is the base's from there on.
        let first = points.iter().zip(base).position(|(p, b)| !same(p, b)).unwrap_or(points.len().min(base.len()));
        let rebased = if points.len() == base.len() {
            points.iter().zip(base).zip(now).map(|((p, b), n)| [0, 1].map(|c| if level(p[c], b[c]) { n[c] } else { p[c] })).collect()
        } else if points.len() == base.len() + 1 && points[first + 1..].iter().zip(&base[first..]).all(|(p, b)| same(p, b)) {
            let mut rebased = now.clone();
            rebased.insert(first, points[first]);
            rebased
        } else if points.len() + 1 == base.len() && points[first..].iter().zip(&base[first + 1..]).all(|(p, b)| same(p, b)) {
            let mut rebased = now.clone();
            rebased.remove(first);
            rebased
        } else {
            return self;
        };
        match rebased.windows(2).all(|w: &[[f32; 2]]| w[0][0] < w[1][0]) {
            true => Curve::Points(rebased),
            false => self,
        }
    }
}

impl Curve {
    /// The unchanged curve: from (0, 0) to (`levels`, `levels`).
    pub fn identity(levels: f32) -> Curve {
        Curve::Points(vec![[0.0, 0.0], [levels, levels]])
    }

    /// The control points, or `None` for a drawn curve.
    pub fn points(&self) -> Option<&[[f32; 2]]> {
        match self {
            Curve::Points(points) => Some(points),
            Curve::Map(_) => None,
        }
    }

    /// Whether the curve leaves every level as it is, within half a level.
    pub fn is_identity(&self, levels: f32) -> bool {
        match self {
            Curve::Points(points) => points.iter().all(|p| p[0] == p[1]),
            Curve::Map(map) => {
                let last = map.len().saturating_sub(1).max(1) as f32;
                map.iter().enumerate().all(|(k, o)| (o - k as f32 / last * levels).abs() <= 0.5)
            }
        }
    }

    /// A drawn curve of `levels + 1` outputs from `curve`: a [`Curve::Map`] as it is (resized),
    /// points through `samples`.
    pub(crate) fn drawn(curve: &Curve, samples: impl FnOnce(&[[f32; 2]]) -> Vec<f32>, levels: f32) -> Vec<f32> {
        let count = levels.max(1.0).round() as usize + 1;
        let mut map = match curve {
            Curve::Map(map) => map.clone(),
            Curve::Points(points) => samples(points),
        };
        let from = map.len();
        map.resize(count, 0.0);
        for (k, o) in map.iter_mut().enumerate().skip(from) {
            *o = k as f32;
        }
        map
    }

    /// Draws a straight line from `from` to `to` (levels) into `map`, one output per input level,
    /// whole levels (PhotoCraft's `pencil_line`). Returns whether `map` changed.
    pub(crate) fn pencil_line(map: &mut [f32], from: [f32; 2], to: [f32; 2], levels: f32) -> bool {
        if !from.iter().chain(to.iter()).all(|v| v.is_finite()) || map.is_empty() {
            return false;
        }
        let clamp = |q: [f32; 2]| [q[0].round().clamp(0.0, levels), q[1].round().clamp(0.0, levels)];
        let (a, b) = (clamp(from), clamp(to));
        let (lo, hi) = if a[0] <= b[0] { (a, b) } else { (b, a) };
        let mut changed = false;
        for x in lo[0] as usize..=hi[0] as usize {
            let t = if hi[0] > lo[0] { (x as f32 - lo[0]) / (hi[0] - lo[0]) } else { 1.0 };
            let y = (lo[1] + t * (hi[1] - lo[1])).round();
            if let Some(slot) = map.get_mut(x)
                && *slot != y
            {
                *slot = y;
                changed = true;
            }
        }
        changed
    }

    /// The output level of every input level `0..=levels` through `points` on a monotone cubic
    /// (Fritsch–Carlson), which never overshoots between points; constant outside the end
    /// points. Camera Raw's interpolation, and the editor's default sampler.
    pub fn monotone(points: &[[f32; 2]], levels: f32) -> Vec<f32> {
        let count = levels.max(1.0).round() as usize + 1;
        let points: Vec<[f64; 2]> = points.iter().filter(|p| p.iter().all(|v| v.is_finite())).map(|p| [f64::from(p[0]), f64::from(p[1])]).collect();
        let at = |x: f64| -> f64 {
            match points.as_slice() {
                [] => x,
                [only] => only[1],
                _ => monotone_at(&points, x),
            }
        };
        (0..count).map(|k| at(k as f64).clamp(0.0, f64::from(levels)) as f32).collect()
    }
}

/// The monotone cubic through `points` (two or more, inputs ascending) at `x`.
fn monotone_at(points: &[[f64; 2]], x: f64) -> f64 {
    let n = points.len();
    let (first, last) = (points[0], points[n - 1]);
    if x <= first[0] {
        return first[1];
    }
    if x >= last[0] {
        return last[1];
    }
    let secant = |k: usize| {
        let dx = points[k + 1][0] - points[k][0];
        if dx > 0.0 { (points[k + 1][1] - points[k][1]) / dx } else { 0.0 }
    };
    let d: Vec<f64> = (0..n - 1).map(secant).collect();
    let mut m: Vec<f64> = (0..n)
        .map(|k| match k {
            0 => d[0],
            k if k == n - 1 => d[n - 2],
            k if d[k - 1] * d[k] <= 0.0 => 0.0,
            k => (d[k - 1] + d[k]) / 2.0,
        })
        .collect();
    for k in 0..n - 1 {
        if d[k] == 0.0 {
            m[k] = 0.0;
            m[k + 1] = 0.0;
            continue;
        }
        let (a, b) = (m[k] / d[k], m[k + 1] / d[k]);
        let s = a * a + b * b;
        if s > 9.0 {
            let t = 3.0 / s.sqrt();
            m[k] = t * a * d[k];
            m[k + 1] = t * b * d[k];
        }
    }
    let k = points.windows(2).position(|w| x < w[1][0]).unwrap_or(n - 2);
    let ([x0, y0], [x1, y1]) = (points[k], points[k + 1]);
    let h = x1 - x0;
    let t = (x - x0) / h;
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * y0 + (t3 - 2.0 * t2 + t) * h * m[k] + (-2.0 * t3 + 3.0 * t2) * y1 + (t3 - t2) * h * m[k + 1]
}

/// Within this many input levels of an existing point, a new point is that point (PhotoCraft's
/// `MIN_GAP`).
pub(crate) const SAME_POINT: f32 = 2.0;

/// The point rules of a curve with `levels` levels and at most `max_points` points.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rules {
    pub levels: f32,
    pub max_points: usize,
}

impl Rules {
    /// Adds a point at `v`, kept sorted, and returns its index. A point within
    /// [`SAME_POINT`] input levels of `v` is returned instead; a full curve takes none.
    pub fn insert(&self, points: &mut Vec<[f32; 2]>, v: [f32; 2]) -> Option<usize> {
        if !v.iter().all(|x| x.is_finite()) {
            return None;
        }
        let v = [v[0].round().clamp(0.0, self.levels), v[1].round().clamp(0.0, self.levels)];
        if let Some(i) = points.iter().position(|q| (q[0] - v[0]).abs() < SAME_POINT) {
            return Some(i);
        }
        if points.len() >= self.max_points {
            return None;
        }
        let i = points.iter().position(|q| q[0] > v[0]).unwrap_or(points.len());
        points.insert(i, v);
        Some(i)
    }

    /// Whether point `i` may go: the end points stay, and a curve keeps two points.
    pub fn can_delete(points: &[[f32; 2]], i: usize) -> bool {
        points.len() > 2 && i > 0 && i < points.len() - 1
    }

    /// Removes point `i`; false when it may not go.
    pub fn delete(points: &mut Vec<[f32; 2]>, i: usize) -> bool {
        if !Rules::can_delete(points, i) {
            return false;
        }
        points.remove(i);
        true
    }

    /// Moves point `i` to `v` in whole levels, strictly between its neighbours' inputs; false
    /// when nothing changed.
    pub fn move_to(&self, points: &mut [[f32; 2]], i: usize, v: [f32; 2]) -> bool {
        let Some(old) = points.get(i).copied() else { return false };
        if !v.iter().chain(old.iter()).all(|x| x.is_finite()) {
            return false;
        }
        let lo = i.checked_sub(1).and_then(|j| points.get(j)).map_or(0.0, |p| p[0] + 1.0);
        let hi = points.get(i + 1).map_or(self.levels, |p| p[0] - 1.0);
        let new = [v[0].round().clamp(lo, hi.max(lo)), v[1].round().clamp(0.0, self.levels)];
        points[i] = new;
        new != old
    }

    /// Where a point added from the keyboard goes: halfway between point `selected` and the next
    /// (the one before for the last point), or across the widest gap when nothing is selected,
    /// on the curve `samples` draws (one output per input level).
    pub fn between(points: &[[f32; 2]], selected: Option<usize>, samples: &[f32]) -> Option<[f32; 2]> {
        let gap = match selected.filter(|&i| i < points.len()) {
            Some(i) if i + 1 < points.len() => i,
            Some(i) => i.checked_sub(1)?,
            None => (0..points.len().checked_sub(1)?).max_by(|&a, &b| {
                let width = |k: usize| points[k + 1][0] - points[k][0];
                // The first of equal gaps.
                width(a).total_cmp(&width(b)).then(b.cmp(&a))
            })?,
        };
        let x = ((points[gap][0] + points[gap + 1][0]) / 2.0).round();
        let y = samples.get(x as usize).copied().unwrap_or((points[gap][1] + points[gap + 1][1]) / 2.0);
        Some([x, y.round()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: Rules = Rules { levels: 255.0, max_points: 16 };

    #[test]
    fn the_monotone_cubic_passes_through_its_points_and_never_overshoots() {
        let points = [[0.0, 0.0], [64.0, 40.0], [128.0, 200.0], [192.0, 210.0], [255.0, 255.0]];
        let samples = Curve::monotone(&points, 255.0);
        assert_eq!(samples.len(), 256);
        for p in points {
            assert!((samples[p[0] as usize] - p[1]).abs() < 1e-3, "{p:?}: {}", samples[p[0] as usize]);
        }
        assert!(samples.windows(2).all(|w| w[1] >= w[0] - 1e-4), "rising points give a rising curve");
        let flat = Curve::monotone(&[[10.0, 50.0], [200.0, 50.0]], 255.0);
        assert!(flat.iter().all(|&v| v == 50.0), "constant between and outside equal end points");
    }

    #[test]
    fn points_keep_their_rules() {
        let mut points = vec![[0.0, 0.0], [255.0, 255.0]];
        assert_eq!(RULES.insert(&mut points, [100.4, 80.6]), Some(1));
        assert_eq!(points[1], [100.0, 81.0], "whole levels");
        assert_eq!(RULES.insert(&mut points, [101.0, 10.0]), Some(1), "within two levels is the same point");
        assert_eq!(RULES.insert(&mut points, [f32::NAN, 1.0]), None);
        assert!(!RULES.move_to(&mut points, 1, [f32::INFINITY, 1.0]));
        assert!(RULES.move_to(&mut points, 1, [300.0, -5.0]));
        assert_eq!(points[1], [254.0, 0.0], "strictly before the next input");
        assert!(!Rules::delete(&mut points, 0) && !Rules::delete(&mut points, 2), "end points stay");
        assert!(Rules::delete(&mut points, 1));
        assert!(!Rules::delete(&mut points, 1), "two points stay");
    }

    #[test]
    fn a_keyboard_point_goes_halfway_to_the_next_or_into_the_widest_gap() {
        let points = [[0.0, 0.0], [50.0, 60.0], [255.0, 255.0]];
        let samples: Vec<f32> = (0..256).map(|k| k as f32).collect();
        assert_eq!(Rules::between(&points, Some(0), &samples), Some([25.0, 25.0]));
        assert_eq!(Rules::between(&points, Some(2), &samples), Some([153.0, 153.0]), "the last point looks back");
        assert_eq!(Rules::between(&points, None, &samples), Some([153.0, 153.0]));
    }

    #[test]
    fn a_pencil_line_writes_one_output_per_level_and_a_vertical_one_the_last_value() {
        let mut map: Vec<f32> = (0..256).map(|k| k as f32).collect();
        assert!(Curve::pencil_line(&mut map, [10.0, 100.0], [20.0, 200.0], 255.0));
        assert_eq!((map[10], map[15], map[20]), (100.0, 150.0, 200.0));
        assert_eq!((map[9], map[21]), (9.0, 21.0), "outside the line stays");
        assert!(Curve::pencil_line(&mut map, [30.0, 5.0], [30.0, 90.0], 255.0));
        assert_eq!(map[30], 90.0);
        assert!(Curve::pencil_line(&mut map, [250.0, 0.0], [300.0, f32::MAX], 255.0), "clamped to the levels");
        assert_eq!(map[255], 255.0);
        assert!(!Curve::pencil_line(&mut map, [f32::NAN, 0.0], [1.0, 1.0], 255.0));
    }

    #[test]
    fn a_drawn_curve_has_one_output_per_level() {
        let from_points = Curve::drawn(&Curve::Points(vec![[0.0, 10.0], [255.0, 10.0]]), |p| Curve::monotone(p, 255.0), 255.0);
        assert!(from_points.len() == 256 && from_points.iter().all(|&v| v == 10.0));
        let short = Curve::drawn(&Curve::Map(vec![5.0, 6.0]), |_| Vec::new(), 255.0);
        assert_eq!((short.len(), short[1], short[200]), (256, 6.0, 200.0), "a short map is completed with the identity");
    }
}
