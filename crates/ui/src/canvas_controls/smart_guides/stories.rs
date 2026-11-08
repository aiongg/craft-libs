//! Stories of `SmartGuides`.

use super::{Measure, SmartGuides, SmartLine};
use crate::contract::Show as _;
use crate::testing::canvas_story::on_canvas;
use crate::testing::{StoryData, StoryOutput, story};
use craft_behavior::kurbo::{Point, Rect as DocRect};
use egui::Ui;

fn lines() -> Vec<SmartLine> {
    vec![SmartLine::new(Point::new(60.0, 10.0), Point::new(60.0, 110.0)), SmartLine::new(Point::new(60.0, 110.0), Point::new(140.0, 110.0)).dashed()]
}

#[story(SmartGuides<'static>, covers = [Default])]
fn alignment_lines(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let lines = lines();
    on_canvas(ui, 1.0, |ui, at, _| SmartGuides::new(&lines, &[]).show_at(ui, at))
}

#[story(SmartGuides<'static>, covers = [Default])]
fn distances_and_bounds(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let lines = lines();
    let measures = [Measure::new(Point::new(60.0, 60.0), Point::new(140.0, 60.0))];
    on_canvas(ui, 1.0, |ui, at, _| SmartGuides::new(&lines, &measures).bounds(DocRect::new(60.0, 10.0, 140.0, 110.0)).show_at(ui, at))
}
