//! In-place rename of a table row.

use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, EditEvent, ItemSource};
use craft_ui::contract::{CollectionEvent, ColumnSpec, ShowCollection as _};
use craft_ui::list::ListRow;
use craft_ui::table::Table;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

struct Names;

impl DisplayOrder<usize> for Names {
    fn len(&self) -> usize {
        1
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key == 0).then_some(0)
    }

    fn text(&self, _: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed("Background"))
    }
}

impl ItemSource<usize> for Names {}

/// The rename field opens with the whole name selected, so typing replaces the name instead of extending it.
#[trial]
fn typing_in_a_row_rename_replaces_the_name() {
    let committed = Rc::new(RefCell::new(Vec::new()));
    let seen = committed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let columns = vec![ColumnSpec::new(ColumnKey::new("name"), "Name", ColumnWidth::Fixed(160.0))];
        let out = Table::<usize>::new("Layers", columns).rename(true).show_items(ui, &Names, &mut |_, _| ListRow::new("Background"));
        for event in out.events {
            if let CollectionEvent::Rename(EditEvent::Commit { after, .. }) = event {
                seen.borrow_mut().push(after.name);
            }
        }
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::F2, Modifiers::NONE);
    fixture.idle(2);
    fixture.type_text("New");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*committed.borrow(), ["New".to_owned()]);
}
