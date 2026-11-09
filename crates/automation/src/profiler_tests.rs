//! Behaviour tests of `profiler`.
use super::*;

fn frame(t: f64) -> FrameStages {
    let mut s = FrameStages::default();
    s.start(t, 0);
    s.ui_start(t + 1.0, None);
    s
}

// Only frames over the threshold are kept, at most SLOW_FRAMES, oldest first.
