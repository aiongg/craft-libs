//! `IconPen` geometry.
use egui::{Color32, Context, Painter, Rect, pos2};

use super::pen::{IconPen, arc_points};

// `at(12, 12)` is the rect's centre and `scale()` its width over 24.
#[test]
fn units_map_onto_the_rect() {
    let ctx = Context::default();
    let painter = Painter::new(ctx, egui::LayerId::background(), Rect::EVERYTHING);
    let rect = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(48.0, 48.0));
    let pen = IconPen::new(&painter, rect, 1.5, Color32::WHITE);
    assert_eq!(pen.at(12.0, 12.0), rect.center());
    assert_eq!(pen.at(0.0, 0.0), rect.min);
    assert_eq!(pen.scale(), 2.0);
}

// `arc_points` gives 7 to 49 points, the first and last at the arc's ends.
#[test]
fn arc_points_are_bounded() {
    for (r, a0, a1) in [(0.1, 0.0, 0.1), (3.0, 0.0, 3.0), (100.0, 0.0, 6.0)] {
        let points = arc_points(5.0, 5.0, r, a0, a1);
        assert!((7..=49).contains(&points.len()), "{}", points.len());
        let end = |a: f32| pos2(5.0 + r * a.cos(), 5.0 + r * a.sin());
        assert!((points[0] - end(a0)).length() < 1e-4);
        assert!((points[points.len() - 1] - end(a1)).length() < 1e-4);
    }
}
