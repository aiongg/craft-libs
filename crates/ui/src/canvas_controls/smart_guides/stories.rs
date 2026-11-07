//! Stories of `SmartGuides`.

use super::{Measure, SmartGuides, SmartLine};
use crate::canvas::{Canvas, ToolPath};
use crate::contract::{Show as _, ShowContainer as _};
use crate::testing::{StoryData, StoryOutput, story};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use egui::Ui;

const MOVE: ToolPath = ToolPath::Command("the Move command");

/// Shows `smart` over a canvas whose view has the document origin at its top left.
fn on_canvas(ui: &mut Ui, smart: SmartGuides<'_>) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut smart = Some(smart);
    let shown = Canvas::new("Document", MOVE).view(view).show(ui, |ui, body| smart.take().map(|smart| smart.show_at(ui, body.rect)));
    shown.inner.flatten().map(StoryOutput::from).unwrap_or_default()
}

fn lines() -> Vec<SmartLine> {
    vec![SmartLine::new(Point::new(60.0, 10.0), Point::new(60.0, 110.0)), SmartLine::new(Point::new(60.0, 110.0), Point::new(140.0, 110.0)).dashed()]
}

#[story(SmartGuides<'static>, covers = [Default])]
fn alignment_lines(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let lines = lines();
    on_canvas(ui, SmartGuides::new(&lines, &[]))
}

#[story(SmartGuides<'static>, covers = [Default])]
fn distances_and_bounds(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let lines = lines();
    let measures = [Measure::new(Point::new(60.0, 60.0), Point::new(140.0, 60.0), "80 px")];
    on_canvas(ui, SmartGuides::new(&lines, &measures).bounds(DocRect::new(60.0, 10.0, 140.0, 110.0)))
}
