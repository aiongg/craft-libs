//! A press on a row's part (17.3 step 8): the lead and key focus go to the row, so the collection's keys and Tab
//! work from there.

use super::kit::{List, ROW_H, Row, Words};
use craft_ui::buttons::Button;
use craft_ui::contract::{Props as _, Role, SelectMode, Show as _, ShowCollection as _};
use craft_ui::testing::access::focused_key;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

fn region() -> Rect {
    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(300.0, 200.0))
}

/// A click on a row's part leaves the lead and key focus on that row; Tab then leaves the list.
#[trial]
fn a_click_on_a_rows_part_puts_focus_on_the_row() {
    let words: &'static Words = Box::leak(Box::new(Words::flat(&["warn one", "warn two"])));
    let ids = Rc::new((Cell::new(Id::NULL), Cell::new(Id::NULL)));
    let log = ids.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out =
            in_region(ui, region(), |ui| List::new(SelectMode::Single).id_salt("list").show_items(ui, words, &mut |_, _| Row::new().dismissable()));
        log.0.set(out.response.id);
        log.1.set(Button::new("After").show(ui).id);
    });
    fixture.settle();
    let mut rows = fixture.by_role(Role::LIST_ITEM);
    rows.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    let row = rows.get(1).copied().expect("the second row");
    fixture.click_at(row.rect.min + Vec2::splat(ROW_H / 2.0)).settle();
    assert_eq!(fixture.focused(), Some(ids.0.get()), "key focus is on the list");
    assert_eq!(focused_key(fixture.tree()), row.node, "the focused node is the pressed row");
    fixture.key(Key::Tab, Modifiers::NONE).settle();
    assert_eq!(fixture.focused(), Some(ids.1.get()), "Tab leaves the list");
}
