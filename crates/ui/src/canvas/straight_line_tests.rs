//! Tests of the straight line preview.

use egui::{Color32, Context, Rect, Shape, pos2, vec2};
use egui_kittest::Harness;
use kurbo::Point;

use super::CanvasView;
use super::straight_line::{LineGuideStyle, StraightLinePreview};
use crate::theme::{Base, current, install};

fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

/// The line segments one preview paints, with the style used.
fn paint(from: Point, to: Point) -> (Vec<Shape>, LineGuideStyle) {
    let style = std::cell::Cell::new(None);
    let harness =
        Harness::builder().with_size([200.0, 200.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(|ui| {
            let s = LineGuideStyle::new(&current(ui.ctx()));
            style.set(Some(s));
            let view = CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)), Point::new(100.0, 100.0), 1.0);
            StraightLinePreview::new(from, to).paint(ui.painter(), &view, &s);
        });
    let mut shapes = Vec::new();
    harness.output().shapes.iter().for_each(|c| flatten(&c.shape, &mut shapes));
    let segments = shapes.into_iter().filter(|s| matches!(s, Shape::LineSegment { .. })).collect();
    (segments, style.get().unwrap())
}

// The preview paints two segments from the last stroke's end to the pointer: the halo first, then
// the line, with the style's widths and colours.
#[test]
fn preview_draws_two_strokes_from_the_last_stroke_end() {
    let (segments, style) = paint(Point::new(10.0, 20.0), Point::new(60.0, 80.0));
    assert_eq!(segments.len(), 2);
    let stroke = |s: &Shape| match s {
        Shape::LineSegment { points, stroke } => (*points, stroke.width, stroke.color),
        other => panic!("a line segment, not {other:?}"),
    };
    let (halo, line) = (stroke(&segments[0]), stroke(&segments[1]));
    assert_eq!((halo.1, halo.2), (style.halo, style.halo_color));
    assert_eq!((line.1, line.2), (style.width, style.line));
    assert!(style.halo > style.width && style.line != Color32::TRANSPARENT);
    for (points, ..) in [halo, line] {
        assert_eq!(points, [pos2(10.0, 20.0), pos2(60.0, 80.0)]);
    }
}

// A non-finite end paints nothing.
#[test]
fn a_non_finite_end_paints_nothing() {
    assert!(paint(Point::new(f64::NAN, 0.0), Point::new(5.0, 5.0)).0.is_empty());
    assert!(paint(Point::new(0.0, 0.0), Point::new(f64::INFINITY, 5.0)).0.is_empty());
}
