//! The styles panel in a harness.

// A click with a target applies, Alt-click applies clearing overrides, double-click asks for
// Style Options; "+" follows an overridden current style.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn rows_apply_clear_overrides_and_open_options() {
    todo!()
}

// A character style click at a bare caret applies nothing and sets the "Select text…" status;
// Delete and Redefine do nothing for character style 0.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn character_style_rules_at_a_caret_and_for_none() {
    todo!()
}

// With a highlight: Shift-click highlights a range and Cmd/Ctrl-click toggles one without
// applying; Delete with three highlighted returns DeleteHighlighted; a highlighted row dropped on
// New returns DuplicateHighlighted, an unhighlighted one Duplicate(id).
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn highlighted_rows_act_together() {
    todo!()
}
