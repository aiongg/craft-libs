//! Tests of the brush cursor and the tip pose.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// With `caps_lock_precise` on and Caps Lock down, every painting preference gives the 6 pt crosshair.
#[test]
#[ignore = "skeleton"]
fn caps_lock_gives_the_precise_crosshair() {
    todo!()
}

// Standard returns the tool icon at its centre; Precise the crosshair.
#[test]
#[ignore = "skeleton"]
fn standard_gives_the_tool_icon_and_precise_the_crosshair() {
    todo!()
}

// Show Only Crosshair While Painting swaps the outline for the crosshair while painting, not before.
#[test]
#[ignore = "skeleton"]
fn crosshair_only_while_painting() {
    todo!()
}

// A round tip gives `normal_radius` or `full_radius` times the zoom as the circle's radius.
#[test]
#[ignore = "skeleton"]
fn normal_and_full_size_pick_the_two_radii() {
    todo!()
}

// Traced rings map through the view like `to_screen` (PhotoCraft's `tip_axes_follow_the_view_like_to_screen`, `the_outline_turns_and_flips_with_the_view`).
#[test]
#[ignore = "skeleton"]
fn traced_rings_follow_zoom_rotation_flip_and_aspect() {
    todo!()
}

// The pose turns the rings before the view and the shared rings are not rebuilt (PhotoCraft's `pen_pose_rotation_precedes_the_view_and_reuses_the_tip_cache`).
#[test]
#[ignore = "skeleton"]
fn pen_pose_precedes_the_view() {
    todo!()
}

// `TipPose::pen` with a tilt shortens the axis along the tilt by its cosine (PhotoCraft's `brush_projection_shortens_the_tilt_axis`).
#[test]
#[ignore = "skeleton"]
fn projection_shortens_the_tilt_axis() {
    todo!()
}

// Unequal or skewed axes turn a round tip into an outline ring with the axes' lengths.
#[test]
#[ignore = "skeleton"]
fn round_tip_becomes_an_ellipse_under_a_skewed_pose() {
    todo!()
}

// An outline under `brush.tip_min_radius` on screen gives the 4 pt crosshair with no gap.
#[test]
#[ignore = "skeleton"]
fn tiny_tips_show_a_crosshair() {
    todo!()
}

// NaN, infinite, negative and zero sizes never panic and never give a non-finite ring.
#[test]
#[ignore = "skeleton"]
fn hostile_sizes_give_finite_cursors() {
    todo!()
}

// `inner_ring` adds a second mapped ring (LightCraft's feather core).
#[test]
#[ignore = "skeleton"]
fn inner_ring_adds_a_second_ring() {
    todo!()
}

// A Pixels tip gives `Hidden`, or the 4 pt crosshair under 6 pt or with `centre`.
#[test]
#[ignore = "skeleton"]
fn pixel_tips_hide_the_pointer_and_small_cells_show_a_crosshair() {
    todo!()
}
