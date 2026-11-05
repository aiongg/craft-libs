//! Tests of Liquify's session, tools and keys.

use egui::{Context, Event, Key, Modifiers, RawInput};

use craft_ui::dialog::{LocalHistory, Step};
use craft_ui::edit::EditEvent;

use super::keys::LiquifyKeys;
use super::session::{LiquifyGlobal, LiquifySession, LiquifySettings, LiquifyStep, LiquifyStroke};
use super::tools::{LiquifyBrush, LiquifyTool, StrokeRecorder};

const ALT: Modifiers = Modifiers { alt: true, ..Modifiers::NONE };

fn stroke(tool: LiquifyTool, points: &[[f64; 3]]) -> LiquifyStroke {
    LiquifyStroke { tool, size: 100.0, density: 50.0, pressure: 100.0, rate: 80.0, points: points.to_vec(), amount: None }
}

fn key_event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

/// One pass that reads Liquify's keys from `events`.
fn read_keys(events: Vec<Event>, settings: &mut LiquifySettings) -> (bool, Vec<LiquifyGlobal>) {
    let ctx = Context::default();
    let mut out = (false, Vec::new());
    ctx.run_ui(RawInput { events, ..RawInput::default() }, |ui| out = LiquifyKeys::read(ui.ctx(), settings)).drop_without_applying_deltas();
    out
}

// W R E C S B O F D L H Z are the tools' letters (Twirl Counterclockwise has none); Alt reverses
// the twirls and swaps Pucker and Bloat, and leaves the others.
#[test]
fn liquify_tool_letters_and_alt_swaps() {
    let letters: String = LiquifyTool::ALL.iter().filter_map(|t| t.shortcut()).collect();
    assert_eq!(letters, "WRECSBOFDLHZ");
    assert_eq!(LiquifyTool::TwirlCounterclockwise.shortcut(), None);
    assert_eq!(LiquifyTool::TwirlClockwise.alternate(), LiquifyTool::TwirlCounterclockwise);
    assert_eq!(LiquifyTool::TwirlCounterclockwise.alternate(), LiquifyTool::TwirlClockwise);
    assert_eq!(LiquifyTool::Pucker.alternate(), LiquifyTool::Bloat);
    assert_eq!(LiquifyTool::Bloat.alternate(), LiquifyTool::Pucker);
    for tool in [LiquifyTool::ForwardWarp, LiquifyTool::Smooth, LiquifyTool::Freeze, LiquifyTool::FreezeLasso, LiquifyTool::Hand] {
        assert_eq!(tool.alternate(), tool);
    }
    let slots = LiquifyTool::slots(&super::LiquifyLabels::default());
    assert_eq!(slots.len(), LiquifyTool::ALL.len());
    let ends: Vec<LiquifyTool> = slots.iter().filter(|s| s.group_end).map(|s| s.tools[0].id).collect();
    assert_eq!(ends, [LiquifyTool::Smooth, LiquifyTool::PushLeft, LiquifyTool::Thaw, LiquifyTool::FreezeLasso]);
}

// A press, two drags and a release report Begin, Update, Update, Commit; each Update carries only
// its new point and the Commit the whole stroke; a press and release one pixel apart (under the
// drag threshold) is recorded whole with both points.
#[test]
fn liquify_stroke_events_begin_update_commit_and_short_strokes_are_recorded_whole() {
    let mut recorder = StrokeRecorder::default();
    let brush = LiquifyBrush::default();
    let begin = recorder.begin(LiquifyTool::Pucker, &brush, [10.0, 10.0, 1.0], ALT);
    assert!(recorder.is_active());
    let Some(first) = begin.value() else { panic!("Begin carries the one-point stroke") };
    assert_eq!(first.tool, LiquifyTool::Bloat);
    assert_eq!(first.points.len(), 1);
    let Some(EditEvent::Update { value: two, .. }) = recorder.extend([20.0, 10.0, 0.5], Modifiers::NONE) else { panic!("Update") };
    assert_eq!(two.points, [[20.0, 10.0, 0.5]], "an Update carries the new point only");
    assert_eq!(two.tool, LiquifyTool::Bloat, "and the stroke's tool and brush");
    assert!(recorder.extend([20.0, 10.0, 0.9], Modifiers::NONE).is_none(), "an equal point is skipped");
    let Some(EditEvent::Update { value: three, .. }) = recorder.extend([30.0, 12.0, 0.5], Modifiers::NONE) else { panic!("Update") };
    assert_eq!(three.points, [[30.0, 12.0, 0.5]]);
    for held in [4, 5] {
        let Some(EditEvent::Update { value, .. }) = recorder.hold([30.0, 12.0, 0.5], Modifiers::NONE) else { panic!("Update") };
        assert_eq!(value.points, [[30.0, 12.0, 0.5]], "a held dab {held} is its own point");
    }
    let Some(EditEvent::Commit { value, changed, id, .. }) = recorder.end([40.0, 12.0], Modifiers::NONE) else { panic!("Commit") };
    assert!(changed && !recorder.is_active());
    assert_eq!(Some(id), begin.id());
    assert_eq!(value.points.last(), Some(&[40.0, 12.0, 0.5]), "the release point keeps the last pressure");
    assert_eq!(value.points.len(), 6);

    recorder.begin(LiquifyTool::ForwardWarp, &brush, [5.0, 5.0, 1.0], Modifiers::NONE);
    let Some(EditEvent::Commit { value: short, .. }) = recorder.end([6.0, 5.0], Modifiers::NONE) else { panic!("Commit") };
    assert_eq!(short.points, [[5.0, 5.0, 1.0], [6.0, 5.0, 1.0]]);

    recorder.begin(LiquifyTool::ForwardWarp, &brush, [5.0, 5.0, 1.0], Modifiers::NONE);
    assert!(matches!(recorder.cancel(craft_ui::edit::CancelReason::Escape), Some(EditEvent::Cancel { .. })));
    assert!(recorder.end([1.0, 1.0], Modifiers::NONE).is_none());
}

// Undo after two strokes gives the first; redo gives both; 101 recorded steps keep at most 100
// undo steps, and the same number of redo steps after undoing them all.
#[test]
fn liquify_undo_redo_ask_the_app_to_replay_and_redo_is_bounded() {
    let mut session = LiquifySession::default();
    let mut history = LocalHistory::new(&session);
    for x in 0..2 {
        session.record(stroke(LiquifyTool::ForwardWarp, &[[f64::from(x), 0.0, 1.0]]));
        history.settle(&session, false);
    }
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(session.len(), 1);
    assert!(history.step(&mut session, Step::Redo));
    assert_eq!(session.len(), 2);

    for x in 2..101 {
        session.record(stroke(LiquifyTool::ForwardWarp, &[[f64::from(x), 0.0, 1.0]]));
        history.settle(&session, false);
    }
    assert_eq!(session.len(), 101);
    let mut undone = 0;
    while history.step(&mut session, Step::Undo) {
        undone += 1;
    }
    assert_eq!(undone, 100);
    let mut redone = 0;
    while history.step(&mut session, Step::Redo) {
        redone += 1;
    }
    assert_eq!(redone, 100);
}

// A closed Freeze Lasso records one stroke with amount 1 (0 with Alt at the press) and the first
// point again at its end; fewer than 3 points record nothing.
#[test]
fn liquify_freeze_lasso_records_a_stroke_and_alt_thaws() {
    let mut session = LiquifySession::default();
    let open = [[0.0, 0.0, 1.0], [10.0, 0.0, 1.0], [10.0, 10.0, 1.0]];
    let mut freeze = stroke(LiquifyTool::FreezeLasso, &open);
    freeze.amount = Some(1.0);
    session.record(freeze);
    let mut thaw = stroke(LiquifyTool::FreezeLasso, &[open[0], open[1], open[2], [0.2, 0.3, 1.0]]);
    thaw.amount = Some(0.0);
    session.record(thaw);
    session.record(stroke(LiquifyTool::FreezeLasso, &open[..2]));
    session.record(stroke(LiquifyTool::ForwardWarp, &[]));
    let [LiquifyStep::Stroke(first), LiquifyStep::Stroke(second)] = session.steps() else { panic!("two lassos recorded") };
    assert_eq!((first.amount, first.points.len(), first.points.last()), (Some(1.0), 4, Some(&open[0])));
    assert_eq!((second.amount, second.points.len()), (Some(0.0), 4), "a point within half a pixel closes it");
}

// Reconstruct, Mask All, None and Invert All, and ⌘I and ⌘D, record the global step; Restore All
// empties the session.
#[test]
fn liquify_global_buttons_and_keys_report_the_global_event() {
    let mut settings = LiquifySettings::default();
    let (changed, ops) = read_keys(vec![key_event(Key::D, Modifiers::COMMAND), key_event(Key::I, Modifiers::COMMAND)], &mut settings);
    assert!(!changed);
    assert_eq!(ops, [LiquifyGlobal::MaskNone, LiquifyGlobal::InvertAll]);
    let (_, ops) = read_keys(vec![key_event(Key::I, Modifiers::COMMAND), key_event(Key::D, Modifiers::COMMAND)], &mut settings);
    assert_eq!(ops, [LiquifyGlobal::InvertAll, LiquifyGlobal::MaskNone], "in the order pressed");

    let mut session = LiquifySession::default();
    for op in [LiquifyGlobal::Reconstruct { amount: 40.0 }, LiquifyGlobal::MaskAll, LiquifyGlobal::MaskNone, LiquifyGlobal::InvertAll] {
        session.record_global(op);
    }
    assert_eq!(session.len(), 4);
    assert_eq!(session.steps()[0], LiquifyStep::Global(LiquifyGlobal::Reconstruct { amount: 40.0 }));
    session.restore_all();
    assert!(session.is_empty());
}

// [ and ] change the size by at least one pixel within 1..=15000; ⌘H toggles Show Mask.
#[test]
fn liquify_brackets_scale_the_brush_and_command_h_toggles_the_mask() {
    let mut settings = LiquifySettings::default();
    let (changed, _) = read_keys(vec![key_event(Key::CloseBracket, Modifiers::NONE)], &mut settings);
    assert!(changed);
    assert_eq!(settings.brush.size, 110.0);
    read_keys(vec![key_event(Key::OpenBracket, Modifiers::NONE)], &mut settings);
    assert_eq!(settings.brush.size, 99.0);

    let at = |size| LiquifyBrush { size, ..LiquifyBrush::default() };
    assert_eq!(at(2.0).smaller().size, 1.0);
    assert_eq!(at(1.0).smaller().size, 1.0);
    assert_eq!(at(3.0).larger().size, 4.0, "at least one pixel of change");
    assert_eq!(at(14999.0).larger().size, LiquifyBrush::SIZE_MAX);
    assert_eq!(at(LiquifyBrush::SIZE_MAX).larger().size, LiquifyBrush::SIZE_MAX);

    assert!(settings.show_mask);
    let (changed, _) = read_keys(vec![key_event(Key::H, Modifiers::COMMAND)], &mut settings);
    assert!(changed && !settings.show_mask);
}

// The settings round-trip through serde; a partial record fills the defaults; `clamped` puts
// out-of-range numbers back in range.
#[test]
fn liquify_settings_round_trip_with_serde_defaults() {
    let settings = LiquifySettings { tool: LiquifyTool::Bloat, show_mesh: true, backdrop_opacity: 20.0, ..LiquifySettings::default() };
    let Ok(json) = serde_json::to_string(&settings) else { panic!("settings serialise") };
    assert_eq!(serde_json::from_str::<LiquifySettings>(&json).ok(), Some(settings));
    let partial = serde_json::from_str::<LiquifySettings>(r#"{"showMesh":true,"brush":{"size":40}}"#).ok();
    let Some(partial) = partial else { panic!("a partial record loads") };
    assert!(partial.show_mesh && partial.show_mask);
    assert_eq!((partial.brush.size, partial.brush.density), (40.0, 50.0));

    let mut wild = LiquifySettings::default();
    wild.brush.size = 1.0e9;
    wild.brush.density = -5.0;
    wild.brush.pressure = f64::NAN;
    wild.reconstruct_amount = 250.0;
    wild.backdrop_opacity = f64::INFINITY;
    // A non-finite number keeps the value in force, not the default.
    let mut old = LiquifySettings::default();
    old.brush.pressure = 70.0;
    old.backdrop_opacity = 20.0;
    let tame = wild.clamped(&old);
    assert_eq!((tame.brush.size, tame.brush.density, tame.brush.pressure), (LiquifyBrush::SIZE_MAX, 0.0, 70.0));
    assert_eq!((tame.reconstruct_amount, tame.backdrop_opacity), (100.0, 20.0));
}

// A stroke serialises with the engine's names (`twirlCw`, `lassoMask`, `points` as triples).
#[test]
fn liquify_strokes_serialise_as_the_engine_reads_them() {
    let mut lasso = stroke(LiquifyTool::FreezeLasso, &[[1.0, 2.0, 1.0]]);
    lasso.amount = Some(0.0);
    let Ok(value) = serde_json::to_value(&lasso) else { panic!("serialise") };
    assert_eq!(value["tool"], "lassoMask");
    assert_eq!(value["amount"], 0.0);
    assert_eq!(value["points"], serde_json::json!([[1.0, 2.0, 1.0]]));
    let Ok(twirl) = serde_json::to_value(stroke(LiquifyTool::TwirlClockwise, &[])) else { panic!("serialise") };
    assert_eq!(twirl["tool"], "twirlCw");
    assert!(twirl.get("amount").is_none());
    let Ok(ccw) = serde_json::to_value(LiquifyTool::TwirlCounterclockwise) else { panic!("serialise") };
    assert_eq!(ccw, "twirlCcw");
}
