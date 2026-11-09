//! A content-sized list with columns in a room wider than its rows.

use super::kit::Words;
use craft_behavior::{ColumnKey, ColumnWidth};
use craft_ui::contract::{ColumnKind, ColumnSpec, Role, ShowCollection as _, Sizing, ToggleColumn};
use craft_ui::list::{List, ListRow, ToggleCell, ToggleState};
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2};

const CHECK: ColumnKey = ColumnKey::new("check");
const ROOM: Vec2 = Vec2::new(900.0, 300.0);

/// The rect of a `Content` list of three rows in `ROOM`, with or without a toggle column, and its first toggle cell.
fn content_list(toggles: bool) -> (Rect, Option<Rect>) {
    let words = Words::flat(&["Short", "The widest row label of the three", "Mid row"]);
    let mut fixture = Fixture::new(move |ui| {
        let list = List::<usize>::new("Effects").sizing(Sizing::Content).header(false);
        let list = if toggles {
            list.columns(vec![ColumnSpec::new(CHECK, "", ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::default()))])
        } else {
            list
        };
        let _ = list.show_items(ui, &words, &mut |key, icx| {
            let row = ListRow::new(words.words[*key]).cells(icx.cells());
            if toggles { row.toggle(ToggleCell::check(CHECK, ToggleState::of(false), words.words[*key])) } else { row }
        });
    });
    fixture.room(ROOM).settle();
    let list = fixture.by_role(Role::LIST).into_iter().next().expect("the list is shown").rect;
    (list, fixture.by_role(Role::CHECK_BOX).first().map(|toggle| toggle.rect))
}

/// A `Content` list with a toggle column takes the toggle cell beside its widest row, not the room: it is as wide
/// as the same list without the column (its widest text and insets) plus the toggle cell.
#[trial]
fn a_content_list_with_a_toggle_column_is_as_wide_as_its_toggle_cell_and_widest_text() {
    let (plain, _) = content_list(false);
    let (list, toggle) = content_list(true);
    let toggle = toggle.expect("the rows show their toggle cells");
    assert!(list.min.x <= toggle.min.x && toggle.max.x <= list.max.x, "the toggle cell is inside the list: {toggle:?} in {list:?}");
    // The check box sits centred in its cell, which starts at the list's edge.
    let cell = toggle.width() + 2.0 * (toggle.min.x - list.min.x);
    let wanted = plain.width() + cell;
    assert!((list.width() - wanted).abs() <= 1.0, "the list is {list:?}, the plain list {plain:?} and the toggle cell {toggle:?}");
    assert!(list.max.x <= ROOM.x / 2.0, "the list ends at its content, inside the room's start plus its width: {list:?}");
}
