//! The frame loop: installed options, the one-press rule and the per-frame output.

use crate::support::Frames;
use craft_ui::runtime::frame::{announce, take_output};
use craft_ui::testing::trial;
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

const AT: Pos2 = Pos2::new(20.0, 12.0);

fn press(pressed: bool) -> Event {
    Event::PointerButton { pos: AT, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn kinds(events: &[Event]) -> Vec<&'static str> {
    events
        .iter()
        .map(|event| match event {
            Event::PointerButton { pressed: true, .. } => "press",
            Event::PointerButton { pressed: false, .. } => "release",
            Event::PointerMoved(_) => "move",
            _ => "other",
        })
        .collect()
}

/// Skeleton: `frames_state::cmd_equals_does_not_zoom_the_ui_and_cmd_q_sends_no_close`. The frames run without
/// `set_input`, which would write the root scale as the zoom factor every frame.
#[trial]
fn cmd_equals_does_not_zoom_the_ui_and_cmd_q_sends_no_close() {
    let frames = Frames::new();
    let mut time = 0.0;
    let mut raw_frame = |events: Vec<Event>| {
        time += 1.0 / 60.0;
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))), time: Some(time), events, ..RawInput::default() };
        let mut out = frames.ctx.run_ui(input, |_| {});
        out.textures_delta.clear();
        out
    };
    raw_frame(vec![]);
    assert!(frames.ctx.options(|o| o.quit_shortcuts.is_empty()), "install empties egui's quit shortcuts");
    let zoom = frames.ctx.zoom_factor();
    let key = |key| Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: Modifiers::COMMAND };
    let mut closed = false;
    for key_pressed in [egui::Key::Equals, egui::Key::Plus, egui::Key::Q] {
        let out = raw_frame(vec![key(key_pressed)]);
        closed |= out.viewport_output.values().any(|v| v.commands.iter().any(|c| matches!(c, egui::ViewportCommand::Close)));
    }
    raw_frame(vec![]);
    raw_frame(vec![]);
    assert_eq!(frames.ctx.zoom_factor(), zoom, "Cmd+= left the zoom factor alone");
    assert!(!closed, "Cmd+Q asked for no close");
    // Control: with egui's own option back on, the same key zooms.
    frames.ctx.options_mut(|o| o.zoom_with_keyboard = true);
    raw_frame(vec![key(egui::Key::Equals)]);
    raw_frame(vec![]);
    raw_frame(vec![]);
    assert_ne!(frames.ctx.zoom_factor(), zoom, "control: egui zooms on Cmd+= when its option is on");
}

/// Replaces the skeleton's slop test (`runtime_writes_the_slop_stand_in_to_max_click_dist_every_pass`): the
/// runtime owns the click duration option every pass.
#[trial]
fn clicks_have_no_time_limit_in_every_pass() {
    let mut frames = Frames::new();
    frames.ctx.options_mut(|o| o.input_options.max_click_duration = 0.5);
    frames.idle(1, |_| {});
    assert_eq!(frames.ctx.options(|o| o.input_options.max_click_duration), f64::INFINITY);
    frames.ctx.options_mut(|o| o.input_options.max_click_duration = 0.5);
    frames.idle(1, |_| {});
    assert_eq!(frames.ctx.options(|o| o.input_options.max_click_duration), f64::INFINITY, "rewritten each pass");
}

/// H0: a pass carries at most one press, as its last pointer event; later events lead the next frame, and a
/// discard pass receives none of the held events.
#[trial]
fn one_press_per_pass_and_a_discard_pass_gets_no_held_events() {
    let mut frames = Frames::new();
    let seen = std::cell::RefCell::new(Vec::<Vec<&'static str>>::new());
    let run = |frames: &mut Frames, events: Vec<Event>, discard_first: bool| {
        seen.borrow_mut().clear();
        let mut first = true;
        frames.frame(events, |ui| {
            seen.borrow_mut().push(kinds(&ui.input(|i| i.events.clone())));
            if discard_first && std::mem::take(&mut first) {
                ui.ctx().request_discard("test");
            }
        });
        seen.borrow().clone()
    };
    assert_eq!(run(&mut frames, vec![Event::PointerMoved(AT), press(true), press(false)], false), vec![vec!["move", "press"]]);
    assert_eq!(run(&mut frames, vec![], false), vec![vec!["release"]], "the release comes one pass after its press");
    assert_eq!(
        run(&mut frames, vec![press(true), press(false), press(true)], true),
        vec![vec!["press"], vec![]],
        "the discard pass receives no held events"
    );
    assert_eq!(run(&mut frames, vec![], false), vec![vec!["release", "press"]], "the held events lead the next frame, and its own press is the last");
    assert_eq!(run(&mut frames, vec![], false), vec![Vec::<&str>::new()]);
}

/// 1.8: outputs accumulate across passes and discarded passes until drained, and drain once.
#[trial]
fn frame_output_accumulates_across_passes_until_taken() {
    let mut frames = Frames::new();
    let mut first = true;
    frames.frame(vec![], |ui| {
        announce(ui.ctx(), "saved".to_owned());
        if std::mem::take(&mut first) {
            ui.ctx().request_discard("test");
        }
    });
    frames.idle(1, |_| {});
    assert_eq!(take_output(&frames.ctx).announcements, vec!["saved", "saved"]);
    assert!(take_output(&frames.ctx).announcements.is_empty(), "draining empties the output");
}
