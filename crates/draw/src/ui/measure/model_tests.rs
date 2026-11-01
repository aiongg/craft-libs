//! Tests of the Ruler line model (task TO-R1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// X, Y, W, H, A and L1 of a line, and A (0 to 180) and L2 with a protractor, equal PhotoCraft's `ruler_info` cases (y up for the angle, -180 to 180).
#[test]
#[ignore = "skeleton"]
fn reading_matches_the_engines_ruler_info() {
    todo!()
}

// `RulerReading::scaled` multiplies X, Y, W, H, L1 and L2 and keeps A.
#[test]
#[ignore = "skeleton"]
fn scaled_multiplies_lengths_and_positions_not_the_angle() {
    todo!()
}

// `part_at` within the reach prefers the protractor's end, then the end, then the start; outside the reach it is `None`.
#[test]
#[ignore = "skeleton"]
fn a_press_finds_the_protractor_then_the_end_then_the_start() {
    todo!()
}
