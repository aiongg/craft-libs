//! Tests of the arrow key nudge.

use egui::{Context, Event, Key, Modifiers, RawInput};
use kurbo::Vec2;

use super::nudge::{self, Nudge, NudgeKeys};
use crate::canvas::ToolKey;

fn press(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn tool_key(key: Key, modifiers: Modifiers) -> ToolKey {
    ToolKey { key, modifiers, pressed: true, repeat: false }
}

/// Runs one pass with `events`, calling `f` inside it, and returns what it gave.
fn pass<R>(ctx: &Context, events: Vec<Event>, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let mut f = Some(f);
    let mut out = None;
    ctx.run_ui(RawInput { events, ..Default::default() }, |ui| out = Some((f.take().unwrap())(ui))).drop_without_applying_deltas();
    out.unwrap()
}

fn read_with(keys: NudgeKeys, events: Vec<Event>) -> Option<Nudge> {
    pass(&Context::default(), events, |ui| keys.read(ui.ctx()))
}

// `NudgeKeys::read` gives 1 for a plain arrow, 10 with Shift, and the alt flag with `.alt(true)`.
#[test]
fn context_read_takes_plain_shift_and_alt_arrows() {
    let keys = NudgeKeys::new().alt(true);
    let n = read_with(keys, vec![press(Key::ArrowRight, Modifiers::NONE)]).unwrap();
    assert_eq!((n.by, n.shift, n.alt), (Vec2::new(1.0, 0.0), false, false));
    let n = read_with(keys, vec![press(Key::ArrowUp, Modifiers::SHIFT)]).unwrap();
    assert_eq!((n.by, n.shift, n.alt), (Vec2::new(0.0, -10.0), true, false));
    let n = read_with(keys, vec![press(Key::ArrowDown, Modifiers::ALT)]).unwrap();
    assert_eq!((n.by, n.shift, n.alt), (Vec2::new(0.0, 1.0), false, true));
    let n = read_with(keys, vec![press(Key::ArrowLeft, Modifiers::ALT | Modifiers::SHIFT)]).unwrap();
    assert_eq!((n.by, n.shift, n.alt), (Vec2::new(-10.0, 0.0), true, true));
    assert_eq!(read_with(NudgeKeys::new(), vec![press(Key::ArrowLeft, Modifiers::ALT)]), None, "alt is off by default");
    assert_eq!(read_with(keys, vec![press(Key::A, Modifiers::NONE)]), None);
    let n = read_with(NudgeKeys::new().steps(0.5, 5.0), vec![press(Key::ArrowDown, Modifiers::SHIFT)]).unwrap();
    assert_eq!(n.by, Vec2::new(0.0, 5.0));
}

// With `.base(Modifiers::COMMAND)` only ⌘/Ctrl arrows are read; without a base, they are not.
#[test]
fn base_modifier_must_be_held_and_nothing_else_of_command() {
    let cmd = Modifiers::COMMAND;
    let based = NudgeKeys::new().alt(true).base(cmd);
    assert!(read_with(based, vec![press(Key::ArrowRight, cmd)]).is_some());
    assert_eq!(read_with(based, vec![press(Key::ArrowRight, cmd | Modifiers::SHIFT)]).unwrap().by, Vec2::new(10.0, 0.0));
    assert!(read_with(based, vec![press(Key::ArrowRight, cmd | Modifiers::ALT)]).unwrap().alt);
    assert_eq!(read_with(based, vec![press(Key::ArrowRight, Modifiers::NONE)]), None, "base must be held");
    assert_eq!(read_with(based, vec![press(Key::ArrowRight, Modifiers::SHIFT)]), None);
    let plain = NudgeKeys::new();
    assert_eq!(read_with(plain, vec![press(Key::ArrowRight, cmd)]), None, "no base: command is not read");
    assert_eq!(read_with(plain, vec![press(Key::ArrowRight, Modifiers::CTRL)]), None);
}

// A focused text field, an open popup and a modal layer above `layer` each block `read`.
#[test]
fn reads_nothing_under_a_text_field_popup_or_modal() {
    let arrow = || vec![press(Key::ArrowRight, Modifiers::NONE)];
    let keys = NudgeKeys::new();

    let ctx = Context::default();
    let mut text = String::new();
    for _ in 0..2 {
        pass(&ctx, Vec::new(), |ui| ui.text_edit_singleline(&mut text).request_focus());
    }
    assert_eq!(pass(&ctx, arrow(), |ui| keys.read(ui.ctx())), None, "text field");

    let ctx = Context::default();
    let id = egui::Id::new("popup");
    for _ in 0..2 {
        pass(&ctx, Vec::new(), |ui| egui::Popup::open_id(ui.ctx(), id));
    }
    assert_eq!(pass(&ctx, arrow(), |ui| keys.read(ui.ctx())), None, "popup");

    let ctx = Context::default();
    for _ in 0..2 {
        pass(&ctx, Vec::new(), |ui| {
            egui::Modal::new(egui::Id::new("modal")).show(ui.ctx(), |ui| ui.label("dialog"));
        });
    }
    assert_eq!(pass(&ctx, arrow(), |ui| keys.read(ui.ctx())), None, "modal above the background");

    let ctx = Context::default();
    pass(&ctx, Vec::new(), |_| {});
    assert!(pass(&ctx, arrow(), |ui| keys.read(ui.ctx())).is_some(), "nothing open");
}

// `from_key` gives what `read` gives for the same key and modifiers, and consumes nothing.
#[test]
fn from_key_parses_a_tool_key_without_consuming() {
    let keys = NudgeKeys::new().alt(true).base(Modifiers::COMMAND);
    for modifiers in [Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT, Modifiers::COMMAND | Modifiers::ALT, Modifiers::NONE] {
        for key in [Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown, Key::Enter] {
            assert_eq!(keys.from_key(&tool_key(key, modifiers)), read_with(keys, vec![press(key, modifiers)]), "{key:?} {modifiers:?}");
        }
    }
    let ctx = Context::default();
    let left = vec![press(Key::ArrowLeft, Modifiers::NONE)];
    pass(&ctx, left, |ui| {
        let k = NudgeKeys::new();
        assert!(k.from_key(&tool_key(Key::ArrowLeft, Modifiers::NONE)).is_some());
        assert_eq!(ui.input(|i| i.events.len()), 1, "the event is still there");
        assert!(k.read(ui.ctx()).is_some());
        assert_eq!(ui.input(|i| i.events.len()), 0, "read consumed it");
    });
    let release = ToolKey { pressed: false, ..tool_key(Key::ArrowLeft, Modifiers::NONE) };
    assert_eq!(NudgeKeys::new().from_key(&release), None);
    let repeat = ToolKey { repeat: true, ..tool_key(Key::ArrowLeft, Modifiers::NONE) };
    assert!(NudgeKeys::new().from_key(&repeat).is_some());
}

// In a right-to-left UI Right is still +x and Left −x.
#[test]
fn right_is_plus_x_in_a_right_to_left_ui() {
    let ctx = Context::default();
    let n = pass(&ctx, vec![press(Key::ArrowRight, Modifiers::NONE), press(Key::ArrowLeft, Modifiers::NONE)], |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let keys = NudgeKeys::new();
            (keys.read(ui.ctx()).unwrap().by, keys.read(ui.ctx()).unwrap().by)
        })
        .inner
    });
    assert_eq!(n, (Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0)));
}

// The old `read(ui, ..)` and `read_with_alt` give the same moves as `NudgeKeys` with the ui's layer.
#[test]
fn old_readers_delegate_to_nudge_keys() {
    let ctx = Context::default();
    pass(&ctx, vec![press(Key::ArrowDown, Modifiers::SHIFT)], |ui| {
        assert_eq!(nudge::read(ui, 2.0, 20.0), Some(Vec2::new(0.0, 20.0)));
    });
    pass(&ctx, vec![press(Key::ArrowLeft, Modifiers::ALT)], |ui| {
        assert_eq!(nudge::read(ui, 1.0, 10.0), None, "plain read leaves Alt arrows");
        assert_eq!(nudge::read_with_alt(ui, 1.0, 10.0), Some((Vec2::new(-1.0, 0.0), true)));
        assert!(nudge::reads(ui));
    });
}
