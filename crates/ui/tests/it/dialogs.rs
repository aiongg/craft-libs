//! The dialog host and stack.

use craft_ui::dialog::{DialogHost, DialogId, DialogResult, DialogStack, StackDialog, StartupStage, dialog_id};
use craft_ui::testing::trial;
use egui::{Context, Ui};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};


/// A dialog with no content: it records whether it was the top of the stack when shown and finishes with
/// `finish` once set.
struct Probe<const N: usize> {
    key: &'static str,
    top: bool,
    finish: Option<DialogResult<u32>>,
}

impl<const N: usize> Probe<N> {
    fn new(key: &'static str) -> Self {
        Self { key, top: false, finish: None }
    }
}

impl<const N: usize> StackDialog for Probe<N> {
    type Result = u32;

    fn id(&self) -> DialogId {
        dialog_id(self.key)
    }

    fn show(&mut self, _: &mut Ui, stack: &DialogStack) -> Option<DialogResult<u32>> {
        self.top = stack.is_top(StackDialog::id(self));
        self.finish.take()
    }
}

fn run(host: &mut DialogHost, ctx: &Context) {
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| host.run(ui));
}

#[trial]
fn only_the_newest_open_dialog_is_the_top() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("first"));
    host.open(Probe::<2>::new("second"));
    run(&mut host, &ctx);
    assert!(!host.get::<Probe<1>>().unwrap().top);
    assert!(host.get::<Probe<2>>().unwrap().top);

    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Ok(0));
    run(&mut host, &ctx);
    run(&mut host, &ctx);
    assert!(host.get::<Probe<1>>().unwrap().top);
    assert!(!host.is_open::<Probe<2>>());
}

#[trial]
fn a_finished_dialog_is_taken_once_with_its_typed_result() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("one"));
    run(&mut host, &ctx);
    assert_eq!(host.take::<Probe<1>>(), None);

    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(7));
    run(&mut host, &ctx);
    assert!(host.is_empty());
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Ok(7)));
    assert_eq!(host.take::<Probe<1>>(), None);
}

#[trial]
fn cancelling_a_parent_cancels_its_children() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    let parent = host.open(Probe::<1>::new("parent"));
    host.open_child(Probe::<2>::new("child"), parent);
    run(&mut host, &ctx);

    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Cancel);
    run(&mut host, &ctx);
    assert!(host.is_open::<Probe<1>>() && !host.is_open::<Probe<2>>());
    assert_eq!(host.take::<Probe<2>>(), Some(DialogResult::Cancel));

    host.open_child(Probe::<2>::new("child"), parent);
    assert!(host.cancel::<Probe<1>>(&ctx));
    assert!(host.is_empty() && host.stack().is_empty());
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Cancel));
    assert_eq!(host.take::<Probe<2>>(), Some(DialogResult::Cancel));
}

#[trial]
fn startup_modals_open_one_at_a_time_in_stage_order() {
    let ctx = Context::default();
    let mut host = DialogHost::new();
    host.enqueue(StartupStage::Update, Probe::<3>::new("update"));
    host.enqueue(StartupStage::Recovery, Probe::<1>::new("recovery"));
    host.enqueue(StartupStage::Warning, Probe::<2>::new("warning"));
    assert!(!host.is_empty());

    run(&mut host, &ctx);
    assert!(host.is_open::<Probe<1>>() && !host.is_open::<Probe<2>>() && !host.is_open::<Probe<3>>());
    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(1));
    run(&mut host, &ctx);
    assert!(host.is_open::<Probe<2>>() && !host.is_open::<Probe<3>>());
    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Ok(2));
    run(&mut host, &ctx);
    assert!(host.is_open::<Probe<3>>());
    host.get_mut::<Probe<3>>().unwrap().finish = Some(DialogResult::Ok(3));
    run(&mut host, &ctx);
    assert!(host.is_empty());
}

#[trial]
fn opening_the_same_dialog_type_again_cancels_the_open_one() {
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("old"));
    host.open(Probe::<1>::new("new"));
    assert_eq!(host.get::<Probe<1>>().unwrap().key, "new");
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Cancel));
    assert_eq!(host.stack().ids().count(), 1);
}

#[trial]
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
    run(&mut host, &ctx);
    // An untaken result is dropped when a new `D` opens.
    assert!(host.open_unless_open(Probe::<1>::new("fourth")).is_ok());
    assert_eq!(host.take::<Probe<1>>(), None);
}

/// Counts the times the host told it that it was closed from outside.
struct Releasing {
    key: &'static str,
    released: Arc<AtomicU32>,
}

impl StackDialog for Releasing {
    type Result = ();

    fn id(&self) -> DialogId {
        dialog_id(self.key)
    }

    fn show(&mut self, _: &mut Ui, _: &DialogStack) -> Option<DialogResult<()>> {
        None
    }

    fn cancelled(&mut self, _: &Context) {
        self.released.fetch_add(1, Ordering::SeqCst);
    }
}

#[trial]
fn a_dialog_closed_from_outside_is_told_so() {
    let ctx = Context::default();
    let count = Arc::new(AtomicU32::new(0));
    let mut host = DialogHost::new();
    host.open(Releasing { key: "a", released: count.clone() });
    assert!(host.cancel::<Releasing>(&ctx));
    assert_eq!(count.load(Ordering::SeqCst), 1);

    host.open(Releasing { key: "b", released: count.clone() });
    host.open(Releasing { key: "c", released: count.clone() });
    run(&mut host, &ctx);
    assert_eq!(count.load(Ordering::SeqCst), 2, "the replaced dialog is told in the pass's run");

    let parent = host.open(Probe::<1>::new("parent"));
    host.open_child(Releasing { key: "d", released: count.clone() }, parent);
    assert!(host.cancel::<Probe<1>>(&ctx));
    assert_eq!(count.load(Ordering::SeqCst), 3, "a child is told when its parent closes");
}

#[trial]
fn the_last_opened_is_the_top_and_parents_know_their_children() {
    let (a, b, c) = (dialog_id("a"), dialog_id("b"), dialog_id("c"));
    let mut stack = DialogStack::default();
    stack.push(a, None);
    stack.push(b, Some(a));
    stack.push(b, Some(a));
    stack.push(c, None);
    assert_eq!(stack.ids().collect::<Vec<_>>(), [a, b, c]);
    assert!(stack.is_top(c) && stack.has_child(a) && !stack.has_child(b));
    assert_eq!(stack.children(a).collect::<Vec<_>>(), [b]);
    stack.remove(c);
    assert!(stack.is_top(b));
}

#[trial]
fn remove_tree_takes_the_descendants_and_leaves_the_rest() {
    let (a, b, c, d) = (dialog_id("a"), dialog_id("b"), dialog_id("c"), dialog_id("d"));
    let mut stack = DialogStack::default();
    stack.push(a, None);
    stack.push(b, Some(a));
    stack.push(c, Some(b));
    stack.push(d, None);
    stack.remove_tree(a);
    assert_eq!(stack.ids().collect::<Vec<_>>(), [d]);
}
