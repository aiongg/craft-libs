//! The CPU raster display.

// texture_factor of 10000 × 3000 at side 4096 is 3; a fitting document is 1.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_document_over_the_texture_limit_is_uploaded_reduced() {
    todo!()
}

// DetailPlan::of at 100 % over a reduced document is Some, snapped to 256; at 10 % it is None.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn zoomed_in_a_reduced_document_gets_a_detail_texture() {
    todo!()
}

// After damage(rect), the next show reads only that rect from the source.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn only_damaged_pixels_are_read_again() {
    todo!()
}

// Squares stay aligned to the canvas's top-left when the view centre moves.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_checkerboard_holds_still_while_the_document_scrolls() {
    todo!()
}
