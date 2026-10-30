//! The row model the host builds each frame, the texts and what the body reports.

use std::ops::RangeInclusive;

use craft_theme::Rgba;

use crate::localized::LocaleNumbers;

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

impl PreferencesTexts {
    /// English captions and English numbers.
    pub fn english() -> Self {
        Self {
            title: "Preferences".into(),
            search_label: "Search settings".into(),
            search_placeholder: "Search".into(),
            clear_search: "Clear search".into(),
            pages_label: "Pages".into(),
            no_matches: "No matching settings".into(),
            empty_page: "These settings aren't available yet.".into(),
            ok: "OK".into(),
            cancel: "Cancel".into(),
            apply: "Apply".into(),
            reset_section: "Reset Section".into(),
            hex_label: "Hex colour".into(),
            default_hint: "Default".into(),
            numbers: LocaleNumbers::from_tag("en"),
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
