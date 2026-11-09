//! The tone of a validity message at the label scope: a warning reads as a warning, a refusal as bad, and a tone the
//! message names wins over both.

use craft_behavior::Sense;
use craft_theme::variant::Tone;
use craft_ui::contract::MeasureCx;
use craft_ui::contract::{Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, Edit as _, EditResponse, Role};
use craft_ui::style::StyleCx;
use egui::{Response, Ui, Vec2};
use std::cell::Cell;

use super::kit::{PlainStyle, SIZE};

/// A control whose value is always checked with `check`.
#[derive(craft_ui::contract::Component)]
struct Checked<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
}

impl Component for Checked<'_> {
    const ROLE: Role = Role::SLIDER;
    type Style = PlainStyle;
    type Output = EditResponse<f64>;

    fn style(_: &StyleCx<'_>, _: CheckedStyleKey) -> PlainStyle {
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

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}

/// A labelled container whose header records the tone of the field message it is given.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Reader<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    seen: &'a Cell<Option<Tone>>,
}

impl Component for Reader<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ReaderStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Reader<'_> {
    fn header(&mut self, _: &mut Ui, cx: &mut BodyCx<'_>) {
        self.seen.set(cx.field_message().map(|(_, tone)| tone));
    }
}
