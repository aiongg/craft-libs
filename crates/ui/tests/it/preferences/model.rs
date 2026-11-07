//! The search over the row model and the settings document, without a window.

use std::sync::Arc;

use craft_host::target::Os;
use craft_i18n::Folder;
use craft_i18n::matcher::Matcher;
use craft_theme::Rgba;
use craft_ui::patterns::preferences::{
    GroupSpec, PageInfo, PageSpec, PrefValue, PreferencesModel, RowKind, RowSpec, SettingRule, SettingsDocument, StandardPage, list_pages, matches,
    row_shown, setting_label, shown_position,
};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::trial;
use serde::{Deserialize, Serialize};

fn matcher(query: &str) -> Matcher {
    Matcher::new(&Folder::new("en"), query)
}

fn pages() -> Vec<PageSpec<'static>> {
    let flag = |key, label| RowSpec::new(key, label, RowKind::Bool(false));
    vec![
        PageSpec::new("general", "General", vec![flag("general.language", "Language:").keywords("locale")]),
        PageSpec::new("tools", "Tools", vec![flag("tools.snap", "Snap to grid"), flag("tools.smoothing", "Smoothing")]),
        PageSpec::new("plugins", "Plug-ins", vec![flag("plugins.hidden", "Hidden").visible(false)]),
        PageSpec::new("advanced", "Advanced", vec![flag("advanced.cache", "Cache").help("Used when memory runs low.")]),
    ]
}

fn listed_ids(pages: &[PageSpec<'_>], query: &str) -> Vec<String> {
    list_pages(pages, &matcher(query)).iter().map(|l| pages[l.index].id.to_owned()).collect()
}

/// A query matches the label, keywords or key, ignoring case, surrounding space and accents, and an empty query matches
/// everything.
#[trial]
fn the_search_rule_ignores_case_space_and_accents() {
    assert!(matches(&matcher("  LANG "), "Language:", "", "general.language"));
    assert!(matches(&matcher("locale"), "Language:", "locale", "x"));
    assert!(matches(&matcher("general.l"), "Language:", "", "general.language"));
    assert!(matches(&matcher("cafe"), "Café", "", ""));
    assert!(!matches(&matcher("zzz"), "Language:", "locale", "general.language"));
    assert!(matches(&matcher(""), "anything", "", ""));
}

/// A title match lists the page with every visible row; a row match lists the page with that row only.
#[trial]
fn a_title_match_shows_every_row_and_a_row_match_only_that_row() {
    let pages = pages();
    let by_title = matcher("tools");
    let listed = list_pages(&pages, &by_title);
    assert_eq!(listed.len(), 1);
    let shown = |matcher: &Matcher, listed: &craft_ui::patterns::preferences::Listed| -> Vec<&str> {
        pages[listed.index].groups[0].rows.iter().filter(|r| row_shown(matcher, listed, r)).map(|r| r.key).collect()
    };
    assert_eq!(shown(&by_title, &listed[0]), ["tools.snap", "tools.smoothing"]);
    let by_row = matcher("smooth");
    let listed = list_pages(&pages, &by_row);
    assert_eq!(listed.len(), 1);
    assert_eq!(shown(&by_row, &listed[0]), ["tools.smoothing"]);
    assert_eq!(listed_ids(&pages, "locale"), ["general"]);
}

/// A page with no visible row is dimmed in the list and drops out of a search unless its title matches.
#[trial]
fn empty_pages_are_dimmed_and_drop_out_of_a_search_unless_the_title_matches() {
    let pages = pages();
    assert!(list_pages(&pages, &matcher("")).iter().any(|l| l.dimmed && pages[l.index].id == "plugins"));
    assert!(!listed_ids(&pages, "hidden").contains(&"plugins".to_owned()));
    assert!(listed_ids(&pages, "plug").contains(&"plugins".to_owned()));
}

/// The selected page stays selected while listed, falls back to the first listed one while the search hides it, and
/// there is none when nothing is listed.
#[trial]
fn the_shown_page_falls_back_to_the_first_listed() {
    let pages = pages();
    let hidden = list_pages(&pages, &matcher("smooth"));
    assert_eq!(shown_position(&hidden, &pages, "general"), Some(0));
    assert_eq!(pages[hidden[0].index].id, "tools");
    let all = list_pages(&pages, &matcher(""));
    assert_eq!(shown_position(&all, &pages, "tools"), Some(1));
    assert_eq!(shown_position(&list_pages(&pages, &matcher("nothing matches")), &pages, "tools"), None);
}

/// A row matches by its help text, and a custom page by its keywords.
#[trial]
fn a_row_matches_by_its_help_and_a_custom_page_by_its_keywords() {
    assert_eq!(listed_ids(&pages(), "memory runs low"), ["advanced"]);
    let custom = vec![PageSpec::custom("graphics", "Graphics").keywords("GPU adapter"), PageSpec::custom("other", "Other")];
    assert_eq!(listed_ids(&custom, "adapter"), ["graphics"]);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Performance {
    use_gpu: bool,
    history_states: i64,
    zoom_step: f64,
    render_mode: String,
    canvas_custom_color: String,
    tile_size: u8,
    levels: Vec<i64>,
}

impl Default for Performance {
    fn default() -> Self {
        Self {
            use_gpu: true,
            history_states: 50,
            zoom_step: 1.5,
            render_mode: "auto".into(),
            canvas_custom_color: "#336699".into(),
            tile_size: 64,
            levels: vec![1, 2],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Type {
    recent_fonts: i64,
    favorite_font: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Prefs {
    performance: Performance,
    #[serde(rename = "type")]
    text: Type,
}

fn rules() -> Vec<SettingRule> {
    vec![
        SettingRule::new("performance.historyStates").int(1..=1000),
        SettingRule::new("performance.zoomStep").float(0.5..=4.0, 2),
        SettingRule::new("performance.renderMode").choices(&[("auto", "Automatic"), ("soft", "Software")]),
        SettingRule::new("performance.canvasCustomColor").color(),
        SettingRule::new("performance.tileSize").int(0..=1000),
    ]
}

fn document() -> SettingsDocument<Prefs> {
    SettingsDocument::new(Prefs::default()).expect("the prefs serialize").pages(StandardPage::ALL.map(PageInfo::from).to_vec()).rules(rules())
}

fn kind_name(kind: &RowKind<'_>) -> &'static str {
    match kind {
        RowKind::Bool(_) => "bool",
        RowKind::Int { .. } => "int",
        RowKind::Float { .. } => "float",
        RowKind::Text(_) => "text",
        RowKind::Choice { .. } => "choice",
        RowKind::Color(_) => "color",
        RowKind::Custom => "custom",
        _ => "other",
    }
}

/// `(key, kind name)` of the visible rows of page `id`, in order.
fn rows(doc: &impl PreferencesModel, id: &str) -> Vec<(String, &'static str)> {
    doc.pages()
        .iter()
        .filter(|page| page.id == id)
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .filter(|row| row.visible)
        .map(|row| (row.key.to_owned(), kind_name(&row.kind)))
        .collect()
}

fn label_of(doc: &impl PreferencesModel, key: &str) -> String {
    doc.pages()
        .iter()
        .flat_map(|page| page.groups.iter().flat_map(|group: &GroupSpec<'_>| group.rows.iter()))
        .find(|row| row.key == key)
        .map(|row| row.label.to_owned())
        .expect("a row")
}

fn history_states(doc: &impl PreferencesModel) -> i64 {
    doc.pages()
        .iter()
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .find_map(|row| match row.kind {
            RowKind::Int { value, .. } if row.key == "performance.historyStates" => Some(value),
            _ => None,
        })
        .expect("a row")
}

/// A struct of a bool, an int with a range, a float, a choice and a colour gives one row of each kind on its page, in
/// field order.
#[trial]
fn rows_follow_the_value_types_and_rules() {
    let doc = document();
    let kinds: Vec<_> = rows(&doc, "performance").into_iter().map(|(key, kind)| (key.trim_start_matches("performance.").to_owned(), kind)).collect();
    // `levels` is an array: no row.
    assert_eq!(
        kinds,
        [
            ("useGpu".to_owned(), "bool"),
            ("historyStates".to_owned(), "int"),
            ("zoomStep".to_owned(), "float"),
            ("renderMode".to_owned(), "choice"),
            ("canvasCustomColor".to_owned(), "color"),
            ("tileSize".to_owned(), "int"),
        ]
    );
    let pages = PreferencesModel::pages(&doc);
    let ids: Vec<_> = pages.iter().map(|page| (page.id.to_owned(), page.title.to_owned())).collect();
    assert_eq!(ids, [("performance".to_owned(), "Performance".to_owned()), ("type".to_owned(), "Type".to_owned())]);
    let RowKind::Color(color) = pages[0].groups[0].rows[4].kind else { panic!("not a colour") };
    assert_eq!(color, Rgba::rgb(0x33, 0x66, 0x99));
    // Without a rule a number takes the open range.
    let RowKind::Int { range, .. } = &pages[1].groups[0].rows[0].kind else { panic!("not an int") };
    assert_eq!(range, &(-1_000_000_000..=1_000_000_000));
}

/// Labels come from the key, a rule's label wins, and a translation under `setting.<key>` replaces the English.
#[trial]
fn labels_come_from_the_key_or_the_catalog() {
    assert_eq!(setting_label("performance.historyStates"), "History states");
    assert_eq!(setting_label("gpuMemoryMb"), "GPU memory mb");
    assert_eq!(setting_label("psdCompatibility"), "PSD compatibility");
    assert_eq!(setting_label("uiScale"), "UI scale");
    assert_eq!(setting_label("exifData"), "EXIF data");

    let doc = document();
    assert_eq!(label_of(&doc, "performance.historyStates"), "History states");
    assert_eq!(label_of(&doc, "performance.zoomStep"), "Zoom step");

    let mut doc = SettingsDocument::new(Prefs::default())
        .expect("the prefs serialize")
        .rules(vec![SettingRule::new("performance.zoomStep").label("Zoom increment")]);
    assert_eq!(label_of(&doc, "performance.zoomStep"), "Zoom increment");

    let (i18n, problems) = craft_i18n::I18n::new(
        &[(
            "fr",
            "",
            "@id\tsetting.performance.historyStates\tÉtats d'historique\n@id\tsetting.performance.zoomStep\tPas du zoom\n@id\tsettingsPage.performance\tPerformances\n",
        )],
        &["fr"],
    );
    assert!(problems.is_empty(), "{problems:?}");
    let fr = i18n.from_code("fr").expect("fr is offered");
    let host = craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).i18n(Arc::new(i18n)).lang(fr).build();
    doc.relabel(&host);
    assert_eq!(label_of(&doc, "performance.historyStates"), "États d'historique");
    assert_eq!(label_of(&doc, "performance.zoomStep"), "Pas du zoom");
    let titled = PreferencesModel::pages(&doc);
    assert_eq!(titled[0].title, "Performances");
    assert_eq!(titled[1].title, "Type");
}

/// `set` clamps to the range and keeps to the choices; Apply returns the typed value once through `take_applied`; Cancel
/// restores the draft; a value the struct can't hold refuses Apply and keeps the draft.
#[trial]
fn apply_returns_the_typed_value_and_cancel_drops_the_draft() {
    let mut doc = document();
    assert!(!doc.differs());
    doc.set("performance.historyStates", PrefValue::Int(5000));
    assert_eq!(history_states(&doc), 1000);
    doc.set("performance.renderMode", PrefValue::Choice("nope".into()));
    doc.set("performance.renderMode", PrefValue::Choice("soft".into()));
    assert!(doc.differs());

    doc.cancel();
    assert!(!doc.differs());
    assert_eq!(history_states(&doc), 50);

    doc.set("performance.historyStates", PrefValue::Int(0));
    doc.set("performance.renderMode", PrefValue::Choice("soft".into()));
    doc.set("performance.canvasCustomColor", PrefValue::Color(Rgba([0x11, 0x22, 0x33, 0x80])));
    doc.set("performance.zoomStep", PrefValue::Float(9.0));
    doc.apply().expect("the draft fits the struct");
    let applied = doc.take_applied().expect("applied");
    assert_eq!(applied.performance.history_states, 1);
    assert_eq!(applied.performance.render_mode, "soft");
    assert_eq!(applied.performance.canvas_custom_color, "#112233");
    assert_eq!(applied.performance.zoom_step, 4.0);
    assert_eq!(applied.performance.levels, [1, 2]);
    assert_eq!(*doc.saved(), applied);
    assert!(doc.take_applied().is_none());
    assert!(!doc.differs());

    doc.set("performance.tileSize", PrefValue::Int(300));
    let error = doc.apply().expect_err("300 doesn't fit a u8");
    assert!(error.starts_with("These settings can't be applied: "), "{error}");
    assert!(doc.take_applied().is_none());
    assert!(doc.differs());
}

/// Reset Section restores the page's defaults except `keep_on_reset` keys; other pages keep their draft.
#[trial]
fn reset_section_keeps_marked_keys() {
    let mut doc =
        SettingsDocument::new(Prefs::default()).expect("the prefs serialize").rules(vec![SettingRule::new("type.favoriteFont").keep_on_reset()]);
    doc.set("type.favoriteFont", PrefValue::Text("Inter".into()));
    doc.set("type.recentFonts", PrefValue::Int(9));
    doc.set("performance.historyStates", PrefValue::Int(7));
    doc.reset_page("type");
    doc.apply().expect("the draft fits the struct");
    let applied = doc.take_applied().expect("applied");
    assert_eq!(applied.text, Type { recent_fonts: 0, favorite_font: "Inter".into() });
    assert_eq!(applied.performance.history_states, 7);
}

/// Hidden rules, `linux_only` off Linux and `shown_when` with another value hide the row.
#[trial]
fn hidden_rows_stay_hidden() {
    let build = |os| {
        SettingsDocument::new(Prefs::default())
            .expect("the prefs serialize")
            .rules(vec![
                SettingRule::new("performance.zoomStep").hidden(),
                SettingRule::new("performance.tileSize").linux_only(),
                SettingRule::new("performance.canvasCustomColor").color().shown_when("performance.renderMode", "soft"),
                SettingRule::new("performance.renderMode").choices(&[("auto", "Automatic"), ("soft", "Software")]),
            ])
            .os(os)
    };
    let keys = |doc: &SettingsDocument<Prefs>| -> Vec<String> { rows(doc, "performance").into_iter().map(|(key, _)| key).collect() };
    let base = ["performance.useGpu", "performance.historyStates", "performance.renderMode"];
    let mut doc = build(Os::Mac);
    assert_eq!(keys(&doc), base);
    assert_eq!(keys(&build(Os::Linux)), [base.as_slice(), &["performance.tileSize"]].concat());

    doc.set("performance.renderMode", PrefValue::Choice("soft".into()));
    assert_eq!(keys(&doc), ["performance.useGpu", "performance.historyStates", "performance.renderMode", "performance.canvasCustomColor"]);
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Sparse {
    brush: SparseBrush,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SparseBrush {
    size: i64,
    last_preset: Option<String>,
}

/// Reset Section replaces the page: a key the defaults leave out (an `Option` that is `None`) goes too.
#[trial]
fn reset_section_drops_keys_the_defaults_do_not_serialize() {
    let saved = Sparse { brush: SparseBrush { size: 9, last_preset: Some("soft".into()) } };
    let mut doc = SettingsDocument::new(saved).expect("serializes");
    doc.reset_page("brush");
    doc.apply().expect("the draft fits the struct");
    assert_eq!(doc.take_applied().expect("applied"), Sparse::default());
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct NoTomlForm {
    map: std::collections::BTreeMap<(i32, i32), i32>,
}

/// A struct without a TOML form is an error, not an empty draft that Apply would read as defaults.
#[trial]
fn a_struct_without_a_toml_form_is_refused() {
    let mut value = NoTomlForm::default();
    value.map.insert((1, 2), 3);
    assert!(SettingsDocument::new(value).is_err());
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Files {
    autosave_minutes: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct FilePrefs {
    general: Files,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegerFiles {
    autosave_minutes: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct IntegerFilePrefs {
    general: IntegerFiles,
}

/// The `(chosen id, [(option id, label)])` of the autosave row.
fn autosave_choice(doc: &impl PreferencesModel) -> (String, Vec<(String, String)>) {
    let pages = doc.pages();
    let row = pages
        .iter()
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .find(|row| row.key == "general.autosaveMinutes")
        .expect("a row");
    let RowKind::Choice { value, options, .. } = &row.kind else { panic!("not a choice") };
    let options = options.iter().map(|entry| (entry.value().expect("an option").to_string(), entry.label().as_str().to_owned())).collect();
    (value.to_string(), options)
}

fn autosave_unit(doc: &impl PreferencesModel) -> Option<UnitKind> {
    let pages = doc.pages();
    let row = pages
        .iter()
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .find(|row| row.key == "general.autosaveMinutes")
        .expect("a row");
    let RowKind::Choice { unit, .. } = &row.kind else { panic!("not a choice") };
    *unit
}

fn autosave_rule() -> Vec<SettingRule> {
    vec![SettingRule::new("general.autosaveMinutes").minutes(&[5, 10, 15, 30, 60])]
}

/// The minutes rule stores integers, an old settings file that holds the minutes as text still shows its choice, and the
/// row offers the five intervals counted in minutes.
#[trial]
fn the_minutes_rule_stores_integers_and_still_reads_text() {
    let pages = StandardPage::ALL.map(PageInfo::from).to_vec();
    let old = SettingsDocument::new(FilePrefs { general: Files { autosave_minutes: "30".into() } })
        .expect("the prefs serialize")
        .pages(pages.clone())
        .rules(autosave_rule());
    assert_eq!(autosave_choice(&old).0, "30");

    let mut doc = SettingsDocument::new(IntegerFilePrefs { general: IntegerFiles { autosave_minutes: 10 } })
        .expect("the prefs serialize")
        .pages(pages)
        .rules(autosave_rule());
    let (chosen, options) = autosave_choice(&doc);
    assert_eq!(chosen, "10");
    let expected: Vec<_> = [5, 10, 15, 30, 60].map(|n| (n.to_string(), n.to_string())).to_vec();
    assert_eq!(options, expected);
    assert_eq!(autosave_unit(&doc), Some(UnitKind::Minutes), "the row spells the counts as minutes");
    doc.set("general.autosaveMinutes", PrefValue::Choice("15".into()));
    assert_eq!(autosave_choice(&doc).0, "15");
    assert_eq!(doc.apply(), Ok(()), "the integer field reads what the row wrote");
}
