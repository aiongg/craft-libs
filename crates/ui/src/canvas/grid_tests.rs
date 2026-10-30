use egui::{Color32, Rect as ScreenRect, pos2};
use kurbo::{Point, Rect, Vec2};

use super::*;
use crate::theme::{Base, install, tokens_in_mode};
use craft_theme::Mode;

fn style() -> GridStyle {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    CanvasGrid::style(&tokens_in_mode(&ctx, Mode::Light))
}

/// A 400 × 300 canvas showing document (0, 0)–(400, 300) at `zoom`.
fn view(zoom: f64) -> CanvasView {
    let rect = ScreenRect::from_min_size(pos2(0.0, 0.0), egui::vec2(400.0, 300.0) * zoom as f32);
    CanvasView::new(rect, Point::new(200.0, 150.0), zoom)
}

fn grid(step: f64, subdivisions: u32) -> CanvasGrid {
    CanvasGrid::new("grid").spacing(Vec2::new(step, step)).subdivisions(subdivisions)
}

#[test]
fn spacing_and_subdivisions_give_the_line_count() {
    let lines = grid(100.0, 4).lines(&view(1.0), &style());
    // x = 0, 100, ..., 400 are majors; three minors in each of the four gaps.
    assert_eq!((lines.x.major.len(), lines.x.minor.len()), (5, 12));
    assert_eq!((lines.y.major.len(), lines.y.minor.len()), (4, 9));
    assert!(lines.x.minor.iter().all(|x| x % 100.0 != 0.0));
}

#[test]
fn majors_alone_when_the_minor_step_is_under_the_minimum() {
    // 100 / 20 = 5 pt minor step, under the 6 pt minimum.
    let lines = grid(100.0, 20).lines(&view(1.0), &style());
    assert_eq!((lines.x.major.len(), lines.x.minor.len()), (5, 0));
    // Exactly the minimum draws.
    let lines = grid(120.0, 20).lines(&view(1.0), &style());
    assert_eq!((lines.x.major.len(), lines.x.minor.len()), (4, 63));
}

#[test]
fn an_axis_under_the_minimum_draws_nothing() {
    let lines = CanvasGrid::new("grid").spacing(Vec2::new(5.0, 100.0)).lines(&view(1.0), &style());
    assert!(lines.x.major.is_empty() && lines.x.minor.is_empty());
    assert_eq!(lines.y.major.len(), 4);
    for bad in [0.0, -10.0, f64::NAN, f64::INFINITY] {
        assert!(grid(bad, 4).lines(&view(1.0), &style()).x.major.is_empty());
    }
}

#[test]
fn origin_shifts_the_lines() {
    let lines = grid(100.0, 1).origin(Point::new(30.0, -20.0)).lines(&view(1.0), &style());
    assert_eq!(lines.x.major, vec![30.0, 130.0, 230.0, 330.0]);
    assert_eq!(lines.y.major, vec![-20.0 + 100.0, 180.0, 280.0]);
}

#[test]
fn bounds_limit_the_lines() {
    let lines = grid(50.0, 1).bounds(Rect::new(100.0, 50.0, 200.0, 150.0)).lines(&view(1.0), &style());
    assert_eq!(lines.x.major, vec![100.0, 150.0, 200.0]);
    assert_eq!(lines.y.major, vec![50.0, 100.0, 150.0]);
    assert_eq!(lines.extent, Some(Rect::new(100.0, 50.0, 200.0, 150.0)));
    // Bounds off screen give nothing.
    assert_eq!(grid(50.0, 1).bounds(Rect::new(1000.0, 0.0, 1100.0, 100.0)).lines(&view(1.0), &style()), GridLines::default());
}

#[test]
fn lines_stay_inside_the_visible_rect_and_the_cap() {
    let lines = grid(6.0, 1).bounds(Rect::new(-1e9, -1e9, 1e9, 1e9)).lines(&view(1.0), &style());
    assert_eq!(lines.x.major.len(), 67);
    assert!(lines.x.major.iter().all(|x| (0.0..=400.0).contains(x)));
    let wide = grid(6.0, 1).lines(&view(1e-4), &style());
    assert!(wide.x.major.is_empty(), "6 doc units at zoom 1e-4 is under the minimum step");
    let many = CanvasGrid::new("g")
        .spacing(Vec2::new(1.0, 1.0))
        .lines(&CanvasView::new(ScreenRect::from_min_size(pos2(0.0, 0.0), egui::vec2(60_000.0, 10.0)), Point::new(0.0, 0.0), 6.0), &style());
    assert_eq!(many.x.major.len(), MAX_LINES);
}

#[test]
fn zoom_and_pixel_aspect_set_each_axis_step() {
    // Aspect 0.5: a document unit is 0.5 pt wide, so a 10 unit step is 5 pt along x (hidden) and
    // 10 pt along y.
    let view = CanvasView { aspect: 0.5, ..view(1.0) };
    let lines = grid(10.0, 1).lines(&view, &style());
    assert!(lines.x.major.is_empty());
    assert!(!lines.y.major.is_empty());
}

#[test]
fn a_rotated_view_maps_lines_onto_the_document() {
    let view = CanvasView { rotation: 90.0, ..view(1.0) };
    let lines = grid(100.0, 1).bounds(Rect::new(0.0, 0.0, 400.0, 300.0)).lines(&view, &style());
    let x100 = lines.x.major.iter().copied().find(|x| *x == 100.0).expect("x = 100");
    let (a, b) = (view.to_screen(Point::new(x100, 0.0)), view.to_screen(Point::new(x100, 300.0)));
    // A document-vertical line is screen-horizontal after a quarter turn.
    assert!((a.y - b.y).abs() < 1e-3 && (a.x - b.x).abs() > 100.0);
}

#[test]
fn crossings_need_the_major_spacing() {
    let s = style();
    assert!(grid(100.0, 4).line(GridLine::Crossings).lines(&view(1.0), &s).crossings);
    assert!(grid(8.0, 1).lines(&view(1.0), &s).crossings);
    assert!(!grid(7.9, 1).lines(&view(1.0), &s).crossings);
    // Subdividing does not enable them below the major minimum.
    assert!(!grid(7.0, 1).lines(&view(1.0), &s).crossings);
}

#[test]
fn min_zoom_hides_the_grid() {
    let s = style();
    assert_eq!(grid(10.0, 1).min_zoom(Some(6.0)).lines(&view(1.0), &s), GridLines::default());
    assert!(!grid(1.0, 1).min_zoom(Some(6.0)).lines(&view(6.0), &s).x.major.is_empty());
}

#[test]
fn the_style_follows_the_tokens() {
    let s = style();
    assert_eq!((s.width, s.minor_alpha, s.min_step, s.crossing, s.crossing_min), (1.0, 0.45, 6.0, 1.5, 8.0));
    assert_eq!(s.color, Color32::from_rgb(0x8c, 0x8c, 0x8c));
    // Dashed and Dots use the guides' values.
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let guide = tokens_in_mode(&ctx, Mode::Light).component.guide;
    assert_eq!((s.dash, s.gap, s.dot_gap, s.dot_radius), (guide.dash, guide.gap, guide.dot_gap, guide.dot_radius));
}

#[test]
fn every_style_draws_in_a_ui() {
    for line in [GridLine::Solid, GridLine::Dashed, GridLine::Dots, GridLine::Crossings] {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(400.0, 300.0))
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| {
                grid(50.0, 5).line(line).show(ui, &view(1.0));
            });
        harness.run();
    }
}
