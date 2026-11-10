//! The Overview: the cards of a level, live.

use craft_settings::{Location, OpenOptions, SettingsStore};
use craft_ui::runtime::access::Role;
use craft_ui::shell::app::ShellApp;
use craft_ui::testing::{Fixture, pointer, trial};
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

/// The top of the first painted run of `text`.
fn top_of(fixture: &Fixture, text: &str) -> Option<f32> {
    fixture.texts().iter().find(|run| run.text == text).map(|run| run.rect.top())
}

/// The registered type name of the component listed as `name`.
fn type_of(name: &str) -> &'static str {
    Catalog::linked().components.iter().find(|component| component.name == name).unwrap_or_else(|| panic!("{name} is listed")).type_name
}

/// The Overview at `level` with `filter`, the component listed as `selected` selected.
fn gallery_selecting(level: &str, filter: &str, selected: &str) -> Gallery {
    let (mut store, report) = SettingsStore::<GallerySettings>::open_with(OpenOptions::new(Location::Memory));
    let mut settings = store.get().clone();
    settings.level = level.to_owned();
    settings.filter = filter.to_owned();
    settings.selected = type_of(selected).to_owned();
    store.set(settings);
    Gallery::new(Catalog::linked(), Persisted::new(store, &report))
}

/// The Overview card shows the cases of the component's first story group, in declaration order, each under its
/// title; picking another group in the Stories panel shows that group, with its name after the card's title.
#[trial]
fn a_card_shows_its_picked_story_group_and_the_stories_panel_picks_another() {
    let catalog = Catalog::linked();
    let slider = catalog.components.iter().find(|component| component.name == "Slider").expect("Slider is listed");
    let [values, step, ..] = slider.groups.as_slice() else { panic!("Slider has groups: {:?}", slider.groups) };
    assert_eq!((values.name.as_str(), step.name.as_str()), ("Values", "Step"), "Slider's groups are the old stories, in order");
    let titles = |group: &craft_ui_gallery::viewer::StoryGroup| group.members.iter().map(|&at| slider.titles[at].clone()).collect::<Vec<_>>();
    let mut gallery = gallery_selecting("Controls", "Slider", "Slider");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    let tops: Vec<f32> = titles(values).iter().map(|title| top_of(&fixture, title).unwrap_or_else(|| panic!("the title `{title}` is painted"))).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "the cases run down the card in declaration order: {:?}", titles(values));
    for title in titles(step) {
        assert!(!painted(&fixture, &title), "a case of another group is not on the card: {title}");
    }
    assert!(!painted(&fixture, "· Values"), "the first group has no note");
    let row = fixture.by_label("Step").expect("the Stories panel lists the group Step");
    fixture.click(&row).settle();
    for title in titles(step) {
        assert!(painted(&fixture, &title), "the picked group's case shows: {title}");
    }
    assert!(!painted(&fixture, &titles(values)[0]), "the first group gave way to the picked one");
    assert!(painted(&fixture, "· Step"), "the picked group's name follows the card's title");
}

/// The painted texts right of `target`, on its row.
fn beside(fixture: &Fixture, target: &craft_ui::testing::Target) -> Vec<String> {
    let row = target.rect;
    fixture
        .texts()
        .iter()
        .filter(|run| run.rect.left() >= row.right() && run.rect.center().y > row.top() && run.rect.center().y < row.bottom())
        .map(|run| run.text.clone())
        .collect()
}

/// A form row's third column shows the slider's value text, the text the slider gives assistive technology, and
/// follows a change of the value.
#[trial]
fn a_form_rows_value_column_shows_and_follows_the_sliders_value_text() {
    let mut gallery = gallery_selecting("Controls", "Slider", "Slider");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).settle();
    let slider = fixture.by_label("Middle").expect("the Middle slider");
    let before = fixture.tree().value_of(slider.node).expect("the slider has a value text").to_owned();
    assert!(beside(&fixture, &slider).contains(&before), "the value column shows `{before}`: {:?}", beside(&fixture, &slider));
    fixture.click(&slider).settle().key(egui::Key::ArrowRight, egui::Modifiers::NONE).settle();
    let slider = fixture.by_label("Middle").expect("the Middle slider");
    let after = fixture.tree().value_of(slider.node).expect("the slider has a value text").to_owned();
    assert_ne!(after, before, "the key changed the value");
    assert!(beside(&fixture, &slider).contains(&after), "the value column follows to `{after}`: {:?}", beside(&fixture, &slider));
}

fn painted(fixture: &Fixture, text: &str) -> bool {
    fixture.texts().iter().any(|run| run.text == text)
}

fn openers(fixture: &Fixture) -> usize {
    fixture.texts().iter().filter(|run| run.text == "Show dialog").count()
}

/// A card whose story opens a window starts closed behind its opener, so no window opens until it is pressed, from the
/// first frame on; the opener runs the story and the window shows.
#[trial]
fn window_stories_start_closed_behind_their_opener() {
    let mut gallery = gallery_filtered("Shells", "Window");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).idle(1);
    let shown = openers(&fixture);
    assert!(shown >= 2, "DocumentWindow and ViewportWindow show their opener, found {shown}");
    for content in ["Show guides", "The export finished."] {
        assert!(!painted(&fixture, content), "no window content shows before the opener is pressed: {content}");
    }
    let opener = fixture.by_label("Show dialog").expect("an opener");
    fixture.click(&opener).settle();
    assert!(painted(&fixture, "Show guides"), "the first card, DocumentWindow, shows its window once its opener is pressed");
}

/// A dialog in an embedded shell of the story's own is out of the story's measure; the card still starts it closed
/// before anything of it paints, opens it by the button, and shows the button again once the story closed both.
#[trial]
fn dialogs_in_a_storys_own_shell_start_closed_and_close_back_to_the_opener() {
    let (mut store, report) = SettingsStore::<GallerySettings>::open_with(OpenOptions::new(Location::Memory));
    let mut settings = store.get().clone();
    settings.level = "Shells".to_owned();
    settings.filter = "AppShell".to_owned();
    settings.stories.insert(type_of("AppShell").to_owned(), "app_shell_dialogs_in_regions".to_owned());
    store.set(settings);
    let mut gallery = Gallery::new(Catalog::linked(), Persisted::new(store, &report));
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).idle(1);
    let over = fixture.by_label("Open").expect("the card's Open").rect.center();
    let high = fixture.shown().height() / 5.0;
    assert_eq!(openers(&fixture), 1, "the picked story shows its opener");
    assert!(!painted(&fixture, "Snap to grid"), "nothing of the dialogs paints before the opener is pressed");
    let opener = fixture.by_label("Show dialog").expect("the opener");
    fixture.click(&opener).settle();
    assert!(painted(&fixture, "Snap to grid"), "the opener opens the dialogs");
    assert_eq!(openers(&fixture), 0, "the opener gives way to the story");
    // The story now takes a view's height under the opener's place: the wheel brings its dialogs up into view.
    for _ in 0..60 {
        if fixture.by_label("Close").is_some_and(|close| close.rect.top() < 2.0 * high) {
            break;
        }
        fixture.frame(vec![pointer::moved(over)]).frame(vec![pointer::wheel(-120.0)]).idle(1);
    }
    for _ in 0..2 {
        let close = fixture.by_label("Close").expect("a dialog's close button");
        fixture.click(&close).settle();
        // The exit fade runs on the fixture's clock.
        fixture.advance(1.0).settle();
    }
    assert!(!painted(&fixture, "Snap to grid"), "both dialogs closed");
    assert_eq!(openers(&fixture), 1, "closing the story's dialogs brings the opener back");
}

/// A dialog the story shows directly closes back to the opener by its own button.
#[trial]
fn a_dialog_closed_by_its_own_button_brings_the_opener_back() {
    let mut gallery = gallery_filtered("Patterns", "AboutDialog");
    let mut fixture = Fixture::new(move |ui| gallery.ui(ui));
    fixture.room(Vec2::new(1400.0, 900.0)).idle(1);
    let stories = openers(&fixture);
    assert!(stories >= 1, "each story of the card shows its opener on its first frame");
    assert!(fixture.by_label("OK").is_none(), "the dialog is closed");
    let opener = fixture.by_label("Show dialog").expect("an opener");
    fixture.click(&opener).settle();
    assert_eq!(openers(&fixture), stories - 1, "the opener gives way to its story's dialog; the others stay");
    let ok = fixture.by_label("OK").expect("the dialog's OK");
    fixture.click(&ok).settle();
    assert_eq!(openers(&fixture), stories - 1, "the dialog's exit fade runs in its card, on the fixture's clock");
    fixture.advance(1.0).settle();
    assert_eq!(openers(&fixture), stories, "OK brings the opener back once the fade ended");
    assert!(fixture.by_label("OK").is_none(), "the dialog is closed");
}

/// Regression: the wheel over a canvas card went to its story and the Overview stopped scrolling; inside a
/// card the wheel takers take the wheel only while focused or armed, so it scrolls the Overview.
#[trial]
fn the_wheel_over_a_card_scrolls_the_overview() {
    // The canvas overlays are Primitives.
    let mut gallery = gallery_at("Primitives");
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
