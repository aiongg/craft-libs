//! `IconPicker`: the search narrows the grid and a pick writes its key.

use craft_ui::contract::Bind;
use craft_ui::pickers::{IconPicker, IconPickerGroup, IconPickerItem};
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
