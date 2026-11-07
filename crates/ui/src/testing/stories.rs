//! Stories of the library's own layout types.

use super::{StoryData, StoryOutput, story};
use crate::contract::{Show as _, ShowContainer as _};
use crate::layout_components::{Grid, Row, Stack};
use crate::sections::FooterBar;
use crate::testing::layout::Block;
use craft_behavior::ColumnWidth;
use egui::{Ui, Vec2};

/// The size of a trial block, in points.
const CELL: Vec2 = Vec2::new(40.0, 20.0);

#[story(Block, covers = [Default])]
fn block(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Block::new(CELL).show(ui).into()
}

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

#[story(FooterBar, covers = [Default])]
fn footer_bar(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    FooterBar::body()
        .show(ui, |ui, _| {
            Block::new(CELL).show(ui);
        })
        .into()
}
