//! The dialog host: ordering, results, the startup queue.

use egui::Context;

use super::*;

/// A dialog with no drawing: it records whether it was the top of the stack when shown and
/// finishes with `finish` once set.
struct Probe<const N: usize> {
    key: &'static str,
    interactive: bool,
    finish: Option<DialogResult<u32>>,
}

impl<const N: usize> Probe<N> {
    fn new(key: &'static str) -> Self {
        Self { key, interactive: false, finish: None }
    }
}

impl<const N: usize> StackDialog for Probe<N> {
    type Result = u32;

    fn id(&self) -> DialogId {
        dialog_id(self.key)
    }

    fn show(&mut self, _ctx: &Context, stack: &DialogStack) -> Option<DialogResult<u32>> {
        self.interactive = stack.is_top(self.id());
        self.finish.take()
    }
}

// Two dialogs opened in turn: the first is disabled while the second is up, and takes the input
// again once the second finishes.
#[test]
fn only_the_newest_open_dialog_takes_the_input() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("first"));
    host.open(Probe::<2>::new("second"));
    host.show(&ctx);
    assert!(!host.get::<Probe<1>>().unwrap().interactive);
    assert!(host.get::<Probe<2>>().unwrap().interactive);

    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Ok(0));
    host.show(&ctx);
    host.show(&ctx);
    assert!(host.get::<Probe<1>>().unwrap().interactive);
    assert!(!host.is_open::<Probe<2>>());
}

// OK on a typed dialog puts `DialogResult::Ok(value)` in `take::<D>()` on the next pass, once.
#[test]
fn a_finished_dialog_is_taken_once_with_its_typed_result() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("one"));
    host.show(&ctx);
    assert_eq!(host.take::<Probe<1>>(), None);

    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(7));
    host.show(&ctx);
    assert!(host.is_empty());
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Ok(7)));
    assert_eq!(host.take::<Probe<1>>(), None);
}

// Esc on a child closes only the child; cancelling the parent closes both with Cancel results.
#[test]
fn cancelling_a_parent_cancels_its_children() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    let parent = host.open(Probe::<1>::new("parent"));
    host.open_child(Probe::<2>::new("child"), parent);
    host.show(&ctx);

    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Cancel);
    host.show(&ctx);
    assert!(host.is_open::<Probe<1>>() && !host.is_open::<Probe<2>>());
    assert_eq!(host.take::<Probe<2>>(), Some(DialogResult::Cancel));

    host.open_child(Probe::<2>::new("child"), parent);
    assert!(host.cancel::<Probe<1>>(&ctx));
    assert!(host.is_empty() && host.stack().is_empty());
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Cancel));
    assert_eq!(host.take::<Probe<2>>(), Some(DialogResult::Cancel));
}

// Modals enqueued as Update, Recovery, Warning open one at a time in Recovery, Warning, Update
// order, each after the previous one closed (gap 75).
#[test]
fn startup_modals_open_one_at_a_time_in_stage_order() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.enqueue(StartupStage::Update, Probe::<3>::new("update"));
    host.enqueue(StartupStage::Recovery, Probe::<1>::new("recovery"));
    host.enqueue(StartupStage::Warning, Probe::<2>::new("warning"));
    assert!(!host.is_empty());

    host.show(&ctx);
    assert!(host.is_open::<Probe<1>>() && !host.is_open::<Probe<2>>() && !host.is_open::<Probe<3>>());
    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(1));
    host.show(&ctx);
    assert!(host.is_open::<Probe<2>>() && !host.is_open::<Probe<3>>());
    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Ok(2));
    host.show(&ctx);
    assert!(host.is_open::<Probe<3>>());
    host.get_mut::<Probe<3>>().unwrap().finish = Some(DialogResult::Ok(3));
    host.show(&ctx);
    assert!(host.is_empty());
}

// Opening a second dialog of the same type cancels the first; its result is Cancel.
#[test]
fn opening_the_same_dialog_type_again_cancels_the_open_one() {
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("old"));
    host.open(Probe::<1>::new("new"));
    assert_eq!(host.get::<Probe<1>>().unwrap().key, "new");
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Cancel));
    assert_eq!(host.stack().ids().count(), 1);
}

// `open_unless_open` refuses a second dialog of the type and leaves the first untouched; `open`
// replaces it, and the replaced one's Cancel is the result to take (once).
#[test]
fn open_unless_open_refuses_and_open_replaces() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    assert!(host.open_unless_open(Probe::<1>::new("first")).is_ok());
    let refused = host.open_unless_open(Probe::<1>::new("second")).unwrap_err();
    assert_eq!(refused.key, "second");
    assert_eq!(host.get::<Probe<1>>().unwrap().key, "first");
    assert_eq!(host.take::<Probe<1>>(), None);

    host.open(Probe::<1>::new("third"));
    assert_eq!(host.get::<Probe<1>>().unwrap().key, "third");
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Cancel));
    assert_eq!(host.take::<Probe<1>>(), None);

    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(5));
    host.show(&ctx);
    // An untaken result is dropped when a new `D` opens.
    assert!(host.open_unless_open(Probe::<1>::new("fourth")).is_ok());
    assert_eq!(host.take::<Probe<1>>(), None);
}

fn blur() -> ParamDialog {
    ParamDialog::new("blur", "Blur", vec![Param::new("radius", "Radius", ParamKind::Int { min: None, max: None, default: 3 })])
}

// A Cancel says which dialog it was, also when a second dialog of the type replaced the first.
#[test]
fn a_cancel_carries_the_dialogs_key() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.open(blur());
    host.open(ParamDialog::new("sharpen", "Sharpen", Vec::new()));
    assert_eq!(host.take_keyed::<ParamDialog>().map(|(result, key)| (result.is_cancel(), key)), Some((true, Some("blur".to_owned()))));
    assert!(host.cancel::<ParamDialog>(&ctx));
    assert_eq!(host.take_keyed::<ParamDialog>().map(|(result, key)| (result.is_cancel(), key)), Some((true, Some("sharpen".to_owned()))));
}

// The top dialog is found by id and downcast to its type, without a registry of the app's own.
#[test]
fn the_top_dialog_is_found_by_id() {
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("under"));
    host.open(blur());
    let top = host.top_id().expect("a dialog is open");
    assert_eq!(top, dialog_id("blur"));
    assert!(host.get_dyn(top).is_some_and(|dialog| dialog.is::<ParamDialog>()));
    assert!(host.get_by_id::<ParamDialog>(top).is_some());
    assert!(host.get_by_id::<ParamDialog>(dialog_id("under")).is_none());
    host.get_dyn_mut(top).and_then(|dialog| dialog.downcast_mut::<ParamDialog>()).expect("typed").fields_mut().set("radius", 9_i64);
    assert_eq!(host.get::<ParamDialog>().unwrap().fields().int("radius"), Some(9));
}

// `confirm` answers OK with the current values, closes the dialog and keeps the answer for `take`;
// a disabled OK refuses.
#[test]
fn confirm_answers_ok_for_automation() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.open(blur().ok_enabled(false));
    assert!(!host.confirm::<ParamDialog>(&ctx));
    assert!(host.is_open::<ParamDialog>());
    host.get_mut::<ParamDialog>().unwrap().set_ok_enabled(true);
    host.get_mut::<ParamDialog>().unwrap().fields_mut().set("radius", 8_i64);
    assert!(host.confirm::<ParamDialog>(&ctx));
    assert!(!host.is_open::<ParamDialog>());
    let Some(DialogResult::Ok(answer)) = host.take::<ParamDialog>() else { panic!("an OK answer") };
    assert_eq!((answer.key(), answer.fields.int("radius")), ("blur", Some(8)));
    assert!(!host.confirm::<ParamDialog>(&ctx), "nothing open");
}

// Dialogs of one key and different instances are open together; each is reached, confirmed and
// taken by id with its own fields. A second open of the same instance replaces the first, and
// without an instance one per type holds (`opening_the_same_dialog_type_again_cancels_the_open_one`).
#[test]
fn dialogs_of_different_instances_stay_open_and_answer_by_id() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    let first = host.open(blur().instance("action 1"));
    let second = host.open(blur().instance("action 2"));
    assert_ne!(first, second);
    host.get_mut_by_id::<ParamDialog>(first).unwrap().fields_mut().set("radius", 4_i64);
    host.get_mut_by_id::<ParamDialog>(second).unwrap().fields_mut().set("radius", 9_i64);
    assert!(host.confirm_by_id::<ParamDialog>(&ctx, first));
    assert!(host.is_open::<ParamDialog>(), "the second stays open");
    assert!(host.confirm_by_id::<ParamDialog>(&ctx, second));
    let Some(DialogResult::Ok(answer)) = host.take_by_id::<ParamDialog>(second) else { panic!("second answered OK") };
    assert_eq!(answer.fields.int("radius"), Some(9));
    let Some(DialogResult::Ok(answer)) = host.take_by_id::<ParamDialog>(first) else { panic!("first answered OK") };
    assert_eq!(answer.fields.int("radius"), Some(4));
    assert!(host.take_by_id::<ParamDialog>(first).is_none());

    host.open(blur().instance("action 1"));
    host.open(blur().instance("action 1"));
    assert!(host.cancel_by_id::<ParamDialog>(&ctx, first));
    assert!(!host.is_open::<ParamDialog>(), "the same instance was replaced, not added");
}

// A dialog with an instance keeps its result for its owner: `take` passes over it, and opening
// another dialog of the type without an instance doesn't drop it.
#[test]
fn an_instance_result_is_taken_only_by_id() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    let own = host.open(blur().instance("library"));
    assert!(host.confirm_by_id::<ParamDialog>(&ctx, own));
    host.open(blur());
    assert!(host.take::<ParamDialog>().is_none(), "no result without an instance waits");
    assert!(matches!(host.take_by_id::<ParamDialog>(own), Some(DialogResult::Ok(_))));
}

/// Counts the times the host told it that it was closed from outside.
struct Releasing {
    key: &'static str,
    released: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl StackDialog for Releasing {
    type Result = ();

    fn id(&self) -> DialogId {
        dialog_id(self.key)
    }

    fn show(&mut self, _ctx: &Context, _stack: &DialogStack) -> Option<DialogResult<()>> {
        None
    }

    fn cancelled(&mut self, _ctx: &Context) {
        self.released.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

// A dialog closed from outside (cancel, replaced by a dialog of its type, parent closing) learns of
// it in the same pass; one that finishes itself does not.
#[test]
fn a_dialog_closed_from_outside_is_told_so() {
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };
    let ctx = Context::default();
    let count = Arc::new(AtomicU32::new(0));
    let mut host = DialogHost::new();
    host.open(Releasing { key: "a", released: count.clone() });
    assert!(host.cancel::<Releasing>(&ctx));
    assert_eq!(count.load(Ordering::SeqCst), 1);

    host.open(Releasing { key: "b", released: count.clone() });
    host.open(Releasing { key: "c", released: count.clone() });
    host.show(&ctx);
    assert_eq!(count.load(Ordering::SeqCst), 2, "the replaced dialog is told in the pass's show");

    let parent = host.open(Probe::<1>::new("parent"));
    host.open_child(Releasing { key: "d", released: count.clone() }, parent);
    assert!(host.cancel::<Probe<1>>(&ctx));
    assert_eq!(count.load(Ordering::SeqCst), 3, "a child is told when its parent closes");
}
