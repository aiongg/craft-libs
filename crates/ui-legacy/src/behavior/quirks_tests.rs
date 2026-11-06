//! The input quirks.
#![expect(clippy::expect_used, reason = "test code")]

use egui::{Context, Event, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{ControlClick, SingleLineDrag, drop_touch_twins};

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn is_primary(e: &Event) -> bool {
    matches!(e, Event::PointerButton { button: PointerButton::Primary, .. })
}

// On macOS a Control press becomes secondary and its release too, even when Control is released
// first; other platforms and Control+Command stay primary.
#[test]
fn control_click_is_secondary_on_macos_only() {
    let p = pos2(50.0, 50.0);
    let mut state = ControlClick::default();
    let mut events = [button(p, true, Modifiers::CTRL), button(p, false, Modifiers::NONE)];
    state.normalize(&mut events, true);
    assert!(events.iter().all(|e| matches!(e, Event::PointerButton { button: PointerButton::Secondary, .. })));
    // A later plain click is primary again.
    let mut events = [button(p, true, Modifiers::NONE), button(p, false, Modifiers::NONE)];
    state.normalize(&mut events, true);
    assert!(events.iter().all(is_primary));

    for (macos, mods) in
        [(false, Modifiers::CTRL), (true, Modifiers::NONE), (true, Modifiers::CTRL | Modifiers::ALT), (true, Modifiers::CTRL | Modifiers::COMMAND)]
    {
        let mut events = vec![button(p, true, mods), button(p, false, mods)];
        ControlClick::default().normalize(&mut events, macos);
        assert!(events.iter().all(is_primary), "macos {macos}, {mods:?}");
    }
}

// A second primary press within half a second while held is dropped, as is a release while up.
#[test]
fn touch_twins_are_dropped() {
    let ctx = Context::default();
    let p = pos2(10.0, 10.0);
    let frame = |events: Vec<Event>, time: f64| {
        let mut raw = RawInput { time: Some(time), events, ..Default::default() };
        drop_touch_twins(&ctx, &mut raw);
        let kept = raw.events.len();
        ctx.run_ui(raw, |_| {}).textures_delta.clear();
        kept
    };
    assert_eq!(frame(vec![button(p, true, Modifiers::NONE)], 1.0), 1);
    assert_eq!(frame(vec![button(p, true, Modifiers::NONE)], 1.1), 0, "the twin press");
    assert_eq!(frame(vec![button(p, true, Modifiers::NONE)], 3.0), 1, "a press held longer passes");
    assert_eq!(frame(vec![button(p, false, Modifiers::NONE), button(p, false, Modifiers::NONE)], 3.1), 1, "one release per press");
    assert_eq!(frame(vec![button(p, false, Modifiers::NONE)], 3.2), 0, "a release while up");
    let all = vec![button(p, true, Modifiers::NONE), button(p, false, Modifiers::NONE), button(p, true, Modifiers::NONE)];
    assert_eq!(frame(all, 5.0), 3, "events that change the state are never twins");
}

const TEXT: &str = "abcdefghijklmnopqrst";
const LINES: &str = "first line\nsecond line\nthird line";

#[derive(Clone, Copy)]
enum Field {
    SingleLine,
    MultiLine,
}

fn frame(ctx: &Context, field: Field, id: Id, events: Vec<Event>) -> Rect {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events, ..Default::default() };
    let mut rect = Rect::NOTHING;
    ctx.run_ui(input, |ui| {
        ui.add_space(100.0);
        rect = match field {
            Field::SingleLine => ui.add(egui::TextEdit::singleline(&mut TEXT.to_string()).id(id).desired_width(400.0)).rect,
            Field::MultiLine => ui.add(egui::TextEdit::multiline(&mut LINES.to_string()).id(id).desired_width(400.0)).rect,
        };
    })
    .textures_delta
    .clear();
    rect
}

/// Press at `from`, drag to `to`, release; the selection as (anchor, end) character indices.
fn drag(field: Field, from: impl Fn(Rect) -> Pos2, to: impl Fn(Rect) -> Pos2) -> (usize, usize) {
    let ctx = Context::default();
    ctx.add_plugin(SingleLineDrag::default());
    let id = Id::unique("quirks-drag");
    let rect = frame(&ctx, field, id, vec![]);
    frame(&ctx, field, id, vec![]);
    let (from, to) = (from(rect), to(rect));
    frame(&ctx, field, id, vec![Event::PointerMoved(from), button(from, true, Modifiers::NONE)]);
    frame(&ctx, field, id, vec![Event::PointerMoved(from + vec2(12.0, 0.0))]);
    frame(&ctx, field, id, vec![Event::PointerMoved(to)]);
    frame(&ctx, field, id, vec![Event::PointerMoved(to)]);
    let state = egui::TextEdit::load_state(&ctx, id).expect("the field stored its state");
    let range = state.cursor.char_range().expect("the field has a cursor");
    (range.secondary.index.into(), range.primary.index.into())
}

// Dragging up and right in a single-line field selects right of the anchor; a multi-line field
// keeps its vertical drag.
#[test]
fn single_line_drag_ignores_vertical_drift() {
    let n = TEXT.chars().count();
    let (anchor, end) = drag(Field::SingleLine, |r| pos2(r.left() + 60.0, r.center().y), |r| pos2(r.right() + 50.0, r.top() - 60.0));
    assert!(anchor > 0 && anchor < n, "anchor {anchor} is mid-text");
    assert_eq!(end, n, "up and right selects to the end, not the start");

    let (anchor, end) = drag(Field::SingleLine, |r| pos2(r.left() + 60.0, r.center().y), |r| pos2(r.left() - 50.0, r.bottom() + 60.0));
    assert!(anchor > 0);
    assert_eq!(end, 0, "down and left selects to the start, not the end");

    let (anchor, end) = drag(Field::MultiLine, |r| pos2(r.left() + 30.0, r.top() + r.height() * 0.6), |r| pos2(r.left() + 30.0, r.top() - 40.0));
    assert!(anchor > "first line\nsecond line\n".len(), "anchor {anchor} is on the last line");
    assert_eq!(end, 0, "dragging above a multi-line field selects to its start");
}
