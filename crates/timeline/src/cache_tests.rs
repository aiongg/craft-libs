//! The frame cache.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// past the budget, the oldest shown frames go down to 80 %.
#[test]
#[ignore = "skeleton"]
fn evicts_to_eighty_percent_least_recently_shown_first() {
    todo!()
}

// with a playhead, unreached frames go first, then the one reached last.
#[test]
#[ignore = "skeleton"]
fn spares_the_frames_the_preview_reaches_next() {
    todo!()
}

// a frame at another scale than the on-screen one is the first victim.
#[test]
#[ignore = "skeleton"]
fn other_scales_go_first() {
    todo!()
}

// nearest finds the exact frame, else the nearest earlier one within max_back.
#[test]
#[ignore = "skeleton"]
fn nearest_holds_an_earlier_frame() {
    todo!()
}

// FramePlayhead::ahead counts past the end and around from the start when looping.
#[test]
#[ignore = "skeleton"]
fn ahead_counts_through_a_loop() {
    todo!()
}
