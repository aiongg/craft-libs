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
        assert!(min <= max, "arg `{name}` has an empty range: min {min} > max {max}");
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

    /// The position of the selected option of a choice arg.
    pub fn choice_index(&self, name: &str) -> usize {
        match self.value(name) {
            ArgValue::Choice { options, index } => (*index).min(options.len().saturating_sub(1)),
            _ => self.wrong_kind(name, "choice"),
        }
    }

    /// Copies the values of `after` that differ from `before` into `self`. All three list the same
    /// args in the same order (a typed story's args before and after it has run). Args the story
    /// left alone keep their value exactly, so a value a typed field cannot hold (a fraction in an
    /// integer arg) is not rounded away.
    fn apply_changes(&mut self, before: &Args, after: Args) {
        debug_assert!(
            self.args.len() == before.args.len() && before.args.len() == after.args.len(),
            "story `{}`: typed args list other args",
            self.story
        );
        for ((arg, old), new) in self.args.iter_mut().zip(&before.args).zip(after.args) {
            if old.value != new.value {
                arg.value = new.value;
            }
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

/// A struct of typed args that converts to and from the erased [`Args`] the gallery edits.
///
/// Derive it (`#[derive(StoryArgs)]`): each field is one arg, in field order. The field type picks
/// the kind: `bool` a checkbox, `String` a text field, a number type (`f64`, `f32`, integers)
/// a number limited by the required `#[arg(range = MIN..=MAX)]`, and any other type a choice
/// (derive [`StoryChoice`] on it). `#[arg(name = "...")]` sets an arg's name where it is not the
/// field name. Integer fields read the rounded value. The type is recognised by its
/// single-segment name only (`bool`, `String`, number types); paths and aliases count as choices.
///
/// A number arg without a range does not compile:
///
/// ```compile_fail
/// use craft_ui::story::StoryArgs;
///
/// #[derive(StoryArgs)]
/// struct Missing {
///     width: f64,
/// }
/// ```
/// Two args with one name, and a `range` on a bool, do not compile either:
///
/// ```compile_fail
/// use craft_ui::story::StoryArgs;
///
/// #[derive(StoryArgs)]
/// struct Twice {
///     on: bool,
///     #[arg(name = "on")]
///     also: bool,
/// }
/// ```
///
/// ```compile_fail
/// use craft_ui::story::StoryArgs;
///
/// #[derive(StoryArgs)]
/// struct Ranged {
///     #[arg(range = 0..=1)]
///     on: bool,
/// }
/// ```
pub trait StoryArgs: Sized {
    /// The args with this value's fields as their values.
    fn to_args(&self) -> Args;

    /// The value of the args' fields. Panics like the [`Args`] getters on a name or kind the type
    /// does not have, which only happens for args not made by [`to_args`](Self::to_args).
    fn from_args(args: &Args) -> Self;
}

/// An enum whose variants are the options of a choice arg.
///
/// Derive it (`#[derive(StoryChoice)]`): an option's name is the variant name in lower case, or
/// `#[choice(name = "...")]`; options keep the variant order.
pub trait StoryChoice: Sized {
    /// The option names, in variant order.
    const OPTIONS: &'static [&'static str];

    /// The position of this variant in [`OPTIONS`](Self::OPTIONS).
    fn index(&self) -> usize;

    /// The variant at `index`; an index past the end gives the last variant.
    fn from_index(index: usize) -> Self;

    /// This variant's option name.
    fn name(&self) -> &'static str {
        Self::OPTIONS[self.index()]
    }
}

/// Runs a typed story function on erased args: reads them as `A`, runs `show`, and stores the
/// values the story changed. `_defaults` ties the story's default values to `A`; it never runs.
/// [`typed_story!`](crate::typed_story) calls this.
#[doc(hidden)]
pub fn show_typed<A: StoryArgs>(ui: &mut egui::Ui, args: &mut Args, _defaults: fn() -> A, show: impl FnOnce(&mut egui::Ui, &mut A)) {
    let mut typed = A::from_args(args);
    let before = typed.to_args();
    show(ui, &mut typed);
    let after = typed.to_args();
    args.apply_changes(&before, after);
}
