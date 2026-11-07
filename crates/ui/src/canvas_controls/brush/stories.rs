//! Stories of the brush overlays: each control on a canvas with a view, as a freehand tool shows it.

use super::{BrushDimensions, BrushResize, BrushResizeOptions, FalloffPreview, TipCell, TipFalloff};
use crate::canvas::{Canvas, ToolPath};
use crate::contract::{Show as _, ShowContainer as _};
use crate::testing::{StoryData, StoryOutput, story};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use egui::{Rect, Response, Ui, Vec2};

/// Screen points per document unit.
const ZOOM: f64 = 2.0;
/// The document point at the middle of the canvas.
const CENTRE: Point = Point::new(100.0, 100.0);
/// The radius of the soft tip, in document units.
const RADIUS: f64 = 40.0;
/// The hardness of the soft tip.
const SOFT: f64 = 0.4;
/// The resize starts at this diameter and hardness, in document units and a share.
const START: BrushDimensions = BrushDimensions::new(40.0, 0.5);
/// The largest diameter the resize accepts, in document units.
const MAX_DIAMETER: f64 = 400.0;
/// How far the resize story drags the pointer from the press, in screen points.
const DRAG: Vec2 = Vec2::new(30.0, 40.0);
/// The aliased tip's cell: its corners in document units.
const CELL: DocRect = DocRect::new(96.0, 96.0, 104.0, 104.0);

/// Shows the control `control` makes as the only member of a freehand canvas that shows `CENTRE` in its middle.
fn on_canvas(ui: &mut Ui, mut control: impl FnMut(&mut Ui, Rect, &CanvasView) -> Response) -> StoryOutput {
    let view = CanvasView::new(ui.available_rect_before_wrap(), CENTRE, ZOOM);
    let shown = Canvas::new("Painting", ToolPath::Freehand).view(view).show(ui, |ui, body| control(ui, body.rect, &view));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

#[story(FalloffPreview<'static>, covers = [Default])]
fn soft_tip(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let falloff = TipFalloff::from_hardness(RADIUS, SOFT);
    on_canvas(ui, |ui, rect, _| FalloffPreview::new(&falloff, CENTRE, RADIUS).show_at(ui, rect))
}

#[story(FalloffPreview<'static>, covers = [Default])]
fn hard_tip(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let falloff = TipFalloff::hard(RADIUS);
    on_canvas(ui, |ui, rect, _| FalloffPreview::new(&falloff, CENTRE, RADIUS).show_at(ui, rect))
}

#[story(FalloffPreview<'static>, covers = [Default])]
fn resize_in_progress(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |ui, rect, view| {
        let mut resize = BrushResize::default();
        let press = view.to_screen(CENTRE);
        resize.begin(view, press, START, &BrushResizeOptions::new(MAX_DIAMETER));
        resize.track(view, press + DRAG);
        let now = resize.dimensions().unwrap_or(START);
        let falloff = TipFalloff::from_hardness(now.diameter / 2.0, now.level);
        match resize.preview(&falloff) {
            Some(preview) => preview.show_at(ui, rect),
            None => FalloffPreview::new(&falloff, CENTRE, 0.0).show_at(ui, rect),
        }
    })
}

#[story(TipCell, covers = [Default])]
fn square_cell(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |ui, rect, _| TipCell::new(CELL, false).show_at(ui, rect))
}

#[story(TipCell, covers = [Default])]
fn round_cell(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |ui, rect, _| TipCell::new(CELL, true).show_at(ui, rect))
}
