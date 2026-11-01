//! Tests of shape outlines and drags (task SH-T1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `box_drag`: ⇧ gives both sides the larger extent with their signs kept; ⌥ doubles the box about
// the press; the top left and size are whole pixels (PhotoCraft `shapes.rs:63-72`).
#[test]
#[ignore = "skeleton"]
fn shape_drag_shift_makes_square_alt_draws_from_centre() {
    todo!()
}

// `line_drag` with ⇧ turns (10, 3) to the 0° step and (7, 6) to 45°, each keeping its length.
#[test]
#[ignore = "skeleton"]
fn line_drag_shift_snaps_to_45_degrees_and_keeps_length() {
    todo!()
}

// `box_drag` of 0.4 px wide and `line_drag` of 0.9 px long return `None`.
#[test]
#[ignore = "skeleton"]
fn shape_drag_under_one_pixel_draws_nothing() {
    todo!()
}

// `Polygon { sides: 3 }` has three corners with the base on the rect's bottom edge; a Rect with
// four equal radii is kurbo's rounded rect of that radius.
#[test]
#[ignore = "skeleton"]
fn triangle_is_three_sided_polygon_and_rounded_rectangle_has_four_equal_radii() {
    todo!()
}

// A Rect with radii [1, 2, 3, 4] rounds top-left 1, top-right 2, bottom-right 3, bottom-left 4.
#[test]
#[ignore = "skeleton"]
fn shape_geometry_to_path_matches_corner_order_tl_tr_br_bl() {
    todo!()
}

// `RadialFrame::drag` follows the pointer's direction; with `upright` the rotation is 0.
// `ratio_holding` keeps the inner radius; `straight_shoulders(5)` is cos 72° / cos 36° and
// `straight_shoulders(4)` is `None` (VectorCraft `straight_ratio`).
#[test]
#[ignore = "skeleton"]
fn radial_frame_rotation_inner_radius_and_straight_shoulders() {
    todo!()
}
