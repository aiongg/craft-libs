//! Tests of the Create Shape dialog (task SH-T7). PhotoCraft's `dialogs/shape_dialog/tests.rs`
//! cases move here; PhotoCraft keeps its control-channel tests.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// OK draws a 100 × 100 rect with its top left at the rounded click; with From Center it is centred
// on the click.
#[test]
#[ignore = "skeleton"]
fn create_shape_ok_draws_at_the_click_and_from_center_centres_it() {
    todo!()
}

// The four radius fields are named Top Left, Top Right, Bottom Right, Bottom Left for AccessKit;
// linked, typing in one sets all four.
#[test]
#[ignore = "skeleton"]
fn create_shape_radius_fields_are_named_for_their_corners_and_link() {
    todo!()
}

// A secondary click on Width picks inches for Width only; Height stays in the ruler unit and the
// next opening shows the ruler unit again.
#[test]
#[ignore = "skeleton"]
fn create_shape_width_unit_is_per_field_and_not_remembered() {
    todo!()
}

// OK stores width, height, From Center and the star ratio under `shape.create.<kind>` and the next
// dialog of that kind opens with them; Cancel stores nothing; 0 or 300001 px shows the error and
// disables OK.
#[test]
#[ignore = "skeleton"]
fn create_shape_remembers_size_and_checks_bad_values() {
    todo!()
}
