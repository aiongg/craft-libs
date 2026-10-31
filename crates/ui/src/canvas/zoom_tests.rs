//! The zoom ladder, fit and fill, zoom about a point, the hold, the Zoom tool's drags, Zoom Resizes Windows.

use egui::{Context, Id, Pos2, RawInput, Rect as ScreenRect, pos2, vec2};
use kurbo::{Point, Size};

use super::{CanvasView, HoldZoom, ZoomAnimation, ZoomDrag, ZoomLadder, ZoomWindowFit, zoom};

const PHOTO: Size = Size::new(2000.0, 1500.0);

fn canvas() -> ScreenRect {
    ScreenRect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 600.0))
}

fn view_at(device: f64, ppp: f32) -> CanvasView {
    CanvasView::new(canvas(), Point::new(1000.0, 750.0), CanvasView::points_per_unit(device, ppp))
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

fn close(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < 1e-3
}

// 880 % steps in to 1200 % and out to 800 %; 12800 % does not step in.
#[test]
fn zoom_in_and_out_step_from_any_zoom_to_the_next_rung() {
    assert_eq!(ZoomLadder::step(8.8, 1, PHOTO), 12.0);
    assert_eq!(ZoomLadder::step(8.8, -1, PHOTO), 8.0);
    assert_eq!(ZoomLadder::step(1.0, 1, PHOTO), 2.0);
    assert_eq!(ZoomLadder::step(1.0, -1, PHOTO), 2.0 / 3.0);
    assert_eq!(ZoomLadder::step(ZoomLadder::MAX, 1, PHOTO), ZoomLadder::MAX);
    let lowest = ZoomLadder::min(PHOTO);
    assert_eq!(ZoomLadder::step(lowest, -1, PHOTO), lowest);
    let mut view = view_at(8.8, 1.0);
    assert!(zoom::step_view(&mut view, 1, PHOTO, 1.0));
    assert!(near(zoom::device_zoom(&view, 1.0), 12.0));
    let mut view = view_at(ZoomLadder::MAX, 2.0);
    assert!(!zoom::step_view(&mut view, 1, PHOTO, 2.0), "no step at the limit");
}

// A 2000 × 1500 document goes down to 2/1500; an empty size gives 0.1 %.
#[test]
fn the_lowest_zoom_shows_the_shorter_side_two_pixels() {
    assert_eq!(ZoomLadder::min(PHOTO), 2.0 / 1500.0);
    assert_eq!(ZoomLadder::min(Size::ZERO), 0.001);
    assert_eq!(ZoomLadder::clamp(f64::NAN, PHOTO), 1.0);
    assert_eq!(ZoomLadder::clamp(1000.0, PHOTO), ZoomLadder::MAX);
    assert_eq!(ZoomLadder::clamp(0.0, PHOTO), 2.0 / 1500.0);
    let range = ZoomLadder::percent_range(500.0, PHOTO);
    assert_eq!(*range.end(), 50000.0, "the field range includes the zoom itself");
}

// A small document fits at 100 % on a 2× display (view.zoom 0.5), centred, 20-point margins.
#[test]
fn fit_never_magnifies_past_one_device_pixel() {
    let small = Size::new(100.0, 80.0);
    let mut view = view_at(3.0, 2.0);
    zoom::fit_view(&mut view, small, 2.0);
    assert!(near(view.zoom, 0.5));
    assert_eq!(view.center, Point::new(50.0, 40.0));
    let mut view = view_at(1.0, 1.0);
    zoom::fit_view(&mut view, PHOTO, 1.0);
    // 760 × 560 points available: the height limits it.
    assert!(near(view.zoom, 560.0 / 1500.0));
    assert!(view.to_screen(Point::new(0.0, 0.0)).y >= 50.0 + 20.0 - 1e-3);
}

// The document covers the canvas; one axis overflows.
#[test]
fn fill_covers_the_canvas_on_one_axis() {
    let tall = Size::new(1000.0, 1500.0);
    let mut view = view_at(1.0, 1.0);
    zoom::fill_view(&mut view, tall, 1.0);
    assert!(near(view.zoom, 0.8), "800 / 1000 is the larger ratio");
    let tl = view.to_screen(Point::ZERO);
    let br = view.to_screen(Point::new(1000.0, 1500.0));
    assert!(tl.x <= 100.0 + 1e-3 && br.x >= 900.0 - 1e-3 && tl.y <= 50.0 && br.y >= 650.0);
    assert!((br.y - tl.y) > 600.0, "the other axis overflows");
}

// The document point under the pointer maps to the same screen point after the zoom, in a turned and mirrored view.
#[test]
fn a_zoom_about_the_pointer_keeps_its_document_point_under_it() {
    let mut view = view_at(1.0, 1.5);
    view.rotation = 30.0;
    view.flip_x = true;
    view.aspect = 1.25;
    let pointer = pos2(300.0, 200.0);
    let doc = view.to_doc(pointer);
    zoom::zoom_about(&mut view, pointer, 3.0, false, 1.5);
    assert!(near(zoom::device_zoom(&view, 1.5), 3.0));
    assert!(close(view.to_screen(doc), pointer));
}

// With the preference the clicked document point lands at the view centre.
#[test]
fn zoom_clicked_point_to_center_centres_the_point() {
    let mut view = view_at(1.0, 1.0);
    let pointer = pos2(300.0, 200.0);
    let doc = view.to_doc(pointer);
    zoom::zoom_about(&mut view, pointer, 2.0, true, 1.0);
    assert!(close(view.to_screen(doc), canvas().center()));
}

// A pointer on the pasteboard anchors at the image's nearest edge point.
#[test]
fn the_wheel_anchor_is_the_nearest_document_point_off_the_image() {
    let mut view = view_at(1.0, 1.0);
    zoom::fit_view(&mut view, PHOTO, 1.0);
    let inside = view.to_screen(Point::new(500.0, 500.0));
    assert_eq!(zoom::wheel_anchor(&view, inside, PHOTO), inside);
    let off = view.to_screen(Point::new(-300.0, 500.0));
    let edge = zoom::wheel_anchor(&view, off, PHOTO);
    assert!(close(edge, view.to_screen(Point::new(0.0, 500.0))));
    view.rotation = 90.0;
    let off = view.to_screen(Point::new(2500.0, 1800.0));
    assert!(close(zoom::wheel_anchor(&view, off, PHOTO), view.to_screen(Point::new(2000.0, 1500.0))));
}

fn pass<R>(ctx: &Context, time: f64, f: impl FnOnce(&Context) -> R) -> R {
    let mut out = None;
    let mut f = Some(f);
    let output = ctx.run_ui(RawInput { time: Some(time), ..Default::default() }, |ui| out = Some((f.take().unwrap())(ui.ctx())));
    let mut textures = output.textures_delta;
    textures.clear();
    out.unwrap()
}

// Shown views pass between the start and the target and equal the target after 0.22 s.
#[test]
fn an_animated_zoom_eases_to_the_target_and_ends_on_it() {
    let ctx = Context::default();
    let id = super::CanvasId::from_id(Id::unique("canvas"));
    let from = view_at(1.0, 1.0);
    let mut target = from;
    let pointer = pos2(300.0, 200.0);
    let doc = from.to_doc(pointer);
    zoom::anchor_view(&mut target, pointer, doc, 4.0, 1.0);
    pass(&ctx, 1.0, |c| ZoomAnimation::begin(c, id, &from, Some((doc, pointer))));
    assert!(pass(&ctx, 1.0, |c| ZoomAnimation::running(c, id)));
    let first = pass(&ctx, 1.0, |c| ZoomAnimation::shown(c, id, &target, true));
    assert_eq!(first.zoom, from.zoom);
    let mid = pass(&ctx, 1.11, |c| ZoomAnimation::shown(c, id, &target, true));
    assert!(mid.zoom > from.zoom && mid.zoom < target.zoom);
    assert!(close(mid.to_screen(doc), pointer), "the anchor stays under the pointer");
    let end = pass(&ctx, 1.23, |c| ZoomAnimation::shown(c, id, &target, true));
    assert_eq!(end, target);
    assert!(!pass(&ctx, 1.23, |c| ZoomAnimation::running(c, id)));
    // Off: the target at once; stop: the target at once.
    pass(&ctx, 2.0, |c| ZoomAnimation::begin(c, id, &from, None));
    assert_eq!(pass(&ctx, 2.0, |c| ZoomAnimation::shown(c, id, &target, false)), target);
    pass(&ctx, 3.0, |c| ZoomAnimation::begin(c, id, &from, None));
    pass(&ctx, 3.0, |c| ZoomAnimation::stop(c, id));
    assert_eq!(pass(&ctx, 3.0, |c| ZoomAnimation::shown(c, id, &target, true)), target);
}

fn held(frame: f64, total: f64) -> f64 {
    let mut view = view_at(1.0, 1.0);
    let mut hold = HoldZoom::press(pos2(300.0, 200.0), view.to_doc(pos2(300.0, 200.0)), 0.0);
    for step in 1..=(total / frame).round() as u32 {
        hold.advance(&mut view, pos2(300.0, 200.0), f64::from(step) * frame, false, PHOTO, 1.0);
    }
    zoom::device_zoom(&view, 1.0)
}

// Two doublings a second whether passes are 10 or 100 ms apart; a 1 s stall moves at most 0.25 s.
#[test]
fn a_held_press_zooms_at_the_same_rate_at_any_frame_rate() {
    // Held 0.3 s still, then 0.5 s of zooming at two doublings a second: 2^1.
    let fast = held(0.01, 0.8);
    let slow = held(0.1, 0.8);
    assert!((fast / 2.0 - 1.0).abs() < 1e-6, "{fast}");
    assert!((slow / 2.0 - 1.0).abs() < 1e-6, "{slow}");
    assert_eq!(HoldZoom::zoom_after(1.0, 1.0, false, PHOTO), HoldZoom::zoom_after(1.0, 0.25, false, PHOTO));
    assert_eq!(HoldZoom::zoom_after(1.0, f64::NAN, false, PHOTO), 1.0);
    assert_eq!(HoldZoom::zoom_after(1.0, -1.0, true, PHOTO), 1.0);
    assert!(near(HoldZoom::zoom_after(1.0, 0.25, true, PHOTO), 0.5_f64.sqrt()));
}

// Moving 7 points before 0.3 s leaves the zoom alone.
#[test]
fn a_press_that_moves_past_the_slop_never_becomes_a_hold() {
    let mut view = view_at(1.0, 1.0);
    let before = view;
    let start = pos2(300.0, 200.0);
    let mut hold = HoldZoom::press(start, view.to_doc(start), 0.0);
    assert!(!hold.advance(&mut view, start + vec2(7.0, 0.0), 0.2, false, PHOTO, 1.0));
    assert!(!hold.advance(&mut view, start, 1.0, false, PHOTO, 1.0), "back at the press, still a drag");
    assert!(!hold.zooming());
    assert_eq!(view, before);
}

// A drag of +100 points doubles the zoom about the press point; -100 halves it.
#[test]
fn scrubby_zoom_doubles_per_hundred_points() {
    assert!(near(ZoomDrag::scrub_zoom(1.0, 100.0, PHOTO), 2.0));
    assert!(near(ZoomDrag::scrub_zoom(1.0, -100.0, PHOTO), 0.5));
    assert_eq!(ZoomDrag::scrub_zoom(1.0, f32::NAN, PHOTO), 1.0);
    let mut view = view_at(1.0, 2.0);
    let press = pos2(300.0, 200.0);
    let doc = view.to_doc(press);
    let mut drag = ZoomDrag::begin(&view, press, true, 2.0);
    drag.drag(&mut view, press + vec2(100.0, 5.0), PHOTO, 2.0);
    assert!(near(zoom::device_zoom(&view, 2.0), 2.0));
    assert!(close(view.to_screen(doc), press));
    assert!(drag.rect().is_none());
}

// Releasing a rectangle zooms so it fills the canvas; under 3 points it does nothing.
#[test]
fn a_zoom_rectangle_fits_its_area_and_a_tiny_one_is_a_click() {
    let mut view = view_at(1.0, 1.0);
    let (a, b) = (pos2(200.0, 100.0), pos2(300.0, 175.0));
    let rect_center = view.to_doc(pos2(250.0, 137.5));
    let mut drag = ZoomDrag::begin(&view, a, false, 1.0);
    drag.drag(&mut view, b, PHOTO, 1.0);
    assert_eq!(drag.rect(), Some(ScreenRect::from_two_pos(a, b)));
    drag.release(&mut view, false, PHOTO, 1.0);
    assert!(near(zoom::device_zoom(&view, 1.0), 8.0), "800 / 100");
    assert!(close(view.to_screen(rect_center), canvas().center()));

    let mut view = view_at(1.0, 1.0);
    let before = view;
    let mut tiny = ZoomDrag::begin(&view, a, false, 1.0);
    tiny.drag(&mut view, a + vec2(3.5, 3.5), PHOTO, 1.0);
    tiny.release(&mut view, false, PHOTO, 1.0);
    assert_eq!(view, before);
}

fn window() -> ZoomWindowFit {
    ZoomWindowFit { chrome: vec2(40.0, 60.0), screen: ScreenRect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0)) }
}

// The wrapped size includes the chrome, stays at least 240 × 160 and inside the screen less 48.
#[test]
fn a_zoomed_window_wraps_the_image_within_the_screen() {
    let fit = window();
    let mut view = view_at(1.0, 1.0);
    view.zoom = 0.2;
    assert_eq!(fit.wrapped_size(&view, PHOTO), Some(vec2(440.0, 360.0)));
    assert_eq!(fit.wrapped_center(&view, PHOTO), Some(Point::new(1000.0, 750.0)));
    view.zoom = 0.01;
    assert_eq!(fit.wrapped_size(&view, PHOTO), Some(vec2(240.0, 160.0)));
    view.zoom = 4.0;
    assert_eq!(fit.wrapped_size(&view, PHOTO), Some(vec2(1920.0 - 48.0, 1080.0 - 48.0)));
    assert_eq!(fit.wrapped_center(&view, PHOTO), None, "a window the screen caps keeps the zoomed spot");
    assert_eq!(fit.wrapped_size(&view, Size::ZERO), None);
}

// Same zoom or preference off: None; a fit never resizes.
#[test]
fn only_a_zoom_change_with_the_preference_on_resizes_the_window() {
    let ctx = Context::default();
    let fit = window();
    let id = Id::unique("window");
    let (mut before, mut after) = (view_at(1.0, 1.0), view_at(1.0, 1.0));
    before.zoom = 0.2;
    after.zoom = 0.2;
    assert_eq!(fit.resize_for(&ctx, id, true, &before, &after, PHOTO), None, "the first call records");
    assert_eq!(fit.resize_for(&ctx, id, true, &before, &after, PHOTO), None, "same zoom");
    after.zoom = 0.3;
    assert_eq!(fit.resize_for(&ctx, id, false, &before, &after, PHOTO), None, "preference off, and a fit passes off");
    before.zoom = 0.3;
    after.zoom = 0.1;
    assert_eq!(fit.resize_for(&ctx, id, true, &before, &after, PHOTO), Some(vec2(240.0, 210.0)));
}
