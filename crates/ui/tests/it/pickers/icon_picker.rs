//! `IconPicker`: the search narrows the grid and a pick writes its key.

use craft_ui::contract::Bind;
use craft_ui::pickers::{IconPicker, IconPickerGroup, IconPickerItem};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

const BRUSH: u8 = 3;
const PEN: u8 = 5;

fn groups() -> Vec<IconPickerGroup<u8>> {
    vec![
        IconPickerGroup::new("Selection", vec![IconPickerItem::new(1, "Move")]),
        IconPickerGroup::new("Drawing", vec![IconPickerItem::new(BRUSH, "Brush"), IconPickerItem::new(4, "Eraser"), IconPickerItem::new(PEN, "Pen")]),
    ]
}

/// Typing in the search keeps the cells that match, and Down then Enter picks the first of them.
#[trial]
fn the_search_filters_the_cells_and_enter_picks_the_first_match() {
    let picked = Rc::new(Cell::new(None::<u8>));
    let shown = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shown.get();
        let groups = groups();
        IconPicker::new("Tool", Bind::bound(&mut value), &groups).show(ui);
        shown.set(value);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("pen");
    fixture.idle(2);
    assert!(fixture.by_label("Pen").is_some(), "the matching cell shows");
    assert!(fixture.by_label("Brush").is_none(), "a cell that does not match is hidden");
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(picked.get(), Some(PEN));
}

/// A group the user closed stays closed over later frames and after a search is typed and cleared.
#[trial]
fn a_closed_group_stays_closed_over_frames_and_a_cleared_search() {
    let mut fixture = Fixture::new(|ui| {
        let mut value = None::<u8>;
        let groups = groups();
        IconPicker::new("Tool", Bind::bound(&mut value), &groups).show(ui);
    });
    fixture.settle();
    assert!(fixture.by_label("Brush").is_some(), "groups start open");
    let header = fixture.by_label("Drawing").expect("the group's header");
    fixture.access(AccessActionKind::Collapse, &header);
    fixture.idle(2);
    assert!(fixture.by_label("Brush").is_none(), "the closed group shows its header only");
    fixture.idle(3);
    assert!(fixture.by_label("Brush").is_none(), "it stays closed over later frames");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("br");
    fixture.idle(2);
    fixture.key(Key::Backspace, Modifiers::NONE);
    fixture.key(Key::Backspace, Modifiers::NONE);
    fixture.idle(2);
    assert!(fixture.by_label("Brush").is_none(), "it stays closed after the search is cleared");
    assert!(fixture.by_label("Move").is_some(), "the other group is still open");
}
