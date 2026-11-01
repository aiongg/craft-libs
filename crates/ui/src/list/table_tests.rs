//! `TableSort` and `TableLayout` rules.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `TableSort::clicked`: the sorting column flips its order; another column starts ascending.
#[test]
#[ignore = "skeleton"]
fn a_header_click_flips_the_sorting_column_and_sorts_another_ascending() {
    todo!()
}

// `TableSort::apply` is stable and reverses the comparison for Descending.
#[test]
#[ignore = "skeleton"]
fn descending_reverses_and_ties_keep_their_order() {
    todo!()
}

// `set_visible(false)` then `(true)` leaves `order` unchanged.
#[test]
#[ignore = "skeleton"]
fn a_hidden_column_comes_back_where_it_was() {
    todo!()
}

// `repaired` drops unknown keys, appends missing columns, drops non-finite widths and clamps the rest.
#[test]
#[ignore = "skeleton"]
fn a_stored_layout_is_repaired_against_the_columns() {
    todo!()
}

// `move_column` with and without `after`.
#[test]
#[ignore = "skeleton"]
fn a_dropped_header_moves_its_column_before_or_after_the_target() {
    todo!()
}
