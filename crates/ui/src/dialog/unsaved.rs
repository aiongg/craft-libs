//! [`UnsavedChangesDialog`]: the Save / Don't Save / Cancel question for one document, asked by
//! an app running a [`CloseGuard`](craft_quit::CloseGuard).

use craft_quit::{CloseAnswer, CloseScope};
use egui::{Context, Key};

use super::buttons::{ButtonRole, Platform};
use super::dialog_host::{DialogResult, StackDialog};
use super::prompt::Prompt;
use super::stack::{DialogId, DialogStack, dialog_id};
use crate::controls::Checkbox;
use crate::localized::Labels as _;

crate::words! {
    /// The unsaved-changes question's words, English by default. Windows and Linux ask Yes / No /
    /// Cancel with "Do you want to save…", macOS Save / Don't Save / Cancel; the dialog picks the
    /// set by [`Platform`](super::Platform).
    #[non_exhaustive]
    pub struct UnsavedLabels {
        title: "unsaved.title" = "Unsaved changes";
        closing: "unsaved.closing" = "Do you want to save the changes you made to “{name}” before closing?";
        quitting: "unsaved.quitting" = "Do you want to save the changes you made to “{name}” before quitting?";
        reverting: "unsaved.reverting" = "Revert “{name}” to the last saved version? Your changes will be lost.";
        save: "unsaved.save" = "Save";
        dont_save: "unsaved.dontSave" = "Don't Save";
        yes: "unsaved.yes" = "Yes";
        no: "unsaved.no" = "No";
        revert: "unsaved.revert" = "Revert";
        apply_to_all: "unsaved.applyToAll" = "Apply to All";
    }
}

/// The answer for one document, with whether it applies to the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnsavedAnswer {
    pub answer: CloseAnswer,
    /// The "Apply to All" box was ticked: the app calls
    /// [`CloseGuard::answer_all`](craft_quit::CloseGuard::answer_all).
    pub apply_to_all: bool,
}

/// The question for one modified document. The user sees a 420 pt prompt titled "Unsaved
/// changes" with the document's name in the message, Save (default, focused), Don't Save
/// (Alternate) and Cancel in the platform's order and words, and, when the guard has more
/// documents, an "Apply to All" check box under the message (spec question 3). Keys: Enter
/// saves, Esc cancels, S / D / C (Y / N / C on Windows and Linux) answer directly; the letters
/// show in the labels off macOS. Revert asks Revert / Cancel instead; Revert answers
/// [`CloseAnswer::DontSave`].
///
/// ```ignore
/// // on GuardStep::Ask(doc):
/// dialogs.open(UnsavedChangesDialog::new(guard.scope(), doc.name()).remaining(guard.remaining()));
/// // later:
/// if let Some(DialogResult::Ok(answer)) = dialogs.take::<UnsavedChangesDialog>() {
///     let step = if answer.apply_to_all { guard.answer_all(answer.answer) } else { guard.answer(answer.answer) };
/// }
/// ```
///
/// Result: `Ok(UnsavedAnswer)` for every button, Cancel included (the guard needs it);
/// `DialogResult::Cancel` only when the host closed the dialog from outside.
pub struct UnsavedChangesDialog {
    id: DialogId,
    scope: CloseScope,
    name: String,
    remaining: usize,
    apply_to_all: bool,
}

impl UnsavedChangesDialog {
    pub fn new(scope: CloseScope, document_name: impl Into<String>) -> Self {
        Self { id: dialog_id("unsaved-changes"), scope, name: document_name.into(), remaining: 1, apply_to_all: false }
    }

    /// How many documents the guard still asks about, this one included; above one the "Apply to
    /// All" box shows.
    pub fn remaining(mut self, remaining: usize) -> Self {
        self.remaining = remaining;
        self
    }
}

impl StackDialog for UnsavedChangesDialog {
    type Result = UnsavedAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/discard_ui.rs:259 `show` (wording per scope, the three
    // buttons with keys, Windows Yes/No words), on `Prompt::new(..).answer(..)` with the check
    // box in the message; "Apply to All" is new (Photoshop).
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<UnsavedAnswer>> {
        let labels = UnsavedLabels::of_ctx(ctx);
        let cancel = super::DialogLabels::of_ctx(ctx).cancel;
        let name = self.name.as_str();
        let message = match self.scope {
            CloseScope::Close => &labels.closing,
            CloseScope::Quit => &labels.quitting,
            CloseScope::Revert => &labels.reverting,
        };
        let message = craft_i18n::fmt(message, &[("name", name)]);
        let mac = Platform::of(ctx) == Platform::Mac;
        let mut prompt = Prompt::new("unsaved-changes", labels.title.clone(), message).stack(stack);
        prompt = if self.scope == CloseScope::Revert {
            prompt.answer(ButtonRole::Default, labels.revert.clone(), Some(Key::R))
        } else if mac {
            prompt.answer(ButtonRole::Default, labels.save.clone(), Some(Key::S)).answer(
                ButtonRole::Alternate,
                labels.dont_save.clone(),
                Some(Key::D),
            )
        } else {
            prompt.answer(ButtonRole::Default, labels.yes.clone(), Some(Key::Y)).answer(ButtonRole::Alternate, labels.no.clone(), Some(Key::N))
        };
        prompt = prompt.answer(ButtonRole::Cancel, cancel, Some(Key::C));
        if self.remaining > 1 && self.scope != CloseScope::Revert {
            let apply_to_all = &mut self.apply_to_all;
            let label = labels.apply_to_all.clone();
            prompt = prompt.below(move |ui| {
                ui.add_space(ui.spacing().item_spacing.y);
                let _ = Checkbox::bool(label, apply_to_all).show(ui);
            });
        }
        let answer = match prompt.show(ctx)? {
            ButtonRole::Default if self.scope == CloseScope::Revert => CloseAnswer::DontSave,
            ButtonRole::Default => CloseAnswer::Save,
            ButtonRole::Alternate => CloseAnswer::DontSave,
            _ => CloseAnswer::Cancel,
        };
        Some(DialogResult::Ok(UnsavedAnswer { answer, apply_to_all: self.apply_to_all && self.remaining > 1 }))
    }
}
