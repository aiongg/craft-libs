//! The shared cell-selection rules.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A range grows over a merge it crosses and again over a merge that one reaches; whole columns
// stay as they are in `CellSpace::cover`.
#[test]
#[ignore = "skeleton"]
fn ranges_grow_over_merged_cells_but_whole_lines_stay() {
    todo!()
}

// `neighbour` leaves a merge from its far edge, skips hidden rows and columns, lands on a merge's
// top-left, steps over unselectable cells and is None at the edge.
#[test]
#[ignore = "skeleton"]
fn neighbour_skips_hidden_lines_and_merges() {
    todo!()
}

// A click selects a merge whole with its top-left active; Shift+click extends from the anchor and
// keeps the active cell; Cmd+click adds a range that becomes active.
#[test]
#[ignore = "skeleton"]
fn click_shift_click_and_command_click_follow_excel() {
    todo!()
}

// Shift+arrows move the head (the corner opposite the anchor), not the active cell; a plain arrow
// collapses to the active cell's neighbour.
#[test]
#[ignore = "skeleton"]
fn shift_arrows_move_the_head_and_arrows_collapse() {
    todo!()
}

// Enter and Tab cycle the active cell through every range of a multi-range selection, both ways,
// and keep the selection; on a single merged cell they move the selection.
#[test]
#[ignore = "skeleton"]
fn enter_and_tab_cycle_inside_the_selection() {
    todo!()
}

// Tab, Tab, Enter goes to the next row in the column the Tabs started in; an arrow ends the run.
#[test]
#[ignore = "skeleton"]
fn enter_after_tabs_returns_to_the_starting_column() {
    todo!()
}

// A document table's Tab runs on to the next row, visits a merge once and is None past the last
// cell and before the first.
#[test]
#[ignore = "skeleton"]
fn reading_order_tab_wraps_rows_and_ends_past_the_last_cell() {
    todo!()
}
