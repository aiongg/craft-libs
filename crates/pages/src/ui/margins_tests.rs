//! Margin handles.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::testing::events;
use craft_ui::theme::{Base, install};
use egui::accesskit::{Action, ActionData, ActionRequest, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::Point;

use super::margins::*;
use super::view::PlacedPage;
use crate::setup::Margins;

fn rules() -> MarginRules {
    MarginRules {
        page: (612.0, 792.0),
        margins: Margins::uniform(72.0),
        gutter: 0.0,
        mirrored_even: false,
        snap: SNAP,
        min_text: MIN_TEXT,
        header: 0.0,
        footer: 0.0,
    }
}

// Dragging the left margin of a Letter page to 100 pt gives 99 (snapped to 4.5); to 600 pt it stops leaving 72 pt of text; Alt drags freely.
#[test]
fn drags_snap_and_leave_room() {
    let at = |at, rules: &MarginRules| dragged_margin(MarginEdge::Left, at, rules);
    assert_eq!(at(100.0, &rules()), MarginChange { edge: MarginEdge::Left, points: 99.0 });
    assert_eq!(at(600.0, &rules()).points, 612.0 - 72.0 - MIN_TEXT);
    assert_eq!(at(-20.0, &rules()).points, 0.0);
    assert_eq!(at(100.0, &MarginRules { snap: 0.0, ..rules() }).points, 100.0);
    assert_eq!(at(f64::NAN, &rules()).points, 72.0);
    let header = dragged_margin(MarginEdge::Header, 40.0, &rules());
    assert_eq!(header, MarginChange { edge: MarginEdge::Header, points: 40.5 });
}

// On an even page of mirrored margins the visible left edge edits the stored right margin with the pointer reflected.
#[test]
fn mirrored_even_pages_swap_edges() {
    let mirrored = MarginRules { mirrored_even: true, margins: Margins { left: 90.0, right: 36.0, top: 72.0, bottom: 72.0 }, ..rules() };
    assert_eq!(dragged_margin(MarginEdge::Left, 100.0, &mirrored), MarginChange { edge: MarginEdge::Right, points: 99.0 });
    assert_eq!(dragged_margin(MarginEdge::Right, 512.0, &mirrored), MarginChange { edge: MarginEdge::Left, points: 99.0 });
    assert_eq!(dragged_margin(MarginEdge::Top, 100.0, &mirrored).edge, MarginEdge::Top);
}

#[derive(Default)]
struct App {
    events: Vec<EditEvent<MarginChange>>,
}

const PAGE: Rect = Rect { min: pos2(100.0, 100.0), max: pos2(712.0, 892.0) };

fn open() -> Harness<'static, App> {
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 1000.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let page = PlacedPage { page: 0, rect: PAGE, scale: 1.0, band: (0.0, PAGE.height()) };
                let horizontal = Rect::from_min_max(pos2(70.0, 60.0), pos2(800.0, 80.0));
                let vertical = Rect::from_min_max(pos2(60.0, 90.0), pos2(80.0, 950.0));
                let response = MarginHandles::new("margins", &page, Margins::uniform(72.0)).show(ui, horizontal, vertical);
                app.events.extend(response.edit);
            },
            App::default(),
        );
    harness.run();
    harness
}

fn pointer(harness: &mut Harness<'_, App>, at: Pos2, pressed: Option<bool>) {
    harness.event(Event::PointerMoved(at));
    if let Some(pressed) = pressed {
        harness.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

// Press, two moves and release report Begin, Update, Update, Commit with changed true; Esc mid-drag reports Cancel.
#[test]
fn a_drag_is_one_gesture() {
    let mut harness = open();
    let edge = pos2(PAGE.min.x + 72.0, 70.0);
    pointer(&mut harness, edge, None);
    pointer(&mut harness, edge, Some(true));
    for dx in [30.0, 40.0] {
        pointer(&mut harness, edge + vec2(dx, 0.0), None);
    }
    pointer(&mut harness, edge + vec2(40.0, 0.0), Some(false));
    let events = &harness.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(events.iter().filter(|e| matches!(e, EditEvent::Update { .. })).count() >= 1, "{events:?}");
    let Some(EditEvent::Commit { value, changed, .. }) = events.last() else { panic!("{events:?}") };
    assert_eq!((*changed, value.edge, value.points), (true, MarginEdge::Left, 112.5));

    harness.state_mut().events.clear();
    pointer(&mut harness, edge, Some(true));
    pointer(&mut harness, edge + vec2(30.0, 0.0), None);
    harness.key_press(Key::Escape);
    harness.run();
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{events:?}");
}

fn set_of(harness: &Harness<'_, App>) -> Option<MarginChange> {
    harness.state().events.iter().rev().find_map(|e| if let EditEvent::Set { value, .. } = e { Some(*value) } else { None })
}

fn request(harness: &mut Harness<'_, App>, name: &str, action: Action, data: Option<ActionData>) {
    let (target_node, target_tree) = harness.get_by_role_and_label(Role::Splitter, name).accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(ActionRequest { action, target_tree, target_node, data }));
    harness.run();
}

// A screen reader's SetValue gives the margin itself (72 on the right edge is 72, not 540 from the page's left) and Increment makes it 4.5 larger on Right and Bottom; the advertised maximum leaves the minimum text.
#[test]
fn screen_reader_requests_set_and_step_the_margin_value() {
    for (name, edge) in [("Right Margin", MarginEdge::Right), ("Bottom Margin", MarginEdge::Bottom)] {
        let mut harness = open();
        let node = harness.get_by_role_and_label(Role::Splitter, name);
        let max = if edge == MarginEdge::Right { 612.0 - 72.0 - MIN_TEXT } else { 792.0 - 72.0 - MIN_TEXT };
        assert_eq!(node.accesskit_node().max_numeric_value(), Some(max), "{name}");
        request(&mut harness, name, Action::SetValue, Some(ActionData::NumericValue(36.0)));
        assert_eq!(set_of(&harness), Some(MarginChange { edge, points: 36.0 }), "{name} SetValue");

        let mut harness = open();
        request(&mut harness, name, Action::Increment, None);
        assert_eq!(set_of(&harness), Some(MarginChange { edge, points: 76.5 }), "{name} Increment");
        let mut harness = open();
        request(&mut harness, name, Action::Decrement, None);
        assert_eq!(set_of(&harness), Some(MarginChange { edge, points: 67.5 }), "{name} Decrement");
    }
}

// Arrow keys move the focused edge toward the arrow: → on the right edge makes the margin 4.5 smaller, ⇧← ten steps larger, ↓ on the bottom edge smaller.
#[test]
fn arrow_keys_move_the_edge_toward_the_arrow() {
    let mut harness = open();
    harness.get_by_role_and_label(Role::Splitter, "Right Margin").focus();
    harness.run();
    for (key, modifiers) in [(Key::ArrowRight, Modifiers::NONE), (Key::ArrowLeft, Modifiers::SHIFT)] {
        harness.state_mut().events.clear();
        for frame in events::key_press(key, modifiers) {
            for event in frame {
                harness.event(event);
            }
            harness.run();
        }
        let expected = if key == Key::ArrowRight { 67.5 } else { 72.0 + 45.0 };
        assert_eq!(set_of(&harness), Some(MarginChange { edge: MarginEdge::Right, points: expected }), "{key:?}");
    }
    let mut harness = open();
    harness.get_by_role_and_label(Role::Splitter, "Bottom Margin").focus();
    harness.run();
    for frame in events::key_press(Key::ArrowDown, Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.run();
    }
    assert_eq!(set_of(&harness), Some(MarginChange { edge: MarginEdge::Bottom, points: 67.5 }));
}

// The top margin never drops below the header's distance, nor the bottom below the footer's; a header stays within the top margin.
#[test]
fn header_and_footer_distances_bound_the_margins() {
    let rules = MarginRules { header: 36.0, footer: 27.0, ..rules() };
    assert_eq!(dragged_margin(MarginEdge::Top, 10.0, &rules).points, 36.0);
    assert_eq!(dragged_margin(MarginEdge::Bottom, 792.0 - 10.0, &rules).points, 27.0);
    assert_eq!(dragged_margin(MarginEdge::Header, 200.0, &rules).points, 72.0);
    assert_eq!(dragged_margin(MarginEdge::Footer, f64::NAN, &rules).points, 27.0);
    assert_eq!(margin_range(MarginEdge::Top, &rules), 36.0..=792.0 - 72.0 - MIN_TEXT);
}

// The spans of a 72 pt margin page at an origin: left, right, top and bottom parts in document coordinates.
#[test]
fn spans_follow_the_visible_edges() {
    let page = PlacedPage { page: 0, rect: PAGE, scale: 1.0, band: (0.0, PAGE.height()) };
    let handles = MarginHandles::new("margins", &page, Margins { left: 90.0, right: 36.0, top: 72.0, bottom: 50.0 });
    let (x, y) = handles.spans(Point::new(10.0, 20.0));
    assert_eq!((x, y), (vec![10.0..100.0, 586.0..622.0], vec![20.0..92.0, 762.0..812.0]));
    let even = MarginHandles::new("margins", &page, Margins { left: 90.0, right: 36.0, top: 72.0, bottom: 50.0 }).mirrored_even(true);
    let (x, _) = even.spans(Point::ORIGIN);
    assert_eq!(x, vec![0.0..36.0, 522.0..612.0]);
}
