//! Margin handles.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::theme::{Base, install};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use egui_kittest::{Harness, HarnessBuilder};

use super::margins::*;
use super::view::PlacedPage;
use crate::setup::Margins;

fn rules() -> MarginRules {
    MarginRules { page: (612.0, 792.0), margins: Margins::uniform(72.0), gutter: 0.0, mirrored_even: false, snap: SNAP, min_text: MIN_TEXT }
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
