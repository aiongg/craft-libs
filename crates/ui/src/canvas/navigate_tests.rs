//! Wheel and pinch, page keys, flick pan, touchpad glide, autoscroll, Rotate View.

// Alt + one notch is Zoom(1.1); no modifier is Pan.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn alt_wheel_zooms_and_the_plain_wheel_scrolls() {
    todo!()
}

// With the preference the wheel zooms and Alt + wheel pans.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn zoom_with_scroll_wheel_swaps_wheel_and_alt_wheel() {
    todo!()
}

// Shift + Alt + one notch is SnapZoom(1); ten notches cap at 10.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn shift_alt_wheel_steps_the_ladder() {
    todo!()
}

// zoom_delta 1.2 is Zoom(1.2) with the preference, ignored without.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_pinch_zooms_only_with_zoom_with_trackpad_pinch() {
    todo!()
}

// At 12800 % a zoom-in notch changes neither zoom nor centre.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_notch_past_the_zoom_limit_leaves_the_view_alone() {
    todo!()
}

// Page Down scrolls the canvas height, Cmd sideways, Shift 10 points; Alt and Shift+Home are None.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn page_keys_scroll_a_canvas_and_shift_ten_points() {
    todo!()
}

// Home puts the top-left corner at the canvas's top-left; End the bottom-right.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn home_and_end_show_the_document_corners() {
    todo!()
}

// Zero inside; 120 pt/s at the edge plus 12 per point past it, capped at 2400; dt capped at 0.05.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn autoscroll_speeds_up_past_the_edge() {
    todo!()
}

// An image that fits the canvas never scrolls; one already past a limit is not pulled back.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn autoscroll_stops_at_the_scroll_limits() {
    todo!()
}

// A fast release keeps panning with falling speed; with the preference off it stops.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_released_hand_drag_glides_and_slows_to_a_stop() {
    todo!()
}

// A point-unit swipe ending with TouchPhase::End glides; a notched wheel never does; two still passes stop it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_touchpad_glide_starts_only_after_a_lift() {
    todo!()
}

// Dragging a quarter turn about the centre adds 90°; mirrored it subtracts; Shift snaps to 15°.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_rotate_drag_follows_the_pointer_and_snaps_with_shift() {
    todo!()
}

// 190 wraps to -170, -180 to 180, NaN to 0.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn angles_wrap_into_a_half_turn_each_way() {
    todo!()
}

// No compass at 0°; at 30° a click on it gives reset.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_compass_shows_when_turned_and_a_click_resets() {
    todo!()
}
