//! Tests of the Shape tools on the tool host (task SH-T3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A drag of 0.4 px reports `Commit { changed: false }` after `Begin` (past the slop at zoom 8)
// and no shape.
#[test]
#[ignore = "skeleton"]
fn shape_drag_under_one_pixel_commits_nothing() {
    todo!()
}

// At zoom 0.5 a 5 px move is a click and at zoom 4 a 1 px move is a drag: the slop is 3 screen
// points over the zoom (PhotoCraft `click_slop_scales_with_zoom`, `dialogs/shape_dialog/tests.rs`).
#[test]
#[ignore = "skeleton"]
fn shape_drag_click_reports_click_scaled_with_zoom() {
    todo!()
}

// Space held mid-drag moves the box by the pointer's move and keeps its size; released, the drag
// sizes again from the moved press.
#[test]
#[ignore = "skeleton"]
fn space_moves_the_shape_being_drawn() {
    todo!()
}

// Illustrator rules: ↑ during a Polygon drag adds a side up to 100, ↓ removes one down to 3; the
// next `Update` carries the count; Photoshop rules leave the arrows alone.
#[test]
#[ignore = "skeleton"]
fn radial_polygon_arrow_keys_change_sides_between_3_and_100() {
    todo!()
}

// Illustrator rules: with Ctrl/Cmd held a Star's inner radius stays while the outer follows; ⌥
// with five points gives the straight-shoulder ratio.
#[test]
#[ignore = "skeleton"]
fn star_ctrl_holds_inner_radius_while_outer_follows() {
    todo!()
}

// Enter with the canvas focused and no drag reports `Click` at the view centre.
#[test]
#[ignore = "skeleton"]
fn enter_with_canvas_focus_opens_create_shape_at_view_centre() {
    todo!()
}

// Esc mid-drag and `ToolHost::cancel` both report `Cancel`; a press inside the slop then Esc
// reports nothing.
#[test]
#[ignore = "skeleton"]
fn shape_drag_cancels_on_escape_and_on_tool_switch() {
    todo!()
}
