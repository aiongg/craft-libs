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
        .unwrap_or_else(|| panic!("the widest row is painted; the list is at {:?}, the texts are {painted:?}", list.rect))
        .rect;
    assert!(list.rect.width() >= label.width(), "the list is as wide as its widest row: {:?} holds {label:?}", list.rect);
}

/// A content-sized list builds every row once to measure, in the measuring run; the real pass builds the rows it
/// shows and its `extent` builds none.
#[trial]
fn a_content_list_builds_each_row_once_to_measure() {
    use std::cell::Cell;
    use std::rc::Rc;
    const ROWS: usize = 50;
    let words = Words::flat(&["row"; ROWS]);
    let (measuring, real) = (Rc::new(Cell::new(0usize)), Rc::new(Cell::new(0usize)));
    let (m, r) = (measuring.clone(), real.clone());
    let mut fixture = Fixture::new(move |ui| {
        let ctx = ui.ctx().clone();
        let _ = List::<usize>::new("Rows").sizing(Sizing::Content).show_items(ui, &words, &mut |key, _| {
            let counter = if craft_ui::testing::measuring_active(&ctx) { &m } else { &r };
            counter.set(counter.get() + 1);
            ListRow::new(words.words[*key])
        });
    });
    fixture.room(Vec2::new(300.0, 2000.0)).settle();
    measuring.set(0);
    real.set(0);
    fixture.frame(vec![]);
    let passes = fixture.passes();
    assert_eq!(measuring.get(), ROWS * passes, "each row is built once per pass to measure");
    assert_eq!(real.get(), ROWS * passes, "the real run builds the shown rows once per pass");
}
