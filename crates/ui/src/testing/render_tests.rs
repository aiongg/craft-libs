//! Behaviour tests of `testing::render` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A 200×100 label at scale 2 renders a 400×200 image whose pixels differ from the background where the label is.
#[test]
#[ignore = "skeleton"]
fn image_is_the_ui_at_the_requested_scale() {
    todo!()
}

// Zero, negative, non-finite and oversize viewports are refused with LightCraft's messages, never clamped.
#[test]
#[ignore = "skeleton"]
fn impossible_sizes_are_refused() {
    todo!()
}

// A UI that requests a repaint every frame ends in NotSettled after max_steps, not a hang.
#[test]
#[ignore = "skeleton"]
fn a_ui_that_never_settles_is_an_error() {
    todo!()
}
