//! A collection in a body that scrolls sideways.

use super::kit::Words;
use craft_ui::contract::{Role, Scroll, ShowCollection as _, ShowContainer as _, Sizing};
use craft_ui::layout_components::Stack;
use craft_ui::list::{List, ListRow};
use craft_ui::testing::{Fixture, trial};
use egui::Vec2;

const LONG: &str = "A row label that is far wider than the room the body offers";

/// The room along a sideways-scrolling body is unbounded: a content-sized list takes the width of its widest row
/// instead of none.
#[trial]
fn a_list_in_a_sideways_scrolling_body_measures_its_widest_row() {
    let words = Words::flat(&["short", LONG, "mid"]);
    let mut fixture = Fixture::new(move |ui| {
        Stack::new().scroll(Scroll::horizontal()).show(ui, |ui, _| {
            let _ = List::<usize>::new("Rows").sizing(Sizing::Content).show_items(ui, &words, &mut |key, _| ListRow::new(words.words[*key]));
        });
    });
    fixture.room(Vec2::new(200.0, 160.0)).settle();
    let list = fixture.by_role(Role::LIST).into_iter().next().expect("the list is shown");
    let painted: Vec<String> = fixture.texts().iter().map(|text| format!("{:?} at {:?}", text.text, text.rect)).collect();
    let label = fixture
        .texts()
        .iter()
        .find(|text| text.text == LONG)
        .unwrap_or_else(|| panic!("PROBE14 the widest row is painted; the list is at {:?}, the texts are {painted:?}", list.rect))
        .rect;
    assert!(list.rect.width() >= label.width(), "the list is as wide as its widest row: {:?} holds {label:?}", list.rect);
}
