//! The Overview: the cards of a level, live.

use craft_settings::{Location, OpenOptions, SettingsStore};
use craft_ui::shell::app::ShellApp;
use craft_ui::testing::{Fixture, trial};
use craft_ui_gallery::chrome::{Gallery, Persisted};
use craft_ui_gallery::viewer::{Catalog, GallerySettings};
use egui::Vec2;

fn gallery_at(level: &str) -> Gallery {
    let (mut store, report) = SettingsStore::<GallerySettings>::open_with(OpenOptions::new(Location::Memory));
    let mut settings = store.get().clone();
    settings.level = level.to_owned();
    store.set(settings);
    Gallery::new(Catalog::linked(), Persisted::new(store, &report))
}

/// Each card of a level runs its story under a header with Open.
#[trial]
fn the_overview_shows_live_cards_with_their_open_command() {
    let mut gallery = gallery_at("Controls");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    assert!(fixture.by_label("Open").is_some(), "a card header has Open");
    assert!(fixture.by_label("AngleDial").is_some(), "the cards are titled by the component's name");
}

/// Every level opens in the Overview and runs all its cards.
#[trial]
fn every_level_of_the_overview_runs_its_cards() {
    for level in ["Primitives", "Controls", "Composites", "Patterns", "Shells"] {
        let mut gallery = gallery_at(level);
        let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
        fixture.room(Vec2::new(1400.0, 900.0)).settle();
        assert!(fixture.by_label("Open").is_some(), "{level} has cards");
    }
}

/// A story that opens a window or a dialog on its own starts closed in its card, behind the one opener button: no
/// window title shows until the button is pressed.
#[trial]
fn window_stories_start_closed_behind_their_opener() {
    let mut gallery = gallery_at("Shells");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    assert!(fixture.by_label("Show dialog").is_some(), "a window story has its opener");
    assert!(fixture.by_label("Export Log").is_none() && fixture.by_label("Landscape.psd").is_none(), "no window opens by itself");
}
