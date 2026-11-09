//! Tests of artboard rectangles.

use kurbo::{Point, Rect, Vec2};

use super::artboard::{drawn_rect, moved_rect, resized_keep_ratio, resized_rect};

// `resized_rect` of (0, 0, 100, 100) with the left handle dragged by 150 gives (100, 0, 150, 100);
// dragged by exactly 100 it is 1 px wide; ⌥ moves the right edge the other way.
#[test]
fn artboard_resize_past_opposite_edge_flips_with_min_one_pixel() {
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert_eq!(resized_rect(r, [-1, 0], Vec2::new(150.0, 0.0), false), Rect::new(100.0, 0.0, 150.0, 100.0));
    assert_eq!(resized_rect(r, [-1, 0], Vec2::new(100.0, 0.0), false), Rect::new(100.0, 0.0, 101.0, 100.0));
    assert_eq!(resized_rect(r, [-1, 0], Vec2::new(20.0, 0.0), true), Rect::new(20.0, 0.0, 80.0, 100.0));
    assert_eq!(resized_rect(r, [1, 1], Vec2::new(10.0, -10.0), true), Rect::new(-10.0, 10.0, 110.0, 90.0));
}

// `resized_keep_ratio` of 200 × 100 by the bottom-right corner (+20, +40) scales by 1.4 to
// 280 × 140 from the fixed top-left corner; an edge handle resizes one axis only.
#[test]
fn artboard_corner_resize_keeps_proportions() {
    let r = Rect::new(0.0, 0.0, 200.0, 100.0);
    assert_eq!(resized_keep_ratio(r, [1, 1], Vec2::new(20.0, 40.0), false), Rect::new(0.0, 0.0, 280.0, 140.0));
    assert_eq!(resized_keep_ratio(r, [-1, -1], Vec2::new(-20.0, -40.0), false), Rect::new(-80.0, -40.0, 200.0, 100.0));
    assert_eq!(resized_keep_ratio(r, [1, 0], Vec2::new(30.0, 40.0), false), Rect::new(0.0, 0.0, 230.0, 100.0));
    // About the centre the corner scales from (100, 50): k = 1.8.
    assert_eq!(resized_keep_ratio(r, [1, 1], Vec2::new(20.0, 40.0), true), Rect::new(-80.0, -40.0, 280.0, 140.0));
}

// `drawn_rect` and `moved_rect` round to whole pixels and order the corners.
