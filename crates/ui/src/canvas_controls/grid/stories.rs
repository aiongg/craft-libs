//! Stories of `CanvasGrid`.

use super::{CanvasGrid, GridLine};
use crate::canvas_controls::marquee::stories::on_canvas;
use crate::contract::Show as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::kurbo::Vec2;
use egui::Ui;

const MAJOR: f64 = 100.0;
const SUBDIVISIONS: u32 = 4;

fn grid() -> CanvasGrid {
    CanvasGrid::new("Grid").spacing(Vec2 { x: MAJOR, y: MAJOR }).subdivisions(SUBDIVISIONS)
}

#[story(CanvasGrid, covers = [Default], drive = Drive::Never)]
fn solid(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| grid().show_at(ui, at))
}

#[story(CanvasGrid, covers = [Default], drive = Drive::Never)]
fn dashed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| grid().line(GridLine::Dashed).show_at(ui, at))
}

#[story(CanvasGrid, covers = [Default], drive = Drive::Never)]
fn dots(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| grid().line(GridLine::Dots).show_at(ui, at))
}

#[story(CanvasGrid, covers = [Default], drive = Drive::Never)]
fn crossings(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| grid().line(GridLine::Crossings).show_at(ui, at))
}
