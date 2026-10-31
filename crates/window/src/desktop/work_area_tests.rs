//! The visible part of the window.

use super::work_area::*;
use WindowState::{FullScreen, Maximized, Normal};
use egui::{Rect, Vec2, pos2, vec2};

const MONITOR: Vec2 = vec2(1366.0, 768.0);

fn window(x: f32, y: f32, w: f32, h: f32) -> (Rect, Rect) {
    (Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h)), Rect::from_min_size(pos2(x, y), vec2(w, h)))
}

#[test]
// A 1440 × 900 window on a 1366 × 768 monitor is cut to the monitor less TASKBAR; full screen and an unknown monitor give the content rect.
fn visible_part_cuts_to_the_monitor_less_the_taskbar() {
    let (content, inner) = window(0.0, 0.0, 1440.0, 900.0);
    assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal), Rect::from_min_max(pos2(0.0, 0.0), pos2(1366.0, 768.0 - TASKBAR)));
    let (content, inner) = window(0.0, 31.0, 1366.0, 737.0);
    assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal).bottom(), 768.0 - TASKBAR - 31.0);
    let (content, inner) = window(1366.0 + 100.0, 0.0, 1440.0, 900.0);
    assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal).bottom(), 768.0 - TASKBAR);

    let (content, inner) = window(0.0, 0.0, 1440.0, 900.0);
    assert_eq!(visible_part(content, Some(MONITOR), Some(inner), FullScreen), content);
    assert_eq!(visible_part(content, None, Some(inner), Normal), content);

    // No position (Wayland): cut at the content's top; a maximized window is all visible.
    assert_eq!(visible_part(content, Some(MONITOR), None, Normal), Rect::from_min_max(pos2(0.0, 0.0), pos2(1366.0, 768.0 - TASKBAR)));
    assert_eq!(visible_part(content, Some(MONITOR), None, Maximized), content);
    // Maximized over a panel is checked like any other window.
    let full = Rect::from_min_size(pos2(0.0, 0.0), vec2(1920.0, 1080.0));
    assert_eq!(visible_part(full, Some(full.size()), Some(full), Maximized).bottom(), 1080.0 - TASKBAR);
}

#[test]
// fits is false for 1440 × 900 on 1366 × 768 and true for 1280 × 720; a result under 160 pt falls back to the content rect.
fn fits_and_the_untrusted_fallback() {
    assert!(!fits(vec2(1440.0, 900.0), MONITOR));
    assert!(fits(vec2(1280.0, 720.0), MONITOR));
    assert!(fits(vec2(1366.0, 720.0), MONITOR) && !fits(vec2(1366.0, 740.0), MONITOR));

    let (content, inner) = window(0.0, 600.0, 1000.0, 300.0);
    assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal), content);
    let (content, inner) = window(100.0, 80.0, 800.0, 500.0);
    assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal), content);
    assert_eq!(visible_part(content, Some(vec2(f32::NAN, 768.0)), Some(inner), Normal), content);
    assert_eq!(visible_part(content, Some(vec2(0.0, 0.0)), Some(inner), Normal), content);
}
