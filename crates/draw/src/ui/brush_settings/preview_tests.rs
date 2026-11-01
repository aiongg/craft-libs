//! Tests of the preview stroke and the stroke strip (task BR-BS3).

use std::cell::Cell;

use egui::accesskit::Role;
use egui::{Color32, ColorImage, Context, Pos2, RawInput, Rect, vec2};

use super::{BrushStrokeStrip, PreviewStroke};
use craft_ui::canvas::textures::Revision;
use craft_ui::theme::{Base, install};

// 65 samples; the first at x = 10 and the last at width − 10, both at mid-height with pressure
// 0.05; the curve's top at s = 0.25 is 0.22 × height above the middle; `fit` is 1 for a small
// brush and 0.55 × height / diameter for a large one.
#[test]
fn preview_stroke_samples_match_the_s_curve() {
    let stroke = PreviewStroke::new(300, 76);
    let samples = stroke.samples();
    assert_eq!(samples.len(), PreviewStroke::SAMPLES);
    let (first, last, top) = (samples[0], samples[64], samples[16]);
    assert!((first.x - 10.0).abs() < 1e-9 && (last.x - 290.0).abs() < 1e-9);
    assert!((first.y - 38.0).abs() < 1e-9 && (last.y - 38.0).abs() < 1e-9);
    assert!((first.pressure - 0.05).abs() < 1e-6 && (last.pressure - 0.05).abs() < 1e-6);
    assert!((top.y - (38.0 - 0.22 * 76.0)).abs() < 1e-9);
    assert!((top.pressure - 0.5f32.sqrt().max(0.0)).abs() < 1e-3);
    assert!((stroke.fit(10.0) - 1.0).abs() < 1e-12);
    assert!((stroke.fit(100.0) - 0.55 * 76.0 / 100.0).abs() < 1e-12);
    assert!((stroke.fit(0.0) - 1.0).abs() < 1e-12);
}

// The strip's render closure runs on the first pass, not again for the same revision and size,
// and again after the revision changes.
#[test]
fn strip_renders_once_per_revision() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let runs = Cell::new(0);
    let pass = |revision: Revision| {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))), ..Default::default() };
        let mut named = false;
        let result = ctx.run_ui(input, |ui| {
            BrushStrokeStrip::new("strip", revision).show(ui, |[w, h]| {
                runs.set(runs.get() + 1);
                ColorImage::filled([w, h], Color32::WHITE)
            });
        });
        if let Some(update) = result.platform_output.accesskit_update.clone() {
            named = update.nodes.iter().any(|(_, n)| n.role() == Role::Image && n.label() == Some("Stroke preview"));
        }
        result.drop_without_applying_deltas();
        named
    };
    let a = Revision::of(&1);
    assert!(pass(a), "the strip is an image named Stroke preview");
    pass(a);
    assert_eq!(runs.get(), 1);
    pass(Revision::of(&2));
    assert_eq!(runs.get(), 2);
}
