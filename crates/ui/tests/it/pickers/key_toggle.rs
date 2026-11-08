//! `toggle_on_key`: a custom face opens its popup on Space and closes it on Escape, as the built-in faces do.

use crate::overlay::kit::{Leaf, PlainStyle, Seen};
use craft_behavior::Sense;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, Overlay, OverlayUi, Overlays as _, Props as _, Role, Show as _, Trigger};
use craft_ui::pickers::toggle_on_key;
use craft_ui::style::StyleCx;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Response, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const FACE: Vec2 = Vec2::new(80.0, 20.0);
const CONTENT: Vec2 = Vec2::new(80.0, 30.0);

/// A face with a dropdown popup that toggles by key through `toggle_on_key`.
#[derive(Component)]
#[component(overlay)]
struct Face<'a> {
    #[common]
    common: Common,
    shown: &'a Cell<bool>,
}

impl Component for Face<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FaceStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        FACE
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("popup");
        popup.trigger(Trigger::Manual, cx);
        toggle_on_key(cx, popup);
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("popup");
        let content = RefCell::new(Seen::default());
        let shown = self.shown;
        ui.show(popup, Overlay::dropdown(), |ui, _| {
            shown.set(true);
            Leaf::new(&content, CONTENT).id_salt("content").show(ui);
        });
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_custom_face_opens_on_space_and_closes_on_escape() {
    let shown = Rc::new(Cell::new(false));
    let seen = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        Face { common: Common::new(), shown: &seen }.show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    shown.set(false);
    fixture.key(Key::Space, Modifiers::NONE);
    fixture.idle(4);
    assert!(shown.get(), "Space opens the popup");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(10);
    shown.set(false);
    fixture.idle(2);
    assert!(!shown.get(), "Escape closes the popup");
}
