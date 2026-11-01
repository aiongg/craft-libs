//! `PlacesTree` through egui_kittest with a fake `Directories` and `ManualJobs` (task B7).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use craft_host::queue::Cancel;
use craft_host::{ManualJobs, Wake};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Id, Modifiers, PointerButton, Ui, ViewportId};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::*;
use crate::places::{Directories, Place, PlaceGroup, Volume};

fn path(s: &str) -> PathBuf {
    PathBuf::from(s)
}

/// `/home/u` holds Photos and Docs; Photos holds 2024 and 2025; 2024 holds Trip.
struct Fake {
    calls: AtomicUsize,
}

impl Directories for Fake {
    fn standard(&self) -> Vec<(StandardFolder, PathBuf)> {
        vec![(StandardFolder::Home, path("/home/u"))]
    }
    fn volumes(&self) -> Vec<Volume> {
        Vec::new()
    }
    fn subfolders(&self, folder: &Path, _: &Cancel) -> std::io::Result<Vec<Folder>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let names: &[&str] = match folder.to_str().unwrap_or_default() {
            "/home/u" => &["Docs", "Photos"],
            "/home/u/Photos" => &["2024", "2025"],
            "/home/u/Photos/2024" => &["Trip"],
            _ => &[],
        };
        Ok(names.iter().map(|n| Folder { path: folder.join(n), name: (*n).into() }).collect())
    }
    fn unlisted_ancestors(&self, _: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
}

struct Host {
    places: Places,
    cache: FolderCache,
    jobs: Arc<ManualJobs>,
    fake: Arc<Fake>,
    current: Option<PathBuf>,
    accepted: &'static str,
    events: Vec<PlacesEvent<u32>>,
}

fn home() -> Places {
    let place = Place { path: path("/home/u"), name: "u".into(), kind: PlaceKind::Standard(StandardFolder::Home) };
    Places { groups: vec![PlaceGroup { kind: PlaceGroupKind::Places, places: vec![place] }], owner: Some((0, 0)), ..Places::default() }
}

fn hidden(names: &[&str]) -> Vec<Place> {
    names.iter().map(|n| Place { path: path(&format!("/home/u/{n}")), name: (*n).into(), kind: PlaceKind::Favorite }).collect()
}

fn show(ui: &mut Ui, host: &mut Host) {
    let accepted = host.accepted;
    let out = PlacesTree::<u32>::new("places", &host.places, &mut host.cache)
        .current(host.current.as_deref())
        .accepts(move |path, _| path.ends_with(accepted))
        .show(ui);
    host.events.extend(out.events);
}

fn harness(current: Option<&str>, places: Places) -> Harness<'static, Host> {
    let jobs = Arc::new(ManualJobs::new());
    let fake = Arc::new(Fake { calls: AtomicUsize::new(0) });
    let cache = FolderCache::new(fake.clone(), jobs.clone(), Wake::new(|| {}));
    let host = Host { places, cache, jobs, fake, current: current.map(path), accepted: "Photos", events: Vec::new() };
    let mut harness = Harness::<Host>::builder()
        .with_size([300.0, 400.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    settle(&mut harness);
    harness
}

/// Runs frames, finishing the listing jobs between them, until nothing is waiting.
fn settle(harness: &mut Harness<'_, Host>) {
    for _ in 0..6 {
        harness.run();
        harness.state().jobs.run_all();
    }
    harness.run();
}

fn rows(harness: &Harness<'_, Host>) -> Vec<String> {
    harness.query_all_by(|n| n.role() == Role::TreeItem).filter_map(|n| n.accesskit_node().label()).collect()
}

fn row<'h>(harness: &'h Harness<'_, Host>, name: &str) -> Node<'h> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::TreeItem && n.label().as_deref() == Some(name.as_str()))
}

fn release_at(harness: &mut Harness<'_, Host>, pos: egui::Pos2) {
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
}

// `current` deep in Home: the rows open once on the way down; collapsing one keeps it closed on the next passes.
#[test]
fn the_tree_reveals_the_browsed_folder_once_and_collapse_sticks() {
    let mut harness = harness(Some("/home/u/Photos/2024/Trip"), home());
    let shown = rows(&harness);
    for name in ["Home", "Docs", "Photos", "2024", "Trip", "2025"] {
        assert!(shown.iter().any(|r| r == name), "{name} is shown: {shown:?}");
    }

    harness.get_by_label("Collapse Photos").click();
    settle(&mut harness);
    assert!(!rows(&harness).iter().any(|r| r == "Trip"), "Photos is closed");
    settle(&mut harness);
    assert!(!rows(&harness).iter().any(|r| r == "2024"), "the reveal does not open it again");
}

// A ListDrag over an accepted row outlines it and the release reports Dropped; a refused row shows nothing.
#[test]
fn a_drop_on_an_accepted_row_reports_it() {
    let mut harness = harness(Some("/home/u/Photos"), home());
    let payload = ListDrag { list: Id::NULL.with("source"), items: vec![7u32], label: "Seven".into() };
    egui::DragAndDrop::set_payload(&harness.ctx, payload.clone());

    let docs = row(&harness, "Docs").rect().center();
    harness.event(Event::PointerMoved(docs));
    harness.run();
    release_at(&mut harness, docs);
    assert!(harness.state().events.iter().all(|e| !matches!(e, PlacesEvent::Dropped { .. })), "a refused row reports nothing");

    egui::DragAndDrop::set_payload(&harness.ctx, payload.clone());
    let photos = row(&harness, "Photos").rect().center();
    harness.event(Event::PointerMoved(photos));
    harness.run();
    release_at(&mut harness, photos);
    let dropped: Vec<_> = harness.state().events.iter().filter(|e| matches!(e, PlacesEvent::Dropped { .. })).cloned().collect();
    assert_eq!(dropped, vec![PlacesEvent::Dropped { place: path("/home/u/Photos"), items: payload }]);
}

// The hidden row with one hidden root restores it at once; with two it opens a menu with Restore All.
#[test]
fn hidden_places_come_back_through_their_row() {
    let mut places = home();
    places.hidden = hidden(&["Music"]);
    let mut one = harness(None, places);
    one.get_by_label("Show 1 hidden place").click();
    settle(&mut one);
    assert_eq!(one.state().events, vec![PlacesEvent::Restore(path("/home/u/Music"))]);

    let mut places = home();
    places.hidden = hidden(&["Music", "Video"]);
    let mut two = harness(None, places);
    two.get_by_label("Show 2 hidden places").click();
    settle(&mut two);
    assert!(two.state().events.is_empty(), "the click only opens the menu");
    two.get_by_label("Restore All").click();
    settle(&mut two);
    assert_eq!(two.state().events, vec![PlacesEvent::RestoreAll]);
}

// Window focus regained: the cache's listings go stale and open folders are listed again.
#[test]
fn regaining_focus_lists_open_folders_again() {
    let mut harness = harness(Some("/home/u/Photos/2024"), home());
    let focus = |harness: &mut Harness<'_, Host>, focused| {
        harness.input_mut().viewports.entry(ViewportId::ROOT).or_default().focused = Some(focused);
        settle(harness);
    };
    focus(&mut harness, false);
    let before = harness.state().fake.calls.load(Ordering::SeqCst);
    focus(&mut harness, true);
    let after = harness.state().fake.calls.load(Ordering::SeqCst);
    assert_eq!(after - before, 2, "Home and Photos are open");
}
