//! `ThumbPaint::paper`: the paper fills under the picture in place of the transparency grid.

use craft_ui::contract::Show as _;
use craft_ui::list::ThumbAspect;
use craft_ui::primitives::Priority;
use craft_ui::primitives::textures::{FakeThumbSource, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails};
use craft_ui::style::DataColor;
use craft_ui::testing::{Frames, trial};
use craft_ui::tile_grid::Tile;
use egui::{Color32, Rect, Shape};

const PAPER: Color32 = Color32::from_rgb(235, 220, 170);

/// The shapes of a frame in which a tile's thumbnail is painted with `paint`.
fn shapes(paint: ThumbPaint) -> Vec<Shape> {
    let mut frames = Frames::new();
    let source = FakeThumbSource::<u32>::new();
    frames.frame_shapes(|ui| {
        Tile::new("Page")
            .thumb(ThumbAspect::new(3, 2), |cx, rect| {
                let ctx = cx.painter.ctx();
                let thumbs = Thumbnails::<u32>::in_context(ctx, "paper", ThumbnailOptions::new());
                let state = thumbs.state(ctx, &source, &0, &ThumbRequest::for_cell(64.0, 1.0), Priority::VISIBLE);
                thumbs.paint(cx, rect, &state, &paint);
            })
            .show(ui);
    })
}

/// The rect of the picture, and the filled rects inside it, in paint order.
fn under_the_picture(shapes: &[Shape]) -> (Rect, Vec<Color32>) {
    let picture = shapes.iter().find_map(|s| if let Shape::Mesh(mesh) = s { Some(mesh.calc_bounds()) } else { None }).expect("the picture painted");
    let inside = shapes
        .iter()
        .filter_map(|s| if let Shape::Rect(r) = s { Some((r.rect, r.fill)) } else { None })
        .filter(|(rect, _)| picture.expand(0.5).contains_rect(*rect))
        .map(|(_, fill)| fill)
        .collect();
    (picture, inside)
}

#[trial]
fn paper_fills_under_the_picture_where_the_grid_would_be() {
    let (_, grid) = under_the_picture(&shapes(ThumbPaint::default()));
    assert!(grid.len() > 1 && !grid.contains(&PAPER), "the default paints the checkerboard's cells: {grid:?}");
    let (_, paper) = under_the_picture(&shapes(ThumbPaint::default().paper(DataColor::new(PAPER))));
    assert_eq!(paper, vec![PAPER], "the paper is the one fill under the picture and no cell of the grid");
}

