//! Tests of the canvas claim and the primary press.

use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, pos2};

use super::claim::{PrimaryPress, click_count};

/// One pass at `time` in which the primary button goes down at `at`; the press's click count.
fn press_at(ctx: &Context, time: f64, at: Pos2) -> u32 {
    let mut count = 0;
    let events = vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }];
    let mut out = ctx.run_ui(RawInput { time: Some(time), events, ..RawInput::default() }, |ui| {
        let ctx = ui.ctx();
        count = PrimaryPress::current(ctx).expect("a primary press began").clicks(ctx);
    });
    out.textures_delta.clear();
    let release = vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }];
    let mut out = ctx.run_ui(RawInput { time: Some(time + 0.01), events: release, ..RawInput::default() }, |_ui| {});
    out.textures_delta.clear();
    count
}

// Two quick presses in one spot count 1 then 2; a third counts 3; a slow or distant press starts at 1.
#[test]
fn primary_press_counts_its_click_run() {
    let ctx = Context::default();
    let at = pos2(40.0, 40.0);
    assert_eq!(press_at(&ctx, 1.0, at), 1);
    assert_eq!(press_at(&ctx, 1.2, at), 2);
    assert_eq!(press_at(&ctx, 1.4, at), 3);
    assert_eq!(press_at(&ctx, 1.6, pos2(400.0, 40.0)), 1, "too far from the last press");
    assert_eq!(press_at(&ctx, 1.8, pos2(400.0, 40.0)), 2);
    assert_eq!(press_at(&ctx, 9.0, pos2(400.0, 40.0)), 1, "too long after the last press");
}

// Asking twice for the same press gives the same count.
#[test]
fn asking_again_for_a_press_keeps_its_count() {
    let ctx = Context::default();
    let at = pos2(10.0, 10.0);
    assert_eq!(click_count(&ctx, PointerButton::Primary, 1.0, at), 1);
    assert_eq!(click_count(&ctx, PointerButton::Primary, 1.1, at), 2);
    assert_eq!(click_count(&ctx, PointerButton::Primary, 1.1, at), 2);
    assert_eq!(click_count(&ctx, PointerButton::Primary, 1.1, at), 2);
}

// A press of another button starts a new click run; a run counts only its own button.
#[test]
fn a_run_counts_only_presses_of_one_button() {
    let ctx = Context::default();
    let at = pos2(10.0, 10.0);
    assert_eq!(click_count(&ctx, PointerButton::Secondary, 1.0, at), 1);
    assert_eq!(click_count(&ctx, PointerButton::Primary, 1.1, at), 1);
    assert_eq!(click_count(&ctx, PointerButton::Primary, 1.2, at), 2);
    assert_eq!(click_count(&ctx, PointerButton::Secondary, 1.3, at), 1);
}
