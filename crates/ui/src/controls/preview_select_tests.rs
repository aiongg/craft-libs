//! Preview selects.

use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{Context, Event, Pos2, Rect, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{LineStyle, LineWeights, Select, SelectText, paint_line_style};
use crate::theme::{Base, ToEgui as _, TokensOf as _, install};

// `LineStyle::label` gives WordCraft's and DesignCraft's names ("Single", "Dot-dash", "Thick - Thin").
#[test]
fn line_style_names_follow_the_apps() {
    let names: Vec<_> = LineStyle::UNDERLINES.iter().map(|s| s.label()).collect();
    assert_eq!(names, ["Single", "Double", "Thick", "Dotted", "Dashed", "Dot-dash", "Dot-dot-dash", "Wave"]);
    let strokes: Vec<_> = LineStyle::STROKES.iter().map(|s| s.label()).collect();
    assert_eq!(strokes, ["Single", "Dashed", "Dotted", "Thick - Thin", "Thin - Thick", "Thin - Thin", "Thick - Thick"]);
}

fn harness(shown: SelectText) -> Harness<'static, LineStyle> {
    let builder: HarnessBuilder<LineStyle> =
        Harness::builder().with_size([320.0, 320.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()));
    builder.build_ui_state(
        move |ui, style: &mut LineStyle| {
            let ink = crate::theme::Tokens::of(ui).color.text.normal.to_egui();
            let _ = Select::new("underline", "Underline", style)
                .rows(LineStyle::UNDERLINES.len(), |i| LineStyle::UNDERLINES[i], |i| LineStyle::UNDERLINES[i].label())
                .row_content(vec2(120.0, 22.0), move |painter, rect, style| paint_line_style(painter, rect, *style, ink, &LineWeights::UNDERLINE))
                .text_shown(shown)
                .show(ui);
        },
        LineStyle::Solid,
    )
}

/// Every text the last frame painted.
fn painted_text<State>(harness: &Harness<'_, State>) -> Vec<String> {
    fn walk(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Text(text) => out.push(text.galley.text().to_owned()),
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    harness.output().shapes.iter().for_each(|clipped| walk(&clipped.shape, &mut out));
    out
}

fn names_painted<State>(harness: &Harness<'_, State>) -> usize {
    painted_text(harness).iter().filter(|text| LineStyle::UNDERLINES.iter().any(|s| s.label() == text.as_str())).count()
}

// In a harness: with `SelectText::Never` no option text is painted, each row is still named by its text, and type-ahead finds it.
#[test]
fn a_select_without_text_keeps_its_names_for_accesskit() {
    let mut harness = harness(SelectText::Never);
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let rows: Vec<_> = harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect();
    assert_eq!(rows.len(), 8);
    assert!(rows.contains(&"Wave".to_owned()) && rows.contains(&"Dot-dash".to_owned()), "{rows:?}");
    assert_eq!(names_painted(&harness), 0, "{:?}", painted_text(&harness));
    harness.event(Event::Text("w".into()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), LineStyle::Wave);
    assert_eq!(names_painted(&harness), 0, "the closed box paints no name either");
    assert_eq!(harness.get_by_role(Role::ComboBox).accesskit_node().value().as_deref(), Some("Wave"));
}

// In a harness: with `SelectText::InList` the closed box paints the content without the text and the list rows paint both.
#[test]
fn the_closed_box_shows_only_the_preview_with_in_list() {
    let mut harness = harness(SelectText::InList);
    harness.run();
    assert_eq!(names_painted(&harness), 0, "{:?}", painted_text(&harness));
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    assert_eq!(names_painted(&harness), 8, "{:?}", painted_text(&harness));

    let mut always = self::harness(SelectText::Always);
    always.run();
    assert_eq!(names_painted(&always), 1, "the closed box names its value");
}

/// The shapes `paint_line_style` adds for `style` at `pixels_per_point`, as rects of lines and
/// dashes (width, height, left) and circles (radius, pitch from the previous one).
fn sample(style: LineStyle, weights: &LineWeights, pixels_per_point: f32) -> Vec<Shape> {
    let ctx = Context::default();
    ctx.set_pixels_per_point(pixels_per_point);
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0))), ..Default::default() };
    let output = ctx.run_ui(input, |ui| {
        let rect = Rect::from_min_size(Pos2::new(5.0, 10.0), vec2(60.0, 20.0));
        paint_line_style(ui.painter(), rect, style, egui::Color32::WHITE, weights);
    });
    let mut out = Vec::new();
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    output.shapes.iter().for_each(|clipped| flatten(&clipped.shape, &mut out));
    output.drop_without_applying_deltas();
    out
}

fn bars(shapes: &[Shape]) -> Vec<Rect> {
    shapes.iter().filter_map(|s| if let Shape::Mesh(mesh) = s { Some(mesh.calc_bounds()) } else { None }).collect()
}

// `paint_line_style` keeps DesignCraft's weights with `LineWeights::STROKE` (solid 5, dashes 4 every 9, dots of radius 2 every 6) and WordCraft's with `UNDERLINE` (1).
#[test]
fn the_samples_keep_each_apps_weights() {
    let solid = bars(&sample(LineStyle::Solid, &LineWeights::STROKE, 1.0));
    assert_eq!(solid.len(), 1);
    assert_eq!(solid[0].height(), 5.0);
    let underline = bars(&sample(LineStyle::Solid, &LineWeights::UNDERLINE, 1.0));
    assert_eq!(underline[0].height(), 1.0);

    let dashes = bars(&sample(LineStyle::Dashed, &LineWeights::STROKE, 1.0));
    assert!(dashes.len() > 3 && dashes.iter().all(|d| d.height() == 4.0), "{dashes:?}");
    assert_eq!(dashes[1].left() - dashes[0].left(), 9.0);

    let dots: Vec<_> = sample(LineStyle::Dotted, &LineWeights::STROKE, 1.0)
        .into_iter()
        .filter_map(|s| if let Shape::Circle(c) = s { Some((c.center.x, c.radius)) } else { None })
        .collect();
    assert!(dots.len() > 3 && dots.iter().all(|(_, r)| *r == 2.0), "{dots:?}");
    assert_eq!(dots[1].0 - dots[0].0, 6.0);
}

// A 1 point line lies on whole device pixels at 1.5 pixels per point (its edges are multiples of 1/1.5).
#[test]
fn a_thin_sample_sits_on_whole_pixels() {
    let line = bars(&sample(LineStyle::Solid, &LineWeights::UNDERLINE, 1.5))[0];
    for edge in [line.top(), line.bottom()] {
        let device = edge * 1.5;
        assert!((device - device.round()).abs() < 1e-3, "{edge}");
    }
}
