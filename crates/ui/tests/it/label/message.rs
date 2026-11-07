//! The tone of a validity message at the label scope: a warning reads as a warning, a refusal as bad, and a tone the
//! message names wins over both.

use craft_behavior::Sense;
use craft_theme::variant::Tone;
use craft_ui::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, Edit as _, EditResponse, HeaderRegion, Props as _, Role,
    Show as _, ShowContainer as _, SideLabel, Validity,
};
use craft_ui::runtime::env::Message;
use craft_ui::style::StyleCx;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Response, Ui, Vec2};
use std::cell::Cell;

use super::kit::PlainStyle;

const AT: Rect = Rect { min: egui::pos2(0.0, 0.0), max: egui::pos2(300.0, 60.0) };

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

/// The tone the container's body reads for a control that reports `validity`.
fn tone_of(validity: fn() -> Validity<f64>) -> Option<Tone> {
    let mut frames = Frames::new();
    let seen = Cell::new(None);
    let mut value = 1.0;
    frames.idle(3, |ui| {
        let props = ContainerProps::new().label(SideLabel::new()).header(HeaderRegion::new());
        let reader = Reader { common: Common::new(), props, seen: &seen }.label("Amount");
        reader.id_salt("reader").show_at(ui, AT, |ui, _| {
            Checked { common: Common::new(), value: Bind::bound(&mut value).check(|_| validity()) }.show(ui);
        });
    });
    seen.get()
}

#[trial]
fn a_warning_reads_as_warn_a_refusal_as_bad_and_a_named_tone_wins() {
    assert_eq!(tone_of(|| Validity::Warn(Message::new("x"))), Some(Tone::Warn));
    assert_eq!(tone_of(|| Validity::Refuse(Message::new("x"))), Some(Tone::Bad));
    assert_eq!(tone_of(|| Validity::Warn(Message::new("x").tone(Tone::Accent))), Some(Tone::Accent));
    assert_eq!(tone_of(|| Validity::Refuse(Message::new("x").tone(Tone::Accent))), Some(Tone::Accent));
}
