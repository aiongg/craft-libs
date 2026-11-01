//! Tests of artboard rectangles (task SH-T1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `resized_rect` of (0, 0, 100, 100) with the left handle dragged by 150 gives (100, 0, 150, 100);
// dragged by exactly 100 it is 1 px wide; ⌥ moves the right edge the other way.
#[test]
#[ignore = "skeleton"]
fn artboard_resize_past_opposite_edge_flips_with_min_one_pixel() {
    todo!()
}

// `resized_keep_ratio` of 200 × 100 by the bottom-right corner (+20, +40) scales by 1.4 to
// 280 × 140 from the fixed top-left corner; an edge handle resizes one axis only.
#[test]
#[ignore = "skeleton"]
fn artboard_corner_resize_keeps_proportions() {
    todo!()
}

// `drawn_rect` and `moved_rect` round to whole pixels and order the corners.
#[test]
#[ignore = "skeleton"]
fn artboard_draw_and_move_are_whole_pixels() {
    todo!()
}
