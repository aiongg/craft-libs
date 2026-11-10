//! The Overview: the cards of a level, live.

use craft_settings::{Location, OpenOptions, SettingsStore};
use craft_ui::runtime::access::Role;
use craft_ui::shell::app::ShellApp;
use craft_ui::testing::{Fixture, pointer, trial};
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

/// Regression: the wheel over a canvas card went to its story and the Overview stopped scrolling; inside a
/// card the wheel takers take the wheel only while focused or armed, so it scrolls the Overview.
#[trial]
fn the_wheel_over_a_card_scrolls_the_overview() {
    // Controls is the level with a canvas card in the first view.
    let mut gallery = gallery_at("Controls");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    let visible = fixture.shown();
    let story = fixture
        .by_role(Role::CANVAS)
        .into_iter()
        .find(|t| visible.contains_rect(t.rect) && t.rect.height() > 40.0)
        .expect("a canvas card is in view");
    let title = fixture.by_label("Open").expect("a card header has Open").rect.min.y;
    fixture.frame(vec![pointer::moved(story.center())]).frame(vec![pointer::wheel(-120.0)]).settle();
    let after = fixture.by_label("Open").expect("a card header has Open").rect.min.y;
    assert!(after < title, "the wheel over a canvas card scrolled the Overview: {title} -> {after}");
}
