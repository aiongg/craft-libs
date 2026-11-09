//! `toggle_on_key`: a custom face opens its popup on Space and closes it on Escape, as the built-in faces do.

use crate::overlay::kit::{Leaf, PlainStyle, Seen};
use craft_behavior::Sense;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, Overlay, OverlayUi, Overlays as _, Props as _, Role, Show as _, Trigger};
use craft_ui::pickers::toggle_on_key;
use craft_ui::style::StyleCx;
use egui::{Response, Vec2};
use std::cell::{Cell, RefCell};

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
