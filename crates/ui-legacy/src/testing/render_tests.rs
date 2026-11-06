//! Behaviour tests of `testing::render`.
use egui::vec2;

use super::{MAX_IMAGE_EDGE, MAX_IMAGE_PIXELS, Render, RenderError, viewport_pixels};

// A 200×100 label at scale 2 renders a 400×200 image whose pixels differ from the background where the label is.
#[test]
fn image_is_the_ui_at_the_requested_scale() {
    let image = Render::new([200.0, 100.0])
        .pixels_per_point(2.0)
        .image(|ui| {
            ui.label("Render");
        })
        .unwrap();
    assert_eq!(image.size, [400, 200]);
    let background = image.pixels[image.pixels.len() - 1];
    assert!(image.pixels.iter().any(|pixel| *pixel != background), "the label left no pixels");
}

// Zero, negative, non-finite and oversize viewports are refused with LightCraft's messages, never clamped.
#[test]
fn impossible_sizes_are_refused() {
    let finite = "viewport dimensions and scale must be finite and positive";
    let edges = format!("scaled viewport edges must be between 1 and {MAX_IMAGE_EDGE} pixels");
    let area = format!("scaled viewport exceeds {MAX_IMAGE_PIXELS} pixels");
    let cases = [
        (vec2(0.0, 100.0), 1.0, finite),
        (vec2(100.0, -1.0), 1.0, finite),
        (vec2(f32::NAN, 100.0), 1.0, finite),
        (vec2(100.0, 100.0), f32::INFINITY, finite),
        (vec2(100.0, 100.0), 0.0, finite),
        (vec2(0.2, 100.0), 1.0, edges.as_str()),
        (vec2(20_000.0, 10.0), 1.0, edges.as_str()),
        (vec2(10_000.0, 10_000.0), 1.0, area.as_str()),
    ];
    for (size, scale, message) in cases {
        assert_eq!(viewport_pixels(size, scale), Err(RenderError::Size(message.into())), "{size:?} at {scale}");
    }
    assert_eq!(viewport_pixels(vec2(200.5, 100.0), 2.0), Ok([401, 200]));
    assert!(Render::new([0.0, 0.0]).image(|_| {}).is_err());
}

// A UI that requests a repaint every frame ends in NotSettled after max_steps, not a hang.
#[test]
fn a_ui_that_never_settles_is_an_error() {
    let error = Render::new([100.0, 50.0])
        .max_steps(4)
        .image(|ui| {
            ui.ctx().request_repaint();
        })
        .unwrap_err();
    assert_eq!(error, RenderError::NotSettled { steps: 4 });
    assert_eq!(error.to_string(), "the UI did not settle within 4 frames");
}

// harness skips the app on the builder's first frame and shows it the raised texture limit from the second; builder refuses an impossible size.
#[test]
fn harness_applies_the_texture_limit_after_the_skipped_first_frame() {
    let mut seen = Vec::new();
    let mut harness = Render::new([100.0, 50.0])
        .max_texture_side(2048)
        .harness(|ui, seen: &mut Vec<usize>| seen.push(ui.ctx().input(|i| i.max_texture_side)), Vec::new())
        .unwrap();
    harness.step();
    std::mem::swap(&mut seen, harness.state_mut());
    assert!(!seen.is_empty() && seen.iter().all(|side| *side == 2048), "{seen:?}");
    assert!(Render::new([0.0, 10.0]).builder::<()>().is_err());
}
