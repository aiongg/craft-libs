//! Whether a window fits its monitor.

use super::work_area::*;
use egui::{Vec2, vec2};

const MONITOR: Vec2 = vec2(1366.0, 768.0);

#[test]
// fits is false for 1440 × 900 on 1366 × 768 and true for 1280 × 720.
fn fits_leaves_the_taskbar_free() {
    assert!(!fits(vec2(1440.0, 900.0), MONITOR));
    assert!(fits(vec2(1280.0, 720.0), MONITOR));
    assert!(fits(vec2(1366.0, 720.0), MONITOR) && !fits(vec2(1366.0, 740.0), MONITOR));
}
