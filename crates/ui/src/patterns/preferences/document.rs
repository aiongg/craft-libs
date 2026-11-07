//! [`SettingsDocument`]: the preferences model over an app's settings struct, one row per setting.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::sync::{Arc, OnceLock};

use craft_host::Host;
use craft_host::target::Os;
use craft_i18n::{I18n, Lang, Words};
use craft_theme::{Rgba, Srgba};
use egui::{Context, Ui};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::model::{GroupSpec, PageSpec, PrefValue, PreferencesModel, RowKind, RowSpec};
use super::words::SettingsDocumentWords;
use crate::contract::Entry;
use crate::runtime::env::{Label, UnitKind};

/// A page of the usual creative-app preferences, in its order, with the id (the top-level key of the settings struct
/// that fills it) and English title each has. An app shows the pages it has settings for; [`PageInfo::new`] adds its
/// own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StandardPage {
    /// General.
    General,
    /// Interface.
    Interface,
    /// Workspace.
    Workspace,
    /// Tools.
    Tools,
    /// History Log.
    HistoryLog,
    /// File Handling.
    FileHandling,
    /// Export.
    Export,
    /// Performance.
    Performance,
    /// Scratch Disks.
    ScratchDisks,
    /// Cursors.
    Cursors,
    /// Transparency & Gamut.
    TransparencyAndGamut,
    /// Units & Rulers.
    UnitsAndRulers,
    /// Guides, Grid & Slices.
    GuidesGridAndSlices,
    /// Plug-ins.
    PlugIns,
    /// Type.
    Type,
    /// Enhanced Controls.
    EnhancedControls,
    /// Camera Raw Defaults.
    RawDefaults,
    /// Integrations.
    Integrations,
}

impl StandardPage {
    /// Every page in its order.
    pub const ALL: [StandardPage; 18] = [
        StandardPage::General,
        StandardPage::Interface,
        StandardPage::Workspace,
        StandardPage::Tools,
        StandardPage::HistoryLog,
        StandardPage::FileHandling,
        StandardPage::Export,
        StandardPage::Performance,
        StandardPage::ScratchDisks,
        StandardPage::Cursors,
        StandardPage::TransparencyAndGamut,
        StandardPage::UnitsAndRulers,
        StandardPage::GuidesGridAndSlices,
        StandardPage::PlugIns,
        StandardPage::Type,
        StandardPage::EnhancedControls,
        StandardPage::RawDefaults,
        StandardPage::Integrations,
    ];

    /// The page's id and English title: `("general", "General")`, `("historyLog", "History Log")`,
    /// `("transparencyAndGamut", "Transparency & Gamut")`, `("guidesGridAndSlices", "Guides, Grid & Slices")`,
    /// `("plugIns", "Plug-ins")`, `("rawDefaults", "Camera Raw Defaults")`, and so on.
    pub fn info(self) -> PageInfo {
        let (id, title) = match self {
            StandardPage::General => ("general", "General"),
            StandardPage::Interface => ("interface", "Interface"),
            StandardPage::Workspace => ("workspace", "Workspace"),
            StandardPage::Tools => ("tools", "Tools"),
            StandardPage::HistoryLog => ("historyLog", "History Log"),
            StandardPage::FileHandling => ("fileHandling", "File Handling"),
            StandardPage::Export => ("export", "Export"),
            StandardPage::Performance => ("performance", "Performance"),
            StandardPage::ScratchDisks => ("scratchDisks", "Scratch Disks"),
            StandardPage::Cursors => ("cursors", "Cursors"),
            StandardPage::TransparencyAndGamut => ("transparencyAndGamut", "Transparency & Gamut"),
            StandardPage::UnitsAndRulers => ("unitsAndRulers", "Units & Rulers"),
            StandardPage::GuidesGridAndSlices => ("guidesGridAndSlices", "Guides, Grid & Slices"),
            StandardPage::PlugIns => ("plugIns", "Plug-ins"),
            StandardPage::Type => ("type", "Type"),
            StandardPage::EnhancedControls => ("enhancedControls", "Enhanced Controls"),
            StandardPage::RawDefaults => ("rawDefaults", "Camera Raw Defaults"),
            StandardPage::Integrations => ("integrations", "Integrations"),
        };
        PageInfo::new(id, title)
    }
}

/// A page: the top-level key of the settings struct it shows, and its English title, translated under
/// `settingsPage.<id>`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageInfo {
    /// The top-level key of the settings struct the page shows (`"interface"`).
    pub id: String,
    /// The English title; translated under `settingsPage.<id>`.
    pub title: String,
}

impl PageInfo {
    /// A page with the given id and English title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self { id: id.into(), title: title.into() }
    }
}

impl From<StandardPage> for PageInfo {
    fn from(page: StandardPage) -> Self {
        page.info()
    }
}

/// What the document knows about one setting beyond its type: its range or choices, its label, when it shows. A setting
/// without a rule still gets a row, from its value's type.
///
/// ```ignore
/// SettingRule::new("performance.historyStates").int(1..=1000),
/// SettingRule::new("interface.canvasColor").choices(&[("gray", "Gray"), ("black", "Black"), ("custom", "Custom")]),
/// SettingRule::new("interface.canvasCustomColor").color().shown_when("interface.canvasColor", "custom"),
/// SettingRule::new("interface.systemTitleBar").restart(),
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SettingRule {
    /// The dotted path: `"<page>.<key>"`.
    pub key: String,
    /// English label; without one the key's last part is spelled out ([`setting_label`]).
    pub label: Option<String>,
    /// English help text shown with the row; `None` shows none.
    pub help: Option<String>,
    /// The control to use; `None` picks it from the value's type.
    pub kind: Option<RuleKind>,
    /// Shown only while another setting has this value.
    pub shown_when: Option<(String, String)>,
    /// Never shown.
    pub hidden: bool,
    /// Shown only on Linux.
    pub linux_only: bool,
    /// Read at launch: "Applies at next launch." under the control.
    pub restart: bool,
    /// Kept by Reset Section.
    pub keep_on_reset: bool,
    /// The titled group the row goes in; rows without one go in the page's first, untitled group.
    pub group: Option<String>,
    /// A choice whose ids are whole numbers is stored as an integer ([`SettingRule::minutes`]).
    integer: bool,
    /// What the ids of a choice count; the row labels the options with it ([`SettingRule::minutes`]).
    counted: Option<UnitKind>,
}

/// A rule's control, where the value's type alone isn't enough.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RuleKind {
    /// An integer field limited to the range.
    #[non_exhaustive]
    Int {
        /// The values the field accepts.
        range: RangeInclusive<i64>,
        /// What the number counts, shown after the field.
        unit: Option<UnitKind>,
    },
    /// A number field limited to the range.
    #[non_exhaustive]
    Float {
        /// The values the field accepts.
        range: RangeInclusive<f64>,
        /// How many decimal places the field shows.
        decimals: u8,
        /// What the number counts, shown after the field.
        unit: Option<UnitKind>,
    },
    /// A list of `(id, English label)`; labels translate under `setting.<key>.<id>`.
    Choice(Vec<(String, String)>),
    /// A `#rrggbb` string as a swatch and hex field.
    Color,
    /// The app draws it ([`CustomSettings::row`]).
    Custom,
}

impl SettingRule {
    /// A rule for the setting at the dotted `key` (`"<page>.<key>"`), with no overrides.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: None,
            help: None,
            kind: None,
            shown_when: None,
            hidden: false,
            linux_only: false,
            restart: false,
            keep_on_reset: false,
            group: None,
            integer: false,
            counted: None,
        }
    }

    /// Sets the English label; it translates under `setting.<key>`.
    #[must_use]
    pub fn label(mut self, english: impl Into<String>) -> Self {
        self.label = Some(english.into());
        self
    }

    /// Sets the English help text shown with the row; it translates under `setting.<key>.help`.
    #[must_use]
    pub fn help(mut self, english: impl Into<String>) -> Self {
        self.help = Some(english.into());
        self
    }

    /// An integer field limited to `range`.
    #[must_use]
    pub fn int(mut self, range: RangeInclusive<i64>) -> Self {
        self.kind = Some(RuleKind::Int { range, unit: None });
        self
    }

    /// A number field limited to `range`, showing `decimals` decimal places.
    #[must_use]
    pub fn float(mut self, range: RangeInclusive<f64>, decimals: u8) -> Self {
        self.kind = Some(RuleKind::Float { range, decimals, unit: None });
        self
    }

    /// What an `int` or `float` field counts, shown after it in the user's language (px, %); ignored on other kinds.
    #[must_use]
    pub fn counting(mut self, kind: UnitKind) -> Self {
        if let Some(RuleKind::Int { unit, .. } | RuleKind::Float { unit, .. }) = &mut self.kind {
            *unit = Some(kind);
        }
        self
    }

    /// A choice among `(id, English label)` pairs; labels translate under `setting.<key>.<id>`.
    #[must_use]
    pub fn choices(mut self, options: &[(&str, &str)]) -> Self {
        self.kind = Some(RuleKind::Choice(options.iter().map(|(id, label)| ((*id).to_owned(), (*label).to_owned())).collect()));
        self
    }

    /// A choice of minute counts, stored as the integer count (`10`). The row spells each count as minutes in the
    /// user's language ([`UnitKind::Minutes`]). A settings file that holds the count as text ("10") still shows its
    /// choice; the next change writes an integer, so the app's field reads both.
    #[must_use]
    pub fn minutes(mut self, counts: &[u32]) -> Self {
        let labels: Vec<(String, String)> = counts.iter().map(|n| (n.to_string(), n.to_string())).collect();
        let options: Vec<(&str, &str)> = labels.iter().map(|(id, label)| (id.as_str(), label.as_str())).collect();
        self.integer = true;
        self.counted = Some(UnitKind::Minutes);
        self.choices(&options)
    }

    /// A `#rrggbb` string shown as a swatch and hex field.
    #[must_use]
    pub fn color(mut self) -> Self {
        self.kind = Some(RuleKind::Color);
        self
    }

    /// The app draws the control ([`CustomSettings::row`]).
    #[must_use]
    pub fn custom(mut self) -> Self {
        self.kind = Some(RuleKind::Custom);
        self
    }

    /// Shows the row only while the setting at `key` has `value`.
    #[must_use]
    pub fn shown_when(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.shown_when = Some((key.into(), value.into()));
        self
    }

    /// Never shows the row.
    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Shows the row only on Linux.
    #[must_use]
    pub fn linux_only(mut self) -> Self {
        self.linux_only = true;
        self
    }

    /// Marks the setting as read at launch: "Applies at next launch." shows under the control.
    #[must_use]
    pub fn restart(mut self) -> Self {
        self.restart = true;
        self
    }

    /// Keeps the setting's value when the user resets its page.
    #[must_use]
    pub fn keep_on_reset(mut self) -> Self {
        self.keep_on_reset = true;
        self
    }

    /// Puts the row in the titled group `english`; the title translates under `settingGroup.<english>`.
    #[must_use]
    pub fn group(mut self, english: impl Into<String>) -> Self {
        self.group = Some(english.into());
        self
    }
}

/// The app's part of a settings document: rows and pages it draws itself, dialogs it opens from them, and settings that
/// live outside the struct (the appearance page writes the suite's theme selection). Every method has a default, so `()`
/// is a document without any.
pub trait CustomSettings<T> {
    /// Draws a [`RuleKind::Custom`] row's control; edits go into `draft`.
    fn row(&mut self, _ui: &mut Ui, _key: &str, _draft: &mut SettingsDraft) {}
    /// Draws a page listed with [`SettingsDocument::custom_page`].
    fn page(&mut self, _ui: &mut Ui, _page: &str, _draft: &mut SettingsDraft) {}
    /// Dialogs opened from a custom row (a colour picker), drawn last in the window's viewport. A dialog's answer goes
    /// into `draft`.
    fn nested(&mut self, _ctx: &Context, _draft: &mut SettingsDraft) {}
    /// Whether a dialog drawn by [`Self::nested`] is open.
    fn nested_open(&self) -> bool {
        false
    }
    /// Whether the app's own part differs from what is saved (enables Apply).
    fn differs(&self) -> bool {
        false
    }
    /// Commits the app's own part; `Err` keeps the window open with the message.
    fn apply(&mut self, _saved: &T) -> Result<(), String> {
        Ok(())
    }
    /// Drops the app's own draft.
    fn cancel(&mut self) {}
    /// Resets the app's own part of `page`.
    fn reset_page(&mut self, _page: &str) {}
}

impl<T> CustomSettings<T> for () {}

/// The draft values by dotted key, for custom rows and pages.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsDraft {
    table: toml::Table,
}

impl SettingsDraft {
    /// The value at `key` (`"tools.pressureCurve"`), when it is there and fits `V`.
    pub fn get<V: DeserializeOwned>(&self, key: &str) -> Option<V> {
        lookup(&self.table, key)?.clone().try_into().ok()
    }

    /// Writes `value` at `key`; a value that doesn't serialize is ignored.
    pub fn set<V: Serialize>(&mut self, key: &str, value: &V) {
        if let Ok(value) = toml::Value::try_from(value) {
            insert(&mut self.table, key, value);
        }
    }

    /// Writes the colour row `key` (the `#rrggbb` text a [`SettingRule::color`] row holds), for the colour a picker
    /// opened from [`CustomSettings::nested`] returned.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        self.set(key, &Rgba::from(color).to_hex());
    }
}

/// The value at the dotted `key` of `table`.
fn lookup<'t>(table: &'t toml::Table, key: &str) -> Option<&'t toml::Value> {
    let (head, rest) = match key.split_once('.') {
        Some((head, rest)) => (head, Some(rest)),
        None => (key, None),
    };
    let value = table.get(head)?;
    match rest {
        None => Some(value),
        Some(rest) => lookup(value.as_table()?, rest),
    }
}

/// Writes `value` at the dotted `key`, creating missing tables on the way; a path through a value that is not a table
/// writes nothing.
fn insert(table: &mut toml::Table, key: &str, value: toml::Value) {
    match key.split_once('.') {
        None => {
            table.insert(key.to_owned(), value);
        }
        Some((head, rest)) => {
            if let toml::Value::Table(inner) = table.entry(head).or_insert_with(|| toml::Value::Table(toml::Table::new())) {
                insert(inner, rest, value);
            }
        }
    }
}

/// The preferences model built from the app's settings struct `T`.
///
/// Every top-level table of `T` is a page ([`StandardPage`], or [`PageInfo`] for the app's own), and every value in it
/// a row by its type: a bool is a check box, an integer an integer field (-1,000,000,000 to 1,000,000,000 without a
/// rule's range), a float a number field with three decimals, a string a text field (a list with
/// [`SettingRule::choices`], a swatch and hex field with [`SettingRule::color`]). Nested tables and arrays get no row
/// unless a rule makes them custom. Rows keep the struct's field order; titled groups follow in their first row's order.
///
/// Labels come from the key through the shared catalog: the id `setting.<key>` (`setting.performance.historyStates`)
/// with the rule's English label, or the key spelled out by [`setting_label`] ("History states"); a missing translation
/// shows the English. Search matches the key, the English label and the translated label; a page whose title matches
/// shows all its rows.
///
/// The window edits a draft. Apply turns the draft back into `T` through serde: a value that doesn't fit refuses Apply
/// with the `settingsDocument.invalid` word; otherwise the draft is the new saved value and
/// [`take_applied`](Self::take_applied) hands it to the app, which saves it. Cancel drops the draft. Reset Section puts
/// `T::default()`'s values of the page into the draft, except rules marked [`keep_on_reset`](SettingRule::keep_on_reset).
///
/// ```ignore
/// let mut prefs = SettingsDocument::new(store.get().clone())?
///     .pages(StandardPage::ALL.map(PageInfo::from).to_vec())
///     .rules(app_rules());
/// // each pass while the window is open:
/// let out = PreferencesWindow::new(label).show(ui, &mut prefs);
/// if let Some(value) = prefs.take_applied() { store.set(value); }
/// ```
pub struct SettingsDocument<T, C = ()> {
    saved: T,
    /// `saved` as a table: what the draft is compared with.
    saved_table: toml::Table,
    draft: SettingsDraft,
    defaults: toml::Table,
    /// The struct's top-level tables and their keys in field order (a `toml::Table` sorts them).
    order: FieldOrder,
    pages: Vec<PageInfo>,
    custom_pages: Vec<String>,
    rules: BTreeMap<String, SettingRule>,
    custom: C,
    /// The system `linux_only` rules compare with.
    os: Os,
    /// The catalogs and language of [`relabel`](Self::relabel); English until then.
    translator: Option<(Arc<I18n>, Lang)>,
    own: Arc<SettingsDocumentWords>,
    /// The page and row texts, built on the first [`PreferencesModel::pages`] after a change of pages, rules, language or
    /// struct.
    texts: OnceLock<Texts>,
    applied: Option<T>,
}

/// The order of the settings struct's fields: its tables, and the keys of each.
#[derive(Clone, Debug, Default)]
struct FieldOrder {
    pages: Vec<String>,
    keys: BTreeMap<String, Vec<String>>,
}

impl FieldOrder {
    fn of<T: Serialize>(value: &T) -> Self {
        let mut order = FieldOrder::default();
        if let Ok(document) = toml_edit::ser::to_document(value) {
            for (page, item) in document.iter() {
                if let Some(table) = item.as_table_like() {
                    order.pages.push(page.to_owned());
                    order.keys.insert(page.to_owned(), table.iter().map(|(key, _)| key.to_owned()).collect());
                }
            }
        }
        order
    }
}

fn table_of<T: Serialize>(value: &T) -> Result<toml::Table, toml::ser::Error> {
    toml::Table::try_from(value)
}

impl<T: Serialize + DeserializeOwned + Default + Clone + PartialEq> SettingsDocument<T> {
    /// A document over `saved`, with the draft equal to it. `Err` when `T` has no TOML form (a map with non-string keys,
    /// a top-level value that is not a table): an empty draft would reset every setting when Apply reads it back.
    pub fn new(saved: T) -> Result<Self, toml::ser::Error> {
        let saved_table = table_of(&saved)?;
        Ok(SettingsDocument {
            order: FieldOrder::of(&saved),
            draft: SettingsDraft { table: saved_table.clone() },
            saved_table,
            defaults: table_of(&T::default())?,
            saved,
            pages: Vec::new(),
            custom_pages: Vec::new(),
            rules: BTreeMap::new(),
            custom: (),
            os: Os::current(),
            translator: None,
            own: Arc::default(),
            texts: OnceLock::new(),
            applied: None,
        })
    }
}

impl<T: Serialize + DeserializeOwned + Default + Clone + PartialEq, C: CustomSettings<T>> SettingsDocument<T, C> {
    /// The page order; tables not listed follow in the struct's order, titled by [`setting_label`] of their key.
    #[must_use]
    pub fn pages(mut self, pages: Vec<PageInfo>) -> Self {
        self.pages = pages;
        self.texts = OnceLock::new();
        self
    }

    /// Pages the app draws whole ([`CustomSettings::page`]), by id.
    #[must_use]
    pub fn custom_page(mut self, page: impl Into<String>) -> Self {
        self.custom_pages.push(page.into());
        self.texts = OnceLock::new();
        self
    }

    /// The per-setting rules. A later rule for the same key replaces an earlier one.
    #[must_use]
    pub fn rules(mut self, rules: Vec<SettingRule>) -> Self {
        self.rules = rules.into_iter().map(|r| (r.key.clone(), r)).collect();
        self.texts = OnceLock::new();
        self
    }

    /// The system that [`SettingRule::linux_only`] compares with; the system this build runs on until set.
    #[must_use]
    pub fn os(mut self, os: Os) -> Self {
        self.os = os;
        self
    }

    /// The app's custom rows, pages and outside settings.
    pub fn custom<D: CustomSettings<T>>(self, custom: D) -> SettingsDocument<T, D> {
        SettingsDocument {
            saved: self.saved,
            saved_table: self.saved_table,
            draft: self.draft,
            defaults: self.defaults,
            order: self.order,
            pages: self.pages,
            custom_pages: self.custom_pages,
            rules: self.rules,
            custom,
            os: self.os,
            translator: self.translator,
            own: self.own,
            texts: self.texts,
            applied: self.applied,
        }
    }

    /// Translates the labels into the Host's language; call when the document is made and when the language changes (a
    /// language preview).
    pub fn relabel(&mut self, host: &Host) {
        self.translator = host.i18n_shared().map(|i18n| (Arc::clone(i18n), host.lang()));
        let words = match &self.translator {
            Some((i18n, lang)) => Words::from_i18n(Arc::clone(i18n), *lang),
            None => Words::english(),
        };
        self.own = words.set::<SettingsDocumentWords>();
        self.texts = OnceLock::new();
    }

    /// The app's custom part, for the app to open the dialog a [`PreferencesOutput::pick_color`](super::PreferencesOutput)
    /// request asks for.
    pub fn custom_mut(&mut self) -> &mut C {
        &mut self.custom
    }

    /// The saved value the window started from (or last applied).
    pub fn saved(&self) -> &T {
        &self.saved
    }

    /// Replaces the saved value and the draft (another instance changed the file). `Err` leaves the document as it was.
    pub fn reload(&mut self, saved: T) -> Result<(), toml::ser::Error> {
        self.saved_table = table_of(&saved)?;
        self.draft = SettingsDraft { table: self.saved_table.clone() };
        self.order = FieldOrder::of(&saved);
        self.saved = saved;
        self.texts = OnceLock::new();
        Ok(())
    }

    /// The value Apply or OK committed, once.
    pub fn take_applied(&mut self) -> Option<T> {
        self.applied.take()
    }
}

/// The translated texts of the pages and rows, built from the draft's keys, the rules and the catalogs.
struct Texts {
    pages: Vec<PageText>,
}

struct PageText {
    id: String,
    title: String,
    /// The English title, for the search.
    keywords: String,
    custom: bool,
    /// The titled groups in the order of their first row.
    groups: Vec<String>,
    rows: Vec<RowText>,
}

struct RowText {
    /// The dotted path.
    key: String,
    label: String,
    /// The English label, for the search.
    keywords: String,
    help: Option<String>,
    /// The index in [`PageText::groups`].
    group: Option<usize>,
    shape: Shape,
}

/// The control of a row, from its rule and the type of its value.
enum Shape {
    Bool,
    Int {
        range: RangeInclusive<i64>,
        unit: Option<UnitKind>,
    },
    Float {
        range: RangeInclusive<f64>,
        decimals: u8,
        unit: Option<UnitKind>,
    },
    Text,
    /// `(id, translated label)`, and what the ids count when the row labels them itself.
    Choice(Vec<(String, Label)>, Option<UnitKind>),
    Color,
    Custom,
}

/// The range of a number without a rule.
const OPEN_RANGE: f64 = 1e9;

fn shape_of(rule: Option<&SettingRule>, value: Option<&toml::Value>) -> Option<Shape> {
    use toml::Value;
    let kind = rule.and_then(|rule| rule.kind.as_ref());
    if matches!(kind, Some(RuleKind::Custom)) {
        return Some(Shape::Custom);
    }
    Some(match (kind, value?) {
        (Some(RuleKind::Choice(options)), Value::String(_) | Value::Integer(_)) => {
            let counted = rule.and_then(|rule| rule.counted);
            Shape::Choice(options.iter().map(|(id, label)| (id.clone(), Label::from(label.clone()))).collect(), counted)
        }
        (Some(RuleKind::Color), Value::String(_)) => Shape::Color,
        (Some(RuleKind::Int { range, unit }), Value::Integer(_)) => Shape::Int { range: range.clone(), unit: *unit },
        (Some(RuleKind::Float { range, decimals, unit }), Value::Float(_)) => Shape::Float { range: range.clone(), decimals: *decimals, unit: *unit },
        (_, Value::Boolean(_)) => Shape::Bool,
        (_, Value::Integer(_)) => Shape::Int { range: -(OPEN_RANGE as i64)..=OPEN_RANGE as i64, unit: None },
        (_, Value::Float(_)) => Shape::Float { range: -OPEN_RANGE..=OPEN_RANGE, decimals: 3, unit: None },
        (_, Value::String(_)) => Shape::Text,
        _ => return None,
    })
}

/// A choice row of the translated `options`, with `value` chosen.
fn choice_kind<'a>(options: &'a [(String, Label)], counted: Option<UnitKind>, value: &'a str) -> RowKind<'a> {
    let kind = RowKind::choice(value, options.iter().map(|(id, label)| Entry::new(id.as_str(), label.clone())).collect());
    match counted {
        Some(unit) => kind.counting(unit),
        None => kind,
    }
}

/// What `shown_when` compares: a string as it is, a bool or number as Rust writes it.
fn value_text(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        toml::Value::Boolean(on) => on.to_string(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(x) => x.to_string(),
        _ => String::new(),
    }
}

impl<T: Serialize + DeserializeOwned + Default + Clone + PartialEq, C: CustomSettings<T>> SettingsDocument<T, C> {
    /// The English text under the catalog `id`, translated into the language of [`relabel`](Self::relabel).
    fn translate(&self, id: &str, english: &str) -> String {
        match &self.translator {
            Some((i18n, lang)) => match i18n.tid(*lang, id, english) {
                "" => english.to_owned(),
                text => text.to_owned(),
            },
            None => english.to_owned(),
        }
    }

    fn texts(&self) -> &Texts {
        self.texts.get_or_init(|| self.build_texts())
    }

    /// Every page with a table in the struct or a custom page, in the app's order and then the struct's: `(id, English
    /// title)`.
    fn page_list(&self) -> Vec<(String, String)> {
        let mut list: Vec<(String, String)> = Vec::new();
        let known = |id: &str| self.order.keys.contains_key(id) || self.custom_pages.iter().any(|page| page == id);
        let add = |id: &str, title: String, list: &mut Vec<(String, String)>| {
            if !list.iter().any(|(listed, _)| listed == id) {
                list.push((id.to_owned(), title));
            }
        };
        for page in self.pages.iter().filter(|page| known(&page.id)) {
            add(&page.id, page.title.clone(), &mut list);
        }
        for id in self.order.pages.iter().chain(&self.custom_pages) {
            add(id, setting_label(id), &mut list);
        }
        list
    }

    fn build_texts(&self) -> Texts {
        let pages = self
            .page_list()
            .into_iter()
            .map(|(id, english)| {
                let custom = self.custom_pages.contains(&id);
                let mut page = PageText {
                    title: self.translate(&format!("settingsPage.{id}"), &english),
                    keywords: english,
                    custom,
                    groups: Vec::new(),
                    rows: Vec::new(),
                    id,
                };
                if !custom {
                    self.build_rows(&mut page);
                }
                page
            })
            .collect();
        Texts { pages }
    }

    fn build_rows(&self, page: &mut PageText) {
        let table = self.saved_table.get(&page.id).and_then(toml::Value::as_table);
        let mut keys: Vec<String> = self.order.keys.get(&page.id).cloned().unwrap_or_default();
        // Custom rows of settings that live outside the struct.
        let prefix = format!("{}.", page.id);
        for (path, rule) in self.rules.range(prefix.clone()..).take_while(|(path, _)| path.starts_with(&prefix)) {
            let key = &path[prefix.len()..];
            if matches!(rule.kind, Some(RuleKind::Custom)) && !keys.iter().any(|k| k == key) {
                keys.push(key.to_owned());
            }
        }
        for key in keys {
            let path = format!("{}.{key}", page.id);
            let rule = self.rules.get(&path);
            let Some(mut shape) = shape_of(rule, table.and_then(|t| t.get(&key))) else { continue };
            if let Shape::Choice(options, None) = &mut shape {
                for (id, label) in options.iter_mut() {
                    *label = Label::from(self.translate(&format!("setting.{path}.{id}"), label.as_str()));
                }
            }
            let english = rule.and_then(|rule| rule.label.clone()).unwrap_or_else(|| setting_label(&path));
            let mut help = Vec::new();
            if let Some(text) = rule.and_then(|rule| rule.help.as_deref()) {
                help.push(self.translate(&format!("setting.{path}.help"), text));
            }
            if rule.is_some_and(|rule| rule.restart) {
                help.push(self.own.restart.as_str().to_owned());
            }
            let group = rule.and_then(|rule| rule.group.as_deref()).map(|english| {
                let title = self.translate(&format!("settingGroup.{english}"), english);
                page.groups.iter().position(|g| *g == title).unwrap_or_else(|| {
                    page.groups.push(title);
                    page.groups.len() - 1
                })
            });
            page.rows.push(RowText {
                label: self.translate(&format!("setting.{path}"), &english),
                keywords: english,
                help: (!help.is_empty()).then(|| help.join("\n")),
                key: path,
                group,
                shape,
            });
        }
    }

    /// Whether the rule lets the row show with the draft's values.
    fn row_visible(&self, key: &str) -> bool {
        let Some(rule) = self.rules.get(key) else { return true };
        if rule.hidden || (rule.linux_only && self.os != Os::Linux) {
            return false;
        }
        match &rule.shown_when {
            Some((other, wanted)) => lookup(&self.draft.table, other).is_some_and(|value| value_text(value) == *wanted),
            None => true,
        }
    }

    fn row_spec<'a>(&'a self, row: &'a RowText) -> Option<RowSpec<'a>> {
        use toml::Value;
        let value = lookup(&self.draft.table, &row.key);
        let kind = match (&row.shape, value) {
            (Shape::Bool, Some(Value::Boolean(on))) => RowKind::Bool(*on),
            (Shape::Int { range, unit }, Some(Value::Integer(n))) => RowKind::Int { value: *n, range: range.clone(), unit: *unit },
            (Shape::Float { range, decimals, unit }, Some(Value::Float(x))) => {
                RowKind::Float { value: *x, range: range.clone(), decimals: *decimals, unit: *unit }
            }
            (Shape::Text, Some(Value::String(text))) => RowKind::Text(text),
            (Shape::Choice(options, counted), Some(Value::Integer(n))) => {
                let id = options.iter().map(|(id, _)| id.as_str()).find(|id| *id == n.to_string()).unwrap_or("");
                choice_kind(options, *counted, id)
            }
            (Shape::Choice(options, counted), Some(Value::String(id))) => choice_kind(options, *counted, id),
            (Shape::Color, Some(Value::String(text))) => RowKind::Color(Rgba::from_hex(text).unwrap_or(Rgba::rgb(128, 128, 128))),
            (Shape::Custom, _) => RowKind::Custom,
            _ => return None,
        };
        let spec = RowSpec::new(&row.key, &row.label, kind).keywords(&row.keywords).visible(self.row_visible(&row.key));
        Some(match &row.help {
            Some(help) => spec.help(help),
            None => spec,
        })
    }
}

impl<T: Serialize + DeserializeOwned + Default + Clone + PartialEq, C: CustomSettings<T>> PreferencesModel for SettingsDocument<T, C> {
    fn pages(&self) -> Vec<PageSpec<'_>> {
        self.texts()
            .pages
            .iter()
            .map(|page| {
                if page.custom {
                    return PageSpec::custom(&page.id, &page.title).keywords(&page.keywords);
                }
                let mut groups: Vec<GroupSpec<'_>> = std::iter::once(None)
                    .chain(page.groups.iter().map(|title| Some(title.as_str())))
                    .map(|title| GroupSpec::new(title, Vec::new()))
                    .collect();
                for row in &page.rows {
                    if let Some(spec) = self.row_spec(row) {
                        groups[row.group.map_or(0, |at| at + 1)].rows.push(spec);
                    }
                }
                groups.retain(|group| !group.rows.is_empty());
                let mut spec = PageSpec::new(&page.id, &page.title, Vec::new()).keywords(&page.keywords);
                spec.groups = groups;
                spec
            })
            .collect()
    }

    /// Writes the row's value into the draft, clamped to its range or kept to its choices.
    fn set(&mut self, key: &str, value: PrefValue) {
        use toml::Value;
        let Some(current) = lookup(&self.draft.table, key) else { return };
        let kind = self.rules.get(key).and_then(|rule| rule.kind.as_ref());
        let int_range = match kind {
            Some(RuleKind::Int { range, .. }) => range.clone(),
            _ => -(OPEN_RANGE as i64)..=OPEN_RANGE as i64,
        };
        let float_range = match kind {
            Some(RuleKind::Float { range, .. }) => range.clone(),
            _ => -OPEN_RANGE..=OPEN_RANGE,
        };
        let clamp_int = |n: i64| n.max(*int_range.start()).min(*int_range.end());
        let clamp_float = |x: f64| x.max(*float_range.start()).min(*float_range.end());
        let new = match (value, current) {
            (PrefValue::Bool(on), Value::Boolean(_)) => Value::Boolean(on),
            (PrefValue::Int(n), Value::Integer(_)) => Value::Integer(clamp_int(n)),
            (PrefValue::Int(n), Value::Float(_)) => Value::Float(clamp_float(n as f64)),
            (PrefValue::Float(x), Value::Float(_)) if x.is_finite() => Value::Float(clamp_float(x)),
            (PrefValue::Float(x), Value::Integer(_)) if x.is_finite() => Value::Integer(clamp_int(x.round() as i64)),
            (PrefValue::Text(text), Value::String(_)) if matches!(kind, None | Some(RuleKind::Custom)) => Value::String(text),
            (PrefValue::Choice(id), Value::String(_) | Value::Integer(_)) => match kind {
                Some(RuleKind::Choice(options)) if options.iter().any(|(option, _)| *option == id) => match id.parse::<i64>() {
                    Ok(n) if self.rules.get(key).is_some_and(|rule| rule.integer) => Value::Integer(n),
                    _ if matches!(current, Value::Integer(_)) => return,
                    _ => Value::String(id),
                },
                _ => return,
            },
            (PrefValue::Color(color), Value::String(_)) if matches!(kind, Some(RuleKind::Color)) => {
                Value::String(Rgba::rgb(color.0[0], color.0[1], color.0[2]).to_hex())
            }
            _ => return,
        };
        insert(&mut self.draft.table, key, new);
    }

    fn custom_row(&mut self, ui: &mut Ui, key: &str) {
        self.custom.row(ui, key, &mut self.draft);
    }

    fn custom_page(&mut self, ui: &mut Ui, page: &str) {
        self.custom.page(ui, page, &mut self.draft);
    }

    fn differs(&self) -> bool {
        self.draft.table != self.saved_table || self.custom.differs()
    }

    fn apply(&mut self) -> Result<(), String> {
        let words = Arc::clone(&self.own);
        let invalid = |error: String| words.invalid.fill(&[("error", &error)]);
        let value: T =
            toml::Value::Table(self.draft.table.clone()).try_into().map_err(|error: toml::de::Error| invalid(error.message().to_owned()))?;
        // The serialized value is the next draft: the struct's own reading of the values.
        let table = table_of(&value).map_err(|error| invalid(error.to_string()))?;
        self.custom.apply(&value)?;
        self.saved_table = table;
        self.draft = SettingsDraft { table: self.saved_table.clone() };
        self.saved = value.clone();
        self.applied = Some(value);
        Ok(())
    }

    fn cancel(&mut self) {
        self.draft = SettingsDraft { table: self.saved_table.clone() };
        self.custom.cancel();
    }

    fn reset_page(&mut self, page: &str) {
        if let Some(toml::Value::Table(defaults)) = self.defaults.get(page) {
            // The page is replaced, so a key the defaults leave out (an `Option` that is `None`) goes too; only the
            // `keep_on_reset` keys survive.
            let mut reset = defaults.clone();
            if let Some(toml::Value::Table(current)) = self.draft.table.get(page) {
                for (key, value) in current {
                    if self.rules.get(&format!("{page}.{key}")).is_some_and(|rule| rule.keep_on_reset) {
                        reset.insert(key.clone(), value.clone());
                    }
                }
            }
            self.draft.table.insert(page.to_owned(), toml::Value::Table(reset));
        }
        self.custom.reset_page(page);
    }

    fn nested(&mut self, ctx: &Context) {
        self.custom.nested(ctx, &mut self.draft);
    }

    fn nested_open(&self) -> bool {
        self.custom.nested_open()
    }
}

/// A key's English label in sentence case: the last part of the dotted key with its first letter upper-cased and each
/// later capital turned into a space and the lower-case letter ("historyStates" becomes "History states"), then these
/// replacements in this order: "Psd" to "PSD", "Gpu" to "GPU", "Ui " to "UI ", "Mb" to "(MB)", "Exif" to "EXIF", "Hud" to
/// "HUD", "Lut " to "LUT ". A rule's label wins over this.
pub fn setting_label(key: &str) -> String {
    /// The spelling of acronyms and units in a label made from a key, applied in this order.
    const REPLACEMENTS: [(&str, &str); 7] =
        [("Psd", "PSD"), ("Gpu", "GPU"), ("Ui ", "UI "), ("Mb", "(MB)"), ("Exif", "EXIF"), ("Hud", "HUD"), ("Lut ", "LUT ")];
    let leaf = key.rsplit('.').next().unwrap_or(key);
    let mut label = String::new();
    for (at, ch) in leaf.chars().enumerate() {
        if at == 0 {
            label.extend(ch.to_uppercase());
        } else if ch.is_uppercase() {
            label.push(' ');
            label.extend(ch.to_lowercase());
        } else {
            label.push(ch);
        }
    }
    REPLACEMENTS.iter().fold(label, |label, (from, to)| label.replace(from, to))
}
