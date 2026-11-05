//! `FullWindowView` and `FullWindowZoomKeys` without a harness.

use egui::{Rect, pos2, vec2};

use super::view::{FullWindowView, FullWindowZoom};
use crate::canvas::{CanvasView, ZoomLadder};

const SIZE: [u32; 2] = [200, 100];

fn room(width: f32, height: f32) -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))
}

// A new view fits: the image centred with the margin on each side, at most 400 %; a resize
// refits; a zoom or a pan ends the fit and `reset_fit` returns to it.
#[test]
fn canvas_fits_on_first_pass_and_until_the_user_zooms() {
    let mut view = FullWindowView::new();
    let shown = view.canvas_view(room(440.0, 240.0), SIZE, 20.0, 1.0);
    assert!(view.is_fit());
    assert_eq!(shown.zoom, 2.0, "(240 - 40) / 100 is the tighter side");
    assert_eq!((shown.center.x, shown.center.y), (100.0, 50.0));
    assert_eq!(view.zoom_percent(), 200.0);

    let wider = view.canvas_view(room(2000.0, 1000.0), SIZE, 20.0, 1.0);
    assert_eq!(wider.zoom, 4.0, "a fitted view never magnifies past 400 %");

    let smaller = view.canvas_view(room(240.0, 140.0), SIZE, 20.0, 1.0);
    assert_eq!(smaller.zoom, 1.0, "a smaller window refits");

    // On a 2x display the zoom on screen is a device zoom: 2 device pixels per point.
    let hidpi = view.canvas_view(room(240.0, 140.0), SIZE, 20.0, 2.0);
    assert_eq!((hidpi.zoom, view.zoom()), (1.0, 2.0));

    // The canvas hands back what a tool or the wheel changed.
    let mut moved: CanvasView = smaller;
    let same = view.canvas_view(room(240.0, 140.0), SIZE, 20.0, 1.0);
    view.take(&same, 1.0);
    assert!(view.is_fit(), "taking the fitted view back changes nothing");
    moved.center.x += 10.0;
    view.take(&moved, 1.0);
    assert!(!view.is_fit() && view.center().x == 110.0, "a pan ends the fit");
    let kept = view.canvas_view(room(2000.0, 1000.0), SIZE, 20.0, 1.0);
    assert_eq!((kept.zoom, kept.center.x), (1.0, 110.0), "a resize no longer refits");

    view.reset_fit();
    assert_eq!(view.canvas_view(room(2000.0, 1000.0), SIZE, 20.0, 1.0).zoom, 4.0);

    view.set_zoom(0.5);
    assert!(!view.is_fit() && view.zoom() == 0.5);
}

// `step` follows `ZoomLadder` from the zoom on screen (the fitted zoom too) and stops at the
// ends of the range for the image's size; `apply` maps In, Out, Fit and ActualPixels.
#[test]
fn zoom_steps_follow_the_ladder_from_the_zoom_on_screen() {
    let mut view = FullWindowView::new();
    view.canvas_view(room(440.0, 240.0), SIZE, 20.0, 1.0);
    assert!(view.step(1));
    assert!(view.zoom() > 2.0 && !view.is_fit());
    assert_eq!(view.zoom(), ZoomLadder::step(2.0, 1, kurbo::Size::new(200.0, 100.0)), "from the fitted 200 %");

    assert!(view.apply(FullWindowZoom::ActualPixels) && view.zoom() == 1.0);
    assert!(!view.apply(FullWindowZoom::ActualPixels), "already 100 %");
    assert!(view.apply(FullWindowZoom::Out) && view.zoom() < 1.0);

    view.set_zoom(1e9);
    assert_eq!(view.zoom(), ZoomLadder::MAX, "set_zoom stays in the ladder's range");
    assert!(!view.step(1), "no rung past the top");
    view.set_zoom(0.0001);
    let lowest = view.zoom();
    assert_eq!(lowest, ZoomLadder::min(kurbo::Size::new(200.0, 100.0)));
    assert!(!view.step(-1) && view.zoom() == lowest, "no rung below the range");

    assert!(view.apply(FullWindowZoom::Fit) && view.is_fit());
    assert!(!view.apply(FullWindowZoom::Fit), "already fitted");
}
