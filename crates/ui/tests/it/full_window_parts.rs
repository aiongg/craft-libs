//! The full-window parts' own logic: the view's fit and zoom rules, and the title text.

use craft_behavior::PixelSize;
use craft_behavior::kurbo::Size;
use craft_ui::canvas::ViewCommand;
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::dialog::{FullWindowDialog, FullWindowTitle, FullWindowView, FullWindowZoomBar};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The canvas region's size in points.
const CANVAS: Vec2 = Vec2::new(800.0, 600.0);

/// An image larger than the canvas, so a fitted view shows it reduced.
const IMAGE: PixelSize = PixelSize::new(4000, 3000);

/// Pixels per point of the tests' screen.
const SCALE: f32 = 1.0;

/// How far a pan moves the view's centre in image pixels.
const PAN: f64 = 10.0;

/// Zooms closer than this are the same zoom.
const SAME: f64 = 1e-6;

fn fitted() -> FullWindowView {
    FullWindowView::new().settled(Rect::from_min_size(Pos2::ZERO, CANVAS), IMAGE, SCALE)
}

#[trial]
fn a_zoom_command_ends_the_fit_and_fit_returns_to_it() {
    let rect = Rect::from_min_size(Pos2::ZERO, CANVAS);
    let mut view = fitted();
    let fit = view.device_zoom();
    assert!(view.is_fit() && fit < 1.0, "a new view fits an image larger than the canvas, reduced");
    assert!(!view.apply(ViewCommand::FitOnScreen), "fitting a fitted view changes nothing");
    assert!(view.apply(ViewCommand::ZoomIn));
    assert!(!view.is_fit() && view.device_zoom() > fit);
    let kept = view.settled(rect, IMAGE, SCALE);
    assert!((kept.device_zoom() - view.device_zoom()).abs() < SAME, "a view that does not fit keeps its zoom when settled");
    assert!(view.apply(ViewCommand::FitOnScreen));
    assert!((view.settled(rect, IMAGE, SCALE).device_zoom() - fit).abs() < SAME, "settling a fitted view fits again");
}

#[trial]
fn actual_pixels_is_one_hundred_percent_and_repeats_to_nothing() {
    let mut view = fitted();
    assert!(view.apply(ViewCommand::ActualPixels));
    assert!((view.zoom_percent() - 100.0).abs() < SAME);
    assert!(!view.apply(ViewCommand::ActualPixels), "the second command finds the view at 100 percent");
}

#[trial]
fn a_view_taken_back_from_the_canvas_ends_the_fit_only_when_it_moved() {
    let view = fitted();
    assert!(view.took(view.view()).is_fit(), "the canvas handing back the view it was given keeps the fit");
    let mut panned = view.view();
    panned.center.x += PAN;
    assert!(!view.took(panned).is_fit(), "a pan ends the fit");
}

#[trial]
fn the_title_drops_the_ellipsis_and_names_what_it_has() {
    let texts = Rc::new(RefCell::new(Vec::new()));
    let shown = texts.clone();
    let mut fixture = Fixture::new(move |ui| {
        let env = Env::of(ui);
        let mut texts = shown.borrow_mut();
        texts.clear();
        for title in [
            FullWindowTitle::new("Liquify…").subject("Layer 1").zoom(66.7),
            FullWindowTitle::new("Liquify…").zoom(66.7),
            FullWindowTitle::new("Wide Angle").subject("Layer 1"),
            FullWindowTitle::new("Filter Gallery…"),
        ] {
            texts.push(title.label(&env).as_str().to_owned());
        }
    });
    fixture.settle();
    assert_eq!(*texts.borrow(), ["Liquify (Layer 1, 66.7%)", "Liquify (66.7%)", "Wide Angle (Layer 1)", "Filter Gallery"]);
}

// Zoom bar and view commands (fwzoom).

/// The legacy zoom bar's image, in pixels, and the canvas region it fits in.
const BAR_IMAGE: PixelSize = PixelSize::new(200, 100);
const BAR_CANVAS: Vec2 = Vec2::new(500.0, 250.0);

/// A fixture that shows a full-window dialog holding a zoom bar bound to the returned view.
fn dialog_with_zoom_bar() -> (Fixture, Rc<RefCell<FullWindowView>>) {
    let view = Rc::new(RefCell::new(FullWindowView::new().settled(Rect::from_min_size(Pos2::ZERO, BAR_CANVAS), BAR_IMAGE, SCALE)));
    let shared = view.clone();
    let fixture = Fixture::new(move |ui| {
        let mut open = true;
        FullWindowDialog::new("Liquify").opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            FullWindowZoomBar::new(Bind::bound(&mut shared.borrow_mut())).show(ui);
        });
    });
    (fixture, view)
}

#[trial]
fn the_zoom_bar_steps_the_ladder_and_lists_an_off_menu_zoom() {
    let (mut fixture, view) = dialog_with_zoom_bar();
    fixture.settle();
    let fitted = view.borrow().zoom_percent();
    assert!(view.borrow().is_fit());

    let zoom_in = fixture.by_label("Zoom In").expect("the Zoom In button is named by its command");
    fixture.click(&zoom_in).settle();
    let stepped = view.borrow().zoom_percent();
    assert!(!view.borrow().is_fit() && stepped > fitted, "Zoom In steps up from the fit: {stepped} after {fitted}");

    let zoom_out = fixture.by_label("Zoom Out").expect("the Zoom Out button is named by its command");
    fixture.click(&zoom_out).settle();
    assert!(view.borrow().zoom_percent() < stepped, "Zoom Out steps back down");

    // 300 % is not on the menu: the select lists it as itself, between 200 % and 400 %, after Fit in View.
    view.borrow_mut().set_device_zoom(3.0);
    fixture.settle();
    let select = fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the zoom select is shown");
    fixture.click(&select).settle();
    let options: Vec<String> =
        fixture.by_role(Role::LIST_BOX_OPTION).iter().filter_map(|option| fixture.tree().name_of(option.node).map(str::to_owned)).collect();
    let at = |text: &str| options.iter().position(|option| option == text).unwrap_or_else(|| panic!("{text} in {options:?}"));
    assert!(at("200%") < at("300%") && at("300%") < at("400%"), "{options:?}");
    assert_eq!(options[0], "Fit in View");
}

#[trial]
fn view_chords_zoom_the_dialogs_view() {
    let (mut fixture, view) = dialog_with_zoom_bar();
    fixture.settle();
    let fitted = view.borrow().zoom_percent();

    fixture.key(Key::Num1, Modifiers::COMMAND).settle();
    assert!((view.borrow().zoom_percent() - 100.0).abs() < SAME && !view.borrow().is_fit(), "Cmd+1 shows 100 percent");
    fixture.key(Key::Equals, Modifiers::COMMAND).settle();
    assert!(view.borrow().zoom_percent() > 100.0, "Cmd+= zooms the dialog's view in");
    fixture.key(Key::Minus, Modifiers::COMMAND).settle();
    assert!((view.borrow().zoom_percent() - 100.0).abs() < SAME, "in and out are inverse steps from a rung");
    fixture.key(Key::Num0, Modifiers::COMMAND).settle();
    assert!(view.borrow().is_fit() && (view.borrow().zoom_percent() - fitted).abs() < SAME, "Cmd+0 fits again");
}

#[trial]
fn fit_on_screen_refits_in_the_same_pass_and_magnifies_a_small_image() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.0));
    let mut view = FullWindowView::new().settled(rect, PixelSize::new(100, 100), SCALE);
    assert!((view.zoom_percent() - 400.0).abs() < SAME, "a small image fits magnified, up to 400 percent");
    assert!(view.apply(ViewCommand::ActualPixels));
    assert!(view.apply(ViewCommand::ZoomIn));
    assert!(view.apply(ViewCommand::FitOnScreen));
    assert!(view.is_fit() && (view.zoom_percent() - 400.0).abs() < SAME, "the zoom read right after the command is the fitted one");
    assert!(!view.apply(ViewCommand::FillScreen) && !view.apply(ViewCommand::RotateClockwise), "only the zoom commands act");
    assert_eq!(view.size(), Size::new(100.0, 100.0));
}
