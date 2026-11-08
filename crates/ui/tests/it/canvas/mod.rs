//! `Canvas` (20.6): a press away from every canvas control's parts reaches the tool.

mod crop;
mod outside_polygon;
mod readout;
mod tool;
mod tool_gestures;

// canvas-navigate-a
mod navigate;
mod tools;

use craft_behavior::CanvasView;
use craft_behavior::geometry::CropBox;
use craft_behavior::kurbo::{Point, Rect, Size};
use craft_ui::canvas::{Canvas, CropOverlay};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
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

// fwj-canvas
mod empty_rect;
mod tool_keys;
// End of fwj-canvas.

fn crop() -> CropBox {
    CropBox::upright(Rect::new(20.0, 20.0, 100.0, 80.0))
}

#[trial]
fn a_press_on_a_crop_corner_stays_with_the_crop_and_one_elsewhere_reaches_the_tool() {
    let shared = Rc::new(RefCell::new(crop()));
    let clicked = Rc::new(RefCell::new(None));
    let (shared_crop, shared_click) = (Rc::clone(&shared), Rc::clone(&clicked));
    let mut fixture = Fixture::new(move |ui| {
        // The overlay takes the canvas's top left only: its whole-frame and turn parts cover nothing past it, where the
        // tool owns the press.
        let canvas = Canvas::new("Document").show(ui, |ui, body| {
            let room = egui::Rect::from_min_size(body.rect.min, egui::vec2(160.0, 120.0));
            CropOverlay::new("Crop", Bind::bound(&mut *shared_crop.borrow_mut())).show_at(ui, room)
        });
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
    assert!(after.rect.x0 > before.rect.x0 && after.rect.y0 == before.rect.y0, "the focused corner moved right: {before:?} -> {after:?}");
}

#[trial]
fn page_down_and_home_scroll_a_focused_canvas_and_the_arrows_stay_the_tools() {
    const SHOWN: egui::Rect = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(800.0, 600.0));
    let view = Rc::new(RefCell::new(CanvasView::new(SHOWN, Point::new(400.0, 300.0), 1.0)));
    let held = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let shown = *held.borrow();
        let canvas = Canvas::new("Document").view(shown).document(Size::new(2000.0, 1500.0)).show_at(ui, SHOWN, |_, _| {});
        if let Some(next) = canvas.output.view {
            *held.borrow_mut() = next;
        }
    });
    fixture.settle();
    fixture.click_at(SHOWN.center()).settle();
    let start = *view.borrow();

    fixture.key(egui::Key::PageDown, egui::Modifiers::NONE).settle();
    let paged = *view.borrow();
    assert!(paged.center.y > start.center.y && paged.center.x == start.center.x, "Page Down shows what is below: {start:?} -> {paged:?}");

    fixture.key(egui::Key::ArrowDown, egui::Modifiers::NONE).settle();
    assert_eq!(*view.borrow(), paged, "an arrow is the tool's nudge, not a scroll");

    fixture.key(egui::Key::Home, egui::Modifiers::NONE).settle();
    let home = *view.borrow();
    let corner = home.to_screen(Point::ORIGIN);
    assert!((corner.x - SHOWN.left()).abs() < 1e-3 && (corner.y - SHOWN.top()).abs() < 1e-3, "Home shows the document's top left: {home:?}");
}
