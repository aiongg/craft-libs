//! `Slider`: a thumb on a track over an `f64` range. A drag or a press on the track runs `ValueDrag`; arrows, the
//! wheel and AccessKit step the value; a double click or the reset intent restores the binding's reset value.

use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Numeric, PaintUi, PartsCx, Pointer as _,
    Remember as _, Role, Styled as _,
};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::runtime::focus::{KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleColor, StyleCx};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{HitLevel, HitPriority, Metrics, PartKey, PressPhase, Sense, ValueDrag};
use craft_theme::{Stateful, States};
use egui::{CornerRadius, Rect, Vec2};
use std::ops::RangeInclusive;

/// The track the thumb rides on.
pub(crate) const TRACK: PartKey = PartKey::new("track");
/// The thumb.
pub(crate) const THUMB: PartKey = PartKey::new("thumb");

/// The room a slider asks for along its axis, in icon sides.
const LENGTH_IN_ICONS: f32 = 8.0;
/// How far the drawn thumb sits inside its hit square.
const THUMB_INSET: f32 = 2.0;
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
    /// The track.
    track: Stateful<StyleColor>,
    /// The part of the track from the start edge to the thumb.
    filled: Stateful<StyleColor>,
    /// The thumb's fill.
    thumb: Stateful<StyleColor>,
    /// The thumb's edge.
    thumb_edge: Stateful<StyleColor>,
    /// The thickness of the track.
    thickness: f32,
    /// The diameter of the drawn thumb.
    diameter: f32,
}

/// The value at the start of the running drag.
#[derive(Clone, Default)]
struct DragStart(f64);

/// A slider over `range`.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Slider<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
    range: RangeInclusive<f64>,
    /// The change of one arrow press; the default is a hundredth of the range.
    #[prop]
    step: Option<f64>,
}

impl<'a> Slider<'a> {
    /// A slider of `value` over `range`.
    pub fn new(value: Bind<'a, f64>, range: RangeInclusive<f64>) -> Self {
        Slider { common: Common::new(), value, range, step: None }
    }

    fn small(&self) -> f64 {
        self.step.unwrap_or((self.range.end() - self.range.start()) * STEP_SHARE)
    }

    fn middle(&self) -> f64 {
        (self.range.start() + self.range.end()) / 2.0
    }

    /// Where along the travel the value stands, 0 at the start edge.
    fn fraction(&self, value: f64) -> f32 {
        let span = self.range.end() - self.range.start();
        if span > 0.0 { ((value - self.range.start()) / span).clamp(0.0, 1.0) as f32 } else { 0.0 }
    }

    fn thumb_rect(&self, value: f64, rect: Rect, flow: craft_behavior::Flow) -> Rect {
        let hit = rect.height();
        flow.span(self.fraction(value) * (rect.width() - hit).max(0.0), hit)
    }
}

impl Component for Slider<'_> {
    const ROLE: Role = Role::SLIDER;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows);
    type Style = SliderStyle;
    type Output = EditResponse<f64>;

    fn style(cx: &StyleCx<'_>, _: SliderStyleKey) -> SliderStyle {
        let control = cx.control();
        let colors = &cx.tokens.color;
        let accent = &colors.accent;
        let disabled = colors.state.disabled_fill;
        let states = |rest, hover, pressed| {
            Stateful::new(cx.color(rest)).off(States::new(cx.color(rest), cx.color(hover), cx.color(pressed), cx.color(disabled)))
        };
        SliderStyle {
            metrics: Metrics::new().min_size(Vec2::new(control.icon * LENGTH_IN_ICONS, control.height)),
            focus: super::focus_look(cx),
            track: states(colors.surface.sunken, colors.surface.sunken, colors.surface.sunken),
            filled: states(accent.strong, accent.strong_hover, accent.strong_pressed),
            thumb: states(colors.surface.knob, colors.surface.knob, colors.surface.knob),
            thumb_edge: states(colors.line.knob_edge, accent.strong_hover, accent.strong_pressed),
            thickness: cx.tokens.stroke.thick,
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
        let value = cx.value().copied().unwrap_or_else(|| self.middle());
        let thumb = self.thumb_rect(value, rect, cx.flow);
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
        let (min, max) = (*self.range.start(), *self.range.end());
        let (rtl, half) = (cx.flow.is_rtl(), cx.rect.height() / 2.0);
        let drag = ValueDrag::absolute((cx.rect.left() + half)..(cx.rect.right() - half), self.range.clone());
        for key in [THUMB, TRACK] {
            let press = cx.part(key).press;
            let start = cx.memory::<DragStart>().0;
            match press.phase {
                PressPhase::DragStart { .. } => {
                    let from = cx.value().copied().unwrap_or_else(|| self.middle());
                    cx.memory::<DragStart>().0 = from;
                    cx.begin();
                }
                PressPhase::Drag { .. } => {
                    if let Some(next) = drag.value(start, &press, rtl) {
                        cx.propose(next);
                    }
                }
                PressPhase::Release { click, .. } => {
                    let running = cx.gesture().is_some();
                    if let Some(next) = drag.value(start, &press, rtl) {
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
        let value = cx.value().copied().unwrap_or_else(|| self.middle());
        let small = self.small();
        while let Some((mv, mods)) = cx.intents().take_move() {
            let steps = ValueSteps::new(value, min, max, small, small * LARGE_STEPS);
            if let StepResult::Set(next) = step_value(&steps, ValueAxes::Both, mv, mods) {
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
            cx.set((current + f64::from(along.signum()) * small).clamp(min, max));
        }
        if let Some(next) = cx.intents().take_set_value() {
            cx.set(next.clamp(min, max));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let range = self.range.clone();
        let numeric = match cx.value() {
            Some(value) => Numeric::new(*value, range),
            None => Numeric::empty(range),
        };
        access.numeric(numeric.step(self.small()));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let draw = ui.draw();
        let flow = cx.flow;
        let value = cx.value().copied();
        let track = Rect::from_center_size(cx.rect.center(), Vec2::new(cx.rect.width(), style.thickness));
        let round = CornerRadius::same((style.thickness / 2.0).round() as u8);
        draw.fill(&PaintShape::Rect(track, round), cx.resolve(&style.track));
        let thumb = self.thumb_rect(value.unwrap_or_else(|| self.middle()), cx.rect, flow);
        if value.is_some() {
            let reach = (thumb.center().x - flow.start_x()).abs();
            let filled = craft_behavior::Flow::new(track, flow.is_rtl()).span(0.0, reach);
            draw.fill(&PaintShape::Rect(filled, round), cx.resolve(&style.filled));
        }
        let knob = PaintShape::Circle(thumb.center(), style.diameter / 2.0);
        draw.fill(&knob, if value.is_some() { cx.resolve_part(THUMB, &style.thumb) } else { cx.resolve(&style.track) });
        draw.stroke(&knob, StrokeWidth::Thin, cx.resolve_part(THUMB, &style.thumb_edge));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}
