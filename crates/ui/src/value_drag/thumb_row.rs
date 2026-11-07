//! `ThumbRow`: triangle thumbs on one row, each setting a value of its own, as the input and output rows of a levels
//! editor do. One leaf with a focusable thumb part per value. A drag or press anywhere on the row moves the thumb it
//! picks; arrows, the wheel and AccessKit step the focused thumb. The row's rules (order, wrap, split halves) are
//! `RowRules`.

use super::words::ValueDragWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Numeric, PaintUi, PartsCx, Pointer as _,
    Remember as _, Role, Styled as _, WordSetCx as _,
};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::runtime::access::Name;
use crate::runtime::focus::{ActivateKeys, FocusShape, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, DataColor, FocusLook, StyleColor, StyleCx};
use craft_behavior::nav::{NavShape, StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::part::PartRef;
use craft_behavior::{HitLevel, HitPriority, Metrics, MoveMods, NavMove, PartKey, PressPhase, PressState, Roving, Sense, ValueDrag};
use craft_theme::{Rgba, Stateful, States};
use egui::{Pos2, Rect, Vec2, pos2};
use std::ops::RangeInclusive;

#[cfg(feature = "testing")]
mod stories;

/// The row-wide part: a press off the thumbs lands here.
pub(crate) const ROW: PartKey = PartKey::new("row");
/// One thumb, `THUMB.at(i)`.
pub(crate) const THUMB: PartKey = PartKey::new("thumb");

/// The room a row asks for along its axis, in icon sides.
const LENGTH_IN_ICONS: f32 = 8.0;
/// The share of the row's height a thumb takes; the rest leaves room for the ring above and below.
const THUMB_SHARE: f32 = 0.8;
/// A thumb's width as a share of its height.
const THUMB_ASPECT: f32 = 0.8;
/// How many small steps a large step is.
const LARGE_STEPS: f64 = 10.0;
/// An arrow press steps this share of the range when the row has no step.
const STEP_SHARE: f64 = 0.01;

/// The style of a thumb row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ThumbRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// A thumb's fill when its `Thumb` gives no face.
    face: Stateful<StyleColor>,
    /// A thumb's edge.
    edge: Stateful<StyleColor>,
    /// The ring round a thumb that is hovered or pressed.
    ring: Stateful<StyleColor>,
    /// How far the ring reaches past the thumb's edge.
    ring_width: f32,
    /// How far from a thumb's centre a press grabs it.
    hit: f32,
}

/// How a thumb's value is kept among the row's other values.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Order {
    /// Anywhere in its range: it passes the other thumbs.
    Free,
    /// At least `gap` above the value of thumb `below` and below that of thumb `above`. A drag stops there; it does
    /// not push the other thumb.
    #[non_exhaustive]
    Between {
        /// The thumb whose value this one stays above, if any.
        below: Option<usize>,
        /// The thumb whose value this one stays below, if any.
        above: Option<usize>,
        /// The least distance kept from each of them, in the row's values.
        gap: f64,
    },
}

impl Order {
    /// Between thumbs `below` and `above`, `gap` apart.
    pub const fn between(below: Option<usize>, above: Option<usize>, gap: f64) -> Self {
        Order::Between { below, above, gap }
    }
}

/// Where a thumb sits that a function of the row's values places (a levels gamma thumb, between the black and the
/// white one). Both functions get the row's values as drawn.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Follow {
    /// The row value the thumb sits at, for its own value.
    pub at: fn(&[f64], f64) -> f64,
    /// The thumb's own value that puts it at row value `x`, unsnapped.
    pub value_at: fn(&[f64], f64) -> f64,
}

impl Follow {
    /// A thumb placed by `at` and set from a row position by `value_at`.
    pub const fn new(at: fn(&[f64], f64) -> f64, value_at: fn(&[f64], f64) -> f64) -> Self {
        Follow { at, value_at }
    }
}

/// A range with an optional step: where values may go and how keys move them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Scale {
    min: f64,
    max: f64,
    step: Option<f64>,
}

impl Scale {
    fn new(range: &RangeInclusive<f64>, step: Option<f64>) -> Self {
        let (a, b) = (*range.start(), *range.end());
        let (min, max) = if a <= b { (a, b) } else { (b, a) };
        Scale { min, max, step: step.filter(|s| s.is_finite() && *s > 0.0) }
    }

    fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Where `value` is drawn: in the range, and at the start when it is not a finite number.
    fn place(&self, value: f64) -> f64 {
        if value.is_finite() { value.clamp(self.min, self.max) } else { self.min }
    }

    /// `value` on the nearest stop, when there is a step: a multiple of the step from the start, or the end.
    fn snap(&self, value: f64) -> f64 {
        let value = self.place(value);
        let Some(step) = self.step else { return value };
        let at = (value - self.min) / step;
        if value == self.max || (at - at.round()).abs() < 1e-9 {
            return value;
        }
        let below = (self.min + at.floor() * step).min(self.max);
        let above = (below + step).min(self.max);
        if value - below < above - value { below } else { above }
    }

    /// One arrow press: the step, or a hundredth of the range.
    fn key_step(&self) -> f64 {
        self.step.unwrap_or(self.span() * STEP_SHARE)
    }
}

/// One thumb of a [`ThumbRow`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Thumb {
    label: String,
    face: Option<DataColor>,
    scale: Option<Scale>,
    follow: Option<Follow>,
    order: Order,
    unset: Option<f64>,
    reset: Option<f64>,
    falling: bool,
    split: bool,
}

impl Thumb {
    /// A thumb named `label`, free among the others, with the row's range and step. The label is the accessible
    /// name, a whole phrase the app translates.
    pub fn new(label: impl Into<String>) -> Self {
        Thumb {
            label: label.into(),
            face: None,
            scale: None,
            follow: None,
            order: Order::Free,
            unset: None,
            reset: None,
            falling: false,
            split: false,
        }
    }

    /// The triangle's fill, a colour from data; the theme's knob colour by default.
    #[must_use]
    pub fn face(mut self, face: DataColor) -> Self {
        self.face = Some(face);
        self
    }

    /// How the value is kept among the others.
    #[must_use]
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    /// A value with its own `range` and `step`, which `follow` places on the row.
    #[must_use]
    pub fn follows(mut self, range: &RangeInclusive<f64>, step: f64, follow: Follow) -> Self {
        self.scale = Some(Scale::new(range, Some(step)));
        self.follow = Some(follow);
        self
    }

    /// Where a value that is not a finite number is drawn; the start of its range by default.
    #[must_use]
    pub fn unset_at(mut self, value: f64) -> Self {
        self.unset = Some(value);
        self
    }

    /// The value a reset of this thumb restores: Reset while it is focused and a double click on it set this thumb
    /// alone; a thumb without one resets the whole binding.
    #[must_use]
    pub fn reset(mut self, value: f64) -> Self {
        self.reset = Some(value);
        self
    }

    /// The thumb is split into two halves and the next thumb of the row is its high half: this one is the low half,
    /// drawn as the left half of a triangle; the next one only brings its own face, range and order. The pair's
    /// values `[low, high]` sit in consecutive entries with `low <= high`.
    ///
    /// A press that begins without Alt moves the pair together, keeping the spread; with Alt it moves only the half
    /// it grabs. Where the halves coincide they draw as one triangle. Keys, the wheel and AccessKit move one half. A
    /// row does not combine a split thumb with wrapping.
    #[must_use]
    pub fn split(mut self) -> Self {
        self.split = true;
        self
    }

    /// The value falls as the thumb moves toward the end (a levels gamma): arrows, Page keys, Home, End and the
    /// wheel still move the thumb the way they point.
    #[must_use]
    pub fn falling(mut self) -> Self {
        self.falling = true;
        self
    }
}

/// The order of a wrapping row's thumbs round the period.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Anchor {
    /// The thumb the others are measured from; it is free to turn.
    pub first: usize,
    /// The most the last thumb may be past the first, in the row's values.
    pub max_span: f64,
}

impl Anchor {
    /// Thumbs keep their index order going round from thumb `first`, the last at most `max_span` past it. The span
    /// is positive and less than the range.
    pub const fn new(first: usize, max_span: f64) -> Self {
        Anchor { first, max_span }
    }
}

/// What a key, the wheel, a screen-reader request or a tap did to a thumb.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ThumbOp {
    /// A move in the direction of the value: `Forward` and `Up` increase it.
    Move(NavMove, MoveMods),
    /// Sets the value.
    Set(f64),
}

/// An operation on one thumb of a row.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RowOp {
    /// The thumb's index in the row.
    pub thumb: usize,
    /// What happens to it.
    pub op: ThumbOp,
}

impl RowOp {
    /// `op` on thumb `thumb`.
    pub const fn new(thumb: usize, op: ThumbOp) -> Self {
        RowOp { thumb, op }
    }
}

/// What a row's values may be: each thumb's range and its order among the others. The row applies every key, wheel
/// and pointer change through these rules.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct RowRules<'r, const N: usize> {
    thumbs: &'r [Thumb; N],
    scale: Scale,
    /// The period values wrap at.
    wrap: Option<f64>,
    anchored: Option<Anchor>,
}

impl<const N: usize> RowRules<'_, N> {
    fn scale_of(&self, i: usize) -> Scale {
        self.thumbs[i].scale.unwrap_or(self.scale)
    }

    /// The other half of the split thumb that thumb `i` is a half of.
    fn partner(&self, i: usize) -> Option<usize> {
        if self.thumbs[i].split { Some(i + 1).filter(|j| *j < N) } else { i.checked_sub(1).filter(|j| self.thumbs[*j].split) }
    }

    /// `value` in a wrapping row: from the start of `scale`, less than a period on.
    fn wrapped(&self, scale: &Scale, value: f64) -> f64 {
        self.wrap.map_or(value, |period| scale.min + (value - scale.min).rem_euclid(period))
    }

    /// `delta` as the short way round a wrapping row, in `-period / 2..=period / 2`.
    fn short(&self, delta: f64) -> f64 {
        self.wrap.map_or(delta, |period| delta - period * (delta / period).round())
    }

    /// The values as drawn: each in its range, and one that is not finite where its thumb says.
    pub fn placed(&self, value: &[f64; N]) -> [f64; N] {
        std::array::from_fn(|i| {
            let scale = self.scale_of(i);
            let place = |v: f64| if self.wrap.is_some() { self.wrapped(&scale, v) } else { scale.place(v) };
            match value[i] {
                v if v.is_finite() => place(v),
                _ => self.thumbs[i].unset.map_or(scale.min, place),
            }
        })
    }

    /// Where thumb `i` may go among the thumbs it is ordered with, which are at `values`. In a wrapping row the
    /// bounds are measured the short way round from the thumb's own value, and open where it has no neighbour.
    fn order_bounds(&self, values: &[f64; N], i: usize) -> (f64, f64) {
        if let Some(anchor) = self.anchored {
            return self.anchored_bounds(values, i, anchor);
        }
        let scale = self.scale_of(i);
        let open = if self.wrap.is_some() { (f64::NEG_INFINITY, f64::INFINITY) } else { (scale.min, scale.max) };
        let Order::Between { below, above, gap } = self.thumbs[i].order else { return open };
        let side = |j: Option<usize>, gap: f64, end: f64| {
            j.and_then(|j| values.get(j)).map_or(end, |v| {
                if self.wrap.is_some() { values[i] + self.short(v - values[i]) + gap } else { (v + gap).clamp(scale.min, scale.max) }
            })
        };
        let (lo, hi) = (side(below, gap, open.0), side(above, -gap, open.1));
        (lo.min(hi), hi)
    }

    /// How far past the first thumb each thumb is, going round from it in index order: the distance from the thumb
    /// before it, the short way on, and at most the span.
    fn offsets(&self, values: &[f64; N], anchor: Anchor, period: f64) -> [f64; N] {
        let mut offset = [0.0; N];
        let mut before = anchor.first;
        for k in 1..N {
            let j = (anchor.first + k) % N;
            offset[j] = (offset[before] + (values[j] - values[before]).rem_euclid(period)).min(anchor.max_span);
            before = j;
        }
        offset
    }

    /// Where thumb `i` may go in an anchored row, in values counted from the first thumb's: a thumb between its
    /// neighbours' places and, for the last, the span; the first thumb at most half a period before the next one
    /// and so that the last stays within the span.
    fn anchored_bounds(&self, values: &[f64; N], i: usize, anchor: Anchor) -> (f64, f64) {
        let Some(period) = self.wrap else { return (f64::NEG_INFINITY, f64::INFINITY) };
        let offset = self.offsets(values, anchor, period);
        let base = values[anchor.first];
        let at = |k: usize| offset[(anchor.first + k) % N];
        match (i + N - anchor.first) % N {
            0 if N == 1 => (f64::NEG_INFINITY, f64::INFINITY),
            0 => (base + (at(1) - period / 2.0).max(at(N - 1) - anchor.max_span), base + at(1)),
            k if k + 1 == N => (base + at(k - 1), base + anchor.max_span),
            k => (base + at(k - 1), base + at(k + 1)),
        }
    }

    /// `value` as thumb `i` is aimed at it: in an anchored row the first thumb goes the short way round, the others
    /// to where the value is, counted from the first thumb forward.
    fn aim(&self, values: &[f64; N], i: usize, value: f64) -> f64 {
        match (self.anchored, self.wrap) {
            (Some(anchor), Some(period)) if i != anchor.first => {
                let base = values[anchor.first];
                base + (value - base).rem_euclid(period)
            }
            (Some(_), _) => values[i] + self.short(value - values[i]),
            _ => value,
        }
    }

    /// Where thumb `i` may go, with the others at `values`: among the thumbs it is ordered with, and a half of a
    /// split thumb between its partner and the end.
    fn bounds(&self, values: &[f64; N], i: usize) -> (f64, f64) {
        let (mut lo, mut hi) = self.order_bounds(values, i);
        match self.partner(i) {
            Some(j) if j > i => {
                hi = hi.min(values[j]);
                lo = lo.min(hi);
            }
            Some(j) => {
                lo = lo.max(values[j]);
                hi = hi.max(lo);
            }
            None => {}
        }
        (lo, hi)
    }

    /// The bounds AccessKit reports for thumb `i`: the range of a wrapping row, else [`bounds`](Self::bounds).
    fn reported_bounds(&self, values: &[f64; N], i: usize) -> (f64, f64) {
        let scale = self.scale_of(i);
        if self.wrap.is_some() { (scale.min, scale.max) } else { self.bounds(values, i) }
    }

    /// `value` for thumb `i`: within `bounds` and on its stops; in a wrapping row, then wrapped.
    fn settle(&self, i: usize, value: f64, (lo, hi): (f64, f64)) -> f64 {
        let scale = self.scale_of(i);
        if self.wrap.is_none() {
            return scale.snap(value).clamp(lo, hi);
        }
        let wrap = |v: f64| self.wrapped(&scale, v);
        wrap(scale.snap(wrap(value.clamp(lo, hi))))
    }

    /// `row` with `op` applied to its thumb: from where the thumb is drawn (an unset gamma steps from where it
    /// sits), on its stops and within its bounds among the others, so the order and the gap hold. The other thumbs
    /// keep `row`'s values.
    pub fn apply(&self, op: &RowOp, row: &[f64; N]) -> [f64; N] {
        let values = self.placed(row);
        let i = op.thumb;
        let mut out = *row;
        out[i] = if self.wrap.is_some() { self.apply_wrapped(&op.op, &values, i) } else { self.apply_plain(&op.op, &values, i) };
        out
    }

    fn apply_plain(&self, op: &ThumbOp, values: &[f64; N], i: usize) -> f64 {
        let scale = self.scale_of(i);
        let bounds = self.bounds(values, i);
        match op {
            ThumbOp::Set(value) => scale.snap(*value).clamp(bounds.0, bounds.1),
            ThumbOp::Move(mv, mods) => {
                let small = scale.key_step();
                let steps = ValueSteps::new(values[i], bounds.0, bounds.1, small, small * LARGE_STEPS);
                match step_value(&steps, ValueAxes::Both, *mv, *mods) {
                    StepResult::Set(next) => scale.snap(next).clamp(bounds.0, bounds.1),
                    _ => values[i],
                }
            }
        }
    }

    /// [`apply_plain`](Self::apply_plain) for a wrapping row: a step goes on past the range's end and comes back at
    /// its start, and Home and End go to the neighbours, or the range's start and its end less a step where there
    /// are none.
    fn apply_wrapped(&self, op: &ThumbOp, values: &[f64; N], i: usize) -> f64 {
        let scale = self.scale_of(i);
        let bounds = self.bounds(values, i);
        let from = if self.anchored.is_some() { self.aim(values, i, values[i]) } else { values[i] };
        let target = match op {
            ThumbOp::Set(value) => self.aim(values, i, *value),
            ThumbOp::Move(NavMove::Start, _) if bounds.0.is_finite() => bounds.0,
            ThumbOp::Move(NavMove::Start, _) => scale.min,
            ThumbOp::Move(NavMove::End, _) if bounds.1.is_finite() => bounds.1,
            ThumbOp::Move(NavMove::End, _) => scale.max - scale.key_step(),
            ThumbOp::Move(mv, mods) => {
                let small = scale.key_step();
                let steps = ValueSteps::new(0.0, f64::MIN, f64::MAX, small, small * LARGE_STEPS);
                let delta = match step_value(&steps, ValueAxes::Both, *mv, *mods) {
                    StepResult::Set(delta) => delta,
                    _ => return values[i],
                };
                from.clamp(bounds.0, bounds.1) + delta
            }
        };
        self.settle(i, target, bounds)
    }
}

/// What a press at a point moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    /// The thumb under the press, by the pointer's movement.
    Grab(usize),
    /// The nearest thumb, to the pointer: the press is off the thumbs.
    Jump(usize),
    /// Overlapping thumbs under the press: a left drag moves the first of them, a right drag the last.
    Either { first: usize, last: usize },
}

impl Press {
    /// The thumb the press moves with the pointer at `x`, from `origin`.
    fn thumb(self, origin: f32, x: f32) -> Option<usize> {
        match self {
            Press::Grab(i) | Press::Jump(i) => Some(i),
            Press::Either { first, .. } if x < origin => Some(first),
            Press::Either { last, .. } if x > origin => Some(last),
            Press::Either { .. } => None,
        }
    }
}

/// The thumbs on the row's track, in the row's pixels.
struct Layout<'r, const N: usize> {
    rules: RowRules<'r, N>,
    left: f32,
    right: f32,
    rtl: bool,
    /// A press within this distance of a thumb's centre grabs it.
    reach: f32,
    /// Thumbs whose centres are closer than this overlap: a triangle's width.
    overlap: f32,
    /// The press moves only the half of a split thumb it grabs.
    alone: bool,
    /// A wrapping row's period in points; 0 when the row does not wrap.
    period: f32,
}

impl<'r, const N: usize> Layout<'r, N> {
    fn new(rules: RowRules<'r, N>, rect: Rect, rtl: bool, reach: f32, alone: bool) -> Self {
        let thumb = thumb_size(rect);
        let (left, right) = (rect.left() + thumb.x / 2.0, rect.right() - thumb.x / 2.0);
        let mut layout = Layout { rules, left, right, rtl, reach, overlap: thumb.x, alone, period: 0.0 };
        layout.period = match rules.wrap {
            Some(period) if rules.scale.span() > 0.0 => ((right - left) as f64 * period / rules.scale.span()) as f32,
            _ => 0.0,
        };
        layout
    }

    fn x_of(&self, value: f64) -> f32 {
        let scale = &self.rules.scale;
        let fraction = if scale.span() > 0.0 { ((value - scale.min) / scale.span()).clamp(0.0, 1.0) as f32 } else { 0.0 };
        self.left + (self.right - self.left) * if self.rtl { 1.0 - fraction } else { fraction }
    }

    /// The row value thumb `i` sits at.
    fn at(&self, values: &[f64; N], i: usize) -> f64 {
        self.rules.thumbs[i].follow.map_or(values[i], |f| (f.at)(values, values[i]))
    }

    fn xs(&self, values: &[f64; N]) -> [f32; N] {
        std::array::from_fn(|i| self.x_of(self.at(values, i)))
    }

    /// `a - b` in points, the short way round in a wrapping row.
    fn dx(&self, a: f32, b: f32) -> f32 {
        let d = a - b;
        if self.period > 0.0 { d - self.period * (d / self.period).round() } else { d }
    }

    /// What a press at `x` moves, with the thumbs at `values`.
    fn press(&self, values: &[f64; N], x: f32) -> Option<Press> {
        let xs = self.xs(values);
        let d = |i: usize| self.dx(x, xs[i]).abs();
        let nearest = (0..N).min_by(|&a, &b| d(a).total_cmp(&d(b)))?;
        let at = xs[nearest];
        let on = d(nearest) <= self.reach;
        // Thumbs stacked at one place: the side of the press picks, as a drag's direction does on them.
        let group = |i: usize| if on { d(i) <= self.reach && self.dx(xs[i], at).abs() < self.overlap } else { self.dx(xs[i], at) == 0.0 };
        let first = (0..N).find(|&i| group(i)).unwrap_or(nearest);
        let last = (0..N).rev().find(|&i| group(i)).unwrap_or(nearest);
        Some(match (on, first == last) {
            (true, true) => Press::Grab(nearest),
            (true, false) => Press::Either { first, last },
            (false, _) if self.dx(x, at) < 0.0 => Press::Jump(first),
            (false, _) => Press::Jump(last),
        })
    }

    /// The values with `press` dragged to the position `state` holds: the moved thumb from where it was at `anchor`
    /// (the values the gesture began from), the others as they were. A grabbed thumb moves by the pointer's travel
    /// and stays while the pointer is still; a jumping one goes to the pointer. The moved value snaps to its step
    /// and stays in its bounds among the others. A half of a split thumb takes its partner along, keeping the
    /// spread, unless the press moves it alone.
    fn moved(&self, anchor: &[f64; N], press: Press, state: &PressState) -> [f64; N] {
        let (origin, pos) = match state.phase {
            PressPhase::DragStart { start, pos, .. } | PressPhase::Drag { start, pos, .. } | PressPhase::Release { start, pos, .. } => {
                (start.origin, pos)
            }
            _ => return *anchor,
        };
        let Some(i) = press.thumb(origin.x, pos.x) else { return *anchor };
        let grab = !matches!(press, Press::Jump(_));
        let mut out = *anchor;
        if grab && pos.x == origin.x {
            return out;
        }
        let rules = &self.rules;
        let from = rules.placed(anchor);
        let placed_at = if rules.anchored.is_some() { rules.aim(&from, i, self.at(&from, i)) } else { self.at(&from, i) };
        let track = self.left..self.right;
        let drag = if grab {
            let rate = if track.end > track.start { rules.scale.span() / f64::from(track.end - track.start) } else { 0.0 };
            ValueDrag::relative(rate, f64::NEG_INFINITY..=f64::INFINITY)
        } else {
            ValueDrag::absolute(track, rules.scale.min..=rules.scale.max)
        };
        let Some(at) = drag.value(placed_at, state, self.rtl) else { return out };
        let value = rules.thumbs[i].follow.map_or(at, |f| (f.value_at)(&from, at));
        // A jump in a wrapping row goes the short way round, in an anchored row forward from the first thumb.
        let value = if grab || rules.wrap.is_none() {
            value
        } else if rules.anchored.is_some() {
            rules.aim(&from, i, value)
        } else {
            from[i] + rules.short(value - from[i])
        };
        if let Some(j) = rules.partner(i).filter(|_| !self.alone) {
            let (lo_i, hi_i) = rules.order_bounds(&from, i);
            let (lo_j, hi_j) = rules.order_bounds(&from, j);
            let (lo, hi) = ((lo_i - from[i]).max(lo_j - from[j]), (hi_i - from[i]).min(hi_j - from[j]));
            let lo = lo.min(hi);
            let moved = rules.scale_of(i).snap(from[i] + (value - from[i]).clamp(lo, hi)).clamp(from[i] + lo, from[i] + hi);
            out[i] = moved;
            out[j] = from[j] + (moved - from[i]);
            return out;
        }
        out[i] = rules.settle(i, value, rules.bounds(&from, i));
        out
    }
}

/// A thumb's size from the row's rect: the row's height gives its height, which sets its width.
fn thumb_size(rect: Rect) -> Vec2 {
    let height = rect.height() * THUMB_SHARE;
    Vec2::new(height * THUMB_ASPECT, height)
}

/// A half of a split thumb, drawn as the half of a triangle on its side of the tip's vertical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Half {
    Low,
    High,
}

/// The triangle, tip up at `tip`, or one `half` of it.
fn triangle(tip: Pos2, size: Vec2, half: Option<Half>) -> Vec<Pos2> {
    let (left, right) = match half {
        None => (tip.x - size.x / 2.0, tip.x + size.x / 2.0),
        Some(Half::Low) => (tip.x - size.x / 2.0, tip.x),
        Some(Half::High) => (tip.x, tip.x + size.x / 2.0),
    };
    vec![tip, pos2(right, tip.y + size.y), pos2(left, tip.y + size.y)]
}

/// The triangle grown by `out` on every side: its sides move out by `out` when it scales about its incentre.
fn grown(points: &[Pos2], out: f32) -> Vec<Pos2> {
    let len = |a: Pos2, b: Pos2| (a - b).length();
    let (a, b, c) = (len(points[1], points[2]), len(points[0], points[2]), len(points[0], points[1]));
    let perimeter = a + b + c;
    if perimeter <= 0.0 {
        return points.to_vec();
    }
    let centre = ((points[0].to_vec2() * a + points[1].to_vec2() * b + points[2].to_vec2() * c) / perimeter).to_pos2();
    let width = points[1].x.max(points[2].x) - points[1].x.min(points[2].x);
    let height = points[1].y - points[0].y;
    let radius = width * height / perimeter;
    if radius <= 0.0 {
        return points.to_vec();
    }
    let k = (radius + out) / radius;
    points.iter().map(|p| centre + (*p - centre) * k).collect()
}

/// The values of the running drag when it began, the press it resolved to, and whether it moves a half alone.
#[derive(Clone, Default)]
struct Held {
    anchor: Vec<f64>,
    press: Option<Press>,
    alone: bool,
}

/// The direction of the thumb when the value falls toward the end of the row.
fn against(mv: NavMove) -> NavMove {
    match mv {
        NavMove::Forward => NavMove::Back,
        NavMove::Back => NavMove::Forward,
        NavMove::Up => NavMove::Down,
        NavMove::Down => NavMove::Up,
        NavMove::PageUp => NavMove::PageDown,
        NavMove::PageDown => NavMove::PageUp,
        NavMove::Start => NavMove::End,
        NavMove::End => NavMove::Start,
        other => other,
    }
}

/// Thumbs on one row, tip up, each a value of its own.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(probe(ThumbRow<'static, 4>))]
pub struct ThumbRow<'a, const N: usize> {
    #[common]
    common: Common,
    value: Bind<'a, [f64; N]>,
    range: RangeInclusive<f64>,
    /// The thumbs, one per value; unnamed free thumbs by default.
    #[prop]
    thumbs: [Thumb; N],
    /// The change of one arrow press; the default is a hundredth of the range.
    #[prop(quantity = "step")]
    step: Option<f64>,
    /// Values wrap at the length of the range (a hue bar): a thumb dragged or stepped past the end appears at the
    /// start. Distances between thumbs are measured the short way round.
    #[prop(flag)]
    wrap: bool,
    /// Thumbs keep their index order going round the period from one anchor thumb; implies `wrap`. An `Order` on a
    /// thumb of an anchored row has no meaning and stays `Free`.
    #[prop]
    anchor: Option<Anchor>,
}

impl<'a, const N: usize> ThumbRow<'a, N> {
    /// A row editing `value` over `range`.
    pub fn new(value: Bind<'a, [f64; N]>, range: RangeInclusive<f64>) -> Self {
        ThumbRow { common: Common::new(), value, range, thumbs: std::array::from_fn(|_| Thumb::new("")), step: None, wrap: false, anchor: None }
    }

    /// What the values of this row may be.
    pub fn rules(&self) -> RowRules<'_, N> {
        let scale = Scale::new(&self.range, self.step);
        let wrap = (self.wrap || self.anchor.is_some()).then_some(scale.span()).filter(|period| *period > 0.0);
        let anchored = self.anchor.filter(|a| wrap.is_some() && a.first < N);
        RowRules { thumbs: &self.thumbs, scale, wrap, anchored }
    }

    fn halves(&self, rules: &RowRules<'_, N>, shown: &[f64; N]) -> [Option<Half>; N] {
        std::array::from_fn(|i| match rules.partner(i) {
            Some(j) if shown[i] != shown[j] => Some(if j > i { Half::Low } else { Half::High }),
            _ => None,
        })
    }
}

impl<const N: usize> Component for ThumbRow<'_, N> {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows).with_activate(ActivateKeys::Neither);
    type Style = ThumbRowStyle;
    type Output = EditResponse<[f64; N]>;

    fn style(cx: &StyleCx<'_>, _: ThumbRowStyleKey<N>) -> ThumbRowStyle {
        let control = cx.control();
        let colors = &cx.tokens.color;
        let (clear, soft, accent) = (Rgba::TRANSPARENT, colors.accent.soft, colors.accent.strong);
        let fixed = |rest: Rgba, disabled: Rgba| {
            Stateful::new(cx.color(rest)).off(States::new(cx.color(rest), cx.color(rest), cx.color(rest), cx.color(disabled)))
        };
        ThumbRowStyle {
            metrics: Metrics::new().min_size(Vec2::new(control.icon * LENGTH_IN_ICONS, control.height)),
            focus: cx.focus_look(),
            face: fixed(colors.surface.knob, colors.state.disabled_fill),
            edge: fixed(colors.text.muted, colors.text.disabled),
            ring: Stateful::new(cx.color(clear)).off(States::new(cx.color(clear), cx.color(soft), cx.color(accent), cx.color(clear))),
            ring_width: cx.tokens.stroke.thick,
            hit: cx.tokens.size.hit.target,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.icon() * LENGTH_IN_ICONS, cx.icon())
    }

    fn sense(&self) -> Sense {
        Sense::WHEEL | Sense::WHEEL_X
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let rules = self.rules();
        debug_assert!(
            rules.anchored.is_none() || rules.thumbs.iter().all(|t| t.order == Order::Free),
            "an anchored row orders its thumbs itself; their `Order` stays `Free`"
        );
        debug_assert!(
            rules.thumbs.iter().enumerate().all(|(i, t)| !t.split || (i + 1 < N && !rules.thumbs[i + 1].split && rules.wrap.is_none())),
            "a split thumb needs a high half after it, which is not split itself, in a row that does not wrap"
        );
        let row = cx.value().copied().unwrap_or([f64::NAN; N]);
        let shown = rules.placed(&row);
        let layout = Layout::new(rules, rect, cx.flow.is_rtl(), 0.0, false);
        let xs = layout.xs(&shown);
        let size = thumb_size(rect);
        let top = rect.top() + (rect.height() - size.y) / 2.0;
        let halves = self.halves(&rules, &shown);
        cx.part(ROW, rect).sense(Sense::CLICK | Sense::DRAG).value_part().pointer_only("the arrow keys step the focused thumb");
        for i in 0..N {
            let (left, width) = match halves[i] {
                None => (xs[i] - size.x / 2.0, size.x),
                Some(Half::Low) => (xs[i] - size.x / 2.0, size.x / 2.0),
                Some(Half::High) => (xs[i], size.x / 2.0),
            };
            let thumb = Rect::from_min_size(pos2(left, top), Vec2::new(width, size.y));
            let part = cx
                .part(THUMB.at(i as u64), thumb)
                .sense(Sense::DRAG)
                .priority(HitPriority::new(HitLevel::Handle, 0))
                .focusable()
                .focus_shape(FocusShape::Rect)
                .value_part();
            if rules.thumbs[i].reset.is_some() {
                part.own_reset();
            }
        }
        cx.roving(Roving::new(NavShape::Flat).home_end(false));
    }

    fn bind(&mut self) -> Option<BindMut<'_, [f64; N]>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.resolved.read_only || !cx.resolved.enabled {
            return;
        }
        let rules = self.rules();
        let (rect, rtl, reach) = (cx.rect, cx.flow.is_rtl(), cx.style.hit);
        let row = cx.value().copied().unwrap_or([f64::NAN; N]);
        let finite = row.iter().all(|v| v.is_finite());
        let parts: Vec<PartRef> = std::iter::once(PartRef::from(ROW)).chain((0..N).map(|i| THUMB.at(i as u64))).collect();
        for part in parts {
            let press = cx.part(part).press;
            match press.phase {
                PressPhase::DragStart { start, .. } if finite => {
                    let layout = Layout::new(rules, rect, rtl, reach, press.mods.alt);
                    if let Some(picked) = layout.press(&rules.placed(&row), start.origin.x) {
                        *cx.memory::<Held>() = Held { anchor: row.to_vec(), press: Some(picked), alone: press.mods.alt };
                        cx.begin();
                        cx.propose(layout.moved(&row, picked, &press));
                    }
                }
                PressPhase::Drag { .. } => {
                    let held = cx.memory::<Held>().clone();
                    if let (Some(picked), Ok(anchor)) = (held.press, <[f64; N]>::try_from(held.anchor)) {
                        let layout = Layout::new(rules, rect, rtl, reach, held.alone);
                        cx.propose(layout.moved(&anchor, picked, &press));
                    }
                }
                PressPhase::Release { start, click, .. } => {
                    if cx.gesture().is_some() {
                        let held = cx.memory::<Held>().clone();
                        if let (Some(picked), Ok(anchor)) = (held.press, <[f64; N]>::try_from(held.anchor)) {
                            let layout = Layout::new(rules, rect, rtl, reach, held.alone);
                            cx.propose(layout.moved(&anchor, picked, &press));
                        }
                        cx.commit();
                    } else if click && finite && start.clicks < 2 {
                        // A tap off the thumbs moves the nearest one there; a tap on a thumb moves none. A second
                        // click is the reset, which the pipeline applies.
                        let layout = Layout::new(rules, rect, rtl, reach, false);
                        if let Some(picked @ Press::Jump(thumb)) = layout.press(&rules.placed(&row), start.origin.x) {
                            let target = layout.moved(&row, picked, &press)[thumb];
                            cx.set(rules.apply(&RowOp::new(thumb, ThumbOp::Set(target)), &row));
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(part) = cx.reset_part().filter(|part| part.key == THUMB)
            && let Some(thumb) = part.index.map(|i| i as usize).filter(|i| *i < N)
            && let Some(value) = rules.thumbs[thumb].reset
        {
            let current = cx.value().copied().unwrap_or(row);
            cx.set(rules.apply(&RowOp::new(thumb, ThumbOp::Set(value)), &current));
        }
        for thumb in 0..N {
            if let Some(next) = cx.intents().take_set_value_at(THUMB.at(thumb as u64)) {
                let current = cx.value().copied().unwrap_or(row);
                cx.set(rules.apply(&RowOp::new(thumb, ThumbOp::Set(next)), &current));
            }
        }
        let focused = cx.active_part().filter(|part| part.key == THUMB).and_then(|part| part.index).map(|i| i as usize).filter(|i| *i < N);
        let Some(thumb) = focused else { return };
        let direction = |mv: NavMove| if rules.thumbs[thumb].falling { against(mv) } else { mv };
        while let Some((mv, mods)) = cx.intents().take_move() {
            let current = cx.value().copied().unwrap_or(row);
            cx.set(rules.apply(&RowOp::new(thumb, ThumbOp::Move(direction(mv), mods)), &current));
        }
        if let Some(wheel) = cx.take_wheel() {
            let along = if wheel.scroll.y != 0.0 {
                wheel.scroll.y
            } else if rtl {
                -wheel.scroll.x
            } else {
                wheel.scroll.x
            };
            let mv = if along > 0.0 { NavMove::Forward } else { NavMove::Back };
            let current = cx.value().copied().unwrap_or(row);
            cx.set(rules.apply(&RowOp::new(thumb, ThumbOp::Move(direction(mv), MoveMods::PLAIN)), &current));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let rules = self.rules();
        let words = cx.words::<ValueDragWords>();
        let row = cx.value().copied();
        let shown = rules.placed(&row.unwrap_or([f64::NAN; N]));
        for i in 0..N {
            let (lo, hi) = rules.reported_bounds(&shown, i);
            let label = &rules.thumbs[i].label;
            let name = match rules.partner(i) {
                Some(j) if j > i => words.thumb_low.fill(&[("name", label.as_str())]),
                Some(j) => words.thumb_high.fill(&[("name", rules.thumbs[j].label.as_str())]),
                None => label.clone(),
            };
            let step = rules.scale_of(i).key_step();
            let numeric = match row {
                Some(row) if row[i].is_finite() => Numeric::new(shown[i], lo..=hi),
                _ => Numeric::empty(lo..=hi),
            };
            let node = access.node(THUMB.at(i as u64), Role::SLIDER);
            node.name = Name::Text(name);
            node.numeric = Some(numeric.step(step));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rules = self.rules();
        let style = cx.style;
        let draw = ui.draw();
        let row = cx.value().copied().unwrap_or([f64::NAN; N]);
        let shown = rules.placed(&row);
        let layout = Layout::new(rules, cx.rect, cx.flow.is_rtl(), 0.0, false);
        let xs = layout.xs(&shown);
        let size = thumb_size(cx.rect);
        let top = cx.rect.top() + (cx.rect.height() - size.y) / 2.0;
        let halves = self.halves(&rules, &shown);
        for i in 0..N {
            // Coinciding halves are one triangle, which the low half draws.
            if halves[i].is_none() && rules.partner(i).is_some_and(|j| j < i) {
                continue;
            }
            let points = triangle(pos2(xs[i], top), size, halves[i]);
            let part = THUMB.at(i as u64);
            draw.fill(&PaintShape::Polygon(grown(&points, style.ring_width)), cx.resolve_part(part, &style.ring));
            let shape = PaintShape::Polygon(points);
            match rules.thumbs[i].face {
                Some(face) => draw.data_fill(&shape, face),
                None => draw.fill(&shape, cx.resolve_part(part, &style.face)),
            }
            draw.stroke(&shape, StrokeWidth::Thin, cx.resolve_part(part, &style.edge));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<[f64; N]> {
        cx.edit_response()
    }
}
