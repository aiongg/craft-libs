//! `Canvas` (20.6): a press away from every canvas control's parts reaches the tool.

use craft_ui::canvas::{Canvas, CropOverlay, ToolPath};
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;
use std::rc::Rc;

fn crop() -> Rect {
    Rect::from_min_max(Pos2::new(20.0, 20.0), Pos2::new(100.0, 80.0))
}

#[trial]
fn a_press_on_a_crop_corner_stays_with_the_crop_and_one_elsewhere_reaches_the_tool() {
    let crop = Rc::new(RefCell::new(crop()));
    let clicked = Rc::new(RefCell::new(None));
    let (shared_crop, shared_click) = (Rc::clone(&crop), Rc::clone(&clicked));
    let mut fixture = Fixture::new(move |ui| {
        let canvas = Canvas::new("Document", ToolPath::Command("the Crop To... command"))
            .show(ui, |ui, body| CropOverlay::new("Crop", Bind::bound(&mut *shared_crop.borrow_mut())).show_at(ui, body.rect));
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
    assert_eq!(*crop.borrow(), crop(), "clicks move no corner");
}
