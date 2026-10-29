//! [`Curve`]: the value a [`CurveEditor`](super::CurveEditor) edits, and the point rules it edits
//! by (PhotoCraft's `point_curve`, shared by Curves, Camera Raw and the contour editor).

/// A tone curve in levels (`0..=levels`, 255 for 8-bit tones): input across, output up.
#[derive(Clone, Debug)]
pub enum Curve {
    /// Control points `[input, output]`, sorted by input, whole levels, the first and last the
    /// curve's end points. `corners[i]` marks point `i` as a corner point (the contour editor): a
    /// corner joins both its neighbours with straight segments, and the smooth points between two
    /// corners take the host's interpolation (its sampler). A short `corners` reads as smooth
    /// points; [`Curve::smooth`] and [`Curve::from_parts`] make the value.
    Points { points: Vec<[f32; 2]>, corners: Vec<bool> },
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
            (Curve::Points { points: a, .. }, Curve::Points { points: b, .. }) => {
                a.len() == b.len()
                    && a.iter().flatten().zip(b.iter().flatten()).all(|(a, b)| same(a, b))
                    && self.corner_flags() == other.corner_flags()
            }
            (Curve::Map(a), Curve::Map(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b)),
            _ => false,
        }
    }
}

impl Curve {
    /// The unchanged curve: from (0, 0) to (`levels`, `levels`).
    pub fn identity(levels: f32) -> Curve {
        Curve::smooth(vec![[0.0, 0.0], [levels, levels]])
    }

    /// `points`, none of them a corner point.
    pub fn smooth(points: Vec<[f32; 2]>) -> Curve {
        Curve::from_parts(points, Vec::new())
    }

    /// `points` with `corners[i]` marking point `i` as a corner point. A short `corners` is
    /// completed with smooth points.
    pub fn from_parts(points: Vec<[f32; 2]>, mut corners: Vec<bool>) -> Curve {
        corners.resize(points.len(), false);
        Curve::Points { points, corners }
    }

    /// The control points, or `None` for a drawn curve.
    pub fn points(&self) -> Option<&[[f32; 2]]> {
        match self {
            Curve::Points { points, .. } => Some(points),
            Curve::Map(_) => None,
        }
    }

    /// Whether point `i` is a corner point.
    pub fn is_corner(&self, i: usize) -> bool {
        matches!(self, Curve::Points { corners, .. } if corners.get(i).copied().unwrap_or(false))
    }

    /// One flag per control point: whether it is a corner point.
    pub(crate) fn corner_flags(&self) -> Vec<bool> {
        match self {
            Curve::Points { points, corners } => (0..points.len()).map(|i| corners.get(i).copied().unwrap_or(false)).collect(),
            Curve::Map(_) => Vec::new(),
        }
    }

    /// Whether the curve leaves every level as it is, within half a level.
    pub fn is_identity(&self, levels: f32) -> bool {
        match self {
            Curve::Points { points, .. } => points.iter().all(|p| p[0] == p[1]),
            Curve::Map(map) => {
                let last = map.len().saturating_sub(1).max(1) as f32;
                map.iter().enumerate().all(|(k, o)| (o - k as f32 / last * levels).abs() <= 0.5)
            }
        }
    }

    /// A drawn curve of `levels + 1` outputs from `curve`: a [`Curve::Map`] as it is (resized),
    /// points through `samples` (given the corner flags).
    pub(crate) fn drawn(curve: &Curve, samples: impl FnOnce(&[[f32; 2]], &[bool]) -> Vec<f32>, levels: f32) -> Vec<f32> {
        let count = levels.max(1.0).round() as usize + 1;
        let mut map = match curve {
            Curve::Map(map) => map.clone(),
            Curve::Points { points, corners } => samples(points, corners),
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

    /// The output of every input level `0..=levels` through `points`, of which `corners` marks
    /// corner points: a segment with a corner at an end is straight, and each run of smooth points
    /// takes `smooth` (the host's interpolation of those points alone). Constant outside the end
    /// points.
    pub(crate) fn through(points: &[[f32; 2]], corners: &[bool], levels: f32, smooth: impl Fn(&[[f32; 2]]) -> Vec<f32>) -> Vec<f32> {
        if !corners.iter().any(|&corner| corner) {
            return smooth(points);
        }
        let count = levels.max(1.0).round() as usize + 1;
        let is_corner = |i: usize| corners.get(i).copied().unwrap_or(false);
        let mut out: Vec<f32> = (0..count).map(|k| straight_at(points, k as f32)).collect();
        let mut i = 0;
        while i < points.len() {
            if is_corner(i) {
                i += 1;
                continue;
            }
            let end = (i..points.len()).find(|&j| is_corner(j)).unwrap_or(points.len());
            if end - i >= 2 {
                let run = smooth(&points[i..end]);
                let (from, to) = (points[i][0].ceil().max(0.0) as usize, points[end - 1][0].floor().max(0.0) as usize);
                let to = to.min(count - 1);
                if from <= to {
                    for (slot, &o) in out[from..=to].iter_mut().zip(&run[from.min(run.len())..]) {
                        *slot = o;
                    }
                }
            }
            i = end;
        }
        out
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

/// `points` joined by straight segments at input `x`, constant outside the end points.
fn straight_at(points: &[[f32; 2]], x: f32) -> f32 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return x };
    if x <= first[0] {
        return first[1];
    }
    if x >= last[0] {
        return last[1];
    }
    let k = points.windows(2).position(|w| x < w[1][0]).unwrap_or(points.len() - 2);
    let ([x0, y0], [x1, y1]) = (points[k], points[k + 1]);
    if x1 > x0 { y0 + (x - x0) / (x1 - x0) * (y1 - y0) } else { y1 }
}

/// The corner flags after `before` became `after` by one insertion, one removal or moves: each flag
/// follows its point, a new point is smooth.
pub(crate) fn follow_corners(before: &[[f32; 2]], after: &[[f32; 2]], corners: &mut Vec<bool>) {
    corners.resize(before.len(), false);
    let first_diff = |a: &[[f32; 2]], b: &[[f32; 2]]| (0..a.len()).find(|&i| a.get(i) != b.get(i)).unwrap_or(a.len());
    if after.len() == before.len() + 1 {
        corners.insert(first_diff(after, before).min(corners.len()), false);
    } else if after.len() + 1 == before.len() {
        let i = first_diff(before, after);
        if i < corners.len() {
            corners.remove(i);
        }
    }
    corners.resize(after.len(), false);
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

/// A point named by its input and its neighbours' inputs, so an operation finds it on a curve
/// other edits changed since ([`find`](PointKey::find)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointKey {
    pub x: f32,
    /// The input of the point before it; `None` for the first point.
    pub prev_x: Option<f32>,
    /// The input of the point after it; `None` for the last point.
    pub next_x: Option<f32>,
}

impl PointKey {
    /// The key of point `i` of `points`.
    pub fn of(points: &[[f32; 2]], i: usize) -> Option<PointKey> {
        let x = points.get(i)?[0];
        Some(PointKey { x, prev_x: i.checked_sub(1).map(|j| points[j][0]), next_x: points.get(i + 1).map(|p| p[0]) })
    }

    /// The point at the key's input; else the one point between the points at its neighbours'
    /// inputs, which is the point the key names wherever it moved, as no point passes its
    /// neighbours. `None` when neither is there.
    pub fn find(&self, points: &[[f32; 2]]) -> Option<usize> {
        let at = |x: f32| points.iter().position(|p| p[0] == x);
        if let Some(i) = at(self.x) {
            return Some(i);
        }
        let from = match self.prev_x {
            Some(x) => at(x)? + 1,
            None => 0,
        };
        let to = match self.next_x {
            Some(x) => at(x)?,
            None => points.len(),
        };
        (to == from + 1).then_some(from)
    }
}

/// A one-shot edit of a curve's points, as the graph's keys and delete-clicks make it. A pencil
/// stroke and a drag are gestures and make none.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CurveOp {
    /// Moves the point by `by` levels, input then output.
    Nudge {
        at: PointKey,
        by: [f32; 2],
    },
    /// Adds a point halfway from point `after` to the next ([`Rules::between`]; across the widest
    /// gap for `None`).
    Insert {
        after: Option<PointKey>,
    },
    Delete(PointKey),
    /// Makes the point a corner point (`true`) or a smooth one.
    Corner(PointKey, bool),
}

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

    /// What `op` makes of `curve`, with `samples` the output of every input level through its
    /// points. `None` when its point isn't on `curve` (a drawn curve has none), or when it changes
    /// nothing. `selected` stays on the point it names, found by its key after the change; an
    /// Insert selects the new point and a Delete clears the selection. A curve left unchanged
    /// leaves `selected` too.
    pub fn apply(
        &self,
        op: &CurveOp,
        curve: &Curve,
        selected: &mut Option<usize>,
        samples: impl Fn(&[[f32; 2]], &[bool]) -> Vec<f32>,
    ) -> Option<Curve> {
        let mut points = curve.points()?.to_vec();
        let before = points.clone();
        let mut corners = curve.corner_flags();
        let kept = selected.and_then(|i| PointKey::of(&points, i));
        let selects = match op {
            CurveOp::Nudge { at, by } => {
                let i = at.find(&points)?;
                let p = points[i];
                if !self.move_to(&mut points, i, [p[0] + by[0], p[1] + by[1]]) {
                    return None;
                }
                kept.and_then(|key| key.find(&points))
            }
            CurveOp::Insert { after } => {
                let anchor = match after {
                    Some(key) => Some(key.find(&points)?),
                    None => None,
                };
                let v = Rules::between(&points, anchor, &samples(&points, &corners))?;
                let count = points.len();
                let i = self.insert(&mut points, v)?;
                // Within two levels of a neighbour: that point, nothing added.
                if points.len() == count {
                    return None;
                }
                Some(i)
            }
            CurveOp::Delete(at) => {
                let i = at.find(&points)?;
                if !Rules::delete(&mut points, i) {
                    return None;
                }
                None
            }
            CurveOp::Corner(at, on) => {
                let i = at.find(&points)?;
                corners.resize(points.len(), false);
                if corners[i] == *on {
                    return None;
                }
                corners[i] = *on;
                kept.and_then(|key| key.find(&points))
            }
        };
        *selected = selects;
        follow_corners(&before, &points, &mut corners);
        Some(Curve::from_parts(points, corners))
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

    /// The output of every input level on the identity curve, for [`Rules::apply`].
    fn identity(_: &[[f32; 2]], _: &[bool]) -> Vec<f32> {
        (0..256).map(|k| k as f32).collect()
    }

    /// An operation applies to the point it names on the curve of its turn, where the owner
    /// moved that point since: a nudge moves it, an insert goes halfway from it to the next and
    /// selects the new point, a delete removes it and clears the selection. A drawn curve takes
    /// none.
    #[test]
    fn an_operation_applies_to_the_point_it_names_on_the_curve_of_its_turn() {
        let made_on = [[0.0, 0.0], [100.0, 100.0], [200.0, 200.0], [255.0, 255.0]];
        let now = Curve::smooth(vec![[0.0, 0.0], [110.0, 100.0], [200.0, 200.0], [255.0, 255.0]]);
        let key = |i| PointKey::of(&made_on, i).unwrap();
        type Points = Vec<[f32; 2]>;
        let cases: [(CurveOp, Option<usize>, Points, Option<usize>); 5] = [
            (CurveOp::Nudge { at: key(1), by: [0.0, 1.0] }, Some(1), vec![[0.0, 0.0], [110.0, 101.0], [200.0, 200.0], [255.0, 255.0]], Some(1)),
            (
                CurveOp::Insert { after: Some(key(1)) },
                Some(0),
                vec![[0.0, 0.0], [110.0, 100.0], [155.0, 155.0], [200.0, 200.0], [255.0, 255.0]],
                Some(2),
            ),
            (CurveOp::Insert { after: None }, None, vec![[0.0, 0.0], [55.0, 55.0], [110.0, 100.0], [200.0, 200.0], [255.0, 255.0]], Some(1)),
            (CurveOp::Delete(key(1)), Some(2), vec![[0.0, 0.0], [200.0, 200.0], [255.0, 255.0]], None),
            (CurveOp::Nudge { at: key(2), by: [-100.0, 0.0] }, Some(1), vec![[0.0, 0.0], [110.0, 100.0], [111.0, 200.0], [255.0, 255.0]], Some(1)),
        ];
        for (op, mut selected, points, selects) in cases {
            let got = RULES.apply(&op, &now, &mut selected, identity);
            assert_eq!(got, Some(Curve::smooth(points)), "{op:?}");
            assert_eq!(selected, selects, "{op:?}");
        }
        let mut selected = Some(1);
        assert_eq!(RULES.apply(&CurveOp::Delete(key(1)), &Curve::Map(vec![0.0; 256]), &mut selected, identity), None, "a drawn curve");
        assert_eq!(
            RULES.apply(&CurveOp::Nudge { at: key(1), by: [0.0, 0.0] }, &now, &mut selected, identity),
            None,
            "a nudge that moves nothing changes nothing"
        );
        assert_eq!(selected, Some(1));
    }

    /// A key finds its point at its input, or between its neighbours wherever the owner moved it,
    /// end points included.
    #[test]
    fn a_key_finds_its_point_where_the_owner_moved_it_between_its_neighbours() {
        let made_on = [[0.0, 0.0], [100.0, 100.0], [200.0, 200.0], [255.0, 255.0]];
        let key = PointKey::of(&made_on, 1).unwrap();
        assert_eq!(key, PointKey { x: 100.0, prev_x: Some(0.0), next_x: Some(200.0) });
        assert_eq!(key.find(&made_on), Some(1));
        let moved = [[0.0, 0.0], [150.0, 90.0], [200.0, 200.0], [255.0, 255.0]];
        assert_eq!(key.find(&moved), Some(1), "moved between its neighbours");
        let ends = [[5.0, 10.0], [100.0, 100.0], [200.0, 200.0], [250.0, 255.0]];
        assert_eq!(PointKey::of(&made_on, 0).and_then(|k| k.find(&ends)), Some(0), "the first point moved");
        assert_eq!(PointKey::of(&made_on, 3).and_then(|k| k.find(&ends)), Some(3), "the last point moved");
        assert_eq!(PointKey::of(&made_on, 4), None);
    }

    /// A key whose point is gone, or can't be told from another, finds none, and its operation
    /// is dropped, leaving the curve and the selection.
    #[test]
    fn a_key_whose_point_cannot_be_found_finds_none_and_its_operation_is_dropped() {
        let made_on = [[0.0, 0.0], [100.0, 100.0], [200.0, 200.0], [255.0, 255.0]];
        let key = PointKey::of(&made_on, 1).unwrap();
        let deleted = [[0.0, 0.0], [200.0, 200.0], [255.0, 255.0]];
        assert_eq!(key.find(&deleted), None, "deleted");
        assert_eq!(
            key.find(&[[0.0, 0.0], [150.0, 90.0], [170.0, 120.0], [200.0, 200.0], [255.0, 255.0]]),
            None,
            "moved, and a point added beside it"
        );
        assert_eq!(key.find(&[[0.0, 0.0], [150.0, 90.0], [210.0, 200.0], [255.0, 255.0]]), None, "moved, and a neighbour too");
        let mut selected = Some(2);
        for op in [CurveOp::Nudge { at: key, by: [0.0, 1.0] }, CurveOp::Insert { after: Some(key) }, CurveOp::Delete(key)] {
            assert_eq!(RULES.apply(&op, &Curve::smooth(deleted.to_vec()), &mut selected, identity), None, "{op:?}");
        }
        assert_eq!(selected, Some(2), "a dropped operation leaves the selection");
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
        let from_points = Curve::drawn(&Curve::smooth(vec![[0.0, 10.0], [255.0, 10.0]]), |p, _| Curve::monotone(p, 255.0), 255.0);
        assert!(from_points.len() == 256 && from_points.iter().all(|&v| v == 10.0));
        let short = Curve::drawn(&Curve::Map(vec![5.0, 6.0]), |_, _| Vec::new(), 255.0);
        assert_eq!((short.len(), short[1], short[200]), (256, 6.0, 200.0), "a short map is completed with the identity");
    }

    /// A corner joins its neighbours with straight segments and leaves the segments between smooth
    /// points on the spline; a curve with no corner is the spline alone.
    #[test]
    fn a_corner_is_straight_to_both_neighbours_and_smooth_points_keep_the_spline() {
        let points = [[0.0, 0.0], [64.0, 40.0], [128.0, 200.0], [192.0, 210.0], [255.0, 255.0]];
        let spline = Curve::monotone(&points, 255.0);
        let smooth = |p: &[[f32; 2]]| Curve::monotone(p, 255.0);
        assert_eq!(Curve::through(&points, &[false; 5], 255.0, smooth), spline);
        let corners = [false, false, true, false, false];
        let shaped = Curve::through(&points, &corners, 255.0, smooth);
        assert!((shaped[96] - 120.0).abs() < 1e-3, "halfway between (64, 40) and the corner (128, 200): {}", shaped[96]);
        assert!((shaped[160] - 205.0).abs() < 1e-3, "halfway from the corner to (192, 210): {}", shaped[160]);
        assert!((shaped[128] - 200.0).abs() < 1e-3 && (shaped[64] - 40.0).abs() < 1e-3, "through its points");
        assert_eq!(shaped.len(), 256);
        let alone = Curve::through(&points[..2], &[false; 2], 255.0, smooth);
        assert_eq!(&shaped[..=64], &alone[..=64], "the two smooth points before it are their own spline");
    }

    /// Toggling flips the point's flag, a point added is smooth, and a point removed takes its
    /// flag; no corner left is a curve with no corner.
    #[test]
    fn corner_flags_follow_their_points_through_toggles_inserts_and_deletes() {
        let made_on = [[0.0, 0.0], [100.0, 100.0], [200.0, 200.0], [255.0, 255.0]];
        let key = |i| PointKey::of(&made_on, i).unwrap();
        let plain = Curve::smooth(made_on.to_vec());
        let mut selected = Some(1);
        let cornered = RULES.apply(&CurveOp::Corner(key(2), true), &plain, &mut selected, identity).unwrap();
        assert_eq!(cornered, Curve::from_parts(made_on.to_vec(), vec![false, false, true, false]));
        assert_eq!(selected, Some(1), "a corner change keeps the selection");
        let inserted = RULES.apply(&CurveOp::Insert { after: Some(key(0)) }, &cornered, &mut selected, identity).unwrap();
        assert_eq!(inserted.corner_flags(), [false, false, false, true, false], "the new point is smooth, the flag stays on its point");
        let deleted = RULES.apply(&CurveOp::Delete(key(1)), &cornered, &mut selected, identity).unwrap();
        assert_eq!(deleted.corner_flags(), [false, true, false]);
        let gone = RULES.apply(&CurveOp::Delete(key(2)), &cornered, &mut selected, identity).unwrap();
        assert_eq!(gone, Curve::smooth(vec![[0.0, 0.0], [100.0, 100.0], [255.0, 255.0]]), "the last corner removed");
        let back = RULES.apply(&CurveOp::Corner(key(2), false), &cornered, &mut selected, identity).unwrap();
        assert_eq!(back, plain);
        assert_eq!(RULES.apply(&CurveOp::Corner(key(1), true), &Curve::Map(vec![0.0; 256]), &mut selected, identity), None);
    }
}
