//! Gestures: Escape restores, an app change or an undo cancels, a cancelled gesture stays cancelled under its press, a
//! reset is one `Set`, a read-only component refuses edits and a composite adopts its children's edits.

use super::kit::{APP_VALUE, AT, Ev, FIRST, HELD, INITIAL, PlainStyle, RESET_TO, Scrub, ev_of, near};
use craft_behavior::{CancelReason, EditEvent, Mixed, Sense};
use craft_ui::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Props as _, Role, Show as _};
use craft_ui::runtime::edit::cancel_all;
use craft_ui::style::StyleCx;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Ui, Vec2};
use std::cell::RefCell;

fn start() -> Pos2 {
    Pos2::new(AT.left() + 10.0, AT.center().y)
}

/// Pressed on the track, past the slop and moved once more: a drag in progress.
fn dragging(frames: &mut Frames, show: &mut impl FnMut(&mut Ui)) {
    frames.idle(3, &mut *show);
    frames.frame(vec![moved(start())], &mut *show);
    frames.frame(vec![press(start())], &mut *show);
    frames.frame(vec![moved(start() + Vec2::new(30.0, 0.0))], &mut *show);
    frames.frame(vec![moved(start() + Vec2::new(35.0, 0.0))], &mut *show);
}

/// Escape during a drag restores the value and cancels once; the press then proposes nothing until it is released, and
/// the next press begins a new gesture.
#[trial]
fn escape_restores_the_value_and_the_press_proposes_nothing_until_release() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value)).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().push(ev_of(&out.event));
    };
    dragging(&mut frames, &mut show);
    seen.borrow_mut().clear();
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    frames.frame(vec![moved(start() + Vec2::new(50.0, 0.0))], &mut show);
    frames.frame(vec![], &mut show);
    let events: Vec<Ev> = seen.borrow().iter().flatten().cloned().collect();
    assert_eq!(events, vec![Ev::Cancel(CancelReason::Escape)], "one cancel, then nothing under the held press");
    frames.frame(vec![release(start() + Vec2::new(50.0, 0.0))], &mut show);
    frames.frame(vec![], &mut show);
    seen.borrow_mut().clear();
    frames.frame(vec![moved(start())], &mut show);
    frames.frame(vec![press(start())], &mut show);
    frames.frame(vec![moved(start() + Vec2::new(30.0, 0.0))], &mut show);
    assert!(seen.borrow().iter().flatten().any(|event| *event == Ev::Begin), "a new press begins a new gesture: {:?}", seen.borrow());
    drop(show);
    assert!(near(value, FIRST), "the first gesture was undone and the second wrote its own value: {value}");
}

/// The app changes the value during a drag: the drag cancels with `HostChanged`, shows the app's value and does not
/// resume under the same press.
#[trial]
fn an_app_change_during_a_drag_cancels_it_and_keeps_the_app_value() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value)).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().push(ev_of(&out.event));
    };
    dragging(&mut frames, &mut show);
    seen.borrow_mut().clear();
    drop(show);
    value = APP_VALUE;
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value)).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().push(ev_of(&out.event));
    };
    frames.frame(vec![moved(start() + Vec2::new(50.0, 0.0))], &mut show);
    frames.frame(vec![moved(start() + Vec2::new(60.0, 0.0))], &mut show);
    let events: Vec<Ev> = seen.borrow().iter().flatten().cloned().collect();
    assert_eq!(events, vec![Ev::Cancel(CancelReason::HostChanged)], "{events:?}");
    drop(show);
    assert_eq!(value, APP_VALUE, "the cancel restored nothing over the app's value");
}

fn scrub(value: &mut f32, seen: &RefCell<Vec<Option<Ev>>>, ui: &mut Ui) {
    let out = Scrub::new(Bind::bound(value)).id_salt("scrub").show_at(ui, AT);
    seen.borrow_mut().push(ev_of(&out.event));
}

/// An undo during a drag ends it with `UndoDuringGesture` and the app's document stands.
#[trial]
fn an_undo_during_a_drag_cancels_it_until_release() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    dragging(&mut frames, &mut |ui| scrub(&mut value, &seen, ui));
    assert!(near(value, HELD), "the drag wrote through: {value}");
    cancel_all(&frames.ctx, CancelReason::UndoDuringGesture);
    value = APP_VALUE;
    seen.borrow_mut().clear();
    frames.frame(vec![moved(start() + Vec2::new(50.0, 0.0))], |ui| scrub(&mut value, &seen, ui));
    frames.frame(vec![moved(start() + Vec2::new(60.0, 0.0))], |ui| scrub(&mut value, &seen, ui));
    let events: Vec<Ev> = seen.borrow().iter().flatten().cloned().collect();
    assert_eq!(events, vec![Ev::Cancel(CancelReason::UndoDuringGesture)], "{events:?}");
    assert_eq!(value, APP_VALUE, "the drag did not resume over the undone document");
}

/// Delete on a focused control with a reset value is one `Set` of that value; a control with no reset value ignores it.
#[trial]
fn delete_resets_a_focused_control_with_one_set() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value).reset(RESET_TO)).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().extend(ev_of(&out.event));
    };
    frames.idle(3, &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.frame(vec![key_press(Key::Delete, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(seen.take(), vec![Ev::Set(RESET_TO)]);
    assert_eq!(value, RESET_TO);
}

/// A read-only component refuses every edit: nothing reaches the binding or the app.
#[trial]
fn a_read_only_component_refuses_edits() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value)).read_only(true).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().extend(ev_of(&out.event));
    };
    dragging(&mut frames, &mut show);
    frames.frame(vec![release(start() + Vec2::new(35.0, 0.0))], &mut show);
    frames.idle(2, &mut show);
    assert!(seen.borrow().is_empty(), "no event: {:?}", seen.borrow());
    assert_eq!(value, INITIAL);
}

/// A composite whose child is a value control: it adopts the child's edit as its own gesture.
#[derive(craft_ui::contract::Component)]
struct Wrapper<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f32>,
}

impl Component for Wrapper<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = EditResponse<f32>;

    fn style(_: &StyleCx<'_>, _: WrapperStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        AT.size()
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn bind(&mut self) -> Option<BindMut<'_, f32>> {
        Some(self.value.reborrow())
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let shown = cx.value().copied().unwrap_or(0.0);
        let child = ui.child("scrub", cx.rect, Scrub::new(Bind::given(shown)));
        cx.adopt(&child, |value, mine| *mine = *value);
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f32> {
        cx.edit_response()
    }
}

/// The child's gesture is the composite's gesture: one `Commit`, with the child's last value.
#[trial]
fn a_composite_adopts_its_childs_gesture_as_its_own() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let out = Wrapper { common: Common::new(), value: Bind::bound(&mut value) }.id_salt("wrapper").show_at(ui, AT);
        seen.borrow_mut().extend(ev_of(&out.event));
    };
    dragging(&mut frames, &mut show);
    frames.frame(vec![release(start() + Vec2::new(35.0, 0.0))], &mut show);
    frames.idle(2, &mut show);
    drop(show);
    let events = seen.take();
    assert_eq!(events.iter().filter(|e| matches!(e, Ev::Commit(_) | Ev::Set(_))).count(), 1, "one commit for one gesture: {events:?}");
    assert!(near(value, HELD), "the child's last value reached the composite's binding: {value}");
}

fn wrapper_events(value: &mut f32, seen: &RefCell<Vec<EditEvent<f32>>>, escaped: &RefCell<bool>, ui: &mut Ui) {
    let out = Wrapper { common: Common::new(), value: Bind::bound(value) }.id_salt("wrapper").show_at(ui, AT);
    seen.borrow_mut().extend(out.event.clone());
    *escaped.borrow_mut() |= out.escaped;
}

/// The adopted `Commit` carries the value the gesture began with, so a host sees a changed value and keeps the step.
#[trial]
fn an_adopted_commit_carries_the_value_the_gesture_began_with() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let (seen, escaped) = (RefCell::new(Vec::new()), RefCell::new(false));
    dragging(&mut frames, &mut |ui| wrapper_events(&mut value, &seen, &escaped, ui));
    frames.frame(vec![release(start() + Vec2::new(35.0, 0.0))], |ui| wrapper_events(&mut value, &seen, &escaped, ui));
    let events = seen.take();
    let commit = events.iter().find_map(|event| match event {
        EditEvent::Commit { before, after, .. } => Some((before.clone(), *after)),
        _ => None,
    });
    let (before, after) = commit.unwrap_or_else(|| panic!("a commit: {events:?}"));
    assert_eq!(before, Mixed::Same(INITIAL));
    assert!(near(after, HELD));
}

/// Escape on the child cancels the composite's gesture and restores the value it began with.
#[trial]
fn escape_on_an_adopted_child_restores_the_composites_value() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let (seen, escaped) = (RefCell::new(Vec::new()), RefCell::new(false));
    dragging(&mut frames, &mut |ui| wrapper_events(&mut value, &seen, &escaped, ui));
    seen.borrow_mut().clear();
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], |ui| wrapper_events(&mut value, &seen, &escaped, ui));
    frames.idle(2, |ui| wrapper_events(&mut value, &seen, &escaped, ui));
    let cancel = seen.borrow().iter().find_map(|event| match event {
        EditEvent::Cancel { reason, before, .. } => Some((*reason, before.clone())),
        _ => None,
    });
    assert_eq!(cancel, Some((CancelReason::Escape, Some(Mixed::Same(INITIAL)))), "{:?}", seen.borrow());
    assert!(*escaped.borrow(), "the response reports the Escape");
    assert_eq!(value, INITIAL);
}

/// Read-only turning on during a drag still ends the gesture for the app.
#[trial]
fn a_gesture_cancelled_while_read_only_still_reaches_the_app() {
    let mut frames = Frames::new();
    let mut value = INITIAL;
    let seen = RefCell::new(Vec::new());
    dragging(&mut frames, &mut |ui| scrub(&mut value, &seen, ui));
    seen.borrow_mut().clear();
    let mut show = |ui: &mut Ui| {
        let out = Scrub::new(Bind::bound(&mut value)).read_only(true).id_salt("scrub").show_at(ui, AT);
        seen.borrow_mut().push(ev_of(&out.event));
    };
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    drop(show);
    let events: Vec<Ev> = seen.borrow().iter().flatten().cloned().collect();
    assert_eq!(events, vec![Ev::Cancel(CancelReason::Escape)]);
}
