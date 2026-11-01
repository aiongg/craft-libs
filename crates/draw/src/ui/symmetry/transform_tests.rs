//! Tests of Transform Symmetry on the canvas tool host (task TO-S2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Through `ToolHost`: a drag from the centre moves it, from the handle turns the axes (Shift in absolute 15° steps), from elsewhere is claimed and changes nothing; events are `Preview` gestures.
#[test]
#[ignore = "skeleton"]
fn centre_moves_handle_turns_with_shift_steps_elsewhere_does_nothing() {
    todo!()
}

// Enter gives Commit of the preview, Esc gives Cancel, an input with `valid` false gives Cancel once and takes no press.
#[test]
#[ignore = "skeleton"]
fn enter_commits_escape_cancels_invalid_cancels() {
    todo!()
}

// Move over the centre, Alias over the handle, the arrow elsewhere.
#[test]
#[ignore = "skeleton"]
fn cursor_follows_the_part_under_the_pointer() {
    todo!()
}

// With Center picked the arrows move it 1, Shift 10; with the handle picked Right turns 1° clockwise, Shift 15°, Left the other way; each is a `Preview(Set)`.
#[test]
#[ignore = "skeleton"]
fn keys_move_the_center_and_turn_the_handle() {
    todo!()
}
