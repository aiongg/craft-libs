//! Tests of the contour field (task AD-10).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Picking Gaussian in the picker reports one Set with the preset; a click on the thumbnail opens
// the editor with the contour's points (as levels) and reports Begin; OK reports Commit named
// "Custom" after an edit, Cancel reports Cancel.
#[test]
#[ignore = "skeleton"]
fn contour_field_picks_a_preset_and_opens_the_editor_with_its_points() {
    todo!()
}

// `to_curve` then `from_curve` returns the same points (within a level) and corners; a drawn
// curve gives None.
#[test]
#[ignore = "skeleton"]
fn contour_converts_to_and_from_the_editor_curve() {
    todo!()
}
