//! The undo adapter: one undo step per gesture, named by the component's label.

use super::kit::{AT, DRAGGED, INITIAL, Scrub, near};
use craft_behavior::edit_history::Step;
use craft_behavior::{CancelReason, DocKey, EditEvent, EditHistory, GestureId, Mixed};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::edit::{GestureHost, StepName, UndoMarks};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Ui, Vec2};

/// A document with the history of an app: every applied edit records a step, and the history can fold or take back steps.
#[derive(Default)]
struct Doc {
    value: f32,
    steps: Vec<(f32, Option<String>)>,
}

impl Doc {
    fn set(&mut self, value: f32) {
        self.steps.push((self.value, None));
        self.value = value;
    }
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.steps.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some((value, _)) = self.steps.get(mark) {
            self.value = *value;
        }
        self.steps.truncate(mark);
    }

    fn squash_since(&mut self, mark: usize, name: &StepName) {
        if self.steps.len() > mark {
            self.steps.truncate(mark + 1);
            self.steps[mark].1 = Some(name.label.to_string());
        }
    }
}

/// Shows a given-bound `Scrub` over the document and feeds its events to the host, frame by frame.
struct Session {
    frames: Frames,
    doc: Doc,
    host: GestureHost<Doc>,
    salt: &'static str,
}

impl Session {
    fn new(value: f32) -> Self {
        Session { frames: Frames::new(), doc: Doc { value, steps: Vec::new() }, host: GestureHost::new(), salt: "gain" }
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        let (doc, host, salt) = (&mut self.doc, &mut self.host, self.salt);
        self.frames.frame(events, |ui: &mut Ui| {
            let out = Scrub::new(Bind::given(doc.value)).label("Gain").doc_key(DocKey::new("trial.gain")).id_salt(salt).show_at(ui, AT);
            if let Some(event) = out.event {
                host.handle(doc, event, |doc, value| doc.set(value));
            }
        });
    }

    fn drag(&mut self) -> Pos2 {
        let at = Pos2::new(AT.left() + 10.0, AT.center().y);
        for _ in 0..3 {
            self.frame(vec![]);
        }
        self.frame(vec![moved(at)]);
        self.frame(vec![press(at)]);
        self.frame(vec![moved(at + Vec2::new(30.0, 0.0))]);
        self.frame(vec![moved(at + Vec2::new(40.0, 0.0))]);
        self.frame(vec![]);
        at + Vec2::new(40.0, 0.0)
    }
}

/// A drag edits the document on every proposal and still leaves one undo step, named by the component's label.
#[trial]
fn a_gesture_leaves_one_named_undo_step() {
    let mut session = Session::new(INITIAL);
    let end = session.drag();
    assert!(session.doc.steps.len() > 1, "the preview edits recorded steps while dragging: {}", session.doc.steps.len());
    session.frame(vec![release(end)]);
    session.frame(vec![]);
    assert_eq!(session.doc.steps.len(), 1, "one step for the gesture");
    assert_eq!(session.doc.steps[0], (INITIAL, Some("Gain".to_owned())));
    assert!(near(session.doc.value, DRAGGED));
}

/// Escape takes the gesture's edits back and leaves no step.
#[trial]
fn a_cancelled_gesture_leaves_no_undo_step() {
    let mut session = Session::new(INITIAL);
    session.drag();
    session.frame(vec![key_press(Key::Escape, Modifiers::NONE)]);
    session.frame(vec![]);
    assert!(session.doc.steps.is_empty(), "no step: {:?}", session.doc.steps);
    assert_eq!(session.doc.value, INITIAL);
}

/// A component evicted while its drag runs ends the gesture as removed: the host takes its edits back.
#[trial]
fn an_evicted_components_gesture_is_cancelled_as_removed() {
    let mut session = Session::new(INITIAL);
    session.drag();
    assert!(session.host.is_active() && !session.doc.steps.is_empty(), "the drag edits the document");
    for _ in 0..6 {
        session.frames.advance(0.2);
        session.frames.frame(vec![], |_: &mut Ui| {});
    }
    craft_ui::testing::memory::dispose_evicted(&session.frames.ctx);
    session.host.handle_removed(&session.frames.ctx, &mut session.doc);
    assert!(!session.host.is_active(), "the gesture ended");
    assert_eq!((session.doc.steps.len(), session.doc.value), (0, INITIAL), "its edits are taken back");
}

/// Undo during a drag: the host takes the gesture back, cancels every running gesture, and the component's own cancel
/// is ignored.
#[trial]
fn undo_during_a_gesture_takes_it_back_once() {
    let mut session = Session::new(INITIAL);
    session.drag();
    session.host.before_undo(&session.frames.ctx, &mut session.doc);
    assert_eq!((session.doc.steps.len(), session.doc.value), (0, INITIAL), "the gesture's edits are gone before the app undoes");
    session.frame(vec![moved(Pos2::new(AT.left() + 60.0, AT.center().y))]);
    session.frame(vec![]);
    assert!(session.doc.steps.is_empty());
    assert!(!session.host.is_active());
}

/// A cancel because the document moved under the gesture leaves the document alone and folds the gesture's steps.
#[trial]
fn a_cancel_for_a_moved_document_folds_the_steps_and_keeps_the_document() {
    let mut doc = Doc::default();
    let mut host = GestureHost::new();
    let begin = EditEvent::<f32>::begin(GestureId::new(1), Mixed::Same(0.0), "Gain".into(), None);
    host.handle(&mut doc, begin, |doc, value| doc.set(value));
    for value in [1.0, 2.0] {
        host.handle(&mut doc, EditEvent::update(GestureId::new(1), value), |doc, value| doc.set(value));
    }
    let cancel = EditEvent::<f32>::cancel(GestureId::new(1), CancelReason::HostChanged, None);
    host.handle(&mut doc, cancel, |doc, value| doc.set(value));
    assert_eq!((doc.value, doc.steps.len()), (2.0, 1), "the document stands and one step holds the gesture");
}

/// A local history takes the same events: a gesture is one step named by its label, and a cancel leaves none.
#[trial]
fn a_local_history_records_one_named_step_per_gesture() {
    let mut history = EditHistory::new(&0.0f32);
    let id = craft_behavior::GestureId::new(1);
    history.record(&EditEvent::begin(id, Mixed::Same(0.0), "Gain".into(), None));
    history.record(&EditEvent::update(id, 5.0));
    history.record(&EditEvent::commit(id, Mixed::Same(0.0), 7.0));
    let id = id.next();
    history.record(&EditEvent::begin(id, Mixed::Same(7.0), "Pan".into(), None));
    history.record(&EditEvent::cancel(id, CancelReason::Escape, Some(Mixed::Same(7.0))));
    assert_eq!(history.undo_name(), Some("Gain"));
    let mut now = 7.0;
    assert!(history.step(&mut now, Step::Undo));
    assert_eq!(now, 0.0, "the whole gesture is one step");
    assert!(!history.can_undo());
    assert!(history.step(&mut now, Step::Redo));
    assert_eq!((now, history.redo_name()), (7.0, None));
}

/// Gesture ids are shared by every component: an undo during one component's drag does not make the host drop the
/// next component's gesture.
#[trial]
fn an_undo_during_one_components_drag_does_not_drop_anothers_edit() {
    let mut session = Session::new(INITIAL);
    let end = session.drag();
    session.host.before_undo(&session.frames.ctx, &mut session.doc);
    session.frame(vec![release(end)]);
    session.frame(vec![]);
    session.salt = "pan";
    session.drag();
    session.frame(vec![release(Pos2::new(AT.left() + 50.0, AT.center().y))]);
    session.frame(vec![]);
    assert!(near(session.doc.value, DRAGGED), "the second component's edit reached the document: {}", session.doc.value);
    assert_eq!(session.doc.steps.len(), 1, "one step for the second gesture");
}
