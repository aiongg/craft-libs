//! Stories of the layout components `Row`, `Stack` and `Grid`.

use super::{Grid, Row, Stack};
use crate::contract::{Show as _, ShowContainer as _};
use crate::testing::layout::Block;
use crate::testing::{StoryData, StoryOutput, story};
use craft_behavior::ColumnWidth;
use egui::{Ui, Vec2};

/// The size of a trial block, in points.
const CELL: Vec2 = Vec2::new(40.0, 20.0);

#[story(Row, covers = [Default])]
fn row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Row::new()
        .show(ui, |ui, _| {
            Block::new(CELL).show(ui);
            Block::new(CELL).show(ui);
        })
        .into()
}

#[story(Stack, covers = [Default])]
fn stack(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Stack::new()
        .show(ui, |ui, _| {
            Block::new(CELL).show(ui);
            Block::new(CELL).show(ui);
        })
        .into()
}

static COLUMNS: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Fixed(60.0)];

#[story(Grid, covers = [Default])]
fn grid(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Grid::new(&COLUMNS)
        .show(ui, |ui, _| {
            for _ in 0..4 {
                Block::new(CELL).show(ui);
            }
        })
        .into()
}
