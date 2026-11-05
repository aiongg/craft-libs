//! Page grids.

use super::grid::*;

// Start -30, increment 12 in a 50 pt page gives 6, 18, 30, 42; an increment of 0.2 counts as 1; never more than MAX_BASELINES lines.
#[test]
fn baseline_lines_start_inside_the_page() {
    assert_eq!(baseline_lines(-30.0, 12.0, 50.0), [6.0, 18.0, 30.0, 42.0]);
    assert_eq!(baseline_lines(0.0, 0.2, 3.5), [0.0, 1.0, 2.0, 3.0]);
    assert_eq!(baseline_lines(0.0, 1.0, 1.0e9).len(), MAX_BASELINES);
    assert!(baseline_lines(f64::NAN, 12.0, 50.0).is_empty());
}

// The baseline grid follows the page view's zoom, not the scale of the view that maps page points to the screen: at 50 % it is hidden below the 75 % threshold although that view's scale is 2, and at 100 % its two lines show.
#[test]
fn baselines_follow_the_page_zoom() {
    use egui::{Context, LayerId, RawInput, Shape};
    use kurbo::{Point, Rect};

    use craft_ui::canvas::CanvasView;
    use craft_ui::theme::current;

    use super::view::PageViewState;

    let ctx = Context::default();
    let colors = PageGrids::style(&current(&ctx));
    let page = Rect::new(0.0, 0.0, 100.0, 100.0);
    let grid = [PageGrid { page, margins: Rect::new(10.0, 10.0, 90.0, 90.0), columns: Vec::new(), baseline: Some((0.0, 50.0)) }];
    let view = CanvasView::new(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0)), Point::new(50.0, 50.0), 2.0);
    let lines = |zoom: f64| {
        let mut state = PageViewState::default();
        state.zoom = zoom;
        let mut output =
            ctx.run_pass(RawInput::default(), |ctx| PageGrids::new(&view, &state, &grid, colors).paint(&ctx.layer_painter(LayerId::background())));
        output.textures_delta.clear();
        output.shapes.iter().filter(|clipped| matches!(clipped.shape, Shape::LineSegment { .. })).count()
    };
    assert_eq!(lines(0.5), 0);
    assert_eq!(lines(1.0), 2);
}
