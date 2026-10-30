//! The row model the host builds each frame, the texts and what the body reports.

use std::ops::RangeInclusive;

use craft_theme::Rgba;

use crate::localized::{Labels, LocaleNumbers, Word, Words};

/// A setting's value as it crosses the boundary between the window and the host.
#[derive(Clone, Debug, PartialEq)]
pub enum PrefValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    /// The id of the chosen option.
    Choice(String),
    Color(Rgba),
}

/// One page of the page list.
#[derive(Clone, Debug, PartialEq)]
pub struct PageSpec<'a> {
    /// Stable id: `"general"`.
    pub id: &'a str,
    /// The translated name in the page list and over the page.
    pub title: &'a str,
    /// The page's groups; one untitled group for a plain page.
    pub groups: Vec<GroupSpec<'a>>,
    /// A note under the last group.
    pub footnote: Option<&'a str>,
    /// The host draws the whole page ([`PreferencesHost::custom_page`](super::PreferencesHost::custom_page)).
    pub custom: bool,
}

impl<'a> PageSpec<'a> {
    /// A plain page of one untitled group.
    pub fn new(id: &'a str, title: &'a str, rows: Vec<RowSpec<'a>>) -> Self {
        Self { id, title, groups: vec![GroupSpec::new(None, rows)], footnote: None, custom: false }
    }

    /// A page the host draws.
    pub fn custom(id: &'a str, title: &'a str) -> Self {
        Self { id, title, groups: Vec::new(), footnote: None, custom: true }
    }

    pub fn footnote(mut self, footnote: &'a str) -> Self {
        self.footnote = Some(footnote);
        self
    }

    /// Whether the page has anything to show: a custom page, or a visible row.
    pub fn has_content(&self) -> bool {
        self.custom || self.groups.iter().any(|g| g.rows.iter().any(|r| r.visible))
    }
}

/// How a group lays out its rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Columns {
    /// Labels in a column, controls beside them.
    #[default]
    Labelled,
    /// No label column: check boxes flush at the start.
    Checks,
}

/// A group of rows: a titled box, or the page itself when untitled.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupSpec<'a> {
    pub title: Option<&'a str>,
    pub columns: Columns,
    pub rows: Vec<RowSpec<'a>>,
}

impl<'a> GroupSpec<'a> {
    pub fn new(title: Option<&'a str>, rows: Vec<RowSpec<'a>>) -> Self {
        Self { title, columns: Columns::Labelled, rows }
    }

    pub fn checks(mut self) -> Self {
        self.columns = Columns::Checks;
        self
    }
}

/// One setting.
#[derive(Clone, Debug, PartialEq)]
pub struct RowSpec<'a> {
    /// The host's path, unique: `"general.language"`.
    pub key: &'a str,
    /// Translated; also the row's accessible name.
    pub label: &'a str,
    /// Extra search text: the key, the English label, synonyms.
    pub keywords: &'a str,
    /// Help under the control: "Applies at next launch."
    pub help: Option<&'a str>,
    /// False for hidden rows and rows hidden by another value.
    pub visible: bool,
    pub enabled: bool,
    pub kind: RowKind<'a>,
}

impl<'a> RowSpec<'a> {
    /// A visible, enabled row with no keywords or help.
    pub fn new(key: &'a str, label: &'a str, kind: RowKind<'a>) -> Self {
        Self { key, label, keywords: "", help: None, visible: true, enabled: true, kind }
    }

    pub fn keywords(mut self, keywords: &'a str) -> Self {
        self.keywords = keywords;
        self
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// What a row edits, with its value.
#[derive(Clone, Debug, PartialEq)]
pub enum RowKind<'a> {
    Bool(bool),
    /// One of `options`, as `(id, label)`.
    Choice {
        value: &'a str,
        options: Vec<(&'a str, &'a str)>,
    },
    /// A swatch and a hex field. A press on the swatch reports
    /// [`PreferencesOutput::pick_color`]; the host opens its colour dialog.
    Color(Rgba),
    Int {
        value: i64,
        range: RangeInclusive<i64>,
        unit: Option<&'a str>,
    },
    Float {
        value: f64,
        range: RangeInclusive<f64>,
        decimals: u8,
        unit: Option<&'a str>,
    },
    Text(&'a str),
    /// The host draws the control ([`PreferencesHost::custom_row`](super::PreferencesHost::custom_row)).
    Custom,
}

/// Every caption of the window, already translated. The host returns them in the draft's
/// language, so a language preview needs no ambient locale.
#[derive(Clone, Debug)]
pub struct PreferencesTexts {
    pub title: String,
    pub search_label: String,
    pub search_placeholder: String,
    pub clear_search: String,
    pub pages_label: String,
    /// The list's and the page's text when nothing matches the search.
    pub no_matches: String,
    /// A page with no visible row.
    pub empty_page: String,
    pub ok: String,
    pub cancel: String,
    pub apply: String,
    pub reset_section: String,
    /// The accessible name of a colour row's hex field.
    pub hex_label: String,
    /// Read to a screen reader on the default button.
    pub default_hint: String,
    /// The digits and separators of number rows.
    pub numbers: LocaleNumbers,
}

const TITLE: Word = Word::keyed("pref.title", "Preferences");
const SEARCH_LABEL: Word = Word::keyed("pref.searchLabel", "Search settings");
const SEARCH_PLACEHOLDER: Word = Word::keyed("pref.searchPlaceholder", "Search");
const CLEAR_SEARCH: Word = Word::keyed("pref.clearSearch", "Clear search");
const PAGES_LABEL: Word = Word::keyed("pref.pagesLabel", "Pages");
const NO_MATCHES: Word = Word::keyed("pref.noMatches", "No matching settings");
const EMPTY_PAGE: Word = Word::keyed("pref.emptyPage", "These settings aren't available yet.");
const OK: Word = Word::keyed("pref.ok", "OK");
const CANCEL: Word = Word::keyed("pref.cancel", "Cancel");
const APPLY: Word = Word::keyed("pref.apply", "Apply");
const RESET_SECTION: Word = Word::keyed("pref.resetSection", "Reset Section");
const HEX_LABEL: Word = Word::keyed("pref.hexLabel", "Hex colour");
const DEFAULT_HINT: Word = Word::keyed("pref.defaultHint", "Default");

impl PreferencesTexts {
    /// English captions and English numbers.
    pub fn english() -> Self {
        let english = |word: Word| word.english.to_owned();
        Self {
            title: english(TITLE),
            search_label: english(SEARCH_LABEL),
            search_placeholder: english(SEARCH_PLACEHOLDER),
            clear_search: english(CLEAR_SEARCH),
            pages_label: english(PAGES_LABEL),
            no_matches: english(NO_MATCHES),
            empty_page: english(EMPTY_PAGE),
            ok: english(OK),
            cancel: english(CANCEL),
            apply: english(APPLY),
            reset_section: english(RESET_SECTION),
            hex_label: english(HEX_LABEL),
            default_hint: english(DEFAULT_HINT),
            numbers: LocaleNumbers::from_tag("en"),
        }
    }
}

/// The numbers stay as they are: they follow [`LocaleNumbers`], not a catalog row.
impl Labels for PreferencesTexts {
    const WORDS: &'static [Word] = &[
        TITLE,
        SEARCH_LABEL,
        SEARCH_PLACEHOLDER,
        CLEAR_SEARCH,
        PAGES_LABEL,
        NO_MATCHES,
        EMPTY_PAGE,
        OK,
        CANCEL,
        APPLY,
        RESET_SECTION,
        HEX_LABEL,
        DEFAULT_HINT,
    ];

    fn english() -> Self {
        PreferencesTexts::english()
    }

    fn localized_in(self, words: &Words) -> Self {
        let get = |word: Word| words.get(word).into_owned();
        Self {
            title: get(TITLE),
            search_label: get(SEARCH_LABEL),
            search_placeholder: get(SEARCH_PLACEHOLDER),
            clear_search: get(CLEAR_SEARCH),
            pages_label: get(PAGES_LABEL),
            no_matches: get(NO_MATCHES),
            empty_page: get(EMPTY_PAGE),
            ok: get(OK),
            cancel: get(CANCEL),
            apply: get(APPLY),
            reset_section: get(RESET_SECTION),
            hex_label: get(HEX_LABEL),
            default_hint: get(DEFAULT_HINT),
            numbers: self.numbers,
        }
    }
}

/// How the window was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closed {
    /// OK: the host applied the draft.
    Ok,
    /// Cancel, Esc or the window's close: the host dropped the draft.
    Cancel,
}

/// What [`PreferencesBody::show`](super::PreferencesBody::show) reports for one pass.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PreferencesOutput {
    /// Set on the pass the window is answered; the host closes it.
    pub closed: Option<Closed>,
    /// The key of a colour row whose swatch was pressed: the host opens its colour dialog and
    /// writes the result with `set`.
    pub pick_color: Option<String>,
    /// Apply ran this pass and succeeded (OK also applies, and closes).
    pub applied: bool,
    /// The id of the page shown, `None` when the search lists no page.
    pub page: Option<String>,
}
