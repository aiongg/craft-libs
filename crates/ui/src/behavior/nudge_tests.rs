//! Tests of the arrow key nudge.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `NudgeKeys::read` gives 1 for a plain arrow, 10 with Shift, and the alt flag with `.alt(true)`.
#[test]
#[ignore = "skeleton"]
fn context_read_takes_plain_shift_and_alt_arrows() {
    todo!()
}

// With `.base(Modifiers::COMMAND)` only ⌘/Ctrl arrows are read; without a base, they are not.
#[test]
#[ignore = "skeleton"]
fn base_modifier_must_be_held_and_nothing_else_of_command() {
    todo!()
}

// A focused text field, an open popup and a modal layer above `layer` each block `read`.
#[test]
#[ignore = "skeleton"]
fn reads_nothing_under_a_text_field_popup_or_modal() {
    todo!()
}

// `from_key` gives what `read` gives for the same key and modifiers, and consumes nothing.
#[test]
#[ignore = "skeleton"]
fn from_key_parses_a_tool_key_without_consuming() {
    todo!()
}

// In a right-to-left UI Right is still +x and Left −x.
#[test]
#[ignore = "skeleton"]
fn right_is_plus_x_in_a_right_to_left_ui() {
    todo!()
}

// The old `read(ui, ..)` and `read_with_alt` give the same moves as `NudgeKeys` with the ui's layer.
#[test]
#[ignore = "skeleton"]
fn old_readers_delegate_to_nudge_keys() {
    todo!()
}
