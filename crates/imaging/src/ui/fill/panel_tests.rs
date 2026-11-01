//! Tests of Content-Aware Fill's panels and workspace (task WS-CA2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Apply reports `Outcome::Apply` and the workspace stays open; OK reports `Outcome::Ok`; Cancel
// `Outcome::Cancel`.
#[test]
#[ignore = "skeleton"]
fn fill_apply_keeps_the_workspace_open_and_ok_closes() {
    todo!()
}

// The tint covers the sampling plane outside the fill area (Indicates › Excluded Area: the rest);
// ⌘H hides it without changing Show Sampling Area.
#[test]
#[ignore = "skeleton"]
fn fill_overlay_tints_the_sampling_area_or_the_excluded_area() {
    todo!()
}

// The Preview panel keeps its own view, opens framed on the sampling window, and shows the
// progress and the failure text.
#[test]
#[ignore = "skeleton"]
fn fill_preview_panel_has_its_own_view_framed_on_the_sampling_window() {
    todo!()
}

// The options bar spans the window above the strip; the footer has Reset, OK, Apply, Cancel in
// the platform's order.
#[test]
#[ignore = "skeleton"]
fn fill_bar_spans_the_window_and_footer_follows_the_platform_order() {
    todo!()
}
