//! Stories of the "+" buttons: an artboard on a pasteboard with the buttons around it, one side kept clear by a
//! neighbour's rect, and the connected-shape form.

use super::AddBesideHandles;
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::{Canvas, PageFrame, Pasteboard};
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Rect, Ui};

/// The artboard the buttons surround, in document units.
fn artboard() -> DocRect {
    DocRect::new(40.0, 40.0, 240.0, 180.0)
}

/// The view of the canvas the story shows: the document origin at the top left of the canvas.
fn view(ui: &Ui) -> CanvasView {
    let rect = ui.available_rect_before_wrap();
    CanvasView::new(rect, Point::new(f64::from(rect.width()) / 2.0, f64::from(rect.height()) / 2.0), 1.0)
}

/// Shows the artboard with `handles` over it on a canvas.
fn on_canvas<'a>(ui: &mut Ui, handles: impl Fn(Rect) -> AddBesideHandles<'a>) -> StoryOutput {
    let board = [PageFrame::new(artboard()).name("Artboard 1").active(true)];
    let view = view(ui);
    let shown = Canvas::new("Document").view(view).show(ui, |ui, body| {
        Pasteboard::new("Pasteboard", &board).show_at(ui, body.rect);
        handles(body.rect).show_at(ui, body.rect)
    });
    // The story reports the buttons' own click: a key or AccessKit activation adds a side too.
    shown.inner.map_or_else(StoryOutput::default, |out| {
        let added = out.added.is_some();
        let mut output = StoryOutput::from(out);
        output.clicked |= added;
        output
    })
}

#[story(AddBesideHandles<'static>, covers = [Default], drive = Drive::Never)]
fn add_beside(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let view = view(ui);
    let neighbour = Rect::from_two_pos(
        view.to_screen(Point::new(artboard().x1 + 16.0, artboard().y0)),
        view.to_screen(Point::new(artboard().x1 + 80.0, artboard().y1)),
    );
    let clear = [neighbour];
    on_canvas(ui, |_| AddBesideHandles::new("Add beside", artboard()).clear_of(&clear))
}

#[story(AddBesideHandles<'static>, covers = [Default], drive = Drive::Never)]
fn add_beside_connecting(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |_| AddBesideHandles::new("Add beside", artboard()).connecting())
}
