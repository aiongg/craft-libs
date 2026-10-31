//! Behaviour tests of `testing::frame` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// click_at clicks a button once, and click_at_with delivers Shift with the press (a button reading modifiers sees them).
#[test]
#[ignore = "skeleton"]
fn clicks_arrive_press_and_release_on_separate_frames() {
    todo!()
}

// drag moves a DragValue by the pointer distance and ends with the button up.
#[test]
#[ignore = "skeleton"]
fn drag_moves_through_every_step_and_releases() {
    todo!()
}

// press(Cmd+Z) is seen by ctx.input_mut().consume_shortcut, and type_text fills a focused TextEdit.
#[test]
#[ignore = "skeleton"]
fn keys_and_text_reach_the_focused_widget() {
    todo!()
}

// pump_idle returns true once a worker thread's flag clears, and false at the timeout while it never does.
#[test]
#[ignore = "skeleton"]
fn pumps_wait_for_background_work_and_time_out() {
    todo!()
}
