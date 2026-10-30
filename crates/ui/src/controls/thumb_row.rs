//! [`ThumbRow`]: triangle thumbs on one row, each setting a value of its own: a levels editor's
//! input row (black, gamma, white) and output row (black, white).

use std::ops::{Deref, DerefMut, RangeInclusive};

use craft_theme::States;
use egui::accesskit::Role;
use egui::{
    AsIdSalt, Color32, CornerRadius, Id, IdSalt, InteractOptions, Key, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, Ui,
    Vec2, pos2, vec2,
};

use super::slider::{self, KEY_FILTER, PressTurn, Scale, Sensing, Track, pointer, press_turn, stepped, tap};
use crate::edit::ops::{Bound, ScalarOp};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{FocusRing, Interactive, StateLayer};
use crate::primitives::{Gap, Stack};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// Width of a thumb's edge, in points.
const EDGE: f32 = 1.0;

/// The row's values as its gesture holds them. An unset value (a levels editor's NaN gamma)
/// equals another, as the gesture's comparisons need ([`Gesture`]).
#[derive(Clone, Copy, Debug)]
struct Row<const N: usize>([f64; N]);

impl<const N: usize> PartialEq for Row<N> {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().zip(&other.0).all(|(a, b)| a == b || (a.is_nan() && b.is_nan()))
    }
}

impl<const N: usize> Deref for Row<N> {
    type Target = [f64; N];

    fn deref(&self) -> &[f64; N] {
        &self.0
    }
}

impl<const N: usize> DerefMut for Row<N> {
    fn deref_mut(&mut self) -> &mut [f64; N] {
        &mut self.0
    }
}

/// A thumb row's sizes and colours, built from the levels editor's style.
#[derive(Clone, Debug, PartialEq)]
pub struct ThumbRowStyle {
    /// Height of the row: `levels.row`.
    pub height: f32,
    /// A triangle's width and height: `levels.thumb_width`, `levels.thumb_height`.
    pub thumb: Vec2,
    /// Distance from a thumb's centre within which a press grabs it: `levels.hit`.
    pub hit: f32,
    /// A thumb's edge: `levels.thumb_edge`.
    pub edge: Color32,
    /// A disabled thumb's edge: `levels.disabled_edge`.
    pub disabled_edge: Color32,
    /// The ring round a thumb, outside its edge: `levels.ring`.
    pub ring: States<Color32>,
    /// `slider.ring_width`.
    pub ring_width: f32,
    /// The accessible name of the low half of a split thumb; `{name}` is the thumb's label.
    /// English default "{name} low".
    pub low_label: String,
    /// The accessible name of the high half of a split thumb. English default "{name} high".
    pub high_label: String,
}

impl ThumbRowStyle {
    /// Replace the half labels with `translate(key)`, keeping the English default where it
    /// returns `None`. Keys: `thumbRow.low` ("{name} low") and `thumbRow.high` ("{name} high"),
    /// with `{name}` standing for the thumb's label.
    #[must_use]
    pub fn localized(mut self, translate: impl Fn(&str) -> Option<String>) -> Self {
        for (key, label) in [("thumbRow.low", &mut self.low_label), ("thumbRow.high", &mut self.high_label)] {
            if let Some(text) = translate(key) {
                *label = text;
            }
        }
        self
    }

    /// The default names of a split thumb's halves.
    pub(crate) fn half_labels() -> (String, String) {
        ("{name} low".to_owned(), "{name} high".to_owned())
    }

    /// How far a thumb's centre stays inside each end of the row: half a thumb, so the triangles
    /// stay inside it.
    pub fn inset(&self) -> f32 {
        self.thumb.x / 2.0
    }
}

/// How a thumb's value is kept among the row's other values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Order {
    /// Anywhere in its range: it passes the other thumbs.
    Free,
    /// At least `gap` above the value of thumb `below` and below that of thumb `above`. A drag
    /// stops there; it doesn't push the other thumb.
    Between { below: Option<usize>, above: Option<usize>, gap: f64 },
}

/// Where a thumb sits that a function of the row's values places (a levels gamma thumb, between
/// the black and the white one). The functions get the row's values as drawn.
#[derive(Clone, Copy, Debug)]
pub struct Follow {
    /// The row value (a level on the row's range) the thumb sits at, for its value.
    pub at: fn(&[f64], f64) -> f64,
    /// The thumb's value that puts it at row value `x`, unsnapped and finite.
    pub value_at: fn(&[f64], f64) -> f64,
}

/// One thumb of a [`ThumbRow`].
#[derive(Clone, Debug)]
pub struct Thumb {
    label: String,
    face: Color32,
    /// Its own range and step; the row's when `None`.
    scale: Option<Scale>,
    follow: Option<Follow>,
    order: Order,
    default: Option<f64>,
    unset: Option<f64>,
    falling: bool,
    split: bool,
}

impl Thumb {
    /// A thumb named `label` (its accessible name, a whole phrase the app translates) with a
    /// triangle of `face`, whose value is the row's range and step, free among the others.
    pub fn new(label: impl Into<String>, face: Color32) -> Self {
        Self { label: label.into(), face, scale: None, follow: None, order: Order::Free, default: None, unset: None, falling: false, split: false }
    }

    /// How the value is kept among the others.
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    /// A value with its own `range` and `step`, which `follow` places on the row.
    pub fn follows(mut self, range: &RangeInclusive<f64>, step: f64, follow: Follow) -> Self {
        self.scale = Some(Scale::new(range, Some(step)));
        self.follow = Some(follow);
        self
    }

    /// The value a double-click on the thumb, or Ctrl/Cmd+Backspace while it has the focus, sets.
    pub fn default_value(mut self, value: f64) -> Self {
        self.default = Some(value);
        self
    }

    /// Where a value that isn't a finite number is drawn; the start of its range by default.
    pub fn unset_at(mut self, value: f64) -> Self {
        self.unset = Some(value);
        self
    }

    /// The thumb is split into two halves, and the next thumb of the row is its high half: this
    /// one is the low half, drawn as the left half of a triangle and named by the style's `low_label`; the next
    /// is named by `high_label` after this thumb's label, and only its face, scale and order are its
    /// own. The row's values `[low, high]` of the pair sit in consecutive entries, `low <= high`.
    ///
    /// A press that begins without Alt held moves the pair together, keeping the spread between the
    /// halves; with Alt held it moves only the half it grabs, which splits the pair. Where the
    /// halves coincide they are drawn as one triangle. Keys, the wheel and screen-reader requests
    /// move one half. A row doesn't combine `split` with [`wrap`](ThumbRow::wrap).
    pub fn split(mut self) -> Self {
        self.split = true;
        self
    }

    /// The value falls as the thumb moves right (a levels gamma): the arrow keys, Page keys, Home,
    /// End and the wheel still move the thumb the way they point.
    pub fn falling(mut self) -> Self {
        self.falling = true;
        self
    }
}

/// Triangle thumbs on one row, tip up, each a value of its own: the rows of a levels editor. It
/// reports the row's values as one array through the edit lifecycle, built like
/// [`RangeSlider`](super::RangeSlider): one gesture on the row, with the thumbs as its focus
/// owners. An edit writes only the edited thumb's value; the others stay as the gesture holds
/// them.
///
/// - Each thumb is a Tab stop and an AccessKit `Slider` named by its label, with its value and
///   bounds; a value that isn't finite (an unset gamma) is reported as mixed, with no value.
/// - Pointer (primary button): a press within `levels.hit` of a thumb grabs it, so it moves with
///   the pointer and doesn't jump; a press elsewhere moves the nearest thumb there. Where thumbs
///   overlap under the press, the drag's direction picks: left the first of them in row order,
///   right the last; a still press moves none. A press focuses the thumb it moves (the nearest
///   one while the drag hasn't picked), which keeps the focus after the release. A tap is a
///   [`Set`](crate::edit::EditEvent::Set) to what the same press and release give over two frames.
/// - A double-click on a thumb, or Ctrl/Cmd+Backspace while it has the focus, sets its default.
/// - Keys, the wheel and screen-reader requests move the focused thumb as on a
///   [`Slider`](super::Slider), within its bounds; a [falling](Thumb::falling) thumb's keys and
///   wheel follow the thumb, its screen-reader requests the value.
/// - The row keeps its geometry right to left: values grow to the right.
#[derive(Clone, Debug)]
#[must_use = "show the thumb row with `show`"]
pub struct ThumbRow<const N: usize> {
    id_salt: IdSalt,
    value: [f64; N],
    scale: Scale,
    thumbs: [Thumb; N],
    width: Option<f32>,
    policy: EditPolicy,
    enabled: bool,
    part: bool,
    wrap: Option<f64>,
    anchored: Option<Anchored>,
}

/// The order of a wrapping row's thumbs round the period ([`ThumbRow::wrap_from`]).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Anchored {
    /// The thumb the others are measured from.
    first: usize,
    /// The most the last thumb may be past the first, in value units.
    max_span: f64,
}

/// What a [`ThumbRow`] shows and reports in a frame.
#[derive(Debug)]
pub struct ThumbRowOutput<const N: usize> {
    /// The frame's edit event; the response covers the row.
    pub edit: EditResponse<[f64; N]>,
    /// The values drawn: the gesture's, each in its range. A composite's other parts draw from
    /// the composite's value instead, which a row edit waiting for its turn isn't in.
    #[cfg(test)]
    pub shown: [f64; N],
    /// The thumb a pointer drag is moving.
    pub dragged: Option<usize>,
    /// What the frame's Set did, in order: each applies to the row the ones before it left
    /// ([`RowRules::apply`]).
    pub(crate) ops: Vec<RowOp>,
}

/// An operation on one thumb of a row: what a key, the wheel, a screen-reader request, a reset or
/// a tap off the thumbs did to it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RowOp {
    pub thumb: usize,
    pub op: ScalarOp,
}

/// What a press at a point moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    /// The thumb under the press, by the pointer's movement.
    Grab(usize),
    /// The nearest thumb, to the pointer: the press is off the thumbs.
    Jump(usize),
    /// Overlapping thumbs under the press: a left drag moves the first of them, a right drag the
    /// last. The nearest has the focus until the drag picks.
    Either { first: usize, last: usize, nearest: usize },
}

impl Press {
    /// The thumb the press focuses before the pointer moves.
    fn focus(self) -> usize {
        match self {
            Press::Grab(i) | Press::Jump(i) => i,
            Press::Either { nearest, .. } => nearest,
        }
    }

    /// The thumb the press moves with the pointer at `x`, from `origin`.
    fn thumb(self, origin: f32, x: f32) -> Option<usize> {
        match self {
            Press::Grab(i) | Press::Jump(i) => Some(i),
            Press::Either { first, .. } if x < origin => Some(first),
            Press::Either { last, .. } if x > origin => Some(last),
            Press::Either { .. } => None,
        }
    }

    /// The one thumb the press moves, before the drag picks among overlapping ones.
    fn single(self) -> Option<usize> {
        match self {
            Press::Grab(i) | Press::Jump(i) => Some(i),
            Press::Either { .. } => None,
        }
    }

    /// Whether the press lights thumb `i`.
    fn lights(self, i: usize) -> bool {
        match self {
            Press::Grab(t) | Press::Jump(t) => t == i,
            Press::Either { first, last, .. } => (first..=last).contains(&i),
        }
    }
}

/// What a row's values may be: each thumb's range, and its order among the others.
#[derive(Clone, Debug)]
pub(crate) struct RowRules<const N: usize> {
    thumbs: [Thumb; N],
    scale: Scale,
    /// The period values wrap at ([`ThumbRow::wrap`]).
    wrap: Option<f64>,
    anchored: Option<Anchored>,
}

impl<const N: usize> RowRules<N> {
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

    /// The values as drawn: each in its range, and one that isn't finite where its thumb says.
    pub(crate) fn placed(&self, value: &[f64; N]) -> [f64; N] {
        std::array::from_fn(|i| {
            let scale = self.scale_of(i);
            let place = |v: f64| if self.wrap.is_some() { self.wrapped(&scale, v) } else { scale.place(v) };
            match value[i] {
                v if v.is_finite() => place(v),
                _ => self.thumbs[i].unset.map_or(scale.min, place),
            }
        })
    }

    /// Where thumb `i` may go among the other thumbs it is ordered with, which are at `values`.
    /// In a wrapping row the bounds are measured the short way round from the thumb's own value,
    /// and open where it has no neighbour.
    fn order_bounds(&self, values: &[f64; N], i: usize) -> (f64, f64) {
        if let Some(anchored) = self.anchored {
            return self.anchored_bounds(values, i, anchored);
        }
        let scale = self.scale_of(i);
        let Order::Between { below, above, gap } = self.thumbs[i].order else {
            return if self.wrap.is_some() { (f64::NEG_INFINITY, f64::INFINITY) } else { (scale.min, scale.max) };
        };
        let side = |j: Option<usize>, gap: f64, end: f64| {
            j.and_then(|j| values.get(j)).map_or(end, |v| {
                if self.wrap.is_some() { values[i] + self.short(v - values[i]) + gap } else { (v + gap).clamp(scale.min, scale.max) }
            })
        };
        let open = if self.wrap.is_some() { (f64::NEG_INFINITY, f64::INFINITY) } else { (scale.min, scale.max) };
        let (lo, hi) = (side(below, gap, open.0), side(above, -gap, open.1));
        (lo.min(hi), hi)
    }

    /// How far past the first thumb each thumb is, going round from it in index order: the
    /// distance from the thumb before it, the short way on, and at most the span.
    fn offsets(&self, values: &[f64; N], anchored: Anchored, period: f64) -> [f64; N] {
        let mut offset = [0.0; N];
        let mut before = anchored.first;
        for k in 1..N {
            let j = (anchored.first + k) % N;
            offset[j] = (offset[before] + (values[j] - values[before]).rem_euclid(period)).min(anchored.max_span);
            before = j;
        }
        offset
    }

    /// Where thumb `i` may go in an anchored row, in values counted from the first thumb's: a
    /// thumb between its neighbours' places and, for the last, the span; the first thumb at most
    /// half a period before the next one and so that the last stays within the span.
    fn anchored_bounds(&self, values: &[f64; N], i: usize, anchored: Anchored) -> (f64, f64) {
        let Some(period) = self.wrap else { return (f64::NEG_INFINITY, f64::INFINITY) };
        let offset = self.offsets(values, anchored, period);
        let base = values[anchored.first];
        let at = |k: usize| offset[(anchored.first + k) % N];
        match (i + N - anchored.first) % N {
            0 if N == 1 => (f64::NEG_INFINITY, f64::INFINITY),
            0 => (base + (at(1) - period / 2.0).max(at(N - 1) - anchored.max_span), base + at(1)),
            k if k + 1 == N => (base + at(k - 1), base + anchored.max_span),
            k => (base + at(k - 1), base + at(k + 1)),
        }
    }

    /// `value` as thumb `i` is aimed at it: in an anchored row the first thumb goes the short way
    /// round, the others to where the value is, counted from the first thumb forward.
    fn aim(&self, values: &[f64; N], i: usize, value: f64) -> f64 {
        match (self.anchored, self.wrap) {
            (Some(anchored), Some(period)) if i != anchored.first => {
                let base = values[anchored.first];
                base + (value - base).rem_euclid(period)
            }
            (Some(_), _) => values[i] + self.short(value - values[i]),
            _ => value,
        }
    }

    /// Where thumb `i` may go, with the others at `values`: among the thumbs it is ordered with,
    /// and a half of a split thumb between its partner and the end.
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

    /// The bounds a screen reader reports for thumb `i`: the range of a wrapping row, else
    /// [`bounds`](Self::bounds).
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

    /// `row` with `op` applied to its thumb ([`slider::apply`]): from where the thumb is drawn
    /// (an unset gamma steps from where it sits), on its stops and within its bounds among the
    /// others, so the order and the gap hold. The other thumbs keep `row`'s values.
    pub(crate) fn apply(&self, op: &RowOp, row: &[f64; N]) -> [f64; N] {
        let values = self.placed(row);
        let i = op.thumb;
        let mut out = *row;
        out[i] = if self.wrap.is_some() {
            self.apply_wrapped(&op.op, &values, i)
        } else {
            slider::apply(&op.op, values[i], self.bounds(&values, i), &self.scale_of(i))
        };
        out
    }

    /// [`slider::apply`] for a wrapping row: a step goes on past the range's end and comes back
    /// at its start, and Home and End go to the neighbours, or the range's start and its end less
    /// a step where there are none.
    fn apply_wrapped(&self, op: &ScalarOp, values: &[f64; N], i: usize) -> f64 {
        let scale = self.scale_of(i);
        let bounds = self.bounds(values, i);
        let from = if self.anchored.is_some() { self.aim(values, i, values[i]) } else { values[i] };
        let by = |delta| scale.moved(from.clamp(bounds.0, bounds.1), delta);
        let target = match op {
            ScalarOp::By(delta) => by(*delta),
            ScalarOp::Presses(presses) => by(scale.key_step() * presses.iter().map(|(m, n)| if m.shift { 10.0 * n } else { *n }).sum::<f64>()),
            ScalarOp::Set(v) => self.aim(values, i, *v),
            ScalarOp::ToBound(Bound::Low) if bounds.0.is_finite() => bounds.0,
            ScalarOp::ToBound(Bound::Low) => scale.min,
            ScalarOp::ToBound(Bound::High) if bounds.1.is_finite() => bounds.1,
            // The range's end is its start again.
            ScalarOp::ToBound(Bound::High) => scale.max - scale.key_step(),
        };
        self.settle(i, target, bounds)
    }

    /// The operation that sets thumb `i` to its default, if it has one.
    fn reset(&self, i: usize) -> Option<RowOp> {
        self.thumbs[i].default.filter(|d| d.is_finite()).map(|d| RowOp { thumb: i, op: ScalarOp::Set(d) })
    }
}

/// The row's thumbs on its track.
struct Layout<'a, const N: usize> {
    rules: &'a RowRules<N>,
    track: Track,
    /// Thumbs whose centres are closer than this overlap: a triangle's width.
    overlap: f32,
    /// The press moves only the half of a split thumb it grabs (Alt was held when it began).
    alone: bool,
    /// A wrapping row's period in points; 0 when the row doesn't wrap.
    period: f32,
}

impl<const N: usize> Deref for Layout<'_, N> {
    type Target = RowRules<N>;

    fn deref(&self) -> &RowRules<N> {
        self.rules
    }
}

impl<const N: usize> Layout<'_, N> {
    /// The row value thumb `i` sits at.
    fn at(&self, values: &[f64; N], i: usize) -> f64 {
        self.thumbs[i].follow.map_or(values[i], |f| (f.at)(values, values[i]))
    }

    /// `a - b` in points, the short way round in a wrapping row.
    fn dx(&self, a: f32, b: f32) -> f32 {
        let d = a - b;
        if self.period > 0.0 { d - self.period * (d / self.period).round() } else { d }
    }

    fn xs(&self, values: &[f64; N]) -> [f32; N] {
        std::array::from_fn(|i| self.track.x_of(self.at(values, i)))
    }

    /// What a press at `x` moves, with the thumbs at `values`.
    fn press(&self, values: &[f64; N], x: f32) -> Option<Press> {
        let xs = self.xs(values);
        let d = |i: usize| self.dx(x, xs[i]).abs();
        let nearest = (0..N).min_by(|&a, &b| d(a).total_cmp(&d(b)))?;
        let at = xs[nearest];
        let on = d(nearest) <= self.track.reach;
        // Off the thumbs, thumbs stacked at one place: the side of the press picks, as a drag's
        // direction does on them.
        let group = |i: usize| if on { d(i) <= self.track.reach && self.dx(xs[i], at).abs() < self.overlap } else { self.dx(xs[i], at) == 0.0 };
        let first = (0..N).find(|&i| group(i)).unwrap_or(nearest);
        let last = (0..N).rev().find(|&i| group(i)).unwrap_or(nearest);
        Some(match (on, first == last) {
            (true, true) => Press::Grab(nearest),
            (true, false) => Press::Either { first, last, nearest },
            (false, _) if self.dx(x, at) < 0.0 => Press::Jump(first),
            (false, _) => Press::Jump(last),
        })
    }

    /// The values while `press` at `origin` is dragged to `x`: the moved thumb from `anchor`, the
    /// values the drag began from, the others at `raw`, the host's values when the gesture began
    /// (a gesture the host ended carries on from them, so a value set meanwhile stays). A grabbed
    /// thumb moves by the pointer's movement and stays where it was while the pointer is still; a
    /// jumping one goes to the pointer. The moved value snaps to its step and stays in its bounds
    /// among the others. A half of a split thumb takes its partner along, keeping the spread,
    /// unless the press moves it alone.
    fn moved(&self, anchor: &[f64; N], raw: &[f64; N], press: Press, origin: f32, x: f32) -> [f64; N] {
        let Some(i) = press.thumb(origin, x) else { return *raw };
        let grab = !matches!(press, Press::Jump(_));
        let pair = self.partner(i).filter(|_| !self.alone);
        let mut out = *raw;
        if grab && x == origin {
            out[i] = anchor[i];
            if let Some(j) = pair {
                out[j] = anchor[j];
            }
            return out;
        }
        let start = &self.placed(raw);
        let from = self.placed(anchor);
        let placed_at = if self.anchored.is_some() { self.aim(start, i, self.at(&from, i)) } else { self.at(&from, i) };
        let at = if grab { placed_at + self.track.value_per(x - origin) } else { self.track.value_at(x) };
        let value = self.thumbs[i].follow.map_or(at, |f| (f.value_at)(start, at));
        // A jump in a wrapping row goes the short way round, in an anchored row forward from the
        // first thumb.
        let value = if grab || self.wrap.is_none() {
            value
        } else if self.anchored.is_some() {
            self.aim(start, i, value)
        } else {
            start[i] + self.short(value - start[i])
        };
        if let Some(j) = pair {
            let (lo_i, hi_i) = self.order_bounds(start, i);
            let (lo_j, hi_j) = self.order_bounds(start, j);
            let (lo, hi) = ((lo_i - from[i]).max(lo_j - from[j]), (hi_i - from[i]).min(hi_j - from[j]));
            let lo = lo.min(hi);
            let moved = self.scale_of(i).snap(from[i] + (value - from[i]).clamp(lo, hi)).clamp(from[i] + lo, from[i] + hi);
            out[i] = moved;
            out[j] = from[j] + (moved - from[i]);
            return out;
        }
        out[i] = self.settle(i, value, self.bounds(start, i));
        out
    }
}

impl<const N: usize> ThumbRow<N> {
    /// A row of `thumbs` showing the host's values (`value[i]` is thumb `i`'s) on `range`, read as
    /// [`Slider::new`](super::Slider::new) reads it, with the `step` levels snap to. `id_salt`
    /// keeps the row's gesture and its thumbs' focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, value: [f64; N], range: &RangeInclusive<f64>, step: Option<f64>, thumbs: [Thumb; N]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            scale: Scale::new(range, step),
            thumbs,
            width: None,
            policy: EditPolicy::default(),
            enabled: true,
            part: false,
            wrap: None,
            anchored: None,
        }
    }

    /// Values wrap at `period` (a hue bar of 360), counted from the start of the range: a thumb
    /// dragged or stepped past the end appears at the start. Distances between thumbs, for the
    /// thumb a press grabs and for the bounds of an [`Order::Between`], are measured the short way
    /// round. Home of a thumb without neighbours goes to the range's start, End to its end less a
    /// step (the end is the start again).
    pub fn wrap(mut self, period: f64) -> Self {
        debug_assert!(period.is_finite() && period > 0.0, "a wrapping row needs a positive period, not {period}");
        self.wrap = Some(period).filter(|p| p.is_finite() && *p > 0.0);
        self
    }

    /// Values wrap at `period` ([`wrap`](Self::wrap)) and the thumbs keep their index order going
    /// round the period from thumb `anchor`, which is free to turn (a hue range bar of four
    /// thumbs: outer low, inner low, inner high, outer high). Every other thumb stays between its
    /// neighbours, measured forward from the anchor, and the last at most `max_span` past it. The
    /// anchor goes the short way round and stays at most half a period before the next thumb, and
    /// far enough forward that the last stays within the span. Home and End of a thumb go to its
    /// neighbours' places. An [`Order`] on a thumb of an anchored row has no meaning and stays
    /// `Free`. The values the row reports change only for the thumb that moved.
    ///
    /// The order belongs to the row, which is why this is a row builder and not an [`Order`]
    /// variant: the thumbs share one anchor and one span, and a thumb's bounds depend on all the
    /// others.
    pub fn wrap_from(mut self, period: f64, anchor: usize, max_span: f64) -> Self {
        debug_assert!(anchor < N, "the anchor thumb {anchor} is not in a row of {N}");
        debug_assert!(max_span > 0.0 && max_span < period, "the span {max_span} must be positive and less than the period {period}");
        self = self.wrap(period);
        self.anchored = (self.wrap.is_some() && anchor < N).then_some(Anchored { first: anchor, max_span });
        self
    }

    /// What the values of this row may be, for the tests.
    #[cfg(test)]
    pub(crate) fn row_rules(&self) -> RowRules<N> {
        RowRules { thumbs: self.thumbs.clone(), scale: self.scale, wrap: self.wrap, anchored: self.anchored }
    }

    /// A thumb row's sizes and colours from the theme's `levels.*` tokens, which the rows of a
    /// levels editor use: the look of every row of triangle thumbs.
    pub fn style(tokens: &UiTokens) -> ThumbRowStyle {
        let l = &tokens.component.levels;
        ThumbRowStyle {
            height: l.row,
            thumb: vec2(l.thumb_width, l.thumb_height),
            hit: l.hit,
            edge: l.thumb_edge.to_egui(),
            disabled_edge: l.disabled_edge.to_egui(),
            ring: l.ring.to_egui(),
            ring_width: tokens.component.slider.ring_width,
            low_label: ThumbRowStyle::half_labels().0,
            high_label: ThumbRowStyle::half_labels().1,
        }
    }

    /// The row's width; the available width by default.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// How the gesture reports ([`EditPolicy`]).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// What the values of a row of `thumbs` on `range` with `step` may be, as [`new`](Self::new)
    /// takes them: a composite applies the row's operations with them before drawing it.
    pub(crate) fn rules(range: &RangeInclusive<f64>, step: Option<f64>, thumbs: &[Thumb; N]) -> RowRules<N> {
        RowRules { thumbs: thumbs.clone(), scale: Scale::new(range, step), wrap: None, anchored: None }
    }

    /// Whether the row is a part of a composite reporting through one edit, whose press takes
    /// its turn from the edit order ([`press_turn`]).
    pub(crate) fn part(mut self, part: bool) -> Self {
        self.part = part;
        self
    }

    /// Shows the row in `style` and returns what it reports.
    pub fn show(self, ui: &mut Ui, style: &ThumbRowStyle) -> ThumbRowOutput<N> {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (_, rect) = ui.allocate_space(vec2(width, style.height));
        let id = ui.make_persistent_id(self.id_salt);
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        let response = ui.interact_opt(rect, id, Sense::DRAG, options);
        let sensing = Sensing::of(&response);
        let rules = RowRules { thumbs: self.thumbs, scale: self.scale, wrap: self.wrap, anchored: self.anchored };
        debug_assert!(
            rules.anchored.is_none() || rules.thumbs.iter().all(|t| t.order == Order::Free),
            "an anchored row orders its thumbs itself; their `Order` stays `Free`"
        );
        debug_assert!(
            rules.thumbs.iter().enumerate().all(|(i, t)| !t.split || (i + 1 < N && !rules.thumbs[i + 1].split && rules.wrap.is_none())),
            "a split thumb needs a high half after it, which isn't split itself, in a row that doesn't wrap"
        );
        // Alt decides at the press whether it moves a half alone; the choice stays for the drag.
        let alone_key = id.with("alone");
        if !response.dragged() || response.drag_started() {
            let alt = ui.input(|i| i.modifiers.alt);
            ui.data_mut(|d| d.insert_temp(alone_key, alt));
        }
        let alone = ui.data(|d| d.get_temp::<bool>(alone_key)).unwrap_or(false);
        let track = Track::inset(rect, style.inset(), style.hit, self.scale);
        let period = match self.wrap {
            Some(period) if self.scale.max > self.scale.min => {
                (track.x_of(self.scale.max) - track.x_of(self.scale.min)) * (period / (self.scale.max - self.scale.min)) as f32
            }
            _ => 0.0,
        };
        let layout = Layout { rules: &rules, track, overlap: style.thumb.x, alone, period };
        let ids: [Id; N] = std::array::from_fn(|i| Interactive::id_in(ui, id.with(("thumb", i))));
        let finite = self.value.iter().all(|v| v.is_finite());

        // The gesture lives on the row, which takes every press; the focus goes to the thumbs.
        let mut gesture = Gesture::load(ui, id, &Row(self.value), self.policy).focus_owners(&ids).event_filter(KEY_FILTER).enabled(sensing.enabled);
        let mut ops = Vec::new();
        let (focus, active) = pointer_input(ui, &response, &layout, &mut gesture, PressInput { sensing, part: self.part, finite }, &mut ops);
        let dragged = active.and_then(Press::single);
        let mut focus = focus.map(|i| ids[i]);
        if response.drag_stopped() && focus.is_none() {
            // A release away from the focused thumb would take its focus (egui's click-away).
            focus = ids.into_iter().find(|t| ui.memory(|m| m.has_focus(*t)));
        }

        for (i, &thumb) in ids.iter().enumerate() {
            let reset = rules.reset(i).filter(|_| sensing.enabled && ui.memory(|m| m.has_focus(thumb)));
            // Taken during a drag too, which it doesn't change, as the arrows are.
            if let Some(reset) = reset
                && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Backspace))
                && !sensing.dragging
            {
                gesture.set(Row(rules.apply(&reset, gesture.shown())));
                ops.push(reset);
            }
            let sensing = Sensing { falling: rules.thumbs[i].falling, ..sensing };
            if let Some(op) = stepped(ui, thumb, sensing, None, None, &layout.scale_of(i)) {
                let op = RowOp { thumb: i, op };
                gesture.set(Row(rules.apply(&op, gesture.shown())));
                ops.push(op);
            }
        }

        let values = gesture.shown().0;
        let shown = layout.placed(&values);
        let xs = layout.xs(&shown);
        let halves: [Option<Half>; N] = std::array::from_fn(|i| match rules.partner(i) {
            Some(j) if shown[i] != shown[j] => Some(if j > i { Half::Low } else { Half::High }),
            _ => None,
        });
        let thumb_rect = |x: f32, half: Option<Half>| {
            let (w, h) = (style.thumb.x, style.thumb.y);
            let (left, width) = match half {
                None => (x - w / 2.0, w),
                Some(Half::Low) => (x - w / 2.0, w / 2.0),
                Some(Half::High) => (x, w / 2.0),
            };
            Rect::from_min_size(pos2(left, rect.top()), vec2(width, h))
        };
        let thumbs: [Response; N] = std::array::from_fn(|i| {
            let (lo, hi) = rules.reported_bounds(&shown, i);
            let name = match rules.partner(i) {
                Some(j) if j > i => style.low_label.replace("{name}", &rules.thumbs[i].label),
                Some(j) => style.high_label.replace("{name}", &rules.thumbs[j].label),
                None => rules.thumbs[i].label.clone(),
            };
            let sensed = Interactive::new(id.with(("thumb", i)), Role::Slider, name)
                .sense(Sense::FOCUSABLE)
                .enabled(sensing.enabled)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, thumb_rect(xs[i], halves[i]));
            // A value that isn't finite (an unset gamma) is drawn where its thumb says but reported
            // as mixed, with no value.
            let step = layout.scale_of(i).key_step();
            match values[i].is_finite() {
                true => sensed.numeric(shown[i], lo..=hi, step),
                false => sensed.numeric_mixed(lo..=hi, step),
            }
            .finish(ui)
        });
        if let Some(thumb) = focus
            && !ui.memory(|m| m.has_focus(thumb))
        {
            ui.memory_mut(|m| m.request_focus(thumb));
        }

        if ui.is_rect_visible(rect) {
            // The thumbs a press would move light up while hovered, and the dragged one while pressed.
            let lit = active.or_else(|| response.hover_pos().and_then(|p| layout.press(&shown, p.x)));
            let down = response.is_pointer_button_down_on();
            for (i, thumb) in thumbs.iter().enumerate() {
                let lit_at = |k: usize| lit.is_some_and(|p| p.lights(k));
                let lit = lit_at(i);
                let mut paint = TrianglePaint {
                    face: rules.thumbs[i].face,
                    edge: if sensing.enabled { style.edge } else { style.disabled_edge },
                    ring: StateLayer::color_of(ui, &style.ring, thumb.id, sensing.enabled, lit, lit && down),
                };
                let tip = pos2(xs[i], rect.top());
                match (halves[i], rules.partner(i)) {
                    // Coinciding halves are one triangle, which the low half draws.
                    (None, Some(j)) if j < i => {
                        let bounds = thumb_rect(xs[i], None);
                        FocusRing::paint_radius(ui, bounds, CornerRadius::ZERO, thumb);
                    }
                    (None, Some(j)) => {
                        if paint.ring == Color32::TRANSPARENT {
                            paint.ring = StateLayer::color_of(ui, &style.ring, thumbs[j].id, sensing.enabled, lit_at(j), lit_at(j) && down);
                        }
                        paint_triangle(ui, style, tip, None, paint);
                        FocusRing::paint_radius(ui, thumb_rect(xs[i], None), CornerRadius::ZERO, thumb);
                    }
                    (half, _) => {
                        paint_triangle(ui, style, tip, half, paint);
                        FocusRing::paint_radius(ui, thumb_rect(xs[i], half), CornerRadius::ZERO, thumb);
                    }
                }
            }
        }
        ThumbRowOutput {
            edit: EditResponse::new(response, gesture.finish_as(|row| row.0)),
            #[cfg(test)]
            shown,
            dragged,
            ops,
        }
    }
}

/// What the row's pointer input needs to know besides the pointer.
#[derive(Clone, Copy, Debug)]
struct PressInput {
    sensing: Sensing,
    /// The row is a part of a composite ([`press_turn`]).
    part: bool,
    /// Every host value is a finite number, so a press may begin a gesture.
    finite: bool,
}

/// Presses, drags, the release and taps on the row: the thumb to focus, and what a drag under way
/// moves (the thumb it picked, or overlapping thumbs while it hasn't). A Set's operations go to
/// `ops`.
fn pointer_input<const N: usize>(
    ui: &Ui,
    response: &Response,
    layout: &Layout<'_, N>,
    gesture: &mut Gesture<Row<N>>,
    input: PressInput,
    ops: &mut Vec<RowOp>,
) -> (Option<usize>, Option<Press>) {
    let modifiers = ui.input(|i| i.modifiers);
    let tapped = tap(ui, response);
    let pressed = response.drag_started_by(PointerButton::Primary).then(|| pointer(ui, response).map(|(_, origin)| origin)).flatten();
    let turn = press_turn(ui, response.id, input.part, gesture.id(), pressed.or(tapped));
    let mut focus = None;
    if let PressTurn::Held(at) = turn
        && input.finite
    {
        gesture.begin();
        if let Some(anchor) = gesture.anchor().copied()
            && let Some(press) = layout.press(&layout.placed(&anchor), at.x)
        {
            focus = Some(press.focus());
            gesture.update(Row(layout.moved(&anchor, &anchor, press, at.x, at.x)), modifiers);
        }
        if !response.dragged_by(PointerButton::Primary) {
            gesture.commit();
        }
    }
    let waits = matches!(turn, PressTurn::Waits);
    if let Some(origin) = pressed {
        focus = layout.press(&layout.placed(gesture.shown()), origin.x).map(Press::focus);
        if input.finite && !waits {
            gesture.begin();
        }
    }
    let mut active = None;
    if response.dragged_by(PointerButton::Primary)
        && let Some(anchor) = gesture.anchor().copied()
        && let Some(start) = gesture.start().copied()
        && let Some((pos, origin)) = pointer(ui, response)
        && let Some(press) = layout.press(&layout.placed(&anchor), origin.x)
    {
        active = Some(press);
        if let (Press::Either { .. }, Some(i)) = (press, press.thumb(origin.x, pos.x)) {
            focus = Some(i);
            active = Some(Press::Grab(i));
        }
        gesture.update(Row(layout.moved(&anchor, &start, press, origin.x, pos.x)), modifiers);
    }
    // The reset ends the second click's gesture, or is its own Set.
    let double = double_click(ui, response, layout, gesture, input.sensing, ops);
    if response.drag_stopped() {
        gesture.commit();
    }
    if let Some(at) = tapped {
        let raw = *gesture.shown();
        if let Some(press) = layout.press(&layout.placed(&raw), at.x) {
            focus = Some(press.focus());
            if double.is_none() && !waits {
                // A tap off the thumbs moves the nearest one there; one on a thumb moves none.
                let moved = layout.moved(&raw, &raw, press, at.x, at.x);
                match press {
                    Press::Jump(thumb) => {
                        let op = RowOp { thumb, op: ScalarOp::Set(moved[thumb]) };
                        gesture.set(Row(layout.apply(&op, &raw)));
                        ops.push(op);
                    }
                    _ => gesture.set(Row(moved)),
                }
            }
        }
    }
    (double.or(focus), active)
}

/// A double-click's second click on the row sets the thumb under it to its default: the first
/// click has moved the nearest thumb there. Returns the thumb; the reset goes to `ops`.
fn double_click<const N: usize>(
    ui: &Ui,
    response: &Response,
    layout: &Layout<'_, N>,
    gesture: &mut Gesture<Row<N>>,
    sensing: Sensing,
    ops: &mut Vec<RowOp>,
) -> Option<usize> {
    if !(sensing.enabled && sensing.hovered && ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary))) {
        return None;
    }
    let i = layout.press(&layout.placed(gesture.shown()), response.hover_pos()?.x)?.focus();
    let reset = layout.reset(i)?;
    gesture.set(Row(layout.apply(&reset, gesture.shown())));
    ops.push(reset);
    Some(i)
}

/// A thumb's colours now.
#[derive(Clone, Copy, Debug)]
struct TrianglePaint {
    face: Color32,
    edge: Color32,
    ring: Color32,
}

/// A half of a split thumb, drawn as the half of a triangle on its side of the tip's vertical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Half {
    Low,
    High,
}

/// Paints a triangle thumb, tip up at `tip`, or one `half` of it.
fn paint_triangle(ui: &Ui, style: &ThumbRowStyle, tip: Pos2, half: Option<Half>, paint: TrianglePaint) {
    let (w, h) = (style.thumb.x, style.thumb.y);
    let (left, right) = match half {
        None => (tip.x - w / 2.0, tip.x + w / 2.0),
        Some(Half::Low) => (tip.x - w / 2.0, tip.x),
        Some(Half::High) => (tip.x, tip.x + w / 2.0),
    };
    let points = [tip, pos2(right, tip.y + h), pos2(left, tip.y + h)];
    if paint.ring != Color32::TRANSPARENT {
        // The triangle grown by the ring and the outer half of the edge on every side: its sides
        // move out by `out` when it scales about its incentre by (r + out) / r. The incentre
        // is the corners weighted by the lengths of the opposite sides.
        let len = |a: Pos2, b: Pos2| (a - b).length();
        let (a, b, c) = (len(points[1], points[2]), len(points[0], points[2]), len(points[0], points[1]));
        let perimeter = a + b + c;
        let centre = ((points[0].to_vec2() * a + points[1].to_vec2() * b + points[2].to_vec2() * c) / perimeter).to_pos2();
        let r = (right - left) * h / perimeter;
        let k = (r + EDGE / 2.0 + style.ring_width) / r;
        let ring = points.map(|p| centre + (p - centre) * k).to_vec();
        ui.painter().add(Shape::convex_polygon(ring, paint.ring, Stroke::NONE));
    }
    ui.painter().add(Shape::convex_polygon(points.to_vec(), paint.face, Stroke::new(EDGE, paint.edge)));
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Thumb row"];

#[derive(StoryArgs, Debug, PartialEq)]
struct RowsArgs {
    disabled: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("controls/thumb-row/rows", Level::Controls, "Thumb row", "Rows", RowsArgs { disabled: false }, show)]
}

/// Two free thumbs, a split black and a split white thumb (Alt-drag moves a half alone), a
/// hue range that wraps round, and a four-thumb hue range bar ordered round from its first thumb.
fn show(ui: &mut Ui, args: &mut RowsArgs) {
    let enabled = !args.disabled;
    let style = ThumbRow::<2>::style(&Tokens::of(ui));
    let faces = [Color32::BLACK, Color32::WHITE];
    Stack::new(Gap::Stack).show(ui, |ui| {
        super::slider::with_values(ui, "thumb-row-free", [[0.0, 255.0]], |ui, values| {
            let thumbs = [Thumb::new("Output black", faces[0]).default_value(0.0), Thumb::new("Output white", faces[1]).default_value(255.0)];
            let out = ThumbRow::new("thumb-row-free", values[0], &(0.0..=255.0), Some(1.0), thumbs).enabled(enabled).show(ui, &style);
            super::slider::store(&out.edit, &mut values[0]);
        });
        super::slider::with_values(ui, "thumb-row-split", [[30.0, 60.0, 190.0, 220.0]], |ui, values| {
            let ordered = |below, above| Order::Between { below, above, gap: 0.0 };
            let thumbs = [
                Thumb::new("Black", Color32::from_gray(20)).split().order(ordered(None, Some(2))),
                Thumb::new("Black", Color32::from_gray(20)).order(ordered(Some(0), Some(2))),
                Thumb::new("White", Color32::from_gray(240)).split().order(ordered(Some(1), None)),
                Thumb::new("White", Color32::from_gray(240)).order(ordered(Some(1), None)),
            ];
            let style = ThumbRow::<4>::style(&Tokens::of(ui));
            let out = ThumbRow::new("thumb-row-split", values[0], &(0.0..=255.0), Some(1.0), thumbs).enabled(enabled).show(ui, &style);
            super::slider::store(&out.edit, &mut values[0]);
        });
        super::slider::with_values(ui, "thumb-row-hue", [[330.0, 40.0]], |ui, values| {
            let thumbs = [Thumb::new("Hue from", Color32::from_rgb(220, 60, 60)), Thumb::new("Hue to", Color32::from_rgb(220, 160, 60))];
            let out = ThumbRow::new("thumb-row-hue", values[0], &(0.0..=360.0), Some(1.0), thumbs).wrap(360.0).enabled(enabled).show(ui, &style);
            super::slider::store(&out.edit, &mut values[0]);
        });
        super::slider::with_values(ui, "thumb-row-hue-range", [[340.0, 10.0, 50.0, 80.0]], |ui, values| {
            let thumbs = [
                Thumb::new("Range outer start", Color32::from_gray(150)),
                Thumb::new("Range inner start", Color32::from_gray(230)),
                Thumb::new("Range inner end", Color32::from_gray(230)),
                Thumb::new("Range outer end", Color32::from_gray(150)),
            ];
            let style = ThumbRow::<4>::style(&Tokens::of(ui));
            let row = ThumbRow::new("thumb-row-hue-range", values[0], &(0.0..=360.0), Some(1.0), thumbs).wrap_from(360.0, 0, 359.0);
            let out = row.enabled(enabled).show(ui, &style);
            super::slider::store(&out.edit, &mut values[0]);
        });
    });
}
