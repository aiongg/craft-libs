//! Stories of the layout components `Row`, `Stack` and `Grid`.

use super::{Grid, Row, Stack};
use crate::contract::{Show as _, ShowContainer as _};
use crate::testing::layout::Block;
use crate::testing::{StoryData, StoryOutput, story};
use craft_behavior::ColumnWidth;
use egui::{Ui, Vec2};

/// The size of a trial block: two control heights wide and one high.
fn cell(ui: &Ui) -> Vec2 {
    let height = crate::style::control_metrics(
        &crate::runtime::env::Env::of(ui).tokens,
        craft_theme::ControlSize::Md,
        crate::fonts::stack_facts(ui.ctx()).line_ratio,
    )
    .height;
    Vec2::new(height * 2.0, height)
}

#[story(Row, covers = [Default])]
fn row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Row::new()
        .show(ui, |ui, _| {
            Block::new(cell(ui)).show(ui);
            Block::new(cell(ui)).show(ui);
        })
        .into()
}

#[story(Stack<'static>, covers = [Default])]
fn stack(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Stack::new()
        .show(ui, |ui, _| {
            Block::new(cell(ui)).show(ui);
            Block::new(cell(ui)).show(ui);
        })
        .into()
}

#[story(Stack<'static>, covers = [])]
fn stack_footer(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Stack::new()
        .footer(|ui, _| {
            Block::new(cell(ui)).show(ui);
        })
        .show(ui, |ui, _| {
            Block::new(cell(ui)).show(ui);
        })
        .into()
}

static COLUMNS: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Fixed(60.0)];

#[story(Grid, covers = [Default])]
fn grid(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Grid::new(&COLUMNS)
        .show(ui, |ui, _| {
            for _ in 0..4 {
                Block::new(cell(ui)).show(ui);
            }
        })
        .into()
}
