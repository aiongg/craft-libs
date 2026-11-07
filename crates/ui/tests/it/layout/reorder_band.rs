//! A reorderable section's header: Alt+Up and Alt+Down move it instead of closing it.

use crate::contract::kit::PlainStyle;
use craft_behavior::{Intent, Sense};
use craft_ui::buttons::Button;
use craft_ui::contract::{
    BodyCx, Collapse, Common, Component, Container, ContainerProps, Cx, DragBand, Focus as _, HeaderRegion, Props as _, Role, Show as _,
    ShowContainer as _,
};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::style::StyleCx;
use craft_ui::testing::focus::key_release;
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 90.0) };

#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Section<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    moved: &'a Cell<Option<bool>>,
    title: &'a Cell<egui::Id>,
}

impl Component for Section<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SectionStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.intents().take(Intent::move_item(true)) {
            self.moved.set(Some(true));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Section<'_> {
    fn header(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        self.title.set(Button::new("Title").id_salt("title").show(ui).id);
    }
}

#[trial]
fn alt_down_in_a_reorderable_sections_header_moves_it_and_does_not_close_it() {
    let mut frames = Frames::new();
    let moved = Cell::new(None);
    let (title, body) = (Cell::new(egui::Id::NULL), Cell::new(egui::Id::NULL));
    let mut show = |ui: &mut Ui| {
        let props = ContainerProps::new().header(HeaderRegion::new().band(DragBand::Reorder)).collapse(Collapse::new().animated(false));
        Section { common: Common::new(), props, moved: &moved, title: &title }.id_salt("section").show_at(ui, AREA, |ui, _| {
            body.set(Button::new("Item").id_salt("item").show(ui).id);
        });
    };
    frames.idle(3, &mut show);
    request(&frames.ctx, FocusTarget::Id(title.get()));
    frames.frame(Vec::new(), &mut show);
    let alt = Modifiers::ALT;
    frames.frame(vec![key_press(Key::ArrowDown, alt)], &mut show);
    frames.frame(vec![key_release(Key::ArrowDown, alt)], &mut show);
    assert_eq!(moved.get(), Some(true), "Alt+Down arrives as a move toward the end");
    assert_ne!(body.get(), egui::Id::NULL, "the section stays open: its body still runs");
}
