//! Edit lifecycle tests: the story's fake slider and text field, and a check box, in a headless
//! egui context under a [`GestureHost`].

use std::time::Duration;

use egui::accesskit::Role;
use egui::{Area, Context, Event, EventFilter, Id, Key, Modal, Modifiers, PointerButton, Pos2, RawInput, Rect, Sense, Ui, vec2};

use super::story::{slider, text_field};
use super::*;
use crate::primitives::Interactive;

/// The rig's document: the controls' values.
#[derive(Clone, Debug, PartialEq)]
struct Values {
    level: f32,
    label: String,
    flag: bool,
}

/// A document whose undo history holds whole snapshots.
#[derive(Debug)]
struct Doc<V> {
    now: V,
    undo: Vec<V>,
}

impl<V: Clone> Doc<V> {
    fn new(now: V) -> Self {
        Self { now, undo: Vec::new() }
    }

    /// Changes the document as one undo step.
    fn apply(&mut self, edit: impl FnOnce(&mut V)) {
        self.undo.push(self.now.clone());
        edit(&mut self.now);
    }

    fn set(&mut self, value: V) {
        self.apply(|now| *now = value);
    }
}

impl<V: Clone> UndoMarks for Doc<V> {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(before) = self.undo.get(mark).cloned() {
            self.now = before;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

/// Which control comes first in the frame: the text field, or the slider and the check box.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Order {
    FieldFirst,
    FieldLast,
}

/// The story's controls and a check box under a [`GestureHost`], frame by frame.
struct Rig {
    ctx: Context,
    policy: EditPolicy,
    scoped: bool,
    order: Order,
    draw_slider: bool,
    slider_enabled: bool,
    draw_field: bool,
    /// A dialog's gesture, kept live every frame.
    outer: Option<OuterGesture>,
    time: f64,
    dt: f64,
    doc: Doc<Values>,
    host: GestureHost<Doc<Values>>,
    slider: Rect,
    field: Rect,
    check: Rect,
    /// Whether a widget after the controls saw Esc.
    escape_seen: bool,
    slider_event: Option<EditEvent<f32>>,
    field_event: Option<EditEvent<String>>,
    check_event: Option<EditEvent<bool>>,
    /// What the host's `end_frame` applied.
    removed: Option<EditEvent<()>>,
}

impl Rig {
    fn new(policy: EditPolicy) -> Self {
        Self::with(policy, false, Order::FieldLast)
    }

    /// The controls inside an [`edit::scope`](scope).
    fn scoped(policy: EditPolicy) -> Self {
        Self::with(policy, true, Order::FieldLast)
    }

    fn ordered(order: Order) -> Self {
        Self::with(EditPolicy::default(), false, order)
    }

    fn with(policy: EditPolicy, scoped: bool, order: Order) -> Self {
        let mut rig = Self {
            ctx: Context::default(),
            policy,
            scoped,
            order,
            draw_slider: true,
            slider_enabled: true,
            draw_field: true,
            outer: None,
            time: 0.0,
            dt: 1.0 / 60.0,
            doc: Doc::new(Values { level: 40.0, label: "Layer 1".to_owned(), flag: false }),
            host: GestureHost::new(),
            slider: Rect::NOTHING,
            field: Rect::NOTHING,
            check: Rect::NOTHING,
            escape_seen: false,
            slider_event: None,
            field_event: None,
            check_event: None,
            removed: None,
        };
        rig.frame(vec![]);
        rig
    }

    fn level(&self) -> f32 {
        self.doc.now.level
    }

    fn label(&self) -> &str {
        &self.doc.now.label
    }

    fn steps(&self) -> usize {
        self.doc.undo.len()
    }

    /// Runs one frame and returns the slider's event; the others' go to their fields.
    fn frame(&mut self, events: Vec<Event>) -> Option<EditEvent<f32>> {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += self.dt;
        (self.slider_event, self.field_event, self.check_event) = (None, None, None);
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            if self.scoped {
                scope(ui, |ui| self.controls(ui));
            } else {
                self.controls(ui);
            }
            self.escape_seen = ui.input(|i| i.key_pressed(Key::Escape));
            self.removed = self.host.end_frame(ui.ctx(), &mut self.doc);
        });
        output.drop_without_applying_deltas();
        self.slider_event.clone()
    }

    /// The controls in the rig's order; the host applies each event as it comes.
    fn controls(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if let Some(outer) = self.outer {
            outer.keep(ui);
        }
        if self.order == Order::FieldFirst {
            self.text_field(ui);
        }
        if self.draw_slider {
            let (level, policy) = (self.doc.now.level, self.policy);
            let edit = ui.add_enabled_ui(self.slider_enabled, |ui| slider(ui, level, policy)).inner;
            self.slider = edit.response.rect;
            self.slider_event.clone_from(&edit.event);
            if let Some(event) = edit.event {
                self.host.handle(&ctx, &mut self.doc, event, |doc, value| doc.apply(|now| now.level = value));
            }
        }
        let edit = check_box(ui, self.doc.now.flag);
        self.check = edit.response.rect;
        self.check_event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.host.handle(&ctx, &mut self.doc, event, |doc, value| doc.apply(|now| now.flag = value));
        }
        if self.order == Order::FieldLast {
            self.text_field(ui);
        }
    }

    fn text_field(&mut self, ui: &mut Ui) {
        if !self.draw_field {
            return;
        }
        let edit = text_field(ui, &self.doc.now.label, self.policy);
        self.field = edit.response.rect;
        self.field_event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.host.handle(ui.ctx(), &mut self.doc, event, |doc, value| doc.apply(|now| now.label = value));
        }
    }

    /// The slider's centre, moved `dx` points.
    fn at(&self, dx: f32) -> Pos2 {
        self.slider.center() + vec2(dx, 0.0)
    }

    fn press(&mut self, dx: f32) -> Option<EditEvent<f32>> {
        let pos = self.at(dx);
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)])
    }

    fn drag(&mut self, dx: f32) -> Option<EditEvent<f32>> {
        let pos = self.at(dx);
        self.frame(vec![Event::PointerMoved(pos)])
    }

    fn release(&mut self, dx: f32) -> Option<EditEvent<f32>> {
        let pos = self.at(dx);
        self.frame(vec![button(pos, false)])
    }

    fn key(&mut self, key: Key) -> Option<EditEvent<f32>> {
        self.frame(vec![key_event(key)])
    }

    /// Clicks the text field to focus it.
    fn focus_field(&mut self) {
        let pos = self.field.center();
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
        self.frame(vec![]);
    }

    fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
    }

    /// Focuses the field and starts a typing run that the host has seen up to "Layer 1x".
    fn typing_run(&mut self) -> GestureId {
        self.focus_field();
        self.type_text("x");
        let Some(EditEvent::Begin { id }) = self.field_event else { panic!("expected Begin, got {:?}", self.field_event) };
        self.frame(vec![]);
        assert_eq!(self.field_event, update(id, "Layer 1x".to_owned()));
        id
    }
}

/// A check box: a click sets the opposite value.
fn check_box(ui: &mut Ui, value: bool) -> EditResponse<bool> {
    let response = Interactive::new("rig-check-box", Role::CheckBox, "Flag").show(ui, vec2(20.0, 20.0));
    let mut gesture = Gesture::load(ui, response.id, &value, EditPolicy::default());
    if response.clicked() {
        gesture.set(!value);
    }
    let event = gesture.finish();
    EditResponse::new(response, event)
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn key_event(key: Key) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }
}

fn begin_id(event: Option<EditEvent<f32>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn update<T>(id: GestureId, value: T) -> Option<EditEvent<T>> {
    Some(EditEvent::Update { id, value, modifiers: Modifiers::NONE })
}

fn commit<T>(id: GestureId, value: T, changed: bool) -> Option<EditEvent<T>> {
    Some(EditEvent::Commit { id, value, changed, modifiers: Modifiers::NONE })
}

fn cancel<T>(id: GestureId, reason: CancelReason) -> Option<EditEvent<T>> {
    Some(EditEvent::Cancel { id, reason })
}

#[test]
fn a_drag_begins_updates_and_commits_and_a_still_release_commits_unchanged() {
    let mut rig = Rig::new(EditPolicy::default());
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(id, 50.0));
    assert_eq!(rig.drag(20.0), update(id, 60.0));
    assert_eq!(rig.release(20.0), commit(id, 60.0, true));
    assert_eq!((rig.level(), rig.steps()), (60.0, 1));

    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.release(0.0), commit(id, 60.0, false));
    assert_eq!((rig.level(), rig.steps()), (60.0, 1));
}

#[test]
fn esc_mid_drag_cancels_without_a_commit_and_is_consumed() {
    let mut rig = Rig::new(EditPolicy::default());
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(id, 50.0));
    assert_eq!(rig.key(Key::Escape), cancel(id, CancelReason::Escape));
    assert!(!rig.escape_seen, "Esc reached a widget after the slider");
    assert_eq!(rig.level(), 40.0);
    assert_eq!(rig.drag(30.0), None);
    assert_eq!(rig.release(30.0), None);
    assert_eq!((rig.level(), rig.steps()), (40.0, 0));
}

#[test]
fn esc_ends_a_drag_that_is_not_cancellable_as_a_commit() {
    let mut rig = Rig::new(EditPolicy::default().cancellable(false));
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(id, 50.0));
    assert_eq!(rig.key(Key::Escape), commit(id, 50.0, true));
    assert!(rig.escape_seen);
}

/// A drag value that doesn't take the focus, as in the [`Gesture`] example.
fn drag_value(ui: &mut Ui, value: f32) -> EditResponse<f32> {
    let response = ui.allocate_response(vec2(100.0, 20.0), Sense::drag());
    let mut gesture = Gesture::load(ui, response.id, &value, EditPolicy::default());
    if response.drag_started() {
        gesture.begin();
    }
    if response.dragged()
        && let Some(start) = gesture.start().copied()
        && let Some((pos, origin)) = ui.input(|i| i.pointer.interact_pos().zip(i.pointer.press_origin()))
    {
        gesture.update(start + (pos.x - origin.x), Modifiers::NONE);
    }
    if response.drag_stopped() {
        gesture.commit();
    }
    let event = gesture.finish();
    EditResponse::new(response, event)
}

/// egui's `Modal` takes the focus from the widgets below it, so a typing run ends when one opens;
/// a drag goes on, and Esc is the modal's. egui still ends the drag on Esc: a commit.
#[test]
fn esc_during_a_drag_under_a_modal_goes_to_the_modal() {
    let ctx = Context::default();
    let (mut value, mut time, mut rect) = (40.0, 0.0, Rect::NOTHING);
    let mut frame = |events: Vec<Event>, modal: bool| {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(time), events, ..Default::default() };
        time += 1.0 / 60.0;
        let mut out = (None, false);
        let output = ctx.run_ui(input, |ui| {
            let edit = drag_value(ui, value);
            rect = edit.response.rect;
            if let Some(v) = edit.event.as_ref().and_then(EditEvent::value) {
                value = *v;
            }
            out.0.clone_from(&edit.event);
            out.1 = modal && Modal::new(Id::unique("modal")).show(ui.ctx(), |ui| ui.label("A dialog")).should_close();
        });
        output.drop_without_applying_deltas();
        (out, rect.center())
    };

    let (_, centre) = frame(vec![], false);
    let ((event, _), _) = frame(vec![Event::PointerMoved(centre), button(centre, true)], false);
    let id = begin_id(event);
    let moved = centre + vec2(10.0, 0.0);
    assert_eq!(frame(vec![Event::PointerMoved(moved)], false).0, (update(id, 50.0), false));
    // The modal is above the drag from its second frame.
    assert_eq!(frame(vec![], true).0, (None, false));
    assert_eq!(frame(vec![], true).0, (None, false));
    assert_eq!(frame(vec![key_event(Key::Escape)], true).0, (commit(id, 50.0, true), true));
}

#[test]
fn disabling_a_control_mid_drag_cancels_its_gesture_and_a_disabled_control_begins_none() {
    let mut rig = Rig::new(EditPolicy::default());
    let first = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(first, 50.0));
    rig.slider_enabled = false;
    assert_eq!(rig.drag(20.0), cancel(first, CancelReason::Disabled));
    assert_eq!((rig.level(), rig.steps()), (40.0, 0));
    assert_eq!(rig.release(20.0), None);
    assert_eq!(rig.press(0.0), None);
    assert_eq!(rig.drag(10.0), None);
    assert_eq!(rig.release(10.0), None);

    // egui hit-tests against the previous pass's widgets: one frame enabled before the press.
    rig.slider_enabled = true;
    rig.frame(vec![]);
    let second = begin_id(rig.press(0.0));
    assert_ne!(first, second);
    assert_eq!(rig.drag(5.0), update(second, 45.0));
}

/// A control whose gesture lives on its own widget while two parts take the focus, as a range
/// slider's thumbs: the Right key on a focused part adds one as a typing run.
fn two_part(ui: &mut Ui, value: f32) -> (Option<EditEvent<f32>>, [Id; 2]) {
    let response = ui.allocate_response(vec2(100.0, 20.0), Sense::hover());
    let parts = [(); 2].map(|()| ui.allocate_response(vec2(20.0, 20.0), Sense::FOCUSABLE).id);
    let mut gesture = Gesture::load(ui, response.id, &value, EditPolicy::default())
        .focus_owners(&parts)
        .event_filter(EventFilter { horizontal_arrows: true, ..Default::default() });
    let focused = ui.memory(|m| parts.iter().any(|&part| m.has_focus(part)));
    if focused && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight)) {
        gesture.begin();
        let next = gesture.shown() + 1.0;
        gesture.update(next, Modifiers::NONE);
    }
    (gesture.finish(), parts)
}

#[test]
fn focus_owners_carry_the_focus_rules_of_a_gesture_on_another_widget() {
    let ctx = Context::default();
    let (mut value, mut parts) = (40.0, [Id::NULL; 2]);
    let mut frame = |events: Vec<Event>| {
        let mut event = None;
        let output = ctx.run_ui(RawInput { events, ..Default::default() }, |ui| {
            (event, parts) = two_part(ui, value);
            if let Some(v) = event.as_ref().and_then(EditEvent::value) {
                value = *v;
            }
        });
        output.drop_without_applying_deltas();
        (event, parts)
    };
    let (_, [first, second]) = frame(vec![]);
    let has_focus = |id| ctx.memory(|m| m.has_focus(id));

    ctx.memory_mut(|m| m.request_focus(first));
    let id = begin_id(frame(vec![key_event(Key::ArrowRight)]).0);
    assert_eq!(frame(vec![]).0, update(id, 41.0));
    assert_eq!(ending(&ctx, id), Ending::Commit { changed: true }, "a typing run on an owner");
    // Moving the focus between owners keeps the gesture; Esc cancels and the owner keeps focus.
    ctx.memory_mut(|m| m.request_focus(second));
    assert_eq!(frame(vec![]).0, None);
    assert_eq!(frame(vec![key_event(Key::Escape)]).0, cancel(id, CancelReason::Escape));
    assert!(has_focus(second));

    // Focus leaving the owners commits the run (from 41: this host keeps a cancelled value).
    let id = begin_id(frame(vec![key_event(Key::ArrowRight)]).0);
    assert_eq!(frame(vec![]).0, update(id, 42.0));
    ctx.memory_mut(|m| m.surrender_focus(second));
    assert_eq!(frame(vec![]).0, commit(id, 42.0, true));
}

#[test]
fn cancel_all_mid_drag_cancels_with_its_reason_and_the_next_drag_is_a_new_gesture() {
    let mut rig = Rig::new(EditPolicy::default());
    let first = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(first, 50.0));
    cancel_all(&rig.ctx, CancelReason::DocumentSwitched);
    assert_eq!(rig.drag(20.0), cancel(first, CancelReason::DocumentSwitched));
    assert_eq!(rig.release(20.0), None);
    assert_eq!(rig.level(), 40.0);

    let second = begin_id(rig.press(0.0));
    assert_ne!(first, second);
    assert_eq!(rig.drag(5.0), update(second, 45.0));
}

#[test]
fn debounced_updates_coalesce_and_the_commit_carries_a_value_still_waiting() {
    let mut rig = Rig::new(EditPolicy::default().update(UpdateRate::Debounced(Duration::from_millis(250))));
    rig.dt = 0.1;
    let id = begin_id(rig.press(0.0));
    let mut events = Vec::new();
    for dx in [5.0, 10.0, 15.0] {
        events.extend(rig.drag(dx));
    }
    // Held still: 0.1 s, 0.2 s, then 0.3 s after the last change.
    for _ in 0..3 {
        events.extend(rig.drag(15.0));
    }
    assert_eq!(events, vec![EditEvent::Update { id, value: 55.0, modifiers: Modifiers::NONE }]);

    // The commit goes out in the release frame, with the value no update carried.
    assert_eq!(rig.drag(25.0), None);
    assert_eq!(rig.release(25.0), commit(id, 65.0, true));
    assert_eq!((rig.level(), rig.steps()), (65.0, 1));
}

#[test]
fn a_commit_goes_out_in_its_frame_even_when_the_control_goes_with_it() {
    // A field in a popup that closes on Enter: the field is not drawn in the next frame.
    let mut rig = Rig::new(EditPolicy::default().update(UpdateRate::Debounced(Duration::from_secs(1))));
    rig.focus_field();
    rig.type_text("x");
    let Some(EditEvent::Begin { id }) = rig.field_event else { panic!("expected Begin, got {:?}", rig.field_event) };
    rig.frame(vec![key_event(Key::Enter)]);
    assert_eq!(rig.field_event, commit(id, "Layer 1x".to_owned(), true));
    rig.draw_field = false;
    rig.frame(vec![]);
    assert_eq!(rig.removed, None);
    assert_eq!((rig.label(), rig.steps()), ("Layer 1x", 1));
}

#[test]
fn on_release_only_reports_no_updates() {
    let mut rig = Rig::new(EditPolicy::default().on_release_only(true));
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), None);
    assert_eq!(rig.drag(20.0), None);
    assert_eq!(rig.release(20.0), commit(id, 60.0, true));
    assert_eq!((rig.level(), rig.steps()), (60.0, 1));
}

#[test]
fn a_typing_run_that_begins_and_commits_in_one_frame_is_a_set() {
    let mut rig = Rig::new(EditPolicy::default());
    rig.focus_field();
    rig.frame(vec![Event::Text("x".to_owned()), key_event(Key::Enter)]);
    assert_eq!(rig.field_event, Some(EditEvent::Set { value: "Layer 1x".to_owned(), modifiers: Modifiers::NONE }));

    // A run over several frames is a gesture.
    rig.type_text("y");
    let Some(EditEvent::Begin { id }) = rig.field_event else { panic!("expected Begin, got {:?}", rig.field_event) };
    rig.frame(vec![]);
    assert_eq!(rig.field_event, update(id, "Layer 1xy".to_owned()));
    rig.frame(vec![key_event(Key::Enter)]);
    assert_eq!(rig.field_event, commit(id, "Layer 1xy".to_owned(), true));
    assert_eq!((rig.label(), rig.steps()), ("Layer 1xy", 2));
}

#[test]
fn inside_a_scope_only_updates_reach_the_host() {
    let only_updates = |events: &[EditEvent<f32>]| events.iter().all(|e| matches!(e, EditEvent::Update { .. }));

    let mut rig = Rig::scoped(EditPolicy::default());
    let mut events: Vec<_> = [rig.press(0.0), rig.drag(10.0), rig.drag(20.0), rig.release(20.0)].into_iter().flatten().collect();
    events.extend(rig.key(Key::Escape));
    assert!(only_updates(&events), "{events:?}");
    assert_eq!(events.iter().filter_map(EditEvent::value).collect::<Vec<_>>(), vec![&50.0, &60.0]);

    // With updates off, the commit still reaches the host, as an update.
    let mut rig = Rig::scoped(EditPolicy::default().on_release_only(true));
    let events: Vec<_> = [rig.press(0.0), rig.drag(10.0), rig.drag(20.0), rig.release(20.0)].into_iter().flatten().collect();
    assert!(only_updates(&events), "{events:?}");
    assert_eq!(events.iter().filter_map(EditEvent::value).collect::<Vec<_>>(), vec![&60.0]);
}

#[test]
fn a_scope_reaches_popups_opened_inside_it() {
    let ctx = Context::default();
    let (mut inside, mut after) = (false, true);
    let output = ctx.run_ui(RawInput::default(), |ui| {
        scope(ui, |ui| {
            Area::new(Id::unique("popup")).show(ui.ctx(), |ui| inside = in_scope(ui));
        });
        Area::new(Id::unique("after")).show(ui.ctx(), |ui| after = in_scope(ui));
    });
    output.drop_without_applying_deltas();
    assert!(inside, "a popup opened inside the scope is outside it");
    assert!(!after, "the scope outlived its closure");
}

#[test]
fn a_dialog_keeps_one_step_on_ok_and_reverts_on_cancel() {
    let mut rig = Rig::scoped(EditPolicy::default());
    let dialog = |rig: &mut Rig| {
        let outer = OuterGesture::begin(&rig.ctx);
        rig.host.begin_outer(&rig.ctx, &mut rig.doc, outer);
        rig.outer = Some(outer);
        for event in [rig.press(0.0), rig.drag(10.0), rig.drag(20.0), rig.release(20.0)] {
            assert!(matches!(event, None | Some(EditEvent::Update { .. })), "{event:?}");
            assert_eq!(rig.removed, None, "a kept dialog was removed");
        }
        outer.id()
    };

    let ok = dialog(&mut rig);
    assert_eq!((rig.level(), rig.steps()), (60.0, 2));
    assert!(rig.host.commit(&mut rig.doc, ok));
    assert_eq!((rig.level(), rig.steps()), (60.0, 1));

    let cancelled = dialog(&mut rig);
    assert_eq!(rig.level(), 80.0);
    assert!(rig.host.cancel(&mut rig.doc, cancelled));
    assert_eq!((rig.level(), rig.steps()), (60.0, 1));

    // A dialog that goes without OK or Cancel is reverted.
    let gone = dialog(&mut rig);
    rig.outer = None;
    rig.frame(vec![]);
    assert_eq!(rig.removed, cancel(gone, CancelReason::Removed));
    assert_eq!((rig.level(), rig.steps()), (60.0, 1));
}

/// A typing run, then another control's change, with the field drawn before or after it: the run
/// is one undo step and the other change another, however the frame orders them.
#[test]
fn a_change_during_a_typing_run_keeps_the_run_as_one_step_in_either_draw_order() {
    for order in [Order::FieldFirst, Order::FieldLast] {
        // A slider drag.
        let mut rig = Rig::ordered(order);
        rig.typing_run();
        let slider = begin_id(rig.press(0.0));
        assert_eq!(rig.drag(10.0), update(slider, 50.0));
        assert_eq!(rig.release(10.0), commit(slider, 50.0, true));
        rig.frame(vec![]);
        assert_eq!((rig.label(), rig.level(), rig.steps()), ("Layer 1x", 50.0, 2), "{order:?}");
        rig.doc.revert_to(1);
        assert_eq!((rig.label(), rig.level()), ("Layer 1x", 40.0), "{order:?}");

        // A check box click.
        let mut rig = Rig::ordered(order);
        rig.typing_run();
        let pos = rig.check.center();
        rig.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
        rig.frame(vec![button(pos, false)]);
        assert_eq!(rig.check_event, Some(EditEvent::Set { value: true, modifiers: Modifiers::NONE }), "{order:?}");
        rig.frame(vec![]);
        assert_eq!((rig.label(), rig.doc.now.flag, rig.steps()), ("Layer 1x", true, 2), "{order:?}");
        rig.doc.revert_to(1);
        assert_eq!((rig.label(), rig.doc.now.flag), ("Layer 1x", false), "{order:?}");
    }
}

#[test]
fn a_drag_whose_control_is_not_drawn_is_removed_and_reverted() {
    let mut rig = Rig::new(EditPolicy::default());
    let first = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(first, 50.0));
    assert_eq!((rig.level(), rig.steps()), (50.0, 1));

    rig.draw_slider = false;
    rig.frame(vec![]);
    assert_eq!(rig.removed, cancel(first, CancelReason::Removed));
    assert_eq!((rig.level(), rig.steps()), (40.0, 0));
    assert!(!rig.host.is_active());

    // Drawn again, the slider is idle: the held pointer and its release do nothing.
    rig.draw_slider = true;
    assert_eq!((rig.drag(0.0), rig.removed.clone()), (None, None));
    assert_eq!(rig.release(0.0), None);
    let second = begin_id(rig.press(0.0));
    assert_ne!(first, second);
    assert_eq!((rig.level(), rig.steps()), (40.0, 0));
}

/// Closing a panel ends editing and keeps the text: the host gets one step with everything typed,
/// also when no update reported the last of it.
#[test]
fn a_typing_run_whose_field_is_not_drawn_is_kept_as_one_step_with_everything_typed() {
    let policies = [
        EditPolicy::default(),
        EditPolicy::default().on_release_only(true),
        EditPolicy::default().update(UpdateRate::Debounced(Duration::from_secs(1))),
    ];
    for policy in policies {
        let mut rig = Rig::new(policy);
        rig.focus_field();
        rig.type_text("x");
        let Some(EditEvent::Begin { id }) = rig.field_event else { panic!("expected Begin, got {:?}", rig.field_event) };
        rig.type_text("y");

        rig.draw_field = false;
        rig.frame(vec![]);
        assert_eq!(rig.removed, commit(id, (), true), "{policy:?}");
        assert_eq!((rig.label(), rig.steps()), ("Layer 1xy", 1), "{policy:?}");
        assert!(!rig.host.is_active());
        rig.doc.revert_to(0);
        assert_eq!(rig.label(), "Layer 1", "{policy:?}");
    }
}

/// A control driven by direct calls, for the display rules.
struct Probe {
    ctx: Context,
    time: f64,
    policy: EditPolicy,
}

impl Probe {
    fn new(policy: EditPolicy) -> Self {
        Self { ctx: Context::default(), time: 0.0, policy }
    }

    /// One frame with the host's `value`; returns the event and what the control shows.
    fn frame(&mut self, value: f32, drive: impl FnOnce(&mut Gesture<f32>)) -> (Option<EditEvent<f32>>, f32) {
        let input = RawInput { time: Some(self.time), ..Default::default() };
        self.time += 0.1;
        let (mut out, mut drive) = ((None, f32::NAN), Some(drive));
        let output = self.ctx.run_ui(input, |ui| {
            let response = ui.allocate_response(vec2(100.0, 20.0), Sense::hover());
            let mut gesture = Gesture::load(ui, response.id, &value, self.policy);
            if let Some(drive) = drive.take() {
                drive(&mut gesture);
            }
            out.1 = *gesture.shown();
            out.0 = gesture.finish();
        });
        output.drop_without_applying_deltas();
        out
    }

    /// Begins at `value` and has the host show `to`.
    fn preview(&mut self, value: f32, to: f32) -> GestureId {
        let (event, _) = self.frame(value, Gesture::begin);
        let Some(EditEvent::Begin { id }) = event else { panic!("expected Begin, got {event:?}") };
        assert_eq!(self.frame(value, |g| g.update(to, Modifiers::NONE)), (update(id, to), to));
        id
    }
}

#[test]
fn after_a_cancel_the_control_shows_the_start_value_only_while_the_host_shows_the_preview() {
    let mut probe = Probe::new(EditPolicy::default());

    // Cancelled while the host shows 50: the control shows 40 until the host has reverted.
    let id = probe.preview(40.0, 50.0);
    assert_eq!(probe.frame(50.0, |g| g.cancel(CancelReason::Escape)), (cancel(id, CancelReason::Escape), 40.0));
    assert_eq!(probe.frame(50.0, |_| {}), (None, 40.0));
    assert_eq!(probe.frame(40.0, |_| {}), (None, 40.0));

    // Cancelled by a document switch: the control shows the other document's 75 at once.
    let id = probe.preview(40.0, 50.0);
    cancel_all(&probe.ctx, CancelReason::DocumentSwitched);
    assert_eq!(probe.frame(75.0, |_| {}), (cancel(id, CancelReason::DocumentSwitched), 75.0));
}

#[test]
fn a_host_value_changed_during_a_gesture_cancels_it() {
    let mut probe = Probe::new(EditPolicy::default());
    let id = probe.preview(40.0, 50.0);
    assert_eq!(probe.frame(70.0, |_| {}), (cancel(id, CancelReason::External), 70.0));

    // With updates off, the host's value is the start value until the commit.
    probe.policy = probe.policy.on_release_only(true);
    let (event, _) = probe.frame(70.0, Gesture::begin);
    let Some(EditEvent::Begin { id }) = event else { panic!("expected Begin, got {event:?}") };
    assert_eq!(probe.frame(70.0, |g| g.update(80.0, Modifiers::NONE)), (None, 80.0));
    assert_eq!(probe.frame(45.0, |_| {}), (cancel(id, CancelReason::External), 45.0));
}

#[test]
fn gesture_host_keeps_one_step_per_commit_restores_on_cancel_and_none_when_unchanged() {
    let ctx = Context::default();
    let mut doc = Doc::new(0);
    let mut host = GestureHost::new();
    let mut feed = |doc: &mut Doc<i32>, events: Vec<EditEvent<i32>>| {
        for event in events {
            host.handle(&ctx, doc, event, Doc::set);
        }
    };
    let update = |id, value| EditEvent::Update { id, value, modifiers: Modifiers::NONE };
    let commit = |id, value, changed| EditEvent::Commit { id, value, changed, modifiers: Modifiers::NONE };

    let id = GestureId::next();
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 1), update(id, 2), update(id, 3), commit(id, 3, true)]);
    assert_eq!((doc.now, doc.undo.len()), (3, 1));

    let id = GestureId::next();
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 7), update(id, 8), EditEvent::Cancel { id, reason: CancelReason::Escape }]);
    assert_eq!((doc.now, doc.undo.len()), (3, 1));

    let id = GestureId::next();
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 5), update(id, 3), commit(id, 3, false)]);
    assert_eq!((doc.now, doc.undo.len()), (3, 1));

    // A commit with a value no update carried applies it, still as one step.
    let id = GestureId::next();
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 4), commit(id, 6, true)]);
    assert_eq!((doc.now, doc.undo.len()), (6, 2));

    feed(&mut doc, vec![EditEvent::Set { value: 9, modifiers: Modifiers::NONE }]);
    assert_eq!((doc.now, doc.undo.len()), (9, 3));

    // An external change keeps the document as it is.
    let id = GestureId::next();
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 10), EditEvent::Cancel { id, reason: CancelReason::External }]);
    assert_eq!((doc.now, doc.undo.len()), (10, 4));
}

#[test]
fn gesture_host_ends_the_gesture_under_way_when_a_set_arrives_and_ignores_it_after() {
    let ctx = Context::default();
    let mut doc = Doc::new(0);
    let mut host = GestureHost::new();
    let update = |id, value| EditEvent::Update { id, value, modifiers: Modifiers::NONE };
    let set = |value| EditEvent::Set { value, modifiers: Modifiers::NONE };

    // A typing run (recorded by its control as one to keep, ending with a value no update
    // carried), then a set from another control.
    let run = GestureId::next();
    live::mark(&ctx, run, Ending::Commit { changed: true }, Some(std::sync::Arc::new(3)));
    for event in [EditEvent::Begin { id: run }, update(run, 1), update(run, 2), set(9)] {
        host.handle(&ctx, &mut doc, event, Doc::set);
    }
    assert_eq!((doc.now, doc.undo.len()), (9, 2));
    assert!(!host.is_active());
    host.handle(&ctx, &mut doc, EditEvent::Commit { id: run, value: 4, changed: true, modifiers: Modifiers::NONE }, Doc::set);
    assert_eq!((doc.now, doc.undo.len()), (9, 2));
    doc.revert_to(1);
    assert_eq!(doc.now, 3);

    // A drag (recorded as one to revert), then a set.
    let drag = GestureId::next();
    live::mark(&ctx, drag, Ending::Cancel, None);
    for event in [EditEvent::Begin { id: drag }, update(drag, 5), set(7)] {
        host.handle(&ctx, &mut doc, event, Doc::set);
    }
    assert_eq!((doc.now, doc.undo.len()), (7, 2));
}

/// A host that ended a dialog's gesture in one context keeps taking gestures after the
/// application moves to another context or resets its memory.
#[test]
fn gesture_ids_grow_across_contexts_and_memory_resets() {
    let (a, b) = (Context::default(), Context::default());
    let mut doc = Doc::new(0);
    let mut host = GestureHost::new();
    let outer = OuterGesture::begin(&a);
    host.begin_outer(&a, &mut doc, outer);
    host.handle(&a, &mut doc, EditEvent::Update { id: outer.id(), value: 1, modifiers: Modifiers::NONE }, Doc::set);
    assert!(host.commit(&mut doc, outer.id()));

    let in_b = OuterGesture::begin(&b).id();
    a.memory_mut(|m| *m = egui::Memory::default());
    let after_reset = OuterGesture::begin(&a).id();
    assert!(outer.id() < in_b && in_b < after_reset, "{} {in_b} {after_reset}", outer.id());

    for (ctx, id, value) in [(&b, in_b, 2), (&a, after_reset, 3)] {
        for event in [
            EditEvent::Begin { id },
            EditEvent::Update { id, value, modifiers: Modifiers::NONE },
            EditEvent::Commit { id, value, changed: true, modifiers: Modifiers::NONE },
        ] {
            host.handle(ctx, &mut doc, event, Doc::set);
        }
        assert_eq!(doc.now, value);
    }
}
