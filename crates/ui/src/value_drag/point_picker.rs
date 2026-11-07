//! `PointPicker`: a point on a well, over an image or a checkerboard. A press on the well jumps the point there, a drag
//! on the point or the well follows the pointer, and the arrow keys move it along one axis each. The X and Y fields
//! are `NumberField`s the caller shows beside it, bound to the same value.

use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Numeric, PaintUi, PartsCx, Pointer as _,
    Remember as _, Role, WordSetCx as _,
};
use crate::primitives::{ContentPainterCx, PaintShape};
use crate::runtime::focus::{ActivateKeys, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleCx};
use craft_behavior::{Axis, HitLevel, HitPriority, Metrics, MoveMods, NavMove, PartKey, PressPhase, PressState, Sense, ValueDrag};
use egui::{Pos2, Rect, Vec2, pos2};
use std::ops::RangeInclusive;

#[cfg(feature = "testing")]
mod stories;

/// The well: a press on it moves the point there.
const WELL: PartKey = PartKey::new("well");
/// The point: a drag on it follows the pointer.
const THUMB: PartKey = PartKey::new("thumb");

/// The well's least width and height in control heights.
const WIDTH_IN_HEIGHTS: f32 = 10.0;
const HEIGHT_IN_HEIGHTS: f32 = 6.0;
/// The point's hit square as a share of the well's height.
const THUMB_SHARE: f32 = 0.25;
/// An arrow press moves this share of an axis's range when the picker has no step.
const STEP_SHARE: f64 = 0.01;
/// An arrow key with Shift takes this many steps.
const LARGE_STEPS: f64 = 10.0;

/// The style of a point picker.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PointPickerStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The length of the cross at the point.
    cross: f32,
}

/// The point at the start of the running drag.
#[derive(Clone, Default)]
struct DragStart([f64; 2]);

/// Paints the image under the point into the plot rect through the content painter: `PointPicker::backdrop`.
pub struct Backdrop<'a>(Box<dyn FnOnce(&ContentPainterCx, Rect) + 'a>);

impl<'a> Backdrop<'a> {
    /// The backdrop that runs `paint`.
    pub fn new(paint: impl FnOnce(&ContentPainterCx, Rect) + 'a) -> Self {
        Backdrop(Box::new(paint))
    }
}

/// A point `[x, y]` picked on a well: x grows to the right and y downwards, as image coordinates do, in either
/// reading direction.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(content_painter)]
pub struct PointPicker<'a> {
    #[common]
    common: Common,
    value: Bind<'a, [f64; 2]>,
    x: RangeInclusive<f64>,
    y: RangeInclusive<f64>,
    /// The step the point snaps to and one arrow key moves it on each axis; the default is a hundredth of the range
    /// with no snapping.
    #[prop(quantity = "step")]
    step: Option<f64>,
    /// The width over the height of the image the well shows; the plot is the image fitted in the well.
    #[prop(quantity = "aspect ratio")]
    aspect: Option<f32>,
    /// Paints the image under the point; without one the well shows a checkerboard.
    #[prop(func = "FnOnce(&ContentPainterCx, Rect)")]
    backdrop: Option<Backdrop<'a>>,
}

impl<'a> PointPicker<'a> {
    /// A picker of `value` over `x` and `y`.
    pub fn new(value: Bind<'a, [f64; 2]>, x: RangeInclusive<f64>, y: RangeInclusive<f64>) -> Self {
        PointPicker { common: Common::new(), value, x, y, step: None, aspect: None, backdrop: None }
    }

    fn ranges(&self) -> [&RangeInclusive<f64>; 2] {
        [&self.x, &self.y]
    }

    fn middle(&self) -> [f64; 2] {
        self.ranges().map(|range| (range.start() + range.end()) / 2.0)
    }

    /// The step of one arrow press on `axis`.
    fn small(&self, axis: usize) -> f64 {
        let range = self.ranges()[axis];
        self.step.unwrap_or((range.end() - range.start()) * STEP_SHARE)
    }

    /// `value` on `axis` in its range, and on the step grid when the picker has a step.
    fn settle(&self, axis: usize, value: f64) -> f64 {
        let (min, max) = (*self.ranges()[axis].start(), *self.ranges()[axis].end());
        let value = value.clamp(min, max);
        match self.step.filter(|step| *step > 0.0) {
            Some(step) => (min + ((value - min) / step).round() * step).min(max),
            None => value,
        }
    }

    /// The image fitted in `well`, else the well.
    fn plot(&self, well: Rect) -> Rect {
        match self.aspect.filter(|aspect| aspect.is_finite() && *aspect > 0.0) {
            Some(aspect) => {
                let width = well.width().min(well.height() * aspect);
                Rect::from_center_size(well.center(), Vec2::new(width, width / aspect))
            }
            None => well,
        }
    }

    /// Where `value` stands in `plot`.
    fn position(&self, value: [f64; 2], plot: Rect) -> Pos2 {
        let at = |value: f64, range: &RangeInclusive<f64>| {
            let span = range.end() - range.start();
            if span > 0.0 { ((value - range.start()) / span).clamp(0.0, 1.0) as f32 } else { 0.0 }
        };
        pos2(plot.left() + at(value[0], &self.x) * plot.width(), plot.top() + at(value[1], &self.y) * plot.height())
    }

    /// The point `press` gives, from the point at the start of the drag.
    fn dragged(&self, drags: &[ValueDrag; 2], start: [f64; 2], press: &PressState) -> Option<[f64; 2]> {
        Some([self.settle(0, drags[0].value(start[0], press, false)?), self.settle(1, drags[1].value(start[1], press, false)?)])
    }

    fn places(&self) -> usize {
        let step = self.small(0).abs();
        (0..3).find(|places| (step * 10f64.powi(*places)).fract().abs() < 1e-9).map_or(3, |places| places as usize)
    }
}

/// The change of an arrow key on the point: one step on x for Right and Left, on y for Down and Up, ten with Shift.
/// Right grows x in either reading direction, because the picker is not mirrored.
fn key_delta(mv: NavMove, mods: MoveMods, rtl: bool) -> Option<[f64; 2]> {
    let steps = if mods.extend { LARGE_STEPS } else { 1.0 };
    let along = if rtl { -steps } else { steps };
    match mv {
        NavMove::Forward => Some([along, 0.0]),
        NavMove::Back => Some([-along, 0.0]),
        NavMove::Down => Some([0.0, steps]),
        NavMove::Up => Some([0.0, -steps]),
        _ => None,
    }
}

impl Component for PointPicker<'_> {
    const ROLE: Role = Role::SLIDER;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows).with_activate(ActivateKeys::Neither);
    type Style = PointPickerStyle;
    type Output = EditResponse<[f64; 2]>;

    fn style(cx: &StyleCx<'_>, _: PointPickerStyleKey) -> PointPickerStyle {
        let control = cx.control();
        PointPickerStyle {
            metrics: Metrics::new().min_size(Vec2::new(control.height * WIDTH_IN_HEIGHTS, control.height * HEIGHT_IN_HEIGHTS)),
            focus: crate::controls::focus_look(cx),
            cross: control.icon,
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::FOCUS
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value().copied().unwrap_or_else(|| self.middle());
        let thumb = Rect::from_center_size(self.position(value, self.plot(rect)), Vec2::splat(rect.height() * THUMB_SHARE));
        cx.part(WELL, rect).sense(Sense::CLICK | Sense::DRAG).value_part().pointer_only("the arrow keys move the point");
        cx.part(THUMB, thumb)
            .sense(Sense::DRAG)
            .priority(HitPriority::new(HitLevel::Handle, 0))
            .value_part()
            .pointer_only("the arrow keys move the point");
    }

    fn bind(&mut self) -> Option<BindMut<'_, [f64; 2]>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let plot = self.plot(cx.rect);
        let drags = [
            ValueDrag::absolute(plot.left()..plot.right(), self.x.clone()),
            ValueDrag::absolute(plot.top()..plot.bottom(), self.y.clone()).axis(Axis::Y),
        ];
        for key in [THUMB, WELL] {
            let press = cx.part(key).press;
            let start = cx.memory::<DragStart>().0;
            match press.phase {
                PressPhase::DragStart { .. } => {
                    let from = cx.value().copied().unwrap_or_else(|| self.middle());
                    cx.memory::<DragStart>().0 = from;
                    cx.begin();
                }
                PressPhase::Drag { .. } => {
                    if let Some(next) = self.dragged(&drags, start, &press) {
                        cx.propose(next);
                    }
                }
                PressPhase::Release { click, .. } => {
                    let running = cx.gesture().is_some();
                    if let Some(next) = self.dragged(&drags, start, &press) {
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
        let (running, rtl) = (cx.gesture().is_some(), cx.flow.is_rtl());
        let mut now = cx.value().copied().unwrap_or_else(|| self.middle());
        while let Some((mv, mods)) = cx.intents().take_move() {
            if running {
                continue;
            }
            if let Some(by) = key_delta(mv, mods, rtl) {
                let next = [self.settle(0, now[0] + by[0] * self.small(0)), self.settle(1, now[1] + by[1] * self.small(1))];
                cx.set(next);
                now = next;
            }
        }
        if let Some(next) = cx.intents().take_set_value().filter(|next| next.is_finite()) {
            cx.set([self.settle(0, next), now[1]]);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let numeric = match cx.value() {
            Some(value) => {
                let number = |axis: usize| cx.format().number(value[axis], self.places());
                let mut numeric = Numeric::new(value[0], self.x.clone());
                numeric.text = Some(format!("{}, {}", number(0), number(1)));
                numeric
            }
            None => Numeric::empty(self.x.clone()),
        };
        access.numeric(numeric.step(self.small(0)));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let painter = ui.content_painter(cx);
        let inside = painter.plot_frame(cx.rect);
        let plot = self.plot(cx.rect);
        match self.backdrop.take() {
            Some(Backdrop(paint)) => paint(&painter, plot),
            None => painter.checker(plot.intersect(inside)),
        }
        if let Some(value) = cx.value() {
            let (center, arm) = (self.position(*value, plot), cx.style.cross / 2.0);
            for (from, to) in [(Vec2::new(arm, 0.0), Vec2::new(-arm, 0.0)), (Vec2::new(0.0, arm), Vec2::new(0.0, -arm))] {
                painter.outline(&PaintShape::Path(vec![center + from, center + to]));
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<[f64; 2]> {
        cx.edit_response()
    }
}
