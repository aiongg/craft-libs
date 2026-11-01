//! Tests of the Connector tool (task SH-T10).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Hovering a target paints its four side-middle sites; the site within reach is the active
// (larger) one.
#[test]
#[ignore = "skeleton"]
fn connector_hover_shows_attach_points_and_active_one_is_larger() {
    todo!()
}

// A 3-point drag makes nothing; a drag from a box back onto it commits `changed: false`; a drag to
// another box commits a `ConnectorDraft` glued at both ends.
#[test]
#[ignore = "skeleton"]
fn connector_drag_under_four_points_or_onto_start_makes_nothing() {
    todo!()
}

// Dragging the selected connector's end onto a target reports `Reattach` glued there; onto empty
// canvas, free at the point.
#[test]
#[ignore = "skeleton"]
fn connector_reattach_to_target_or_free_point() {
    todo!()
}

// A target with no sites glues at side middles (FigCraft); one with numbered sites glues to the
// nearest site's index (DeckCraft).
#[test]
#[ignore = "skeleton"]
fn connector_sites_accept_side_middles_and_numbered_sites() {
    todo!()
}

// Dragging an elbow's middle path handle reports `Reshape` with `ConnectorPath::Runs`.
#[test]
#[ignore = "skeleton"]
fn connector_path_handle_reshapes() {
    todo!()
}

// Esc and `ToolHost::cancel` mid-drag report `Cancel` for `Draw`, `Reattach` and `Reshape`.
#[test]
#[ignore = "skeleton"]
fn connector_cancels_on_escape_and_on_tool_switch() {
    todo!()
}
