//! The Navigator on screen (egui_kittest): pointer, wheel, keys, the zoom row and AccessKit.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on the preview sets the view centre to the point, clamped to the document.
#[test]
#[ignore = "skeleton"]
fn navigator_click_centres_the_view_on_the_point() {
    todo!()
}

// A drag moves the centre each pass and reports `view_changed`; Esc puts the press view back.
#[test]
#[ignore = "skeleton"]
fn navigator_drag_follows_and_esc_restores() {
    todo!()
}

// The proxy is drawn from the view passed in (the animated one), size and centre as PhotoCraft's
// test.
#[test]
#[ignore = "skeleton"]
fn navigator_proxy_follows_the_animated_view() {
    todo!()
}

// With Animated Zoom off one notch over the preview is ×1.1 and leaves the centre; outside the
// preview nothing changes (PhotoCraft's `navigator_wheel_changes_zoom_only_when_over_preview`,
// panels.rs:336).
#[test]
#[ignore = "skeleton"]
fn navigator_wheel_zooms_only_over_the_preview_and_never_pans() {
    todo!()
}

// A notch past 12800 % changes nothing and settles the wheel.
#[test]
#[ignore = "skeleton"]
fn navigator_wheel_at_the_limit_settles() {
    todo!()
}

// Arrows, Shift + arrows, Page Up and Down, `+`, `-` and Home on the focused preview.
#[test]
#[ignore = "skeleton"]
fn navigator_keys_move_the_view_and_zoom() {
    todo!()
}

// Zoom Out, Zoom In and the field's Up and Down step the ladder; the buttons set `animate_zoom`.
#[test]
#[ignore = "skeleton"]
fn navigator_zoom_row_steps_the_ladder() {
    todo!()
}

// The two AccessKit scroll bar nodes carry the percent value, and Increment moves the view.
#[test]
#[ignore = "skeleton"]
fn navigator_preview_scroll_bars_expose_the_view() {
    todo!()
}
