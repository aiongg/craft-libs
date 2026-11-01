//! Tests of the aliased tip's cell on the pixel grid (screen).

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// At 1.5 and 2 pixels per point the painted cell's edges land on `1 / ppp` multiples, square and round.
#[test]
#[ignore = "skeleton"]
fn pixel_tip_snaps_to_physical_pixels() {
    todo!()
}
