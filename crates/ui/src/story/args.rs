//! [`Args`]: the values the gallery's args panel edits and a story reads.

// A misspelt arg name is a story bug that should show at once, with the story's id.
#![allow(clippy::panic, reason = "typed getters panic on an unknown name or a wrong kind")]

/// The value of one arg.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgValue {
    Bool(bool),
    /// One of `options`, by position.
    Choice {
        options: &'static [&'static str],
        index: usize,
    },
    Text(String),
    Number {
        value: f64,
        min: f64,
        max: f64,
    },
}

/// A named arg.
#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: &'static str,
    pub value: ArgValue,
}

/// A story's args, in the order the panel lists them.
///
/// The getters panic with the story id and the arg name when the name is unknown or has another
/// kind, so a typo fails in the gallery and in tests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Args {
    story: &'static str,
    args: Vec<Arg>,
}

impl Args {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a checkbox arg.
    pub fn with_bool(mut self, name: &'static str, value: bool) -> Self {
        self.args.push(Arg { name, value: ArgValue::Bool(value) });
        self
    }

    /// Adds a choice arg with `options[index]` selected.
    pub fn with_choice(mut self, name: &'static str, options: &'static [&'static str], index: usize) -> Self {
        self.args.push(Arg { name, value: ArgValue::Choice { options, index: index.min(options.len().saturating_sub(1)) } });
        self
    }

    /// Adds a text arg.
    pub fn with_text(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.args.push(Arg { name, value: ArgValue::Text(value.into()) });
        self
    }

    /// Adds a number arg limited to `min..=max`.
    pub fn with_number(mut self, name: &'static str, value: f64, min: f64, max: f64) -> Self {
        self.args.push(Arg { name, value: ArgValue::Number { value: value.clamp(min, max), min, max } });
        self
    }

    pub(super) fn for_story(mut self, story: &'static str) -> Self {
        self.story = story;
        self
    }

    /// The args in panel order.
    pub fn iter(&self) -> impl Iterator<Item = &Arg> {
        self.args.iter()
    }

    /// The args in panel order, for the panel to edit.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Arg> {
        self.args.iter_mut()
    }

    pub fn get(&self, name: &str) -> Option<&Arg> {
        self.args.iter().find(|a| a.name == name)
    }

    fn value(&self, name: &str) -> &ArgValue {
        match self.get(name) {
            Some(arg) => &arg.value,
            None => panic!("story `{}` has no arg `{name}`", self.story),
        }
    }

    fn wrong_kind(&self, name: &str, wanted: &str) -> ! {
        panic!("arg `{name}` of story `{}` is not a {wanted}", self.story)
    }

    pub fn bool(&self, name: &str) -> bool {
        match self.value(name) {
            ArgValue::Bool(b) => *b,
            _ => self.wrong_kind(name, "bool"),
        }
    }

    /// Sets a bool arg, as a story does when its own control (a dialog's close box) answers it.
    pub fn set_bool(&mut self, name: &str, value: bool) {
        let Some(Arg { value: ArgValue::Bool(b), .. }) = self.args.iter_mut().find(|a| a.name == name) else { self.wrong_kind(name, "bool") };
        *b = value;
    }

    /// The selected option.
    pub fn choice(&self, name: &str) -> &'static str {
        match self.value(name) {
            ArgValue::Choice { options, index } => options.get(*index).copied().unwrap_or(""),
            _ => self.wrong_kind(name, "choice"),
        }
    }

    /// Selects the option `option` of a choice arg; an unknown option leaves it unchanged.
    pub fn set_choice(&mut self, name: &str, option: &str) {
        let Some(Arg { value: ArgValue::Choice { options, index }, .. }) = self.args.iter_mut().find(|a| a.name == name) else {
            self.wrong_kind(name, "choice")
        };
        if let Some(found) = options.iter().position(|o| *o == option) {
            *index = found;
        }
    }

    pub fn text(&self, name: &str) -> &str {
        match self.value(name) {
            ArgValue::Text(t) => t,
            _ => self.wrong_kind(name, "text"),
        }
    }

    pub fn number(&self, name: &str) -> f64 {
        match self.value(name) {
            ArgValue::Number { value, .. } => *value,
            _ => self.wrong_kind(name, "number"),
        }
    }
}
