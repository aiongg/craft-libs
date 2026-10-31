//! [`StandardPrompt`]: the confirmations every app asks the same way, on the
//! [`DialogHost`](super::DialogHost), with "Don't show again" where the question allows it.
//! `prompt.rs` holds the general [`Prompt`](super::Prompt) these are built on.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::stack::{DialogId, DialogStack, dialog_id};

crate::words! {
    /// The standard prompts' titles, messages and buttons, English by default.
    #[non_exhaustive]
    pub struct PromptLabels {
        confirm_delete_title: "prompt.confirmDelete.title" = "Delete";
        confirm_delete_one: "prompt.confirmDelete.one" = "Delete {name}?";
        confirm_delete_many: "prompt.confirmDelete.many" = "Delete {count} items? {names}";
        delete: "prompt.delete" = "Delete";
        damaged_title: "prompt.damaged.title" = "Damaged file";
        damaged_message: "prompt.damaged.message" = "“{name}” has damaged data. Open it anyway?";
        open_anyway: "prompt.openAnyway" = "Open Anyway";
        changed_on_disk_title: "prompt.changedOnDisk.title" = "File changed";
        changed_on_disk_message: "prompt.changedOnDisk.message" = "“{name}” was changed by another app after it was opened or saved here.";
        changed_on_disk_detail: "prompt.changedOnDisk.detail" = "Replace it with this document, or cancel and use Save As to keep both.";
        replace: "prompt.replace" = "Replace";
        replace_file_title: "prompt.replaceFile.title" = "Replace file";
        replace_file_message: "prompt.replaceFile.message" = "“{name}” already exists. Replace it?";
        replace_file_detail: "prompt.replaceFile.detail" = "The name in the save dialog was “{answered}”.";
        trash_mask_title: "prompt.trashMask.title" = "Delete mask";
        trash_mask_message: "prompt.trashMask.message" = "Apply the mask to the layer before removing it?";
        apply: "prompt.apply" = "Apply";
        discard: "prompt.discard" = "Delete";
        dont_show_again: "prompt.dontShowAgain" = "Don't show again";
    }
}

/// Which question a [`StandardPrompt`] asks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PromptKind {
    /// "Delete layer ‘Background’?" with Delete and Cancel. `names` are up to four display
    /// names; `count` the whole number.
    ConfirmDelete { names: Vec<String>, count: usize },
    /// A file with damaged data: Open Anyway or Cancel.
    DamagedFile { name: String },
    /// The document's own file changed on disk since it was opened or saved: Replace or Cancel
    /// (Save As keeps both; the message says so).
    ChangedOnDisk { name: String },
    /// A save would overwrite a file the save dialog did not ask about: Replace or Cancel.
    /// `answered` is the name the dialog answered, when it differs.
    ReplaceFile { name: String, answered: Option<String> },
    /// Trashing a layer mask: Apply, Delete (the Alternate) or Cancel.
    TrashMask,
}

/// The answer to a [`StandardPrompt`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PromptChoice {
    /// The default button: Delete, Open Anyway, Replace, Apply.
    Proceed,
    /// The Alternate button where the prompt has one (Trash mask's Delete).
    Alternate,
    Cancel,
}

/// A standard confirmation on the dialog host. The user sees a 420 pt wide question with the
/// platform's button order, the default button focused, accelerator keys on Windows and Linux
/// ([`Prompt`](super::Prompt)); with [`dont_show_again`](StandardPrompt::dont_show_again) a
/// check box under the message. Esc and the close box cancel.
///
/// ```ignore
/// dialogs.open(StandardPrompt::confirm_delete(["Background"], 1).dont_show_again("layer.delete"));
/// match dialogs.take::<StandardPrompt>() {
///     Some(DialogResult::Ok(PromptChoice::Proceed)) => delete(),
///     _ => {}
/// }
/// ```
///
/// With a `dont_show_again` key the app asks [`StandardPrompt::skipped`] first: when the user
/// ticked the box before, the prompt is not opened and the app proceeds. The choice is stored
/// through craft-settings' `Remembered<bool>` under `prompt.<key>.skip`; the app's Preferences
/// "Reset All Warning Dialogs" clears every `prompt.*` key.
pub struct StandardPrompt {
    id: DialogId,
    kind: PromptKind,
    skip_key: Option<String>,
    skip: bool,
}

impl StandardPrompt {
    pub fn new(kind: PromptKind) -> Self {
        Self { id: dialog_id(("standard-prompt", std::mem::discriminant(&kind))), kind, skip_key: None, skip: false }
    }

    // pc: crates/ui-egui/src/dialogs/delete_layer_prompt.rs:9 `intercept` (message, names, skip).
    pub fn confirm_delete(names: impl IntoIterator<Item = impl Into<String>>, count: usize) -> Self {
        Self::new(PromptKind::ConfirmDelete { names: names.into_iter().map(Into::into).collect(), count })
    }

    // pc: crates/ui-egui/src/dialogs/open_prompt.rs:185 `show_damage` and :206 `damage_body`.
    pub fn damaged_file(name: impl Into<String>) -> Self {
        Self::new(PromptKind::DamagedFile { name: name.into() })
    }

    // pc: crates/ui-egui/src/dialogs/disk_change_prompt.rs:21 `intercept` and :37 `body`.
    pub fn changed_on_disk(name: impl Into<String>) -> Self {
        Self::new(PromptKind::ChangedOnDisk { name: name.into() })
    }

    // pc: crates/ui-egui/src/dialogs/replace_prompt.rs:23 `open` and :31 `body`.
    pub fn replace_file(name: impl Into<String>, answered: Option<String>) -> Self {
        Self::new(PromptKind::ReplaceFile { name: name.into(), answered })
    }

    // pc: crates/ui-egui/src/dialogs/mask_trash_prompt.rs.
    pub fn trash_mask() -> Self {
        Self::new(PromptKind::TrashMask)
    }

    /// Adds the "Don't show again" box, remembered under `key`.
    pub fn dont_show_again(mut self, key: impl Into<String>) -> Self {
        self.skip_key = Some(key.into());
        self
    }

    /// Whether the user asked not to see the prompt under `key` again: the app proceeds without
    /// opening it.
    // pc: crates/ui-egui/src/dialogs/delete_layer_prompt.rs:13 (`prefs.dialogs[PREF].skip`).
    pub fn skipped(ctx: &Context, key: &str) -> bool {
        todo!()
    }

    pub fn kind(&self) -> &PromptKind {
        &self.kind
    }
}

impl StackDialog for StandardPrompt {
    type Result = PromptChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: the prompt files named on the constructors; built on `Prompt::new(..).answer(..)` with
    // the labels from `PromptLabels::of_ctx`, the check box drawn in the message; on Proceed with
    // the box ticked, the skip is stored.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<PromptChoice>> {
        todo!()
    }
}
