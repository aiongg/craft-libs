//! The dialog host in a harness: ordering, results, the startup queue.

// Two dialogs opened in turn: the first is disabled while the second is up, and takes the input
// again once the second finishes.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn only_the_newest_open_dialog_takes_the_input() {
    todo!()
}

// OK on a typed dialog puts `DialogResult::Ok(value)` in `take::<D>()` on the next pass, once.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_finished_dialog_is_taken_once_with_its_typed_result() {
    todo!()
}

// Esc on a child closes only the child; cancelling the parent closes both with Cancel results.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn cancelling_a_parent_cancels_its_children() {
    todo!()
}

// Modals enqueued as Update, Recovery, Warning open one at a time in Recovery, Warning, Update
// order, each after the previous one closed (gap 75).
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn startup_modals_open_one_at_a_time_in_stage_order() {
    todo!()
}

// Opening a second dialog of the same type cancels the first; its result is Cancel.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn opening_the_same_dialog_type_again_cancels_the_open_one() {
    todo!()
}
