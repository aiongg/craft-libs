//! Tests of slice rectangles (task SH-T1).

use kurbo::{Point, Rect, Vec2};

use super::slice::{fixed_ratio_pixels, handle_at, resized_rect};

// A 2 : 1 Fixed Ratio drag from (10.4, 10.6) to (20.9, 13.1) gives 10 × 5 on the press's pixel,
// not 5 × 3 or 11 × 6; drawn up and left it holds the press's pixel (PhotoCraft
// `fixed_ratio_slices_keep_their_ratio_from_a_fractional_drag`, `tools/slice_ui/tests.rs`).
#[test]
fn slice_fixed_ratio_keeps_ratio_from_a_fractional_drag() {
    let start = Point::new(10.4, 10.6);
    assert_eq!(fixed_ratio_pixels(start, Point::new(20.9, 13.1), (2.0, 1.0)), Rect::new(10.0, 10.0, 20.0, 15.0));
    // Up and left: grows from the far side of the press's pixel (11, 11).
    assert_eq!(fixed_ratio_pixels(start, Point::new(0.0, 8.0), (2.0, 1.0)), Rect::new(1.0, 6.0, 11.0, 11.0));
    // A click has no size.
    let click = fixed_ratio_pixels(start, Point::new(10.6, 10.7), (2.0, 1.0));
    assert!(click.width() == 0.0 && click.height() == 0.0);
}

// `resized_rect` with the right-edge handle moves only x1; past x0 the rect turns round; the
// result is at least 1 × 1. `handle_at` finds a corner before the edge middle that overlaps it.
#[test]
fn slice_handles_resize_the_edges_they_hang_off_and_flip_past_the_opposite() {
    let r = Rect::new(10.0, 10.0, 20.0, 20.0);
    assert_eq!(resized_rect(r, [1, 0], Vec2::new(5.4, 9.0)), Rect::new(10.0, 10.0, 25.0, 20.0));
    assert_eq!(resized_rect(r, [1, 0], Vec2::new(-15.0, 0.0)), Rect::new(5.0, 10.0, 10.0, 20.0));
    assert_eq!(resized_rect(r, [1, 1], Vec2::new(-10.0, -10.0)), Rect::new(10.0, 10.0, 11.0, 11.0));
    let small = Rect::new(0.0, 0.0, 2.0, 2.0);
    assert_eq!(handle_at(small, Point::new(1.0, 0.0), 1.5), Some([-1, -1]));
    assert_eq!(handle_at(small, Point::new(1.0, 0.0), 0.5), Some([0, -1]));
    assert_eq!(handle_at(small, Point::new(50.0, 50.0), 1.5), None);
}
