//! Frame rates, sample rates and pixel aspect ratios.

use crate::rates::{FrameRate, PixelAspect};
use crate::time::{TICKS_PER_SECOND, Tick};

#[test]
fn frame_boundaries_round_trip_at_every_rate() {
    for rate in FrameRate::COMMON.into_iter().chain([FrameRate::new(37_516, 1000)]) {
        for f in [-1000, -1, 0, 1, 2, 29, 1799, 1_000_003] {
            assert_eq!(rate.frame_at(rate.tick_of(f)), f, "{rate:?} frame {f}");
            assert_eq!(rate.frame_at(rate.tick_of(f) - Tick(1)), f - 1, "{rate:?} before frame {f}");
        }
    }
    assert_eq!(FrameRate::FPS_25.frame_duration(), Tick(TICKS_PER_SECOND / 25));
    assert_eq!(FrameRate::new(48, 2), FrameRate::new(24, 1));
}

#[test]
fn snap_nearest_picks_the_nearer_boundary() {
    let rate = FrameRate::FPS_25;
    let d = rate.frame_duration().0;
    let start = rate.tick_of(10);
    assert_eq!(rate.snap(start + Tick(d - 1)), start);
    assert_eq!(rate.snap_nearest(start + Tick(d / 2 - 1)), start);
    // The exact middle goes to the earlier boundary.
    assert_eq!(rate.snap_nearest(start + Tick(d / 2)), start);
    assert_eq!(rate.snap_nearest(start + Tick(d / 2 + 1)), rate.tick_of(11));
    assert_eq!(rate.snap_nearest(Tick::MAX).0, rate.snap_nearest(Tick::MAX).0);
}

#[test]
fn timecode_base_and_drop_frame_support() {
    assert_eq!(FrameRate::FPS_29_97.timecode_base(), 30);
    assert!(FrameRate::FPS_29_97.supports_drop_frame());
    assert_eq!(FrameRate::FPS_23_976.timecode_base(), 24);
    assert!(!FrameRate::FPS_23_976.supports_drop_frame());
    assert!(FrameRate::FPS_59_94.supports_drop_frame() && FrameRate::FPS_119_88.supports_drop_frame());
    assert!(!FrameRate::FPS_30.supports_drop_frame());
}

#[test]
fn frame_rate_from_f64_snaps_to_common_rates() {
    assert_eq!(FrameRate::from_f64(29.97), FrameRate::FPS_29_97);
    assert_eq!(FrameRate::from_f64(12.5), FrameRate::new(25, 2));
}

#[test]
fn insane_rates_count_as_the_default() {
    let default = FrameRate::default();
    for bad in [FrameRate { num: 0, den: 1 }, FrameRate { num: 30, den: 0 }, FrameRate { num: -30, den: 1 }] {
        assert_eq!(bad.frame_duration(), default.frame_duration());
        assert_eq!(bad.frame_at(Tick(TICKS_PER_SECOND)), default.frame_at(Tick(TICKS_PER_SECOND)));
        assert_eq!(bad.tick_of(7), default.tick_of(7));
        assert_eq!(bad.timecode_base(), 24);
    }
}
