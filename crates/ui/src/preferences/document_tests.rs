//! `SettingsDocument` without a window.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A struct of a bool, an int with a range, a choice and a colour gives one row of each kind on
// its page, in field order.
#[test]
#[ignore = "skeleton"]
fn rows_follow_the_value_types_and_rules() {
    todo!()
}

// Labels: "historyStates" → "History states", "gpuMemoryMb" style replacements, a rule's label
// wins, and a translation under setting.<key> replaces the English.
#[test]
#[ignore = "skeleton"]
fn labels_come_from_the_key_or_the_catalog() {
    todo!()
}

// set clamps to the range; Apply returns the typed value once through take_applied; Cancel
// restores the draft.
#[test]
#[ignore = "skeleton"]
fn apply_returns_the_typed_value_and_cancel_drops_the_draft() {
    todo!()
}

// Reset Section restores the page's defaults except keep_on_reset keys; other pages keep their
// draft.
#[test]
#[ignore = "skeleton"]
fn reset_section_keeps_marked_keys() {
    todo!()
}

// Hidden rules, linux_only off Linux and shown_when with another value hide the row.
#[test]
#[ignore = "skeleton"]
fn hidden_rows_stay_hidden() {
    todo!()
}
