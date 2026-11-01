//! Tests of the anchor tools (task PE-B1) and the Curvature tool (task PE-C1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// PE-B1. Add Anchor: a click on a segment reports `insert` at the hit's t; a click on an anchor
// is claimed and reports nothing; with ⌥ the click on an anchor reports `delete`.
#[test]
#[ignore = "skeleton"]
fn add_anchor_tool_reports_insert_on_a_segment_click_and_alt_swaps() {
    todo!()
}

// PE-B1. Delete Anchor: a click on an anchor reports `delete` of that anchor; ⌥ on a segment
// reports `insert`.
#[test]
#[ignore = "skeleton"]
fn delete_anchor_tool_reports_delete_on_an_anchor_click() {
    todo!()
}

// PE-B1. Auto Add/Delete: the first and last anchors of an open subpath and empty canvas are not
// claimed (the pen draft takes them); a closed subpath's every anchor is.
#[test]
#[ignore = "skeleton"]
fn auto_add_delete_leaves_open_ends_to_the_pen() {
    todo!()
}

// PE-B1. Hovering a segment with Add draws the segment mark and a hollow anchor at t; hovering an
// anchor with Delete draws the enlarged anchor.
#[test]
#[ignore = "skeleton"]
fn anchor_tools_draw_the_hover_mark_of_what_a_press_takes() {
    todo!()
}

// PE-C1. A drag from an anchor reports MoveCurvature Begin past the click slop, Update, Commit;
// Esc cancels and the rest of the press does nothing.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_drags_an_anchor_and_reflows() {
    todo!()
}

// PE-C1. A drag from a segment is one InsertCurvature gesture with t clamped to 0.01..=0.99.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_insert_and_drag_is_one_gesture() {
    todo!()
}

// PE-C1. A click without drag on a segment reports one Set of InsertCurvature with `by` zero.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_click_on_a_segment_inserts_only() {
    todo!()
}

// PE-C1. The second click of a double-click on an anchor reports a ToggleCurvature Set.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_double_click_toggles_an_anchor() {
    todo!()
}

// PE-C1. A press on empty canvas is not claimed, so the pen draft shown after starts a path.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_does_not_claim_empty_canvas() {
    todo!()
}

// PE-C1. With focus, the arrows report a MoveCurvature Set of the selected anchors (⇧ 10) and
// Enter a ToggleCurvature Set of the picked anchor.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_keys_move_and_toggle() {
    todo!()
}
