//! Tests of Select and Mask's tools (task WS-SM2), on a `ToolHost` with injected input.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click (or a drag under 4 screen points) gives `ObjectTarget::Point`; a longer drag a rect.
#[test]
#[ignore = "skeleton"]
fn select_mask_object_click_and_drag_make_point_or_rect() {
    todo!()
}

// Polygonal Lasso: clicks add vertices; Backspace removes the last; Enter closes (3 or more) into
// a Lasso edit; Esc drops the polygon; a click on the first vertex within 6 points closes.
#[test]
#[ignore = "skeleton"]
fn select_mask_polygonal_lasso_enter_escape_backspace() {
    todo!()
}

// With Real-Time Refinement on, a stroke reports Updates; off, only Begin and Commit.
#[test]
#[ignore = "skeleton"]
fn select_mask_real_time_refinement_reports_updates_only_when_on() {
    todo!()
}

// [ and ] change the current brush tool's own size by at least a pixel within 1..=5000; Alt at
// the press reverses the bar's mode.
#[test]
#[ignore = "skeleton"]
fn select_mask_brush_sizes_are_per_tool_and_alt_reverses_the_mode() {
    todo!()
}
