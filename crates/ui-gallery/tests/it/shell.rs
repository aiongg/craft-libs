//! The gallery's shell: the window the app opens with.

use craft_settings::{Location, OpenOptions, SettingsStore};
use craft_ui::shell::app::ShellApp;
use craft_ui::testing::{Fixture, trial};
use craft_ui_gallery::chrome::{Gallery, Persisted};
use craft_ui_gallery::viewer::{Catalog, GallerySettings};
use egui::Vec2;

/// The shell shown at a desktop size runs its first frames: a panel at an explicit rect must not look like a second
/// scroll owner to the collection it holds, which panicked the app at start.
#[trial]
fn the_shell_shows_at_a_desktop_size() {
    let (store, report) = SettingsStore::<GallerySettings>::open_with(OpenOptions::new(Location::Memory));
    let mut gallery = Gallery::new(Catalog::linked(), Persisted::new(store, &report));
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    assert!(fixture.by_label("Components").is_some(), "the components panel is shown");
}
