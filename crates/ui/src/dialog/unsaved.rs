//! `UnsavedChangesDialog` (24.4): the Save, Don't Save and Cancel question for one document, a configuration of
//! `Prompt`.

use super::host::{DialogResult, StackDialog};
use super::prompt::{Prompt, Reply, buttons};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::words::UnsavedLabels;
use crate::choice::Checkbox;
use crate::contract::{Bind, Props as _, Show as _};
use crate::runtime::env::Env;
use crate::runtime::windows::os_of;
use crate::sections::DialogRole;
use craft_host::target::Os;
use craft_quit::{CloseAnswer, CloseScope};
use egui::{Context, Ui};

/// The answer for one document, with whether it applies to the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct UnsavedAnswer {
    /// The button the user pressed.
    pub answer: CloseAnswer,
    /// The "Apply to All" box was ticked: the app calls `CloseGuard::answer_all`.
    pub apply_to_all: bool,
}

/// The question for one modified document: a prompt titled "Unsaved changes" with the document's name in the
/// message, Save (the accepting answer), Don't Save and Cancel in the platform's order and words, and, while the
/// guard has more documents, an "Apply to All" check box. Windows and Linux ask Yes, No and Cancel. Reverting asks
/// Revert and Cancel; Revert answers `CloseAnswer::DontSave`.
///
/// The result is `Ok(UnsavedAnswer)` for every answer, Cancel included (the guard needs it), and
/// `DialogResult::Cancel` only when the host closed the dialog from outside.
#[derive(crate::contract::Pattern)]
pub struct UnsavedChangesDialog {
    id: DialogId,
    scope: CloseScope,
    name: String,
    remaining: usize,
    apply_to_all: bool,
    open: bool,
    /// What a programmatic OK answers; the accepting answer's when `None`.
    chosen: Option<CloseAnswer>,
}

impl UnsavedChangesDialog {
    /// The question for the document `document_name`, closing under `scope`; the guard asks about one document.
    pub fn new(scope: CloseScope, document_name: impl Into<String>) -> Self {
        Self { id: dialog_id("unsaved-changes"), scope, name: document_name.into(), remaining: 1, apply_to_all: false, open: true, chosen: None }
    }

    /// The answer a host-side `confirm` gives, as pressing that button would; the accepting answer's until set.
    pub fn set_answer(&mut self, answer: CloseAnswer) {
        self.chosen = Some(answer);
    }

    /// Sets the "Apply to All" box. It counts only while more than one document remains.
    pub fn set_apply_to_all(&mut self, on: bool) {
        self.apply_to_all = on;
    }

    /// How many documents the guard still asks about, this one included; above one the "Apply to All" box shows.
    #[must_use]
    pub fn remaining(mut self, remaining: usize) -> Self {
        self.remaining = remaining;
        self
    }

    /// The answers in the order the buttons are given: what each means and how it is asked.
    fn replies(&self, words: &UnsavedLabels, cancel: crate::runtime::env::Label, os: Os) -> Vec<(CloseAnswer, Reply)> {
        let cancel = (CloseAnswer::Cancel, Reply::new(cancel, DialogRole::Cancel, Some('C')));
        if self.scope == CloseScope::Revert {
            return vec![(CloseAnswer::DontSave, Reply::new(words.revert.clone(), DialogRole::Accept, Some('R'))), cancel];
        }
        if os == Os::Mac {
            vec![
                (CloseAnswer::Save, Reply::new(words.save.clone(), DialogRole::Accept, Some('S'))),
                (CloseAnswer::DontSave, Reply::new(words.dont_save.clone(), DialogRole::Other, Some('D'))),
                cancel,
            ]
        } else {
            vec![
                (CloseAnswer::Save, Reply::new(words.yes.clone(), DialogRole::Accept, Some('Y'))),
                (CloseAnswer::DontSave, Reply::new(words.no.clone(), DialogRole::Other, Some('N'))),
                cancel,
            ]
        }
    }

    fn answer(&self, answer: CloseAnswer) -> UnsavedAnswer {
        UnsavedAnswer { answer, apply_to_all: self.apply_to_all && self.remaining > 1 }
    }
}

impl StackDialog for UnsavedChangesDialog {
    type Result = UnsavedAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<UnsavedAnswer>> {
        let env = Env::of(ui);
        let (words, prompt_words) = (env.words::<UnsavedLabels>(), env.words::<super::words::PromptLabels>());
        let template = match self.scope {
            CloseScope::Close => &words.closing,
            CloseScope::Quit => &words.quitting,
            CloseScope::Revert => &words.reverting,
        };
        let message = template.fill(&[("name", self.name.as_str())]);
        let replies = self.replies(&words, prompt_words.cancel.clone(), os_of(ui.ctx()));
        let answers: Vec<CloseAnswer> = replies.iter().map(|(answer, _)| *answer).collect();
        let replies: Vec<Reply> = replies.into_iter().map(|(_, reply)| reply).collect();
        let mut prompt = Prompt::new(words.title.clone(), message).answers(buttons(&replies, os_of(ui.ctx()))).opened(Bind::bound(&mut self.open));
        if self.remaining > 1 && self.scope != CloseScope::Revert {
            let (apply_to_all, label) = (&mut self.apply_to_all, words.apply_to_all.clone());
            prompt = prompt.extras(move |ui| {
                Checkbox::new(Bind::bound(apply_to_all)).label(label).show(ui);
            });
        }
        let picked = prompt.show(ui).picked?;
        Some(DialogResult::Ok(self.answer(answers[picked])))
    }

    /// The answer set with `set_answer`; the accepting answer's (Save, or Don't Save when reverting) unless set.
    fn confirm(&mut self, _: &Context) -> Option<UnsavedAnswer> {
        let accepting = if self.scope == CloseScope::Revert { CloseAnswer::DontSave } else { CloseAnswer::Save };
        Some(self.answer(self.chosen.unwrap_or(accepting)))
    }
}
