//! Story of [`SettingsDocument`]: a preferences window built from a sample settings struct with a
//! row of every kind.

use std::sync::{Arc, Mutex, PoisonError};

use egui::{Ui, vec2};
use serde::{Deserialize, Serialize};

use super::{CustomSettings, PageInfo, PreferencesBody, SettingRule, SettingsDocument, SettingsDraft, StandardPage};
use crate::controls::Button;
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct DocumentArgs {
    search: String,
    #[arg(range = 560.0..=1040.0)]
    width: f32,
    #[arg(range = 400.0..=800.0)]
    height: f32,
}

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/settings-document/window",
        Level::Patterns,
        "SettingsDocument",
        "Window",
        DocumentArgs { search: String::new(), width: 760.0, height: 480.0 },
        show_document
    )]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct General {
    language: String,
    name: String,
    autosave: bool,
    recent_files: i64,
}

impl Default for General {
    fn default() -> Self {
        Self { language: "en".into(), name: "Studio".into(), autosave: true, recent_files: 20 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Interface {
    ui_scale: f64,
    canvas_color: String,
    canvas_custom_color: String,
    system_title_bar: bool,
    tooltips: bool,
    rulers: bool,
}

impl Default for Interface {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            canvas_color: "gray".into(),
            canvas_custom_color: "#336699".into(),
            system_title_bar: false,
            tooltips: true,
            rulers: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Performance {
    use_gpu: bool,
    history_states: i64,
    cache_size: i64,
}

impl Default for Performance {
    fn default() -> Self {
        Self { use_gpu: true, history_states: 50, cache_size: 2048 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Tools {
    smoothing: f64,
    brush_shortcut: String,
    legacy_mode: bool,
}

impl Default for Tools {
    fn default() -> Self {
        Self { smoothing: 0.5, brush_shortcut: "B".into(), legacy_mode: false }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Sample {
    general: General,
    interface: Interface,
    performance: Performance,
    tools: Tools,
}

const SHORTCUTS: [&str; 3] = ["B", "Shift+B", "Alt+B"];

/// The app's own row: the brush shortcut, cycled by a button.
struct BrushShortcut;

impl CustomSettings<Sample> for BrushShortcut {
    fn row(&mut self, ui: &mut Ui, key: &str, draft: &mut SettingsDraft) {
        let current: String = draft.get(key).unwrap_or_default();
        if ui.add(Button::new(current.as_str()).id_salt(key)).clicked() {
            let next = SHORTCUTS.iter().position(|s| *s == current).map_or(0, |at| (at + 1) % SHORTCUTS.len());
            draft.set(key, &SHORTCUTS[next]);
        }
    }
}

fn rules() -> Vec<SettingRule> {
    vec![
        SettingRule::new("general.language").choices(&[("en", "English"), ("de", "Deutsch"), ("fr", "Fran\u{e7}ais")]).restart(),
        SettingRule::new("general.recentFiles").int(0..=100),
        SettingRule::new("interface.uiScale").float(0.5..=3.0, 2),
        SettingRule::new("interface.canvasColor").choices(&[("gray", "Gray"), ("black", "Black"), ("custom", "Custom")]),
        SettingRule::new("interface.canvasCustomColor").color().shown_when("interface.canvasColor", "custom"),
        SettingRule::new("interface.systemTitleBar").restart(),
        SettingRule::new("interface.tooltips").group("Help"),
        SettingRule::new("interface.rulers").group("Help"),
        SettingRule::new("performance.historyStates").int(1..=1000).unit("states"),
        SettingRule::new("performance.cacheSize").int(64..=65536).unit("MB"),
        SettingRule::new("tools.smoothing").float(0.0..=1.0, 2),
        SettingRule::new("tools.brushShortcut").custom().keep_on_reset(),
        SettingRule::new("tools.legacyMode").hidden(),
    ]
}

type Document = SettingsDocument<Sample, BrushShortcut>;

#[allow(clippy::expect_used, reason = "the sample struct always serializes")]
fn document() -> Document {
    let pages = [StandardPage::General, StandardPage::Interface, StandardPage::Performance, StandardPage::Tools].map(PageInfo::from).to_vec();
    SettingsDocument::new(Sample::default()).expect("the sample serializes").pages(pages).rules(rules()).custom(BrushShortcut)
}

type Shared = Arc<Mutex<Document>>;

/// The window's body over the document; Apply writes the draft into the saved value and the line
/// under the body reports what the app would save.
fn show_document(ui: &mut Ui, args: &mut DocumentArgs) {
    let ctx = ui.ctx().clone();
    let key = ui.make_persistent_id("craft_ui::preferences::document-story");
    let model: Shared = ctx.data(|d| d.get_temp::<Shared>(key)).unwrap_or_else(|| Arc::new(Mutex::new(document())));
    let last_search: Option<String> = ctx.data(|d| d.get_temp(key.with("search")));
    let size = vec2(args.width.min(ui.available_width()), args.height);
    let mut model_guard = model.lock().unwrap_or_else(PoisonError::into_inner);
    ui.allocate_ui(size, |ui| {
        ui.set_min_size(size);
        if last_search.as_ref() != Some(&args.search) {
            let body = PreferencesBody::id(ui, "story-document");
            let mut state = PreferencesBody::state(&ctx, body);
            state.search.clone_from(&args.search);
            PreferencesBody::set_state(&ctx, body, state);
        }
        let _ = PreferencesBody::new("story-document").show(ui, &mut *model_guard);
    });
    if let Some(applied) = model_guard.take_applied() {
        ctx.data_mut(|d| d.insert_temp(key.with("applied"), format!("{applied:?}")));
    }
    drop(model_guard);
    if let Some(applied) = ctx.data(|d| d.get_temp::<String>(key.with("applied"))) {
        ui.label(format!("Applied: {applied}"));
    }
    ctx.data_mut(|d| {
        d.insert_temp(key, model);
        d.insert_temp(key.with("search"), args.search.clone());
    });
}
