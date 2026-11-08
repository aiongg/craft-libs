//! The full-window parts' own logic: the view's fit and zoom rules, and the title text.

use craft_behavior::PixelSize;
use craft_ui::dialog::{FullWindowTitle, FullWindowView, FullWindowZoomKeys};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2};
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
    assert!(!view.apply(FullWindowZoomKeys::Fit), "fitting a fitted view changes nothing");
    assert!(view.apply(FullWindowZoomKeys::In));
    assert!(!view.is_fit() && view.device_zoom() > fit);
    let kept = view.settled(rect, IMAGE, SCALE);
    assert!((kept.device_zoom() - view.device_zoom()).abs() < SAME, "a view that does not fit keeps its zoom when settled");
    assert!(view.apply(FullWindowZoomKeys::Fit));
    assert!((view.settled(rect, IMAGE, SCALE).device_zoom() - fit).abs() < SAME, "settling a fitted view fits again");
}

#[trial]
fn actual_pixels_is_one_hundred_percent_and_repeats_to_nothing() {
    let mut view = fitted();
    assert!(view.apply(FullWindowZoomKeys::ActualPixels));
    assert!((view.zoom_percent() - 100.0).abs() < SAME);
    assert!(!view.apply(FullWindowZoomKeys::ActualPixels), "the second command finds the view at 100 percent");
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
    assert_eq!(*texts.borrow(), ["Liquify (Layer 1, 67%)", "Liquify (67%)", "Wide Angle (Layer 1)", "Filter Gallery"]);
}
