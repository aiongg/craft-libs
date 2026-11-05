//! [`StandardPrompt`]: the confirmations every app asks the same way, on the
//! [`DialogHost`](super::DialogHost), with "Don't show again" where the question allows it.
//! `prompt.rs` holds the general [`Prompt`](super::Prompt) these are built on.

use std::any::Any;
use std::fmt;

use craft_settings::Remembered;
use craft_theme::Tone;
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
        trash_vector_mask_title: "prompt.trashVectorMask.title" = "Delete vector mask";
        trash_vector_mask_message: "prompt.trashVectorMask.message" = "Delete the vector mask?";
        /// The title of a [`StandardPrompt::custom`] prompt the app gave none.
        custom_title: "prompt.custom.title" = "Confirm";
        /// The header of the collapsible section of a custom prompt.
        details: "prompt.details" = "Details";
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
    ConfirmDelete {
        /// Display names of the first few items.
        names: Vec<String>,
        /// How many items the action affects.
        count: usize,
    },
    /// A file with damaged data: Open Anyway or Cancel.
    DamagedFile {
        /// The file's name.
        name: String,
    },
    /// The document's own file changed on disk since it was opened or saved: Replace or Cancel
    /// (Save As keeps both; the message says so).
    ChangedOnDisk {
        /// The document's file name.
        name: String,
    },
    /// A save would overwrite a file the save dialog did not ask about: Replace or Cancel.
    /// `answered` is the name the dialog answered, when it differs.
    ReplaceFile {
        /// The existing file's name.
        name: String,
        /// The name the save dialog answered, when it differs from `name`.
        answered: Option<String>,
    },
    /// Trashing a layer mask: Apply, Delete (the Alternate) or Cancel.
    TrashMask,
    /// Trashing a vector mask: "Delete the vector mask?" with OK and Cancel.
    TrashVectorMask,
    /// A prompt whose message and buttons the app gives ([`StandardPrompt::custom`]).
    Custom,
}

/// The answer to a [`StandardPrompt`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PromptChoice {
    /// The default button: Delete, Open Anyway, Replace, Apply.
    Proceed,
    /// The Alternate button where the prompt has one (Trash mask's Delete).
    Alternate,
    /// Dismiss without acting.
    Cancel,
}

/// What a [`StandardPrompt`] answers with: the choice and the [`tag`](StandardPrompt::tag) the
/// app gave the prompt, so a result says which question it answers (the document, the path, the
/// layers).
pub struct PromptAnswer {
    /// The button the user pressed.
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
    /// What a programmatic OK ([`StandardPrompt::set_choice`]) answers.
    chosen: PromptChoice,
    tag: Option<Box<dyn Any + Send>>,
    noun: Option<(String, String)>,
    title: Option<String>,
    message: Option<String>,
    buttons: Vec<(PromptChoice, String, Option<Key>)>,
    details: Option<String>,
    tone: Option<(Tone, String)>,
    warnings: Vec<String>,
    can_apply: bool,
}

impl StandardPrompt {
    /// A prompt asking `kind`, with no tag and no "Don't show again" box.
    pub fn new(kind: PromptKind) -> Self {
        Self {
            id: dialog_id(("standard-prompt", std::mem::discriminant(&kind))),
            kind,
            skip_key: None,
            skip: false,
            chosen: PromptChoice::Proceed,
            tag: None,
            noun: None,
            title: None,
            message: None,
            buttons: Vec::new(),
            details: None,
            tone: None,
            warnings: Vec::new(),
            can_apply: true,
        }
    }

    /// The answer a host-side [`confirm`](DialogHost::confirm) gives, as pressing that button
    /// would; Proceed (the default button) until set.
    pub fn set_choice(&mut self, choice: PromptChoice) {
        self.chosen = choice;
    }

    /// Sets the "Don't show again" box; it is stored with a Proceed answer, as a click would.
    pub fn set_skip(&mut self, skip: bool) {
        self.skip = skip;
    }

    // pc: crates/ui-egui/src/dialogs/delete_layer_prompt.rs:9 `intercept` (message, names, skip).
    /// The delete confirmation for `count` items; `names` are the first few display names.
    pub fn confirm_delete(names: impl IntoIterator<Item = impl Into<String>>, count: usize) -> Self {
        Self::new(PromptKind::ConfirmDelete { names: names.into_iter().map(Into::into).collect(), count })
    }

    // pc: crates/ui-egui/src/dialogs/open_prompt.rs:185 `show_damage` and :206 `damage_body`.
    /// The damaged-file question for the file `name`.
    pub fn damaged_file(name: impl Into<String>) -> Self {
        Self::new(PromptKind::DamagedFile { name: name.into() })
    }

    // pc: crates/ui-egui/src/dialogs/disk_change_prompt.rs:21 `intercept` and :37 `body`.
    /// The changed-on-disk question for the file `name`.
    pub fn changed_on_disk(name: impl Into<String>) -> Self {
        Self::new(PromptKind::ChangedOnDisk { name: name.into() })
    }

    // pc: crates/ui-egui/src/dialogs/replace_prompt.rs:23 `open` and :31 `body`.
    /// The replace-file question for `name`; `answered` is the name the save dialog answered, when it differs.
    pub fn replace_file(name: impl Into<String>, answered: Option<String>) -> Self {
        Self::new(PromptKind::ReplaceFile { name: name.into(), answered })
    }

    // pc: crates/ui-egui/src/dialogs/mask_trash_prompt.rs.
    /// The layer-mask trash question.
    pub fn trash_mask() -> Self {
        Self::new(PromptKind::TrashMask)
    }

    /// Trashing a vector mask: "Delete the vector mask?" with OK (the default, answering
    /// [`PromptChoice::Proceed`]) and Cancel.
    pub fn trash_vector_mask() -> Self {
        Self::new(PromptKind::TrashVectorMask)
    }

    /// A prompt with the app's own wording and buttons: rasterize a layer (Convert to Smart
    /// Object, Rasterize, Cancel), or a GPU warning (Keep Using CPU, Retry GPU). Add the buttons
    /// with [`button`](Self::button); the choice of the pressed one is in the [`PromptAnswer`].
    /// The title is the library's "Confirm" until [`title`](Self::title) sets it.
    pub fn custom(message: impl Into<String>) -> Self {
        Self::new(PromptKind::Custom).message(message)
    }

    /// Replaces the title of any kind.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replaces the message of any kind. Separate paragraphs with a blank line.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds a button with the app's translated label and its key (a plain letter, drawn as the
    /// accelerator on Windows and Linux). The first call replaces the kind's own buttons.
    /// [`PromptChoice::Proceed`] is the default button (Enter), [`PromptChoice::Cancel`] answers
    /// Esc and the close box, and [`PromptChoice::Alternate`] is a third answer; each is listed
    /// once.
    pub fn button(mut self, choice: PromptChoice, label: impl Into<String>, key: Option<Key>) -> Self {
        debug_assert!(self.buttons.iter().all(|(c, ..)| *c != choice), "craft-ui: a prompt answers each choice once");
        self.buttons.push((choice, label.into(), key));
        self
    }

    /// A collapsible "Details" section under the message, closed on open. Space and Enter on its
    /// header toggle it.
    pub fn details(mut self, text: impl Into<String>) -> Self {
        self.details = Some(text.into());
        self
    }

    /// A line under the message in the colour of `tone` (`Warn` or `Bad`, from the theme's
    /// status tokens).
    pub fn tone(mut self, tone: Tone, text: impl Into<String>) -> Self {
        self.tone = Some((tone, text.into()));
        self
    }

    /// A scrolling list of warnings under the message (a damaged file's problems).
    pub fn warnings(mut self, warnings: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.warnings = warnings.into_iter().map(Into::into).collect();
        self
    }

    /// Whether the host can apply the mask ([`PromptKind::TrashMask`]); with `false` the Apply
    /// button is disabled and takes no click, Enter or key. Other kinds ignore it.
    pub fn can_apply(mut self, can_apply: bool) -> Self {
        self.can_apply = can_apply;
        self
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
    fn content(&self, ctx: &Context, labels: &PromptLabels) -> (String, String, Vec<(ButtonRole, String, Option<Key>)>) {
        let (mut title, mut message, mut answers) = self.kind_content(ctx, labels);
        if let Some(own) = &self.title {
            title.clone_from(own);
        }
        if let Some(own) = &self.message {
            message.clone_from(own);
        }
        if !self.buttons.is_empty() {
            answers = self
                .buttons
                .iter()
                .map(|(choice, label, key)| {
                    let role = match choice {
                        PromptChoice::Proceed => ButtonRole::Default,
                        PromptChoice::Alternate => ButtonRole::Alternate,
                        PromptChoice::Cancel => ButtonRole::Cancel,
                    };
                    (role, label.clone(), *key)
                })
                .collect();
        }
        if title.is_empty() {
            title.clone_from(&labels.custom_title);
        }
        (title, message, answers)
    }

    fn kind_content(&self, ctx: &Context, labels: &PromptLabels) -> (String, String, Vec<(ButtonRole, String, Option<Key>)>) {
        let cancel = (ButtonRole::Cancel, "Cancel".to_owned(), Some(Key::C));
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
                (labels.confirm_delete_title.clone(), message, vec![(ButtonRole::Default, labels.delete.clone(), Some(Key::D)), cancel])
            }
            PromptKind::DamagedFile { name } => (
                labels.damaged_title.clone(),
                craft_i18n::fmt(&labels.damaged_message, &[("name", name)]),
                vec![(ButtonRole::Default, labels.open_anyway.clone(), Some(Key::O)), cancel],
            ),
            PromptKind::ChangedOnDisk { name } => (
                labels.changed_on_disk_title.clone(),
                format!("{}\n\n{}", craft_i18n::fmt(&labels.changed_on_disk_message, &[("name", name)]), labels.changed_on_disk_detail),
                vec![(ButtonRole::Default, labels.replace.clone(), Some(Key::R)), cancel],
            ),
            PromptKind::ReplaceFile { name, answered } => {
                let mut message = craft_i18n::fmt(&labels.replace_file_message, &[("name", name)]);
                if let Some(answered) = answered {
                    message = format!("{message}\n\n{}", craft_i18n::fmt(&labels.replace_file_detail, &[("answered", answered)]));
                }
                (labels.replace_file_title.clone(), message, vec![(ButtonRole::Default, labels.replace.clone(), Some(Key::R)), cancel])
            }
            PromptKind::Custom => (String::new(), String::new(), Vec::new()),
            PromptKind::TrashVectorMask => (
                labels.trash_vector_mask_title.clone(),
                labels.trash_vector_mask_message.clone(),
                vec![(ButtonRole::Default, super::DialogLabels::of_ctx(ctx).ok, Some(Key::O)), cancel],
            ),
            PromptKind::TrashMask => (
                labels.trash_mask_title.clone(),
                labels.trash_mask_message.clone(),
                vec![
                    (ButtonRole::Default, labels.apply.clone(), Some(Key::A)),
                    (ButtonRole::Alternate, labels.discard.clone(), Some(Key::D)),
                    cancel,
                ],
            ),
        }
    }

    /// The question this prompt asks.
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
        let own_buttons = !self.buttons.is_empty();
        for (role, label, key) in answers {
            let label = if role == ButtonRole::Cancel && !own_buttons { cancel.clone() } else { label };
            prompt = prompt.answer(role, label, key);
        }
        if self.kind == PromptKind::TrashMask && !self.can_apply {
            prompt = prompt.disable(ButtonRole::Default);
        }
        if let Some((tone, text)) = &self.tone {
            prompt = prompt.tone(*tone, text.clone());
        }
        if !self.warnings.is_empty() {
            prompt = prompt.lines(self.warnings.iter().cloned());
        }
        if let Some(text) = &self.details {
            prompt = prompt.details(labels.details.clone(), text.clone());
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
        Some(DialogResult::Ok(self.answer(ctx, role)))
    }

    /// The answer set with [`StandardPrompt::set_choice`]: the default button's, Proceed, unless set.
    fn confirm(&mut self, ctx: &Context) -> Option<PromptAnswer> {
        let role = match self.chosen {
            PromptChoice::Proceed => ButtonRole::Default,
            PromptChoice::Alternate => ButtonRole::Alternate,
            PromptChoice::Cancel => ButtonRole::Cancel,
        };
        Some(self.answer(ctx, role))
    }
}

impl StandardPrompt {
    /// The answer a press of the `role` button gives; Proceed stores the skip choice.
    fn answer(&mut self, ctx: &Context, role: ButtonRole) -> PromptAnswer {
        let choice = match role {
            ButtonRole::Default => PromptChoice::Proceed,
            ButtonRole::Alternate => PromptChoice::Alternate,
            _ => PromptChoice::Cancel,
        };
        if let (PromptChoice::Proceed, true, Some(key), Some(host)) = (choice, self.skip, &self.skip_key, craft_host::Host::try_of_ctx(ctx)) {
            Self::remembered(key).store(&host, &true);
        }
        PromptAnswer { choice, tag: self.tag.take() }
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
