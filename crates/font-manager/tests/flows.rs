//! The main flows against a small index, a font folder made per test and a fake font store.
//! `CRAFT_FONT_MANAGER_SNAPSHOTS=<dir>` also saves a screenshot; run that with
//! `-- --test-threads=1`, as rendering on several threads at once can crash older Vulkan loaders.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use craft_font_manager::craft_font_catalog::Os;
use craft_font_manager::index::Index;
use craft_font_manager::store::{Call, FakeStore};
use craft_font_manager::{Catalog, FontEnv, Source};
use craft_font_manager::{FontsPanel, Options, Runner, Settings, Tab, install_interface_fonts};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

const HACK: &str = "ofl/hack/Hack-Regular.ttf";
const UBUNTU: &str = "ufl/ubuntu/Ubuntu-Light.ttf";

fn index() -> Index {
    let file = |path: &str, size: usize| {
        format!(r#"{{"path":"{path}","size":{size},"sha256":"{}","style":"normal","weight":400,"axes":[],"postscript":[]}}"#, "0a".repeat(32))
    };
    let family = |name: &str, category: &str, languages: &str, scripts: &str, files: String| {
        format!(
            r#"{{"family":"{name}","designer":"","categories":["{category}"],"languages":[{languages}],"scripts":[{scripts}],"license":"OFL","license_path":null,"files":[{files}]}}"#
        )
    };
    let json = format!(
        r#"{{"schema":1,"version":"1.0","source":"google/fonts","commit":"45e3d5269e4a17d823f6d84470bc084dc5ae46f6","base_url":"https://cdn.jsdelivr.net/gh/google/fonts@45e3d5269e4a17d823f6d84470bc084dc5ae46f6/","fallback_base_url":"https://raw.githubusercontent.com/google/fonts/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/",
        "languages":[
          {{"id":"ar_Arab","name":"Arabic","autonym":"العربية","script":"Arab","population":1,"historical":false}},
          {{"id":"en_Latn","name":"English","autonym":"English","script":"Latn","population":1,"historical":false}},
          {{"id":"ja_Jpan","name":"Japanese","autonym":"日本語","script":"Jpan","population":1,"historical":false}}],
        "scripts":[{{"id":"Arab","name":"Arabic"}},{{"id":"Jpan","name":"Japanese"}},{{"id":"Latn","name":"Latin"}}],
        "families":[{},{},{},{}]}}"#,
        family("Amiri", "SERIF", "0,1", r#""Arab","Latn""#, file("ofl/amiri/Amiri-Regular.ttf", 1000)),
        family("Hack", "MONOSPACE", "1", r#""Latn""#, file(HACK, epaint_default_fonts::HACK_REGULAR.len())),
        family("Noto Sans JP", "SANS_SERIF", "1,2", r#""Jpan","Latn""#, file("ofl/notosansjp/NotoSansJP[wght].ttf", 1000)),
        family("Ubuntu", "SANS_SERIF", "1", r#""Latn""#, file(UBUNTU, epaint_default_fonts::UBUNTU_LIGHT.len())),
    );
    Index::from_json(json.as_bytes()).unwrap()
}

/// A system with Ubuntu Light installed, an empty font store, and a fake store that can "download"
/// Hack and Ubuntu Light.
fn setup(name: &str, settings: Settings) -> (Harness<'static, FontsPanel>, Arc<FakeStore>) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("flows").join(name);
    let _ = std::fs::remove_dir_all(&root);
    let system = root.join("usr/share/fonts");
    std::fs::create_dir_all(&system).unwrap();
    std::fs::write(system.join("Ubuntu-Light.ttf"), epaint_default_fonts::UBUNTU_LIGHT).unwrap();
    let store_dir = root.join("store");
    std::fs::create_dir_all(&store_dir).unwrap();
    let env = FontEnv { os: Os::Unix, root: root.clone(), font_store: Some(store_dir.clone()), ..FontEnv::default() };
    let catalog = Arc::new(Catalog::new(env).with_font_service(false));
    catalog.rescan();
    let files = HashMap::from([
        (HACK.to_owned(), epaint_default_fonts::HACK_REGULAR.to_vec()),
        (UBUNTU.to_owned(), epaint_default_fonts::UBUNTU_LIGHT.to_vec()),
    ]);
    let store = Arc::new(FakeStore::new(store_dir, files));
    let options = Options { catalog, installer: store.clone(), index: Ok(index()), runner: Runner::Inline };
    let app = FontsPanel::new(options, settings);
    let mut harness = Harness::builder()
        .with_size([1200.0, 760.0])
        // Autonyms such as العربية need fonts beyond egui's; the tests don't depend on the host's.
        .allow_missing_glyphs()
        .with_context_setup(|ctx| install_interface_fonts(ctx, &[]))
        .build_ui_state(|ui, app: &mut FontsPanel| app.ui(ui), app);
    harness.run_steps(4);
    (harness, store)
}

fn settle(h: &mut Harness<'_, FontsPanel>) {
    h.run_steps(4);
}

fn click(h: &mut Harness<'_, FontsPanel>, label: &str) {
    h.get_all_by_label(label).next().unwrap_or_else(|| panic!("no {label:?}")).click();
    settle(h);
}

fn type_into(h: &mut Harness<'_, FontsPanel>, label: &str, text: &str) {
    h.get_by_role_and_label(Role::TextInput, label).focus();
    h.step();
    h.get_by_role_and_label(Role::TextInput, label).type_text(text);
    settle(h);
}

fn catalogue(h: &mut Harness<'_, FontsPanel>) {
    click(h, "Catalogue");
    assert_eq!(h.state().tab(), Tab::Catalogue);
}

#[test]
fn search_and_category_filter_the_catalogue() {
    let (mut h, _) = setup("search", Settings::default());
    catalogue(&mut h);
    assert_eq!(h.state().results(), ["Amiri", "Hack", "Noto Sans JP", "Ubuntu"]);

    type_into(&mut h, "Search", "noto");
    assert_eq!(h.state().results(), ["Noto Sans JP"]);

    type_into(&mut h, "Search", "XYZ");
    assert!(h.state().results().is_empty());
    assert!(h.query_by_label("No fonts match.").is_some());

    let mut h = setup("category", Settings::default()).0;
    catalogue(&mut h);
    click(&mut h, "Category");
    click(&mut h, "Serif");
    assert_eq!(h.state().results(), ["Amiri"]);
}

#[test]
fn language_filter_finds_languages_by_their_own_name() {
    let (mut h, _) = setup("language", Settings::default());
    catalogue(&mut h);
    click(&mut h, "Language");
    type_into(&mut h, "Find a language", "日本");
    assert!(h.query_by_label("Arabic · العربية").is_none());
    click(&mut h, "Japanese · 日本語");
    assert_eq!(h.state().results(), ["Noto Sans JP"]);
}

#[test]
fn install_through_the_store_updates_the_installed_list_and_remove_takes_it_out() {
    let (mut h, store) = setup("install", Settings::default());
    assert!(!h.state().installed().iter().any(|f| f.name == "Hack"));

    catalogue(&mut h);
    click(&mut h, "Hack");
    click(&mut h, "Install");
    click(&mut h, "Download");
    assert_eq!(store.calls(), [Call::Install("Hack".into())]);
    assert!(h.state().downloads_allowed());
    let hack = h.state().installed().iter().find(|f| f.name == "Hack").cloned().expect("Hack is listed");
    assert_eq!(hack.source, Source::Store);
    assert!(h.query_by_label("Hack is installed.").is_some());

    click(&mut h, "Installed");
    click(&mut h, "Hack");
    let remove = h.get_by_label("Remove");
    assert!(!remove.accesskit_node().is_disabled());
    remove.click();
    settle(&mut h);
    assert_eq!(store.calls().last(), Some(&Call::Remove("Hack".into())));
    assert!(!h.state().installed().iter().any(|f| f.name == "Hack"));
}

#[test]
fn system_fonts_cannot_be_removed() {
    let (mut h, store) = setup("system", Settings::default());
    let system = h.state().installed().first().cloned().expect("the system font is listed");
    assert_eq!(system.source, Source::System);
    click(&mut h, &system.name);
    let remove = h.get_by_label("Remove");
    assert!(remove.accesskit_node().is_disabled());
    remove.click();
    settle(&mut h);
    assert!(store.calls().is_empty());
}

#[test]
fn nothing_downloads_without_consent() {
    let (mut h, store) = setup("consent", Settings::default());
    catalogue(&mut h);
    click(&mut h, "Hack");

    click(&mut h, "Preview downloads this font");
    assert!(h.query_by_label("Download from Google Fonts?").is_some());
    click(&mut h, "Cancel");
    click(&mut h, "Install");
    click(&mut h, "Cancel");
    assert!(store.calls().is_empty());
    assert!(!h.state().downloads_allowed());
    assert!(h.query_by_label("Download from Google Fonts?").is_none());

    // With the switch on, a preview downloads without asking and installs nothing.
    click(&mut h, "Allow downloads");
    click(&mut h, "Preview downloads this font");
    assert_eq!(store.calls(), [Call::Download("Hack".into())]);
    assert!(!h.state().installed().iter().any(|f| f.name == "Hack"));
}

#[test]
fn screenshot() {
    let Some(dir) = std::env::var_os("CRAFT_FONT_MANAGER_SNAPSHOTS") else { return };
    let (mut h, _) = setup("screenshot", Settings::default());
    click(&mut h, "Allow downloads");
    catalogue(&mut h);
    click(&mut h, "Hack");
    click(&mut h, "Preview downloads this font");
    h.run_steps(12);
    let image = h.render().unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    image.save(PathBuf::from(dir).join("craft-font-manager-catalogue.png")).unwrap();
}
