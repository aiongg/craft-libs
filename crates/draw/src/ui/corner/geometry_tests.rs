//! Tests of the corner widgets' geometry (task PE-A5).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A 100 × 60 rectangle's corners at radius 0 sit 10 points inside along each axis at zoom 1 and
// 2; at radius 20 at the arc centre; a regular hexagon's corner on its bisector.
#[test]
#[ignore = "skeleton"]
fn corner_widget_sits_at_the_arc_centre_and_inside_by_the_inset() {
    todo!()
}

// A side under `corner.min_side` screen points hides the corner's widget, and zooming in shows it;
// an inner angle over `hide_above` hides it.
#[test]
#[ignore = "skeleton"]
fn corner_widgets_hide_on_short_sides_and_wide_angles() {
    todo!()
}

// A drag far past the limit gives each target's own maximum: `min(w, h) / 2` for rectangles, the
// tightest fillet for a triangle.
#[test]
#[ignore = "skeleton"]
fn corner_drag_caps_the_radius_at_the_fillet_limit() {
    todo!()
}

// `dragged_radius` projects the move on the step and never goes below 0; `drag_edit` with
// `one_corner` changes one corner of a rectangle and every corner of a polygon.
#[test]
#[ignore = "skeleton"]
fn corner_drag_sets_every_target_and_alt_sets_one_corner() {
    todo!()
}

// The nearest widget within 7 points wins; of two at the same distance the first target's.
#[test]
#[ignore = "skeleton"]
fn corner_hit_takes_the_nearest_widget_topmost_first() {
    todo!()
}
