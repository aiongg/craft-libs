//! Stories of `Guides`.

use super::{GUIDE, Guides, GuidesOutput};
use crate::canvas::{Canvas, Guide, GuideDrag, GuideKey, GuideLine, ToolPath};
use crate::contract::{Bind, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{Axis, CanvasView, GesturePhase, Mixed};
use craft_keymap::Mods;
use egui::{Ui, Vec2};

const NEW_GUIDE: ToolPath = ToolPath::Command("the New Guide command");

/// The most guides the invalid story accepts.
const MOST_GUIDES: usize = 2;

fn guides() -> Vec<Guide> {
    let mut dashed = Guide::new(GuideKey::new(3), Axis::X, 140.0);
    dashed.line = GuideLine::Dashed;
    let mut locked = Guide::new(GuideKey::new(4), Axis::Y, 30.0);
    locked.locked = true;
    vec![Guide::new(GuideKey::new(1), Axis::Y, 120.0), Guide::new(GuideKey::new(2), Axis::X, 80.0), dashed, locked]
}

/// Shows `control` as the only canvas control of a canvas whose view has the document origin at its top left.
fn on_canvas(ui: &mut Ui, control: Guides<'_>) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut control = Some(control);
    let shown = Canvas::new("Document", NEW_GUIDE)
        .view(view)
        .show(ui, |ui, body| -> Option<GuidesOutput> { control.take().map(|control| control.show_at(ui, body.rect)) });
    shown.inner.flatten().map(StoryOutput::from).unwrap_or_default()
}

/// Drags the first guide across itself by twice the drag threshold.
fn drag_first(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(line) = fixture.part(id, GuideKey::new(1).part(GUIDE)) else { return };
    let travel = 2.0 * crate::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&line, Vec2::new(travel, 0.0));
}

#[story(Guides<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn guides_over_a_canvas(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let guides = data.value("guides", guides());
    on_canvas(ui, Guides::new("Guides", Bind::bound(&mut *guides)).interactive(true).selected_guide(GuideKey::new(2)))
}

#[story(Guides<'static>, covers = [Mixed], drive = Drive::Custom(drag_first))]
fn guides_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let guides = data.value("guides", Mixed::<Vec<Guide>>::Mixed);
    on_canvas(ui, Guides::new("Guides", Bind::bound_mixed(&mut *guides)).interactive(true))
}

#[story(Guides<'static>, covers = [Invalid], drive = Drive::Custom(drag_first))]
fn too_many_guides(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let guides = data.value("guides", guides());
    let few = |guides: &Vec<Guide>| {
        if guides.len() > MOST_GUIDES { Validity::Refuse(Message::new("There are too many guides")) } else { Validity::Ok }
    };
    on_canvas(ui, Guides::new("Guides", Bind::bound(&mut *guides).check(few)).interactive(true))
}

#[story(Guides<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn guides_that_only_draw(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let guides = data.value("guides", guides());
    on_canvas(ui, Guides::new("Guides", Bind::bound(&mut *guides)))
}

#[story(Guides<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn guide_dragged_out_of_a_ruler(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let guides = data.value("guides", guides());
    let drag = GuideDrag::new_guide(Axis::Y, GesturePhase::Update, Some(60.0), Mods::NONE);
    on_canvas(ui, Guides::new("Guides", Bind::bound(&mut *guides)).interactive(true).pending(drag))
}

#[story(Guides<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn bounded_guides(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let guides = data.value("guides", guides());
    let page = DocRect::new(0.0, 0.0, 200.0, 150.0);
    on_canvas(ui, Guides::new("Guides", Bind::bound(&mut *guides)).interactive(true).bounds(page))
}
