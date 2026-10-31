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
    assert!(host.cancel::<Probe<1>>());
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
