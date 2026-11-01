//! Screen and AccessKit tests of the undo list (task ST3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Hover on row 2 fills rows 0 to 2 with `list.hover`.
#[test]
#[ignore = "skeleton"]
fn hovering_a_row_marks_it_and_every_newer_row() {
    todo!()
}

// A click on row 2 reports `UndoThrough(3)`.
#[test]
#[ignore = "skeleton"]
fn a_click_reports_the_count_of_marked_rows() {
    todo!()
}

// With no row marked the footer is the Cancel row; clicking it reports nothing and closes the popup.
#[test]
#[ignore = "skeleton"]
fn the_footer_says_cancel_while_nothing_is_marked_and_the_cancel_row_closes() {
    todo!()
}

// Down twice then Enter reports `UndoThrough(2)`; the marked row stays in view past 18 rows.
#[test]
#[ignore = "skeleton"]
fn arrow_keys_move_the_mark_and_enter_commits() {
    todo!()
}
