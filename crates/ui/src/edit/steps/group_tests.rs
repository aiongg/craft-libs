//! Behaviour tests of the grouping marks and `QuietMarks` (task H2).

use egui::{Context, Modifiers};

use super::{HistoryStep, QuietMarks, StepList};
use crate::edit::{EditEvent, GestureHost, GestureId, UndoMarks};

fn step(label: &str) -> HistoryStep {
    HistoryStep::new(label)
}

// `mark`, three records, `squash_since`: one new state with the first record's label; the later records return `extended`.
#[test]
fn a_gesture_is_one_state() {
    let mut list = StepList::new(step("Open"));
    let mark = list.mark();
    let first = list.record(step("Brush"));
    let second = list.record(step("later"));
    let third = list.record(step("latest"));
    list.squash_since(mark);
    assert!(!first.extended);
    assert!(second.extended && third.extended);
    assert_eq!((second.id, third.id), (first.id, first.id));
    assert_eq!(list.len(), 2);
    assert_eq!(list.label_of(first.id), Some("Brush"));
    assert!(!list.record(step("next")).extended, "a closed run is not extended");
}

// `mark` then `revert_to` with no record leaves the rows and the revision as they were.
#[test]
fn an_unchanged_gesture_leaves_no_state() {
    let mut list = StepList::new(step("Open"));
    list.record(step("one"));
    let (revision, current) = (list.revision(), list.current_id());
    let mark = list.mark();
    list.revert_to(mark);
    assert_eq!((list.len(), list.revision(), list.current_id()), (2, revision, current));
    assert!(list.take_dropped().is_empty());
}

// `mark`, two records, `revert_to`: the run's state is dropped and reported, the marked state is current.
#[test]
fn cancel_removes_the_state_the_gesture_made() {
    let mut list = StepList::new(step("Open"));
    let before = list.record(step("one")).id;
    let mark = list.mark();
    let made = list.record(step("drag")).id;
    list.record(step("drag"));
    list.revert_to(mark);
    assert_eq!(list.len(), 2);
    assert_eq!(list.current_id(), before);
    assert_eq!(list.take_dropped(), vec![made]);
    assert!(list.row_of(made).is_none());
}

// Three `record_keyed` with one key make one state; a different key starts a new one.
#[test]
fn keyed_records_with_the_same_key_merge_into_one_state() {
    let mut list = StepList::new(step("Open"));
    let first = list.record_keyed(step("Type"), "text");
    let second = list.record_keyed(step("Type"), "text");
    let third = list.record_keyed(step("Type"), "text");
    assert!(!first.extended && second.extended && third.extended);
    assert_eq!((second.id, third.id), (first.id, first.id));
    assert_eq!(list.len(), 2);
    let other = list.record_keyed(step("Nudge"), "nudge");
    assert!(!other.extended);
    assert_eq!(list.len(), 3);
}

// `record_keyed`, `back`, `forward`, `record_keyed` with the same key adds a state; so does a plain `record` between them.
#[test]
fn a_jump_or_another_record_ends_a_keyed_run() {
    let mut list = StepList::new(step("Open"));
    list.record_keyed(step("Type"), 1);
    list.back();
    list.forward();
    assert!(!list.record_keyed(step("Type"), 1).extended);
    assert_eq!(list.len(), 3);

    list.record(step("Other"));
    assert!(!list.record_keyed(step("Type"), 1).extended);
    assert_eq!(list.len(), 5);
}

// `GestureHost<QuietMarks<_>>` begin, update, commit and cancel leave a `StepList` untouched and restore the value on cancel.
#[test]
fn quiet_marks_never_touch_a_step_list() {
    let list = StepList::new(step("Open"));
    let revision = list.revision();
    let ctx = Context::default();
    let mut host: GestureHost<QuietMarks<i32>> = GestureHost::new();
    let mut value = QuietMarks(1);
    let set = |doc: &mut QuietMarks<i32>, v: i32| doc.0 = v;

    let id = GestureId::next();
    host.handle(&ctx, &mut value, EditEvent::begin(id), set);
    host.handle(&ctx, &mut value, EditEvent::update(id, 5, Modifiers::NONE), set);
    assert_eq!(value.0, 5);
    host.handle(&ctx, &mut value, EditEvent::commit(id, 5, true, Modifiers::NONE), set);
    assert_eq!(value.0, 5);

    let id = GestureId::next();
    host.handle(&ctx, &mut value, EditEvent::begin(id), set);
    host.handle(&ctx, &mut value, EditEvent::update(id, 9, Modifiers::NONE), set);
    host.handle(&ctx, &mut value, EditEvent::cancel(id, crate::edit::CancelReason::Escape), set);
    assert_eq!(value.0, 5);
    assert_eq!(value.undo_mark(), 5);
    assert_eq!((list.len(), list.revision()), (1, revision));
}
