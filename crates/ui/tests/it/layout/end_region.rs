//! The end region of a container: at the line's end edge, in the body's roving group.

use crate::contract::kit::PlainStyle;
use craft_behavior::{NavShape, Roving, Sense};
use craft_ui::buttons::Button;
use craft_ui::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, EndRegion, Props as _, Role, Show as _, ShowContainer as _, Sizing,
};
use craft_ui::style::StyleCx;
use craft_ui::testing::focus::{egui_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Response, Ui};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(410.0, 50.0) };

#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Ended<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    end_at: &'a Cell<Rect>,
}

impl Component for Ended<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: EndedStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Ended<'_> {
    fn end(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        Button::new("Commit").id_salt("commit").show(ui);
        let last = Button::new("Cancel").id_salt("cancel").show(ui);
        self.end_at.set(last.rect);
    }
}

fn tab(frames: &mut Frames, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut *show);
    frames.frame(vec![key_release(Key::Tab, Modifiers::NONE)], show);
}

#[trial]
fn an_end_regions_buttons_sit_at_the_end_edge_and_join_the_roving_group() {
    let mut frames = Frames::new();
    let (last, before, after) = (Cell::new(Rect::NOTHING), Cell::new(Id::NULL), Cell::new(Id::NULL));
    let mut show = |ui: &mut Ui| {
        before.set(Button::new("Before").id_salt("before").show_at(ui, Rect::from_min_size(Pos2::new(10.0, 60.0), egui::vec2(60.0, 20.0))).id);
        let props = ContainerProps::new().sizing(Sizing::Fill).end(EndRegion::new()).roving(Roving::new(NavShape::Flat));
        Ended { common: Common::new(), props, end_at: &last }.id_salt("ended").show_at(ui, AREA, |ui, _| {
            Button::new("Body").id_salt("body").show(ui);
        });
        after.set(Button::new("After").id_salt("after").show_at(ui, Rect::from_min_size(Pos2::new(10.0, 90.0), egui::vec2(60.0, 20.0))).id);
    };
    frames.idle(4, &mut show);
    assert!(AREA.max.x - last.get().max.x < 40.0, "the last end button sits against the line's end edge: {:?}", last.get());
    craft_ui::runtime::focus::request(&frames.ctx, craft_ui::runtime::focus::FocusTarget::Id(before.get()));
    frames.frame(Vec::new(), &mut show);
    tab(&mut frames, &mut show);
    let group = egui_focused(&frames.ctx);
    assert!(group.is_some() && group != Some(after.get()), "Tab enters the row's roving group");
    tab(&mut frames, &mut show);
    assert_eq!(egui_focused(&frames.ctx), Some(after.get()), "the body and end region buttons are one Tab stop");
}
