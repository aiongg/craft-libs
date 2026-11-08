//! A canvas is an instance of the view commands: the chords and the View menu act on its view.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Size};
use craft_ui::canvas::{Canvas, NavigationOptions, ToolPath, ViewCommand};
use craft_ui::contract::ShowContainer as _;
use craft_ui::runtime::commands;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect};
use std::cell::RefCell;
use std::rc::Rc;

const DOC: Size = Size::new(2000.0, 1500.0);
const CANVAS: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));

/// How close two zooms must be to count as the same, in points per document unit.
const SAME_ZOOM: f64 = 1e-9;

/// A canvas over `DOC` whose app copies the view the canvas reports into its view, with Animated Zoom off.
fn canvas() -> (Fixture, Rc<RefCell<CanvasView>>) {
    let view = Rc::new(RefCell::new(CanvasView::new(CANVAS, Point::new(1000.0, 750.0), 0.25)));
    let held = Rc::clone(&view);
    let fixture = Fixture::new(move |ui| {
        let mut navigation = NavigationOptions::default();
        navigation.animated_zoom = false;
        let shown = *held.borrow();
        let canvas = Canvas::new("Document", ToolPath::Freehand).view(shown).document(DOC).navigation(navigation).show_at(ui, CANVAS, |_, _| {});
        if let Some(next) = canvas.output.view {
            *held.borrow_mut() = next;
        }
    });
    (fixture, view)
}

#[trial]
fn cmd_equals_zooms_a_focused_canvas_and_cmd_0_fits_it() {
    let (mut fixture, view) = canvas();
    fixture.settle();
    fixture.click_at(CANVAS.center()).settle();
    let before = *view.borrow();

    fixture.key(Key::Equals, Modifiers::COMMAND).settle();
    assert!(view.borrow().zoom > before.zoom, "Cmd+= zooms the canvas's view in: {:?}", view.borrow());

    fixture.key(Key::Num0, Modifiers::COMMAND).settle();
    let mut fitted = before;
    ViewCommand::FitOnScreen.apply(&mut fitted, DOC, fixture.ctx().pixels_per_point());
    assert!((view.borrow().zoom - fitted.zoom).abs() < SAME_ZOOM, "Cmd+0 fits the document: {:?}", view.borrow());
}

#[trial]
fn the_view_menu_zoom_in_is_enabled_while_a_canvas_is_shown() {
    let (mut fixture, _) = canvas();
    fixture.settle();
    assert!(commands::resolve(fixture.ctx(), ViewCommand::ZoomIn).enabled, "the canvas takes Zoom In");
}
