//! `StandardPrompt` (24.4): the confirmations every app asks the same way, on the dialog host, as configurations of
//! `Prompt`.

use super::host::{DialogResult, StackDialog};
use super::prompt::{Prompt, Reply, buttons};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::words::PromptLabels;
use crate::choice::Checkbox;
use crate::contract::{Bind, Props as _, Show as _};
use crate::runtime::env::{Env, Label};
use crate::runtime::windows::os_of;
use crate::sections::DialogRole;
use craft_i18n::{Count, Format};
use craft_theme::Tone;
use egui::{Context, Ui};
use std::any::Any;
use std::fmt;

/// Which question a `StandardPrompt` asks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PromptKind {
    /// "Delete layer ‘Background’?" with Delete and Cancel. `names` are up to four display names; `count` the whole
    /// number.
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
    /// The document's own file changed on disk since it was opened or saved: Replace or Cancel (Save As keeps both;
    /// the message says so).
    ChangedOnDisk {
        /// The document's file name.
        name: String,
    },
    /// A save would overwrite a file the save dialog did not ask about: Replace or Cancel. `answered` is the name
    /// the dialog answered, when it differs.
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
    /// A prompt whose message and buttons the app gives (`StandardPrompt::custom`).
    Custom,
}

/// The answer to a `StandardPrompt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PromptChoice {
    /// The default button: Delete, Open Anyway, Replace, Apply.
    Proceed,
    /// The Alternate button where the prompt has one (Trash mask's Delete).
    Alternate,
    /// Dismiss without acting.
    Cancel,
}

impl PromptChoice {
    fn role(self) -> DialogRole {
        match self {
            PromptChoice::Proceed => DialogRole::Accept,
            PromptChoice::Alternate => DialogRole::Other,
            PromptChoice::Cancel => DialogRole::Cancel,
        }
    }
}

/// What a `StandardPrompt` answers with: the choice, whether "Don't show again" was ticked with Proceed, and the
/// `tag` the app gave the prompt, so a result says which question it answers (the document, the path, the layers).
#[non_exhaustive]
pub struct PromptAnswer {
    /// The button the user pressed.
    pub choice: PromptChoice,
    /// "Don't show again" was ticked and the choice is Proceed: the app stores it and skips the prompt next time.
    pub skip: bool,
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
        f.debug_struct("PromptAnswer").field("choice", &self.choice).field("skip", &self.skip).field("tag", &self.tag.is_some()).finish()
    }
}

/// A standard confirmation on the dialog host: a prompt with the platform's button order, a letter on every answer
/// and, with `dont_show_again`, a check box under the message. Escape and the close action cancel.
///
/// The question comes from the `PromptKind` constructor; `title`, `message`, `paragraph` and `button` replace or add
/// to it. The app's own texts are labels, translated by the scope's catalog before they reach the prompt. The skip
/// choice is the app's to store: the answer says whether the box was ticked.
#[derive(crate::contract::Pattern)]
pub struct StandardPrompt {
    id: DialogId,
    kind: PromptKind,
    skip_box: bool,
    skip: bool,
    open: bool,
    /// What a programmatic OK answers.
    chosen: PromptChoice,
    tag: Option<Box<dyn Any + Send>>,
    noun: Option<Count>,
    title: Option<Label>,
    message: Option<Label>,
    paragraphs: Vec<Label>,
    buttons: Vec<(PromptChoice, Label, Option<char>)>,
    details: Option<Label>,
    tone: Option<(Tone, Label)>,
    warnings: Vec<Label>,
    can_apply: bool,
    /// Set by `instance`.
    instance: Option<DialogId>,
}

/// A prompt's title, message and answers.
type Content = (Label, Label, Vec<(PromptChoice, Reply)>);

impl StandardPrompt {
    /// A prompt asking `kind`, with no tag and no "Don't show again" box.
    pub fn new(kind: PromptKind) -> Self {
        Self {
            id: dialog_id(("standard-prompt", std::mem::discriminant(&kind))),
            kind,
            skip_box: false,
            skip: false,
            open: true,
            chosen: PromptChoice::Proceed,
            tag: None,
            noun: None,
            title: None,
            message: None,
            paragraphs: Vec::new(),
            buttons: Vec::new(),
            details: None,
            tone: None,
            warnings: Vec::new(),
            can_apply: true,
            instance: None,
        }
    }

    /// Makes this prompt one of several of its kind, for a library feature that asks its own question beside the
    /// app's prompts. Its result is kept apart: only `DialogHost::take_by_id` with the id `open` returns reaches it,
    /// so the app's `take::<StandardPrompt>()` never takes it, and opening another prompt doesn't drop it.
    #[must_use]
    pub fn instance(mut self, instance: impl std::hash::Hash + fmt::Debug) -> Self {
        self.id = dialog_id(("standard-prompt", std::mem::discriminant(&self.kind), instance));
        self.instance = Some(self.id);
        self
    }

    /// The answer a host-side `confirm` gives, as pressing that button would; Proceed (the default button) until
    /// set.
    pub fn set_choice(&mut self, choice: PromptChoice) {
        self.chosen = choice;
    }

    /// Sets the "Don't show again" box; it is reported with a Proceed answer, as a click would.
    pub fn set_skip(&mut self, skip: bool) {
        self.skip = skip;
    }

    /// The delete confirmation for `count` items; `names` are the first few display names.
    pub fn confirm_delete(names: impl IntoIterator<Item = impl Into<String>>, count: usize) -> Self {
        Self::new(PromptKind::ConfirmDelete { names: names.into_iter().map(Into::into).collect(), count })
    }

    /// The damaged-file question for the file `name`.
    pub fn damaged_file(name: impl Into<String>) -> Self {
        Self::new(PromptKind::DamagedFile { name: name.into() })
    }

    /// The changed-on-disk question for the file `name`.
    pub fn changed_on_disk(name: impl Into<String>) -> Self {
        Self::new(PromptKind::ChangedOnDisk { name: name.into() })
    }

    /// The replace-file question for `name`; `answered` is the name the save dialog answered, when it differs.
    pub fn replace_file(name: impl Into<String>, answered: Option<String>) -> Self {
        Self::new(PromptKind::ReplaceFile { name: name.into(), answered })
    }

    /// The layer-mask trash question.
    pub fn trash_mask() -> Self {
        Self::new(PromptKind::TrashMask)
    }

    /// Trashing a vector mask: "Delete the vector mask?" with OK (answering `PromptChoice::Proceed`) and Cancel.
    pub fn trash_vector_mask() -> Self {
        Self::new(PromptKind::TrashVectorMask)
    }

    /// A prompt with the app's own wording and buttons: rasterize a layer, or a GPU warning. Add the buttons with
    /// `button`; the choice of the pressed one is in the `PromptAnswer`. The title is the library's "Confirm" until
    /// `title` sets it.
    pub fn custom(message: impl Into<Label>) -> Self {
        Self::new(PromptKind::Custom).message(message)
    }

    /// Replaces the title of any kind.
    #[must_use]
    pub fn title(mut self, title: impl Into<Label>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replaces the message of any kind.
    #[must_use]
    pub fn message(mut self, message: impl Into<Label>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds a paragraph after the message, a label of its own.
    #[must_use]
    pub fn paragraph(mut self, paragraph: impl Into<Label>) -> Self {
        self.paragraphs.push(paragraph.into());
        self
    }

    /// Adds a button with the app's label and its letter. The first call replaces the kind's own buttons.
    /// `PromptChoice::Proceed` is the default button (Enter), `PromptChoice::Cancel` answers Escape and the close
    /// action, and `PromptChoice::Alternate` is a third answer; each is listed once.
    #[must_use]
    pub fn button(mut self, choice: PromptChoice, label: impl Into<Label>, letter: Option<char>) -> Self {
        debug_assert!(self.buttons.iter().all(|(listed, ..)| *listed != choice), "a prompt answers each choice once");
        self.buttons.push((choice, label.into(), letter));
        self
    }

    /// A collapsible "Details" section under the message.
    #[must_use]
    pub fn details(mut self, text: impl Into<Label>) -> Self {
        self.details = Some(text.into());
        self
    }

    /// A line under the message in the colour of `tone`.
    #[must_use]
    pub fn tone(mut self, tone: Tone, text: impl Into<Label>) -> Self {
        self.tone = Some((tone, text.into()));
        self
    }

    /// Warnings under the message, one text each (a damaged file's problems).
    #[must_use]
    pub fn warnings(mut self, warnings: impl IntoIterator<Item = Label>) -> Self {
        self.warnings = warnings.into_iter().collect();
        self
    }

    /// Whether the host can apply the mask (`PromptKind::TrashMask`); with `false` the Apply button is disabled and
    /// takes no click, letter or Enter. Other kinds ignore it.
    #[must_use]
    pub fn can_apply(mut self, can_apply: bool) -> Self {
        self.can_apply = can_apply;
        self
    }

    /// Carries `tag` back in the `PromptAnswer`, typed (a document id with the layer ids); read it with
    /// `PromptAnswer::tag` or `PromptAnswer::take_tag`.
    #[must_use]
    pub fn tag<T: Any + Send>(mut self, tag: T) -> Self {
        self.tag = Some(Box::new(tag));
        self
    }

    /// The counted noun of the delete question, as a count word with `{n}` (one and other forms); the default is
    /// "{n} item" and "{n} items".
    #[must_use]
    pub fn noun(mut self, noun: Count) -> Self {
        self.noun = Some(noun);
        self
    }

    /// Adds the "Don't show again" box.
    #[must_use]
    pub fn dont_show_again(mut self) -> Self {
        self.skip_box = true;
        self
    }

    /// The question this prompt asks.
    pub fn kind(&self) -> &PromptKind {
        &self.kind
    }

    /// The title, message and answers of the kind, with the app's replacements applied.
    fn content(&self, words: &PromptLabels, format: &Format) -> Content {
        let (mut title, mut message, mut answers) = self.kind_content(words, format);
        if let Some(own) = &self.title {
            title = own.clone();
        }
        if let Some(own) = &self.message {
            message = own.clone();
        }
        if !self.buttons.is_empty() {
            answers = self
                .buttons
                .iter()
                .map(|(choice, label, letter)| (*choice, Reply::new(label.clone(), choice.role(), *letter)))
                .collect();
        }
        if title.as_str().is_empty() {
            title = words.custom_title.clone();
        }
        (title, message, answers)
    }

    fn kind_content(&self, words: &PromptLabels, format: &Format) -> Content {
        let cancel = (PromptChoice::Cancel, Reply::new(words.cancel.clone(), DialogRole::Cancel, Some('C')));
        let proceed = |label: &Label, role: DialogRole, letter: char| (PromptChoice::Proceed, Reply::new(label.clone(), role, Some(letter)));
        match &self.kind {
            PromptKind::ConfirmDelete { names, count } => {
                let message = if *count <= 1 && names.len() == 1 {
                    words.confirm_delete_one.fill(&[("name", names[0].as_str())])
                } else {
                    let mut listed = names.join(", ");
                    if *count > names.len() {
                        listed.push('…');
                    }
                    let n = u64::try_from(*count).unwrap_or(u64::MAX);
                    let noun = self.noun.clone().unwrap_or_else(|| words.items.clone());
                    let things = noun.text(n, &format.integer(n));
                    words.confirm_delete_many.fill(&[("things", things.as_str()), ("names", listed.as_str())])
                };
                (words.confirm_delete_title.clone(), message.into(), vec![proceed(&words.delete, DialogRole::Destructive, 'D'), cancel])
            }
            PromptKind::DamagedFile { name } => (
                words.damaged_title.clone(),
                words.damaged_message.fill(&[("name", name.as_str())]).into(),
                vec![proceed(&words.open_anyway, DialogRole::Accept, 'O'), cancel],
            ),
            PromptKind::ChangedOnDisk { name } => {
                let message = format!("{}\n\n{}", words.changed_on_disk_message.fill(&[("name", name.as_str())]), words.changed_on_disk_detail.as_str());
                (
                    words.changed_on_disk_title.clone(),
                    message.into(),
                    vec![proceed(&words.replace, DialogRole::Destructive, 'R'), cancel],
                )
            }
            PromptKind::ReplaceFile { name, answered } => {
                let mut message = words.replace_file_message.fill(&[("name", name.as_str())]);
                if let Some(answered) = answered {
                    message = format!("{message}\n\n{}", words.replace_file_detail.fill(&[("answered", answered.as_str())]));
                }
                (words.replace_file_title.clone(), message.into(), vec![proceed(&words.replace, DialogRole::Destructive, 'R'), cancel])
            }
            PromptKind::Custom => (Label::Empty, Label::Empty, Vec::new()),
            PromptKind::TrashVectorMask => (
                words.trash_vector_mask_title.clone(),
                words.trash_vector_mask_message.clone(),
                vec![proceed(&words.ok, DialogRole::Accept, 'O'), cancel],
            ),
            PromptKind::TrashMask => (
                words.trash_mask_title.clone(),
                words.trash_mask_message.clone(),
                vec![
                    (PromptChoice::Proceed, Reply::new(words.apply.clone(), DialogRole::Accept, Some('A')).enabled(self.can_apply)),
                    (PromptChoice::Alternate, Reply::new(words.discard.clone(), DialogRole::Other, Some('D'))),
                    cancel,
                ],
            ),
        }
    }

    /// The answer of `choice`; Proceed reports the skip box.
    fn answer(&mut self, choice: PromptChoice) -> PromptAnswer {
        PromptAnswer { choice, skip: choice == PromptChoice::Proceed && self.skip_box && self.skip, tag: self.tag.take() }
    }
}

impl StackDialog for StandardPrompt {
    type Result = PromptAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    fn instance(&self) -> Option<DialogId> {
        self.instance
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<PromptAnswer>> {
        let env = Env::of(ui);
        let words = env.words::<PromptLabels>();
        let (title, message, answers) = self.content(&words, env.format());
        let choices: Vec<PromptChoice> = answers.iter().map(|(choice, _)| *choice).collect();
        let replies: Vec<Reply> = answers.into_iter().map(|(_, reply)| reply).collect();
        let enter = replies.iter().all(|reply| reply.enabled || reply.role != DialogRole::Accept);
        let mut prompt = Prompt::new(title, message)
            .answers(buttons(&replies, os_of(ui.ctx())))
            .enter(enter)
            .opened(Bind::bound(&mut self.open));
        for paragraph in &self.paragraphs {
            prompt = prompt.paragraph(paragraph.clone());
        }
        if let Some((tone, text)) = &self.tone {
            prompt = prompt.tone(*tone, text.clone());
        }
        if !self.warnings.is_empty() {
            prompt = prompt.lines(self.warnings.iter().cloned());
        }
        if let Some(text) = &self.details {
            prompt = prompt.details(words.details.clone(), text.clone());
        }
        if self.skip_box {
            let (skip, label) = (&mut self.skip, words.dont_show_again.clone());
            prompt = prompt.extras(move |ui| {
                Checkbox::new(Bind::bound(skip)).label(label).show(ui);
            });
        }
        let picked = prompt.show(ui).picked?;
        Some(DialogResult::Ok(self.answer(choices[picked])))
    }

    /// The answer set with `set_choice`: the default button's, Proceed, unless set.
    fn confirm(&mut self, _: &Context) -> Option<PromptAnswer> {
        Some(self.answer(self.chosen))
    }
}
