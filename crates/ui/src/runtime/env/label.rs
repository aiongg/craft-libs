//! `Label` and `Word`: the text a component shows, beside the words it may come from.

use std::borrow::Cow;
use std::sync::Arc;

/// A word from a `words!` catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Word(pub &'static str);

impl Word {
    /// The word `text`, as a catalogue holds it.
    pub const fn new(text: &'static str) -> Self {
        Word(text)
    }
}

/// A label: a literal stays borrowed, a built string is shared. Cloning is cheap.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Label {
    /// No text.
    #[default]
    Empty,
    /// A literal.
    Static(&'static str),
    /// A built string.
    Shared(Arc<str>),
    /// A word from a catalogue.
    Word(Word),
}

impl Label {
    /// The text.
    pub fn as_str(&self) -> &str {
        match self {
            Label::Empty => "",
            Label::Static(text) => text,
            Label::Shared(text) => text,
            Label::Word(word) => word.0,
        }
    }
}

impl From<&'static str> for Label {
    fn from(text: &'static str) -> Self {
        Label::Static(text)
    }
}

impl From<String> for Label {
    fn from(text: String) -> Self {
        Label::Shared(text.into())
    }
}

impl From<Arc<str>> for Label {
    fn from(text: Arc<str>) -> Self {
        Label::Shared(text)
    }
}

impl From<Word> for Label {
    fn from(word: Word) -> Self {
        Label::Word(word)
    }
}

impl From<Label> for Cow<'static, str> {
    fn from(label: Label) -> Self {
        match label {
            Label::Empty => Cow::Borrowed(""),
            Label::Static(text) => Cow::Borrowed(text),
            Label::Shared(text) => Cow::Owned(text.to_string()),
            Label::Word(word) => Cow::Borrowed(word.0),
        }
    }
}

/// The text of a `Validity`: a label and an optional tone override. `Warn` defaults to `Tone::Warn`, `Refuse` to
/// `Tone::Bad`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Message {
    /// The text.
    pub text: Label,
    /// The tone, when it is not the one the kind of validity implies.
    pub tone: Option<craft_theme::Tone>,
}

impl Message {
    /// A message with the default tone.
    pub fn new(text: impl Into<Label>) -> Self {
        Message { text: text.into(), tone: None }
    }

    /// The message in `tone`.
    pub fn tone(mut self, tone: craft_theme::Tone) -> Self {
        self.tone = Some(tone);
        self
    }
}
