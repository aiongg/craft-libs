//! The update dialogs on craft-ui's dialog host: [`UpdatePromptDialog`] (a release is available)
//! and [`UpdateCheckDialog`] (the result of a manual check), on [`Dialog`](craft_ui::dialog::Dialog)
//! with the app's name and version from `Host::app` (craft-host) and the words from
//! [`UpdateLabels`]. [`dialog`](crate::dialog) keeps the hand-drawn cards until the apps have moved.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog, dialog_id};
use egui::Context;

use crate::release::Release;
use crate::updater::Status;

craft_ui::words! {
    /// The update dialogs' words, English by default.
    #[non_exhaustive]
    pub struct UpdateLabels {
        available_title: "update.available.title" = "Update available";
        available_message: "update.available.message" = "{app} {version} is available. You have {running}.";
        update_and_restart: "update.updateAndRestart" = "Update and Restart";
        download: "update.download" = "Download";
        later: "update.later" = "Later";
        installing: "update.installing" = "Installing…";
        check_title: "update.check.title" = "Check for Updates";
        checking: "update.checking" = "Checking for updates…";
        up_to_date: "update.upToDate" = "{app} {running} is the newest version.";
        failed: "update.failed" = "The update check failed.";
        check_again: "update.checkAgain" = "Check Again";
    }
}

/// What the user chose in the update prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UpdateChoice {
    /// Update and Restart: the default where the app can update itself.
    UpdateAndRestart,
    /// Open the release page in the browser (the default elsewhere).
    Download,
    /// Later: close the prompt for this run.
    Later,
}

/// "Update available", on the startup queue at [`StartupStage::Update`](craft_ui::dialog::StartupStage::Update)
/// so it never overlaps the GPU warning or recovery (gap 75), or opened by a manual check that
/// found a release. The user sees the app's icon and name, the new and running versions, the
/// release notes link, and Update and Restart / Download / Later in the platform order; while an
/// install runs the buttons are disabled and a status line shows. Enter takes the default, Esc is
/// Later. Result: [`UpdateChoice`].
pub struct UpdatePromptDialog {
    id: DialogId,
    release: Release,
    running_version: String,
    self_updating: bool,
    status: Option<String>,
    installing: bool,
}

impl UpdatePromptDialog {
    /// A prompt for `release`; `self_updating` offers Update and Restart as the default.
    pub fn new(release: Release, running_version: impl Into<String>, self_updating: bool) -> Self {
        Self { id: dialog_id("update-prompt"), release, running_version: running_version.into(), self_updating, status: None, installing: false }
    }

    /// The install's progress line while [`installing`](UpdatePromptDialog::set_installing); the
    /// app sets it through `DialogHost::get_mut`.
    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }

    /// Disables the buttons while the installer runs.
    pub fn set_installing(&mut self, installing: bool) {
        self.installing = installing;
    }

    pub fn release(&self) -> &Release {
        &self.release
    }
}

impl StackDialog for UpdatePromptDialog {
    type Result = UpdateChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/update/src/dialog.rs:42 `StartupPrompt::show` (the card's content and buttons),
    // on `Dialog::new(..).stack(stack).width(Width::Standard).footer(Footer::standard()..)`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<UpdateChoice>> {
        todo!()
    }
}

/// What the user chose in the check-result dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckChoice {
    Close,
    /// Failed state only: run another check.
    CheckAgain,
}

/// The result of a manual "Check for Updates": a spinner while checking, "newest version" when
/// up to date, the error and Check Again when it failed. The app opens it when the user starts a
/// check and feeds [`set_status`](UpdateCheckDialog::set_status) each pass; an available release
/// closes it and opens an [`UpdatePromptDialog`] instead. OK, Enter, Esc and the close box close
/// it. Result: [`CheckChoice`].
pub struct UpdateCheckDialog {
    id: DialogId,
    running_version: String,
    status: Status,
}

impl UpdateCheckDialog {
    pub fn new(running_version: impl Into<String>) -> Self {
        Self { id: dialog_id("update-check"), running_version: running_version.into(), status: Status::Checking }
    }

    /// The updater's status this pass.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

impl StackDialog for UpdateCheckDialog {
    type Result = CheckChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/update/src/dialog.rs:323 `CheckResultDialog::show`, on `Dialog` with a
    // `Spinner` while checking and `Footer::standard().without_cancel()`; Check Again as the
    // Alternate in the failed state.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<CheckChoice>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    // Enter takes Update and Restart when self-updating and Download otherwise; Esc is Later.
    #[test]
    #[ignore = "skeleton"]
    fn the_default_button_depends_on_self_updating_and_esc_is_later() {
        todo!()
    }

    // Enqueued at the Update stage behind a Warning alert, the prompt opens only after the alert
    // closes (gap 75).
    #[test]
    #[ignore = "skeleton"]
    fn the_prompt_waits_for_earlier_startup_modals() {
        todo!()
    }

    // The check dialog shows the spinner, then the up-to-date or failed text as the status
    // changes, and Check Again only when failed.
    #[test]
    #[ignore = "skeleton"]
    fn the_check_dialog_follows_the_status() {
        todo!()
    }
}
