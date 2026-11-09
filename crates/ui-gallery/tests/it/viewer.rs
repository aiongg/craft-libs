//! The viewer's own logic: what it saves and restores, and how the filter narrows the component tree.

use craft_behavior::Selection;
use craft_settings::{Location, OpenOptions, SettingsFiles, SettingsStore};
use craft_theme::Mode;
use craft_ui::list::OpenSet;
use craft_ui::testing::trial;
use craft_ui_gallery::viewer::{Catalog, ComponentInfo, GallerySettings, PanelShares, TreeKey, Viewer, rows};
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

#[trial]
fn a_saved_layout_restores_the_shares() {
    let mut shares = PanelShares::default();
    shares.columns = [1.0, 4.0, 2.5];
    shares.right = [3.0, 1.0];
    assert_eq!(PanelShares::load(&shares.save()), shares);
    assert_eq!(PanelShares::of_tree(&shares.tree()), Some(shares));
}

#[trial]
fn settings_restore_the_state_they_were_saved_from() {
    let mut before = viewer(&GallerySettings::default());
    before.env.mode = Mode::Dark;
    before.browser.filter = "u".to_owned();
    before.browser.selection = Selection::single(TreeKey::Component("craft_ui::buttons::link::Link"));
    before.browser.open.set(&TreeKey::Module("craft_ui::buttons::link"), true);
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
