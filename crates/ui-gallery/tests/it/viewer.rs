//! The viewer's own logic: what it saves and restores, and how the filter narrows the component tree.

use craft_behavior::Selection;
use craft_settings::{Location, OpenOptions, SettingsFiles, SettingsStore};
use craft_theme::ModeChoice;
use craft_ui::contract::{Level, Registration};
use craft_ui::list::OpenSet;
use craft_ui::testing::trial;
use craft_ui_gallery::viewer::{
    Catalog, ComponentInfo, GalleryCommands, GallerySettings, PanelId, TreeKey, Viewer, Workspace, by_level, default_for, load_layout, open_tabs,
    rows, save_layout,
};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Settings files held in memory: the format the app writes, without a disk.
#[derive(Default)]
struct MemoryFiles(Mutex<BTreeMap<PathBuf, String>>);

impl SettingsFiles for MemoryFiles {
    fn read(&self, path: &Path) -> io::Result<Option<String>> {
        Ok(self.0.lock().unwrap_or_else(PoisonError::into_inner).get(path).cloned())
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(path.to_path_buf(), text.to_owned());
        Ok(())
    }

    fn move_aside(&self, path: &Path) -> io::Result<PathBuf> {
        Ok(path.to_path_buf())
    }

    fn write_backup(&self, _: &Path, _: &str) -> io::Result<()> {
        Ok(())
    }

    fn check_writable(&self, _: &Path) -> io::Result<()> {
        Ok(())
    }
}

fn components() -> Vec<ComponentInfo> {
    vec![
        ComponentInfo::new(
            "craft_ui::buttons::button::Button<'_>",
            Level::Controls,
            "Button",
            &[("button", "Default"), ("button_long_text", "Long text")],
        ),
        ComponentInfo::new("craft_ui::buttons::link::Link", Level::Controls, "Link", &[("link", "Default")]),
        ComponentInfo::new("craft_ui::display::text::Text", Level::Primitives, "text", &[("text", "Default")]),
        ComponentInfo::new("craft_ui::value_drag::AngleDial", Level::Controls, "AngleDial", &[("dial", "Default")]),
    ]
}

fn viewer(settings: &GallerySettings) -> Viewer {
    Viewer::new(Catalog::listing(components()), settings)
}

const COMPONENTS: [PanelId; 6] = [PanelId::Components, PanelId::Stories, PanelId::Properties, PanelId::Inspect, PanelId::Overview, PanelId::Tokens];
const THEME: [PanelId; 5] = [PanelId::Preview, PanelId::Detail, PanelId::Problems, PanelId::AppSizes, PanelId::Tokens];

/// Regression: a workspace's default layout that held another workspace's panel, or lost one of its own, opened the
/// gallery with a missing or foreign panel.
#[trial]
fn each_default_layout_is_valid_and_holds_exactly_its_workspaces_panels() {
    for (workspace, panels) in [(Workspace::Components, &COMPONENTS[..]), (Workspace::Theme, &THEME[..])] {
        let layout = default_for(workspace);
        assert_eq!(layout.validate(), Ok(()));
        for panel in COMPONENTS.iter().chain(&THEME) {
            assert_eq!(layout.layout.contains(panel), panels.contains(panel), "{workspace:?} {panel:?}");
        }
    }
}

/// Regression: a saved layout naming a component the catalog lacks, or lacking a default panel, must still load.
#[trial]
fn a_saved_layout_loads_without_the_components_the_catalog_lacks_and_with_every_default_panel() {
    let mut saved = default_for(Workspace::Components);
    let _ = saved.set_visible(PanelId::Component("craft_ui::buttons::link::Link"), true, Some(PanelId::Overview), false, |panel| panel.permissions());
    let _ = saved.set_visible(PanelId::Component("gone::Removed"), true, Some(PanelId::Overview), false, |panel| panel.permissions());
    let _ = saved.set_visible(PanelId::Stories, false, None, false, |_| craft_behavior::dock::Permissions::ALL);
    assert!(!saved.layout.contains(&PanelId::Stories));
    let loaded = load_layout(&save_layout(&saved), Workspace::Components, &|name| name == "craft_ui::buttons::link::Link");
    assert_eq!(loaded.validate(), Ok(()));
    assert_eq!(open_tabs(&loaded), ["craft_ui::buttons::link::Link"]);
    assert!(loaded.hidden.iter().all(|(panel, _)| !matches!(panel, PanelId::Component(_))));
    for panel in COMPONENTS {
        assert!(loaded.layout.contains(&panel), "{panel:?}");
    }
    let unreadable = load_layout("{\"layout\":", Workspace::Theme, &|_| true);
    assert_eq!(unreadable, default_for(Workspace::Theme));
}

/// Regression: Reset layout closed the component tabs the user had open.
#[trial]
fn reset_layout_keeps_the_open_component_tabs() {
    let mut viewer = viewer(&GallerySettings::default());
    viewer.open_component("craft_ui::buttons::link::Link");
    let _ = viewer.browser.dock.set_visible(PanelId::Stories, false, None, false, |_| craft_behavior::dock::Permissions::ALL);
    viewer.run(GalleryCommands::ResetLayout);
    assert_eq!(open_tabs(&viewer.browser.dock), ["craft_ui::buttons::link::Link"]);
    for panel in COMPONENTS {
        assert!(viewer.browser.dock.layout.contains(&panel), "{panel:?}");
    }
}

#[trial]
fn settings_restore_the_state_they_were_saved_from() {
    let mut before = viewer(&GallerySettings::default());
    before.env.mode = ModeChoice::System;
    before.browser.filter = "u".to_owned();
    before.browser.selection = Selection::single(TreeKey::Component("craft_ui::buttons::link::Link"));
    before.browser.open.set(&TreeKey::Level(Level::Controls), true);
    before.workspace = Workspace::Theme;
    before.open_component("craft_ui::buttons::link::Link");
    before.browser.level = "controls".to_owned();
    before.browser.story_of.insert("craft_ui::buttons::button::Button<'_>", "button_long_text");
    let saved = GallerySettings::of(&before);
    let files = Arc::new(MemoryFiles::default());
    let options = || OpenOptions::new(Location::Dir(PathBuf::from("settings"))).files(files.clone());
    let (mut store, _) = SettingsStore::<GallerySettings>::open_with(options());
    assert!(store.set(saved.clone()));
    store.flush().expect("the store writes its file");
    let (reopened, _) = SettingsStore::<GallerySettings>::open_with(options());
    let after = viewer(reopened.get());
    assert_eq!(GallerySettings::of(&after), saved);
    assert_eq!(after.selected_type(), Some("craft_ui::buttons::link::Link"));
    assert_eq!(after.open_levels(), vec!["controls".to_owned()]);
    assert_eq!(after.workspace, Workspace::Theme);
    assert_eq!(open_tabs(&after.browser.dock), ["craft_ui::buttons::link::Link"]);
    assert_eq!(after.browser.level, "controls");
}

/// The tree lists the levels in order and each level's components by plain name ignoring case; a filter opens the
/// levels that hold a match and leaves the remembered open set as it was.
#[trial]
fn the_tree_lists_levels_in_order_and_the_filter_keeps_the_open_set() {
    let mut open = OpenSet::new();
    let labels = |rows: &[craft_ui_gallery::viewer::TreeRow]| rows.iter().map(|row| row.label.clone()).collect::<Vec<_>>();
    assert_eq!(labels(&rows(&components(), "", &open)), ["Primitives", "Controls"], "closed levels show alone");

    open.set(&TreeKey::Level(Level::Controls), true);
    assert_eq!(labels(&rows(&components(), "", &open)), ["Primitives", "Controls", "AngleDial", "Button", "Link"]);

    let by_story = rows(&components(), "long text", &open);
    assert_eq!(labels(&by_story), ["Controls", "Button"]);

    let by_name = rows(&components(), "TEXT", &open);
    assert_eq!(labels(&by_name), ["Primitives", "text", "Controls", "Button"], "a filter opens every level with a match");
    assert!(!open.is_open(&TreeKey::Level(Level::Primitives)), "and leaves the open set alone");

    assert!(rows(&components(), "no such component", &open).is_empty());
}

/// A crate with no registrations here was linked out, which in a release web build loses its stories.
#[trial]
fn every_linked_crate_registers() {
    let (counts, stories) = craft_ui_gallery::linked_counts();
    assert!(stories > 0);
    let empty: Vec<_> = counts.iter().filter(|(_, count)| *count == 0).collect();
    assert!(empty.is_empty(), "no registrations from {empty:?}");
}

/// Regression: the Overview and the filter listed only craft-ui's types, so every domain crate's component had no
/// card, no module and no story tab.
#[trial]
fn domain_crate_types_have_an_overview_entry_and_a_declaration() {
    let _ = craft_ui_gallery::linked_counts();
    let catalog = Catalog::linked();
    let listed: Vec<&str> = by_level(&catalog, "").into_iter().flat_map(|(_, components)| components).map(|component| component.type_name).collect();
    let domain: Vec<&'static str> = craft_ui::contract::registrations()
        .into_iter()
        .filter(|registration| registration.crate_name() != "craft-ui")
        .filter_map(|registration| match registration {
            Registration::Component(probe) => Some(probe.type_name()),
            Registration::Pattern(probe) => Some(probe.type_name()),
            _ => None,
        })
        .filter(|type_name| catalog.entries.iter().any(|entry| entry.type_name == *type_name))
        .collect();
    assert!(!domain.is_empty(), "no domain-crate type with a story is linked");
    for type_name in domain {
        assert!(listed.contains(&type_name), "{type_name} has no Overview entry");
        assert!(catalog.declared.contains_key(type_name), "{type_name} has no declaration");
    }
}

/// Regression: the Overview lacked the Foundations level and the old gallery's domain composites, and a generic
/// registration (`PresetBrowser<'_, u32>`) must be listed under its plain name.
#[trial]
fn levels_list_the_old_galleries_foundations_and_composites() {
    let _ = craft_ui_gallery::linked_counts();
    let catalog = Catalog::linked();
    let groups = by_level(&catalog, "");
    let names = |level: craft_ui::contract::Level| -> Vec<String> {
        groups
            .iter()
            .filter(|(shown, _)| *shown == level)
            .flat_map(|(_, components)| components.iter().map(|component| component.name.clone()))
            .collect()
    };
    assert!(names(craft_ui::contract::Level::Foundations).contains(&"TokenBrowser".to_owned()));
    let composites = names(craft_ui::contract::Level::Composites);
    for expected in ["AxisSliders", "BlendIfBar", "BlendRows", "BlurPinsOverlay", "PresetBrowser"] {
        assert!(composites.contains(&expected.to_owned()), "{expected} is not listed under Composites");
    }
}
