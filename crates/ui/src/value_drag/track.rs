//! What `Slider`, `RangeSlider` and `ThumbRow` share: the scale that spreads a range over a track, the groove and fill,
//! marks, and the press resolver that says which thumb a press moves and how.

use crate::contract::{PaintUi, Show};
use crate::primitives::{Draw, PaintShape, TextRun};
use crate::runtime::env::Label;
use crate::style::{StyleColor, StyleCx, TextLook, TrackLook, TypeRole};
use craft_behavior::intent::{MoveMods, NavMove};
use craft_behavior::nav::{LARGE_STEPS, STEP_SHARE, StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{Flow, PartKey, PartRef, PressPhase, PressState, ValueDrag, Wheel, wheel_step};
use craft_theme::{Rgba, Stateful, States};
use egui::{CornerRadius, Pos2, Rect, Vec2};
use std::ops::{Range, RangeInclusive};

/// The part a track face paints as: the groove across the middle of the control.
pub(crate) const GROOVE: PartKey = PartKey::new("groove");

/// A colour that rests, lights on hover and press, and takes the disabled fill when disabled.
pub(crate) fn states(cx: &StyleCx<'_>, rest: Rgba, hover: Rgba, pressed: Rgba) -> Stateful<StyleColor> {
    let (rest, hover, pressed, disabled) = (cx.color(rest), cx.color(hover), cx.color(pressed), cx.color(cx.tokens.color.state.disabled_fill));
    Stateful::new(rest).off(States::new(rest, hover, pressed, disabled))
}

/// The look of the labels under labelled marks: caption type in the muted text colour.
pub(crate) fn mark_text(cx: &StyleCx<'_>) -> TextLook {
    TextLook::new(TypeRole::Caption, Stateful::new(cx.color(cx.tokens.color.text.muted)))
}

/// The groove across the middle of `rect`.
pub(crate) fn groove_rect(look: &TrackLook, rect: Rect) -> Rect {
    Rect::from_center_size(rect.center(), Vec2::new(rect.width(), look.thickness))
}

/// The groove across the middle of `rect` when a gradient ramp is its face.
pub(crate) fn ramp_rect(look: &TrackLook, rect: Rect) -> Rect {
    Rect::from_center_size(rect.center(), Vec2::new(rect.width(), look.ramp))
}

/// A caller's look for the groove: a component shown as the face of the `GROOVE` part, such as a colour gradient. It
/// replaces the groove and the fill.
pub struct TrackFace<'a>(FaceFn<'a>);

type FaceFn<'a> = Box<dyn Fn(&mut PaintUi<'_>, PartRef) + 'a>;

impl<'a> TrackFace<'a> {
    /// A track painted by `face`.
    pub fn new<C: Show + Clone + 'a>(face: C) -> Self {
        TrackFace(Box::new(move |ui, part| ui.face(part, face.clone())))
    }

    pub(crate) fn paint(&self, ui: &mut PaintUi<'_>) {
        (self.0)(ui, GROOVE.into());
    }
}

/// The room marks keep: the gap below a tick before its label, and how near a mark a drag snaps to it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MarkSpace {
    pub label_gap: f32,
    pub snap: f32,
}

impl MarkSpace {
    pub fn new(cx: &StyleCx<'_>) -> Self {
        MarkSpace { label_gap: cx.tokens.space.gap.tight, snap: cx.tokens.size.handle.md }
    }
}

/// Marks and ticks along a track.
#[derive(Clone, Default)]
pub(crate) struct Marks {
    space: MarkSpace,
    /// The values with a mark, which a drag snaps to.
    values: Vec<f64>,
    /// The labels under marks, with their values.
    labels: Vec<(f64, Label)>,
    /// Evenly spaced ticks: one more than this, none for 0.
    pub ticks: usize,
    /// The labels resolved and measured by the last `lay_out`.
    shaped: Vec<(f64, String, Vec2)>,
}

impl Marks {
    pub fn set(&mut self, values: &[f64]) {
        self.values = values.iter().copied().filter(|value| value.is_finite()).collect();
    }

    pub fn set_labelled<L: Into<Label>>(&mut self, marks: impl IntoIterator<Item = (f64, L)>) {
        self.labels = marks.into_iter().map(|(value, label)| (value, label.into())).filter(|(value, _)| value.is_finite()).collect();
        self.values = self.labels.iter().map(|(value, _)| *value).collect();
    }

    /// Resolves and measures the labels in the caption role.
    pub fn lay_out(&mut self, look: &TextLook, space: MarkSpace, resolve: impl Fn(&Label) -> Label, measure: impl Fn(&TextRun<'_>) -> Vec2) {
        self.space = space;
        self.shaped = self
            .labels
            .iter()
            .map(|(value, label)| {
                let text = resolve(label).as_str().to_owned();
                let size = measure(&TextRun::new(text.clone()).role(look.role));
                (*value, text, size)
            })
            .collect();
    }

    /// The room the labels take below the track: their tallest line and the gaps around it; none without labels.
    pub fn below(&self, look: &TrackLook) -> f32 {
        let tallest = self.shaped.iter().map(|(_, _, size)| size.y).fold(0.0, f32::max);
        if self.shaped.is_empty() { 0.0 } else { tallest + look.mark_length + self.space.label_gap }
    }

    /// The part of `rect` the track and thumbs use: all of it, less the room of the labels.
    pub fn area(&self, rect: Rect, look: &TrackLook) -> Rect {
        Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.max.y - self.below(look)))
    }

    /// The marked value whose position along a track of `length` is within the snap distance of `position`.
    pub fn snapped(&self, scale: &Scale, length: f32, position: f64) -> Option<f64> {
        self.values
            .iter()
            .copied()
            .filter(|value| (scale.min..=scale.max).contains(value))
            .map(|value| (value, (scale.position(value) - position).abs() * f64::from(length)))
            .filter(|(_, distance)| *distance <= f64::from(self.space.snap))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(value, _)| value)
    }
}

/// Where positions lie along a track: the area it runs in, and the room a thumb keeps at each end.
#[derive(Clone, Copy)]
pub(crate) struct Along {
    pub area: Rect,
    pub flow: Flow,
}

impl Along {
    /// The x of `position` (0 to 1) along the track, mirrored under right-to-left.
    fn x(&self, position: f64) -> f32 {
        let half = self.area.height() / 2.0;
        let length = (self.area.width() - self.area.height()).max(0.0);
        self.flow.span(half + position as f32 * length, 0.0).center().x
    }
}

/// The colours of marks and their labels.
pub(crate) struct MarkInk {
    pub marks: StyleColor,
    pub labels: StyleColor,
    pub role: TypeRole,
}

/// Paints the ticks, the marks and the labels under the groove of `along`, the labels kept inside `whole`.
pub(crate) fn paint_marks(draw: &Draw<'_>, look: &TrackLook, marks: &Marks, scale: &Scale, along: Along, whole: Rect, ink: &MarkInk) {
    let top = groove_rect(look, along.area).bottom() + look.mark_length;
    let ticks = (marks.ticks > 0).then(|| (0..=marks.ticks).map(|at| at as f64 / marks.ticks as f64));
    let positions = ticks
        .into_iter()
        .flatten()
        .chain(marks.values.iter().filter(|value| (scale.min..=scale.max).contains(value)).map(|value| scale.position(*value)));
    for position in positions {
        let line = Rect::from_min_size(Pos2::new(along.x(position) - look.mark_width / 2.0, top), Vec2::new(look.mark_width, look.mark_length));
        draw.fill(&PaintShape::Rect(line, CornerRadius::ZERO), ink.marks);
    }
    let label_top = top + look.mark_length + marks.space.label_gap;
    for (value, text, size) in &marks.shaped {
        if !(scale.min..=scale.max).contains(value) {
            continue;
        }
        let left = (along.x(scale.position(*value)) - size.x / 2.0).clamp(whole.left(), (whole.right() - size.x).max(whole.left()));
        draw.text(Rect::from_min_size(Pos2::new(left, label_top), *size), &TextRun::new(text.as_str()).role(ink.role).color(ink.labels));
    }
}

/// The part of the groove painted in `color`, from `from` to `to` along the reading direction, measured from the start edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fill {
    pub color: StyleColor,
    pub from: f32,
    pub to: f32,
}

/// Paints the groove across the middle of `rect` in `groove`, and `fill` over it.
pub(crate) fn paint(draw: &Draw<'_>, look: &TrackLook, rect: Rect, flow: Flow, groove: StyleColor, fill: Option<Fill>) {
    let track = groove_rect(look, rect);
    let round = CornerRadius::same((look.thickness / 2.0).round() as u8);
    draw.fill(&PaintShape::Rect(track, round), groove);
    if let Some(fill) = fill {
        let filled = Flow::new(track, flow.is_rtl()).span(fill.from, (fill.to - fill.from).max(0.0));
        draw.fill(&PaintShape::Rect(filled, round), fill.color);
    }
}

/// How positions along a slider's track map to values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum SliderScale {
    /// Equal distances are equal changes.
    #[default]
    Linear,
    /// Equal distances are equal ratios, as brush sizes want. The range must start above zero.
    Log,
    /// The value grows with the position raised to the exponent: above 1 the start edge gets more room, below 1 the
    /// end edge. The exponent must be positive.
    Power(f32),
}

/// A range with an optional step and the scale that spreads it over a track.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scale {
    pub min: f64,
    pub max: f64,
    step: Option<f64>,
    kind: SliderScale,
}

/// Whether a count of steps from the start is a whole number, allowing for rounding.
fn on_stop(at: f64) -> bool {
    (at - at.round()).abs() < 1e-9
}

/// One Page key's change along a log track: a tenth of the track.
const LARGE_POSITION: f64 = 0.1;

impl Scale {
    /// The range in ascending order. A step that is not a positive number is none. A scale that does not fit the
    /// range stays linear.
    pub fn new(range: &RangeInclusive<f64>, step: Option<f64>, kind: SliderScale) -> Self {
        let (a, b) = (*range.start(), *range.end());
        debug_assert!(a.is_finite() && b.is_finite(), "a slider's range needs finite bounds, not {a}..={b}");
        let (min, max) = if a <= b { (a, b) } else { (b, a) };
        let fits = match kind {
            SliderScale::Linear => true,
            SliderScale::Power(k) => k.is_finite() && k > 0.0,
            SliderScale::Log => min > 0.0 && max > min,
        };
        debug_assert!(fits, "a slider's {kind:?} scale does not fit {min}..={max}");
        Scale { min, max, step: step.filter(|step| step.is_finite() && *step > 0.0), kind: if fits { kind } else { SliderScale::Linear } }
    }

    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Where `value` lies along the track, 0 at the start edge and 1 at the end edge.
    pub fn position(&self, value: f64) -> f64 {
        if self.span() <= 0.0 {
            return 0.0;
        }
        let linear = ((value - self.min) / self.span()).clamp(0.0, 1.0);
        match self.kind {
            SliderScale::Linear => linear,
            SliderScale::Power(k) => linear.powf(1.0 / f64::from(k)),
            SliderScale::Log => ((value.max(self.min) / self.min).ln() / (self.max / self.min).ln()).clamp(0.0, 1.0),
        }
    }

    /// The value at `position` along the track, unsnapped: the inverse of [`Scale::position`].
    pub fn at_position(&self, position: f64) -> f64 {
        let t = position.clamp(0.0, 1.0);
        match self.kind {
            SliderScale::Linear => self.min + t * self.span(),
            SliderScale::Power(k) => self.min + t.powf(f64::from(k)) * self.span(),
            SliderScale::Log => self.min * (self.max / self.min).powf(t),
        }
    }

    /// The value in the range, and at the start when it is not a finite number.
    pub fn place(&self, value: f64) -> f64 {
        if value.is_finite() { value.clamp(self.min, self.max) } else { self.min }
    }

    /// The value halfway along the track.
    pub fn middle(&self) -> f64 {
        self.at_position(0.5)
    }

    /// The value on the nearest stop when there is a step: a multiple of the step from the start, or the end. A value
    /// on a stop stays as it is.
    pub fn snap(&self, value: f64) -> f64 {
        let value = self.place(value);
        let Some(step) = self.step else { return value };
        let at = (value - self.min) / step;
        if value == self.max || on_stop(at) {
            return value;
        }
        let below = (self.min + at.floor() * step).min(self.max);
        let above = (below + step).min(self.max);
        if value - below < above - value { below } else { above }
    }

    /// The change of one arrow press: the step, or a hundredth of the range.
    pub fn small(&self) -> f64 {
        self.step.unwrap_or(self.span() * STEP_SHARE)
    }

    /// Whether keys and the wheel move along the track instead of through values: a log scale with no step, where a
    /// fixed change of value would be too coarse at the start and too fine at the end.
    fn by_position(&self) -> bool {
        self.kind == SliderScale::Log && self.step.is_none()
    }

    /// The change of one arrow press along the track on a log scale: a tenth of a decade.
    fn small_position(&self) -> f64 {
        0.1 / (self.max / self.min).log10()
    }

    /// The steps from `value` between `low` and `high`: over values, or over positions along the track where keys
    /// move by position. `notch` is one wheel notch in values; a positional scale takes its own small step.
    fn steps(&self, value: f64, (low, high): (f64, f64), notch: Option<f64>) -> ValueSteps {
        if self.by_position() {
            return ValueSteps::new(self.position(value), self.position(low), self.position(high), self.small_position(), LARGE_POSITION);
        }
        let small = self.small();
        let steps = ValueSteps::new(value, low, high, small, small * LARGE_STEPS);
        match notch {
            Some(notch) => steps.notch(notch),
            None => steps,
        }
    }

    /// The value a result of [`Scale::steps`] stands for, on its stops and between `low` and `high`.
    fn settled(&self, next: f64, (low, high): (f64, f64)) -> f64 {
        let value = if self.by_position() { self.at_position(next) } else { next };
        self.snap(value).clamp(low, high)
    }

    /// The value a move gives from `value`, kept between `low` and `high`.
    pub fn stepped(&self, value: f64, bounds: (f64, f64), mv: NavMove, mods: MoveMods) -> Option<f64> {
        let StepResult::Set(next) = step_value(&self.steps(value, bounds, None), ValueAxes::Both, mv, mods) else { return None };
        Some(self.settled(next, bounds))
    }

    /// The value a wheel event gives from `value`, kept between `low` and `high`: one notch is `notch` values, the
    /// step by default.
    pub fn wheeled(&self, value: f64, bounds: (f64, f64), wheel: &Wheel, rtl: bool, notch: Option<f64>) -> Option<f64> {
        let StepResult::Set(next) = wheel_step(&self.steps(value, bounds, notch), ValueAxes::Both, wheel, rtl) else { return None };
        Some(self.settled(next, bounds))
    }
}

/// The square a thumb at `value` takes along `rect`: as tall as the rect, its centre moving from half a thumb inside
/// the start edge to half a thumb inside the end edge.
pub(crate) fn thumb_rect(scale: &Scale, value: f64, rect: Rect, flow: Flow) -> Rect {
    let hit = rect.height();
    flow.span(scale.position(value) as f32 * (rect.width() - hit).max(0.0), hit)
}

/// What a press on a track moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Press {
    /// The thumb under the press, by the pointer's travel: a click on it changes nothing.
    Grab(usize),
    /// The nearest thumb, to the pointer: the press is off the thumbs.
    Jump(usize),
    /// Overlapping thumbs under the press: a left drag moves the first of them, a right drag the last.
    Either { first: usize, last: usize },
    /// Between two thumbs, where the track allows it (`Pick::span`): the thumbs move together by the pointer's travel.
    Span,
}

impl Press {
    /// The thumb the press moves with the pointer at `x`, from `origin`.
    pub fn thumb(self, origin: f32, x: f32) -> Option<usize> {
        match self {
            Press::Grab(i) | Press::Jump(i) => Some(i),
            Press::Either { first, .. } if x < origin => Some(first),
            Press::Either { last, .. } if x > origin => Some(last),
            Press::Either { .. } | Press::Span => None,
        }
    }

    /// The press moves a thumb by the pointer's travel instead of taking it to the pointer.
    pub fn grabs(self) -> bool {
        !matches!(self, Press::Jump(_))
    }
}

/// Where the pointer was when the press began and where it is, once a drag has started.
pub(crate) fn pointer(state: &PressState) -> Option<(Pos2, Pos2)> {
    match state.phase {
        PressPhase::DragStart { start, pos, .. } | PressPhase::Drag { start, pos, .. } | PressPhase::Release { start, pos, .. } => {
            Some((start.origin, pos))
        }
        _ => None,
    }
}

/// Which thumb a press at a point moves, for thumbs at given places along a track.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pick {
    /// A press within this distance of a thumb's centre grabs it.
    pub reach: f32,
    /// Thumbs whose centres are closer than this overlap.
    pub overlap: f32,
    /// A wrapping track's period in points; 0 when it does not wrap.
    pub period: f32,
    /// A press between two thumbs drags them together instead of moving the nearer one.
    pub span: bool,
}

impl Pick {
    /// `a - b` in points, the short way round on a wrapping track.
    fn dx(&self, a: f32, b: f32) -> f32 {
        let d = a - b;
        if self.period > 0.0 { d - self.period * (d / self.period).round() } else { d }
    }

    /// What a press at `x` moves, with the thumbs' centres at `xs`.
    pub fn press(&self, xs: &[f32], x: f32) -> Option<Press> {
        let d = |i: usize| self.dx(x, xs[i]).abs();
        let nearest = (0..xs.len()).min_by(|&a, &b| d(a).total_cmp(&d(b)))?;
        let at = xs[nearest];
        let on = d(nearest) <= self.reach;
        // Thumbs stacked at one place: the side of the press picks, as a drag's direction does on them.
        let group = |i: usize| if on { d(i) <= self.reach && self.dx(xs[i], at).abs() < self.overlap } else { self.dx(xs[i], at) == 0.0 };
        let first = (0..xs.len()).find(|&i| group(i)).unwrap_or(nearest);
        let last = (0..xs.len()).rev().find(|&i| group(i)).unwrap_or(nearest);
        let between = xs.iter().any(|&other| other < x) && xs.iter().any(|&other| other > x);
        Some(match (on, first == last) {
            (false, _) if self.span && between => Press::Span,
            (true, true) => Press::Grab(nearest),
            (true, false) => Press::Either { first, last },
            (false, _) if self.dx(x, at) < 0.0 => Press::Jump(first),
            (false, _) => Press::Jump(last),
        })
    }
}

/// Turns a press into the value of the thumb it moves, in `units` along a track whose thumbs' centres run over `track`.
pub(crate) struct Travel {
    pub track: Range<f32>,
    pub units: RangeInclusive<f64>,
    pub rtl: bool,
}

impl Travel {
    /// The value a press gives the thumb that was at `from` when the gesture began, `None` while it gives none. A
    /// grab moves the thumb by the pointer's travel, Shift ten times as far; a jump puts it at the pointer. Alt
    /// scales a drag finer when `fine`; a click passes false, so it lands where it points.
    pub fn value(&self, press: Press, from: f64, state: &PressState, fine: bool) -> Option<f64> {
        let drag = if press.grabs() {
            let length = f64::from(self.track.end - self.track.start);
            let rate = if length > 0.0 { (self.units.end() - self.units.start()) / length } else { 0.0 };
            ValueDrag::relative(rate, f64::NEG_INFINITY..=f64::INFINITY).coarse(LARGE_STEPS)
        } else {
            ValueDrag::absolute(self.track.clone(), self.units.clone())
        };
        let drag = if fine { drag } else { drag.fine(1.0) };
        drag.value(from, state, self.rtl)
    }
}
