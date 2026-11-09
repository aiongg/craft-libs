//! Frame rates, sample rates and pixel aspect ratios.

use crate::rates::FrameRate;
use crate::time::{TICKS_PER_SECOND, Tick};

#[test]
fn frame_boundaries_round_trip_and_snap_to_the_nearer() {
    for rate in FrameRate::COMMON.into_iter().chain([FrameRate::new(37_516, 1000)]) {
        for f in [-1000, -1, 0, 1, 2, 29, 1799, 1_000_003] {
            assert_eq!(rate.frame_at(rate.tick_of(f)), f, "{rate:?} frame {f}");
            assert_eq!(rate.frame_at(rate.tick_of(f) - Tick(1)), f - 1, "{rate:?} before frame {f}");
        }
    }
    assert_eq!(FrameRate::FPS_25.frame_duration(), Tick(TICKS_PER_SECOND / 25));
    assert_eq!(FrameRate::new(48, 2), FrameRate::new(24, 1));

    let rate = FrameRate::FPS_25;
    let d = rate.frame_duration().0;
    let start = rate.tick_of(10);
    assert_eq!(rate.snap(start + Tick(d - 1)), start);
    assert_eq!(rate.snap_nearest(start + Tick(d / 2 - 1)), start);
    // The exact middle goes to the earlier boundary.
    assert_eq!(rate.snap_nearest(start + Tick(d / 2)), start);
    assert_eq!(rate.snap_nearest(start + Tick(d / 2 + 1)), rate.tick_of(11));
}

#[test]
fn drop_frame_is_for_ntsc_rates_only() {
    assert!(FrameRate::FPS_29_97.supports_drop_frame(), "NTSC rates support drop-frame");
    assert!(!FrameRate::FPS_30.supports_drop_frame(), "whole rates do not");
}

#[test]
fn frame_rate_from_f64_snaps_or_falls_back() {
    assert_eq!(FrameRate::from_f64(29.97), FrameRate::FPS_29_97);
    assert_eq!(FrameRate::from_f64(12.5), FrameRate::new(25, 2));

    let default = FrameRate::default();
    for bad in [FrameRate::new(0, 1), FrameRate::new(30, 0), FrameRate::new(-30, 1)] {
        assert_eq!(bad.frame_duration(), default.frame_duration());
        assert_eq!(bad.frame_at(Tick(TICKS_PER_SECOND)), default.frame_at(Tick(TICKS_PER_SECOND)));
        assert_eq!(bad.tick_of(7), default.tick_of(7));
        assert_eq!(bad.timecode_base(), 24);
    }
}
