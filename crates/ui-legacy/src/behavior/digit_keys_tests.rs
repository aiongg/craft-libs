//! Number keys for opacity.

use egui::{Event, Key, Modifiers, RawInput};

use super::{DigitEntry, DigitKeys};

fn entry(percent: f64, shift: bool) -> Option<DigitEntry> {
    Some(DigitEntry { percent, shift })
}

// 5 gives 50, 0 gives 100.
#[test]
fn a_digit_sets_ten_times_it_and_zero_a_hundred() {
    let mut keys = DigitKeys::default();
    assert_eq!(keys.press(5, false, 0.0), entry(50.0, false));
    assert_eq!(keys.press(0, false, 10.0), entry(100.0, false));
    assert_eq!(keys.press(10, false, 20.0), None, "not a digit");
}

// 4 then 5 within 0.8 s gives 40 then 45; after 0.8 s the second is 50.
#[test]
fn two_quick_digits_set_the_exact_value() {
    let mut keys = DigitKeys::default();
    assert_eq!(keys.press(4, false, 1.0), entry(40.0, false));
    assert_eq!(keys.press(5, false, 1.7), entry(45.0, false));
    assert_eq!(keys.press(4, false, 5.0), entry(40.0, false));
    assert_eq!(keys.press(5, false, 5.9), entry(50.0, false));
}

// 0 then 0 quickly gives 100 then 0.
#[test]
fn zero_zero_sets_zero() {
    let mut keys = DigitKeys::default();
    assert_eq!(keys.press(0, false, 0.0), entry(100.0, false));
    assert_eq!(keys.press(0, false, 0.1), entry(0.0, false));
}

// 4 then Shift+5 gives 40 then 50 with shift.
#[test]
fn a_shift_change_starts_a_new_pair() {
    let mut keys = DigitKeys::default();
    assert_eq!(keys.press(4, false, 0.0), entry(40.0, false));
    assert_eq!(keys.press(5, true, 0.1), entry(50.0, true));
}

fn press_event(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat, modifiers }
}

// handle returns None and consumes nothing while a TextEdit has focus or enabled is false.
#[test]
fn digits_are_left_to_a_focused_field() {
    let ctx = egui::Context::default();
    let mut keys = DigitKeys::default();
    let mut text = String::new();
    let events = || vec![press_event(Key::Num7, Modifiers::NONE, false)];

    // Disabled: left in the queue.
    ctx.run_ui(RawInput { events: events(), ..Default::default() }, |ui| {
        assert_eq!(keys.read(ui.ctx(), false), None);
        assert_eq!(ui.input(|i| i.events.len()), 1);
    })
    .drop_without_applying_deltas();

    // Enabled with nothing focused: read and consumed; modified and repeated presses behave.
    let mut got = None;
    let mut all = vec![press_event(Key::Num8, Modifiers::NONE, false)];
    all.push(press_event(Key::Num3, Modifiers::COMMAND, false));
    all.push(press_event(Key::Num8, Modifiers::NONE, true));
    ctx.run_ui(RawInput { events: all, ..Default::default() }, |ui| {
        got = keys.read(ui.ctx(), true);
        assert_eq!(ui.input(|i| i.events.len()), 1, "Cmd+3 is left for its shortcut; the repeat is consumed");
    })
    .drop_without_applying_deltas();
    assert_eq!(got, entry(80.0, false), "a repeat is not a second digit");

    // A focused text field keeps the keys.
    ctx.run_ui(RawInput::default(), |ui| {
        ui.text_edit_singleline(&mut text).request_focus();
    })
    .drop_without_applying_deltas();
    ctx.run_ui(RawInput { events: events(), ..Default::default() }, |ui| {
        assert_eq!(keys.read(ui.ctx(), true), None);
        assert_eq!(ui.input(|i| i.events.len()), 1);
    })
    .drop_without_applying_deltas();
}

#[test]
fn digits_run_for_a_focused_canvas_or_button_but_not_a_claiming_widget() {
    use crate::behavior::ClaimedKey;
    for (canvas, claims, expected) in
        [(true, vec![], entry(60.0, false)), (false, vec![], entry(60.0, false)), (false, vec![ClaimedKey::exact(Modifiers::NONE, Key::Num6)], None)]
    {
        let ctx = egui::Context::default();
        crate::focus::key_owner_tests::focus_widget_claiming(&ctx, egui::Id::unique("w"), canvas, &claims);
        let mut keys = DigitKeys::default();
        let mut got = None;
        ctx.run_ui(RawInput { events: vec![press_event(Key::Num6, Modifiers::NONE, false)], ..Default::default() }, |ui| {
            got = keys.read(ui.ctx(), true);
        })
        .drop_without_applying_deltas();
        assert_eq!(got, expected);
    }
}
