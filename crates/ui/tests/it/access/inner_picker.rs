//! A component that shows a `PointPicker` as its own control has one focus stop and one node: the owner's, which
//! carries the owner's name and the picker's value, and the arrow keys move the point.

use crate::contract::kit::PlainStyle;
use craft_behavior::{Mixed, Sense};
use craft_ui::contract::{Bind, Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Show as _, WordSetCx as _};
use craft_ui::style::StyleCx;
use craft_ui::testing::access::{NodeKey, focused_key, name};
use craft_ui::testing::{Fixture, trial};
use craft_ui::value_drag::PointPicker;
use egui::{Key, Modifiers, Response, Vec2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const NAME: &str = "Saturation and brightness";

/// A field that draws its control with a point picker.
#[derive(craft_ui::contract::Component)]
struct Field<'a> {
    #[common]
    common: Common,
    point: &'a RefCell<[Mixed<f64>; 2]>,
}

impl Component for Field<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FieldStyleKey) -> PlainStyle {
        PlainStyle::min(vec2(120.0, 120.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        vec2(120.0, 120.0)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let mut point = self.point.borrow_mut();
        ui.inner("picker", cx.rect, PointPicker::new(Bind::bound(&mut *point), 0.0..=100.0, 0.0..=100.0).label(cx.label()));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Rig {
    fixture: Fixture,
    point: Rc<RefCell<[Mixed<f64>; 2]>>,
    id: Rc<Cell<egui::Id>>,
}

fn rig() -> Rig {
    let point = Rc::new(RefCell::new([Mixed::Same(50.0), Mixed::Same(50.0)]));
    let id = Rc::new(Cell::new(egui::Id::NULL));
    let (bound, seen) = (point.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        seen.set(Field { common: Common::new(), point: &bound }.label(NAME).show(ui).id);
    });
    fixture.settle();
    Rig { fixture, point, id }
}

#[trial]
fn a_picker_inside_a_component_is_the_components_one_node() {
    let rig = rig();
    let tree = rig.fixture.tree();
    let owner = NodeKey::of(rig.id.get());
    let named: Vec<NodeKey> = tree.keys().into_iter().filter(|key| name(tree, *key) == Some(NAME) && !tree.is_hidden(*key)).collect();
    assert_eq!(named, vec![owner], "one visible node carries the owner's name");
    assert!(tree.value_of(owner).is_some_and(|text| text.contains("50")), "it carries the picker's value: {:?}", tree.value_of(owner));
    let groups = tree.with_role(Role::GROUP).into_iter().filter(|key| !tree.is_hidden(*key)).count();
    assert_eq!(groups, 1, "the picker has no node of its own");
}

#[trial]
fn tab_focuses_the_owners_node_and_the_arrows_move_the_point() {
    let mut rig = rig();
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.idle(2);
    assert_eq!(focused_key(rig.fixture.tree()), NodeKey::of(rig.id.get()), "focus lands on the owner's node");
    rig.fixture.key(Key::ArrowRight, Modifiers::NONE);
    rig.fixture.idle(2);
    let [x, y] = *rig.point.borrow();
    assert!(x.same().is_some_and(|x| *x > 50.0), "Right raises the across component: {x:?}");
    assert_eq!(y, Mixed::Same(50.0), "and leaves the down one");
}
