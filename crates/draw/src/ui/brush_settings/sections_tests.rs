//! Tests of the Brush Settings section list (task BR-BS4).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on an off pane row's name reports `Select` then `Enable { on: true }`; on an on row only `Select`.
#[test]
#[ignore = "skeleton"]
fn a_click_on_a_pane_row_selects_and_enables_it() {
    todo!()
}

// A click on the enable box reports `Enable` with the flipped state and no `Select`.
#[test]
#[ignore = "skeleton"]
fn the_enable_box_toggles_without_selecting() {
    todo!()
}

// A click on a switch row reports `Switch` with the flipped state and never `Select`.
#[test]
#[ignore = "skeleton"]
fn switch_rows_only_toggle() {
    todo!()
}

// A click on the lock reports `Lock` with the flipped state only; the lock shows on unlocked rows too.
#[test]
#[ignore = "skeleton"]
fn the_lock_toggles_without_selecting() {
    todo!()
}

// Focused list: Down onto a pane row reports `Select`; Space toggles its box (`Enable`) or a switch
// (`Switch`); Shift+Space its lock; Enter on an off pane row reports `Select` and `Enable`.
#[test]
#[ignore = "skeleton"]
fn section_keys_move_toggle_and_lock() {
    todo!()
}

// With `link(true)` the "Brushes" row shows first and a click reports `OpenLink`; without it, no row.
#[test]
#[ignore = "skeleton"]
fn the_link_row_reports_open_link() {
    todo!()
}

// AccessKit: pane rows are Buttons with `selected`, boxes CheckBoxes "Enable {name}", locks
// Buttons "Lock: {name}" with `selected`, switch rows one CheckBox each.
#[test]
#[ignore = "skeleton"]
fn section_rows_have_their_accessible_roles_and_names() {
    todo!()
}
