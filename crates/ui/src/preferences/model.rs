//! The row model the host builds each frame, the texts and what the body reports.

use std::ops::RangeInclusive;

use craft_theme::Rgba;

use crate::dialog::ColorRequest;
use crate::localized::{Labels, Word, Words};

/// A setting's value as it crosses the boundary between the window and the host.
#[derive(Clone, Debug, PartialEq)]
pub enum PrefValue {
    /// A check box or switch.
    Bool(bool),
    /// An integer field.
    Int(i64),
    /// A number field.
    Float(f64),
    /// A text field.
    Text(String),
    /// The id of the chosen option.
    Choice(String),
    /// A colour.
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
    /// Extra search text of the page, for a custom page whose controls the search can't see.
    pub keywords: &'a str,
    /// The host draws the whole page ([`PreferencesModel::custom_page`](super::PreferencesModel::custom_page)).
    pub custom: bool,
}

impl<'a> PageSpec<'a> {
    /// A plain page of one untitled group.
    pub fn new(id: &'a str, title: &'a str, rows: Vec<RowSpec<'a>>) -> Self {
        Self { id, title, groups: vec![GroupSpec::new(None, rows)], footnote: None, keywords: "", custom: false }
    }

    /// A page the host draws.
    pub fn custom(id: &'a str, title: &'a str) -> Self {
        Self { id, title, groups: Vec::new(), footnote: None, keywords: "", custom: true }
    }

    /// Sets the note shown under the last group.
    pub fn footnote(mut self, footnote: &'a str) -> Self {
        self.footnote = Some(footnote);
        self
    }

    /// Sets extra search text for the page.
    pub fn keywords(mut self, keywords: &'a str) -> Self {
        self.keywords = keywords;
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
    /// Check boxes start at the group's left edge, under the label column; the other rows keep
    /// their labels in it. Without this, a check box sits in the control column.
    Checks,
}

/// A group of rows: a titled box, or the page itself when untitled.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupSpec<'a> {
    /// The translated title of the box; `None` for the page's untitled group.
    pub title: Option<&'a str>,
    /// How the rows are laid out.
    pub columns: Columns,
    /// The group's rows, in order.
    pub rows: Vec<RowSpec<'a>>,
}

impl<'a> GroupSpec<'a> {
    /// A group with [`Columns::Labelled`] layout.
    pub fn new(title: Option<&'a str>, rows: Vec<RowSpec<'a>>) -> Self {
        Self { title, columns: Columns::Labelled, rows }
    }

    /// Switches the group to [`Columns::Checks`] layout.
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
    /// Extra search text: the key, the English label, synonyms. A [`RowKind::Custom`] row, which
    /// the search can't look into, lists the terms of its control here.
    pub keywords: &'a str,
    /// Help under the control: "Applies at next launch." A line break starts a new paragraph.
    /// The search matches it.
    pub help: Option<&'a str>,
    /// False for hidden rows and rows hidden by another value.
    pub visible: bool,
    /// False: the control is drawn disabled.
    pub enabled: bool,
    /// The step of a [`RowKind::Float`] row (arrow keys, wheel, drag); without one the step comes
    /// from the range and decimals. Ignored by other kinds.
    pub step: Option<f64>,
    /// The control and the value it shows.
    pub kind: RowKind<'a>,
}

impl<'a> RowSpec<'a> {
    /// A visible, enabled row with no keywords or help.
    pub fn new(key: &'a str, label: &'a str, kind: RowKind<'a>) -> Self {
        Self { key, label, keywords: "", help: None, visible: true, enabled: true, step: None, kind }
    }

    /// Sets extra search text for the row.
    pub fn keywords(mut self, keywords: &'a str) -> Self {
        self.keywords = keywords;
        self
    }

    /// Sets the help text under the control.
    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    /// Shows or hides the row.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Enables or disables the control.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the step of a [`RowKind::Float`] row.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }
}

/// What a row edits, with its value.
#[derive(Clone, Debug, PartialEq)]
pub enum RowKind<'a> {
    /// A check box.
    Bool(bool),
    /// One of `options`, as `(id, label)`.
    Choice {
        /// The id of the chosen option.
        value: &'a str,
        /// The options as `(id, translated label)`.
        options: Vec<(&'a str, &'a str)>,
    },
    /// A swatch and a hex field. A press on the swatch reports
    /// [`PreferencesOutput::pick_color`]; the host opens its colour picker.
    Color(Rgba),
    /// An integer field.
    Int {
        /// The current value.
        value: i64,
        /// The values the field accepts.
        range: RangeInclusive<i64>,
        /// Text shown after the field ("px").
        unit: Option<&'a str>,
    },
    /// A number field.
    Float {
        /// The current value.
        value: f64,
        /// The values the field accepts.
        range: RangeInclusive<f64>,
        /// How many decimal places the field shows.
        decimals: u8,
        /// Text shown after the field ("%").
        unit: Option<&'a str>,
    },
    /// A text field.
    Text(&'a str),
    /// The host draws the control ([`PreferencesModel::custom_row`](super::PreferencesModel::custom_row)).
    Custom,
}

/// Every caption of the window, in the language of the installed Host.
#[derive(Clone, Debug)]
pub struct PreferencesTexts {
    /// The window's title.
    pub title: String,
    /// The accessible name of the search field.
    pub search_label: String,
    /// The search field's hint text.
    pub search_placeholder: String,
    /// The accessible name of the button that clears the search.
    pub clear_search: String,
    /// The accessible name of the page list.
    pub pages_label: String,
    /// The list's and the page's text when nothing matches the search.
    pub no_matches: String,
    /// A page with no visible row.
    pub empty_page: String,
    /// The OK button.
    pub ok: String,
    /// The Cancel button.
    pub cancel: String,
    /// The Apply button.
    pub apply: String,
    /// The button that resets the shown page to its defaults.
    pub reset_section: String,
    /// The accessible name of a colour row's hex field.
    pub hex_label: String,
    /// Read to a screen reader on the default button.
    pub default_hint: String,
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
const HEX_LABEL: Word = Word::keyed("pref.hexLabel", "Hex color");
const DEFAULT_HINT: Word = Word::keyed("pref.defaultHint", "Default");

impl PreferencesTexts {
    /// English captions.
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
        }
    }
}

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
    /// The colour row whose swatch was pressed, for this pass only: the host opens its colour
    /// dialog and answers with `PreferencesModel::set_color` on the request's key.
    pub pick_color: Option<ColorRequest>,
    /// Apply ran this pass and succeeded (OK also applies, and closes).
    pub applied: bool,
    /// The id of the page shown, `None` when the search lists no page.
    pub page: Option<String>,
}
