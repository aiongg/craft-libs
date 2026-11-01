//! Behaviour tests of snapshots, the source choice and the forms (task H3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Add, remove, add: the second id differs from the first, and the suggestion counter keeps counting.
#[test]
#[ignore = "skeleton"]
fn snapshot_ids_are_never_reused() {
    todo!()
}

// `add(None)` and `add(Some("  "))` give "Snapshot 1" and "Snapshot 2"; `add(Some(" A "))` gives "A".
#[test]
#[ignore = "skeleton"]
fn a_blank_name_takes_the_suggestion_and_names_are_trimmed() {
    todo!()
}

// `rename(id, "  ")` returns false and keeps the name; a real name is trimmed.
#[test]
#[ignore = "skeleton"]
fn rename_refuses_a_blank_name() {
    todo!()
}

// After `set(Snapshot(id))`, `resolve` is that snapshot; after removing it, `None`.
#[test]
#[ignore = "skeleton"]
fn the_source_resolves_to_a_snapshot_when_one_is_chosen() {
    todo!()
}

// Choosing a snapshot unpins the state; choosing a state pins it in the step list.
#[test]
#[ignore = "skeleton"]
fn a_snapshot_source_and_a_state_source_are_separate_choices() {
    todo!()
}

// After `reset`, `resolve` is the oldest state and nothing is pinned.
#[test]
#[ignore = "skeleton"]
fn reset_returns_to_the_oldest_state() {
    todo!()
}

// The form's fields read back through `SnapshotOptions::from_fields`; no From row without options.
#[test]
#[ignore = "skeleton"]
fn new_snapshot_form_returns_name_and_from_by_key() {
    todo!()
}

// `nonLinear` and an extra row's key are both in the fields; `HistoryOptions::from_fields` reads the check.
#[test]
#[ignore = "skeleton"]
fn history_options_form_returns_the_check_and_the_extra_rows() {
    todo!()
}
