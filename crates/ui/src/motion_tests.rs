//! Caret and outline motion.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `ease`: 0 at 0, 1 at 1, above 1/3 at 0.07 and about 0.9 at 0.5; a non-finite input is 1.
#[test]
#[ignore = "skeleton"]
fn the_glide_curve_is_words() {
    todo!()
}

// `blink`: 1 for the first 0.53 s, then fades off over the fade time, off, and fades back on.
#[test]
#[ignore = "skeleton"]
fn a_resting_caret_blinks_with_fades() {
    todo!()
}

// `CaretGlide::frame`: a changed cause with `keys` glides from the old place (half way after a short time); with `pointer` it is at the target at once.
#[test]
#[ignore = "skeleton"]
fn a_key_move_glides_and_a_click_jumps() {
    todo!()
}

// `CaretGlide::frame`: a new `view`, or a `frame_nr` two ahead, puts the caret at its target.
#[test]
#[ignore = "skeleton"]
fn a_view_change_or_a_skipped_frame_never_glides() {
    todo!()
}

// `CaretGlide::with_secs(0.0)`: the caret is at its target, opacity 0 or 1.
#[test]
#[ignore = "skeleton"]
fn zero_seconds_jumps_and_blinks_hard() {
    todo!()
}

// `OutlineGlide::frame`: a new selection glides; the same selection moved on screen is drawn on its cells.
#[test]
#[ignore = "skeleton"]
fn the_outline_glides_edge_by_edge_and_stays_on_scrolled_cells() {
    todo!()
}

// `OutlineGlide::frame`: another `place`, or `animate` false, puts the outline on its cells.
#[test]
#[ignore = "skeleton"]
fn no_glide_between_places_or_with_animate_off() {
    todo!()
}
