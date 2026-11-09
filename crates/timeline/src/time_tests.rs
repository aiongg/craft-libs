//! Tick, TimeRange and TimeView.

use crate::time::{TICKS_PER_SECOND, Tick, TimeRange, TimeView};

fn secs(s: i64) -> Tick {
    Tick(s * TICKS_PER_SECOND)
}

#[test]
fn seconds_and_units_convert_exactly() {
    assert_eq!(Tick::from_units(48_000, 48_000), secs(1));
    assert_eq!(Tick::from_units(5, 0), Tick::ZERO);
    for n in [0, 1, 44_099, 44_100, 1_234_567, -7] {
        assert_eq!(Tick::from_units(n, 44_100).to_units_floor(44_100), n);
    }
    // Floor, also below zero.
    assert_eq!(Tick(-1).to_units_floor(48_000), -1);
    assert_eq!(Tick::from_seconds_f64(1.5), Tick(TICKS_PER_SECOND * 3 / 2));
    assert_eq!(Tick::from_seconds_f64(f64::NAN), Tick::ZERO);
}

#[test]
fn rational_timestamps_round_and_floor() {
    // 1500 ms in a 1/1000 timebase.
    assert_eq!(Tick::from_rational(1500, 1, 1000), Tick(TICKS_PER_SECOND * 3 / 2));
    assert_eq!(Tick::from_rational(1, 1, 0), Tick::ZERO);
    assert_eq!(Tick::from_rational(i64::MAX, i64::MAX, 1), Tick(i64::MAX));
    assert_eq!(Tick::from_rational(i64::MIN, i64::MAX, 1), Tick(i64::MIN));
    let half_ms = Tick(TICKS_PER_SECOND / 2000);
    let t = Tick::from_rational(10, 1, 1000) + half_ms;
    assert_eq!(t.to_rational_floor(1, 1000), 10);
    assert_eq!(t.to_rational_round(1, 1000), 11);
    assert_eq!((t - Tick(1)).to_rational_round(1, 1000), 10);
    assert_eq!(t.to_rational_floor(0, 1000), 0);
    assert_eq!(t.to_rational_round(0, 1000), 0);
}

#[test]
fn arithmetic_saturates() {
    assert_eq!(Tick(i64::MAX) + Tick(1), Tick(i64::MAX));
    assert_eq!(Tick(i64::MIN) - Tick(1), Tick(i64::MIN));
    assert_eq!(-Tick(i64::MIN), Tick(i64::MAX));
    assert_eq!(Tick::MAX + Tick::MAX, Tick(i64::MAX / 4 * 2));
    assert_eq!(Tick(i64::MAX) + Tick::MAX, Tick(i64::MAX));
    assert_eq!(Tick(10).mul_ratio(1, 0), Tick::ZERO);
    assert_eq!(Tick(-3).mul_ratio(1, 2), Tick(-2));
    assert_eq!(Tick(5).clamp(Tick(10), Tick(2)), Tick(2));
}

#[test]
fn ranges_are_half_open() {
    let a = TimeRange::new(secs(0), secs(2));
    let b = TimeRange::new(secs(2), secs(2));
    assert!(!a.overlaps(&b) && a.contains(secs(1)) && !a.contains(secs(2)));
    assert_eq!(a.intersect(&TimeRange::new(secs(1), secs(5))), Some(TimeRange::from_bounds(secs(1), secs(2))));
    assert!(TimeRange::new(secs(1), Tick::ZERO).is_empty());
}

#[test]
fn zoom_about_keeps_the_anchor_under_the_pointer() {
    let mut view = TimeView::new(secs(10), 40.0);
    let anchor = secs(15);
    let x = view.x_of(anchor, 100.0);
    view.zoom_about(2.0, anchor, 1000.0);
    assert_eq!(view.points_per_second, 80.0);
    assert!((view.x_of(anchor, 100.0) - x).abs() < 0.01);
    assert_eq!(view.time_at(x, 100.0), anchor);

    // The zoom stops at its limits.
    view.zoom_about(1e9, anchor, 1000.0);
    assert_eq!(view.points_per_second, TimeView::MAX_ZOOM);
    view.zoom_about(1e-12, anchor, 1000.0);
    assert_eq!(view.points_per_second, TimeView::MIN_ZOOM);

    // An anchor outside the view zooms about the middle; the start never goes below zero.
    let mut view = TimeView::new(secs(10), 40.0);
    view.zoom_about(2.0, secs(500), 1000.0);
    assert!((view.x_of(secs(500), 0.0) - 500.0).abs() < 0.01);
    let mut view = TimeView::new(Tick::ZERO, 40.0);
    view.zoom_about(0.5, secs(1), 1000.0);
    assert!(view.start >= Tick::ZERO);
}

#[test]
fn reveal_puts_the_playhead_five_percent_from_the_left() {
    let view = TimeView::new(secs(10), 10.0);
    assert_eq!(view.scroll_to_reveal(secs(20), 1000.0), None);
    // 5 % of 1000 points at 10 points a second is 5 s.
    assert_eq!(view.scroll_to_reveal(secs(200), 1000.0), Some(secs(195)));
    assert_eq!(view.scroll_to_reveal(secs(2), 1000.0), Some(Tick::ZERO));
    assert_eq!(view.scroll_to_reveal(secs(200), f32::NAN), None);
    assert_eq!(TimeView::new(Tick::ZERO, 0.0).scroll_to_reveal(secs(200), 1000.0), None);
}

#[test]
fn fit_shows_the_range_with_a_margin() {
    let view = TimeView::fit(TimeRange::new(secs(5), secs(10)), 110.0, 10.0);
    assert_eq!(view.start, secs(5));
    assert_eq!(view.points_per_second, 10.0);
}

#[test]
fn checked_views_are_clamped_or_refused() {
    let view = TimeView::new(secs(-5), 1e9).checked().expect("finite");
    assert_eq!(view.start, Tick::ZERO);
    assert_eq!(view.points_per_second, 100_000.0);
    assert_eq!(TimeView::new(Tick::MAX, 0.0).checked().map(|v| v.points_per_second), Some(0.001));
    assert_eq!(TimeView::new(secs(20_000_000), 1.0).checked().map(|v| v.start), Some(secs(10_000_000)));
    assert_eq!(TimeView::new(Tick::ZERO, f64::NAN).checked(), None);

    // The scroll start stops past the end of the sequence, and not at all when the view spans more than that.
    let view = TimeView::new(Tick::ZERO, 10.0);
    assert!(view.max_start(secs(6), 100.0) > secs(6), "the view scrolls past the end");
    assert_eq!(view.max_start(secs(6), 1e9), Tick::ZERO);
    assert_eq!(TimeView::new(Tick::ZERO, 0.0).max_start(secs(6), 100.0), Tick::ZERO);
}
