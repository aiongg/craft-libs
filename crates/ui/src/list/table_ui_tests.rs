//! `Table` in a harness.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A header click reports `TableEvent::Sort`; the header then reads "Name, sorted ascending" with `sort_direction`.
#[test]
#[ignore = "skeleton"]
fn clicking_a_header_sorts_and_reads_its_order() {
    todo!()
}

// A drag on the edge between two headers widens the left column, not past its least width, and writes every width into the layout.
#[test]
#[ignore = "skeleton"]
fn dragging_a_column_edge_resizes_it_and_keeps_the_widths() {
    todo!()
}

// The header menu's check row hides a hideable column and the menu stays open; Reset Columns shows it again.
#[test]
#[ignore = "skeleton"]
fn the_header_menu_hides_a_column_and_stays_open() {
    todo!()
}

// Dropping a header past another's centre moves its column after that one and reports `Layout`.
#[test]
#[ignore = "skeleton"]
fn a_header_dragged_onto_another_moves_the_column() {
    todo!()
}

// Tab reaches the body once; ↓ and Shift+↓ select as a `List` does; Enter reports `Activate`.
#[test]
#[ignore = "skeleton"]
fn the_body_is_one_tab_stop_with_the_list_keys() {
    todo!()
}

// Tab from the first header reaches its edge; → widens the column by 8 points and ← narrows it,
// not below its least width; each press reports `Layout` and writes the widths; the edge reads as
// a `Splitter` "Resize Name" with the width as its value, and SetValue sets it.
#[test]
#[ignore = "skeleton"]
fn arrow_keys_on_a_focused_column_edge_resize_the_column() {
    todo!()
}
