//! The divider a component can use without a container: a bar of its own that takes the drag and the keys, and the
//! name a split area gives its divider.

use crate::contract::kit::PlainStyle;
use craft_behavior::{Axis, PartKey};
use craft_ui::contract::{Access, Common, Component, Cx, MeasureCx, PartsCx, Props as _, Role, Show as _};
use craft_ui::docking::divider::{self, Divider, Drag};
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::StyleCx;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Rect, Response, Vec2, vec2};
use std::cell::Cell;
use std::rc::Rc;

const BAR: PartKey = PartKey::new("bar");
const LEAST: f32 = 20.0;
const MOST: f32 = 180.0;
const REACH: f32 = 8.0;
const STEP: f32 = 4.0;
const JUMP: f32 = 16.0;

/// A component with one vertical divider whose coordinate the host keeps, built from the public divider functions.
#[derive(Component)]
struct Bar {
    #[common]
    common: Common,
    at: Rc<Cell<f32>>,
}

impl Bar {
    fn divider(&self, rect: Rect) -> Divider {
        Divider::new(Axis::X, self.at.get(), rect, (LEAST, MOST), REACH)
    }
}

impl Component for Bar {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::SPLITTER;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BarStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        vec2(200.0, 60.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        divider::declare(cx, rect, BAR, 0, &self.divider(rect));
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let divider = self.divider(cx.rect);
        if let Some(Drag::To(coord)) = divider::pointer(cx, BAR, 0, &divider) {
            self.at.set(coord.clamp(divider.lo, divider.hi));
        }
        if let Some((_, coord)) = divider::keys(cx, BAR, &[(0, divider)], STEP, JUMP) {
            self.at.set(coord);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        divider::describe(access, BAR, 0, &self.divider(cx.rect), "Resize bar", STEP);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_component_drives_its_own_divider_with_the_public_functions() {
    let at = Rc::new(Cell::new(100.0));
    let kept = Rc::clone(&at);
    let mut fixture = Fixture::new(move |ui| {
        Bar { common: Common::new(), at: Rc::clone(&kept) }.label("Bar").show(ui);
    });
    fixture.settle();
    let divider = fixture.by_label("Resize bar").expect("the divider is named by the component");
    let start = at.get();
    fixture.drag(&divider, vec2(30.0, 0.0));
    fixture.settle();
    assert!(at.get() > start, "dragging right moved the divider: {}", at.get());

    let before = at.get();
    let divider = fixture.by_label("Resize bar").expect("the divider stays");
    fixture.click(&divider);
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    fixture.settle();
    assert!(at.get() < before, "Left on the focused divider moves it back: {before} -> {}", at.get());
}
