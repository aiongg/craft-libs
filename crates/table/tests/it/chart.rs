//! Chart painting.

use craft_table::ui::{ChartPrimitive, HAlign, VAlign, paint_chart};
use craft_theme::{Contrast, Mode};
use craft_ui::runtime::env::Direction;
use craft_ui::style::{DataColor, Ink};
use craft_ui::testing::style::{paint_frame, tokens};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Pos2, Rect, Shape, pos2};

const RED: DataColor = DataColor::new(Color32::from_rgb(200, 30, 30));

/// The shapes `paint_chart` adds for `primitives`.
fn painted(primitives: &[ChartPrimitive], origin: Pos2, zoom: f32) -> Vec<Shape> {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| paint_chart(draw, origin, primitives, zoom)).0
}

fn bounds(shapes: &[Shape]) -> Rect {
    shapes.iter().map(Shape::visual_bounding_rect).reduce(Rect::union).expect("shapes")
}

/// Each primitive kind adds shapes at the zoomed positions.
#[trial]
fn every_primitive_paints() {
    let origin = pos2(10.0, 20.0);
    let zoom = 2.0;
    let rect = ChartPrimitive::rect(5.0, 6.0, 30.0, 10.0).filled(RED);
    assert_eq!(bounds(&painted(&[rect], origin, zoom)), Rect::from_min_max(pos2(20.0, 32.0), pos2(80.0, 52.0)));

    let triangle = ChartPrimitive::polygon(vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]], RED);
    assert_eq!(painted(&[triangle], origin, zoom).len(), 1, "a convex polygon is one shape");
    let mountain = ChartPrimitive::polygon(vec![[0.0, 2.0], [3.0, 0.0], [6.0, 6.0], [10.0, 1.0], [10.0, 10.0], [0.0, 10.0]], RED);
    assert!(painted(&[mountain], origin, zoom).len() > 1, "a concave polygon is painted in strips");

    let wedge = ChartPrimitive::wedge([50.0, 50.0], (10.0, 20.0), (0.0, 1.5), RED);
    assert!(painted(&[wedge], origin, zoom).len() >= 2, "a wedge is painted in steps");
    let circle = ChartPrimitive::circle([50.0, 50.0], 5.0).filled(RED);
    assert_eq!(bounds(&painted(&[circle], origin, zoom)).center(), pos2(110.0, 120.0));

    let text = ChartPrimitive::text(10.0, 10.0, "Sales").bold().anchored(HAlign::Right, VAlign::Bottom);
    let at = bounds(&painted(&[text], origin, zoom));
    assert!((at.right() - 30.0).abs() < 1.0 && at.bottom() <= 40.5 && at.top() < at.bottom(), "text is anchored at its bottom right: {at:?}");
}
