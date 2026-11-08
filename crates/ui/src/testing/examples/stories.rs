//! Stories of the examples: the code editor, the sheet and the track stack.

use super::code_editor::{CodeDocument, CodeEditor};
use super::sheet::{Rows, SheetGrid, SheetRow};
use super::tracks::{Clip, LANE, LOCK, MUTE, NAME, Track, TrackRow, TrackStack, Tracks};
use crate::contract::{Bind, Props as _, Show as _, ShowCollection as _};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::{CellSelection, ColumnKey, Flow};
use egui::{Pos2, Rect, Ui, Vec2};

/// The width and height of a cell in a stand-alone row story: two and a half control heights wide and one high.
fn cell(ui: &Ui) -> Vec2 {
    let height = crate::runtime::env::Env::of(ui).view().control(craft_theme::ControlSize::Md).height;
    Vec2::new(height * 2.5, height)
}

/// Side-by-side cells of `keys`, as the item step lists them for a row: screen rects, mirrored under right-to-left.
fn cells(ui: &Ui, keys: &[ColumnKey]) -> Vec<(ColumnKey, Rect)> {
    let size = cell(ui);
    let rtl = crate::runtime::env::Env::of(ui).view().direction().is_rtl();
    let row = Rect::from_min_size(Pos2::ZERO, Vec2::new(size.x * keys.len() as f32, size.y));
    let flow = Flow::new(row, rtl);
    keys.iter().enumerate().map(|(at, key)| (key.clone(), flow.span(size.x * at as f32, size.x))).collect()
}

/// The seconds the story lanes show.
const VISIBLE: f32 = 10.0;

#[story(CodeEditor<'static>, covers = [Default])]
fn code_editor(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let doc = data.value("doc", CodeDocument::new("fn main() {\n    run();\n}"));
    CodeEditor::new(doc).label("Source").show(ui).into()
}

#[story(CodeEditor<'static>, covers = [LongText])]
fn code_editor_long(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let doc = data.value("doc", CodeDocument::new(&format!("let total = {};", "value + ".repeat(40))));
    CodeEditor::new(doc).label("Source").show(ui).into()
}

fn sheet_rows(count: usize) -> Vec<Vec<String>> {
    (0..count).map(|row| (0..4).map(|column| format!("{}", row * 4 + column)).collect()).collect()
}

fn sheet(ui: &mut Ui, data: &mut StoryData, count: usize) -> StoryOutput {
    sheet_of(ui, data, sheet_rows(count))
}

fn sheet_of(ui: &mut Ui, data: &mut StoryData, initial: Vec<Vec<String>>) -> StoryOutput {
    let rows = data.value("rows", initial.clone()).clone();
    let out = SheetGrid::new(Bind::bound(data.value("cells", CellSelection::<usize>::default())), &rows).label("Sheet").show_items(
        ui,
        &Rows(rows.len()),
        &mut |key, icx| SheetRow::new(&rows[*key], icx.cells().to_vec()),
    );
    if let Some(edit) = out.committed {
        data.value("rows", initial)[edit.row][edit.column] = edit.text;
    }
    out.collection.into()
}

#[story(SheetGrid<'static>, covers = [Empty], drive = Drive::Never)]
fn sheet_empty(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    sheet(ui, data, 0)
}

#[story(SheetGrid<'static>, covers = [One], drive = Drive::Never)]
fn sheet_one(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    sheet(ui, data, 1)
}

#[story(SheetGrid<'static>, covers = [LongText], drive = Drive::Never)]
fn sheet_long_text(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut rows = sheet_rows(2);
    rows[0][0] = "a cell text far wider than its column ".repeat(4);
    sheet_of(ui, data, rows)
}

#[story(SheetGrid<'static>, covers = [Default, Many], drive = Drive::Never)]
fn sheet_many(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    sheet(ui, data, 12)
}

#[story(SheetRow<'static>, covers = [Default], drive = Drive::Never)]
fn sheet_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let values = sheet_rows(1).remove(0);
    SheetRow::new(&values, cells(ui, &[ColumnKey::new("a"), ColumnKey::new("b"), ColumnKey::new("c"), ColumnKey::new("d")]))
        .place(0, 1)
        .label("Row 1")
        .show(ui)
        .into()
}

fn tracks(count: usize) -> Vec<Track> {
    (0..count).map(|id| Track::new(id, format!("Track {}", id + 1)).clips(vec![Clip::new(1, 1.0, 3.0), Clip::new(2, 5.0, 2.0)])).collect()
}

fn stack(ui: &mut Ui, count: usize) -> StoryOutput {
    let tracks = tracks(count);
    TrackStack::new("Tracks").show_items(ui, &Tracks(&tracks), &mut |key, icx| TrackRow::new(&tracks[*key], icx.cells().to_vec(), VISIBLE)).into()
}

#[story(TrackStack, covers = [Empty], drive = Drive::Never)]
fn track_stack_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    stack(ui, 0)
}

#[story(TrackStack, covers = [One], drive = Drive::Never)]
fn track_stack_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    stack(ui, 1)
}

#[story(TrackStack, covers = [Default, Many], drive = Drive::Never)]
fn track_stack_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    stack(ui, 8)
}

#[story(TrackRow<'static>, covers = [Default], drive = Drive::Never)]
fn track_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let tracks = tracks(1);
    TrackRow::new(&tracks[0], cells(ui, &[NAME, MUTE, LOCK, LANE]), VISIBLE).show(ui).into()
}
