//! The viewer's own logic: what it saves and restores, and how the filter narrows the component tree.

use craft_behavior::Selection;
use craft_settings::{Location, OpenOptions, SettingsFiles, SettingsStore};
use craft_theme::ModeChoice;
use craft_ui::list::OpenSet;
use craft_ui::testing::trial;
use craft_ui_gallery::viewer::{
    Catalog, ComponentInfo, GalleryCommands, GallerySettings, PanelId, TreeKey, Viewer, Workspace, default_for, load_layout, open_tabs, rows,
    save_layout,
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
        ComponentInfo::new("craft_ui::buttons::button::Button<'_>", "craft_ui::buttons::button", vec!["button", "button_long_text"]),
        ComponentInfo::new("craft_ui::buttons::link::Link", "craft_ui::buttons::link", vec!["link"]),
        ComponentInfo::new("craft_ui::display::text::Text", "craft_ui::display::text", vec!["text"]),
        ComponentInfo::new("crate::Undeclared", "", vec!["undeclared"]),
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
    before.browser.open.set(&TreeKey::Module("craft_ui::buttons::link"), true);
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
    assert_eq!(after.open_modules(), vec!["craft_ui::buttons::link".to_owned()]);
    assert_eq!(after.workspace, Workspace::Theme);
    assert_eq!(open_tabs(&after.browser.dock), ["craft_ui::buttons::link::Link"]);
    assert_eq!(after.browser.level, "controls");
}

#[trial]
fn the_filter_shows_the_groups_that_hold_a_match_open() {
    let closed = OpenSet::new();
    let all = rows(&components(), "", &closed);
    assert_eq!(all.len(), 4, "closed groups show alone");
    assert!(all.iter().all(|row| row.group));

    let by_type = rows(&components(), "LINK", &closed);
    let labels: Vec<_> = by_type.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["craft_ui::buttons::link", "Link"]);

    let by_story = rows(&components(), "long_text", &closed);
    assert_eq!(by_story.last().map(|row| row.label.as_str()), Some("Button<'_>"));

    let none = rows(&components(), "no such component", &closed);
    assert!(none.is_empty());
}

/// A crate with no registrations here was linked out, which in a release web build loses its stories.
#[trial]
fn every_linked_crate_registers() {
    let (counts, stories) = craft_ui_gallery::linked_counts();
    assert!(stories > 0);
    let empty: Vec<_> = counts.iter().filter(|(_, count)| *count == 0).collect();
    assert!(empty.is_empty(), "no registrations from {empty:?}");
}
