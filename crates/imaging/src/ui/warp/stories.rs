//! Stories of the warp grid, its options-bar group and the mesh lines.

use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{CanvasView, Mixed};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, pointer, story};
use egui::{Ui, Vec2};

use super::{MeshLines, POINT, WarpFieldValues, WarpFields, WarpGrid, WarpGridChoice, WarpMesh, WarpSplit, WarpStyleEntry};

/// The picture the mesh starts over, in document units.
const PICTURE: DocRect = DocRect::new(40.0, 30.0, 360.0, 270.0);

/// The most patches across the invalid story accepts.
const MOST_COLUMNS: usize = 2;

/// How far the drag stories move the first anchor, in points.
fn pull() -> Vec2 {
    Vec2::X * 24.0 + Vec2::Y * 16.0
}

/// Shows `control` as the only canvas control of a canvas whose view has the document origin at its top left.
fn on_canvas<O>(ui: &mut Ui, control: impl FnOnce(&mut Ui, egui::Rect) -> O) -> Option<O> {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut control = Some(control);
    Canvas::new("Document").view(view).show(ui, |ui, body| control.take().map(|control| control(ui, body.rect))).inner.flatten()
}

fn pull_first_anchor(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(anchor) = fixture.part(id, POINT.at(0)) else { return };
    fixture.drag(&anchor, pull());
}

fn hover_middle(fixture: &mut Fixture) {
    let middle = fixture.shown().center();
    fixture.frame(vec![pointer::moved(middle)]);
}

#[story(WarpGrid<'static>, covers = [Default])]
fn one_patch_with_thirds(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mesh = data.value("mesh", WarpMesh::identity(PICTURE, 1, 1));
    on_canvas(ui, |ui, rect| WarpGrid::new("Warp", Bind::bound(&mut *mesh), PICTURE).show_at(ui, rect)).map(StoryOutput::from).unwrap_or_default()
}

#[story(WarpGrid<'static>, covers = [Default], drive = Drive::Custom(pull_first_anchor))]
fn three_by_three_after_a_drag(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mesh = data.value("mesh", WarpMesh::identity(PICTURE, 3, 3));
    on_canvas(ui, |ui, rect| WarpGrid::new("Warp", Bind::bound(&mut *mesh), PICTURE).show_at(ui, rect)).map(StoryOutput::from).unwrap_or_default()
}

#[story(WarpGrid<'static>, covers = [Default], drive = Drive::Custom(hover_middle))]
fn cross_split_over_a_patch(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mesh = data.value("mesh", WarpMesh::identity(PICTURE, 3, 3));
    on_canvas(ui, |ui, rect| WarpGrid::new("Warp", Bind::bound(&mut *mesh), PICTURE).split(WarpSplit::Cross).show_at(ui, rect))
        .map(StoryOutput::from)
        .unwrap_or_default()
}

#[story(WarpGrid<'static>, covers = [Mixed], drive = Drive::Never)]
fn meshes_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mesh = data.value("mixed", Mixed::<WarpMesh>::Mixed);
    on_canvas(ui, |ui, rect| WarpGrid::new("Warp", Bind::bound_mixed(&mut *mesh), PICTURE).show_at(ui, rect))
        .map(StoryOutput::from)
        .unwrap_or_default()
}

#[story(WarpGrid<'static>, covers = [Invalid], drive = Drive::Never)]
fn too_many_patches(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mesh = data.value("invalid", WarpMesh::identity(PICTURE, 3, 3));
    let few =
        |mesh: &WarpMesh| if mesh.columns() > MOST_COLUMNS { Validity::Refuse(Message::new("There are too many patches")) } else { Validity::Ok };
    on_canvas(ui, |ui, rect| WarpGrid::new("Warp", Bind::bound(&mut *mesh).check(few), PICTURE).show_at(ui, rect))
        .map(StoryOutput::from)
        .unwrap_or_default()
}

#[story(WarpGrid<'static>, covers = [Default])]
fn lines_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mesh = data.value("mesh", WarpMesh::identity(PICTURE, 3, 3));
    on_canvas(ui, |ui, rect| WarpGrid::new("Warp", Bind::bound(&mut *mesh), PICTURE).lines_only().show_at(ui, rect))
        .map(StoryOutput::from)
        .unwrap_or_default()
}

fn styles() -> Vec<WarpStyleEntry> {
    vec![
        WarpStyleEntry::new("none", "None"),
        WarpStyleEntry::new("custom", "Custom").custom(true),
        WarpStyleEntry::new("arc", "Arc").preset(true),
        WarpStyleEntry::new("flag", "Flag").preset(true),
    ]
}

#[story(WarpFields<'static>, covers = [Default])]
fn fields_on_a_preset_style(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let styles = styles();
    let mut values = WarpFieldValues::new("arc");
    values.bend = 50.0;
    values.grid = WarpGridChoice::Three;
    WarpFields::new(&styles, values).show(ui);
    StoryOutput::default()
}

#[story(WarpFields<'static>, covers = [Default])]
fn fields_on_a_type_layer(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let styles = styles();
    WarpFields::new(&styles, WarpFieldValues::new("none")).on_type().show(ui);
    StoryOutput::default()
}

#[story(MeshLines<'static>, covers = [Default])]
fn lines_over_a_small_grid(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let (columns, rows) = (4, 3);
    let (width, height) = (PICTURE.width() / f64::from(columns), PICTURE.height() / f64::from(rows));
    let at = |i: i32, j: i32| Point::new(PICTURE.x0 + f64::from(i) * width, PICTURE.y0 + f64::from(j) * height + f64::from(i) * 4.0);
    let mut edges = Vec::new();
    for j in 0..=rows {
        for i in 0..=columns {
            if i < columns {
                edges.push([at(i, j), at(i + 1, j)]);
            }
            if j < rows {
                edges.push([at(i, j), at(i, j + 1)]);
            }
        }
    }
    on_canvas(ui, |ui, rect| MeshLines::new(&edges).show_at(ui, rect)).map(StoryOutput::from).unwrap_or_default()
}
