//! Momentary keys and spring-loaded tool letters.

use egui::{Context, Event, Id, Key, Modifiers, PointerButton, RawInput, pos2};

use super::{CanvasId, MomentaryKeys, MomentaryTool, SpringTool};

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers }
}

fn button(pressed: bool) -> Event {
    Event::PointerButton { pos: pos2(10.0, 10.0), button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// Runs one pass at `time` with `events`; `modifiers` is what is held during it.
fn pass<R>(ctx: &Context, time: f64, modifiers: Modifiers, mut events: Vec<Event>, f: impl FnOnce(&Context) -> R) -> R {
    events.insert(0, Event::ModifiersChanged(modifiers));
    let mut f = Some(f);
    let mut result = None;
    let mut output = ctx.run_ui(RawInput { time: Some(time), events, ..RawInput::default() }, |ui| {
        if let Some(f) = f.take() {
            result = Some(f(ui.ctx()));
        }
    });
    output.textures_delta.clear();
    result.expect("the pass ran")
}

const CANVAS: fn() -> CanvasId = || CanvasId::from_id(Id::unique("canvas"));

fn held(ctx: &Context, time: f64, modifiers: Modifiers, events: Vec<Event>) -> Option<MomentaryTool> {
    pass(ctx, time, modifiers, events, |ctx| MomentaryKeys::default().update(ctx, CANVAS(), false))
}

// Space then Cmd and Cmd then Space both give ZoomIn; Space alone gives Hand.
#[test]
fn space_holds_the_hand_and_cmd_space_zooms_in_in_either_order() {
    let ctx = Context::default();
    assert_eq!(held(&ctx, 0.0, Modifiers::NONE, vec![key_event(Key::Space, true, Modifiers::NONE)]), Some(MomentaryTool::Hand));
    assert_eq!(held(&ctx, 0.1, Modifiers::COMMAND, vec![]), Some(MomentaryTool::ZoomIn));
    assert_eq!(held(&ctx, 0.2, Modifiers::NONE, vec![key_event(Key::Space, false, Modifiers::NONE)]), None);

    let ctx = Context::default();
    assert_eq!(held(&ctx, 0.0, Modifiers::COMMAND, vec![]), None);
    assert_eq!(held(&ctx, 0.1, Modifiers::COMMAND, vec![key_event(Key::Space, true, Modifiers::COMMAND)]), Some(MomentaryTool::ZoomIn));
}

// Cmd+Alt+Space gives ZoomOut.
#[test]
fn alt_turns_a_held_zoom_in_into_zoom_out() {
    let ctx = Context::default();
    let both = Modifiers::COMMAND | Modifiers::ALT;
    assert_eq!(held(&ctx, 0.0, both, vec![key_event(Key::Space, true, both)]), Some(MomentaryTool::ZoomOut));
    let ctx = Context::default();
    assert_eq!(held(&ctx, 0.0, Modifiers::ALT, vec![key_event(Key::Space, true, Modifiers::ALT)]), Some(MomentaryTool::ZoomOut));
}

// Space up during a Hand drag still gives Hand until the pass the button is released.
#[test]
fn a_drag_keeps_its_temporary_tool_until_the_release() {
    let ctx = Context::default();
    let n = Modifiers::NONE;
    assert_eq!(held(&ctx, 0.0, n, vec![key_event(Key::Space, true, n)]), Some(MomentaryTool::Hand));
    assert_eq!(held(&ctx, 0.1, n, vec![Event::PointerMoved(pos2(10.0, 10.0)), button(true)]), Some(MomentaryTool::Hand));
    assert_eq!(held(&ctx, 0.2, n, vec![key_event(Key::Space, false, n)]), Some(MomentaryTool::Hand), "still dragging");
    assert_eq!(held(&ctx, 0.3, n, vec![button(false)]), Some(MomentaryTool::Hand), "the release pass");
    assert_eq!(held(&ctx, 0.4, n, vec![]), None);
}

// Space pressed in the middle of a stroke starts no Hand: the stroke finishes with its own tool.
#[test]
fn a_temporary_tool_cannot_start_in_the_middle_of_a_drag() {
    let ctx = Context::default();
    let n = Modifiers::NONE;
    assert_eq!(held(&ctx, 0.0, n, vec![Event::PointerMoved(pos2(10.0, 10.0)), button(true)]), None, "the stroke begins with the selected tool");
    assert_eq!(held(&ctx, 0.1, n, vec![key_event(Key::Space, true, n)]), None, "Space mid-stroke");
    assert_eq!(held(&ctx, 0.2, n, vec![button(false)]), None, "the release pass still belongs to the stroke");
    assert_eq!(held(&ctx, 0.3, n, vec![]), Some(MomentaryTool::Hand), "Space is still down: the Hand for the next gesture");
}

// With drawing true, Space gives None.
#[test]
fn space_while_drawing_is_not_the_hand() {
    let ctx = Context::default();
    let n = Modifiers::NONE;
    let tool = pass(&ctx, 0.0, n, vec![key_event(Key::Space, true, n)], |ctx| MomentaryKeys::default().update(ctx, CANVAS(), true));
    assert_eq!(tool, None);
}

// A focused TextEdit leaves Space to the field.
#[test]
fn no_temporary_tool_while_a_text_field_has_the_keys() {
    let ctx = Context::default();
    let n = Modifiers::NONE;
    let mut text = String::new();
    let field = Id::unique("field");
    for time in [0.0, 0.1] {
        let mut output = ctx.run_ui(RawInput { time: Some(time), ..RawInput::default() }, |ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut text).id(field));
            response.request_focus();
        });
        output.textures_delta.clear();
    }
    assert!(ctx.text_edit_focused());
    assert_eq!(held(&ctx, 0.2, n, vec![key_event(Key::Space, true, n)]), None);
}

fn tick(spring: &mut SpringTool<char>, ctx: &Context, time: f64, events: Vec<Event>, current: char, in_gesture: bool) -> Option<char> {
    pass(ctx, time, Modifiers::NONE, events, |ctx| ctx.input(|i| spring.tick(i, &current, in_gesture)))
}

fn down(ctx: &Context, time: f64) {
    pass(ctx, time, Modifiers::NONE, vec![key_event(Key::Z, true, Modifiers::NONE)], |_| ());
}

fn up(ctx: &Context, time: f64, spring: &mut SpringTool<char>, current: char, in_gesture: bool) -> Option<char> {
    tick(spring, ctx, time, vec![key_event(Key::Z, false, Modifiers::NONE)], current, in_gesture)
}

// Held 0.3 s, releasing the letter returns the previous tool.
#[test]
fn a_held_letter_springs_back_on_release() {
    let ctx = Context::default();
    let mut spring = SpringTool::default();
    down(&ctx, 1.0);
    spring.pressed(Key::Z, 'b', 'z', true, 1.0);
    assert_eq!(spring.previous(), Some(&'b'));
    assert_eq!(tick(&mut spring, &ctx, 1.2, vec![], 'z', false), None, "the letter is still down");
    assert_eq!(up(&ctx, 1.3, &mut spring, 'z', false), Some('b'));
    assert_eq!(spring.previous(), None);
}

// A tap under 0.3 s with no canvas use keeps the new tool.
#[test]
fn a_quick_tap_switches_for_good() {
    let ctx = Context::default();
    let mut spring = SpringTool::default();
    down(&ctx, 1.0);
    spring.pressed(Key::Z, 'b', 'z', true, 1.0);
    assert_eq!(up(&ctx, 1.1, &mut spring, 'z', false), None);
    assert_eq!(spring.previous(), None);
}

// canvas_used during a short hold springs back on release.
#[test]
fn using_the_canvas_makes_a_short_hold_temporary() {
    let ctx = Context::default();
    let mut spring = SpringTool::default();
    down(&ctx, 1.0);
    spring.pressed(Key::Z, 'b', 'z', true, 1.0);
    spring.canvas_used();
    assert_eq!(up(&ctx, 1.1, &mut spring, 'z', false), Some('b'));
}

// tick returns None while in_gesture, the previous tool after.
#[test]
fn a_drag_running_at_release_finishes_with_the_sprung_tool() {
    let ctx = Context::default();
    let mut spring = SpringTool::default();
    down(&ctx, 1.0);
    spring.pressed(Key::Z, 'b', 'z', true, 1.0);
    assert_eq!(up(&ctx, 1.1, &mut spring, 'z', true), None, "a gesture runs");
    assert_eq!(tick(&mut spring, &ctx, 1.2, vec![], 'z', true), None);
    assert_eq!(tick(&mut spring, &ctx, 1.3, vec![], 'z', false), Some('b'));
}
