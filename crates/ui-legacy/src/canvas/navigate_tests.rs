//! Wheel and pinch, page keys, flick pan, touchpad glide, autoscroll, Rotate View.

use egui::{Context, Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, RawInput, TouchPhase, pos2, vec2};
use egui_kittest::Harness;
use kurbo::{Point, Size};

use super::navigate::{Autoscroll, FlickPan, NavigationOptions, PageKey, RotateView, TouchpadGlide, WheelAction, WheelInput, WheelNavigation, pan};
use super::{CanvasView, ZoomLadder};
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn view(zoom: f64, center: Point) -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)), center, zoom)
}

/// One pass of `dy` points of wheel with the given modifiers held.
fn wheel(dy: f32, alt: bool, shift: bool) -> WheelInput {
    WheelInput { scroll: vec2(0.0, dy), raw: vec2(0.0, dy), zoom_delta: 1.0, alt, shift, notch: 40.0 }
}

fn zoom_factor(action: Option<WheelAction>) -> f64 {
    match action {
        Some(WheelAction::Zoom(f)) => f,
        other => panic!("expected a zoom, got {other:?}"),
    }
}

// Alt + one notch is Zoom(1.1); no modifier is Pan.
#[test]
fn alt_wheel_zooms_and_the_plain_wheel_scrolls() {
    let prefs = NavigationOptions::default();
    let f = zoom_factor(WheelNavigation::classify(wheel(40.0, true, false), &prefs));
    assert!((f - 1.1).abs() < 1e-6, "{f}");
    assert_eq!(WheelNavigation::classify(wheel(40.0, false, false), &prefs), Some(WheelAction::Pan(vec2(0.0, 40.0))));
    assert_eq!(WheelNavigation::classify(wheel(0.0, false, false), &prefs), None);
}

// With the preference the wheel zooms and Alt + wheel pans.
#[test]
fn zoom_with_scroll_wheel_swaps_wheel_and_alt_wheel() {
    let prefs = NavigationOptions { zoom_with_wheel: true, ..NavigationOptions::default() };
    let f = zoom_factor(WheelNavigation::classify(wheel(40.0, false, false), &prefs));
    assert!((f - 1.1).abs() < 1e-6, "{f}");
    assert_eq!(WheelNavigation::classify(wheel(40.0, true, false), &prefs), Some(WheelAction::Pan(vec2(0.0, 40.0))));
    assert_eq!(WheelNavigation::classify(wheel(40.0, false, true), &prefs), Some(WheelAction::SnapZoom(1)));
}

// Shift + Alt + one notch is SnapZoom(1); ten notches cap at 10.
#[test]
fn shift_alt_wheel_steps_the_ladder() {
    let prefs = NavigationOptions::default();
    assert_eq!(WheelNavigation::classify(wheel(40.0, true, true), &prefs), Some(WheelAction::SnapZoom(1)));
    assert_eq!(WheelNavigation::classify(wheel(-40.0, true, true), &prefs), Some(WheelAction::SnapZoom(-1)));
    assert_eq!(WheelNavigation::classify(wheel(1000.0, true, true), &prefs), Some(WheelAction::SnapZoom(10)));
}

// zoom_delta 1.25 is Zoom(1.25) with the preference, ignored without.
#[test]
fn a_pinch_zooms_only_with_zoom_with_trackpad_pinch() {
    let pinch = WheelInput { zoom_delta: 1.25, ..wheel(0.0, false, false) };
    let on = NavigationOptions::default();
    assert_eq!(WheelNavigation::classify(pinch, &on), Some(WheelAction::Zoom(1.25)));
    let off = NavigationOptions { pinch_zooms: false, ..on };
    assert_eq!(WheelNavigation::classify(pinch, &off), None);
}

// At 12800 % a zoom-in notch changes neither zoom nor centre.
#[test]
fn a_notch_past_the_zoom_limit_leaves_the_view_alone() {
    let size = Size::new(1000.0, 800.0);
    let prefs = NavigationOptions::default();
    let mut v = view(ZoomLadder::MAX, Point::new(500.0, 400.0));
    let before = v;
    assert!(!WheelNavigation::apply(&mut v, WheelAction::Zoom(1.1), pos2(100.0, 100.0), size, &prefs, 1.0));
    assert_eq!(v, before);
    // Short of the limit a notch zooms about the pointer.
    let mut v = view(1.0, Point::new(500.0, 400.0));
    let at = pos2(100.0, 100.0);
    let doc = v.to_doc(at);
    assert!(WheelNavigation::apply(&mut v, WheelAction::Zoom(1.1), at, size, &prefs, 1.0));
    assert!((v.zoom - 1.1).abs() < 1e-9);
    assert!((v.to_screen(doc) - at).length() < 1e-3, "the point under the pointer stays");
}

// Page Down scrolls the canvas height, Cmd sideways, Shift 10 points; Alt and Shift+Home are None.
#[test]
fn page_keys_scroll_a_canvas_and_shift_ten_points() {
    let canvas = vec2(800.0, 600.0);
    let of = |key, modifiers| PageKey::of(key, modifiers, canvas);
    assert_eq!(of(Key::PageDown, Modifiers::NONE), Some(PageKey::Scroll(vec2(0.0, 600.0))));
    assert_eq!(of(Key::PageUp, Modifiers::NONE), Some(PageKey::Scroll(vec2(0.0, -600.0))));
    assert_eq!(of(Key::PageDown, Modifiers::COMMAND), Some(PageKey::Scroll(vec2(800.0, 0.0))));
    assert_eq!(of(Key::PageUp, Modifiers::COMMAND), Some(PageKey::Scroll(vec2(-800.0, 0.0))));
    assert_eq!(of(Key::PageDown, Modifiers::SHIFT), Some(PageKey::Scroll(vec2(0.0, 10.0))));
    assert_eq!(of(Key::PageDown, Modifiers::ALT), None);
    assert_eq!(of(Key::Home, Modifiers::SHIFT), None);
    assert_eq!(of(Key::A, Modifiers::NONE), None);

    // The scroll shows what is below: the document moves up by that many points.
    let mut v = view(0.5, Point::new(1000.0, 750.0));
    let before = v.to_screen(Point::new(0.0, 0.0));
    assert!(PageKey::Scroll(vec2(0.0, 100.0)).apply(&mut v, Size::new(2000.0, 1500.0), true));
    assert!((before.y - v.to_screen(Point::new(0.0, 0.0)).y - 100.0).abs() < 1e-3);
}

// Home puts the top-left corner at the canvas's top-left; End the bottom-right.
#[test]
fn home_and_end_show_the_document_corners() {
    let size = Size::new(2000.0, 1500.0);
    let mut v = view(1.0, Point::new(1000.0, 750.0));
    assert!(PageKey::Corner { end: false }.apply(&mut v, size, true));
    assert!((v.to_screen(Point::new(0.0, 0.0)) - v.rect.left_top()).length() < 1e-3);
    assert_eq!(v.zoom, 1.0, "the zoom stays");
    assert!(PageKey::Corner { end: true }.apply(&mut v, size, true));
    assert!((v.to_screen(Point::new(2000.0, 1500.0)) - v.rect.right_bottom()).length() < 1e-3);
    // A view turned half a turn runs the document from the opposite canvas corner.
    let mut v = CanvasView { rotation: 180.0, ..view(1.0, Point::new(1000.0, 750.0)) };
    PageKey::Corner { end: false }.apply(&mut v, size, true);
    assert!((v.to_screen(Point::new(0.0, 0.0)) - v.rect.right_bottom()).length() < 1e-3);
}

// Zero inside; 120 pt/s at the edge plus 12 per point past it, capped at 2400; dt capped at 0.05.
#[test]
fn autoscroll_speeds_up_past_the_edge() {
    let rect = egui::Rect::from_min_max(pos2(100.0, 50.0), pos2(500.0, 350.0));
    assert_eq!(Autoscroll::edge_step(pos2(300.0, 200.0), rect, 0.01), vec2(0.0, 0.0));
    assert_eq!(Autoscroll::edge_step(pos2(500.0, 350.0), rect, 0.01), vec2(0.0, 0.0));
    // 10 points past the right edge: (120 + 12 * 10) pt/s over 0.01 s.
    let s = Autoscroll::edge_step(pos2(510.0, 200.0), rect, 0.01);
    assert!((s.x - 2.4).abs() < 1e-4 && s.y == 0.0, "{s:?}");
    // Left and up are negative.
    let s = Autoscroll::edge_step(pos2(90.0, 40.0), rect, 0.01);
    assert!(s.x < 0.0 && s.y < 0.0);
    // Far past the edge the speed caps at 2400 pt/s.
    let s = Autoscroll::edge_step(pos2(5000.0, 200.0), rect, 0.01);
    assert!((s.x - 24.0).abs() < 1e-4, "{s:?}");
    // A long frame counts for 0.05 s.
    let s = Autoscroll::edge_step(pos2(510.0, 200.0), rect, 5.0);
    assert!((s.x - 12.0).abs() < 1e-4, "{s:?}");
}

// An image that fits the canvas never scrolls; one already past a limit is not pulled back.
#[test]
fn autoscroll_stops_at_the_scroll_limits() {
    let big = Size::new(2000.0, 1500.0);
    let mut fits = view(1.0, Point::new(150.0, 100.0));
    assert_eq!(Autoscroll::scroll(&mut fits, vec2(50.0, 50.0), Size::new(300.0, 200.0)), None);
    // At the left limit: further left does nothing, right moves.
    let mut v = view(1.0, Point::new(400.0, 300.0));
    assert_eq!(Autoscroll::scroll(&mut v, vec2(-50.0, 0.0), big), None);
    let moved = Autoscroll::scroll(&mut v, vec2(50.0, 0.0), big).expect("moves right");
    assert!((moved.x - 50.0).abs() < 1e-9 && v.center.x == 450.0);
    // Past the left limit (Overscroll was on): not pulled back, and further left does nothing.
    let mut past = view(1.0, Point::new(300.0, 300.0));
    assert_eq!(Autoscroll::scroll(&mut past, vec2(-50.0, 0.0), big), None);
    assert_eq!(past.center.x, 300.0);
    // The right limit stops a step at the edge, not past it.
    let mut right = view(1.0, Point::new(1590.0, 300.0));
    let moved = Autoscroll::scroll(&mut right, vec2(50.0, 0.0), big).expect("moves to the limit");
    assert!((moved.x - 10.0).abs() < 1e-9, "{moved:?}");
}

/// Runs one egui pass at `time`.
fn pass(ctx: &Context, time: f64, f: impl FnOnce(&Context)) {
    let mut f = Some(f);
    let mut output = ctx.run_ui(RawInput { time: Some(time), ..RawInput::default() }, |ui| {
        if let Some(f) = f.take() {
            f(ui.ctx());
        }
    });
    output.textures_delta.clear();
}

// A fast release keeps panning with falling speed; with the preference off it stops.
#[test]
fn a_released_hand_drag_glides_and_slows_to_a_stop() {
    const DT: f64 = 1.0 / 60.0;
    let run = |enabled: bool| {
        let ctx = Context::default();
        let id = super::CanvasId::from_id(egui::Id::unique("flick"));
        let mut t = 0.0;
        for _ in 0..10 {
            t += DT;
            pass(&ctx, t, |ctx| assert_eq!(FlickPan::frame(ctx, id, true, vec2(20.0, 0.0), enabled), None));
        }
        let mut steps = Vec::new();
        for _ in 0..600 {
            t += DT;
            let mut step = None;
            pass(&ctx, t, |ctx| step = FlickPan::frame(ctx, id, false, vec2(0.0, 0.0), enabled));
            match step {
                Some(d) => steps.push(d.x),
                None => break,
            }
        }
        steps
    };
    let glide = run(true);
    assert!(glide.len() > 10, "glides on for a while: {}", glide.len());
    assert!(glide.windows(2).all(|w| w[1] < w[0]), "slows down");
    assert!(glide[0] > 15.0 && glide[0] < 25.0, "starts at the drag's speed: {}", glide[0]);
    assert!(glide.len() < 200, "and comes to rest");
    assert!(run(false).is_empty(), "Flick Panning off stops where the drag ended");
}

fn wheel_event(unit: MouseWheelUnit, dy: f32, phase: TouchPhase) -> Event {
    Event::MouseWheel { unit, delta: vec2(0.0, dy), phase, modifiers: Modifiers::NONE, source: MouseWheelSource::Unknown }
}

// A point-unit swipe ending with TouchPhase::End glides; a notched wheel never does; two still passes stop it.
#[test]
fn the_touchpad_glide_starts_only_after_a_lift() {
    const DT: f64 = 1.0 / 60.0;
    // Six passes of 25 points and the lift one pass later; returns the time of the lift.
    let swipe = |glide: &mut TouchpadGlide, unit: MouseWheelUnit| {
        let mut now = 0.0;
        glide.read(&[wheel_event(unit, -25.0, TouchPhase::Start)], now, true);
        for _ in 0..6 {
            now += DT;
            glide.read(&[wheel_event(unit, -25.0, TouchPhase::Move)], now, true);
        }
        now += DT;
        glide.read(&[wheel_event(unit, 0.0, TouchPhase::End)], now, true);
        now
    };
    let mut glide = TouchpadGlide::new();
    let now = swipe(&mut glide, MouseWheelUnit::Point);
    let first = glide.step(now + DT);
    assert!(first.y < -15.0 && first.x == 0.0, "keeps the swipe's direction: {first:?}");
    let second = glide.step(now + 2.0 * DT);
    assert!(second.y < 0.0 && second.y > first.y, "slows: {first:?} {second:?}");
    // Any wheel input stops it.
    glide.read(&[wheel_event(MouseWheelUnit::Point, 5.0, TouchPhase::Start)], now + 3.0 * DT, true);
    assert_eq!(glide.step(now + 4.0 * DT), vec2(0.0, 0.0));

    let mut notched = TouchpadGlide::new();
    let now = swipe(&mut notched, MouseWheelUnit::Line);
    assert_eq!(notched.step(now + DT), vec2(0.0, 0.0), "a notched wheel never glides");

    // Against an edge two passes without a move end the glide.
    let mut edge = TouchpadGlide::new();
    let now = swipe(&mut edge, MouseWheelUnit::Point);
    assert!(edge.step(now + DT).y < 0.0);
    edge.moved(false);
    edge.moved(false);
    assert_eq!(edge.step(now + 2.0 * DT), vec2(0.0, 0.0));
}

// Dragging a quarter turn about the centre adds 90°; mirrored it subtracts; Shift snaps to 15°.
#[test]
fn a_rotate_drag_follows_the_pointer_and_snaps_with_shift() {
    let v = view(1.0, Point::new(0.0, 0.0));
    let c = v.rect.center();
    let turn = RotateView::begin(&v, pos2(c.x + 100.0, c.y));
    let down = pos2(c.x, c.y + 100.0);
    assert!((turn.drag(down, false, false) - 90.0).abs() < 1e-9);
    assert!((turn.drag(down, false, true) + 90.0).abs() < 1e-9);
    let at = |deg: f64| pos2(c.x + 100.0 * deg.to_radians().cos() as f32, c.y + 100.0 * deg.to_radians().sin() as f32);
    assert!((turn.drag(at(100.0), true, false) - 105.0).abs() < 1e-9);
    assert!((turn.drag(at(100.0), false, false) - 100.0).abs() < 1e-3);
    // The drag adds to the angle the view had.
    let mut start = v;
    start.rotation = 170.0;
    let turn = RotateView::begin(&start, pos2(c.x + 100.0, c.y));
    assert!((turn.drag(down, false, false) + 100.0).abs() < 1e-9, "170 + 90 wraps to -100");
    // A pan keeps the document under the pointer in a turned view.
    let mut turned = start;
    let doc = turned.to_doc(pos2(100.0, 100.0));
    pan(&mut turned, vec2(30.0, -20.0));
    assert!((turned.to_screen(doc) - pos2(130.0, 80.0)).length() < 1e-3);
}

// 190 wraps to -170, -180 to 180, NaN to 0.
#[test]
fn angles_wrap_into_a_half_turn_each_way() {
    assert_eq!(RotateView::wrap_deg(190.0), -170.0);
    assert_eq!(RotateView::wrap_deg(-180.0), 180.0);
    assert_eq!(RotateView::wrap_deg(180.0), 180.0);
    assert_eq!(RotateView::wrap_deg(725.0), 5.0);
    assert_eq!(RotateView::wrap_deg(f64::NAN), 0.0);
    assert_eq!(RotateView::snap_deg(100.0, 15.0), 105.0);
    assert_eq!(RotateView::snap_deg(100.0, 0.0), 100.0, "a step that is not positive is ignored");
    assert_eq!(RotateView::snap_deg(179.0, 15.0), 180.0);
}

struct Compass {
    view: CanvasView,
    reset: bool,
    /// The compass's centre, as the last pass placed it.
    at: egui::Pos2,
}

fn click(h: &mut Harness<'_, Compass>, at: egui::Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
}

// No compass at 0°; at 30° a click on it gives reset.
#[test]
fn the_compass_shows_when_turned_and_a_click_resets() {
    let rect = egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0));
    let upright = CanvasView::new(rect, Point::new(0.0, 0.0), 1.0);
    let mut h =
        Harness::<Compass>::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, state: &mut Compass| {
                let tokens = Tokens::of(ui);
                state.at = RotateView::compass_rect(state.view.rect, &tokens).center();
                state.reset |= RotateView::compass(ui, &state.view, &tokens).reset;
            },
            Compass { view: upright, reset: false, at: pos2(0.0, 0.0) },
        );
    h.run_steps(2);
    let at = h.state().at;
    click(&mut h, at);
    assert!(!h.state().reset, "no compass at 0 degrees");
    h.state_mut().view = CanvasView { rotation: 30.0, ..upright };
    h.run_steps(2);
    click(&mut h, at);
    assert!(h.state().reset, "a click on the compass resets the angle");
}

#[test]
fn page_keys_run_for_a_focused_canvas_or_button_but_not_a_claiming_widget() {
    use crate::behavior::ClaimedKey;
    let page_down = || Event::Key { key: Key::PageDown, physical_key: Some(Key::PageDown), pressed: true, repeat: false, modifiers: Modifiers::NONE };
    for (canvas, claims, moved) in
        [(true, vec![], true), (false, vec![], true), (false, vec![ClaimedKey::exact(Modifiers::NONE, Key::PageDown)], false)]
    {
        let ctx = Context::default();
        crate::focus::key_owner_tests::focus_widget_claiming(&ctx, egui::Id::unique("w"), canvas, &claims);
        let mut v = view(1.0, Point::new(400.0, 300.0));
        let before = v.center;
        let mut ran = false;
        ctx.run_ui(RawInput { events: vec![page_down()], ..Default::default() }, |ui| {
            ran = PageKey::handle(ui.ctx(), &mut v, Size::new(4000.0, 4000.0), true);
        })
        .drop_without_applying_deltas();
        assert_eq!((ran, v.center != before), (moved, moved));
    }
}

// A page view's wheel: Ctrl/Cmd + wheel and pinch zoom by `zoom_delta`, the wheel scrolls, and
// the navigation preferences do not apply.
#[test]
fn the_page_wheel_scrolls_and_zooms_by_zoom_delta() {
    assert_eq!(WheelNavigation::classify_page(vec2(0.0, 40.0), 1.0), Some(WheelAction::Pan(vec2(0.0, 40.0))));
    assert_eq!(WheelNavigation::classify_page(vec2(30.0, 0.0), 1.0), Some(WheelAction::Pan(vec2(30.0, 0.0))));
    assert_eq!(WheelNavigation::classify_page(vec2(0.0, 40.0), 1.25), Some(WheelAction::Zoom(1.25)));
    assert_eq!(WheelNavigation::classify_page(egui::Vec2::ZERO, 1.0), None);
}
