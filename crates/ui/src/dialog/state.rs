//! [`DialogState`]: the chrome state a host keeps for one dialog while it is open.

use egui::{Context, Id};

use crate::edit::{GestureHost, OuterGesture, UndoMarks};

/// What a dialog needs between passes, held by the host next to its fields. It never holds the
/// fields: [`Dialog::show`](super::Dialog::show) takes them by `&mut`. `H` is the host's fields
/// type; the state keeps a copy taken when the dialog opens, for Reset.
#[derive(Clone, Debug)]
pub struct DialogState<H = ()> {
    open: bool,
    outer: Option<OuterGesture>,
    snapshot: Option<H>,
    return_focus: Option<Id>,
    focus_due: bool,
}

impl<H> Default for DialogState<H> {
    fn default() -> Self {
        Self { open: false, outer: None, snapshot: None, return_focus: None, focus_due: false }
    }
}

impl<H> DialogState<H> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the dialog has been shown and not yet answered.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The outer gesture begun when the dialog opened, while it is open.
    pub fn outer(&self) -> Option<OuterGesture> {
        self.outer
    }

    /// Marks the dialog open: the next [`Dialog::show`](super::Dialog::show) opens it afresh.
    /// The dialog does this itself on an `Ok`, `Cancel` or `Close` outcome; a host that removes a
    /// dialog without an outcome calls it so the next open starts clean.
    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub(super) fn begin(&mut self, ctx: &Context, snapshot: H, outer: bool) {
        self.open = true;
        self.snapshot = Some(snapshot);
        self.outer = outer.then(|| OuterGesture::begin(ctx));
        self.return_focus = ctx.memory(|m| m.focused());
        self.focus_due = true;
    }

    pub(super) fn take_focus_due(&mut self) -> bool {
        std::mem::take(&mut self.focus_due)
    }

    pub(super) fn focus_due(&self) -> bool {
        self.focus_due
    }

    pub(super) fn return_focus(&self) -> Option<Id> {
        self.return_focus
    }
}

impl<H> DialogState<H> {
    /// Answers [`Outcome::Apply`](super::Outcome::Apply) for a dialog with an outer gesture: the
    /// gesture is committed (one undo step) and a new one begins, so Cancel afterwards reverts
    /// only what came after the Apply. Does nothing without an outer gesture.
    pub fn apply<U: UndoMarks>(&mut self, ctx: &Context, host: &mut GestureHost<U>, doc: &mut U) {
        let Some(old) = self.outer else { return };
        host.commit(doc, old.id());
        let new = OuterGesture::begin(ctx);
        host.begin_outer(ctx, doc, new);
        self.outer = Some(new);
    }
}

impl<H: Clone> DialogState<H> {
    /// Sets `fields` back to the values they had when the dialog opened; false before it opened.
    /// The host passes each changed control's value on as an update.
    pub fn reset(&self, fields: &mut H) -> bool {
        match &self.snapshot {
            Some(snapshot) => {
                fields.clone_from(snapshot);
                true
            }
            None => false,
        }
    }
}
