//! Tests of the aliased tip's cell on the pixel grid (screen).

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use egui::{Rect as ScreenRect, Shape, Vec2, pos2};
use egui_kittest::Harness;
use kurbo::{Point, Rect};

use super::{BrushCursor, BrushStyle, BrushTip, BrushTipOutline};
use crate::canvas::CanvasView;
use crate::theme::{Base, current, install};

fn painted(ppp: f32, round: bool) -> Vec<Shape> {
    let tip = BrushTip { diameter: 7.0, outline: BrushTipOutline::Pixels { cell: Rect::new(3.3, 4.1, 10.3, 11.1), round } };
    let harness = Harness::builder()
        .with_size([100.0, 100.0])
        .with_pixels_per_point(ppp)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui(|ui| {
            let view = CanvasView::new(ScreenRect::from_min_size(pos2(0.0, 0.0), Vec2::splat(100.0)), Point::new(50.0, 50.0), 1.7);
            BrushCursor::new(&tip).paint_pixels(ui.painter(), &view, &BrushStyle::of(&current(ui.ctx())));
        });
    let mut out = Vec::new();
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    harness.output().shapes.iter().for_each(|c| flatten(&c.shape, &mut out));
    out
}

// At 1.5 and 2 pixels per point the painted cell's edges land on `1 / ppp` multiples, square and round.
#[test]
fn pixel_tip_snaps_to_physical_pixels() {
    let on_grid = |v: f32, ppp: f32| ((v * ppp) - (v * ppp).round()).abs() < 1e-3;
    for ppp in [1.5, 2.0] {
        let squares: Vec<_> = painted(ppp, false)
            .into_iter()
            .filter_map(|s| {
                if let Shape::Rect(r) = s
                    && !r.stroke.is_empty()
                {
                    Some(r.rect)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(squares.len(), 2);
        for r in squares {
            for v in [r.min.x, r.min.y, r.max.x, r.max.y] {
                assert!(on_grid(v, ppp), "{v} at {ppp}");
            }
        }
        let circles: Vec<_> = painted(ppp, true).into_iter().filter_map(|s| if let Shape::Circle(c) = s { Some(c) } else { None }).collect();
        assert_eq!(circles.len(), 2);
        // The halo's outer edge, `centre - (r + px / 2) - px / 2`, is on the grid.
        let halo = &circles[0];
        assert!(on_grid(halo.center.x - halo.radius - 0.5 / ppp, ppp) && on_grid(halo.center.y - halo.radius - 0.5 / ppp, ppp));
    }
}
