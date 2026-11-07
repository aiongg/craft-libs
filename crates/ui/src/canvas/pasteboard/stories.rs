//! Stories of the pasteboard.

use super::{NAME, PageBorder, PageFrame, Pasteboard, PasteboardColor};
use crate::canvas::{Canvas, ToolPath};
use crate::contract::{Props as _, Show as _, ShowContainer as _};
use crate::style::DataColor;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use egui::{Color32, Ui};

/// Degrees the turned story rotates the view by.
const TURN: f64 = 15.0;

const PLACE_PAGES: ToolPath = ToolPath::Command("the New Artboard command");

fn boards() -> Vec<PageFrame> {
    vec![
        PageFrame::new(DocRect::new(40.0, 40.0, 240.0, 200.0)).name("Artboard 1").active(true),
        PageFrame::new(DocRect::new(280.0, 40.0, 440.0, 160.0)).name("Artboard 2").active(false).fill(DataColor::new(Color32::WHITE)),
    ]
}

/// The view of the canvas the story shows, with the document origin at the top left.
fn view(ui: &Ui) -> CanvasView {
    let rect = ui.available_rect_before_wrap();
    CanvasView::new(rect, Point::new(f64::from(rect.width()) / 2.0, f64::from(rect.height()) / 2.0), 1.0)
}

/// Shows `pasteboard` as the only canvas control of a canvas with `view`.
fn on_canvas<'p>(ui: &mut Ui, view: CanvasView, pasteboard: impl Fn() -> Pasteboard<'p>) -> StoryOutput {
    let shown = Canvas::new("Document", PLACE_PAGES).view(view).show(ui, |ui, body| pasteboard().show_at(ui, body.rect));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

/// Clicks the name of the first page.
fn click_first_name(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(name) = fixture.part(id, NAME.at(0)) else { return };
    fixture.click(&name);
}

#[story(Pasteboard<'static>, covers = [Default], drive = Drive::Custom(click_first_name))]
fn artboards(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let pages: &[PageFrame] = data.value("pages", boards());
    let view = view(ui);
    on_canvas(ui, view, || Pasteboard::new("Pasteboard", pages))
}

#[story(Pasteboard<'static>, covers = [Default], drive = Drive::Never)]
fn page_with_a_line(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let pages: &[PageFrame] = data.value("pages", vec![PageFrame::new(DocRect::new(60.0, 40.0, 300.0, 220.0))]);
    let view = view(ui);
    on_canvas(ui, view, || Pasteboard::new("Pasteboard", pages).color(PasteboardColor::DarkGray).border(PageBorder::Line))
}

#[story(Pasteboard<'static>, covers = [Default], drive = Drive::Never)]
fn turned_view(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let pages: &[PageFrame] = data.value("pages", boards());
    let view = view(ui).rotation(TURN);
    on_canvas(ui, view, || Pasteboard::new("Pasteboard", pages))
}
