//! The canvas the stories of canvas controls show their control on.

use super::StoryOutput;
use crate::canvas::Canvas;
use crate::contract::ShowContainer as _;
use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use egui::{Rect, Ui};

/// A view of `room` that puts the document's (0, 0) at its top left, `zoom` points per document unit.
pub(crate) fn origin_view(room: Rect, zoom: f64) -> CanvasView {
    CanvasView::new(room, Point::new(f64::from(room.width()) / zoom / 2.0, f64::from(room.height()) / zoom / 2.0), zoom)
}

/// Shows `control` as the only canvas control of a canvas over the room left in `ui`, whose view is [`origin_view`]
/// at `zoom`, and returns the control's output. The control gets the canvas's rect and the view it shows.
pub(crate) fn on_canvas<R: Into<StoryOutput>>(ui: &mut Ui, zoom: f64, control: impl FnMut(&mut Ui, Rect, &CanvasView) -> R) -> StoryOutput {
    on_canvas_view(ui, |room| origin_view(room, zoom), control)
}

/// The same over the view `view` makes of the room left in `ui`.
pub(crate) fn on_canvas_view<R: Into<StoryOutput>>(
    ui: &mut Ui,
    view: impl FnOnce(Rect) -> CanvasView,
    mut control: impl FnMut(&mut Ui, Rect, &CanvasView) -> R,
) -> StoryOutput {
    let view = view(ui.available_rect_before_wrap());
    on_canvas_with(ui, Canvas::new("Document").view(view), |ui, rect| control(ui, rect, &view))
}

/// Shows `control` as the only canvas control of `canvas`, which the caller sets up, and returns the control's
/// output. The body runs once to measure and once to draw, so `control` builds the control on each call.
pub(crate) fn on_canvas_with<R: Into<StoryOutput>>(ui: &mut Ui, canvas: Canvas<'_>, mut control: impl FnMut(&mut Ui, Rect) -> R) -> StoryOutput {
    let shown = canvas.show(ui, |ui, body| control(ui, body.rect));
    shown.inner.map(Into::into).unwrap_or_default()
}
