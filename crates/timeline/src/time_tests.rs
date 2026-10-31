//! Tick, TimeRange and TimeView.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Tick::from_units(48_000, 48_000) is one second; samples at 44.1 k round-trip through to_units_floor.
#[test]
#[ignore = "skeleton"]
fn seconds_and_units_convert_exactly() {
    todo!()
}

// from_rational / to_rational_floor / to_rational_round agree with FilmCraft's cases, a zero den gives zero.
#[test]
#[ignore = "skeleton"]
fn rational_timestamps_round_and_floor() {
    todo!()
}

// Tick::MAX + Tick::MAX and Tick(i64::MIN) - Tick(1) saturate.
#[test]
#[ignore = "skeleton"]
fn arithmetic_saturates() {
    todo!()
}

// zoom_about keeps the anchor's x, clamps the zoom, falls back to the middle outside the view.
#[test]
#[ignore = "skeleton"]
fn zoom_about_keeps_the_anchor_under_the_pointer() {
    todo!()
}

// scroll_to_reveal returns None in view and 5 % of the width otherwise.
#[test]
#[ignore = "skeleton"]
fn reveal_puts_the_playhead_five_percent_from_the_left() {
    todo!()
}

// max_start is end + 600 s less the view span.
#[test]
#[ignore = "skeleton"]
fn max_start_reaches_ten_minutes_past_the_end() {
    todo!()
}

// checked clamps zoom and start and refuses NaN.
#[test]
#[ignore = "skeleton"]
fn checked_views_are_clamped_or_refused() {
    todo!()
}
