//! The gallery's shell: the window the app opens with.

use craft_settings::{Location, OpenOptions, SettingsStore};
use craft_ui::shell::app::ShellApp;
use craft_ui::testing::{Fixture, trial};
use craft_ui_gallery::chrome::{Gallery, Persisted};
use craft_ui_gallery::viewer::{Catalog, GallerySettings};
use egui::Vec2;

fn gallery_in(workspace: &str) -> Gallery {
    let (mut store, report) = SettingsStore::<GallerySettings>::open_with(OpenOptions::new(Location::Memory));
    let mut settings = store.get().clone();
    settings.workspace = workspace.to_owned();
    store.set(settings);
    Gallery::new(Catalog::linked(), Persisted::new(store, &report))
}

/// Each workspace's dock runs its first frames at a desktop size and shows its panels' tabs: the components workspace
/// the tree, the theme workspace the preview, which runs the mock application inside the dock.
#[trial]
fn the_shell_shows_each_workspaces_panels_at_a_desktop_size() {
    for (workspace, tab) in [("components", "Components"), ("theme", "Preview")] {
        let mut gallery = gallery_in(workspace);
        let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
        fixture.room(Vec2::new(1400.0, 900.0)).settle();
        assert!(fixture.by_label(tab).is_some(), "the {workspace} workspace shows its {tab} tab");
    }
}
