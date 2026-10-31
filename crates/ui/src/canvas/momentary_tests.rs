//! Momentary keys and spring-loaded tool letters.

// Space then Cmd and Cmd then Space both give ZoomIn; Space alone gives Hand.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn space_holds_the_hand_and_cmd_space_zooms_in_in_either_order() {
    todo!()
}

// Cmd+Alt+Space gives ZoomOut.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn alt_turns_a_held_zoom_in_into_zoom_out() {
    todo!()
}

// Space up during a Hand drag still gives Hand until the pass the button is released.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_drag_keeps_its_temporary_tool_until_the_release() {
    todo!()
}

// With drawing true, Space gives None.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn space_while_drawing_is_not_the_hand() {
    todo!()
}

// A focused TextEdit leaves Space to the field.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn no_temporary_tool_while_a_text_field_has_the_keys() {
    todo!()
}

// Held 0.3 s, releasing the letter returns the previous tool.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_held_letter_springs_back_on_release() {
    todo!()
}

// A tap under 0.3 s with no canvas use keeps the new tool.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_quick_tap_switches_for_good() {
    todo!()
}

// canvas_used during a short hold springs back on release.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn using_the_canvas_makes_a_short_hold_temporary() {
    todo!()
}

// tick returns None while in_gesture, the previous tool after.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_drag_running_at_release_finishes_with_the_sprung_tool() {
    todo!()
}
