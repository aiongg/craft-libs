//! Behaviour tests of the grouping marks and `QuietMarks` (task H2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `mark`, three records, `squash_since`: one new state with the first record's label; the later records return `extended`.
#[test]
#[ignore = "skeleton"]
fn a_gesture_is_one_state() {
    todo!()
}

// `mark` then `revert_to` with no record leaves the rows and the revision as they were.
#[test]
#[ignore = "skeleton"]
fn an_unchanged_gesture_leaves_no_state() {
    todo!()
}

// `mark`, two records, `revert_to`: the run's state is dropped and reported, the marked state is current.
#[test]
#[ignore = "skeleton"]
fn cancel_removes_the_state_the_gesture_made() {
    todo!()
}

// Three `record_keyed` with one key make one state; a different key starts a new one.
#[test]
#[ignore = "skeleton"]
fn keyed_records_with_the_same_key_merge_into_one_state() {
    todo!()
}

// `record_keyed`, `back`, `forward`, `record_keyed` with the same key adds a state; so does a plain `record` between them.
#[test]
#[ignore = "skeleton"]
fn a_jump_or_another_record_ends_a_keyed_run() {
    todo!()
}

// `GestureHost<QuietMarks<_>>` begin, update, commit and cancel leave a `StepList` untouched and restore the value on cancel.
#[test]
#[ignore = "skeleton"]
fn quiet_marks_never_touch_a_step_list() {
    todo!()
}
