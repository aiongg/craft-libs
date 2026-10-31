//! [`StandardPrompt`]: the confirmations every app asks the same way, on the
//! [`DialogHost`](super::DialogHost), with "Don't show again" where the question allows it.
//! `prompt.rs` holds the general [`Prompt`](super::Prompt) these are built on.

use std::any::Any;
use std::fmt;

use craft_settings::Remembered;
use egui::{Context, Key};

use super::buttons::ButtonRole;
use super::dialog_host::{DialogResult, StackDialog};
use super::prompt::Prompt;
use super::stack::{DialogId, DialogStack, dialog_id};
use crate::controls::Checkbox;
use crate::localized::Labels as _;

crate::words! {
    /// The standard prompts' titles, messages and buttons, English by default.
    #[non_exhaustive]
    pub struct PromptLabels {
        confirm_delete_title: "prompt.confirmDelete.title" = "Delete";
        confirm_delete_one: "prompt.confirmDelete.one" = "Delete {name}?";
        confirm_delete_many: "prompt.confirmDelete.many" = "Delete {things}? {names}";
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

/// What a [`StandardPrompt`] answers with: the choice and the [`tag`](StandardPrompt::tag) the
/// app gave the prompt, so a result says which question it answers (the document, the path, the
/// layers).
pub struct PromptAnswer {
    pub choice: PromptChoice,
    tag: Option<Box<dyn Any + Send>>,
}

impl PromptAnswer {
    /// The tag, when the prompt had one of type `T`.
    pub fn tag<T: Any>(&self) -> Option<&T> {
        self.tag.as_deref()?.downcast_ref()
    }

    /// Takes the tag out when it is a `T`; a tag of another type stays.
    pub fn take_tag<T: Any>(&mut self) -> Option<T> {
        match self.tag.take()?.downcast::<T>() {
            Ok(tag) => Some(*tag),
            Err(other) => {
                self.tag = Some(other);
                None
            }
        }
    }
}

impl fmt::Debug for PromptAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromptAnswer").field("choice", &self.choice).field("tag", &self.tag.is_some()).finish()
    }
}

/// A standard confirmation on the dialog host. The user sees a 420 pt wide question with the
/// platform's button order, the default button focused, accelerator keys on Windows and Linux
/// ([`Prompt`](super::Prompt)); with [`dont_show_again`](StandardPrompt::dont_show_again) a
/// check box under the message. Esc and the close box cancel.
///
/// ```ignore
/// let prompt = StandardPrompt::confirm_delete(["Background"], 1).noun("{n} layer", "{n} layers").tag(doc_id);
/// if dialogs.open_unless_open(prompt).is_err() { /* a prompt is already up */ }
/// match dialogs.take::<StandardPrompt>() {
///     Some(DialogResult::Ok(mut answer)) if answer.choice == PromptChoice::Proceed => delete(answer.take_tag::<DocId>()),
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
    tag: Option<Box<dyn Any + Send>>,
    noun: Option<(String, String)>,
}

impl StandardPrompt {
    pub fn new(kind: PromptKind) -> Self {
        Self { id: dialog_id(("standard-prompt", std::mem::discriminant(&kind))), kind, skip_key: None, skip: false, tag: None, noun: None }
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

    /// Carries `tag` back in the [`PromptAnswer`], typed (a document id with the layer ids:
    /// `(DocId, Vec<LayerId>)`); read it with [`PromptAnswer::tag`] or [`PromptAnswer::take_tag`].
    pub fn tag<T: Any + Send>(mut self, tag: T) -> Self {
        self.tag = Some(Box::new(tag));
        self
    }

    /// The counted noun of the delete question, as the `one` and `other` forms with `{n}`
    /// (`"{n} layer"`, `"{n} layers"`); plural rules of the language choose the form. The
    /// default is "{n} item" and "{n} items".
    pub fn noun(mut self, one: impl Into<String>, other: impl Into<String>) -> Self {
        self.noun = Some((one.into(), other.into()));
        self
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
        craft_host::Host::try_of_ctx(ctx).is_some_and(|host| Self::remembered(key).recall(&host) == Some(true))
    }

    fn remembered(key: &str) -> Remembered<bool> {
        Remembered::new(format!("prompt.{key}.skip"))
    }

    /// The title, message and answers of the kind: `(role, label, key)`.
    fn content(&self, ctx: &Context, labels: &PromptLabels) -> (String, String, Vec<(ButtonRole, String, Key)>) {
        let cancel = (ButtonRole::Cancel, "Cancel".to_owned(), Key::C);
        match &self.kind {
            PromptKind::ConfirmDelete { names, count } => {
                let message = if *count <= 1 && names.len() == 1 {
                    craft_i18n::fmt(&labels.confirm_delete_one, &[("name", &names[0])])
                } else {
                    let mut listed = names.join(", ");
                    if *count > names.len() {
                        listed.push('…');
                    }
                    let (one, other) = self.noun.as_ref().map_or(("{n} item", "{n} items"), |(one, other)| (one.as_str(), other.as_str()));
                    let things = counted(ctx, *count, one, other);
                    craft_i18n::fmt(&labels.confirm_delete_many, &[("things", &things), ("names", &listed)])
                };
                (labels.confirm_delete_title.clone(), message, vec![(ButtonRole::Default, labels.delete.clone(), Key::D), cancel])
            }
            PromptKind::DamagedFile { name } => (
                labels.damaged_title.clone(),
                craft_i18n::fmt(&labels.damaged_message, &[("name", name)]),
                vec![(ButtonRole::Default, labels.open_anyway.clone(), Key::O), cancel],
            ),
            PromptKind::ChangedOnDisk { name } => (
                labels.changed_on_disk_title.clone(),
                format!("{}\n\n{}", craft_i18n::fmt(&labels.changed_on_disk_message, &[("name", name)]), labels.changed_on_disk_detail),
                vec![(ButtonRole::Default, labels.replace.clone(), Key::R), cancel],
            ),
            PromptKind::ReplaceFile { name, answered } => {
                let mut message = craft_i18n::fmt(&labels.replace_file_message, &[("name", name)]);
                if let Some(answered) = answered {
                    message = format!("{message}\n\n{}", craft_i18n::fmt(&labels.replace_file_detail, &[("answered", answered)]));
                }
                (labels.replace_file_title.clone(), message, vec![(ButtonRole::Default, labels.replace.clone(), Key::R), cancel])
            }
            PromptKind::TrashMask => (
                labels.trash_mask_title.clone(),
                labels.trash_mask_message.clone(),
                vec![(ButtonRole::Default, labels.apply.clone(), Key::A), (ButtonRole::Alternate, labels.discard.clone(), Key::D), cancel],
            ),
        }
    }

    pub fn kind(&self) -> &PromptKind {
        &self.kind
    }
}

impl StackDialog for StandardPrompt {
    type Result = PromptAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: the prompt files named on the constructors; built on `Prompt::new(..).answer(..)` with
    // the labels from `PromptLabels::of_ctx`, the check box drawn in the message; on Proceed with
    // the box ticked, the skip is stored.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<PromptAnswer>> {
        let labels = PromptLabels::of_ctx(ctx);
        let cancel = super::DialogLabels::of_ctx(ctx).cancel;
        let (title, message, answers) = self.content(ctx, &labels);
        let mut prompt = Prompt::new(("standard-prompt", std::mem::discriminant(&self.kind)), title, message).stack(stack);
        for (role, label, key) in answers {
            let label = if role == ButtonRole::Cancel { cancel.clone() } else { label };
            prompt = prompt.answer(role, label, Some(key));
        }
        if self.skip_key.is_some() {
            let skip = &mut self.skip;
            let label = labels.dont_show_again.clone();
            prompt = prompt.below(move |ui| {
                ui.add_space(ui.spacing().item_spacing.y);
                let _ = Checkbox::bool(label, skip).show(ui);
            });
        }
        let role = prompt.show(ctx)?;
        let choice = match role {
            ButtonRole::Default => PromptChoice::Proceed,
            ButtonRole::Alternate => PromptChoice::Alternate,
            _ => PromptChoice::Cancel,
        };
        if let (PromptChoice::Proceed, true, Some(key), Some(host)) = (choice, self.skip, &self.skip_key, craft_host::Host::try_of_ctx(ctx)) {
            Self::remembered(key).store(&host, &true);
        }
        Some(DialogResult::Ok(PromptAnswer { choice, tag: self.tag.take() }))
    }
}

/// `n` with its noun in the Host's language: plural rules and the catalog's `@plural` row, else
/// the English forms.
fn counted(ctx: &Context, n: usize, one: &str, other: &str) -> String {
    let count = u64::try_from(n).unwrap_or(u64::MAX);
    if let Some(host) = craft_host::Host::try_of_ctx(ctx)
        && let Some(i18n) = host.i18n()
    {
        return i18n.trn(host.lang(), count, one, other);
    }
    craft_i18n::fmt(if n == 1 { one } else { other }, &[("n", &n.to_string())])
}
