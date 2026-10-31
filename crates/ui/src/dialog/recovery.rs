#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`RecoveryDialog`]: the offer to recover a document an earlier run autosaved and never saved.

use std::time::SystemTime;

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::stack::{DialogId, DialogStack, dialog_id};

crate::words! {
    /// The recovery dialog's words, English by default.
    #[non_exhaustive]
    pub struct RecoveryLabels {
        title: "recovery.title" = "Recover Unsaved Work";
        /// `{name}` and `{age}` ("5 minutes ago").
        message: "recovery.message" = "“{name}” has unsaved changes from an earlier run, autosaved {age}.";
        recover: "recovery.recover" = "Recover";
        discard: "recovery.discard" = "Discard";
        just_now: "recovery.justNow" = "less than a minute ago";
        minute_one: "recovery.minuteOne" = "{n} minute ago";
        minute_other: "recovery.minuteOther" = "{n} minutes ago";
        hour_one: "recovery.hourOne" = "{n} hour ago";
        hour_other: "recovery.hourOther" = "{n} hours ago";
        day_one: "recovery.dayOne" = "{n} day ago";
        day_other: "recovery.dayOther" = "{n} days ago";
        recently: "recovery.recently" = "recently";
    }
}

/// One autosaved document, as the app's recovery folder lists it
/// (`craft_platform::autosave::RecoveryEntry` has these fields; craft-ui doesn't depend on
/// craft-platform, so the app copies them).
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredDocument {
    /// The entry's key in the recovery folder; the app passes it back to recover or discard.
    pub key: String,
    /// The document's name ("poster.psd", "Untitled-2").
    pub name: String,
    /// When it was autosaved.
    pub saved: SystemTime,
    /// Why Recover is disabled now, shown as its tooltip ("Stop recording first"); `None`
    /// enables it.
    pub blocked: Option<String>,
}

/// What the user chose for one document. Closing the dialog (Esc, the close box) is
/// `DialogResult::Cancel`: the copy stays and is offered again at the next launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Open the autosaved copy as an unsaved document.
    Recover,
    /// Delete the autosaved copy.
    Discard,
}

/// Recover Unsaved Work, one document at a time, after SoundCraft's dialog (spec Q10; Photoshop
/// and PhotoCraft reopen recovered documents without asking, see the spec).
///
/// The user sees "“poster.psd” has unsaved changes from an earlier run, autosaved 5 minutes
/// ago." and Discard and Recover (Recover is the default button). Esc or the close box keep the
/// copy for the next launch. The app enqueues the first document at
/// [`StartupStage::Recovery`](super::StartupStage::Recovery) and opens the next after taking each
/// answer. Result: [`RecoveryAction`].
pub struct RecoveryDialog {
    id: DialogId,
    document: RecoveredDocument,
}

impl RecoveryDialog {
    pub fn new(document: RecoveredDocument) -> Self {
        Self { id: dialog_id("recovery"), document }
    }

    /// The document asked about; the app reads its key with the answer.
    pub fn document(&self) -> &RecoveredDocument {
        &self.document
    }
}

impl StackDialog for RecoveryDialog {
    type Result = RecoveryAction;

    fn id(&self) -> DialogId {
        self.id
    }

    // soundcraft: crates/ui-egui/src/dialogs/recovery.rs:8 show
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<RecoveryAction>> {
        todo!()
    }
}

/// How long ago `seconds` was: "less than a minute ago" under 60 s, then whole minutes, hours and
/// days ("1 minute ago", "3 hours ago", "2 days ago") with the catalog's plural forms.
// soundcraft: crates/ui-egui/src/dialogs/recovery.rs:42 ago
pub fn relative_age(seconds: u64, labels: &RecoveryLabels) -> String {
    todo!()
}
