//! Behaviour tests of `trace`.
use super::*;

fn frame(t: &mut FrameTrace, now: f64, cpu: f32, dragging: bool) {
    let mut s = FrameStages::default();
    s.start(now, 3);
    s.ui_start(now, Some(cpu / 1000.0));
    if dragging {
        s.gesture_ms = 1.0;
        s.refresh("rect", 10, 1.5, 0.5);
    }
    s.ui_ms = 4.0;
    t.frame(&s, dragging, || "vulkan Test GPU".into());
}

// Rows accumulate while in a gesture; the first frame after it produces the summary.
#[test]
fn gesture_end_produces_a_summary() {
    let mut t = FrameTrace::from_value("1");
    frame(&mut t, 0.0, 5.0, false);
    frame(&mut t, 100.0, 5.0, true);
    for k in 1..=10 {
        frame(&mut t, 100.0 + 16.0 * f64::from(k), 6.0, true);
    }
    assert!(t.last.is_none(), "still in the gesture");
    frame(&mut t, 276.0, 6.0, false);
    let s = t.last.clone().expect("the gesture ended");
    assert_eq!(s.frames, 11);
    assert!(s.interval.iter().all(|ms| *ms < 100.0), "the idle gap before the gesture is not a frame interval: {:?}", s.interval);
    assert!(s.line().contains("11"), "{}", s.line());
    assert_eq!(percentiles(vec![f64::NAN]), [0.0, 0.0]);

    let mut off = FrameTrace::from_value("off");
    frame(&mut off, 0.0, 1.0, true);
    frame(&mut off, 16.0, 1.0, false);
    assert!(off.last.is_none());
    off.set_on(true);
    assert!(off.is_on() == !cfg!(target_arch = "wasm32"));
}
