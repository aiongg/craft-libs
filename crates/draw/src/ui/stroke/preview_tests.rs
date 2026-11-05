//! Tests of the stroke preview painter.

use egui::{Color32, Pos2, Rect, Shape};

use super::StrokePreview;
use crate::stroke::{LineCap, LineJoin, StrokeStyle};

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, egui::vec2(200.0, 60.0))
}

fn count(style: &StrokeStyle, closed: bool) -> usize {
    StrokePreview::new(style, 2.5, Color32::WHITE).closed(closed).shapes(rect()).len()
}

// A dashed open line gives one set of convex pieces per dash; round caps add a fan at each dash
// end; a closed sample's miter, round and bevel corners add a wedge, a fan and a triangle.
#[test]
fn stroke_preview_dashes_follow_cap_and_join() {
    let solid = StrokeStyle { width: 2.0, ..StrokeStyle::default() };
    assert_eq!(count(&solid, false), 1);

    let dashed = StrokeStyle { dashes: vec![2.0, 2.0], ..solid.clone() };
    let dashes = count(&dashed, false);
    assert!(dashes > 10, "{dashes} dashes");
    let round = StrokeStyle { cap: LineCap::Round, ..dashed.clone() };
    assert_eq!(count(&round, false), dashes * 3, "a quad and two fans per dash");
    let square = StrokeStyle { cap: LineCap::Square, ..dashed };
    assert_eq!(count(&square, false), dashes * 3, "a quad and two square caps per dash");

    let corners = |join| count(&StrokeStyle { join, ..solid.clone() }, true);
    assert_eq!(corners(LineJoin::Miter), 4 + 4, "four sides and four wedges");
    assert_eq!(corners(LineJoin::Bevel), 4 + 4, "four sides and four triangles");
    assert_eq!(corners(LineJoin::Round), 4 + 4, "four sides and four fans");
    let wedge = |join| {
        let style = StrokeStyle { join, ..solid.clone() };
        let shapes = StrokePreview::new(&style, 2.5, Color32::WHITE).closed(true).shapes(rect());
        shapes.iter().skip(4).map(|s| if let Shape::Path(p) = s { p.points.len() } else { 0 }).collect::<Vec<_>>()
    };
    assert_eq!(wedge(LineJoin::Miter), [4; 4]);
    assert_eq!(wedge(LineJoin::Bevel), [3; 4]);
    assert!(wedge(LineJoin::Round).iter().all(|&n| n >= 8));
    let low_limit = StrokeStyle { miter_limit: 1.0, ..solid };
    assert_eq!(count(&low_limit, true), 8, "a corner past the limit is bevelled");
}

// An invalid style draws nothing; opacity 0.5 halves the colour's alpha; a right-to-left layout
// still draws left to right.
#[test]
fn stroke_preview_skips_invalid_styles_and_applies_opacity() {
    let bad = StrokeStyle { width: -1.0, ..StrokeStyle::default() };
    assert!(StrokePreview::new(&bad, 2.5, Color32::WHITE).shapes(rect()).is_empty());

    let half = StrokeStyle { width: 2.0, opacity: 0.5, ..StrokeStyle::default() };
    let shapes = StrokePreview::new(&half, 2.5, Color32::WHITE).shapes(rect());
    let Some(Shape::Path(path)) = shapes.first() else { panic!("one polygon") };
    assert_eq!(path.fill, Color32::WHITE.gamma_multiply(0.5));
    assert!((i32::from(path.fill.a()) - 128).abs() <= 1);

    // The points are screen coordinates, so a mirrored layout draws the same line.
    let xs: Vec<f32> = path.points.iter().map(|p| p.x).collect();
    assert_eq!(xs.iter().copied().fold(f32::MAX, f32::min), 4.0);
    assert_eq!(xs.iter().copied().fold(f32::MIN, f32::max), 196.0);
}
