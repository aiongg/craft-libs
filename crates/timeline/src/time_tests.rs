//! Tick, TimeRange and TimeView.

use crate::time::{TICKS_PER_SECOND, Tick, TimeRange, TimeView, VIEW_EXTENT_PAST_END};

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

