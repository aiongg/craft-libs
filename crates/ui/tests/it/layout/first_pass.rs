//! Rows that place their block and grids that size their columns do it in the pass that shows them first.

use craft_behavior::ColumnWidth;
use craft_ui::buttons::Button;
use craft_ui::contract::{RowAlign, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::testing::{Frames, trial};
use egui::Rect;
use std::cell::Cell;

const SLACK: f32 = 0.5;

#[trial]
fn an_end_row_ends_at_the_edge_in_its_first_pass() {
    let mut frames = Frames::new();
    let seen = Cell::new((Rect::NOTHING, Rect::NOTHING));
    let passes = frames.frame(Vec::new(), |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            let mut last = Rect::NOTHING;
            let row = Row::new().fill_width().align(RowAlign::End).show(ui, |ui, _| {
                let _ = Button::new("Cancel").show(ui);
                last = Button::new("Apply").show(ui).rect;
            });
            seen.set((row.output.rect, last));
        });
    });
    let (row, last) = seen.get();
    assert_eq!(passes, 1, "the frame took one pass");
    assert!((last.max.x - row.max.x).abs() <= SLACK, "the last button ends at the row's end edge: {last:?} in {row:?}");
}

#[trial]
fn a_grid_sizes_its_columns_in_its_first_pass() {
    static COLUMNS: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Content];
    const GAP: f32 = 0.0;
    let mut frames = Frames::new();
    let seen = Cell::new([Rect::NOTHING; 4]);
    let passes = frames.frame(Vec::new(), |ui| {
        Grid::new(&COLUMNS).show(ui, |ui, _| {
            let rects = [
                Text::new("Short").show(ui).rect,
                Text::new("Second column").show(ui).rect,
                Text::new("A much longer first column label").show(ui).rect,
                Text::new("Again").show(ui).rect,
            ];
            seen.set(rects);
        });
    });
    let rects = seen.get();
    let widest = rects[0].width().max(rects[2].width());
    assert_eq!(passes, 1, "the frame took one pass");
    assert!(rects[1].min.x >= rects[0].min.x + widest + GAP - SLACK, "column 1 starts after the widest label of column 0: {rects:?}");
    assert!((rects[1].min.x - rects[3].min.x).abs() <= SLACK, "column 1 is aligned: {rects:?}");
}

#[trial]
fn a_spread_row_spreads_in_its_first_pass() {
    let mut frames = Frames::new();
    let seen = Cell::new((Rect::NOTHING, Rect::NOTHING, Rect::NOTHING));
    let _ = frames.frame(Vec::new(), |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            let mut rects = [Rect::NOTHING; 3];
            let row = Row::new().fill_width().align(RowAlign::SpaceBetween).show(ui, |ui, _| {
                for (i, label) in ["One", "Two", "Three"].into_iter().enumerate() {
                    rects[i] = Button::new(label).show(ui).rect;
                }
            });
            seen.set((row.output.rect, rects[0], rects[2]));
        });
    });
    let (row, first, last) = seen.get();
    assert!((first.min.x - row.min.x).abs() <= SLACK, "the first button is at the start edge: {first:?} in {row:?}");
    assert!((last.max.x - row.max.x).abs() <= SLACK, "the last button is at the end edge: {last:?} in {row:?}");
}
