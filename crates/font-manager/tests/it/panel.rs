//! The panel's flows through `FontsPanelView`, against a small index, a font folder made per test and a fake font store.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use craft_font_manager::craft_font_catalog::Os;
use craft_font_manager::index::Index;
use craft_font_manager::store::{Call, FakeStore, Installer};
use craft_font_manager::{Catalog, FontEnv, FontSources, FontsPanel, FontsPanelSettings, FontsPanelView, Options, Source, Tab};
use craft_host::{AppInfo, Host, InlineJobs, Jobs, MemorySections};
use craft_ui::contract::{Role, ShowContainer as _};
use craft_ui::layout_components::Row;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Pos2, Vec2};

/// The room the panel is shown in, and a wider one where the details sheet and the sources need more.
const ROOM: Vec2 = Vec2::new(1120.0, 678.0);
const WIDE_ROOM: Vec2 = Vec2::new(1400.0, 760.0);

const HACK: &str = "ofl/hack/Hack-Regular.ttf";
const UBUNTU: &str = "ufl/ubuntu/Ubuntu-Light.ttf";
const APP: AppInfo = AppInfo { id: "fonts-test", name: "Fonts test", version: "1.0" };
const USER_INDEX: &str = "https://fonts.example.org/index.json";

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

/// A Host with `sections` as its settings and inline jobs.
fn host(sections: Arc<MemorySections>) -> Arc<Host> {
    Arc::new(Host::builder(APP).settings(sections).jobs(Arc::new(InlineJobs) as Arc<dyn Jobs>).build())
}

struct Rig {
    fx: Fixture,
    panel: Rc<RefCell<FontsPanel>>,
    store: Arc<FakeStore>,
}

fn root(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("panel").join(name);
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// A system with Ubuntu Light installed, an empty font store, and a fake store that can "download" Hack and Ubuntu
/// Light.
fn setup(name: &str) -> Rig {
    setup_with(name, Arc::new(MemorySections::default()))
}

fn setup_with(name: &str, sections: Arc<MemorySections>) -> Rig {
    let root = root(name);
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
    let panel = Rc::new(RefCell::new(FontsPanel::new(Options::new(catalog, store.clone(), Ok(index())))));
    let shown = Rc::clone(&panel);
    let mut fx = Fixture::new(move |ui| {
        FontsPanelView::new(&mut shown.borrow_mut()).show(ui);
    });
    fx.room(ROOM).host(host(sections));
    fx.settle();
    Rig { fx, panel, store }
}

impl Rig {
    fn named(&self, role: Role, name: &str) -> Target {
        self.fx
            .by_role(role)
            .into_iter()
            .find(|target| self.fx.tree().name_of(target.node).is_some_and(|found| found == name || found.starts_with(&format!("{name},"))))
            .unwrap_or_else(|| panic!("no {role:?} named `{name}` on screen"))
    }

    fn click_role(&mut self, role: Role, name: &str) {
        let target = self.named(role, name);
        self.fx.click(&target);
        self.fx.settle();
    }

    /// Presses the control named `name`.
    fn click(&mut self, name: &str) {
        let target = self.fx.control(name).unwrap_or_else(|| panic!("no control `{name}` on screen"));
        self.fx.click(&target);
        self.fx.settle();
    }

    fn type_into(&mut self, name: &str, text: &str) {
        let target = self.fx.control(name).unwrap_or_else(|| panic!("no field `{name}` on screen"));
        self.fx.click(&target);
        self.fx.type_text(text);
        self.fx.settle();
    }

    fn key(&mut self, key: Key) {
        self.fx.key(key, Modifiers::NONE);
        self.fx.settle();
    }

    fn present(&self, text: &str) -> bool {
        self.fx.by_label(text).is_some() || self.fx.texts().iter().any(|painted| painted.text == text)
    }

    fn allow(&mut self) {
        self.panel.borrow_mut().set_downloads_allowed(true).unwrap();
        self.fx.settle();
    }

    fn installed_tab(&mut self) {
        self.click_role(Role::TAB, "Installed");
        assert_eq!(self.panel.borrow().tab(), Tab::Installed);
    }

    fn has_installed(&self, family: &str) -> bool {
        self.panel.borrow().installed().iter().any(|f| f.name == family)
    }

    fn results(&self) -> Vec<String> {
        self.panel.borrow().results()
    }

    /// Opens the details of the card named `family`: the card takes the focus, Enter opens it.
    fn open(&mut self, family: &str) {
        let card = self.named(Role::LIST_ITEM, family);
        self.fx.click_at(Pos2::new(card.rect.left() + 4.0, card.rect.top() + 4.0));
        self.key(Key::Enter);
        assert_eq!(self.panel.borrow().details_family().as_deref(), Some(family));
    }
}

#[trial]
fn search_and_filters_narrow_the_catalogue() {
    let mut rig = setup("search");
    assert_eq!(rig.panel.borrow().tab(), Tab::Catalogue);
    assert_eq!(rig.results(), ["Amiri", "Hack", "Noto Sans JP", "Ubuntu"]);

    rig.type_into("Search", "noto");
    assert_eq!(rig.results(), ["Noto Sans JP"]);

    rig.type_into("Search", "XYZ");
    assert!(rig.results().is_empty());
    assert!(rig.present("No fonts match"));
    rig.click("Clear search and filters");
    assert_eq!(rig.results().len(), 4);

    rig.click_role(Role::RADIO_BUTTON, "Serif");
    assert_eq!(rig.results(), ["Amiri"]);
    rig.click_role(Role::RADIO_BUTTON, "All categories");

    // Ubuntu Light is installed on the system.
    rig.click_role(Role::RADIO_BUTTON, "Installed");
    assert_eq!(rig.results(), ["Ubuntu"]);
    rig.click_role(Role::RADIO_BUTTON, "Not installed");
    assert_eq!(rig.results(), ["Amiri", "Hack", "Noto Sans JP"]);
    rig.click("Clear filters");
    assert_eq!(rig.results().len(), 4);
}

#[trial]
fn settings_changes_land_in_the_hosts_section_and_a_new_panel_reads_them() {
    let sections = Arc::new(MemorySections::default());
    let mut rig = setup_with("settings", Arc::clone(&sections));
    assert_eq!(rig.panel.borrow().settings(), &FontsPanelSettings::default());
    rig.type_into("Sample text", "Hamburgefonstiv");
    assert_eq!(rig.panel.borrow().settings().sample, "Hamburgefonstiv");
    let saved: FontsPanelSettings = host(Arc::clone(&sections)).setting();
    assert_eq!(saved.sample, "Hamburgefonstiv");

    let again = setup_with("settings-again", sections);
    assert_eq!(again.panel.borrow().settings().sample, "Hamburgefonstiv");
}

#[trial]
fn language_filter_finds_languages_by_their_own_name() {
    let mut rig = setup("language");
    rig.type_into("Find a language", "日本");
    assert!(!rig.present("Arabic · العربية"));
    rig.click_role(Role::LIST_ITEM, "Japanese · 日本語");
    assert_eq!(rig.results(), ["Noto Sans JP"]);
}

#[trial]
fn install_a_family_then_remove_it_from_its_card() {
    let mut rig = setup("install");
    assert!(!rig.has_installed("Hack"));

    rig.open("Hack");
    rig.click("Install all");
    rig.click("Download");
    assert_eq!(rig.store.calls(), [Call::Install("Hack".into())]);
    assert!(rig.panel.borrow().downloads_allowed());
    let hack = rig.panel.borrow().installed().iter().find(|f| f.name == "Hack").cloned().expect("Hack is listed");
    assert_eq!(hack.source, Source::Store);
    assert!(rig.present("Remove all"));

    rig.key(Key::Escape);
    assert!(rig.panel.borrow().details_family().is_none());
    rig.installed_tab();
    rig.type_into("Search", "Hack");
    rig.click("Remove");
    assert_eq!(rig.store.calls().last(), Some(&Call::Remove("Hack".into())));
    assert!(!rig.has_installed("Hack"));
}

#[trial]
fn install_and_remove_one_style() {
    let mut rig = setup("style");
    rig.allow();
    rig.open("Hack");
    rig.click("Install Regular");
    assert_eq!(rig.store.calls(), [Call::InstallFile("Hack".into(), HACK.into())]);
    assert!(rig.has_installed("Hack"));

    rig.click("Remove Regular");
    assert_eq!(rig.store.calls().last(), Some(&Call::Remove("Hack".into())));
    assert!(!rig.has_installed("Hack"));
}

#[trial]
fn system_fonts_cannot_be_removed() {
    let mut rig = setup("system");
    rig.installed_tab();
    let system = rig.panel.borrow().installed().first().cloned().expect("the system font is listed");
    assert_eq!(system.source, Source::System);
    assert!(rig.fx.control("Remove").is_none(), "a system family has no Remove on its card");
    rig.open(&system.name);
    let remove = rig.fx.control("Remove all").expect("the sheet offers Remove all");
    assert!(rig.fx.tree().is_disabled_key(remove.node));
    rig.fx.click(&remove);
    rig.fx.settle();
    assert!(rig.store.calls().is_empty());
}

#[trial]
fn nothing_downloads_without_consent() {
    let mut rig = setup("consent");
    assert!(rig.present("Downloads are off"));
    rig.open("Hack");

    rig.click("Install all");
    assert!(rig.present("Download from Google Fonts?"));
    rig.click("Cancel");
    assert!(rig.store.calls().is_empty());
    assert!(!rig.panel.borrow().downloads_allowed());
    assert!(!rig.present("Download from Google Fonts?"));
    rig.key(Key::Escape);

    // Allowed from the note, a preview downloads without asking and installs nothing.
    rig.type_into("Search", "Hack");
    rig.click_role(Role::BUTTON, "Allow downloads");
    assert!(rig.panel.borrow().downloads_allowed());
    assert!(!rig.present("Downloads are off"));
    rig.click("Preview");
    assert_eq!(rig.store.calls(), [Call::Download("Hack".into())]);
    assert!(!rig.has_installed("Hack"));
}

#[trial]
fn a_failed_install_offers_retry() {
    let mut rig = setup("retry");
    rig.allow();
    rig.type_into("Search", "Hack");
    rig.store.fail_next("network unreachable");
    rig.click("Install");
    assert!(rig.present("Couldn't install"));
    assert!(!rig.has_installed("Hack"));
    rig.click("Retry");
    assert!(rig.has_installed("Hack"));
    assert!(!rig.present("Couldn't install"));
}

#[trial]
fn arrow_keys_move_between_cards_and_enter_opens_one() {
    let mut rig = setup("keys");
    let first = rig.named(Role::LIST_ITEM, "Amiri");
    rig.fx.click_at(Pos2::new(first.rect.left() + 4.0, first.rect.top() + 4.0));
    rig.fx.settle();
    rig.key(Key::ArrowDown);
    rig.key(Key::Enter);
    assert_eq!(rig.panel.borrow().details_family().as_deref(), Some("Hack"));
}

/// A source a user could add: another "Hack", from an index of its own.
fn user_index() -> Vec<u8> {
    format!(
        r#"{{"schema":1,"version":"1.1","source":"examplefonts","commit":"{commit}",
        "source_info":{{"id":"examplefonts","name":"Example Fonts","curated":true,"file_urls":["https://fonts.example.org/files/{{path}}"]}},
        "languages":[{{"id":"en_Latn","name":"English","script":"Latn","population":1}}],
        "scripts":[{{"id":"Latn","name":"Latin"}}],
        "families":[{{"family":"Hack","source":"examplefonts","categories":["MONOSPACE"],"license":"OFL","license_path":null,"languages":[0],"scripts":["Latn"],
          "files":[{{"path":"{HACK}","size":{size},"sha256":"{sha}","style":"normal","weight":400,"axes":[],"postscript":[]}}]}}]}}"#,
        commit = "1".repeat(40),
        size = epaint_default_fonts::HACK_REGULAR.len(),
        sha = "0a".repeat(32),
    )
    .into_bytes()
}

/// The panel with its sources beside it, as a host's settings show them.
fn sources_setup(name: &str) -> Rig {
    let root = root(name);
    let store_dir = root.join("store");
    std::fs::create_dir_all(&store_dir).unwrap();
    let env = FontEnv { os: Os::Unix, root: root.clone(), font_store: Some(store_dir.clone()), ..FontEnv::default() };
    let catalog = Arc::new(Catalog::new(env).with_font_service(false));
    catalog.rescan();
    let store = Arc::new(FakeStore::new(store_dir, HashMap::new()).with_index(index()).serving_index(USER_INDEX, user_index()));
    store.allow_downloads(true).unwrap();
    let options = Options::new(catalog, store.clone(), store.indexes().map_err(|e| e.0));
    let panel = Rc::new(RefCell::new(FontsPanel::new(options)));
    let shown = Rc::clone(&panel);
    let mut fx = Fixture::new(move |ui| {
        let mut panel = shown.borrow_mut();
        Row::new().fill().show(ui, |ui, _| {
            FontSources::new(&mut panel).show(ui);
            FontsPanelView::new(&mut panel).show(ui);
        });
    });
    fx.room(WIDE_ROOM).host(host(Arc::new(MemorySections::default())));
    fx.settle();
    Rig { fx, panel, store }
}

impl Rig {
    /// Frames until `done`, for work on the panel's own threads.
    fn wait(&mut self, done: impl Fn(&FontsPanel) -> bool) {
        for _ in 0..200 {
            if done(&self.panel.borrow()) {
                self.fx.settle();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            self.fx.idle(1);
        }
        panic!("timed out");
    }

    fn count_named(&self, name: &str) -> usize {
        self.fx.tree().keys().into_iter().filter(|key| self.fx.tree().name_of(*key) == Some(name)).count()
    }
}

#[trial]
fn an_added_source_is_not_curated_and_can_be_removed() {
    let mut rig = sources_setup("sources");
    assert_eq!(rig.results().len(), 4);
    rig.type_into("Index address", USER_INDEX);
    rig.click_role(Role::BUTTON, "Add");
    rig.wait(|panel| panel.results().len() == 5);
    // Two families named Hack, one from each source, each with its own card.
    assert_eq!(rig.results().iter().filter(|family| *family == "Hack").count(), 2);
    assert!(rig.count_named("Not curated") >= 2, "a badge on the card and one in the sources");
    assert!(!rig.present("Changed since added"));

    rig.store.change_index(USER_INDEX);
    rig.click("Update sources");
    rig.wait(|panel| !panel.fetching_sources());
    assert!(rig.present("Changed since added"));

    let remove = rig
        .fx
        .tree()
        .keys()
        .into_iter()
        .find(|key| rig.fx.tree().name_of(*key).is_some_and(|name| name.contains("Remove") && name.contains(USER_INDEX)))
        .and_then(|key| rig.fx.target(key))
        .expect("the source offers Remove");
    rig.fx.click(&remove);
    rig.fx.settle();
    assert_eq!(rig.results().len(), 4);
    assert!(rig.store.user_sources().is_empty());
}
