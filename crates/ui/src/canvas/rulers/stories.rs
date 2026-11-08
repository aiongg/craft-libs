//! Stories of the rulers.

use super::{CORNER, Rulers};
use crate::canvas::{Canvas, RulerText};
use crate::contract::{Bind, Show as _, Validity};
use crate::runtime::env::{Message, UnitKind};
use crate::testing::canvas_story::on_canvas_with;
use crate::testing::pointer;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, Mixed, RulerUnits};
use egui::{Ui, Vec2};

/// How far the corner is dragged, in drag thresholds: out of the corner and onto the document.
const DRAG_THRESHOLDS: f32 = 12.0;

/// The document point the view is centred on.
const CENTRE: Point = Point::new(200.0, 150.0);

/// The least x the invalid story accepts for the zero point.
const LEAST_X: f64 = 0.0;

/// The units the menu story offers.
const UNITS: [UnitKind; 2] = [UnitKind::Pixels, UnitKind::Points];

fn pixels() -> RulerText {
    RulerText::new(RulerUnits::pixels(), UnitKind::Pixels)
}

fn drag_corner(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(corner) = fixture.part(id, CORNER) else { return };
    let travel = DRAG_THRESHOLDS * crate::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&corner, Vec2::splat(travel));
}

/// Opens the units menu with a secondary click on the corner.
fn open_units_menu(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(corner) = fixture.part(id, CORNER) else { return };
    let at = corner.center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.idle(2);
}

/// A canvas whose view lies inside the strips and whose rulers count from `zero` in pixels.
fn rulers_canvas(ui: &Ui, zero: Point) -> Canvas<'static> {
    let text = pixels();
    Canvas::new("Document").view(CanvasView::new(ui.available_rect_before_wrap(), CENTRE, 1.0)).show_rulers(true).rulers(text, text, zero)
}

#[story(Rulers<'static>, covers = [Default], drive = Drive::Custom(drag_corner))]
fn zero_point(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Point::ZERO);
    let canvas = rulers_canvas(ui, *zero);
    on_canvas_with(ui, canvas, |ui, at| Rulers::new("Rulers", Bind::bound(&mut *zero)).show_at(ui, at))
}

#[story(Rulers<'static>, covers = [Open], drive = Drive::Custom(open_units_menu))]
fn units_menu(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Point::ZERO);
    let canvas = rulers_canvas(ui, *zero);
    on_canvas_with(ui, canvas, |ui, at| Rulers::new("Rulers", Bind::bound(&mut *zero)).units(&UNITS).show_at(ui, at))
}

#[story(Rulers<'static>, covers = [Mixed], drive = Drive::Custom(drag_corner))]
fn zero_points_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Mixed::<Point>::Mixed);
    let canvas = rulers_canvas(ui, Point::ZERO);
    on_canvas_with(ui, canvas, |ui, at| Rulers::new("Rulers", Bind::bound_mixed(&mut *zero)).show_at(ui, at))
}

#[story(Rulers<'static>, covers = [Invalid], drive = Drive::Custom(drag_corner))]
fn zero_point_left_of_the_document(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let zero = data.value("zero", Point::new(LEAST_X - 1.0, 0.0));
    let canvas = rulers_canvas(ui, *zero);
    on_canvas_with(ui, canvas, |ui, at| {
        let inside =
            |zero: &Point| if zero.x < LEAST_X { Validity::Refuse(Message::new("The zero point is left of the document")) } else { Validity::Ok };
        Rulers::new("Rulers", Bind::bound(&mut *zero).check(inside)).show_at(ui, at)
    })
}
