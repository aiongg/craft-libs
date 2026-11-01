//! Behaviour tests of the step list core (task H1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A record makes the new state current and returns a new id, never one used before.
#[test]
#[ignore = "skeleton"]
fn record_adds_one_state_after_the_current_one() {
    todo!()
}

// After back, back, record: the two later ids come out of `take_dropped` once, and a second call is empty.
#[test]
#[ignore = "skeleton"]
fn linear_record_drops_later_states_and_reports_them() {
    todo!()
}

// Non-linear: back, record keeps the later rows in order and puts the new state at the end, current.
#[test]
#[ignore = "skeleton"]
fn non_linear_record_keeps_later_states_before_the_new_one() {
    todo!()
}

// `go_to` back and forward returns the `Jump`; no id is dropped; `go_to` the current id is `None`.
#[test]
#[ignore = "skeleton"]
fn go_to_moves_the_current_state_and_keeps_later_states() {
    todo!()
}

// With limit 3 and the oldest pinned, a fifth record drops the next oldest, never the pinned or current state.
#[test]
#[ignore = "skeleton"]
fn limit_drops_the_oldest_past_state_but_not_the_current_or_the_source() {
    todo!()
}

// `evict_oldest` skips the pinned source, returns `None` once only the current and pinned states are left.
#[test]
#[ignore = "skeleton"]
fn evict_oldest_spares_the_pinned_source_and_the_current_state() {
    todo!()
}

// Linear: deleting row 2 of 5 (current 4) leaves rows 0 and 1, row 1 current, three ids dropped.
#[test]
#[ignore = "skeleton"]
fn delete_in_linear_history_drops_the_state_and_everything_after_it() {
    todo!()
}

// Non-linear: deleting a later state drops only it; deleting the current state makes the one before it current.
#[test]
#[ignore = "skeleton"]
fn delete_in_non_linear_history_drops_only_that_state() {
    todo!()
}

// `delete(oldest)` is `Err(Oldest)` and changes nothing, revision included.
#[test]
#[ignore = "skeleton"]
fn the_oldest_state_cannot_be_deleted() {
    todo!()
}

// Linear: `delete(later)` is `Err(Later)`.
#[test]
#[ignore = "skeleton"]
fn a_later_state_cannot_be_deleted_in_linear_history() {
    todo!()
}

// Clear leaves one row and drops nothing; restore brings rows, current and pin back; after clear then record the set-aside ids are dropped.
#[test]
#[ignore = "skeleton"]
fn clear_keeps_only_the_current_state_and_restore_brings_the_rest_back() {
    todo!()
}

// `source()` is the oldest without a pin; after deleting the pinned state it is `None`.
#[test]
#[ignore = "skeleton"]
fn source_defaults_to_the_oldest_state_and_is_gone_after_its_state_is_deleted() {
    todo!()
}

// Every mutating call that changes the list bumps `revision`; reads and refused calls do not.
#[test]
#[ignore = "skeleton"]
fn revision_changes_on_every_change_and_not_on_reads() {
    todo!()
}
