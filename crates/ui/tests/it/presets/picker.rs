//! `PresetPicker`: a pick in the popover writes its key, and closing the popover without a pick keeps the value.

use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::patterns::presets::{PresetPicker, PresetRow, PresetState};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const AMBER: u32 = 1;
const CORAL: u32 = 2;

fn rows() -> Vec<PresetRow<u32>> {
    vec![PresetRow::item(AMBER, "Amber", 0), PresetRow::item(CORAL, "Coral", 0)]
}

fn picker() -> (Fixture, Rc<Cell<Option<u32>>>) {
    let value = Rc::new(Cell::new(Some(AMBER)));
    let shown = value.clone();
    let state = Rc::new(RefCell::new(PresetState::<u32>::new()));
    let mut fixture = Fixture::new(move |ui| {
        let mut held = shown.get();
        let rows = rows();
        let mut view = state.borrow_mut();
        let _ = PresetPicker::new("Swatch", Bind::bound(&mut held), &rows, Bind::bound(&mut *view)).show(ui);
        shown.set(held);
    });
    fixture.settle();
    (fixture, value)
}

fn open(fixture: &mut Fixture) {
    let field = fixture.by_role(Role::COMBO_BOX).first().copied().expect("the picker's field");
    fixture.click(&field);
    fixture.idle(3);
}

/// A click on a preset in the popover writes its key.
#[trial]
fn a_click_on_a_preset_picks_it() {
    let (mut fixture, value) = picker();
    open(&mut fixture);
    let coral = fixture.by_label("Coral").expect("the preset's tile");
    fixture.click(&coral);
    fixture.idle(3);
    assert_eq!(value.get(), Some(CORAL));
}

/// Escape closes the popover and the value stays what it was.
#[trial]
fn escape_closes_the_popover_without_a_pick() {
    let (mut fixture, value) = picker();
    open(&mut fixture);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(value.get(), Some(AMBER));
}
