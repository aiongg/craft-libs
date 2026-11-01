//! `FullWindowCanvas` in a `FullWindowDialog` harness.

#![expect(clippy::todo, reason = "skeleton: K16 fills these tests")]

// The wheel zooms about the pointer by the user's `NavigationOptions` (Alt + wheel by default,
// the wheel with Zoom with Scroll Wheel), not `exp(scroll / 200)`.
#[test]
#[ignore = "skeleton"]
fn wheel_zooms_about_the_pointer_by_navigation_options() {
    todo!()
}

// Space held and the middle button pan whatever the tool; the Hand tool pans.
#[test]
#[ignore = "skeleton"]
fn space_and_middle_button_pan_and_hand_tool_pans() {
    todo!()
}

// ⌘+, ⌘−, ⌘0 and ⌘1 change the workspace's view and never reach a canvas drawn behind the
// dialog; a focused text field keeps them.
#[test]
#[ignore = "skeleton"]
fn zoom_keys_act_on_the_workspace_view_not_the_canvas_behind() {
    todo!()
}

// The Zoom tool: a click zooms in about the point, Alt + click out, a drag zooms to the rect.
#[test]
#[ignore = "skeleton"]
fn zoom_tool_click_alt_click_and_marquee() {
    todo!()
}

// An overlay at texel 4 covers the same screen rect as the image at every zoom, tinted.
#[test]
#[ignore = "skeleton"]
fn overlay_source_draws_at_its_texel_scale() {
    todo!()
}

// The canvas is one Tab stop, a Canvas named "Image" with the zoom as its value.
#[test]
#[ignore = "skeleton"]
fn canvas_is_one_tab_stop_named_image_with_the_zoom() {
    todo!()
}
