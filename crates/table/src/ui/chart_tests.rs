//! Chart painting.

use egui::{Context, LayerId, Pos2, RawInput, Rect, Shape, pos2, vec2};

use super::chart::{ChartPrimitive, HAlign, VAlign, paint_chart};

const RED: [u8; 4] = [200, 30, 30, 255];

/// The shapes `paint_chart` adds for `primitives`.
fn painted(primitives: &[ChartPrimitive], origin: Pos2, zoom: f32) -> Vec<Shape> {
    let ctx = Context::default();
    let mut output = ctx.run_ui(RawInput::default(), |_| paint_chart(&ctx.layer_painter(LayerId::background()), origin, primitives, zoom));
    output.textures_delta.clear();
    output.shapes.into_iter().map(|clipped| clipped.shape).collect()
}

fn bounds(shapes: &[Shape]) -> Rect {
    shapes.iter().map(Shape::visual_bounding_rect).reduce(Rect::union).expect("shapes")
}

// each ChartPrimitive kind adds shapes at the zoomed positions.
#[test]
fn every_primitive_paints() {
    let origin = pos2(10.0, 20.0);
    let zoom = 2.0;
    let rect = ChartPrimitive::Rect { x: 5.0, y: 6.0, w: 30.0, h: 10.0, fill: Some(RED), stroke: None, radius: 0.0 };
    let shapes = painted(&[rect], origin, zoom);
    assert_eq!(bounds(&shapes), Rect::from_min_size(pos2(20.0, 32.0), vec2(60.0, 20.0)));

    let line = ChartPrimitive::Line { points: vec![[0.0, 0.0], [10.0, 5.0]], color: RED, width: 1.0, dash: false };
    let shapes = painted(&[line], origin, zoom);
    assert!(bounds(&shapes).contains(pos2(30.0, 30.0)), "the line ends at the zoomed point");
    let dashed = ChartPrimitive::Line { points: vec![[0.0, 0.0], [40.0, 0.0]], color: RED, width: 1.0, dash: true };
    assert!(painted(&[dashed], origin, zoom).len() > 1, "a dashed line is several segments");
    let short = ChartPrimitive::Line { points: vec![[0.0, 0.0]], color: RED, width: 1.0, dash: false };
    assert!(painted(&[short], origin, zoom).is_empty(), "a line needs two points");

    let triangle = ChartPrimitive::Polygon { points: vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]], fill: RED, stroke: None };
    assert!(!painted(&[triangle], origin, zoom).is_empty());
    let mountain =
        ChartPrimitive::Polygon { points: vec![[0.0, 2.0], [3.0, 0.0], [6.0, 6.0], [10.0, 1.0], [10.0, 10.0], [0.0, 10.0]], fill: RED, stroke: None };
    assert!(matches!(painted(&[mountain], origin, zoom)[0], Shape::Mesh(_)), "a concave polygon is a mesh");

    let wedge = ChartPrimitive::Wedge { cx: 50.0, cy: 50.0, r_outer: 20.0, r_inner: 0.0, a0: 0.0, a1: 1.5, fill: RED, stroke: Some((RED, 1.0)) };
    let shapes = painted(&[wedge], origin, zoom);
    assert!(shapes.len() >= 2, "a wedge is a mesh and its outline");
    let circle = ChartPrimitive::Circle { cx: 50.0, cy: 50.0, r: 5.0, fill: Some(RED), stroke: None };
    let shapes = painted(&[circle], origin, zoom);
    assert_eq!(bounds(&shapes).center(), pos2(110.0, 120.0));

    let text = ChartPrimitive::Text {
        x: 10.0,
        y: 10.0,
        text: "Sales".into(),
        size: 12.0,
        color: RED,
        bold: true,
        align: HAlign::Right,
        valign: VAlign::Bottom,
        rotation: 0.0,
    };
    let shapes = painted(&[text], origin, zoom);
    let at = bounds(&shapes);
    // The ink bounds stop short of the line box at the bottom (descender room).
    assert!((at.right() - 30.0).abs() < 1.0 && (35.0..=40.0).contains(&at.bottom()), "text is anchored at its bottom right: {at:?}");
}
