//! Tests of slice rectangles (task SH-T1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A 2 : 1 Fixed Ratio drag from (10.4, 10.6) to (20.9, 13.1) gives 10 × 5 on the press's pixel,
// not 5 × 3 or 11 × 6; drawn up and left it holds the press's pixel (PhotoCraft
// `fixed_ratio_slices_keep_their_ratio_from_a_fractional_drag`, `tools/slice_ui/tests.rs`).
#[test]
#[ignore = "skeleton"]
fn slice_fixed_ratio_keeps_ratio_from_a_fractional_drag() {
    todo!()
}

// `resized_rect` with the right-edge handle moves only x1; past x0 the rect turns round; the
// result is at least 1 × 1. `handle_at` finds a corner before the edge middle that overlaps it.
#[test]
#[ignore = "skeleton"]
fn slice_handles_resize_the_edges_they_hang_off_and_flip_past_the_opposite() {
    todo!()
}
