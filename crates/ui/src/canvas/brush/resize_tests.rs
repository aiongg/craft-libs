//! Tests of the brush resize gesture and the tip falloff.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Alt with the secondary and Ctrl + Alt with the primary are chords; Alt with the primary alone is not.
#[test]
#[ignore = "skeleton"]
fn chords_are_alt_secondary_and_ctrl_alt_primary() {
    todo!()
}

// PhotoCraft's `resized` maths: `start + 2 × dx` rounded and clamped to 1..=max; the level clamped and in whole percent.
#[test]
#[ignore = "skeleton"]
fn right_grows_and_down_hardens() {
    todo!()
}

// Right stays bigger and down harder in a rotated or flipped view (PhotoCraft's `a_rotated_view_keeps_horizontal_for_size_and_vertical_for_hardness`).
#[test]
#[ignore = "skeleton"]
fn directions_are_screen_relative_in_a_turned_or_flipped_view() {
    todo!()
}

// With `BrushLevel::Opacity` the vertical travel changes the level and the readout says Opacity.
#[test]
#[ignore = "skeleton"]
fn vary_hardness_off_moves_opacity() {
    todo!()
}

// Begin, Update per change and Commit share one id; an unmoved click commits with `changed` false.
#[test]
#[ignore = "skeleton"]
fn one_gesture_is_begin_update_commit() {
    todo!()
}

// `cancel` returns `Cancel`; a `GestureHost` fed the events is back at the start values.
#[test]
#[ignore = "skeleton"]
fn escape_restores_the_start_values() {
    todo!()
}

// `begin` while active drops the old run; the new run starts from the new `start` with a new id.
#[test]
#[ignore = "skeleton"]
fn a_new_press_ends_a_resize_whose_release_was_lost() {
    todo!()
}

// The preview mesh's first ring has the full tint and its last is transparent (PhotoCraft's `the_tip_preview_follows_the_brush_falloff`).
#[test]
#[ignore = "skeleton"]
fn falloff_mesh_is_solid_to_the_core_and_clear_at_the_reach() {
    todo!()
}

// The readout rows are Diameter and the level's label, with whole pixels and a whole percentage.
#[test]
#[ignore = "skeleton"]
fn readout_lists_diameter_and_the_level() {
    todo!()
}
