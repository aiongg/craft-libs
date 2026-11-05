//! [`RecoveryDialog`]: the offer to recover a document an earlier run autosaved and never saved.

use std::time::SystemTime;

use egui::Context;

use super::buttons::{ButtonRole, FooterButton};
use super::dialog::{Dialog, InitialFocus, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::Footer;
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::localized::{AgeStyle, Labels as _, relative_age};

crate::words! {
    /// The recovery dialog's words, English by default.
    #[non_exhaustive]
    pub struct RecoveryLabels {
        title: "recovery.title" = "Recover Unsaved Work";
        /// `{name}` and `{age}` ("5 minutes ago").
        message: "recovery.message" = "“{name}” has unsaved changes from an earlier run, autosaved {age}.";
        recover: "recovery.recover" = "Recover";
        discard: "recovery.discard" = "Discard";
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
pub enum RecoveryAnswer {
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
/// answer. Result: [`RecoveryAnswer`].
pub struct RecoveryDialog {
    id: DialogId,
    document: RecoveredDocument,
    state: DialogState,
}

impl RecoveryDialog {
    /// A dialog offering to recover `document`.
    pub fn new(document: RecoveredDocument) -> Self {
        Self { id: dialog_id("recovery"), document, state: DialogState::new() }
    }

    /// The document asked about; the app reads its key with the answer.
    pub fn document(&self) -> &RecoveredDocument {
        &self.document
    }
}

impl StackDialog for RecoveryDialog {
    type Result = RecoveryAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    // soundcraft: crates/ui-egui/src/dialogs/recovery.rs:8 show
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<RecoveryAnswer>> {
        let labels = RecoveryLabels::of_ctx(ctx);
        let age = SystemTime::now()
            .duration_since(self.document.saved)
            .map_or_else(|_| labels.recently.clone(), |d| relative_age(ctx, d.as_secs(), AgeStyle::Long));
        let message = craft_i18n::fmt(&labels.message, &[("name", self.document.name.as_str()), ("age", age.as_str())]);
        let blocked = self.document.blocked.clone();
        let (discard, recover) = (labels.discard.clone(), labels.recover.clone());
        let footer = Footer::custom(move |ui, fx| {
            let buttons =
                vec![FooterButton::new(ButtonRole::Alternate, discard), FooterButton::new(ButtonRole::Default, recover).enabled(blocked.is_none())];
            let row = fx.buttons(ui, buttons);
            if let (Some(reason), Some(response)) = (&blocked, row.responses.get(1)) {
                response.clone().on_disabled_hover_text(reason);
            }
        });
        let shown = Dialog::with_id(self.id, labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(crate::theme::current(ctx).component.dialog.width_alert))
            .footer(footer)
            .initial_focus(InitialFocus::DefaultButton)
            .answers()
            .show(ctx, &mut self.state, &mut (), |ui, ()| {
                ui.label(message);
            });
        match shown.outcome? {
            Outcome::Answer(ButtonRole::Default) => Some(DialogResult::Ok(RecoveryAnswer::Recover)),
            Outcome::Answer(_) => Some(DialogResult::Ok(RecoveryAnswer::Discard)),
            Outcome::Cancel | Outcome::Close => Some(DialogResult::Cancel),
            _ => None,
        }
    }

    /// Recover, the default button; `None` while recovery is blocked.
    fn confirm(&mut self, _ctx: &Context) -> Option<RecoveryAnswer> {
        self.document.blocked.is_none().then_some(RecoveryAnswer::Recover)
    }
}
