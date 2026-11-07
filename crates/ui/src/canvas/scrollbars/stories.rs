//! Stories of the scroll bars.

use super::{CanvasScrollbars, HORIZONTAL};
use crate::canvas::{Canvas, ToolPath};
use crate::contract::{Bind, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, Target, story};
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{CanvasView, Mixed};
use egui::{Rect, Ui, Vec2, vec2};

/// How far the thumb is dragged, in points.
const DRAG: f32 = 20.0;

/// Where the thumb is pressed, from the start of its track, in points: the views of the stories start at the document's
/// start, so the thumb begins there and is longer than this.
const GRIP: f32 = 8.0;

/// The document the bars scroll, in document units.
const DOCUMENT: DocRect = DocRect::new(0.0, 0.0, 800.0, 600.0);

/// The zoom that shows part of the document in the story's canvas.
const ZOOM: f64 = 2.0;

/// The view centre of the stories: the document's start, which puts each thumb at the start of its track.
const CENTRE: Point = Point::new(0.0, 0.0);

/// The view centre the invalid story refuses to leave to the right.
const LAST_CENTRE_X: f64 = 0.0;

const SCROLLS: ToolPath = ToolPath::Freehand;

fn view_over(rect: Rect) -> CanvasView {
    CanvasView::new(rect, CENTRE, ZOOM)
}

fn drag_thumb(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(track) = fixture.part(id, HORIZONTAL) else { return };
    let grip = Target { node: track.node, rect: Rect::from_center_size(track.rect.left_center() + vec2(GRIP, 0.0), Vec2::ZERO) };
    fixture.drag(&grip, Vec2::new(DRAG, 0.0));
}

/// Shows the bars over `view` as the only canvas control of a canvas, and returns their output.
fn on_canvas(ui: &mut Ui, bars: impl FnOnce(&mut Ui, Rect) -> super::ScrollbarsOutput) -> StoryOutput {
    let full = ui.available_rect_before_wrap();
    Canvas::new("Document", SCROLLS).view(view_over(full)).show(ui, |ui, body| bars(ui, body.rect)).inner.map(StoryOutput::from).unwrap_or_default()
}

#[story(CanvasScrollbars<'static>, covers = [Default], drive = Drive::Custom(drag_thumb))]
fn scrolled_view(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let full = ui.available_rect_before_wrap();
    let view = data.value("view", view_over(full));
    on_canvas(ui, |ui, at| CanvasScrollbars::new("Scroll bars", Bind::bound(&mut *view), DOCUMENT).show_at(ui, at))
}

#[story(CanvasScrollbars<'static>, covers = [Mixed], drive = Drive::Custom(drag_thumb))]
fn views_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", Mixed::<CanvasView>::Mixed);
    on_canvas(ui, |ui, at| CanvasScrollbars::new("Scroll bars", Bind::bound_mixed(&mut *view), DOCUMENT).show_at(ui, at))
}

#[story(CanvasScrollbars<'static>, covers = [Invalid], drive = Drive::Custom(drag_thumb))]
fn view_scrolled_too_far(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let full = ui.available_rect_before_wrap();
    let view = data.value("view", view_over(full));
    on_canvas(ui, |ui, at| {
        let kept = |view: &CanvasView| {
            if view.center.x > LAST_CENTRE_X { Validity::Refuse(Message::new("The view cannot scroll further right")) } else { Validity::Ok }
        };
        CanvasScrollbars::new("Scroll bars", Bind::bound(&mut *view).check(kept), DOCUMENT).show_at(ui, at)
    })
}
