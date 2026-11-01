//! Tests of the Ruler tool on the canvas tool host (task TO-R2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Through `ToolHost`: a press within 7 points of the end drags it; Alt at the far end swaps the ends and starts the protractor at the vertex; elsewhere a new line starts; the events are Begin, Update, Commit.
#[test]
#[ignore = "skeleton"]
fn a_press_near_an_end_grabs_it_and_alt_starts_the_protractor() {
    todo!()
}

// Shift keeps a dragged end on 45° rays about the other end, and the protractor's end about the vertex.
#[test]
#[ignore = "skeleton"]
fn shift_constrains_to_45_degrees_about_the_fixed_end() {
    todo!()
}

// A new line under 0.5 document units commits `None`; a short drag of an existing end does not clear.
#[test]
#[ignore = "skeleton"]
fn a_tiny_new_line_is_cleared_on_release() {
    todo!()
}

// Page Down picks Start, End, Protractor; arrows move the picked point 1 unit, Shift 10, as `Set`; Delete gives `Set(None)`.
#[test]
#[ignore = "skeleton"]
fn keys_move_the_picked_part_and_delete_clears() {
    todo!()
}

// Esc during a drag gives Cancel and the app's line stays the press's.
#[test]
#[ignore = "skeleton"]
fn esc_cancels_a_drag() {
    todo!()
}
