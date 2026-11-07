//! `Prompt` (24.4): a question with answers on the modal preset of `Dialog`, and `accelerator_labels`.

use super::frame::DialogOutput;
use super::modal::Dialog;
use crate::contract::{Bind, NoValue, Outcome, Show as _, ShowContainer as _};
use crate::display::Text;
use crate::layout_components::Stack;
use crate::notices::{Notice, NoticeLayout};
use crate::runtime::env::Label;
use crate::runtime::focus::Initial;
use crate::runtime::overlay::CloseReason;
use crate::sections::{DialogRole, FooterButton, Section};
use craft_host::target::Os;
use craft_theme::Tone;
use egui::{Response, Ui};

#[cfg(feature = "testing")]
mod stories;

/// The labels of one prompt's buttons with their letters in the same style: `(Y)es` when every label starts with
/// its letter (case-insensitive), otherwise the letter is appended to every label (`Ja (Y)`, `Nein (N)`), so one
/// prompt never mixes the two. An answer without a letter keeps its text. macOS shows plain labels: the letters
/// still work.
pub fn accelerator_labels(answers: &[(&str, Option<char>)], os: Os) -> Vec<String> {
    let starts_with = |label: &str, letter: char| label.chars().next().is_some_and(|first| first.to_uppercase().eq(letter.to_uppercase()));
    let inline = answers.iter().all(|(label, letter)| letter.is_none_or(|letter| starts_with(label, letter)));
    answers
        .iter()
        .map(|(label, letter)| match letter {
            Some(letter) if os != Os::Mac => match label.chars().next() {
                Some(first) if inline => format!("({first}){}", &label[first.len_utf8()..]),
                _ => format!("{label} ({})", letter.to_uppercase()),
            },
            _ => (*label).to_owned(),
        })
        .collect()
}

/// One answer of a prompt before its button exists: the label, what the button means, its letter and whether it is
/// available.
pub(super) struct Reply {
    pub(super) label: Label,
    pub(super) role: DialogRole,
    pub(super) letter: Option<char>,
    pub(super) enabled: bool,
}

impl Reply {
    pub(super) fn new(label: impl Into<Label>, role: DialogRole, letter: Option<char>) -> Self {
        Reply { label: label.into(), role, letter, enabled: true }
    }

    pub(super) fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
}

/// The buttons of `replies`, in the given order, with their letters shown in the labels by `accelerator_labels`.
pub(super) fn buttons(replies: &[Reply], os: Os) -> Vec<FooterButton> {
    let shown: Vec<(&str, Option<char>)> = replies.iter().map(|reply| (reply.label.as_str(), reply.letter)).collect();
    let labels = accelerator_labels(&shown, os);
    replies
        .iter()
        .zip(labels)
        .map(|(reply, label)| {
            let button = FooterButton::new(label, reply.role).enabled(reply.enabled);
            reply.letter.map_or(button, |letter| button.letter(letter))
        })
        .collect()
}

/// What a prompt reports: its response, how it closed in this pass and the answer given.
#[derive(Debug)]
#[non_exhaustive]
pub struct PromptOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the prompt closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The index, in the order given to `answers`, of the answer given in this pass: the button clicked or lettered,
    /// the Accept answer for Enter, the escape answer for Escape and the close action.
    pub picked: Option<usize>,
}

impl Outcome for PromptOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The index Escape and the close action answer with: the answer of `chosen` when there is one, else Cancel, else
/// the first Other answer, else the default answer.
fn escape_answer(roles: &[DialogRole], chosen: Option<DialogRole>) -> Option<usize> {
    let find = |wanted: DialogRole| roles.iter().position(|role| *role == wanted);
    chosen.and_then(find).or_else(|| find(DialogRole::Cancel)).or_else(|| find(DialogRole::Other)).or_else(|| default_answer(roles))
}

/// The index Enter answers with: the first Accept answer, else the first Destructive one.
fn default_answer(roles: &[DialogRole]) -> Option<usize> {
    let find = |wanted: DialogRole| roles.iter().position(|role| *role == wanted);
    find(DialogRole::Accept).or_else(|| find(DialogRole::Destructive))
}

type Below<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// What a prompt shows under its message, in this order: a tone line, paragraphs, a list of lines, a collapsible
/// section, and the caller's own content.
#[derive(Default)]
pub(super) struct Extras<'a> {
    tone: Option<(Tone, Label)>,
    paragraphs: Vec<Label>,
    lines: Vec<Label>,
    details: Option<(Label, Label)>,
    below: Option<Below<'a>>,
}

impl Extras<'_> {
    fn is_empty(&self) -> bool {
        self.tone.is_none() && self.paragraphs.is_empty() && self.lines.is_empty() && self.details.is_none() && self.below.is_none()
    }

    fn show(self, ui: &mut Ui) {
        let Extras { tone, paragraphs, lines, details, below } = self;
        if let Some((tone, text)) = tone {
            Notice::new(tone, text).layout(NoticeLayout::Inline).show(ui);
        }
        for paragraph in paragraphs {
            Text::new(paragraph).show(ui);
        }
        for line in lines {
            Text::new(line).show(ui);
        }
        if let Some((label, text)) = details {
            Section::new(label).show(ui, |ui, _| {
                Text::new(text.clone()).show(ui);
            });
        }
        if let Some(below) = below {
            below(ui);
        }
    }
}

/// A question with answers: a dialog on the modal preset, centred and not movable, with a message, optional extras
/// and one footer button per answer.
///
/// - Answers: `FooterButton`s with a `DialogRole` each, placed in the platform's order. A bare letter given with
///   `FooterButton::letter` answers while the focus is inside the prompt and no text part owns keys.
/// - Escape and the close action give the escape answer (Cancel, else the first Other answer, else the default
///   answer); Enter, when no part takes it, gives the Accept answer. Both come back as `PromptOutput::picked`.
/// - The open state is the `opened` binding, else the prompt's own, which starts open.
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct Prompt<'a> {
    title: Label,
    message: Label,
    answers: Vec<FooterButton>,
    extras: Extras<'a>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// The role whose answer Escape and the close action give, instead of the default choice.
    #[prop]
    escape: Option<DialogRole>,
    /// Whether Enter gives the Accept answer when no part takes it; off while that answer is not available.
    #[prop]
    enter: bool,
}

impl<'a> Prompt<'a> {
    /// A prompt titled `title` asking `message`; add its answers with `answers`.
    pub fn new(title: impl Into<Label>, message: impl Into<Label>) -> Self {
        Prompt {
            title: title.into(),
            message: message.into(),
            answers: Vec::new(),
            extras: Extras::default(),
            open: None,
            escape: None,
            enter: true,
        }
    }

    /// The answers, one footer button each.
    #[must_use]
    pub fn answers(mut self, answers: impl IntoIterator<Item = FooterButton>) -> Self {
        self.answers = answers.into_iter().collect();
        self
    }

    /// A line under the message in the colour of `tone`, with the tone's icon.
    #[must_use]
    pub fn tone(mut self, tone: Tone, text: impl Into<Label>) -> Self {
        self.extras.tone = Some((tone, text.into()));
        self
    }

    /// A paragraph after the message, a label of its own.
    #[must_use]
    pub fn paragraph(mut self, text: impl Into<Label>) -> Self {
        self.extras.paragraphs.push(text.into());
        self
    }

    /// Lines under the message, one text each (a file's warnings).
    #[must_use]
    pub fn lines(mut self, lines: impl IntoIterator<Item = Label>) -> Self {
        self.extras.lines = lines.into_iter().collect();
        self
    }

    /// A collapsible section under the message; `label` names its header ("Details").
    #[must_use]
    pub fn details(mut self, label: impl Into<Label>, text: impl Into<Label>) -> Self {
        self.extras.details = Some((label.into(), text.into()));
        self
    }

    /// The caller's content under the message and the extras: a "Don't show again" check box.
    #[must_use]
    pub fn extras(mut self, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.extras.below = Some(Box::new(body));
        self
    }

    /// Shows the prompt.
    pub fn show(self, ui: &mut Ui) -> PromptOutput {
        let Prompt { title, message, answers, extras, open, escape, enter } = self;
        let roles: Vec<DialogRole> = answers.iter().map(FooterButton::role).collect();
        let (escape_at, enter_at) = (escape_answer(&roles, escape), default_answer(&roles).filter(|_| enter));
        let mut dialog = Dialog::prompt(title).buttons(answers).initial_focus(Initial::Default).close_on_enter(enter_at.is_some());
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }
        let mut extras = Some(extras);
        let shown = dialog.show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                Text::new(message.clone()).show(ui);
                if let Some(extras) = extras.take().filter(|extras| !extras.is_empty()) {
                    extras.show(ui);
                }
            });
        });
        let DialogOutput { response, closed, picked } = shown.output;
        let picked = picked.or(match closed {
            Some(CloseReason::Enter) => enter_at,
            Some(_) => escape_at,
            None => None,
        });
        PromptOutput { response, closed, picked }
    }
}
