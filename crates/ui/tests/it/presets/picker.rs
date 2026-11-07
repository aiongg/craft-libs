//! `PresetPicker`: a pick in the popover writes its key, and closing the popover without a pick keeps the value.

use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::patterns::presets::{PresetEvent, PresetPicker, PresetRow, PresetState};
use craft_ui::testing::{Fixture, pointer, trial};
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

/// A picker whose row menu may delete only Amber, with the events it reported.
fn deleting_picker() -> (Fixture, Rc<RefCell<Vec<PresetEvent<u32>>>>) {
    let value = Rc::new(Cell::new(Some(AMBER)));
    let state = Rc::new(RefCell::new(PresetState::<u32>::new()));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let reported = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut held = value.get();
        let rows = rows();
        let mut view = state.borrow_mut();
        let out = PresetPicker::new("Swatch", Bind::bound(&mut held), &rows, Bind::bound(&mut *view)).can_delete(|key| *key == AMBER).show(ui);
        reported.borrow_mut().extend(out.events);
        value.set(held);
    });
    fixture.settle();
    (fixture, seen)
}

/// Opens the popover, opens the row menu of the preset named `label` and runs its Delete row; what the picker
/// reported.
fn events_after_menu_delete(label: &str) -> Vec<PresetEvent<u32>> {
    let (mut fixture, seen) = deleting_picker();
    open(&mut fixture);
    let at = fixture.by_label(label).expect("the preset's tile").rect.center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
    let names: Vec<String> = fixture.tree().keys().into_iter().map(|k| format!("{:?}", fixture.tree().name_of(k))).collect();
    let delete = fixture.by_label("Delete");
    if label == "Coral" {
        panic!("DEBUGC {names:?} {:?} {}", seen.borrow(), delete.is_some());
    }
    let delete = delete.unwrap_or_else(|| panic!("DEBUG {names:?} {:?}", seen.borrow()));
    fixture.click(&delete);
    fixture.idle(3);
    let reported = seen.borrow().clone();
    reported
}

/// The row menu deletes a preset that `can_delete` passes.
#[trial]
fn the_row_menu_deletes_a_preset_can_delete_passes() {
    assert!(events_after_menu_delete("Amber").contains(&PresetEvent::Delete(AMBER)));
}

/// The row menu of a preset that `can_delete` refuses opens, and its Delete row reports no delete, neither for that
/// preset nor for the selected one.
#[trial]
fn the_row_menu_cannot_delete_a_preset_can_delete_refuses() {
    let events = events_after_menu_delete("Coral");
    assert!(!events.iter().any(|event| matches!(event, PresetEvent::Delete(_))), "no delete was reported: {events:?}");
}

#[trial]
fn debug_amber_frames() {
    let (mut fixture, _seen) = deleting_picker();
    open(&mut fixture);
    let at = fixture.by_label("Amber").expect("tile").rect.center();
    let mut log = Vec::new();
    let names = |f: &Fixture| -> Vec<String> { f.tree().keys().into_iter().filter_map(|k| f.tree().name_of(k).map(str::to_owned)).collect() };
    fixture.frame(vec![pointer::moved(at)]);
    log.push(names(&fixture));
    fixture.frame(vec![pointer::secondary(at, true)]);
    log.push(names(&fixture));
    fixture.frame(vec![pointer::secondary(at, false)]);
    log.push(names(&fixture));
    fixture.idle(2);
    log.push(names(&fixture));
    panic!("DEBUGF {log:?}");
}
