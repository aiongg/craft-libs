//! `SliderField`: a slider and a number field over one value.

#[cfg(feature = "testing")]
mod stories;

use crate::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Props as _, Role, WordSetCx as _};
use crate::runtime::env::{Label, UnitKind};
use crate::style::{ComponentStyle, StyleCx};
use crate::text::NumberField;
use crate::value_drag::{Slider, SliderScale};
use craft_behavior::{Flow, Metrics, Mixed, Proposal, Sense, decimals_for_step};
use egui::{Rect, Vec2, pos2};
use std::ops::RangeInclusive;

/// The step of the number field when the field sets none: hundredths on a short range, whole numbers on a long one.
const SHORT_RANGE: f64 = 10.0;
const SHORT_STEP: f64 = 0.01;
const LONG_STEP: f64 = 1.0;

/// The style of a slider field: the gap between slider and number field, and the number field's width.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SliderFieldStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The width of the number field, the same for every row so a column of fields lines up.
    field: f32,
}

/// A slider with a number field after it over one value. A drag on the slider, a step key, typing in the field or a
/// scrub by the enclosing `Field`'s label is one edit stream (begin, updates, commit) of the one binding.
///
/// The step moves the slider's keys and wheel and the field's keys, wheel and scrub; typed values snap to it. Without
/// a step the field steps by a hundredth on a range 10 wide or less and by 1 on a longer one, and rounds typed values
/// to the places that step needs. The field and then the slider are Tab stops. A log or power scale curves the
/// slider's track only: the field, typing and steps stay linear.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct SliderField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
    range: RangeInclusive<f64>,
    /// How positions along the slider's track map to values.
    #[prop]
    scale: SliderScale,
    /// The change of one arrow press, wheel notch or label scrub point, and the grid typed values snap to.
    #[prop(quantity = "step")]
    step: Option<f64>,
    /// The change of one wheel notch over the slider where it is coarser than the step.
    #[prop(quantity = "step")]
    wheel_step: Option<f64>,
    /// What the number counts: the field shows its unit and reads its spellings.
    #[prop]
    unit_kind: Option<UnitKind>,
}

impl<'a> SliderField<'a> {
    /// A slider and number field named `label` over `value` in `range`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, f64>, range: RangeInclusive<f64>) -> Self {
        SliderField { common: Common::new(), value, range, scale: SliderScale::Linear, step: None, wheel_step: None, unit_kind: None }.label(label)
    }

    /// The step the host set, when it is a usable one.
    fn set_step(&self) -> Option<f64> {
        self.step.filter(|step| step.is_finite() && *step > 0.0)
    }

    fn field_step(&self) -> f64 {
        self.set_step().unwrap_or(if (self.range.end() - self.range.start()).abs() <= SHORT_RANGE { SHORT_STEP } else { LONG_STEP })
    }

    fn slider(&self, name: &Label, shown: Mixed<f64>) -> Slider<'static> {
        let slider = Slider::new(self.given(shown), self.range.clone()).scale(self.scale).label(name.clone());
        let slider = match self.set_step() {
            Some(step) => slider.step(step),
            None => slider,
        };
        match self.wheel_step {
            Some(step) => slider.wheel_step(step),
            None => slider,
        }
    }

    /// The binding a child shows `shown` through, with the reset value of this component's own.
    fn given(&self, shown: Mixed<f64>) -> Bind<'static, f64> {
        let bind = Bind::given(shown);
        match self.value.reset_value() {
            Some(default) => bind.reset(*default),
            None => bind,
        }
    }

    fn field(&self, name: &Label, shown: Mixed<f64>) -> NumberField<'static> {
        let number = match shown {
            Mixed::Same(value) => Mixed::Same(Some(value)),
            Mixed::Mixed => Mixed::Mixed,
        };
        let bind = Bind::given(number);
        let bind = match self.value.reset_value() {
            Some(default) => bind.reset(Some(*default)),
            None => bind,
        };
        let step = self.field_step();
        let field = NumberField::new(name.clone(), bind)
            .range(self.range.clone())
            .step(step)
            .snap_typed(self.set_step().is_some())
            .commit_decimals(decimals_for_step(step))
            .scrub_by_label();
        match self.unit_kind {
            Some(kind) => field.unit_kind(kind),
            None => field,
        }
    }
}

impl Component for SliderField<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = SliderFieldStyle;
    type Output = EditResponse<f64>;

    fn style(cx: &StyleCx<'_>, _: SliderFieldStyleKey) -> SliderFieldStyle {
        SliderFieldStyle { metrics: Metrics::new().gap(cx.control().gap), field: cx.tokens.size.length.short }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let shown = shown_of(cx.value());
        let name = cx.label();
        let slider = cx.child("slider", &mut self.slider(&name, shown), Proposal::Ideal);
        let field = cx.child("field", &mut self.field(&name, shown), Proposal::Ideal);
        Vec2::new(slider.x + cx.gap() + cx.style.field, slider.y.max(field.y))
    }

    fn bind(&mut self) -> Option<BindMut<'_, f64>> {
        Some(self.value.reborrow())
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let shown = shown_of(cx.value());
        let name = cx.label();
        let (track, field) = arrange(cx.rect.size(), cx.style.metrics.gap, cx.style.field);
        let flow = Flow::new(cx.rect, cx.env.direction().is_rtl());
        let place = |logical: Rect| flow.to_physical(logical.translate(cx.rect.min.to_vec2()));
        // The field is shown first, so it is the first Tab stop.
        // The group has no stop, so its tooltip, and with it the reason it is disabled, is carried by the first stop.
        let field_view = self.field(&name, shown);
        let field_view = match cx.resolved.tooltip {
            Some(tooltip) => field_view.tooltip(tooltip.clone()),
            None => field_view,
        };
        let typed = ui.child("field", place(field), field_view);
        let dragged = ui.child("slider", place(track), self.slider(&name, shown));
        let start = *self.range.start();
        cx.adopt(&typed, &start, |value, mine| {
            if let Some(value) = value {
                *mine = *value;
            }
        });
        cx.adopt(&dragged, &start, |value, mine| *mine = *value);
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}

/// The value the children show: none while the binding is mixed.
fn shown_of(shown: Option<&f64>) -> Mixed<f64> {
    shown.map_or(Mixed::Mixed, |value| Mixed::Same(*value))
}

/// The slider's and the number field's rects in a room of `size`, in logical coordinates from the start edge: the
/// field at the end, the slider in the room before it.
fn arrange(size: Vec2, gap: f32, field_width: f32) -> (Rect, Rect) {
    let field = field_width.min(size.x);
    let track = (size.x - field - gap).max(0.0);
    let rect = |x: f32, width: f32| Rect::from_min_size(pos2(x, 0.0), Vec2::new(width, size.y));
    (rect(0.0, track), rect(size.x - field, field))
}
