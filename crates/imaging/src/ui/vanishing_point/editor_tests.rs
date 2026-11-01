//! Tests of Vanishing Point's planes and editor (task WS-VP1), on a `ToolHost` with a fake scene.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Four clicks in any order make one plane (normalized), selected, and the tool turns to Edit Plane.
#[test]
#[ignore = "skeleton"]
fn vp_four_clicks_make_a_plane_in_any_order() {
    todo!()
}

// Dragging an edge node slides the edge along the plane's perspective at the press.
#[test]
#[ignore = "skeleton"]
fn vp_edge_node_slides_along_the_planes_perspective() {
    todo!()
}

// ⌘-drag on an edge node tears off a plane at 90° and the drag sets its depth; with Alt the far
// edge sets its angle.
#[test]
#[ignore = "skeleton"]
fn vp_ctrl_drag_tears_off_a_plane_and_far_edge_sets_depth_or_angle() {
    todo!()
}

// Delete removes the selected plane, torn children become drawn planes; Esc drops pending corners
// before it cancels.
#[test]
#[ignore = "skeleton"]
fn vp_delete_removes_the_plane_and_esc_drops_a_half_plane_first() {
    todo!()
}

// Alt-click sets the source; strokes report Begin, Update, Commit with the options; Aligned keeps
// the first stroke's offset.
#[test]
#[ignore = "skeleton"]
fn vp_stamp_alt_click_sets_source_and_strokes_report_events() {
    todo!()
}

// Undo steps back through a stroke, a tear-off and a plane, in that order.
#[test]
#[ignore = "skeleton"]
fn vp_undo_steps_back_through_planes_paste_and_strokes() {
    todo!()
}

// At 64 planes Create Plane and tear-off add nothing.
#[test]
#[ignore = "skeleton"]
fn vp_planes_are_capped_at_sixty_four() {
    todo!()
}
