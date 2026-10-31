//! Behaviour tests of `profiler`.
use super::*;

fn frame(t: f64) -> FrameStages {
    let mut s = FrameStages::default();
    s.start(t, 0);
    s.ui_start(t + 1.0, None);
    s
}

// Only frames over the threshold are kept, at most SLOW_FRAMES, oldest first.
#[test]
fn slow_frames_ring_keeps_the_latest() {
    let mut r = SlowFrames::default();
    for k in 0..100 {
        let t = f64::from(k) * 100.0;
        r.finish(frame(t), t + 60.0 + f64::from(k), false, None);
    }
    r.finish(frame(10_000.0), 10_050.0, false, None);
    assert_eq!(r.frames().count(), SLOW_FRAMES);
    assert_eq!(r.frames().next().map(|f| f.t), Some(3600.0), "the oldest are dropped");
    assert_eq!(r.frames().last().map(|f| f.frame_ms), Some(159.0), "exactly 50 ms is not slow");
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json.as_array().map(Vec::len), Some(SLOW_FRAMES));
    assert_eq!(json[0]["frameMs"], 96.0);
    assert!(json[0].get("uiMs").is_none(), "per-frame detail stays out of the perf output");
}
