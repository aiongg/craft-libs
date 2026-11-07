//! Stories of the hover and selection outlines.

use super::{HoverMask, HoverOutline, HoverShape, SelectionOutline};
use crate::canvas_controls::marquee::stories::on_canvas;
use crate::contract::Show as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::kurbo::{BezPath, Rect, Shape as _};
use egui::Ui;

fn bounds() -> Rect {
    Rect::new(40.0, 30.0, 200.0, 140.0)
}

fn paths() -> Vec<BezPath> {
    vec![bounds().to_path(0.1), Rect::new(70.0, 50.0, 120.0, 100.0).to_path(0.1)]
}

#[story(HoverOutline, covers = [Default], drive = Drive::Never)]
fn rollover(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| HoverOutline::new("Rollover", Some(HoverShape::Rect(bounds()))).show_at(ui, at))
}

#[story(HoverOutline, covers = [Default], drive = Drive::Never)]
fn coverage_tint(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mask = HoverMask::sampled(bounds(), |x, y| if (x - 120.0).hypot(y - 85.0) < 50.0 { 1.0 } else { 0.0 });
    on_canvas(ui, 1.0, |ui, at| HoverOutline::new("Object", Some(HoverShape::Mask(mask.clone()))).show_at(ui, at))
}

#[story(SelectionOutline<'static>, covers = [Default], drive = Drive::Never)]
fn solid(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let paths = paths();
    on_canvas(ui, 1.0, |ui, at| SelectionOutline::solid("Selected paths", &paths).show_at(ui, at))
}

#[story(SelectionOutline<'static>, covers = [Default], drive = Drive::Never)]
fn dashed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let paths = paths();
    on_canvas(ui, 1.0, |ui, at| SelectionOutline::dashed("Selected paths", &paths).show_at(ui, at))
}
