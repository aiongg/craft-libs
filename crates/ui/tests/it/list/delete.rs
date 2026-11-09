//! `List::deletable`: Delete and Backspace at the lead report the item to delete.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{CollectionEvent, SelectMode, ShowCollection as _};
use craft_ui::list::{List, ListRow};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

struct Three;

impl DisplayOrder<usize> for Three {
    fn len(&self) -> usize {
        3
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Three {}

/// A list of three rows with focus on it, whose Delete reports are collected.
fn rig(deletable: bool) -> (Fixture, Rc<RefCell<Vec<Vec<usize>>>>) {
    let reported = Rc::new(RefCell::new(Vec::new()));
    let sink = reported.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = List::new("Items")
            .select(SelectMode::Single)
            .deletable(deletable)
            .show_items(ui, &Three, &mut |key, _| ListRow::new(format!("Row {key}")));
        for event in out.events {
            if let CollectionEvent::Delete(keys) = event {
                sink.borrow_mut().push(keys);
            }
        }
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    for _ in 0..2 {
        fixture.key(Key::ArrowDown, Modifiers::NONE);
        fixture.idle(2);
    }
    (fixture, reported)
}

/// Delete and Backspace on the row at the lead of a deletable list report that row; a list that is not deletable
/// reports nothing, so the keys stay shortcut candidates.
#[trial]
fn delete_reports_the_row_only_on_a_deletable_list() {
    for (deletable, expected) in [(true, vec![vec![1], vec![1]]), (false, vec![])] {
        let (mut fixture, reported) = rig(deletable);
        fixture.key(Key::Delete, Modifiers::NONE);
        fixture.idle(2);
        fixture.key(Key::Backspace, Modifiers::NONE);
        fixture.idle(2);
        assert_eq!(*reported.borrow(), expected, "deletable: {deletable}");
    }
}
