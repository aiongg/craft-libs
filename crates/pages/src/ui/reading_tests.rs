//! Reading view geometry and turning.

// A 1200 pt wide view gives four columns of at least 192 pt; a requested 612 pt column in a 500 pt view shrinks to the window less two gaps.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn reading_size_fits_whole_columns() {
    todo!()
}

// One line notch turns once (three lines on the web); 30 then 30 touchpad points turn once and reset; Ctrl+wheel turns nothing.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn wheel_notches_and_touchpads_turn_once() {
    todo!()
}

// Steps toward 300 from 0 never overshoot and end exactly on 300.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn animation_settles_on_the_boundary() {
    todo!()
}
