//! Tests of rounded corners (task PE-A5).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// PhotoCraft's values: a right angle of a 100 × 60 rectangle caps at 30 and steps (1, 1); an
// acute triangle vertex; a small radius is kept.
#[test]
#[ignore = "skeleton"]
fn fillet_matches_photocraft() {
    todo!()
}

// `None` for a zero radius, NaN, a zero-length edge, a straight and a folded vertex.
#[test]
#[ignore = "skeleton"]
fn fillet_refuses_degenerate_vertices() {
    todo!()
}
