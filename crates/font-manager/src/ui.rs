//! The story of [`FontsPanel`](crate::FontsPanel).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use craft_font_catalog::{Limits, Os};
use craft_host::{AppInfo, Host, InlineJobs, MemorySections};
use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

use crate::index::Index;
use crate::store::FakeStore;
use crate::{Catalog, FontEnv, FontsPanel, Options, Source};

/// The components the stories cover.
pub const COMPONENTS: &[&str] = &["Fonts panel"];

const APP: AppInfo = AppInfo { id: "fonts-story", name: "Fonts story", version: "1.0" };

/// The stories of the font manager's panel.
pub fn stories() -> Vec<Story> {
    vec![typed_story!("patterns/fonts-panel/panel", Level::Patterns, "Fonts panel", "Panel", NoArgs {}, show)]
}

/// Four families of the Google Fonts index; the panel's store is a fake that refuses downloads.
#[allow(clippy::panic, reason = "the story's own fixed index is valid")]
fn index() -> Index {
    let file = |path: &str| {
        format!(r#"{{"path":"{path}","size":1000,"sha256":"{}","style":"normal","weight":400,"axes":[],"postscript":[]}}"#, "0a".repeat(32))
    };
    let family = |name: &str, category: &str, languages: &str, scripts: &str, path: &str| {
        format!(
            r#"{{"family":"{name}","designer":"","categories":["{category}"],"languages":[{languages}],"scripts":[{scripts}],"license":"OFL","license_path":null,"files":[{}]}}"#,
            file(path)
        )
    };
    let json = format!(
        r#"{{"schema":1,"version":"1.0","source":"google/fonts","commit":"45e3d5269e4a17d823f6d84470bc084dc5ae46f6","base_url":"https://cdn.jsdelivr.net/gh/google/fonts@45e3d5269e4a17d823f6d84470bc084dc5ae46f6/","fallback_base_url":"https://raw.githubusercontent.com/google/fonts/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/",
        "languages":[
          {{"id":"en_Latn","name":"English","autonym":"English","script":"Latn","population":1,"historical":false}},
          {{"id":"ja_Jpan","name":"Japanese","autonym":"日本語","script":"Jpan","population":1,"historical":false}}],
        "scripts":[{{"id":"Jpan","name":"Japanese"}},{{"id":"Latn","name":"Latin"}}],
        "families":[{},{},{},{}]}}"#,
        family("Amiri", "SERIF", "1", r#""Latn""#, "ofl/amiri/Amiri-Regular.ttf"),
        family("Hack", "MONOSPACE", "0", r#""Latn""#, "ofl/hack/Hack-Regular.ttf"),
        family("Noto Sans JP", "SANS_SERIF", "0,1", r#""Jpan","Latn""#, "ofl/notosansjp/NotoSansJP[wght].ttf"),
        family("Ubuntu", "SANS_SERIF", "0", r#""Latn""#, "ufl/ubuntu/Ubuntu-Light.ttf"),
    );
    Index::from_json(json.as_bytes()).unwrap_or_else(|e| panic!("the story index: {e}"))
}

/// The panel over an in-memory catalog (two bundled faces, no folders scanned) and the fixed index.
fn panel() -> FontsPanel {
    let catalog = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
    for bytes in [epaint_default_fonts::UBUNTU_LIGHT, epaint_default_fonts::HACK_REGULAR] {
        let _ = catalog.add_bytes(bytes.to_vec(), Source::Bundled, &Limits::default());
    }
    let store = FakeStore::new(std::env::temp_dir().join("craft-fonts-panel-story"), HashMap::new());
    FontsPanel::new(Options { catalog, installer: Arc::new(store), index: Ok(index()) })
}

fn show(ui: &mut Ui, _: &mut NoArgs) {
    if Host::try_of(ui).is_none() {
        Host::builder(APP).jobs(Arc::new(InlineJobs)).settings(Arc::new(MemorySections::default())).install(ui.ctx());
    }
    let key = ui.scope_id().with("fonts-panel-story");
    let shared: Arc<Mutex<FontsPanel>> = ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(panel()))).clone());
    let mut panel = shared.lock().unwrap_or_else(PoisonError::into_inner);
    ui.set_min_height(480.0);
    panel.ui(ui);
}
