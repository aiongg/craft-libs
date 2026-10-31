//! The tool host: one tool's presses, drags, releases, keys and paint.

// A drag in a zoomed, turned view arrives as the mapped document points, Began/Moved/Ended.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_tool_gets_press_drag_release_in_document_units() {
    todo!()
}

// A press a guide claimed never reaches press.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn an_overlay_claim_keeps_the_press_from_the_tool() {
    todo!()
}

// Esc calls cancel, gesture Cancelled, no release.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn esc_during_a_drag_cancels_the_gesture() {
    todo!()
}

// A point within 8 / zoom of a target snaps; with Ctrl it stays raw.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn ctrl_turns_snapping_off_during_a_drag() {
    todo!()
}

// scrolled_by is Some for a marquee-like tool, None when autoscrolls is false.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_drag_past_the_edge_autoscrolls_for_tools_that_ask() {
    todo!()
}

// inject Down/Move/Up gives the same calls as the OS pointer.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn injected_input_drives_the_tool_like_the_pointer() {
    todo!()
}

// ToolHost::cancel then show calls cancel once.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn cancel_from_the_app_runs_the_tools_cancel() {
    todo!()
}

// A key with a TextEdit focused never reaches key; took_key reports a taken key.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn keys_reach_the_tool_only_while_the_canvas_has_the_keyboard() {
    todo!()
}

// ToolAction::Ignored fills ignored_press.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn an_ignored_press_goes_back_to_the_app() {
    todo!()
}
