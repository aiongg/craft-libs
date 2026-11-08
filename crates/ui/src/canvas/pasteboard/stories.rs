//! Stories of the pasteboard.

use super::{NAME, PageBorder, PageFrame, Pasteboard, PasteboardColor};
use crate::contract::Show as _;
use crate::style::DataColor;
use crate::testing::canvas_story::{on_canvas, on_canvas_view, origin_view};
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::kurbo::Rect as DocRect;
use egui::{Color32, Ui};

/// Degrees the turned story rotates the view by.
const TURN: f64 = 15.0;

fn boards() -> Vec<PageFrame> {
    vec![
        PageFrame::new(DocRect::new(40.0, 40.0, 240.0, 200.0)).name("Artboard 1").active(true),
        PageFrame::new(DocRect::new(280.0, 40.0, 440.0, 160.0)).name("Artboard 2").active(false).fill(DataColor::new(Color32::from_gray(u8::MAX))),
    ]
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
    on_canvas(ui, 1.0, |ui, at, _| Pasteboard::new("Pasteboard", pages).show_at(ui, at))
}

#[story(Pasteboard<'static>, covers = [Default], drive = Drive::Never)]
fn page_with_a_line(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let pages: &[PageFrame] = data.value("pages", vec![PageFrame::new(DocRect::new(60.0, 40.0, 300.0, 220.0))]);
    on_canvas(ui, 1.0, |ui, at, _| Pasteboard::new("Pasteboard", pages).color(PasteboardColor::DarkGray).border(PageBorder::Line).show_at(ui, at))
}

#[story(Pasteboard<'static>, covers = [Default], drive = Drive::Never)]
fn turned_view(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let pages: &[PageFrame] = data.value("pages", boards());
    on_canvas_view(ui, |room| origin_view(room, 1.0).rotation(TURN), |ui, at, _| Pasteboard::new("Pasteboard", pages).show_at(ui, at))
}
