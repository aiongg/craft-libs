//! Stories of `Marquee`, and the canvas the stories of the other selection outlines show their control on.

use super::{Marquee, MarqueeLook};
use crate::canvas::Canvas;
use crate::canvas_controls::StraightLinePreview;
use crate::contract::{Show as _, ShowContainer as _};
use crate::style::DataColor;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect};
use egui::{Color32, Response, Ui};

/// Shows `control` as the only canvas control of a canvas whose view puts document (0, 0) at the canvas's top left,
/// `zoom` points per document unit, and returns the control's output.
pub(in crate::canvas_controls) fn on_canvas(ui: &mut Ui, zoom: f64, mut control: impl FnMut(&mut Ui, egui::Rect) -> Response) -> StoryOutput {
    let at = ui.available_rect_before_wrap();
    let center = Point::new(f64::from(at.width()) / zoom / 2.0, f64::from(at.height()) / zoom / 2.0);
    let view = CanvasView::new(at, center, zoom);
    let shown = Canvas::new("Document").view(view).show(ui, |ui, body| control(ui, body.rect));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

fn selection() -> Rect {
    Rect::new(40.0, 30.0, 200.0, 140.0)
}

fn lasso() -> Vec<Point> {
    vec![Point::new(40.0, 60.0), Point::new(120.0, 20.0), Point::new(190.0, 90.0), Point::new(110.0, 150.0)]
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn ants(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| Marquee::rect("Selection", selection()).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn elliptical_ants(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| Marquee::ellipse("Selection", selection()).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn band(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| Marquee::polygon("Selection", lasso()).look(MarqueeLook::Band).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn drawing(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| Marquee::polygon("Lasso", lasso()).look(MarqueeLook::Drawing).show_at(ui, at))
}

#[story(Marquee, covers = [Default], drive = Drive::Never)]
fn contrast(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let color = DataColor::new(Color32::from_rgb(230, 70, 70));
    on_canvas(ui, 1.0, |ui, at| Marquee::rect("Shape", selection()).look(MarqueeLook::Contrast(color)).show_at(ui, at))
}

#[story(StraightLinePreview, covers = [Default], drive = Drive::Never)]
fn shift_click_line(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, 1.0, |ui, at| StraightLinePreview::new("Straight line", Point::new(40.0, 120.0), Point::new(200.0, 40.0)).show_at(ui, at))
}
