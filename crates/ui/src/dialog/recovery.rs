//! `RecoveryDialog` (24.4): the offer to recover a document an earlier run autosaved and never saved, a
//! configuration of `Prompt`.

use super::host::{DialogResult, StackDialog};
use super::prompt::{Prompt, Reply, buttons};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::words::RecoveryLabels;
use crate::contract::Bind;
use crate::runtime::env::Env;
use crate::runtime::overlay::CloseReason;
use crate::runtime::windows::os_of;
use crate::sections::DialogRole;
use craft_theme::Tone;
use egui::{Context, Ui};
use std::time::Duration;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// One autosaved document, as the app's recovery folder lists it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RecoveredDocument {
    /// The entry's key in the recovery folder; the app passes it back to recover or discard.
    pub key: String,
    /// The document's name ("poster.psd", "Untitled-2").
    pub name: String,
    /// How long before the app listed the recovery folder it was autosaved.
    pub age: Duration,
    /// Why Recover is disabled now ("Stop recording first"), shown under the message; `None` enables it.
    pub blocked: Option<String>,
}

impl RecoveredDocument {
    /// A document autosaved `age` ago, with Recover available.
    pub fn new(key: impl Into<String>, name: impl Into<String>, age: Duration) -> Self {
        RecoveredDocument { key: key.into(), name: name.into(), age, blocked: None }
    }

    /// Disables Recover for `reason`.
    #[must_use]
    pub fn blocked(mut self, reason: impl Into<String>) -> Self {
        self.blocked = Some(reason.into());
        self
    }
}

/// What the user chose for one document. Closing the dialog (Escape, the close action) is `DialogResult::Cancel`:
/// the copy stays and is offered again at the next launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryAnswer {
    /// Open the autosaved copy as an unsaved document.
    Recover,
    /// Delete the autosaved copy.
    Discard,
}

/// Recover Unsaved Work, one document at a time: a prompt saying when the document was autosaved, with Discard and
/// Recover (the accepting answer). Escape and the close action keep the copy for the next launch. The app enqueues
/// the first document at `StartupStage::Recovery` and opens the next after taking each answer.
#[derive(crate::contract::Pattern)]
pub struct RecoveryDialog {
    id: DialogId,
    document: RecoveredDocument,
    open: bool,
}

impl RecoveryDialog {
    /// A dialog offering to recover `document`.
    pub fn new(document: RecoveredDocument) -> Self {
        Self { id: dialog_id("recovery"), document, open: true }
    }

    /// The document asked about; the app reads its key with the answer.
    pub fn document(&self) -> &RecoveredDocument {
        &self.document
    }
}

/// How long ago `seconds` was: the whole minutes, hours or days, "recently" under a minute.
fn age_text(words: &RecoveryLabels, format: &craft_i18n::Format, seconds: u64) -> String {
    let (count, n) = match seconds {
        0..MINUTE => return words.recently.as_str().to_owned(),
        MINUTE..HOUR => (&words.minutes_ago, seconds / MINUTE),
        HOUR..DAY => (&words.hours_ago, seconds / HOUR),
        _ => (&words.days_ago, seconds / DAY),
    };
    count.text(n, &format.integer(n))
}

impl StackDialog for RecoveryDialog {
    type Result = RecoveryAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<RecoveryAnswer>> {
        let env = Env::of(ui);
        let words = env.words::<RecoveryLabels>();
        let age = age_text(&words, env.format(), self.document.age.as_secs());
        let message = words.message.fill(&[("name", self.document.name.as_str()), ("age", age.as_str())]);
        let blocked = self.document.blocked.is_some();
        let replies = [
            Reply::new(words.discard.clone(), DialogRole::Other, None),
            Reply::new(words.recover.clone(), DialogRole::Accept, None).enabled(!blocked),
        ];
        let mut prompt = Prompt::new(words.title.clone(), message)
            .answers(buttons(&replies, os_of(ui.ctx())))
            .enter(!blocked)
            .opened(Bind::bound(&mut self.open));
        if let Some(reason) = &self.document.blocked {
            prompt = prompt.tone(Tone::Warn, reason.clone());
        }
        let shown = prompt.show(ui);
        let closed = shown.closed?;
        Some(match (closed, shown.picked) {
            (CloseReason::Activated | CloseReason::Enter, Some(0)) => DialogResult::Ok(RecoveryAnswer::Discard),
            (CloseReason::Activated | CloseReason::Enter, Some(_)) => DialogResult::Ok(RecoveryAnswer::Recover),
            _ => DialogResult::Cancel,
        })
    }

    /// Recover, the accepting answer; `None` while recovery is blocked.
    fn confirm(&mut self, _: &Context) -> Option<RecoveryAnswer> {
        self.document.blocked.is_none().then_some(RecoveryAnswer::Recover)
    }
}
