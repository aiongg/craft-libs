//! The built-in strings of craft-ui's components, in the language of the app.
//!
//! A [`Word`] names one string: its id, its English text and its context. The app installs a
//! [`Words`] on the context once per language change ([`Words::from_i18n`] reads a craft-i18n
//! catalog); components and labels structs ([`Labels`]) resolve their strings through it, and show
//! English where no translator is installed or a row is missing. A labels struct is plain data: a
//! host that sets a field gets exactly that text.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use craft_i18n::{I18n, Lang};
use egui::plugin::Plugin;
use egui::{Context, Ui};

mod registry;

/// One built-in string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Word {
    /// `<module>.<camelCase>`, or `""` for a word looked up by its English text.
    pub id: &'static str,
    /// The text shown when no translation exists.
    pub english: &'static str,
    /// The catalog context of a word with no id (`key` for key caps), or `""`.
    pub context: &'static str,
}

impl Word {
    /// A word with an id; a catalog may translate it by the id or by the English text.
    pub const fn keyed(id: &'static str, english: &'static str) -> Word {
        Word { id, english, context: "" }
    }

    /// A word looked up by its English text alone.
    pub const fn plain(english: &'static str) -> Word {
        Word { id: "", english, context: "" }
    }

    /// A word whose translation is only the row of `context`, English where there is none.
    pub const fn in_context(context: &'static str, english: &'static str) -> Word {
        Word { id: "", english, context }
    }

    /// What a `translate(key)` hook of a labels struct receives: the id, or the English text of a
    /// word with no id.
    pub const fn key(&self) -> &'static str {
        if self.id.is_empty() { self.english } else { self.id }
    }
}

type Lookup = Arc<dyn Fn(Word) -> Option<String> + Send + Sync>;
type Plural = Arc<dyn Fn(&'static str, u64, &'static str, &'static str) -> Option<String> + Send + Sync>;

/// The app's lookup, installed on the context with [`Words::install`].
#[derive(Clone, Default)]
pub struct Words {
    lookup: Option<Lookup>,
    plural: Option<Plural>,
}

impl std::fmt::Debug for Words {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Words").field("translated", &self.lookup.is_some()).finish()
    }
}

impl Words {
    /// English everywhere.
    pub fn english() -> Words {
        Words::default()
    }

    /// Words translated by `lookup`, which returns `None` where it has no text.
    pub fn new(lookup: impl Fn(Word) -> Option<String> + Send + Sync + 'static) -> Words {
        Words { lookup: Some(Arc::new(lookup)), plural: None }
    }

    /// Counts ([`Words::count`]) written by `plural(id, n, one, other)`, which returns `None` to
    /// fall back to the English forms.
    #[must_use]
    pub fn with_plural(mut self, plural: impl Fn(&'static str, u64, &'static str, &'static str) -> Option<String> + Send + Sync + 'static) -> Words {
        self.plural = Some(Arc::new(plural));
        self
    }

    /// The words of `i18n` in `lang`: [`I18n::tid`] for a word with an id, [`I18n::get`] for one
    /// with a context, [`I18n::t`] for the rest, and [`I18n::trn`] for counts. `None` where the
    /// catalog has no row.
    pub fn from_i18n(i18n: Arc<I18n>, lang: Lang) -> Words {
        let words = Arc::clone(&i18n);
        let lookup = move |word: Word| -> Option<String> {
            let text = if !word.id.is_empty() {
                words.tid(lang, word.id, word.english)
            } else if !word.context.is_empty() {
                words.get(lang, word.context, word.english)?
            } else {
                words.t(lang, word.english)
            };
            (text != word.english).then(|| text.to_owned())
        };
        Words::new(lookup).with_plural(move |_, n, one, other| Some(i18n.trn(lang, n, one, other)))
    }

    /// Makes these the words [`Words::of`] returns for every widget in `ctx`; call it when the
    /// language changes. The context keeps them outside its [`egui::Memory`], so a memory reset
    /// leaves them installed.
    pub fn install(self, ctx: &Context) {
        ctx.plugin_or_default::<Installed>().lock().0 = Some(Arc::new(self));
    }

    /// The words installed in `ui`'s context ([`Words::install`]), English when none are.
    pub fn of(ui: &Ui) -> Arc<Words> {
        static ENGLISH: LazyLock<Arc<Words>> = LazyLock::new(|| Arc::new(Words::english()));
        let installed = ui.ctx().plugin_opt::<Installed>().and_then(|installed| installed.lock().0.clone());
        installed.unwrap_or_else(|| Arc::clone(&ENGLISH))
    }

    /// The text of `word`: never empty. English when no translator is installed (without
    /// allocating), when it has no text, when its text is empty, or when its text lacks a
    /// `{placeholder}` of the English (logged once per word).
    pub fn get(&self, word: Word) -> Cow<'static, str> {
        let Some(lookup) = &self.lookup else {
            return Cow::Borrowed(word.english);
        };
        match lookup(word) {
            Some(text) if text.is_empty() => Cow::Borrowed(word.english),
            Some(text) if !keeps_placeholders(word.english, &text) => {
                warn_once(word, &text);
                Cow::Borrowed(word.english)
            }
            Some(text) => Cow::Owned(text),
            None => Cow::Borrowed(word.english),
        }
    }

    /// [`Words::get`] with each `{name}` of the text filled from `args` by [`craft_i18n::fmt`],
    /// which isolates values inside right-to-left text.
    pub fn format(&self, word: Word, args: &[(&str, &str)]) -> String {
        craft_i18n::fmt(&self.get(word), args)
    }

    /// `n` things: `one` when `n` is 1 and `other` otherwise in English, the locale's plural forms
    /// when the installed translator has them. `{n}` stands for the number; `id` names the count
    /// for a custom [`Words::with_plural`].
    pub fn count(&self, id: &'static str, n: u64, one: &'static str, other: &'static str) -> String {
        if let Some(text) = self.plural.as_ref().and_then(|plural| plural(id, n, one, other)) {
            return text;
        }
        craft_i18n::fmt(if n == 1 { one } else { other }, &[("n", &n.to_string())])
    }
}

/// The words [`Words::install`] set, kept as a plugin so that a reset of the context's memory does
/// not drop them.
#[derive(Default)]
struct Installed(Option<Arc<Words>>);

impl Plugin for Installed {
    fn debug_name(&self) -> &'static str {
        "craft_ui::localized::words"
    }
}

/// A labels struct: plain data whose every field is one [`Word`] of its table.
///
/// Declared with [`words!`](crate::words), which keeps the fields, the English text, the ids and
/// the translation in one table.
pub trait Labels: Sized {
    /// The table: one word per field, in field order.
    const WORDS: &'static [Word];

    /// Every field in English; the same as `Default`.
    fn english() -> Self;

    /// Every field replaced by its word's text in `words`.
    #[must_use]
    fn localized_in(self, words: &Words) -> Self;

    /// The English labels, translated by the words installed in `ui`'s context. Take it as the base
    /// of a partial override: `Labels { rename: mine, ..Labels::of(ui) }`.
    fn of(ui: &Ui) -> Self {
        Self::english().localized_in(&Words::of(ui))
    }
}

/// Every registered word, once, in registration order.
pub fn all() -> impl Iterator<Item = Word> {
    let mut seen = HashSet::new();
    registry::TABLES.iter().flat_map(|table| table.iter().copied()).filter(move |word| seen.insert(*word))
}

/// Declares a labels struct from one table, `field: "id" = "English";` per row, and implements
/// [`Labels`] and `Default` for it. An empty id makes the word a plain one, looked up by its
/// English text. Every field is a `String`.
///
/// The struct also gets `localized(translate)`, which replaces each field with
/// `translate(`[`Word::key`]`)` where it returns `Some`.
#[macro_export]
macro_rules! words {
    ($(#[$meta:meta])* $vis:vis struct $name:ident {
        $($(#[$fmeta:meta])* $field:ident : $id:literal = $english:literal;)*
    }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        $vis struct $name {
            $($(#[$fmeta])* pub $field: String,)*
        }

        impl $crate::localized::Labels for $name {
            const WORDS: &'static [$crate::localized::Word] = &[$($crate::localized::Word::keyed($id, $english)),*];

            fn english() -> Self {
                Self { $($field: ::std::string::String::from($english)),* }
            }

            fn localized_in(self, words: &$crate::localized::Words) -> Self {
                Self { $($field: words.get($crate::localized::Word::keyed($id, $english)).into_owned()),* }
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self {
                <Self as $crate::localized::Labels>::english()
            }
        }

        impl $name {
            /// Replace each label with `translate(key)`, keeping the current text where it
            /// returns `None`. The key is the word's id, or its English text when it has no id.
            #[must_use]
            pub fn localized(mut self, translate: impl ::std::ops::Fn(&str) -> ::std::option::Option<::std::string::String>) -> Self {
                $(
                    if let ::std::option::Option::Some(text) = translate($crate::localized::Word::keyed($id, $english).key()) {
                        self.$field = text;
                    }
                )*
                self
            }
        }
    };
}

/// The `{name}` placeholders of `text`.
fn placeholders(text: &str) -> impl Iterator<Item = &str> {
    text.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}'))
        .map(|(name, _)| name)
        .filter(|name| !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_'))
}

fn keeps_placeholders(english: &str, translation: &str) -> bool {
    placeholders(english).all(|name| placeholders(translation).any(|found| found == name))
}

fn warn_once(word: Word, translation: &str) {
    static WARNED: LazyLock<Mutex<HashSet<&'static str>>> = LazyLock::new(Mutex::default);
    if WARNED.lock().unwrap_or_else(PoisonError::into_inner).insert(word.key()) {
        log::warn!("translation of {:?} ignored: {translation:?} lacks a placeholder of {:?}", word.key(), word.english);
    }
}

/// What is wrong with `words` as a registry: an id that is not `module.camelCase`, or one id with
/// two English texts.
#[cfg(test)]
pub(crate) fn problems(words: impl IntoIterator<Item = Word>) -> Vec<String> {
    let camel = |part: &str| part.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && part.chars().all(|c| c.is_ascii_alphanumeric());
    let mut problems = Vec::new();
    let mut by_id: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for word in words {
        if word.id.is_empty() {
            continue;
        }
        let parts: Vec<&str> = word.id.split('.').collect();
        if parts.len() < 2 || !parts.iter().all(|part| camel(part)) {
            problems.push(format!("id {:?} is not module.camelCase", word.id));
        }
        if let Some(other) = by_id.insert(word.id, word.english)
            && other != word.english
        {
            problems.push(format!("id {:?} has two texts: {other:?} and {:?}", word.id, word.english));
        }
    }
    problems
}
