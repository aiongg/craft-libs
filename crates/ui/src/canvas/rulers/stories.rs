//! Stories of the rulers.

use super::{CORNER, Rulers};
use crate::canvas::{Canvas, RulerText, ToolPath};
use crate::contract::{Bind, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::{Env, Message, UnitKind};
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, Mixed, RulerUnits};
use egui::{Rect, Ui, Vec2};

/// How far the corner is dragged, in points: out of the corner and onto the document.
const DRAG: f32 = 80.0;

/// The document point the view is centred on.
const CENTRE: Point = Point::new(200.0, 150.0);

/// The least x the invalid story accepts for the zero point.
const LEAST_X: f64 = 0.0;

const CANVAS_AT: ToolPath = ToolPath::Command("the New Guide... command");

fn pixels() -> RulerText {
    RulerText::new(RulerUnits::pixels(), UnitKind::Pixels)
}

fn drag_corner(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(corner) = fixture.part(id, CORNER) else { return };
    fixture.drag(&corner, Vec2::splat(DRAG));
}

/// Shows `rulers` as the only canvas control of a canvas whose view lies inside the strips.
fn on_canvas(ui: &mut Ui, mut rulers: impl FnMut(&mut Ui, Rect) -> super::RulerOutput) -> StoryOutput {
    let full = ui.available_rect_before_wrap();
    let content = Rulers::content_rect(&Env::of(ui).tokens, full);
    let view = CanvasView::new(content, CENTRE, 1.0);
    Canvas::new("Document", CANVAS_AT).view(view).show(ui, |ui, body| rulers(ui, body.rect)).inner.map(StoryOutput::from).unwrap_or_default()
}

#[story(Rulers<'static>, covers = [Default], drive = Drive::Custom(drag_corner))]
fn zero_point(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Point::ZERO);
    let text = pixels();
    on_canvas(ui, |ui, at| Rulers::new("Rulers", &text, &text, Bind::bound(&mut *zero)).show_at(ui, at))
}

#[story(Rulers<'static>, covers = [Mixed], drive = Drive::Custom(drag_corner))]
fn zero_points_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Mixed::<Point>::Mixed);
    let text = pixels();
    on_canvas(ui, |ui, at| Rulers::new("Rulers", &text, &text, Bind::bound_mixed(&mut *zero)).show_at(ui, at))
}

#[story(Rulers<'static>, covers = [Invalid], drive = Drive::Custom(drag_corner))]
fn zero_point_left_of_the_document(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Point::new(LEAST_X - 1.0, 0.0));
    let text = pixels();
    on_canvas(ui, |ui, at| {
        let inside =
            |zero: &Point| if zero.x < LEAST_X { Validity::Refuse(Message::new("The zero point is left of the document")) } else { Validity::Ok };
        Rulers::new("Rulers", &text, &text, Bind::bound(&mut *zero).check(inside)).show_at(ui, at)
    })
}
