//! The browser's reorder rules: where a dragged preset or group may land.

use craft_ui::contract::{Bind, ListDrag};
use craft_ui::patterns::presets::{PresetBrowser, PresetEvent, PresetReorder, PresetRow, PresetState};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Id, Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const A: u32 = 1;
const A_ONE: u32 = 2;
const INNER: u32 = 3;
const INNER_ONE: u32 = 4;
const B: u32 = 5;
const B_ONE: u32 = 6;

/// Group A holding a preset and the folder Inner; group B holding a preset.
fn rows() -> Vec<PresetRow<u32>> {
    vec![
        PresetRow::group(A, "A", 0),
        PresetRow::item(A_ONE, "One", 1),
        PresetRow::group(INNER, "Inner", 1),
        PresetRow::item(INNER_ONE, "Two", 2),
        PresetRow::group(B, "B", 0),
        PresetRow::item(B_ONE, "Three", 1),
    ]
}

fn drag(key: u32) -> ListDrag<u32> {
    ListDrag::new(Id::NULL, vec![key], key)
}

/// A preset lands next to another preset or on a group header, which it goes into; groups stay put unless asked to
/// move.
#[trial]
fn presets_land_next_to_presets() {
    let rows = rows();
    let rules = PresetReorder::<u32>::items();
    assert!(rules.allows(&rows, &drag(A_ONE), Some(&B_ONE), 5));
    assert!(rules.allows(&rows, &drag(A_ONE), Some(&INNER_ONE), 3));
    assert!(rules.allows(&rows, &drag(A_ONE), Some(&B), 4));
    assert!(!rules.allows(&rows, &drag(B), Some(&A), 0));
}

/// A group lands next to a group at a depth `nest` allows, and never inside its own subtree.
#[trial]
fn groups_land_next_to_groups_within_the_nesting_limit() {
    let rows = rows();
    let flat = PresetReorder::<u32>::items_and_groups();
    assert!(flat.allows(&rows, &drag(B), Some(&A), 0));
    assert!(!flat.allows(&rows, &drag(B), Some(&A_ONE), 1));
    assert!(!flat.allows(&rows, &drag(B), Some(&INNER), 2));
    assert!(!flat.allows(&rows, &drag(A), Some(&INNER), 2));

    let nested = PresetReorder::<u32>::items_and_groups().nest(1);
    assert!(nested.allows(&rows, &drag(B), Some(&INNER), 2));
    assert!(nested.allows(&rows, &drag(A), Some(&B), 4));
}

/// The host's test refuses a landing the rules would allow.
#[trial]
fn the_hosts_test_refuses_a_landing() {
    let rows = rows();
    let rules = PresetReorder::<u32>::items().accepts(|drag, over, _| drag.grabbed != A_ONE || over != Some(&B_ONE));
    assert!(!rules.allows(&rows, &drag(A_ONE), Some(&B_ONE), 5));
    assert!(rules.allows(&rows, &drag(A_ONE), Some(&INNER_ONE), 3));
}

const AMBER: u32 = 101;
const CORAL: u32 = 102;

/// A secondary click on a preset that is not the selected one opens the row menu for that preset, and the menu's
/// Rename renames it in a field over its tile, which has no caption.
#[trial]
fn the_row_menu_of_an_unselected_preset_acts_on_that_preset() {
    let state = Rc::new(RefCell::new(PresetState::<u32>::new()));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let reported = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let rows = vec![PresetRow::item(AMBER, "Amber", 0), PresetRow::item(CORAL, "Coral", 0)];
        let mut view = state.borrow_mut();
        let out = PresetBrowser::new("Swatches", Bind::bound(&mut *view), &rows).selected(AMBER).show(ui);
        reported.borrow_mut().extend(out.events);
    });
    fixture.settle();
    let at = fixture.by_label("Coral").expect("the preset's tile").rect.center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
    let rename = fixture.by_label("Rename…").expect("a secondary click on a preset opens the row menu");
    fixture.click(&rename);
    fixture.idle(3);
    fixture.type_text("Plum");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    let renamed = seen.borrow().iter().any(|event| matches!(event, PresetEvent::Rename { key: CORAL, text, .. } if text == "Plum"));
    assert!(renamed, "events: {:?}", seen.borrow());
}

/// A rename requested by the id the browser was given reaches it from outside the scope it is shown in.
#[trial]
fn a_rename_request_names_the_browser_by_its_own_id() {
    let state = Rc::new(RefCell::new(PresetState::<u32>::new()));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let reported = seen.clone();
    let browser = Id::new("swatch-browser");
    let mut fixture = Fixture::new(move |ui| {
        let rows = vec![PresetRow::item(AMBER, "Amber", 0), PresetRow::item(CORAL, "Coral", 0)];
        let mut view = state.borrow_mut();
        ui.vertical(|ui| {
            let out = PresetBrowser::new("Swatches", Bind::bound(&mut *view), &rows).browser_id(browser).show(ui);
            reported.borrow_mut().extend(out.events);
        });
    });
    fixture.settle();
    PresetBrowser::<u32>::request_rename(fixture.ctx(), browser, &CORAL);
    fixture.idle(3);
    fixture.type_text("Plum");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    let renamed = seen.borrow().iter().any(|event| matches!(event, PresetEvent::Rename { key: CORAL, text, .. } if text == "Plum"));
    assert!(renamed, "events: {:?}", seen.borrow());
}
