//! The dialog host and stack.

use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::{Dialog, DialogHost, DialogId, DialogResult, DialogStack, StackDialog, StartupStage, dialog_id};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, Frames, pointer, trial};
use egui::{Context, Key, Modifiers, Pos2, Ui, Vec2};
use std::cell::Cell;
use std::rc::Rc;
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

fn run(host: &mut DialogHost, frames: &mut Frames) {
    frames.frame(Vec::new(), |ui| host.run(ui));
}

#[trial]
fn only_the_newest_open_dialog_is_the_top() {
    let mut frames = Frames::new();
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("first"));
    host.open(Probe::<2>::new("second"));
    run(&mut host, &mut frames);
    assert!(!host.get::<Probe<1>>().unwrap().top);
    assert!(host.get::<Probe<2>>().unwrap().top);

    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Ok(0));
    run(&mut host, &mut frames);
    run(&mut host, &mut frames);
    assert!(host.get::<Probe<1>>().unwrap().top);
    assert!(!host.is_open::<Probe<2>>());
}

#[trial]
fn a_finished_dialog_is_taken_once_with_its_typed_result() {
    let mut frames = Frames::new();
    let mut host = DialogHost::new();
    host.open(Probe::<1>::new("one"));
    run(&mut host, &mut frames);
    assert_eq!(host.take::<Probe<1>>(), None);

    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(7));
    run(&mut host, &mut frames);
    assert!(host.is_empty());
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Ok(7)));
    assert_eq!(host.take::<Probe<1>>(), None);
}

#[trial]
fn cancelling_a_parent_cancels_its_children() {
    let mut frames = Frames::new();
    let mut host = DialogHost::new();
    let parent = host.open(Probe::<1>::new("parent"));
    host.open_child(Probe::<2>::new("child"), parent);
    run(&mut host, &mut frames);

    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Cancel);
    run(&mut host, &mut frames);
    assert!(host.is_open::<Probe<1>>() && !host.is_open::<Probe<2>>());
    assert_eq!(host.take::<Probe<2>>(), Some(DialogResult::Cancel));

    host.open_child(Probe::<2>::new("child"), parent);
    assert!(host.cancel::<Probe<1>>(&frames.ctx));
    assert!(host.is_empty() && host.stack().is_empty());
    assert_eq!(host.take::<Probe<1>>(), Some(DialogResult::Cancel));
    assert_eq!(host.take::<Probe<2>>(), Some(DialogResult::Cancel));
}

#[trial]
fn startup_modals_open_one_at_a_time_in_stage_order() {
    let mut frames = Frames::new();
    let mut host = DialogHost::new();
    host.enqueue(StartupStage::Update, Probe::<3>::new("update"));
    host.enqueue(StartupStage::Recovery, Probe::<1>::new("recovery"));
    host.enqueue(StartupStage::Warning, Probe::<2>::new("warning"));
    assert!(!host.is_empty());

    run(&mut host, &mut frames);
    assert!(host.is_open::<Probe<1>>() && !host.is_open::<Probe<2>>() && !host.is_open::<Probe<3>>());
    host.get_mut::<Probe<1>>().unwrap().finish = Some(DialogResult::Ok(1));
    run(&mut host, &mut frames);
    assert!(host.is_open::<Probe<2>>() && !host.is_open::<Probe<3>>());
    host.get_mut::<Probe<2>>().unwrap().finish = Some(DialogResult::Ok(2));
    run(&mut host, &mut frames);
    assert!(host.is_open::<Probe<3>>());
    host.get_mut::<Probe<3>>().unwrap().finish = Some(DialogResult::Ok(3));
    run(&mut host, &mut frames);
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
    let mut frames = Frames::new();
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
    run(&mut host, &mut frames);
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
    let mut frames = Frames::new();
    let count = Arc::new(AtomicU32::new(0));
    let mut host = DialogHost::new();
    host.open(Releasing { key: "a", released: count.clone() });
    assert!(host.cancel::<Releasing>(&frames.ctx));
    assert_eq!(count.load(Ordering::SeqCst), 1);

    host.open(Releasing { key: "b", released: count.clone() });
    host.open(Releasing { key: "c", released: count.clone() });
    run(&mut host, &mut frames);
    assert_eq!(count.load(Ordering::SeqCst), 2, "the replaced dialog is told in the pass's run");

    let parent = host.open(Probe::<1>::new("parent"));
    host.open_child(Releasing { key: "d", released: count.clone() }, parent);
    assert!(host.cancel::<Probe<1>>(&frames.ctx));
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

/// What a shown dialog reports: how it closed and which button closed it.
type Closed = Rc<Cell<(Option<CloseReason>, Option<usize>)>>;

/// A prompt (or a plain dialog) with a destructive and a cancel button; `delete` is the destructive one.
fn dialog_fixture(prompt: bool, delete: FooterButton) -> (Fixture, Closed) {
    let closed: Closed = Rc::default();
    let sink = closed.clone();
    let fixture = Fixture::new(move |ui| {
        let (mut open, mut snap) = (true, false);
        let dialog = if prompt { Dialog::prompt("Discard changes?") } else { Dialog::new("Discard changes?") };
        let shown =
            dialog.opened(Bind::bound(&mut open)).buttons([delete.clone(), FooterButton::new("Cancel", DialogRole::Cancel)]).show(ui, |ui, _| {
                Checkbox::new(Bind::bound(&mut snap)).label("Keep a copy").show(ui);
            });
        if shown.output.closed.is_some() {
            sink.set((shown.output.closed, shown.output.picked));
        }
    });
    (fixture, closed)
}

/// Drags the title band of the only dialog by 40 points and returns where the dialog was before and after.
fn drag_title(fixture: &mut Fixture) -> (egui::Rect, egui::Rect) {
    fixture.settle();
    let rect_of = |fixture: &Fixture| fixture.by_role(Role::DIALOG).first().map(|dialog| dialog.rect);
    let before = rect_of(fixture).unwrap_or(egui::Rect::NOTHING);
    let grab = Pos2::new(before.center().x, before.min.y + 12.0);
    fixture.frame(vec![pointer::moved(grab)]);
    fixture.frame(vec![pointer::press(grab)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(grab + Vec2::new(10.0, 10.0) * step as f32)]);
    }
    fixture.frame(vec![pointer::release(grab + Vec2::new(40.0, 40.0))]);
    fixture.idle(2);
    (before, rect_of(fixture).unwrap_or(egui::Rect::NOTHING))
}

#[trial]
fn a_prompt_opens_centred_and_a_dialog_moves_but_a_prompt_does_not() {
    let (mut prompt, _) = dialog_fixture(true, FooterButton::new("Delete", DialogRole::Destructive));
    let (before, after) = drag_title(&mut prompt);
    let middle = prompt.ctx().content_rect().center();
    assert!((before.center() - middle).abs().max_elem() < 2.0, "the prompt opens in the middle of the window: {before:?}");
    assert_eq!(before, after, "the title band of a prompt moves nothing");
    let (mut dialog, _) = dialog_fixture(false, FooterButton::new("Delete", DialogRole::Destructive));
    let (before, after) = drag_title(&mut dialog);
    assert_ne!(before, after, "the same drag moves a dialog");
}

#[trial]
fn escape_cancels_a_prompt() {
    let (mut fixture, closed) = dialog_fixture(true, FooterButton::new("Delete", DialogRole::Destructive));
    fixture.settle();
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(closed.get().0, Some(CloseReason::Escape));
}

#[trial]
fn a_footer_buttons_letter_activates_it_and_a_disabled_buttons_does_not() {
    let (mut fixture, closed) = dialog_fixture(true, FooterButton::new("Delete", DialogRole::Destructive).letter('d'));
    fixture.settle();
    fixture.key(Key::D, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(closed.get(), (Some(CloseReason::Activated), Some(0)), "the letter picks the button as a click does");

    let (mut fixture, closed) = dialog_fixture(true, FooterButton::new("Delete", DialogRole::Destructive).letter('d').enabled(false));
    fixture.settle();
    fixture.key(Key::D, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(closed.get(), (None, None), "a disabled button ignores its letter");
}

#[trial]
fn a_viewport_dialog_shows_on_its_fallback_layer_where_viewports_are_unavailable() {
    let closed: Closed = Rc::default();
    let sink = closed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        let shown = Dialog::new("Preferences").modeless().viewport().opened(Bind::bound(&mut open)).show(ui, |_, _| {});
        if shown.output.closed.is_some() {
            sink.set((shown.output.closed, shown.output.picked));
        }
    });
    fixture.settle();
    assert!(fixture.by_label("Preferences").is_some(), "the title shows on the fallback layer");
    assert_eq!(fixture.by_role(Role::DIALOG).len(), 1);
    let close = fixture.by_label("Close").expect("the close action");
    fixture.click(&close);
    fixture.idle(2);
    assert!(closed.get().0.is_some(), "the close action closes it");
}
