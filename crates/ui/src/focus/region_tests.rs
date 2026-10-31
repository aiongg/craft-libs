//! F6 region focus.

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// step(1) moves focus to the next registered region's entry widget, wrapping; step(-1) goes back; an unregistered region is skipped.
fn f6_cycles_registered_regions_and_wraps() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// A press inside a region makes it current with no ring; F6 afterwards shows the ring on the next region.
fn pointer_press_sets_the_region_without_a_ring() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// With a popup or modal open, step leaves focus where it is.
fn f6_does_nothing_under_a_modal_layer() {
    todo!()
}
