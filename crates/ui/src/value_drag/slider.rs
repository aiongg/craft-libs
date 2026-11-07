//! `Slider`: a thumb on a track over an `f64` range. A drag or a press on the track runs `ValueDrag`; arrows, the
//! wheel and AccessKit step the value; a double click or the reset intent restores the binding's reset value. The
//! track maps positions to values through a [`SliderScale`], linear by default.

use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Numeric, PaintUi, PartsCx, Pointer as _,
    Remember as _, Role, Styled as _,
};
use super::track::{self, Fill, TrackLook};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::runtime::focus::{ActivateKeys, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleColor, StyleCx};
use craft_behavior::intent::{MoveMods, NavMove};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{HitLevel, HitPriority, Metrics, PartKey, PressPhase, Sense, ValueDrag};
use craft_theme::Stateful;
use egui::{Rect, Vec2};
use std::ops::RangeInclusive;

#[cfg(feature = "testing")]
mod stories;

/// The track the thumb rides on.
pub(crate) const TRACK: PartKey = PartKey::new("track");
/// The thumb.
pub(crate) const THUMB: PartKey = PartKey::new("thumb");

/// The room a slider asks for along its axis, in icon sides.
pub(super) const LENGTH_IN_ICONS: f32 = 8.0;
/// How far the drawn thumb sits inside its hit square.
pub(super) const THUMB_INSET: f32 = 2.0;
/// An arrow press steps this share of the range; Shift and the Page keys step ten times that.
const STEP_SHARE: f64 = 0.01;
/// How many small steps a large step is.
const LARGE_STEPS: f64 = 10.0;

/// The style of a slider.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SliderStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The track and the part of it from the start edge to the thumb.
    track: TrackLook,
    /// The thumb's fill.
    thumb: Stateful<StyleColor>,
    /// The thumb's edge.
    thumb_edge: Stateful<StyleColor>,
    /// The diameter of the drawn thumb.
    diameter: f32,
}

/// The value at the start of the running drag.
#[derive(Clone, Default)]
struct DragStart(f64);

/// How positions along a slider's track map to values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

    fn span(&self) -> f64 {
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

    /// The value a move gives from `value`, kept between `low` and `high`.
    pub fn stepped(&self, value: f64, (low, high): (f64, f64), mv: NavMove, mods: MoveMods) -> Option<f64> {
        let next = if self.by_position() {
            let steps = ValueSteps::new(self.position(value), self.position(low), self.position(high), self.small_position(), LARGE_POSITION);
            let StepResult::Set(next) = step_value(&steps, ValueAxes::Both, mv, mods) else { return None };
            self.at_position(next)
        } else {
            let steps = ValueSteps::new(value, low, high, self.small(), self.small() * LARGE_STEPS);
            let StepResult::Set(next) = step_value(&steps, ValueAxes::Both, mv, mods) else { return None };
            next
        };
        Some(self.snap(next).clamp(low, high))
    }

    /// The value one small step from `value` in `direction` (1 or -1), kept between `low` and `high`.
    pub fn nudged(&self, value: f64, direction: f64, (low, high): (f64, f64)) -> f64 {
        let next = if self.by_position() {
            self.at_position(self.position(value) + direction * self.small_position())
        } else {
            value + direction * self.small()
        };
        self.snap(next).clamp(low, high)
    }
}

/// One Page key's change along a log track: a tenth of the track.
const LARGE_POSITION: f64 = 0.1;

/// A slider over `range`.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Slider<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
    range: RangeInclusive<f64>,
    /// The change of one arrow press, and the stops the value snaps to; the default is a hundredth of the range and
    /// no stops.
    #[prop(quantity = "step")]
    step: Option<f64>,
    /// How positions along the track map to values.
    #[prop]
    scale: SliderScale,
}

impl<'a> Slider<'a> {
    /// A slider of `value` over `range`.
    pub fn new(value: Bind<'a, f64>, range: RangeInclusive<f64>) -> Self {
        Slider { common: Common::new(), value, range, step: None, scale: SliderScale::Linear }
    }

    fn scale(&self) -> Scale {
        Scale::new(&self.range, self.step, self.scale)
    }
}

/// The square a thumb at `value` takes along `rect`: as tall as the rect, its centre moving from half a thumb inside
/// the start edge to half a thumb inside the end edge.
pub(super) fn thumb_rect(scale: &Scale, value: f64, rect: Rect, flow: craft_behavior::Flow) -> Rect {
    let hit = rect.height();
    flow.span(scale.position(value) as f32 * (rect.width() - hit).max(0.0), hit)
}

impl Component for Slider<'_> {
    const ROLE: Role = Role::SLIDER;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows).with_activate(ActivateKeys::Neither);
    type Style = SliderStyle;
    type Output = EditResponse<f64>;

    fn style(cx: &StyleCx<'_>, _: SliderStyleKey) -> SliderStyle {
        let control = cx.control();
        let colors = &cx.tokens.color;
        let accent = &colors.accent;
        SliderStyle {
            metrics: Metrics::new().min_size(Vec2::new(control.icon * LENGTH_IN_ICONS, control.height)),
            focus: crate::controls::focus_look(cx),
            track: TrackLook::new(cx),
            thumb: track::states(cx, colors.surface.knob, colors.surface.knob, colors.surface.knob),
            thumb_edge: track::states(cx, colors.line.knob_edge, accent.strong_hover, accent.strong_pressed),
            diameter: control.icon - THUMB_INSET * 2.0,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.icon() * LENGTH_IN_ICONS, cx.icon())
    }

    fn sense(&self) -> Sense {
        Sense::FOCUS | Sense::WHEEL | Sense::WHEEL_X
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let scale = self.scale();
        let value = cx.value().copied().unwrap_or_else(|| scale.middle());
        let thumb = thumb_rect(&scale, value, rect, cx.flow);
        cx.part(TRACK, rect).sense(Sense::CLICK | Sense::DRAG).value_part().pointer_only("the arrow keys step the value");
        cx.part(THUMB, thumb)
            .sense(Sense::DRAG)
            .priority(HitPriority::new(HitLevel::Handle, 0))
            .value_part()
            .pointer_only("the arrow keys step the value");
    }

    fn bind(&mut self) -> Option<BindMut<'_, f64>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.resolved.read_only || !cx.resolved.enabled {
            return;
        }
        let scale = self.scale();
        let bounds = (scale.min, scale.max);
        let (rtl, half) = (cx.flow.is_rtl(), cx.rect.height() / 2.0);
        // The drag runs over positions along the track, so every scale shares the maths.
        let drag = ValueDrag::absolute((cx.rect.left() + half)..(cx.rect.right() - half), 0.0..=1.0);
        let at = |position: Option<f64>| position.map(|position| scale.snap(scale.at_position(position)));
        for key in [THUMB, TRACK] {
            let press = cx.part(key).press;
            let start = scale.position(cx.memory::<DragStart>().0);
            match press.phase {
                PressPhase::DragStart { .. } => {
                    let from = cx.value().copied().unwrap_or_else(|| scale.middle());
                    cx.memory::<DragStart>().0 = from;
                    cx.begin();
                }
                PressPhase::Drag { .. } => {
                    if let Some(next) = at(drag.value(start, &press, rtl)) {
                        cx.propose(next);
                    }
                }
                PressPhase::Release { click, .. } => {
                    let running = cx.gesture().is_some();
                    if let Some(next) = at(drag.value(start, &press, rtl)) {
                        if running {
                            cx.propose(next);
                        } else if click {
                            cx.set(next);
                        }
                    }
                    if running {
                        cx.commit();
                    }
                }
                _ => {}
            }
        }
        let value = cx.value().copied().unwrap_or_else(|| scale.middle());
        while let Some((mv, mods)) = cx.intents().take_move() {
            if let Some(next) = scale.stepped(value, bounds, mv, mods) {
                cx.set(next);
            }
        }
        if let Some(wheel) = cx.take_wheel() {
            let along = if wheel.scroll.y != 0.0 {
                wheel.scroll.y
            } else if rtl {
                -wheel.scroll.x
            } else {
                wheel.scroll.x
            };
            let current = cx.value().copied().unwrap_or(value);
            cx.set(scale.nudged(current, f64::from(along.signum()), bounds));
        }
        if let Some(next) = cx.intents().take_set_value() {
            cx.set(scale.snap(next));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let scale = self.scale();
        let range = scale.min..=scale.max;
        let numeric = match cx.value() {
            Some(value) => Numeric::new(*value, range),
            None => Numeric::empty(range),
        };
        access.numeric(numeric.step(scale.small()));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let draw = ui.draw();
        let flow = cx.flow;
        let scale = self.scale();
        let value = cx.value().copied();
        let thumb = thumb_rect(&scale, value.unwrap_or_else(|| scale.middle()), cx.rect, flow);
        let fill = value.map(|_| Fill { color: cx.resolve(&style.track.filled), from: 0.0, to: (thumb.center().x - flow.start_x()).abs() });
        track::paint(&draw, &style.track, cx.rect, flow, cx.resolve(&style.track.groove), fill);
        let knob = PaintShape::Circle(thumb.center(), style.diameter / 2.0);
        draw.fill(&knob, if value.is_some() { cx.resolve_part(THUMB, &style.thumb) } else { cx.resolve(&style.track.groove) });
        draw.stroke(&knob, StrokeWidth::Thin, cx.resolve_part(THUMB, &style.thumb_edge));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}
