//! Behaviour tests of `profiler`.
use super::*;

fn frame(t: f64) -> FrameStages {
    let mut s = FrameStages::default();
    s.start(t, 0);
    s.ui_start(t + 1.0, None);
    s
}

// Only frames over the threshold are kept, at most SLOW_FRAMES, oldest dropped first.
#[test]
fn slow_frames_ring_keeps_the_latest() {
    let mut r = SlowFrames::default();
    let starts: Vec<f64> = (0..100).map(|k| f64::from(k) * 100.0).collect();
    for k in 0..100u32 {
        let t = f64::from(k) * 100.0;
        r.finish(frame(t), t + 60.0 + f64::from(k), false, None);
    }
    r.finish(frame(10_000.0), 10_050.0, false, None);
    assert_eq!(r.frames().count(), SLOW_FRAMES);
    assert_eq!(r.frames().next().map(|f| f.t), Some(starts[100 - SLOW_FRAMES]), "the oldest are dropped");
    assert_eq!(r.frames().last().map(|f| f.t), starts.last().copied(), "the newest slow frame is kept, one at the threshold is not slow");
    assert_eq!(serde_json::to_value(&r).unwrap().as_array().map(Vec::len), Some(SLOW_FRAMES));
}
