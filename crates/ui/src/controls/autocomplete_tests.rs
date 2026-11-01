//! `Autocomplete` in a harness.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// With the list showing, ↓ past the last row goes to the first; Tab reports `accepted` with the active row and the editor never sees the Tab.
#[test]
#[ignore = "skeleton"]
fn arrows_wrap_and_tab_accepts() {
    todo!()
}

// Esc reports `dismissed`; the editor keeps its text and its focus; a second Esc reaches the editor.
#[test]
#[ignore = "skeleton"]
fn esc_closes_the_list_and_keeps_the_text() {
    todo!()
}

// `completion_rest("Seattle", "sea")` is "ttle"; a typed text longer than or different from it gives `None`.
#[test]
#[ignore = "skeleton"]
fn the_rest_of_a_completion_follows_the_typed_text() {
    todo!()
}
