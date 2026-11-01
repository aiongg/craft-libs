//! Tests of the preview maps and painter (task TR-T10).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A Distort dragged past the horizon leaves the cells with a corner on the far side out of the
// mesh; the cells in front stay.
#[test]
#[ignore = "skeleton"]
fn cells_past_the_horizon_are_left_out_of_a_distorted_quad() {
    todo!()
}

// A quad hanging past the canvas edge draws nothing past it, also under a turned view.
#[test]
#[ignore = "skeleton"]
fn pixels_past_the_canvas_edge_are_clipped_in_document_space() {
    todo!()
}

// A pulled warp's cells come out from the most displaced to the least.
#[test]
#[ignore = "skeleton"]
fn the_most_displaced_triangles_are_drawn_first() {
    todo!()
}

// A narrow, hard-pulled patch gets as many samples as its stretch needs (up to 96 per patch, 240
// per axis); an unwarped mesh keeps 4 per patch.
#[test]
#[ignore = "skeleton"]
fn a_narrow_pulled_cell_gets_as_many_samples_as_its_stretch_needs() {
    todo!()
}

// An identity mesh at zoom 1 samples each patch at the minimum.
#[test]
#[ignore = "skeleton"]
fn an_unwarped_mesh_stays_cheap() {
    todo!()
}

// Opacity 0.5 tints every vertex white at alpha 128.
#[test]
#[ignore = "skeleton"]
fn opacity_tints_the_mesh() {
    todo!()
}

// A pin map draws the solver's triangles in its order with texels at the rest positions; a
// function map draws a 40 × 40 grid through the function.
#[test]
#[ignore = "skeleton"]
fn pin_and_function_maps_draw_their_meshes() {
    todo!()
}
