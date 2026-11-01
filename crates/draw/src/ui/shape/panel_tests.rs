//! Tests of the shape panel (task SH-T6).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Scrubbing W reports `Begin`, `Update`s and one `Commit` of `ShapeChange::Size`.
#[test]
#[ignore = "skeleton"]
fn shape_panel_scrub_is_one_undo_gesture() {
    todo!()
}

// ↓ on a stroke width of 0 reports nothing; turning the stroke chip from No Color to Solid at
// width 0 reports `StrokeWidth(3)` with the paint.
#[test]
#[ignore = "skeleton"]
fn shape_panel_stroke_width_zero_does_not_add_a_stroke_and_on_gives_three_px() {
    todo!()
}

// A locked appearance disables every field and chip (AccessKit `disabled`).
#[test]
#[ignore = "skeleton"]
fn shape_panel_disables_for_locked_layer() {
    todo!()
}

// A rect shows "Corner radius" then Top Left, Top Right, Bottom Left, Bottom Right; editing Bottom
// Left reports `Radii` with index 3 changed.
#[test]
#[ignore = "skeleton"]
fn shape_panel_rect_shows_uniform_radius_and_four_corners() {
    todo!()
}

// With `stroke_open` the Stroke style button shows `ShapeAppearance::stroke_style`, is disabled
// at width 0, and a pick in its popover (Dashed on) reports `ShapeChange::StrokeStyle`.
#[test]
#[ignore = "skeleton"]
fn shape_panel_stroke_style_button_reports_the_style() {
    todo!()
}
