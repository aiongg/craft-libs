//! `Canvas` (20.6): a press away from every canvas control's parts reaches the tool.

mod readout;
mod tool;

// canvas-navigate-a
mod navigate;
mod tools;

use craft_ui::canvas::{Canvas, CropOverlay};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;
use std::rc::Rc;

mod space;

mod pasteboard;
mod split;

mod raster;

mod rulers;
mod scrollbars;

// canvas-navigator-a
mod navigator;
// End of canvas-navigator-a.

// l4e-navpanel
mod navigator_panel;
mod view_commands;
// End of l4e-navpanel.

// l4f-fcanvas
mod document_canvas;
// End of l4f-fcanvas.

fn crop() -> Rect {
    Rect::from_min_max(Pos2::new(20.0, 20.0), Pos2::new(100.0, 80.0))
}

#[trial]
fn a_press_on_a_crop_corner_stays_with_the_crop_and_one_elsewhere_reaches_the_tool() {
    let shared = Rc::new(RefCell::new(crop()));
    let clicked = Rc::new(RefCell::new(None));
    let (shared_crop, shared_click) = (Rc::clone(&shared), Rc::clone(&clicked));
    let mut fixture = Fixture::new(move |ui| {
        let canvas =
            Canvas::new("Document").show(ui, |ui, body| CropOverlay::new("Crop", Bind::bound(&mut *shared_crop.borrow_mut())).show_at(ui, body.rect));
        if let Some(at) = canvas.output.click_at() {
            *shared_click.borrow_mut() = Some(at);
        }
    });
    fixture.settle();

    let corner = fixture.by_label("Crop corner, top left").expect("the crop shows its corners");
    fixture.click(&corner);
    assert_eq!(*clicked.borrow(), None, "a click on a corner is the crop's, not the tool's");

    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the canvas has a node");
    fixture.click(&canvas);
    let at = clicked.borrow().expect("a click away from the corners reaches the tool");
    assert!(canvas.rect.contains(at), "the tool gets the point inside the canvas");
    assert_eq!(*shared.borrow(), crop(), "clicks move no corner");
}

#[trial]
fn an_arrow_on_a_focused_crop_corner_nudges_it() {
    let shared = Rc::new(RefCell::new(crop()));
    let shown = Rc::clone(&shared);
    let mut fixture = Fixture::new(move |ui| {
        let _ = Canvas::new("Document").show(ui, |ui, body| CropOverlay::new("Crop", Bind::bound(&mut *shown.borrow_mut())).show_at(ui, body.rect));
    });
    fixture.settle();
    let corner = fixture.by_label("Crop corner, top left").expect("the crop shows its corners");
    fixture.click(&corner);
    let before = *shared.borrow();
    fixture.key(egui::Key::ArrowRight, egui::Modifiers::NONE);
    let after = *shared.borrow();
    assert!(after.min.x > before.min.x && after.min.y == before.min.y, "the focused corner moved right: {before:?} -> {after:?}");
}
