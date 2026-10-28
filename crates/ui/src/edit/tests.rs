//! Edit lifecycle tests: the story's fake slider and text field in a headless egui context.

use std::time::Duration;

use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Ui, vec2};

use super::story::{slider, text_field};
use super::*;

/// The story's controls under a host that echoes every value, frame by frame.
struct Rig {
    ctx: Context,
    policy: EditPolicy,
    scoped: bool,
    time: f64,
    dt: f64,
    level: f32,
    /// The host's level when the slider's gesture began.
    before: f32,
    label: String,
    slider: Rect,
    field: Rect,
    /// Whether a widget after the controls saw Esc.
    escape_seen: bool,
    field_event: Option<EditEvent<String>>,
}

impl Rig {
    fn new(policy: EditPolicy) -> Self {
        Self::with(policy, false)
    }

    /// The controls inside an [`edit::scope`](scope).
    fn scoped(policy: EditPolicy) -> Self {
        Self::with(policy, true)
    }

    fn with(policy: EditPolicy, scoped: bool) -> Self {
        let mut rig = Self {
            ctx: Context::default(),
            policy,
            scoped,
            time: 0.0,
            dt: 1.0 / 60.0,
            level: 40.0,
            before: 40.0,
            label: "Layer 1".to_owned(),
            slider: Rect::NOTHING,
            field: Rect::NOTHING,
            escape_seen: false,
            field_event: None,
        };
        rig.frame(vec![]);
        rig
    }

    /// Runs one frame and returns the slider's event; the field's goes to `field_event`.
    fn frame(&mut self, events: Vec<Event>) -> Option<EditEvent<f32>> {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += self.dt;
        let (level, label, policy, scoped) = (self.level, self.label.clone(), self.policy, self.scoped);
        let mut out = None;
        let output = self.ctx.run_ui(input, |ui| {
            let controls = |ui: &mut Ui| (slider(ui, level, policy), text_field(ui, &label, policy));
            let (s, f) = if scoped { scope(ui, controls).inner } else { controls(ui) };
            let escape_seen = ui.input(|i| i.key_pressed(Key::Escape));
            out = Some((s, f, escape_seen));
        });
        output.drop_without_applying_deltas();
        let (s, f, escape_seen) = out.expect("the frame ran");
        (self.slider, self.field, self.escape_seen) = (s.response.rect, f.response.rect, escape_seen);
        self.echo_level(s.event.as_ref());
        if let Some(value) = f.event.as_ref().and_then(EditEvent::value) {
            self.label.clone_from(value);
        }
        self.field_event = f.event;
        s.event
    }

    fn echo_level(&mut self, event: Option<&EditEvent<f32>>) {
        match event {
            Some(EditEvent::Begin { .. }) => self.before = self.level,
            Some(EditEvent::Cancel { .. }) => self.level = self.before,
            Some(e) => self.level = e.value().copied().unwrap_or(self.level),
            None => {}
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

fn update(id: GestureId, value: f32) -> Option<EditEvent<f32>> {
    Some(EditEvent::Update { id, value, modifiers: Modifiers::NONE })
}

#[test]
fn a_drag_begins_updates_and_commits_and_a_still_release_commits_unchanged() {
    let mut rig = Rig::new(EditPolicy::default());
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(id, 50.0));
    assert_eq!(rig.drag(20.0), update(id, 60.0));
    assert_eq!(rig.release(20.0), Some(EditEvent::Commit { id, value: 60.0, changed: true }));

    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.release(0.0), Some(EditEvent::Commit { id, value: 60.0, changed: false }));
}

#[test]
fn esc_mid_drag_cancels_without_a_commit_and_is_consumed() {
    let mut rig = Rig::new(EditPolicy::default());
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(id, 50.0));
    assert_eq!(rig.key(Key::Escape), Some(EditEvent::Cancel { id, reason: CancelReason::Escape }));
    assert!(!rig.escape_seen, "Esc reached a widget after the slider");
    assert_eq!(rig.level, 40.0);
    assert_eq!(rig.drag(30.0), None);
    assert_eq!(rig.release(30.0), None);
    assert_eq!(rig.level, 40.0);
}

#[test]
fn esc_ends_a_drag_that_is_not_cancellable_as_a_commit() {
    let mut rig = Rig::new(EditPolicy { cancellable: false, ..EditPolicy::default() });
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(id, 50.0));
    assert_eq!(rig.key(Key::Escape), Some(EditEvent::Commit { id, value: 50.0, changed: true }));
    assert!(rig.escape_seen);
}

#[test]
fn cancel_all_mid_drag_cancels_with_its_reason_and_the_next_drag_is_a_new_gesture() {
    let mut rig = Rig::new(EditPolicy::default());
    let first = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), update(first, 50.0));
    cancel_all(&rig.ctx, CancelReason::DocumentSwitched);
    assert_eq!(rig.drag(20.0), Some(EditEvent::Cancel { id: first, reason: CancelReason::DocumentSwitched }));
    assert_eq!(rig.release(20.0), None);

    let second = begin_id(rig.press(0.0));
    assert_ne!(first, second);
    assert_eq!(rig.drag(5.0), update(second, 45.0));
}

#[test]
fn debounced_updates_coalesce_and_the_last_value_goes_out_before_the_commit() {
    let mut rig = Rig::new(EditPolicy { update: UpdateRate::Debounced(Duration::from_millis(250)), ..EditPolicy::default() });
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

    assert_eq!(rig.drag(25.0), None);
    assert_eq!(rig.release(25.0), update(id, 65.0));
    assert_eq!(rig.frame(vec![]), Some(EditEvent::Commit { id, value: 65.0, changed: true }));
}

#[test]
fn on_release_only_reports_no_updates() {
    let mut rig = Rig::new(EditPolicy { on_release_only: true, ..EditPolicy::default() });
    let id = begin_id(rig.press(0.0));
    assert_eq!(rig.drag(10.0), None);
    assert_eq!(rig.drag(20.0), None);
    assert_eq!(rig.release(20.0), Some(EditEvent::Commit { id, value: 60.0, changed: true }));
}

#[test]
fn a_typing_run_that_begins_and_commits_in_one_frame_is_a_set() {
    let mut rig = Rig::new(EditPolicy::default());
    let pos = rig.field.center();
    rig.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
    rig.frame(vec![]);
    rig.frame(vec![Event::Text("x".to_owned()), key_event(Key::Enter)]);
    assert_eq!(rig.field_event, Some(EditEvent::Set { value: "Layer 1x".to_owned() }));

    // A run over several frames is a gesture.
    rig.frame(vec![Event::Text("y".to_owned())]);
    let Some(EditEvent::Begin { id }) = rig.field_event else { panic!("expected Begin, got {:?}", rig.field_event) };
    rig.frame(vec![]);
    assert_eq!(rig.field_event, Some(EditEvent::Update { id, value: "Layer 1xy".to_owned(), modifiers: Modifiers::NONE }));
    rig.frame(vec![key_event(Key::Enter)]);
    assert_eq!(rig.field_event, Some(EditEvent::Commit { id, value: "Layer 1xy".to_owned(), changed: true }));
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
    let mut rig = Rig::scoped(EditPolicy { on_release_only: true, ..EditPolicy::default() });
    let events: Vec<_> = [rig.press(0.0), rig.drag(10.0), rig.drag(20.0), rig.release(20.0)].into_iter().flatten().collect();
    assert!(only_updates(&events), "{events:?}");
    assert_eq!(events.iter().filter_map(EditEvent::value).collect::<Vec<_>>(), vec![&60.0]);
}

/// A document whose undo history holds the previous values.
#[derive(Default)]
struct Doc<V> {
    value: V,
    undo: Vec<V>,
}

impl<V: Copy> Doc<V> {
    /// Sets the value as one undo step.
    fn apply(&mut self, value: V) {
        self.undo.push(self.value);
        self.value = value;
    }
}

impl<V: Copy> UndoMarks for Doc<V> {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&value) = self.undo.get(mark) {
            self.value = value;
            self.undo.truncate(mark);
        }
    }
}

#[test]
fn gesture_host_keeps_one_step_per_commit_restores_on_cancel_and_none_when_unchanged() {
    let mut doc = Doc::<i32>::default();
    let mut host = GestureHost::new();
    let mut feed = |doc: &mut Doc<i32>, events: Vec<EditEvent<i32>>| {
        for event in events {
            host.handle(doc, event, Doc::apply);
        }
    };
    let update = |id, value| EditEvent::Update { id, value, modifiers: Modifiers::NONE };

    let id = GestureId(1);
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 1), update(id, 2), update(id, 3), EditEvent::Commit { id, value: 3, changed: true }]);
    assert_eq!((doc.value, doc.undo.len()), (3, 1));

    let id = GestureId(2);
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 7), update(id, 8), EditEvent::Cancel { id, reason: CancelReason::Escape }]);
    assert_eq!((doc.value, doc.undo.len()), (3, 1));

    let id = GestureId(3);
    feed(&mut doc, vec![EditEvent::Begin { id }, update(id, 5), update(id, 3), EditEvent::Commit { id, value: 3, changed: false }]);
    assert_eq!((doc.value, doc.undo.len()), (3, 1));

    feed(&mut doc, vec![EditEvent::Set { value: 9 }]);
    assert_eq!((doc.value, doc.undo.len()), (9, 2));
}

#[test]
fn a_gesture_whose_control_is_not_drawn_is_removed_and_the_host_reverts_it() {
    let ctx = Context::default();
    let mut doc = Doc { value: 40.0, undo: Vec::new() };
    let mut host = GestureHost::new();
    let mut time = 0.0;
    // One pass, the slider drawn or not; the slider's event, the host's end_frame result, and the
    // slider's rect.
    let mut frame = |doc: &mut Doc<f32>, host: &mut GestureHost<Doc<f32>>, events: Vec<Event>, draw: bool| {
        time += 1.0 / 60.0;
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(time), events, ..Default::default() };
        let mut out = (None, None, Rect::NOTHING);
        let output = ctx.run_ui(input, |ui| {
            if draw {
                let edit = slider(ui, doc.value, EditPolicy::default());
                out.0.clone_from(&edit.event);
                out.2 = edit.response.rect;
                if let Some(event) = edit.event {
                    host.handle(doc, event, Doc::apply);
                }
            }
            out.1 = host.end_frame(ui.ctx(), doc);
        });
        output.drop_without_applying_deltas();
        out
    };

    let centre = frame(&mut doc, &mut host, vec![], true).2.center();
    let first = begin_id(frame(&mut doc, &mut host, vec![Event::PointerMoved(centre), button(centre, true)], true).0);
    let moved = centre + vec2(10.0, 0.0);
    assert_eq!(frame(&mut doc, &mut host, vec![Event::PointerMoved(moved)], true).0, update(first, 50.0));
    assert_eq!((doc.value, doc.undo.len()), (50.0, 1));

    let (_, removed, _) = frame(&mut doc, &mut host, vec![], false);
    assert_eq!(removed, Some(EditEvent::Cancel { id: first, reason: CancelReason::Removed }));
    assert_eq!((doc.value, doc.undo.len()), (40.0, 0));
    assert!(!host.is_active());

    // Drawn again, the slider is idle: the held pointer and its release do nothing.
    let (event, removed, _) = frame(&mut doc, &mut host, vec![Event::PointerMoved(centre)], true);
    assert_eq!((event, removed), (None, None));
    assert_eq!(frame(&mut doc, &mut host, vec![button(centre, false)], true).0, None);
    let second = begin_id(frame(&mut doc, &mut host, vec![button(centre, true)], true).0);
    assert_ne!(first, second);
    assert_eq!((doc.value, doc.undo.len()), (40.0, 0));
}
