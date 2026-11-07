//! Navigation rules (wheel, page keys, flick, autoscroll), the zoom motion and the momentary tool order.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CanvasView, Granularity, MoveMods, NavMove, Wheel, WheelPhase, WheelUnit};
use craft_keymap::Mods;
use craft_ui::canvas::{
    Autoscroll, FlickPan, HoldZoom, Momentary, MomentaryCommand, MomentaryTool, NavigationOptions, PageKey, ViewTurn, WheelAction, WheelInput,
    WheelNavigation, ZoomAnimation, ZoomDrag,
};
use craft_ui::runtime::commands::FirePhase;
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2};

/// Points of one wheel notch.
const NOTCH: f32 = 40.0;
const CANVAS: Vec2 = Vec2::new(800.0, 600.0);
const DOCUMENT: Size = Size::new(2000.0, 1500.0);

fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(1000.0, 750.0), 1.0)
}

fn input(scroll: Vec2, mods: Mods) -> WheelInput {
    WheelInput::of(&Wheel::new(scroll, mods, WheelUnit::Point, WheelPhase::Move))
}

#[trial]
fn the_wheel_scrolls_and_alt_wheel_zooms_unless_the_choice_swaps_them() {
    let options = NavigationOptions::default();
    let mut wheel = WheelNavigation::default();
    let scroll = Vec2::new(0.0, NOTCH);
    assert_eq!(wheel.classify(input(scroll, Mods::NONE), &options), Some(WheelAction::Pan(scroll)));
    let zoom = wheel.classify(input(scroll, Mods::ALT), &options);
    assert!(matches!(zoom, Some(WheelAction::Zoom(factor)) if (factor - WheelNavigation::NOTCH).abs() < 1e-9), "an Alt notch zooms: {zoom:?}");

    let mut swapped = NavigationOptions::default();
    swapped.zoom_with_wheel = true;
    assert!(matches!(wheel.classify(input(scroll, Mods::NONE), &swapped), Some(WheelAction::Zoom(_))));
    assert_eq!(wheel.classify(input(scroll, Mods::ALT), &swapped), Some(WheelAction::Pan(scroll)));
}

#[trial]
fn shift_wheel_scrolls_sideways_and_shift_alt_wheel_steps_the_ladder() {
    let options = NavigationOptions::default();
    let mut wheel = WheelNavigation::default();
    let scroll = Vec2::new(0.0, NOTCH);
    assert_eq!(wheel.classify(input(scroll, Mods::SHIFT), &options), Some(WheelAction::Pan(Vec2::new(NOTCH, 0.0))));
    let both = Mods { alt: true, shift: true, ..Mods::NONE };
    assert_eq!(wheel.classify(input(scroll, both), &options), Some(WheelAction::SnapZoom(1)));
}

#[trial]
fn a_part_of_a_ladder_notch_is_kept_until_it_adds_up() {
    let options = NavigationOptions::default();
    let mut wheel = WheelNavigation::default();
    let both = Mods { alt: true, shift: true, ..Mods::NONE };
    let quarter = Vec2::new(0.0, NOTCH / 4.0);
    let steps: Vec<_> = (0..4).map(|_| wheel.classify(input(quarter, both), &options)).collect();
    assert_eq!(steps, vec![None, None, None, Some(WheelAction::SnapZoom(1))]);
}

#[trial]
fn a_pinch_zooms_only_when_the_choice_is_on() {
    let mut wheel = WheelNavigation::default();
    let pinch = WheelInput::of(&Wheel::new(Vec2::ZERO, Mods::NONE, WheelUnit::Point, WheelPhase::Move).zoom(1.2));
    let on = NavigationOptions::default();
    assert!(matches!(wheel.classify(pinch, &on), Some(WheelAction::Zoom(_))));
    let mut off = NavigationOptions::default();
    off.pinch_zooms = false;
    assert_eq!(wheel.classify(pinch, &off), None);
}

#[trial]
fn a_wheel_zoom_keeps_the_point_under_the_pointer_and_stops_at_the_limit() {
    let mut zoomed = view();
    let at = Pos2::new(600.0, 200.0);
    let doc = zoomed.to_doc(at);
    assert!(WheelNavigation::apply(&mut zoomed, WheelAction::Zoom(2.0), at, DOCUMENT, 1.0));
    let after = zoomed.to_doc(at);
    assert!((after.x - doc.x).abs() < 1e-6 && (after.y - doc.y).abs() < 1e-6, "the point under the pointer stays: {doc:?} -> {after:?}");

    let mut limit = view();
    limit.zoom = craft_behavior::ZoomLadder::MAX;
    assert!(!WheelNavigation::apply(&mut limit, WheelAction::Zoom(2.0), at, DOCUMENT, 1.0), "past the last rung the view stays put");
}

#[trial]
fn page_keys_scroll_a_screenful_and_home_end_show_the_corners() {
    let plain = MoveMods::PLAIN;
    assert_eq!(PageKey::of(NavMove::PageDown, plain, CANVAS), Some(PageKey::Scroll(Vec2::new(0.0, CANVAS.y))));
    assert_eq!(PageKey::of(NavMove::PageUp, plain, CANVAS), Some(PageKey::Scroll(Vec2::new(0.0, -CANVAS.y))));
    let sideways = MoveMods::new(false, true, Granularity::Unit);
    assert_eq!(PageKey::of(NavMove::PageDown, sideways, CANVAS), Some(PageKey::Scroll(Vec2::new(CANVAS.x, 0.0))));
    let small = MoveMods::new(true, false, Granularity::Unit);
    assert_eq!(PageKey::of(NavMove::PageDown, small, CANVAS), Some(PageKey::Scroll(Vec2::new(0.0, PageKey::SMALL_STEP))));
    assert_eq!(PageKey::of(NavMove::Start, small, CANVAS), None, "Shift+Home is not a page key");
    assert_eq!(PageKey::of(NavMove::End, plain, CANVAS), Some(PageKey::Corner { end: true }));

    let mut end = view();
    assert!(PageKey::Corner { end: true }.apply(&mut end, DOCUMENT));
    let corner = end.to_screen(Point::new(DOCUMENT.width, DOCUMENT.height));
    assert!(
        (corner.x - end.rect.right()).abs() < 1e-3 && (corner.y - end.rect.bottom()).abs() < 1e-3,
        "End puts the document's corner at the canvas corner: {corner:?}"
    );
}

#[trial]
fn a_fast_hand_release_glides_and_slows_to_a_stop_and_a_slow_one_does_not() {
    const STEP: f64 = 0.016;
    let mut flick = FlickPan::default();
    let mut now = 0.0;
    for _ in 0..4 {
        now += STEP;
        flick.drag(now, Vec2::new(20.0, 0.0));
    }
    flick.release(now, true);
    assert!(flick.gliding(), "a release at speed glides");
    let mut travelled = 0.0;
    let mut passes = 0;
    while let Some(step) = flick.advance(now + STEP * f64::from(passes + 1)) {
        travelled += step.x;
        passes += 1;
        assert!(passes < 1000, "the glide stops");
    }
    assert!(travelled > 0.0 && !flick.gliding());

    let mut slow = FlickPan::default();
    now = 0.0;
    for _ in 0..4 {
        now += STEP;
        slow.drag(now, Vec2::new(0.1, 0.0));
    }
    slow.release(now, true);
    assert!(!slow.gliding(), "a slow release does not glide");

    let mut off = FlickPan::default();
    now = 0.0;
    for _ in 0..4 {
        now += STEP;
        off.drag(now, Vec2::new(20.0, 0.0));
    }
    off.release(now, false);
    assert!(!off.gliding(), "Flick Panning off never glides");
}

#[trial]
fn autoscroll_is_zero_inside_and_grows_past_the_edge_up_to_a_limit() {
    let rect = Rect::from_min_size(Pos2::ZERO, CANVAS);
    const DT: f32 = 0.016;
    assert_eq!(Autoscroll::edge_step(rect.center(), rect, DT), Vec2::ZERO);
    let near = Autoscroll::edge_step(Pos2::new(rect.right() + 5.0, rect.center().y), rect, DT);
    let far = Autoscroll::edge_step(Pos2::new(rect.right() + 500.0, rect.center().y), rect, DT);
    assert!(near.x > 0.0 && far.x > near.x && near.y == 0.0, "scrolls toward the pointer, faster further out: {near:?} {far:?}");
    let farther = Autoscroll::edge_step(Pos2::new(rect.right() + 5000.0, rect.center().y), rect, DT);
    assert_eq!(farther.x, far.x, "the speed has a limit");
    let left = Autoscroll::edge_step(Pos2::new(rect.left() - 5.0, rect.center().y), rect, DT);
    assert!(left.x < 0.0);
}

#[trial]
fn the_zoom_ease_runs_from_the_old_view_to_the_new_one() {
    let from = view();
    let mut target = from;
    target.zoom = 2.0;
    let mut animation = ZoomAnimation::default();
    animation.begin(&from, None, animation.target());
    assert!(animation.running());
    let start = animation.shown(&target, animation.target() - 1.0);
    assert_eq!(start.zoom, from.zoom, "the ease starts at the view drawn last");
    let middle = animation.shown(&target, animation.target() - 0.5);
    assert!(middle.zoom > from.zoom && middle.zoom < target.zoom);
    let end = animation.shown(&target, animation.target());
    assert_eq!(end.zoom, target.zoom);
    assert!(!animation.running(), "the ease ends at the target");
}

#[trial]
fn a_press_held_still_zooms_after_the_delay_and_a_moved_press_never_holds() {
    let size = DOCUMENT;
    let anchor = Pos2::new(400.0, 300.0);
    let mut view = view();
    let mut hold = HoldZoom::press(anchor, view.to_doc(anchor), 0.0);
    assert!(!hold.advance(&mut view, anchor, HoldZoom::DELAY / 2.0, false, size, 1.0), "no zoom inside the delay");
    assert!(hold.advance(&mut view, anchor, HoldZoom::DELAY, false, size, 1.0));
    assert!(hold.advance(&mut view, anchor, HoldZoom::DELAY + 0.1, false, size, 1.0));
    assert!(view.zoom > 1.0 && hold.zooming());

    let mut dragged = HoldZoom::press(anchor, view.to_doc(anchor), 0.0);
    let away = anchor + Vec2::new(HoldZoom::SLOP * 2.0, 0.0);
    assert!(!dragged.advance(&mut view, away, HoldZoom::DELAY * 2.0, false, size, 1.0), "a press that moved is a drag");
}

#[trial]
fn a_scrubby_drag_zooms_about_the_press_and_a_small_rectangle_is_a_click() {
    let anchor = Pos2::new(400.0, 300.0);
    let mut scrubbed = view();
    let doc = scrubbed.to_doc(anchor);
    let mut scrub = ZoomDrag::begin(&scrubbed, anchor, true, 1.0);
    scrub.drag(&mut scrubbed, anchor + Vec2::new(ZoomDrag::DOUBLING, 0.0), DOCUMENT, 1.0);
    assert!((scrubbed.zoom - 2.0).abs() < 1e-9, "a drag of one doubling distance doubles the zoom");
    let after = scrubbed.to_doc(anchor);
    assert!((after.x - doc.x).abs() < 1e-6 && (after.y - doc.y).abs() < 1e-6);

    let mut ruled = view();
    let mut rectangle = ZoomDrag::begin(&ruled, anchor, false, 1.0);
    rectangle.drag(&mut ruled, anchor + Vec2::splat(ZoomDrag::CLICK / 2.0), DOCUMENT, 1.0);
    assert!(rectangle.is_click(anchor + Vec2::splat(ZoomDrag::CLICK / 2.0)));
    rectangle.drag(&mut ruled, anchor + Vec2::new(200.0, 150.0), DOCUMENT, 1.0);
    assert!(!rectangle.is_click(anchor + Vec2::new(200.0, 150.0)));
    rectangle.release(&mut ruled, false, DOCUMENT, 1.0);
    assert!(ruled.zoom > 1.0, "a rectangle zooms to fit it");
}

#[trial]
fn the_compass_snaps_to_fifteen_degrees_and_wraps_into_half_a_turn() {
    assert_eq!(ViewTurn::snap_deg(22.0, 15.0), 15.0);
    assert_eq!(ViewTurn::snap_deg(23.0, 15.0), 30.0);
    assert_eq!(ViewTurn::wrap_deg(190.0), -170.0);
    assert_eq!(ViewTurn::wrap_deg(-180.0), 180.0);
    assert_eq!(ViewTurn::wrap_deg(f64::NAN), 0.0);
}

#[trial]
fn space_is_the_hand_and_cmd_or_alt_added_in_any_order_switch_to_zoom() {
    let mut momentary = Momentary::default();
    assert_eq!(momentary.tool(Mods::NONE, false), None);
    momentary.fired(MomentaryCommand::Hand, FirePhase::Press);
    assert_eq!(momentary.tool(Mods::NONE, false), Some(MomentaryTool::Hand));
    assert_eq!(momentary.tool(Mods::CMD, false), Some(MomentaryTool::ZoomIn));
    assert_eq!(momentary.tool(Mods::ALT, false), Some(MomentaryTool::ZoomOut));
    momentary.fired(MomentaryCommand::ZoomIn, FirePhase::Press);
    assert_eq!(momentary.tool(Mods::CMD, false), Some(MomentaryTool::ZoomIn));
    let both = Mods { cmd: true, alt: true, ..Mods::NONE };
    assert_eq!(momentary.tool(both, false), Some(MomentaryTool::ZoomOut), "Alt added to a held Zoom In zooms out");
    assert_eq!(momentary.tool(Mods::NONE, true), Some(MomentaryTool::ZoomOut), "a drag keeps the tool it began with");
    momentary.fired(MomentaryCommand::ZoomIn, FirePhase::Release);
    assert_eq!(momentary.tool(Mods::NONE, false), None);
}
