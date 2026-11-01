//! Tests of the pins' model and grabs (task WS-BP1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A new set has one pin in the middle, selected; adding takes the selected pin's amount and shape;
// moves stay inside the bounds; removing keeps the last pin; 64 pins at most.
#[test]
#[ignore = "skeleton"]
fn blur_pins_add_move_tune_remove_and_stay_in_bounds() {
    todo!()
}

// Iris handles: an axis handle sets its radius, the square the roundness, a feather dot the
// feather; `clean` puts each back in range.
#[test]
#[ignore = "skeleton"]
fn blur_pins_iris_handles_resize_turn_round_and_feather() {
    todo!()
}

// Tilt-Shift: the solid lines set the focus, the dashed the transition, the dots the angle (folded
// into −90..=90); a click adds no pin.
#[test]
#[ignore = "skeleton"]
fn blur_pins_tilt_shift_lines_and_dots() {
    todo!()
}

// A whole turn of the ring sweeps the full range (Spin: 360°), clamped at the ends.
#[test]
#[ignore = "skeleton"]
fn blur_pins_spin_ring_sets_the_angle() {
    todo!()
}

// A new pin copies the selected pin's amount and shape.
#[test]
#[ignore = "skeleton"]
fn blur_pins_new_pin_takes_the_selected_pins_amount_and_shape() {
    todo!()
}

// The cursor over each grab: Alias, Move, ResizeVertical, Grab, Crosshair (Default for Tilt-Shift).
#[test]
#[ignore = "skeleton"]
fn blur_pins_cursors_follow_the_grab() {
    todo!()
}
