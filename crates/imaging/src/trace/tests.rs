//! Tests of the outline tracer (task SE-F5). Each uses a small `MaskSource` over a `Vec<f32>`
//! written in the test.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Segments of a known 4 × 3 block mask: two merged horizontal runs, two merged vertical runs, one
// padding cell found at a mask touching `bounds`.
#[test]
#[ignore = "skeleton"]
fn trace_merges_runs_and_pads_one_cell() {
    todo!()
}

// A one-pixel-wide line traced at step 8 still gives segments (the coarse-cell edge sample), and
// every segment end stays inside `bounds`.
#[test]
#[ignore = "skeleton"]
fn trace_coarse_cells_keep_one_pixel_selections() {
    todo!()
}

// `bounded_step`: 200 × 150 keeps step 1, 6000 × 6000 gives 8, and huge or negative bounds give a
// grid of at most `TRACE_BUDGET` cells (PhotoCraft's `whole_transform_trace_has_a_bounded_grid`).
#[test]
#[ignore = "skeleton"]
fn trace_bounded_step_caps_cells() {
    todo!()
}

// `edges_of_grid` gives PhotoCraft's Content-Aware Fill unit edges for an L-shaped grid, and
// `None` past the limit.
#[test]
#[ignore = "skeleton"]
fn trace_grid_edges_match_unit_edges() {
    todo!()
}

// `trace_step` at 100 %, 50 %, 12 % and 1 % (with a reduction of 2), and `quantised_region` keeps
// the region for a pan of less than `256 · step` and changes it past that.
#[test]
#[ignore = "skeleton"]
fn trace_step_and_region_follow_zoom_and_pan() {
    todo!()
}
