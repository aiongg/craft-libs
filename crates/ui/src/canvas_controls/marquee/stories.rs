//! Stories of `Marquee`.

use super::{Marquee, MarqueeLook};
use crate::canvas_controls::StraightLinePreview;
use crate::contract::Show as _;
use crate::style::DataColor;
use crate::testing::canvas_story::on_canvas;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::kurbo::{Point, Rect};
use egui::{Color32, Ui};

fn selection() -> Rect {
    Rect::new(40.0, 30.0, 200.0, 140.0)
}

fn lasso() -> Vec<Point> {
    vec![Point::new(40.0, 60.0), Point::new(120.0, 20.0), Point::new(190.0, 90.0), Point::new(110.0, 150.0)]
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn ants(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at, _| Marquee::rect("Selection", selection()).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn elliptical_ants(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at, _| Marquee::ellipse("Selection", selection()).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn band(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at, _| Marquee::polygon("Selection", lasso()).look(MarqueeLook::Band).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn drawing(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at, _| Marquee::polygon("Lasso", lasso()).look(MarqueeLook::Drawing).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn contrast(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let color = DataColor::new(Color32::from_rgb(230, 70, 70));
    on_canvas(ui, 1.0, |ui, at, _| Marquee::rect("Shape", selection()).look(MarqueeLook::Contrast(color)).show_at(ui, at))
}

#[story(StraightLinePreview, covers = [Default], drive = Drive::Never)]
fn shift_click_line(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at, _| StraightLinePreview::new("Straight line", Point::new(40.0, 120.0), Point::new(200.0, 40.0)).show_at(ui, at))
}
