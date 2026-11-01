//! Tests of the Lasso on the tool host (task SE-F9). The draft's own rules are tested in
//! `craft_ui::canvas::draft`; these check what the tool adds: events, operation, keys, the drag
//! inside the selection.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A freehand drag commits one `Polygon` with the operation latched at the press (⇧ Add), and
// nothing more.
#[test]
#[ignore = "skeleton"]
fn lasso_commits_closed_outline_with_latched_operation() {
    todo!()
}

// Between ⌥ clicks the tool takes Backspace (retract) and Enter (close) and keeps the keys from
// the app; with no outline open it takes neither.
#[test]
#[ignore = "skeleton"]
fn lasso_takes_retract_and_enter_only_while_open() {
    todo!()
}

// Releasing ⌥ with the button up closes the outline in the next pass (`each_pass`), and a
// double-click closes it.
#[test]
#[ignore = "skeleton"]
fn lasso_alt_release_and_double_click_close() {
    todo!()
}

// A plain press inside the selection drags the outline instead of drawing.
#[test]
#[ignore = "skeleton"]
fn lasso_press_inside_drags_selection() {
    todo!()
}

// An open outline is dropped without an event by `reset` and by a change of the probe's document.
#[test]
#[ignore = "skeleton"]
fn lasso_dropped_on_reset_or_document_change() {
    todo!()
}
