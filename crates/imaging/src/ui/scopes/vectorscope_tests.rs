//! The vectorscope's geometry.

use egui::{Rect, pos2, vec2};

use super::scope_point;

// `scope_point` puts red at 3 o'clock with `red_right` and at −100° without; saturation 1 is the
// radius (half the shorter side less 14).
#[test]
fn vectorscope_places_red_at_the_chosen_angle() {
    let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 100.0));
    let (center, radius) = (rect.center(), 36.0);
    let near = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-3;
    assert!(near(scope_point(rect, 14.0, true, 0.0, 1.0), center + vec2(radius, 0.0)));
    assert!(near(scope_point(rect, 14.0, true, 0.0, 0.5), center + vec2(radius * 0.5, 0.0)));
    let angle = -100.0f32.to_radians();
    assert!(near(scope_point(rect, 14.0, false, 0.0, 1.0), center + vec2(angle.cos(), angle.sin()) * radius));
    assert!(near(scope_point(rect, 14.0, true, 0.0, 0.0), center));
}

fn run_plot(light: bool, size: egui::Vec2) -> (Vec<egui::epaint::ClippedShape>, f32, std::sync::Arc<craft_ui::theme::UiTokens>) {
    use craft_ui::theme::{Base, install, tokens_in_mode};
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    if light {
        ctx.set_theme(egui::ThemePreference::Light);
    }
    let frame = super::VectorscopeFrame { counts: &[], hues: 0, saturations: 0, hover: None };
    let mut height = 0.0;
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), size)), ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| {
        height = super::VectorscopePlot::new("scope", &frame).show(ui).rect.height();
    });
    let dark = tokens_in_mode(&ctx, craft_theme::Mode::Dark);
    let shapes = std::mem::take(&mut output.shapes);
    output.drop_without_applying_deltas();
    (shapes, height, dark)
}

// The plot is dark in every theme, so the skin-tone line, spokes and rings read the dark mode's
// text colours even in a light theme.
#[test]
fn the_graticule_reads_the_dark_modes_text_colours_in_a_light_theme() {
    use craft_ui::theme::ToEgui as _;
    let (shapes, _, dark) = run_plot(true, vec2(300.0, 300.0));
    let muted = dark.color.text.muted.to_egui();
    let skin = shapes
        .iter()
        .any(|clipped| matches!(&clipped.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == muted && stroke.width == 1.0));
    assert!(skin, "the skin-tone line is the dark mode's text.muted");
}

// Without a height the plot takes 30 % of the available height between the tokens' bounds.
#[test]
fn the_default_height_is_clamped_by_the_tokens() {
    let (_, tall, _) = run_plot(false, vec2(300.0, 2000.0));
    let (_, short, _) = run_plot(false, vec2(300.0, 100.0));
    assert_eq!(tall, 220.0);
    assert_eq!(short, 100.0);
}

// Margin is part of the geometry: a larger margin pulls saturation 1 in.
#[test]
fn the_margin_sets_the_rings_radius() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
    assert!((scope_point(rect, 20.0, true, 0.0, 1.0).x - (rect.center().x + 30.0)).abs() < 1e-3);
}

// The summary writes its numbers in the locale's digits.
#[test]
fn the_summary_uses_the_locale_digits() {
    use craft_ui::localized::LocaleNumbers;
    let counts = [0, 0, 0, 5];
    let frame = super::VectorscopeFrame { counts: &counts, hues: 2, saturations: 2, hover: None };
    let labels = super::ScopePanelLabels::default();
    let latin = frame.summary(&labels, &LocaleNumbers::from_tag("en")).expect("counted");
    let arabic = frame.summary(&labels, &LocaleNumbers::from_tag("ar-EG")).expect("counted");
    assert!(latin.chars().any(|c| c.is_ascii_digit()));
    assert!(!arabic.chars().any(|c| c.is_ascii_digit()), "{arabic}");
}
