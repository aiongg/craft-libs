//! `SettingsDocument` without a window.

use std::sync::Arc;

use craft_theme::Rgba;
use serde::{Deserialize, Serialize};

use super::{PageInfo, PrefValue, PreferencesModel, RowKind, SettingRule, SettingsDocument, StandardPage, setting_label};

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

/// `(key, kind name)` of the visible rows of page `id`, in order.
fn rows(doc: &SettingsDocument<Prefs>, id: &str) -> Vec<(String, &'static str)> {
    PreferencesModel::pages(doc)
        .iter()
        .filter(|page| page.id == id)
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .filter(|row| row.visible)
        .map(|row| {
            let kind = match row.kind {
                RowKind::Bool(_) => "bool",
                RowKind::Int { .. } => "int",
                RowKind::Float { .. } => "float",
                RowKind::Text(_) => "text",
                RowKind::Choice { .. } => "choice",
                RowKind::Color(_) => "color",
                RowKind::Custom => "custom",
            };
            (row.key.to_owned(), kind)
        })
        .collect()
}

fn history_states(doc: &SettingsDocument<Prefs>) -> i64 {
    PreferencesModel::pages(doc)
        .iter()
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .find_map(|row| match row.kind {
            RowKind::Int { value, .. } if row.key == "performance.historyStates" => Some(value),
            _ => None,
        })
        .unwrap()
}

// A struct of a bool, an int with a range, a choice and a colour gives one row of each kind on
// its page, in field order.
#[test]
fn rows_follow_the_value_types_and_rules() {
    let doc = document();
    let kinds: Vec<_> = rows(&doc, "performance").into_iter().map(|(key, kind)| (key.trim_start_matches("performance.").to_owned(), kind)).collect();
    // `levels` is an array: no row. The page order is the app's, then the struct's.
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
    let ids: Vec<_> = PreferencesModel::pages(&doc).iter().map(|page| (page.id.to_owned(), page.title.to_owned())).collect();
    assert_eq!(ids, [("performance".to_owned(), "Performance".to_owned()), ("type".to_owned(), "Type".to_owned())]);
    let pages = PreferencesModel::pages(&doc);
    let RowKind::Color(color) = pages[0].groups[0].rows[4].kind else { panic!("not a colour") };
    assert_eq!(color, Rgba::rgb(0x33, 0x66, 0x99));
    // Without a rule a number takes PhotoCraft's open range.
    let RowKind::Int { range, .. } = &pages[1].groups[0].rows[0].kind else { panic!("not an int") };
    assert_eq!(range, &(-1_000_000_000..=1_000_000_000));
}

// Labels: "historyStates" → "History states", "gpuMemoryMb" style replacements, a rule's label
// wins, and a translation under setting.<key> replaces the English.
#[test]
fn labels_come_from_the_key_or_the_catalog() {
    assert_eq!(setting_label("performance.historyStates"), "History states");
    assert_eq!(setting_label("gpuMemoryMb"), "GPU memory mb");
    assert_eq!(setting_label("psdCompatibility"), "PSD compatibility");
    assert_eq!(setting_label("uiScale"), "UI scale");
    assert_eq!(setting_label("exifData"), "EXIF data");

    let label_of = |doc: &SettingsDocument<Prefs>, key: &str| {
        PreferencesModel::pages(doc)
            .iter()
            .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
            .find(|row| row.key == key)
            .map(|row| row.label.to_owned())
            .unwrap()
    };
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
    let fr = i18n.from_code("fr").unwrap();
    let host = craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).i18n(Arc::new(i18n)).lang(fr).build();
    doc.relabel(&host);
    assert_eq!(label_of(&doc, "performance.historyStates"), "États d'historique");
    // The rule's English label is the catalog's source text; without a row it shows.
    assert_eq!(label_of(&doc, "performance.zoomStep"), "Pas du zoom");
    assert_eq!(PreferencesModel::pages(&doc)[0].title, "Performances");
    assert_eq!(PreferencesModel::pages(&doc)[1].title, "Type");
}

// set clamps to the range; Apply returns the typed value once through take_applied; Cancel
// restores the draft.
#[test]
fn apply_returns_the_typed_value_and_cancel_drops_the_draft() {
    let mut doc = document();
    assert!(!doc.differs());
    doc.set("performance.historyStates", PrefValue::Int(5000));
    assert_eq!(history_states(&doc), 1000);
    doc.set("performance.renderMode", PrefValue::Choice("nope".into()));
    doc.set("performance.renderMode", PrefValue::Choice("soft".into()));
    doc.set("performance.canvasCustomColor", PrefValue::Color(Rgba([0x11, 0x22, 0x33, 0x80])));
    doc.set("performance.zoomStep", PrefValue::Float(9.0));
    assert!(doc.differs());

    doc.cancel();
    assert!(!doc.differs());
    assert_eq!(history_states(&doc), 50);

    doc.set("performance.historyStates", PrefValue::Int(0));
    doc.set("performance.renderMode", PrefValue::Choice("soft".into()));
    doc.set("performance.canvasCustomColor", PrefValue::Color(Rgba([0x11, 0x22, 0x33, 0x80])));
    doc.set("performance.zoomStep", PrefValue::Float(9.0));
    doc.apply().unwrap();
    let applied = doc.take_applied().unwrap();
    assert_eq!(applied.performance.history_states, 1);
    assert_eq!(applied.performance.render_mode, "soft");
    assert_eq!(applied.performance.canvas_custom_color, "#112233");
    assert_eq!(applied.performance.zoom_step, 4.0);
    assert_eq!(applied.performance.levels, [1, 2]);
    assert_eq!(*doc.saved(), applied);
    assert!(doc.take_applied().is_none());
    assert!(!doc.differs());

    // A value the struct can't hold refuses Apply and keeps the draft.
    doc.set("performance.tileSize", PrefValue::Int(300));
    let error = doc.apply().unwrap_err();
    assert!(error.starts_with("These settings can't be applied: "), "{error}");
    assert!(doc.take_applied().is_none());
    assert!(doc.differs());
}

// Reset Section restores the page's defaults except keep_on_reset keys; other pages keep their
// draft.
#[test]
fn reset_section_keeps_marked_keys() {
    let mut doc =
        SettingsDocument::new(Prefs::default()).expect("the prefs serialize").rules(vec![SettingRule::new("type.favoriteFont").keep_on_reset()]);
    doc.set("type.favoriteFont", PrefValue::Text("Inter".into()));
    doc.set("type.recentFonts", PrefValue::Int(9));
    doc.set("performance.historyStates", PrefValue::Int(7));
    doc.reset_page("type");
    doc.apply().unwrap();
    let applied = doc.take_applied().unwrap();
    assert_eq!(applied.text, Type { recent_fonts: 0, favorite_font: "Inter".into() });
    assert_eq!(applied.performance.history_states, 7);
}

// Hidden rules, linux_only off Linux and shown_when with another value hide the row.
#[test]
fn hidden_rows_stay_hidden() {
    let mut doc = SettingsDocument::new(Prefs::default()).expect("the prefs serialize").rules(vec![
        SettingRule::new("performance.zoomStep").hidden(),
        SettingRule::new("performance.tileSize").linux_only(),
        SettingRule::new("performance.canvasCustomColor").color().shown_when("performance.renderMode", "soft"),
        SettingRule::new("performance.renderMode").choices(&[("auto", "Automatic"), ("soft", "Software")]),
    ]);
    let keys = |doc: &SettingsDocument<Prefs>| -> Vec<String> { rows(doc, "performance").into_iter().map(|(key, _)| key).collect() };
    let mut expected = vec!["performance.useGpu", "performance.historyStates", "performance.renderMode"];
    if cfg!(target_os = "linux") {
        expected.push("performance.tileSize");
    }
    assert_eq!(keys(&doc), expected);

    doc.set("performance.renderMode", PrefValue::Choice("soft".into()));
    let mut shown = expected.clone();
    shown.insert(3, "performance.canvasCustomColor");
    assert_eq!(keys(&doc), shown);
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

// Reset Section replaces the page: a key the defaults leave out (an Option that is None) goes too.
#[test]
fn reset_section_drops_keys_the_defaults_do_not_serialize() {
    let saved = Sparse { brush: SparseBrush { size: 9, last_preset: Some("soft".into()) } };
    let mut doc = SettingsDocument::new(saved).expect("serializes");
    doc.reset_page("brush");
    doc.apply().unwrap();
    assert_eq!(doc.take_applied().unwrap(), Sparse::default());
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct NoTomlForm {
    map: std::collections::BTreeMap<(i32, i32), i32>,
}

// A struct toml can't serialize is an error, not an empty draft that Apply would read as defaults.
#[test]
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

// The autosave row stores an integer; an old settings file that holds the minutes as text still
// shows its choice.
#[test]
fn the_minutes_rule_stores_integers_and_still_reads_text() {
    let rules = || vec![SettingRule::new("general.autosaveMinutes").minutes(&[5, 10, 15, 30, 60])];
    let selected = |doc: &dyn PreferencesModel| {
        let pages = doc.pages();
        let row =
            pages.iter().flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter())).find(|row| row.key == "general.autosaveMinutes");
        let Some(RowKind::Choice { value, .. }) = row.map(|row| &row.kind) else { panic!("no choice row") };
        value.to_string()
    };
    let old = SettingsDocument::new(FilePrefs { general: Files { autosave_minutes: "30".into() } })
        .expect("the prefs serialize")
        .pages(StandardPage::ALL.map(PageInfo::from).to_vec())
        .rules(rules());
    assert_eq!(selected(&old), "30");

    let mut doc = SettingsDocument::new(IntegerFilePrefs { general: IntegerFiles { autosave_minutes: 10 } })
        .expect("the prefs serialize")
        .pages(StandardPage::ALL.map(PageInfo::from).to_vec())
        .rules(rules());
    assert_eq!(selected(&doc), "10");
    PreferencesModel::set(&mut doc, "general.autosaveMinutes", PrefValue::Choice("15".into()));
    assert_eq!(selected(&doc), "15");
    assert_eq!(PreferencesModel::apply(&mut doc), Ok(()), "the integer field reads what the row wrote");
}

// The autosave row offers the five interval choices as "{n} minutes", with 10 selected by default.
#[test]
fn the_minutes_rule_offers_the_autosave_intervals_with_ten_selected() {
    let prefs = FilePrefs { general: Files { autosave_minutes: "10".into() } };
    let doc = SettingsDocument::new(prefs)
        .expect("the prefs serialize")
        .pages(StandardPage::ALL.map(PageInfo::from).to_vec())
        .rules(vec![SettingRule::new("general.autosaveMinutes").minutes(&[5, 10, 15, 30, 60])]);
    let pages = PreferencesModel::pages(&doc);
    let row = pages
        .iter()
        .flat_map(|page| page.groups.iter().flat_map(|group| group.rows.iter()))
        .find(|row| row.key == "general.autosaveMinutes")
        .expect("a row");
    let RowKind::Choice { value, options } = &row.kind else { panic!("not a choice") };
    assert_eq!(*value, "10");
    let options: Vec<_> = options.iter().map(|(id, label)| (id.to_string(), label.to_string())).collect();
    let expected: Vec<_> = [5, 10, 15, 30, 60].map(|n| (n.to_string(), format!("{n} minutes"))).to_vec();
    assert_eq!(options, expected);
}
