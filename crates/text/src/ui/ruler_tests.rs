//! The text ruler in a harness.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::edit::EditEvent;
use craft_ui::localized::units::LengthUnit;
use craft_ui::theme::{Base, install};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use egui_kittest::{Harness, HarnessBuilder};

use super::ruler::*;

const STRIP: Rect = Rect { min: pos2(100.0, 100.0), max: pos2(568.0, 120.0) };
const WIDTH: f32 = 468.0;

struct App {
    values: RulerValues,
    start: RulerValues,
    kind: TabAlign,
    right_to_left: bool,
    events: Vec<EditEvent<RulerValues>>,
}

fn open(values: RulerValues, right_to_left: bool) -> Harness<'static, App> {
    let app = App { start: values.clone(), values, kind: TabAlign::Left, right_to_left, events: Vec::new() };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(700.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let response = TextRuler::new("ruler", &app.values, WIDTH)
                    .unit(LengthUnit::Inches)
                    .kind(&mut app.kind)
                    .scale(Some(1.0))
                    .right_to_left(app.right_to_left)
                    .show(ui, STRIP);
                match &response.edit {
                    Some(EditEvent::Begin { .. }) => app.start = app.values.clone(),
                    Some(EditEvent::Cancel { .. }) => app.values = app.start.clone(),
                    Some(event) => {
                        if let Some(value) = event.value() {
                            app.values = value.clone();
                        }
                    }
                    None => {}
                }
                app.events.extend(response.edit);
            },
            app,
        );
    harness.run();
    harness
}

fn stop(position: f32, align: TabAlign) -> TabStop {
    TabStop { position, align, leader: TabLeader::None }
}

fn pointer(harness: &mut Harness<'_, App>, at: Pos2, button: Option<(PointerButton, bool)>) {
    harness.event(Event::PointerMoved(at));
    if let Some((button, pressed)) = button {
        harness.event(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

/// A press and release in one frame: the harness's frames are long enough for a slow press not to
/// count as a click.
fn click(harness: &mut Harness<'_, App>, at: Pos2, button: PointerButton) {
    pointer(harness, at, None);
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

/// Press at `from`, move through `via` to `to` and release there.
fn drag(harness: &mut Harness<'_, App>, from: Pos2, via: &[Pos2], to: Pos2) {
    pointer(harness, from, None);
    pointer(harness, from, Some((PointerButton::Primary, true)));
    for at in via {
        pointer(harness, *at, None);
    }
    pointer(harness, to, None);
    pointer(harness, to, Some((PointerButton::Primary, false)));
}

fn last_value(harness: &Harness<'_, App>) -> Option<RulerValues> {
    harness.state().events.last().and_then(|event| event.value().cloned())
}

// With the selector on Center a click at 1.03 in sets a centre stop at 1 in (snapped to 1/16 in); Alt-click sets it unsnapped; the selector cycles Left, Center, Right, Decimal, Bar and back with right-click.
#[test]
fn click_sets_a_stop_of_the_selectors_kind() {
    let mut harness = open(RulerValues::default(), false);
    harness.state_mut().kind = TabAlign::Center;
    click(&mut harness, pos2(STRIP.min.x + 1.03 * 72.0, 110.0), PointerButton::Primary);
    assert!(matches!(harness.state().events.as_slice(), [EditEvent::Set { .. }]), "{:?}", harness.state().events);
    assert_eq!(last_value(&harness).expect("a value").stops, [stop(72.0, TabAlign::Center)]);
    assert_eq!(snap_stop(1.03 * 72.0, 4.5, WIDTH, true), 1.03 * 72.0, "Alt places freely");

    let corner = pos2(STRIP.min.x - STRIP.height() / 2.0, 110.0);
    click(&mut harness, corner, PointerButton::Primary);
    assert_eq!(harness.state().kind, TabAlign::Right);
    click(&mut harness, corner, PointerButton::Secondary);
    click(&mut harness, corner, PointerButton::Secondary);
    assert_eq!(harness.state().kind, TabAlign::Left);

    let forward: Vec<TabAlign> = std::iter::successors(Some(TabAlign::Left), |kind| Some(kind.next(false))).take(6).collect();
    assert_eq!(forward, [TabAlign::Left, TabAlign::Center, TabAlign::Right, TabAlign::Decimal, TabAlign::Bar, TabAlign::Left]);
    assert_eq!(TabAlign::Left.next(true), TabAlign::Bar);
}

// Dragging a stop 20 pt below the strip draws it faded and on release commits values without it; Esc mid-drag cancels.
#[test]
fn dragging_a_stop_off_the_ruler_clears_it() {
    let values = RulerValues { stops: vec![stop(72.0, TabAlign::Left)], ..RulerValues::default() };
    let mut harness = open(values.clone(), false);
    let glyph = pos2(STRIP.min.x + 72.0, STRIP.max.y - 2.0);
    let off = pos2(glyph.x + 30.0, STRIP.max.y + 20.0);
    drag(&mut harness, glyph, &[glyph + vec2(15.0, 0.0)], off);
    let events = &harness.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(
        events.iter().filter(|e| !matches!(e, EditEvent::Commit { .. })).all(|e| e.value().is_none_or(|v| v.stops.len() == 1)),
        "the stop stays until release: {events:?}"
    );
    let Some(EditEvent::Commit { value, changed, .. }) = events.last() else { panic!("{events:?}") };
    assert!(*changed && value.stops.is_empty(), "{events:?}");

    harness.state_mut().values = values.clone();
    harness.state_mut().events.clear();
    harness.run();
    pointer(&mut harness, glyph, Some((PointerButton::Primary, true)));
    pointer(&mut harness, glyph + vec2(15.0, 0.0), None);
    pointer(&mut harness, off, None);
    harness.key_press(Key::Escape);
    harness.run();
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Cancel { .. })), "{events:?}");
    assert_eq!(harness.state().values, values);
}

// Dragging the left box moves first-line and left together; the hanging triangle moves left only; the right triangle moves the right indent; each drag is one gesture.
#[test]
fn indent_markers_move_as_word_does() {
    let values = RulerValues { left_indent: 36.0, first_line_indent: 18.0, ..RulerValues::default() };
    let at = |position: f32, y: f32| pos2(STRIP.min.x + position, y);
    let cases = [
        // (marker position, y, move, left, first line relative to left, right)
        ("first line", at(54.0, STRIP.min.y + 2.0), 18.0, (36.0, 36.0, 0.0)),
        ("left box", at(36.0, STRIP.max.y - 2.0), 36.0, (72.0, 18.0, 0.0)),
        ("hanging", at(36.0, STRIP.max.y - 6.0), 36.0, (72.0, -18.0, 0.0)),
        ("right", at(WIDTH, STRIP.max.y - 2.0), -36.0, (36.0, 18.0, 36.0)),
    ];
    for (name, from, by, (left, first, right)) in cases {
        let mut harness = open(values.clone(), false);
        drag(&mut harness, from, &[from + vec2(by / 2.0, 0.0)], from + vec2(by, 0.0));
        let events = &harness.state().events;
        assert_eq!(events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count(), 1, "{name}: {events:?}");
        let Some(EditEvent::Commit { value, changed: true, .. }) = events.last() else { panic!("{name}: {events:?}") };
        assert_eq!((value.left_indent, value.first_line_indent, value.right_indent), (left, first, right), "{name}");
        assert!(value.stops.is_empty(), "{name}");
    }
}

// In a right-to-left paragraph a stop at 72 pt sits 72 pt from the strip's right end.
#[test]
fn right_to_left_mirrors_positions() {
    let mut harness = open(RulerValues::default(), true);
    click(&mut harness, pos2(STRIP.max.x - 72.0, 110.0), PointerButton::Primary);
    assert_eq!(last_value(&harness).expect("a value").stops, [stop(72.0, TabAlign::Left)]);
    harness.state_mut().values = RulerValues::default();
    click(&mut harness, pos2(STRIP.min.x + 72.0, 110.0), PointerButton::Primary);
    assert_eq!(last_value(&harness).expect("a value").stops, [stop(WIDTH - 72.0, TabAlign::Left)]);
}
