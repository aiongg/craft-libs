//! `BodyLayout` under direction: the physical layouts, measured `Row` placement and the `Grid` cursor.

use craft_behavior::ColumnWidth;
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::display::Divider;
use craft_ui::layout::{BodyLayout, RowAlign};
use craft_ui::layout_components::Grid;
use craft_ui::runtime::env::{Direction, with_env};
use craft_ui::testing::layout::{Block, body};
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const ITEM: Vec2 = Vec2::new(40.0, 10.0);

/// Runs `show` for five frames, in `direction` when given, and returns the last frame's region and rects.
fn lay_out(direction: Option<Direction>, show: impl Fn(&mut Ui) -> Vec<Rect>) -> (Rect, Vec<Rect>) {
    let mut frames = Frames::new();
    let region = Cell::new(Rect::NOTHING);
    let rects = RefCell::new(Vec::new());
    frames.idle(5, |ui| {
        region.set(ui.available_rect_before_wrap());
        *rects.borrow_mut() = match direction {
            Some(direction) => with_env(ui, |o| o.direction = Some(direction), &show),
            None => show(ui),
        };
    });
    (region.get(), rects.into_inner())
}

/// Skeleton: `frames_state::row_lays_out_horizontally_in_reading_order_and_stack_vertically`.
#[trial]
fn row_runs_in_reading_order_and_places_its_block_from_last_frame() {
    let row = |align: RowAlign| move |ui: &mut Ui| body(ui, "r", &BodyLayout::row().align(align), &[ITEM, ITEM]);
    let (region, ltr) = lay_out(None, row(RowAlign::Start));
    assert!(ltr[0].max.x <= ltr[1].min.x && (ltr[0].center().y - ltr[1].center().y).abs() < 1.0, "side by side: {ltr:?}");
    assert!((ltr[0].min.x - region.min.x).abs() < 1.0);
    let (_, rtl) = lay_out(Some(Direction::Rtl), row(RowAlign::Start));
    assert!(rtl[1].max.x <= rtl[0].min.x && (rtl[0].max.x - region.max.x).abs() < 1.0, "RTL: the first child is on the right edge: {rtl:?}");

    let (_, end) = lay_out(None, row(RowAlign::End));
    assert!((end[1].max.x - region.max.x).abs() < 1.0, "End hugs the right edge under LTR: {end:?}");
    let (_, end_rtl) = lay_out(Some(Direction::Rtl), row(RowAlign::End));
    assert!((end_rtl[1].min.x - region.min.x).abs() < 1.0, "End is the left edge under RTL: {end_rtl:?}");
    let (_, centred) = lay_out(None, row(RowAlign::Center));
    let (left, right) = (centred[0].min.x - region.min.x, region.max.x - centred[1].max.x);
    assert!((left - right).abs() < 1.0, "Center leaves equal space: {centred:?}");
}

static COLUMNS: [ColumnWidth; 3] = [ColumnWidth::Fixed(100.0), ColumnWidth::Content, ColumnWidth::fill(1.0)];

/// Skeleton: `frames_state::grid_places_children_row_major_with_shared_column_widths`.
#[trial]
fn grid_places_children_row_major_with_shared_column_widths() {
    let grid = |long: bool| {
        move |ui: &mut Ui| {
            let wide = Vec2::new(if long { 150.0 } else { 40.0 }, 10.0);
            body(ui, "g", &BodyLayout::grid(&COLUMNS), &[ITEM, ITEM, ITEM, ITEM, wide, ITEM])
        }
    };
    let (_, ltr) = lay_out(None, grid(false));
    assert!((ltr[0].min.x - ltr[3].min.x).abs() < 1.0 && (ltr[2].min.x - ltr[5].min.x).abs() < 1.0, "a column keeps its x across rows");
    assert!(ltr[0].min.x < ltr[1].min.x && ltr[1].min.x < ltr[2].min.x, "columns run in reading order");
    assert!((ltr[1].min.x - ltr[0].min.x - 100.0).abs() < 1.0, "the Fixed column is the app's width");
    assert!(ltr[3].min.y >= ltr[0].max.y && (ltr[4].min.y - ltr[3].min.y).abs() < 1.0, "the second row lies under the first");
    let (_, long) = lay_out(None, grid(true));
    assert!(long[2].min.x > ltr[2].min.x + 100.0, "a Content column takes its widest child: {:?} -> {:?}", ltr[2], long[2]);
    let (region, rtl) = lay_out(Some(Direction::Rtl), grid(false));
    assert!(rtl[0].min.x > rtl[1].min.x && rtl[1].min.x > rtl[2].min.x, "under RTL the first column is on the right");
    assert!((rtl[0].max.x - region.max.x).abs() < 1.0);
}

static CONTENT_THEN_FIXED: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Fixed(60.0)];

/// A child that fills its width in a `Content` column measures at its content and fills the column the widest cell
/// settles on, so the next column starts inside the room.
#[trial]
fn a_fill_width_child_in_a_content_column_leaves_the_next_column_in_the_room() {
    let mut frames = Frames::new();
    let region = Cell::new(Rect::NOTHING);
    let rects = RefCell::new(Vec::new());
    frames.idle(5, |ui| {
        region.set(ui.available_rect_before_wrap());
        let mut placed = Vec::new();
        Grid::new(&CONTENT_THEN_FIXED).show(ui, |ui, _| {
            placed.push(Divider::horizontal().show(ui).rect);
            placed.push(Block::new(ITEM).show(ui).rect);
            placed.push(Block::new(ITEM).show(ui).rect);
            placed.push(Block::new(ITEM).show(ui).rect);
        });
        *rects.borrow_mut() = placed;
    });
    let rects = rects.into_inner();
    assert!(rects[1].max.x <= region.get().max.x, "the second column lies inside the room: {rects:?} in {:?}", region.get());
    assert!(!rects[0].intersect(rects[1]).is_positive(), "the filling child ends where the next column starts: {rects:?}");
}

