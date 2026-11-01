//! Frame rates, sample rates and pixel aspect ratios.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// frame_at(tick_of(f)) == f for COMMON rates and 37.516 fps.
#[test]
#[ignore = "skeleton"]
fn frame_boundaries_round_trip_at_every_rate() {
    todo!()
}

// snap and snap_nearest at both sides of a frame's middle.
#[test]
#[ignore = "skeleton"]
fn snap_nearest_picks_the_nearer_boundary() {
    todo!()
}

// 29.97 has base 30 and supports drop-frame, 23.976 base 24 does not.
#[test]
#[ignore = "skeleton"]
fn timecode_base_and_drop_frame_support() {
    todo!()
}

// label gives 24, 23.976, 29.97, 59.94.
#[test]
#[ignore = "skeleton"]
fn labels_trim_trailing_zeros() {
    todo!()
}

// a zero or negative rate frames like 29.97.
#[test]
#[ignore = "skeleton"]
fn insane_rates_count_as_the_default() {
    todo!()
}
