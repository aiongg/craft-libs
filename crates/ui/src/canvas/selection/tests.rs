//! Tests of the selection frame, outlines and range helpers.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect as ScreenRect, Sense, pos2, vec2};
use egui_kittest::Harness;
use kurbo::{Point, Rect, Vec2};

use super::{SelectionFrame, range_bands, rect_around};
use crate::canvas::CanvasView;
use crate::canvas::quad::{self, BoxAllow, BoxPart, BoxRules, BoxState, TransformMode};
use crate::edit::{CancelReason, EditEvent};
use crate::theme::{Base, current, install};

/// The box before any drag.
const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

struct Host {
    state: BoxState,
    before: Option<BoxState>,
    rules: Option<BoxRules>,
    grips: bool,
    view: Option<CanvasView>,
    events: Vec<EditEvent<BoxState>>,
    picked: Option<BoxPart>,
    hovered: Option<BoxPart>,
    claimed: bool,
    hit: f32,
}

fn harness(setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        state: BoxState::from_rect(RECT),
        before: None,
        rules: None,
        grips: true,
        view: None,
        events: Vec::new(),
        picked: None,
        hovered: None,
        claimed: false,
        hit: 0.0,
    };
    setup(&mut host);
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            // Not a Tab stop, so Tab goes to the frame.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            host.hit = SelectionFrame::style(&current(ui.ctx())).handle.hit;
            let state = host.state;
            let mut frame = SelectionFrame::new("frame", &state).grips(host.grips);
            if let Some(rules) = host.rules {
                frame = frame.rules(rules);
            }
            let r = frame.show(ui, &canvas, &view);
            match &r.event {
                Some(EditEvent::Begin { value, .. }) => {
                    host.before = Some(host.state);
                    if let Some(value) = value {
                        host.state = *value;
                    }
                }
                Some(EditEvent::Update { value, .. }) => host.state = *value,
                Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                    host.before = None;
                    host.state = *value;
                }
                Some(EditEvent::Cancel { .. }) => {
                    if let Some(before) = host.before.take() {
                        host.state = before;
                    }
                }
                None => {}
            }
            host.events.extend(r.event);
            (host.picked, host.hovered, host.claimed) = (r.picked, r.hovered, r.claimed);
        },
        host,
    )
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    h.state().view.map_or(Pos2::ZERO, |v| v.to_screen(p))
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// Moves to `from`, presses, moves to `to` and releases there, with `modifiers` held.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, modifiers));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

/// Presses at `from` and moves to `to`, the button held.
fn hold(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-3
}

fn rect_of(h: &Harness<'_, Host>) -> Rect {
    let q = h.state().state.quad;
    Rect::new(q[0].x, q[0].y, q[2].x, q[2].y)
}

// Resize-only rules: a corner drag stretches freely and ⇧ keeps the proportions, ⌥ scales about the
// centre, an edge stretches one axis, the inside moves and ⇧ locks it to an axis.
#[test]
fn frame_resize_rules_follow_legacy_free() {
    let corner = |h: &Harness<'_, Host>| screen(h, Point::new(300.0, 220.0));
    let mut h = harness(|_| {});
    h.run();
    let from = corner(&h);
    drag(&mut h, from, from + vec2(20.0, 10.0), Modifiers::NONE);
    assert_eq!(rect_of(&h), Rect::new(100.0, 80.0, 320.0, 230.0), "a corner stretches freely");

    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, from, from + vec2(40.0, 0.0), Modifiers::SHIFT);
    let r = rect_of(&h);
    assert!(((r.width() / r.height()) - 200.0 / 140.0).abs() < 1e-6, "⇧ keeps the proportions: {r:?}");
    assert!(r.width() > 200.0);

    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, from, from + vec2(20.0, 10.0), Modifiers::ALT);
    let r = rect_of(&h);
    assert!(near(r.origin(), Point::new(80.0, 70.0)) && near(Point::new(r.x1, r.y1), Point::new(320.0, 230.0)), "⌥ scales about the centre: {r:?}");

    let mut h = harness(|_| {});
    h.run();
    let right = screen(&h, Point::new(300.0, 150.0));
    drag(&mut h, right, right + vec2(20.0, 30.0), Modifiers::NONE);
    assert_eq!(rect_of(&h), Rect::new(100.0, 80.0, 320.0, 220.0), "an edge stretches its one axis");

    let mut h = harness(|_| {});
    h.run();
    let inside = screen(&h, Point::new(200.0, 150.0));
    drag(&mut h, inside, inside + vec2(30.0, 5.0), Modifiers::NONE);
    assert_eq!(rect_of(&h), RECT.with_origin((130.0, 85.0)), "the inside moves");

    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, inside, inside + vec2(30.0, 5.0), Modifiers::SHIFT);
    assert_eq!(rect_of(&h), RECT.with_origin((130.0, 80.0)), "⇧ locks the move to an axis");
}

// With focus, Page Down picks the whole frame, then the corners and edges clockwise; an arrow moves
// the picked part by the step (⇧ the big step), one `Set` per press.
#[test]
fn frame_keyboard_parts_and_nudge() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert!(h.state().events.is_empty(), "the arrows stay with the host while the frame has no focus");

    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Inside), "focus picks the whole frame");
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().state, BoxState::from_rect(RECT).moved(Vec2::new(1.0, 0.0)));
    assert!(matches!(h.state().events.last(), Some(EditEvent::Set { .. })));
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(h.state().state, BoxState::from_rect(RECT).moved(Vec2::new(1.0, 10.0)));

    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Corner(0)));
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Corner(1)));
    h.key_press(Key::PageUp);
    h.run();
    let start = h.state().state;
    h.key_press(Key::ArrowLeft);
    h.run();
    let want = quad::drag(
        &BoxState { pivot: start.quad[0].midpoint(start.quad[2]), ..start },
        &start,
        BoxPart::Corner(0),
        start.quad[0],
        start.quad[0] + Vec2::new(-1.0, 0.0),
        Modifiers::NONE,
        &BoxRules::resize_only(),
    );
    assert!(near(h.state().state.quad[0], want.quad[0]) && near(h.state().state.quad[2], start.quad[2]), "a plain corner drag");
}

// Esc during a drag gives Cancel and leaves the frame focused; Esc with no drag gives nothing.
#[test]
fn frame_esc_cancels_drag_only() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert!(h.state().events.is_empty(), "Esc with no drag gives nothing");

    let from = screen(&h, Point::new(200.0, 150.0));
    hold(&mut h, from, from + vec2(40.0, 0.0));
    assert_eq!(rect_of(&h), RECT.with_origin((140.0, 80.0)), "the host shows the drag");
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })));
    assert_eq!(h.state().state, BoxState::from_rect(RECT), "Esc undoes the drag");
    let count = h.state().events.len();
    h.event(Event::PointerMoved(from + vec2(60.0, 0.0)));
    h.event(button(from + vec2(60.0, 0.0), false, Modifiers::NONE));
    h.run();
    assert_eq!(h.state().events.len(), count, "the rest of the press does nothing");
}

// Parts the rules refuse show no grip and take no press; `grips(false)` hides all and the inside
// still moves.
#[test]
fn frame_hides_grips_when_refused() {
    let distort = BoxRules { mode: TransformMode::Distort, allow: BoxAllow { rotate: false, skew: false, distort: true }, ..BoxRules::resize_only() };
    let mut h = harness(|h| h.rules = Some(distort));
    h.run();
    let edge = screen(&h, Point::new(300.0, 150.0));
    h.event(Event::PointerMoved(edge));
    h.run();
    assert_eq!(h.state().hovered, None, "an edge the rules refuse takes no press");
    let corner = screen(&h, Point::new(300.0, 220.0));
    h.event(Event::PointerMoved(corner));
    h.run();
    assert_eq!(h.state().hovered, Some(BoxPart::Corner(2)));

    let mut h = harness(|h| h.grips = false);
    h.run();
    let near_corner = screen(&h, Point::new(295.0, 215.0));
    h.event(Event::PointerMoved(near_corner));
    h.run();
    assert_eq!(h.state().hovered, Some(BoxPart::Inside), "no grip there");
    drag(&mut h, near_corner, near_corner + vec2(20.0, 20.0), Modifiers::NONE);
    assert_eq!(rect_of(&h), RECT.with_origin((120.0, 100.0)), "the inside still moves");
}

// A press outside the frame is not claimed and reaches the tool.
#[test]
fn frame_press_outside_is_the_tools() {
    let mut h = harness(|_| {});
    h.run();
    let outside = screen(&h, Point::new(30.0, 30.0));
    h.event(Event::PointerMoved(outside));
    h.event(button(outside, true, Modifiers::NONE));
    h.run();
    assert!(!h.state().claimed);
    assert_eq!(h.state().hovered, None);
    assert!(h.state().events.is_empty());
}

// `HandleStyle::hit` is a square's side: a press takes a grip within half of it of the grip's centre.
#[test]
fn frame_grip_reach_is_half_of_handle_hit() {
    let mut h = harness(|_| {});
    h.run();
    let reach = h.state().hit / 2.0;
    let left = screen(&h, Point::new(RECT.x0, 150.0));
    h.event(Event::PointerMoved(left - vec2(reach - 1.0, 0.0)));
    h.run();
    assert_eq!(h.state().hovered, Some(BoxPart::Edge(3)));
    h.event(Event::PointerMoved(left - vec2(reach + 1.0, 0.0)));
    h.run();
    assert_eq!(h.state().hovered, None, "past the reach and outside the box");
}

/// The item's frame without grips and the content frame inside it, each with its own box.
struct Content {
    item: BoxState,
    content: BoxState,
    view: Option<CanvasView>,
}

// A placed graphic's content frame (own colour, eight grips) inside the item's frame (grips off)
// takes the presses at its corners; the item's frame keeps moving by its inside.
#[test]
fn content_frame_carries_its_own_grips() {
    let host = Content {
        item: BoxState::from_rect(RECT),
        content: BoxState::from_rect(Rect::new(RECT.x0 + 20.0, RECT.y0 + 20.0, RECT.x1 - 20.0, RECT.y1 - 20.0)),
        view: None,
    };
    let mut h =
        Harness::<Content>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Content| {
                let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
                let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
                host.view = Some(view);
                let (item, content) = (host.item, host.content);
                let own = SelectionFrame::new("content", &content).color(egui::Color32::from_rgb(0xc4, 0x6f, 0x2b)).show(ui, &canvas, &view);
                if let Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }) = own.event {
                    host.content = value;
                }
                let frame = SelectionFrame::new("item", &item).grips(false).show(ui, &canvas, &view);
                if let Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }) = frame.event {
                    host.item = value;
                }
            },
            host,
        );
    h.run();
    let to_screen = |h: &Harness<'_, Content>, p: Point| h.state().view.map_or(Pos2::ZERO, |v| v.to_screen(p));
    let corner = to_screen(&h, Point::new(RECT.x1 - 20.0, RECT.y1 - 20.0));
    h.event(Event::PointerMoved(corner));
    h.event(button(corner, true, Modifiers::NONE));
    h.event(Event::PointerMoved(corner + vec2(5.0, 5.0)));
    h.event(Event::PointerMoved(corner + vec2(10.0, 10.0)));
    h.event(button(corner + vec2(10.0, 10.0), false, Modifiers::NONE));
    h.run();
    let content = h.state().content.quad;
    assert!(near(content[2], Point::new(RECT.x1 - 10.0, RECT.y1 - 10.0)), "the content corner followed: {content:?}");
    assert_eq!(h.state().item, BoxState::from_rect(RECT), "the item's frame did not move");
}

// `range_bands` gives the four sides cut to the clip, none for non-finite input; `rect_around` never
// overlaps, leaves the hole clear and keeps the area whole for a broken hole.
#[test]
fn range_bands_split_hole_and_clip() {
    let rect = ScreenRect::from_min_max(pos2(10.0, 10.0), pos2(50.0, 40.0));
    let clip = ScreenRect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
    let sides = range_bands(rect, 2.0, clip);
    assert_eq!(sides.len(), 4);
    assert_eq!(sides[0], ScreenRect::from_min_max(pos2(9.0, 9.0), pos2(11.0, 41.0)), "left first");
    assert_eq!(sides[2], ScreenRect::from_min_max(pos2(9.0, 9.0), pos2(51.0, 11.0)), "then top");

    // A column 20 million points tall: the sides stay near the clip.
    let column = ScreenRect::from_min_max(pos2(10.0, -1.0e7), pos2(50.0, 1.0e7));
    let sides = range_bands(column, 2.0, clip);
    assert_eq!(sides.len(), 2, "the top and bottom are outside the clip");
    assert!(sides.iter().all(|s| s.min.y >= -4.0 && s.max.y <= 104.0));
    assert!(range_bands(ScreenRect::from_min_max(pos2(f32::NAN, 0.0), pos2(1.0, 1.0)), 2.0, clip).is_empty());
    assert!(range_bands(rect, 2.0, ScreenRect::from_min_max(pos2(0.0, 0.0), pos2(f32::INFINITY, 1.0))).is_empty());

    let area = ScreenRect::from_min_max(pos2(0.0, 0.0), pos2(30.0, 20.0));
    let hole = ScreenRect::from_min_max(pos2(10.0, 5.0), pos2(20.0, 10.0));
    let pieces = rect_around(area, hole);
    assert_eq!(pieces.len(), 4);
    let sum: f32 = pieces.iter().map(|r| r.area()).sum();
    assert_eq!(sum, area.area() - hole.area(), "the pieces cover the area but the hole");
    for (i, a) in pieces.iter().enumerate() {
        assert!(!a.intersects(hole.shrink(0.01)));
        for b in &pieces[i + 1..] {
            assert!(!a.shrink(0.01).intersects(*b), "{a:?} overlaps {b:?}");
        }
    }
    assert_eq!(rect_around(area, hole.translate(vec2(100.0, 0.0))), vec![area], "a hole that misses leaves the area whole");
    assert_eq!(rect_around(area, ScreenRect::from_min_max(pos2(f32::NAN, 5.0), pos2(20.0, 10.0))), vec![area], "a broken hole is dropped");
    let flush = ScreenRect::from_min_max(pos2(0.0, 0.0), pos2(30.0, 10.0));
    assert_eq!(rect_around(area, flush).len(), 1, "pieces with no size are dropped");
}
