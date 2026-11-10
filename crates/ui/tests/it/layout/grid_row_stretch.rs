//! In a `Grid` row, children that fill share the row's top and bottom edges, whatever their content's height; a
//! content-sized child keeps its own height and sits at the centre of its cell.

use craft_behavior::ColumnWidth;
use craft_ui::buttons::Button;
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::layout_components::{Grid, Stack};
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROOM: Vec2 = Vec2::new(600.0, 400.0);
static TWO: [ColumnWidth; 2] = [ColumnWidth::fill(1.0), ColumnWidth::fill(1.0)];
const EDGE_SLACK: f32 = 0.5;

#[trial]
fn fill_children_of_a_grid_row_share_its_top_and_bottom_and_content_children_keep_their_height() {
    let cells = Rc::new(Cell::new([Rect::NOTHING; 4]));
    let seen = Rc::clone(&cells);
    let mut fixture = Fixture::new(move |ui| {
        let mut rects = [Rect::NOTHING; 4];
        Grid::new(&TWO).fill_width().show(ui, |ui, _| {
            let tall = Stack::new().fill_width().fill().show(ui, |ui, _| {
                for label in ["a1", "a2", "a3"] {
                    let _ = Button::new(label).show(ui);
                }
            });
            let short = Stack::new().fill().show(ui, |ui, _| {
                let _ = Button::new("b1").show(ui);
            });
            let content_tall = Stack::new().show(ui, |ui, _| {
                for label in ["c1", "c2", "c3"] {
                    let _ = Button::new(label).show(ui);
                }
            });
            let content_short = Stack::new().show(ui, |ui, _| {
                let _ = Button::new("d1").show(ui);
            });
            rects = [tall.output.rect, short.output.rect, content_tall.output.rect, content_short.output.rect];
        });
        seen.set(rects);
    });
    fixture.room(ROOM).settle();
    let [tall, short, content_tall, content_short] = cells.get();
    let dump = format!("{tall:?} {short:?} {content_tall:?} {content_short:?}");
    assert!(
        (tall.min.y - short.min.y).abs() <= EDGE_SLACK && (tall.max.y - short.max.y).abs() <= EDGE_SLACK,
        "fill children share the row's edges: {dump}"
    );
    assert!(content_short.height() < content_tall.height() - EDGE_SLACK, "a content-sized child keeps its height: {dump}");
}
