//! The frame loop: installed options, the one-press rule and the per-frame output.

use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::frames::announce_text;
use craft_ui::testing::input::{allow_zoom_keys, key_press, max_click_dist, set_max_click_duration};
use craft_ui::testing::{Frames, trial};
use egui::{Event, Modifiers, PointerButton, Pos2};

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
    let mut frames = Frames::new();
    let raw_frame = |frames: &mut Frames, events: Vec<Event>| frames.frame_without_input(events, |_| {});
    raw_frame(&mut frames, vec![]);
    assert!(frames.ctx.options(|o| o.quit_shortcuts.is_empty()), "install empties egui's quit shortcuts");
    let zoom = frames.ctx.zoom_factor();
    let key = |key| key_press(key, Modifiers::COMMAND);
    let mut closed = false;
    for key_pressed in [egui::Key::Equals, egui::Key::Plus, egui::Key::Q] {
        closed |= raw_frame(&mut frames, vec![key(key_pressed)]);
    }
    raw_frame(&mut frames, vec![]);
    raw_frame(&mut frames, vec![]);
    assert_eq!(frames.ctx.zoom_factor(), zoom, "Cmd+= left the zoom factor alone");
    assert!(!closed, "Cmd+Q asked for no close");
    // Control: with egui's own option back on, the same key zooms.
    allow_zoom_keys(&frames.ctx);
    raw_frame(&mut frames, vec![key(egui::Key::Equals)]);
    raw_frame(&mut frames, vec![]);
    raw_frame(&mut frames, vec![]);
    assert_ne!(frames.ctx.zoom_factor(), zoom, "control: egui zooms on Cmd+= when its option is on");
}

/// H2: the runtime owns the click options every pass: the slop token is the click distance and a click has no
/// time limit.
#[trial]
fn the_slop_token_is_the_click_distance_and_clicks_have_no_time_limit_in_every_pass() {
    let mut frames = Frames::new();
    let mut root = RootEnv::default();
    root.hit.slop = 5.0;
    frames.root(root);
    set_max_click_duration(&frames.ctx, 0.5);
    frames.idle(1, |_| {});
    assert_eq!(max_click_dist(&frames.ctx), 5.0);
    assert_eq!(frames.ctx.options(|o| o.input_options.max_click_duration), f64::INFINITY);
    set_max_click_duration(&frames.ctx, 0.5);
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
        announce_text(ui.ctx(), "saved");
        if std::mem::take(&mut first) {
            ui.ctx().request_discard("test");
        }
    });
    frames.idle(1, |_| {});
    assert_eq!(take_output(&frames.ctx).announcements, vec!["saved", "saved"]);
    assert!(take_output(&frames.ctx).announcements.is_empty(), "draining empties the output");
}

/// `install` on a context that already ran passes: the passes before the plugin existed have no hooks to
/// wait for, so the first `set_input` is no debug assertion failure.
#[trial]
fn install_on_a_context_that_already_ran_passes() {
    let mut frames = Frames::uninstalled();
    for _ in 0..3 {
        frames.frame_without_input(vec![], |_| {});
    }
    frames.install();
    frames.idle(3, |_| {});
}
