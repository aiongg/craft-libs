//! Binding sources: given and bound give equal events, a mixed value commits a concrete one, a kept value stays, a
//! check fixes or refuses.

use super::kit::{AT, CAP, DRAGGED, Ev, INITIAL, STEP, Scrub, Seen, ev_of, near};
use craft_behavior::{CancelReason, DocKey, EditEvent};
use craft_ui::contract::{Bind, Props as _, Show as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::style::Flags;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Ui, Vec2};
use std::cell::RefCell;

/// Hover, press and release a drag along the track: the first move passes the drag slop in one step, so the drag
/// starts and proposes its first value in one pass.
fn drag(frames: &mut Frames, show: &mut impl FnMut(&mut Ui)) {
    let at = Pos2::new(AT.left() + 10.0, AT.center().y);
    frames.idle(2, &mut *show);
    frames.frame(vec![moved(at)], &mut *show);
    frames.frame(vec![press(at)], &mut *show);
    frames.frame(vec![moved(at + Vec2::new(30.0, 0.0))], &mut *show);
    frames.frame(vec![moved(at + Vec2::new(40.0, 0.0))], &mut *show);
    frames.frame(vec![], &mut *show);
    frames.frame(vec![release(at + Vec2::new(40.0, 0.0))], &mut *show);
    frames.frame(vec![], &mut *show);
}

fn events(given: bool) -> (Vec<Ev>, f32) {
    let mut frames = Frames::new();
    let seen = RefCell::new(Vec::new());
    let mut value = INITIAL;
    let mut show = |ui: &mut Ui| {
        let bind = if given { Bind::given(value) } else { Bind::bound(&mut value) };
        let out = Scrub::new(bind).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().extend(ev_of(&out.event));
        // A given caller applies what it receives, as a document does.
        if given && let Some(event) = &out.event {
            event.apply(&mut value);
        }
    };
    drag(&mut frames, &mut show);
    (seen.take(), value)
}

/// One path: a write-through source and a value-in, event-out source produce the same events, also for a drag whose
/// `Begin` and first `Update` fall in one pass.
#[trial]
fn given_and_bound_bindings_give_equal_events() {
    let (given, given_value) = events(true);
    let (bound, bound_value) = events(false);
    assert_eq!(given, bound, "the events follow the source-independent rules");
    assert_eq!(given_value, bound_value);
    assert!(matches!(given.first(), Some(Ev::Begin)), "the folded Begin comes first: {given:?}");
    assert!(matches!(given.last(), Some(Ev::Commit(v)) if near(*v, DRAGGED)), "the drag commits its last value: {given:?}");
}

/// A given caller that applies only `Commit` keeps its drag: the host-change rule compares with what the pipeline
/// emitted, so the unchanged given value is not an app change.
#[trial]
fn a_given_drag_that_applies_only_the_commit_is_not_cancelled() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Vec::new());
    let mut value = INITIAL;
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::given(value)).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().extend(ev_of(&out.event));
        if let Some(EditEvent::Commit { after, .. }) = &out.event {
            value = *after;
        }
    };
    drag(&mut frames, &mut show);
    let seen = seen.take();
    assert!(!seen.iter().any(|event| matches!(event, Ev::Cancel(_))), "no host-change cancel: {seen:?}");
    assert!(matches!(seen.last(), Some(Ev::Commit(_))), "{seen:?}");
}

/// A kept binding holds its value in the component's own memory across frames.
#[trial]
fn a_kept_binding_holds_the_edited_value_across_frames() {
    let mut frames = Frames::new();
    let seen = Seen::default();
    let mut show = |ui: &mut Ui| {
        Scrub::new(Bind::kept()).seen(&seen).id_salt("scrub").show_at(ui, AT);
    };
    frames.idle(3, &mut show);
    assert_eq!(seen.shown.get(), None, "nothing kept yet");
    drag(&mut frames, &mut show);
    assert!(seen.shown.get().is_some_and(|v| near(v, DRAGGED)), "the dragged value stays shown: {:?}", seen.shown.get());
}

/// A gesture that begins and commits in one pass reaches the app as one `Set` that names itself as its `Begin` would.
#[trial]
fn a_gesture_in_one_pass_reaches_the_app_as_one_set_with_label_and_key() {
    let mut frames = Frames::new();
    let mut value = 0.0f32;
    let sets = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value)).one_pass().label("Nudge").doc_key(DocKey::new("trial.nudge")).id_salt("one").show_at(ui, AT);
        sets.borrow_mut().extend(out.event.clone());
        if let Some(EditEvent::Set { .. }) = &out.event {
            assert!(out.response.changed(), "a Set marks the response changed");
        }
    };
    let at = AT.center();
    frames.idle(3, &mut show);
    frames.frame(vec![moved(at)], &mut show);
    frames.frame(vec![press(at)], &mut show);
    frames.frame(vec![release(at)], &mut show);
    frames.idle(2, &mut show);
    let sets = sets.take();
    assert_eq!(value, STEP);
    assert_eq!(sets.len(), 1, "one event for the whole gesture: {sets:?}");
    let EditEvent::Set { label, doc_key, after, .. } = &sets[0] else { panic!("a Set, not {:?}", sets[0]) };
    assert_eq!((&**label, *doc_key, *after), ("Nudge", Some(DocKey::new("trial.nudge")), STEP));
}

/// The check replaces a value with `Fix`, and a refused commit ends the drag with `Cancel(Refused)` and restores the value.
#[trial]
fn a_check_fixes_values_and_a_refusing_check_blocks_the_commit() {
    let run = |check: fn(&f32) -> Validity<f32>| {
        let mut frames = Frames::new();
        let seen = RefCell::new(Vec::new());
        let flags = Seen::default();
        let mut value = INITIAL;
        let mut show = |ui: &mut Ui| {
            let out = Scrub::new(Bind::bound(&mut value).check(check)).seen(&flags).id_salt("scrub").show_at(ui, AT);
            seen.borrow_mut().extend(ev_of(&out.event));
        };
        drag(&mut frames, &mut show);
        (seen.take(), value, flags.flags.get())
    };
    let (events, value, _) = run(|v| if *v > CAP { Validity::Fix(CAP) } else { Validity::Ok });
    assert_eq!(value, CAP, "the fixed value reached the binding");
    assert!(matches!(events.last(), Some(Ev::Commit(v)) if *v == CAP), "{events:?}");

    let (events, value, flags) = run(|v| if *v > CAP { Validity::Refuse(Message::new("too far")) } else { Validity::Ok });
    assert!(events.contains(&Ev::Cancel(CancelReason::Refused)), "{events:?}");
    assert!(!events.iter().any(|event| matches!(event, Ev::Commit(_))), "no commit reached the app: {events:?}");
    assert_eq!(value, INITIAL, "the cancel restored the value from before the drag");
    assert!(!flags.contains(Flags::INVALID), "the restored value is valid");
}
