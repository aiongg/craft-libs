//! Stories of painting symmetry: the menu, the guides and the handles of Transform Symmetry.

use super::handles::{CENTRE, SYMMETRY};
use super::{Symmetry, SymmetryChoice, SymmetryHandles, SymmetryMenu, SymmetryMode, SymmetryOverlay, SymmetryPathRow};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Size};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// The page the guides run over, in document units.
const PAGE: Size = Size::new(400.0, 300.0);

/// The symmetry of the stories: Dual Axis at the page centre, turned a little.
fn sample(mode: SymmetryMode) -> Symmetry {
    Symmetry { mode, center: Point::new(200.0, 150.0), rotation: 20.0 }
}

/// A view that shows the whole page at its origin.
fn view(room: egui::Rect) -> CanvasView {
    CanvasView::new(room, Point::new(PAGE.width / 2.0, PAGE.height / 2.0), 1.0)
}

#[story(SymmetryMenu<'static>, covers = [Default], drive = Drive::Never)]
fn symmetry_off(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let choice = SymmetryChoice::Off;
    SymmetryMenu::new("Symmetry", &choice).show(ui);
    StoryOutput::default()
}

#[story(SymmetryMenu<'static>, covers = [Default], drive = Drive::Never)]
fn symmetry_on(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let choice = SymmetryChoice::Mode(SymmetryMode::Dual);
    let paths = [SymmetryPathRow::new("work", "Work Path")];
    SymmetryMenu::new("Symmetry", &choice).paths(&paths).can_transform(true).show(ui);
    StoryOutput::default()
}

#[story(SymmetryOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn dual_axis(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let symmetry = sample(SymmetryMode::Dual);
    let room = ui.available_rect_before_wrap();
    Canvas::new("Document").view(view(room)).show(ui, |ui, body| {
        SymmetryOverlay::new("Symmetry", &symmetry, PAGE).show_at(ui, body.rect);
    });
    StoryOutput::default()
}

#[story(SymmetryOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn editing_guides(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let symmetry = sample(SymmetryMode::Vertical);
    let room = ui.available_rect_before_wrap();
    Canvas::new("Document").view(view(room)).show(ui, |ui, body| {
        SymmetryOverlay::new("Symmetry", &symmetry, PAGE).editing(true).show_at(ui, body.rect);
    });
    StoryOutput::default()
}

/// Drags the centre handle.
fn drag_the_centre(fixture: &mut Fixture) {
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    let centre = fixture.root_id().and_then(|id| fixture.part(id, SYMMETRY.at(CENTRE)));
    if let Some(centre) = centre {
        fixture.drag(&centre, Vec2::new(travel, 0.0));
    }
}

#[story(SymmetryHandles<'static>, covers = [Default], drive = Drive::Custom(drag_the_centre))]
fn handles_on_a_canvas(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let symmetry = data.value("symmetry", sample(SymmetryMode::Vertical));
    let room = ui.available_rect_before_wrap();
    let shown = Canvas::new("Document")
        .view(view(room))
        .show(ui, |ui, body| SymmetryHandles::new("Symmetry", Bind::bound(&mut *symmetry), PAGE).show_at(ui, body.rect));
    shown.inner.map(|output| StoryOutput::from(output.edit)).unwrap_or_default()
}
