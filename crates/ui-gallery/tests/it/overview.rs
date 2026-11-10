//! The Overview: the cards of a level, live.

use craft_settings::{Location, OpenOptions, SettingsStore};
use craft_ui::shell::app::ShellApp;
use craft_ui::testing::{Fixture, trial};
use craft_ui_gallery::chrome::{Gallery, Persisted};
use craft_ui_gallery::viewer::{Catalog, GallerySettings};
use egui::Vec2;

fn gallery_at(level: &str) -> Gallery {
    gallery_filtered(level, "")
}

fn gallery_filtered(level: &str, filter: &str) -> Gallery {
    let (mut store, report) = SettingsStore::<GallerySettings>::open_with(OpenOptions::new(Location::Memory));
    let mut settings = store.get().clone();
    settings.level = level.to_owned();
    settings.filter = filter.to_owned();
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

/// A card whose story opens a window starts closed behind its opener, so no window opens until it is pressed; the
/// opener runs the story and the window shows.
#[trial]
fn window_stories_start_closed_behind_their_opener() {
    let mut gallery = gallery_filtered("Shells", "Window");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    let painted = |fixture: &Fixture, text: &str| fixture.texts().iter().any(|run| run.text == text);
    let openers = fixture.texts().iter().filter(|run| run.text == "Show dialog").count();
    assert!(openers >= 2, "DocumentWindow and ViewportWindow show their opener, found {openers}");
    for content in ["Show guides", "The export finished."] {
        assert!(!painted(&fixture, content), "no window content shows before the opener is pressed: {content}");
    }
    let opener = fixture.by_label("Show dialog").expect("an opener");
    fixture.click(&opener).settle();
    assert!(painted(&fixture, "Show guides"), "the first card, DocumentWindow, shows its window once its opener is pressed");
}
