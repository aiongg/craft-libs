//! A list row's detail line counts in the row's width: a list sized from its content shows the detail whole.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{ShowCollection as _, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::list::{List, ListRow};
use craft_ui::testing::{Fixture, trial};
use egui::Vec2;

const DETAIL: &str = "Current Time · Program Monitor";

struct One;

impl DisplayOrder<usize> for One {
    fn len(&self) -> usize {
        1
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for One {}

#[trial]
fn a_content_sized_list_is_as_wide_as_a_detail_longer_than_its_label() {
    let mut fixture = Fixture::new(|ui| {
        Stack::new().show(ui, |ui, _| {
            let _ = List::new("Times").show_items(ui, &One, &mut |_, _| ListRow::new("00:01").detail(DETAIL));
        });
    });
    fixture.room(Vec2::new(800.0, 300.0)).settle();
    let detail = fixture.texts().iter().find(|text| text.text == DETAIL).expect("the row paints its detail line");
    assert!(!detail.elided, "the detail line is whole while the room has space: {:?}", detail.rect);
}
