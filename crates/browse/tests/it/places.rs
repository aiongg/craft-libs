//! `PlacesTree` with `FakeDirectories` and `ManualJobs`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use craft_browse::places::{FakeDirectories, FolderCache, Place, PlaceGroup, PlaceGroupKind, PlaceKind, Places, StandardFolder};
use craft_browse::ui::places::{PlacesEvent, PlacesTree};
use craft_host::{ManualJobs, Wake};
use craft_ui::contract::{Entry, Role};
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Key, Modifiers, PointerButton};

fn path(s: &str) -> PathBuf {
    PathBuf::from(s)
}

/// `/home/u` holds Photos and Docs; Photos holds 2024 and 2025; 2024 holds Trip.
fn fake() -> FakeDirectories {
    FakeDirectories::new()
        .standard_folder(StandardFolder::Home, "/home/u")
        .folder("/home/u/Docs")
        .folder("/home/u/Photos/2025")
        .folder("/home/u/Photos/2024/Trip")
}

fn home() -> Places {
    let place = Place { path: path("/home/u"), name: "u".into(), kind: PlaceKind::Standard(StandardFolder::Home) };
    Places { groups: vec![PlaceGroup { kind: PlaceGroupKind::Places, places: vec![place] }], owner: Some((0, 0)), ..Places::default() }
}

fn hidden(names: &[&str]) -> Vec<Place> {
    names.iter().map(|n| Place { path: path(&format!("/home/u/{n}")), name: (*n).into(), kind: PlaceKind::Favorite }).collect()
}

struct Rig {
    places: Places,
    cache: FolderCache,
    jobs: Arc<ManualJobs>,
    fake: Arc<FakeDirectories>,
    current: Option<PathBuf>,
    host_menu: bool,
    events: Vec<PlacesEvent>,
}

fn rig(current: Option<&str>, places: Places) -> (Fixture, Rc<RefCell<Rig>>) {
    let jobs = Arc::new(ManualJobs::new());
    let fake = Arc::new(fake());
    let cache = FolderCache::new(fake.clone(), jobs.clone(), Wake::new(|| {}));
    let state = Rc::new(RefCell::new(Rig { places, cache, jobs, fake, current: current.map(path), host_menu: false, events: Vec::new() }));
    let shown = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut rig = shown.borrow_mut();
        let Rig { places, cache, current, host_menu, .. } = &mut *rig;
        let host_menu = *host_menu;
        let mut tree = PlacesTree::new("Places", places, cache)
            .row_menu(move |_| if host_menu { vec![Entry::new(String::from("folder.rename"), "Rename Folder…")] } else { Vec::new() });
        if let Some(current) = current.as_deref() {
            tree = tree.current(current);
        }
        let out = tree.show(ui);
        rig.events.extend(out.events);
    });
    settle(&mut fixture, &state);
    (fixture, state)
}

/// Runs frames, finishing the listing jobs between them, until nothing is waiting.
fn settle(fixture: &mut Fixture, state: &Rc<RefCell<Rig>>) {
    for _ in 0..6 {
        fixture.idle(2);
        state.borrow().jobs.run_all();
    }
    fixture.idle(2);
}

fn rows(fixture: &Fixture) -> Vec<String> {
    fixture.by_role(Role::TREE_ITEM).iter().filter_map(|row| name(fixture.tree(), row.node).map(str::to_owned)).collect()
}

fn click_row(fixture: &mut Fixture, label: &str) {
    let row = fixture.by_label(label).unwrap_or_else(|| panic!("{label} is shown: {:?}", rows(fixture)));
    fixture.click(&row);
}

fn secondary_click(fixture: &mut Fixture, label: &str) {
    let at = fixture.by_label(label).unwrap_or_else(|| panic!("{label} is shown")).center();
    for pressed in [true, false] {
        fixture.frame(vec![
            Event::PointerMoved(at),
            Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE },
        ]);
    }
}

/// `current` deep in Home: the rows open once on the way down; collapsing one keeps it closed on the next passes.
#[trial]
fn the_tree_reveals_the_browsed_folder_once_and_collapse_sticks() {
    let (mut fixture, state) = rig(Some("/home/u/Photos/2024/Trip"), home());
    let shown = rows(&fixture);
    for name in ["Home", "Docs", "Photos", "2024", "Trip", "2025"] {
        assert!(shown.iter().any(|row| row == name), "{name} is shown: {shown:?}");
    }

    click_row(&mut fixture, "Photos");
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    settle(&mut fixture, &state);
    assert!(!rows(&fixture).iter().any(|row| row == "Trip"), "Photos is closed");
    settle(&mut fixture, &state);
    assert!(!rows(&fixture).iter().any(|row| row == "2024"), "the reveal does not open it again");
}

/// A click on a folder row reports `Open`, and Enter on the lead does too.
#[trial]
fn activating_a_folder_row_reports_open() {
    let (mut fixture, state) = rig(Some("/home/u/Photos"), home());
    click_row(&mut fixture, "Docs");
    settle(&mut fixture, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::Open(path("/home/u/Docs"))]);

    state.borrow_mut().events.clear();
    fixture.key(Key::Enter, Modifiers::NONE);
    settle(&mut fixture, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::Open(path("/home/u/Docs"))]);
}

/// The hidden row with one hidden root restores it at once; with two its menu lists them and Restore All.
#[trial]
fn hidden_places_come_back_through_their_row() {
    let mut places = home();
    places.hidden = hidden(&["Music"]);
    let (mut one, state) = rig(None, places);
    click_row(&mut one, "Show 1 hidden place");
    settle(&mut one, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::Restore(path("/home/u/Music"))]);

    let mut places = home();
    places.hidden = hidden(&["Music", "Video"]);
    let (mut two, state) = rig(None, places);
    secondary_click(&mut two, "Show 2 hidden places");
    settle(&mut two, &state);
    assert!(state.borrow().events.is_empty(), "the menu only opens");
    click_row(&mut two, "Restore All");
    settle(&mut two, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::RestoreAll]);
}

/// Window focus regained: the cache's listings go stale and the open folders are listed again.
#[trial]
fn regaining_focus_lists_open_folders_again() {
    let (mut fixture, state) = rig(Some("/home/u/Photos/2024"), home());
    fixture.frame(vec![Event::WindowFocused(false)]);
    settle(&mut fixture, &state);
    let before = state.borrow().fake.calls();
    fixture.frame(vec![Event::WindowFocused(true)]);
    settle(&mut fixture, &state);
    let after = state.borrow().fake.calls();
    assert_eq!(after - before, 2, "Home and Photos are open");
}

/// A fixture with Photos open and `/home/u/Docs` browsed, the keyboard on the tree.
fn browsing_docs() -> (Fixture, Rc<RefCell<Rig>>) {
    let (mut fixture, state) = rig(Some("/home/u/Photos/2024"), home());
    state.borrow_mut().current = Some(path("/home/u/Docs"));
    settle(&mut fixture, &state);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    (fixture, state)
}

/// The selection follows `current` when it changes, and not on the passes between.
#[trial]
fn the_selection_follows_current_only_when_it_changes() {
    let (mut fixture, state) = browsing_docs();
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    settle(&mut fixture, &state);
    fixture.key(Key::Enter, Modifiers::NONE);
    settle(&mut fixture, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::Open(path("/home/u/Photos"))], "the lead stayed on the row the key reached");

    state.borrow_mut().events.clear();
    state.borrow_mut().current = Some(path("/home/u/Photos/2025"));
    settle(&mut fixture, &state);
    fixture.key(Key::Enter, Modifiers::NONE);
    settle(&mut fixture, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::Open(path("/home/u/Photos/2025"))], "a new `current` moves the selection to its row");
}

/// A `current` spelled differently from the listing's still opens the rows on its way.
#[trial]
fn reveal_matches_folders_by_their_key_not_their_spelling() {
    let (fixture, _state) = rig(Some("/home/u/Docs/../Photos/2024/Trip/"), home());
    let shown = rows(&fixture);
    for name in ["Photos", "2024", "Trip"] {
        assert!(shown.iter().any(|row| row == name), "{name} is shown: {shown:?}");
    }
}

/// The host's folder menu rows come after the library's and report their command with the folder.
#[trial]
fn a_host_menu_row_reports_its_command_and_folder() {
    let (mut fixture, state) = rig(Some("/home/u/Photos"), home());
    state.borrow_mut().host_menu = true;
    fixture.idle(2);
    secondary_click(&mut fixture, "Docs");
    settle(&mut fixture, &state);
    click_row(&mut fixture, "Rename Folder…");
    settle(&mut fixture, &state);
    assert_eq!(state.borrow().events, vec![PlacesEvent::Menu(path("/home/u/Docs"), "folder.rename".into())]);
}
