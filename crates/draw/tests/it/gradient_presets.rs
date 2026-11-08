//! `GradientPalette` and `GradientMapEditor`, through the public API.

use std::cell::RefCell;
use std::rc::Rc;

use craft_draw::ui::gradient::{ColorStop, GradientGroup, GradientItem, GradientMap, GradientMapEditor, GradientPalette, GradientRef, GradientStops};
use craft_theme::Srgba;
use craft_ui::contract::Bind;
use craft_ui::patterns::presets::PresetState;
use craft_ui::testing::{Fixture, trial};

fn pair(a: f32, b: f32) -> GradientStops {
    let color = |v: f32| Srgba { r: v, g: v, b: v, a: 0.5 };
    GradientStops::new(vec![ColorStop::new(0.0, color(a)), ColorStop::new(1.0, color(b))])
}

fn groups() -> Vec<GradientGroup> {
    vec![GradientGroup::new("Basics", vec![GradientItem::new("Dark", pair(0.0, 0.5)), GradientItem::new("Light", pair(0.5, 1.0))])]
}

#[trial]
fn a_click_is_one_pick_of_the_preset_clicked_also_when_it_is_selected() {
    let picks: Rc<RefCell<Vec<GradientRef>>> = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&picks);
    let groups = groups();
    let mut state = PresetState::new();
    let mut fixture = Fixture::new(move |ui| {
        let out = GradientPalette::new("Gradients", &groups, Bind::bound(&mut state)).selected(GradientRef::Item { group: 0, item: 1 }).show(ui);
        seen.borrow_mut().extend(out.picked);
    });
    fixture.settle();
    let light = fixture.by_label("Light").expect("the palette shows the preset");
    fixture.click(&light);
    assert_eq!(*picks.borrow(), vec![GradientRef::Item { group: 0, item: 1 }]);
}

#[trial]
fn the_map_editor_shows_its_options_and_reverse_is_one_edit() {
    let map = Rc::new(RefCell::new(GradientMap { stops: pair(0.0, 1.0), reverse: false, dither: false }));
    let held = Rc::clone(&map);
    let mut fixture = Fixture::new(move |ui| {
        let out = GradientMapEditor::new("Gradient Map", &held.borrow().clone()).show(ui);
        if let Some(after) = out.event.as_ref().and_then(craft_behavior::EditEvent::value) {
            *held.borrow_mut() = after.clone();
        }
    });
    fixture.settle();
    let reverse = fixture.by_label("Reverse").expect("the editor shows Reverse");
    fixture.click(&reverse);
    assert!(map.borrow().reverse && !map.borrow().dither);
}
