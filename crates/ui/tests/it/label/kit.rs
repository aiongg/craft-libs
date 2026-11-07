//! Components for the label-step trials: a scrubbing value control, a field container and a leaf that labels the
//! control it paints.

use craft_behavior::{GesturePhase, Metrics, PressPhase, Sense, ValueDrag};
use craft_ui::contract::{
    Accessible as _, Bind, BindMut, Common, Component, Container, ContainerProps, Cx, Edit as _, EditResponse, FieldLabel, MeasureCx, PaintUi,
    Props as _, Remember as _, Role, SideLabel,
};
use craft_ui::primitives::TextRun;
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Id, Rect, Response, Vec2};
use std::cell::RefCell;

/// A style with no padding.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PlainStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a dial saw of its label scope and itself.
#[non_exhaustive]
pub struct Seen {
    pub id: Option<Id>,
    pub rect: Rect,
    pub label: Option<FieldLabel>,
}

impl Default for Seen {
    fn default() -> Self {
        Seen { id: None, rect: Rect::NOTHING, label: None }
    }
}

pub const SIZE: Vec2 = Vec2::new(100.0, 20.0);
/// The value change per point of label drag.
const RATE: f64 = 1.0;
/// The value a reset restores.
pub const RESET: f64 = 7.0;
const RANGE: std::ops::RangeInclusive<f64> = 0.0..=1000.0;

#[derive(Clone, Default)]
struct Start(f64);

/// A number control that scrubs by its label and paints its value as text, so it has a baseline.
#[derive(Component)]
pub struct Dial<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
    seen: &'a RefCell<Seen>,
}

impl<'a> Dial<'a> {
    pub fn new(value: &'a mut f64, seen: &'a RefCell<Seen>) -> Self {
        Dial { common: Common::new(), value: Bind::bound(value).reset(RESET), seen }
    }
}

impl Component for Dial<'_> {
    const ROLE: Role = Role::SLIDER;
    type Style = PlainStyle;
    type Output = EditResponse<f64>;

    fn style(_: &StyleCx<'_>, _: DialStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        SIZE
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn bind(&mut self) -> Option<BindMut<'_, f64>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let label = cx.field_label();
        {
            let mut seen = self.seen.borrow_mut();
            seen.rect = cx.rect;
            seen.id = Some(cx.id);
            seen.label = label.clone();
        }
        let Some(label) = label else { return };
        let rtl = cx.env.direction().is_rtl();
        let drag = ValueDrag::relative(RATE, RANGE);
        match label.press.phase {
            PressPhase::DragStart { .. } => {
                cx.memory::<Start>().0 = cx.value().copied().unwrap_or(0.0);
                cx.begin();
            }
            PressPhase::Drag { .. } => {
                let start = cx.memory::<Start>().0;
                if let Some(next) = drag.value(start, &label.press, rtl) {
                    cx.propose(next);
                }
            }
            PressPhase::Release { .. } if cx.gesture().is_some_and(|phase| matches!(phase, GesturePhase::Begin | GesturePhase::Update)) => {
                cx.commit()
            }
            _ => {}
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let text = format!("{}", cx.value().copied().unwrap_or(0.0));
        ui.draw().text(cx.rect, &TextRun::new(text));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}

/// A container with a side label whose body is the caller's.
#[derive(Component)]
#[component(container)]
pub struct Field {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Field {
    pub fn new(label: &'static str, side: SideLabel) -> Self {
        Field { common: Common::new(), props: ContainerProps::new().label(side) }.label(label)
    }
}

impl Component for Field {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FieldStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Field {}

/// A leaf that shows a labelled dial in its own rect with `PaintUi::labelled`.
#[derive(Component)]
pub struct LabelledDial<'a> {
    #[common]
    common: Common,
    value: &'a mut f64,
    label: &'static str,
    side: SideLabel,
    seen: &'a RefCell<Seen>,
}

impl<'a> LabelledDial<'a> {
    pub fn new(label: &'static str, side: SideLabel, value: &'a mut f64, seen: &'a RefCell<Seen>) -> Self {
        LabelledDial { common: Common::new(), value, label, side, seen }
    }
}

impl Component for LabelledDial<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: LabelledDialStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(300.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.labelled("dial", cx.rect, self.label.into(), self.side, Dial::new(&mut *self.value, self.seen));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
