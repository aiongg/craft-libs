//! `LinkList` and `RelinkDialog` through the fixture, with a fake `LinkFiles`, `ManualJobs` and scripted picks.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use craft_behavior::Selection;
use craft_browse::ui::relink::{LinkFilter, LinkList, LinkListEvent, LinkListState, RelinkDialog, RelinkOptions, RelinkOutcome, RelinkRow};
use craft_docpath::links::{FakeLinkFiles, Link, LinkStatus, identity_of};
use craft_host::{AppInfo, FileError, Files, Host, Jobs, ManualJobs, PickId, PickKind, PickRequest, Picked};
use craft_ui::contract::Bind;
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::display::Text;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

const STAMP: u64 = 1;

/// The bytes of a file that is the same as every link was recorded with.
const SAME: &[u8] = b"abc";
/// The fixture room of `width` by `height` points.
fn room_of(width: f32, height: f32) -> egui::Vec2 {
    egui::vec2(width, height)
}

/// Bytes of another size, so the file is a different one.
const OTHER: &[u8] = b"abcdefghi";

/// A `Files` service whose pickers stay open until the test answers them.
#[derive(Default)]
struct ScriptedFiles {
    state: Mutex<(u64, Vec<(PickId, PickRequest)>, Vec<(PickId, Picked)>)>,
}

impl ScriptedFiles {
    fn open(&self) -> Vec<PickRequest> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).1.iter().map(|(_, request)| request.clone()).collect()
    }

    fn answer(&self, picked: Picked) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let (id, _) = state.1.remove(0);
        state.2.push((id, picked));
    }
}

impl Files for ScriptedFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.0 += 1;
        let id = PickId(state.0);
        state.1.push((id, request));
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let at = state.2.iter().position(|(answered, _)| *answered == id)?;
        Some(state.2.remove(at).1)
    }

    fn forget(&self, id: PickId) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.1.retain(|(open, _)| *open != id);
        state.2.retain(|(answered, _)| *answered != id);
    }

    fn pick_pending(&self) -> bool {
        !self.state.lock().unwrap_or_else(PoisonError::into_inner).1.is_empty()
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

struct App {
    dialog: RelinkDialog<u32>,
    stack: DialogStack,
    result: Option<DialogResult<RelinkOutcome<u32>>>,
}

/// A dialog under test: the fixture, the dialog's app and the jobs and picks it runs on.
struct Rig {
    fx: Fixture,
    app: Rc<RefCell<App>>,
    jobs: Arc<ManualJobs>,
    picks: Arc<ScriptedFiles>,
}

fn row(key: u32, owner: &str, stored: &str) -> RelinkRow<u32> {
    let identity = identity_of(&mut Cursor::new(SAME), SAME.len() as u64, Some(SystemTime::UNIX_EPOCH + Duration::from_secs(STAMP))).ok();
    RelinkRow::new(Link { key, name: owner.into(), stored: stored.into(), identity, page: None, embedded: false }, owner, "Media missing")
}

/// A file system with `files` (path, bytes), all modified at the stamp the links were recorded with.
fn files(files: &[(&str, &[u8])]) -> Arc<FakeLinkFiles> {
    Arc::new(files.iter().fold(FakeLinkFiles::new(), |fake, (path, bytes)| fake.with(*path, bytes, STAMP)))
}

fn rig(rows: Vec<RelinkRow<u32>>, files: &Arc<FakeLinkFiles>, folder: &str, relink_others: bool, with_jobs: bool) -> Rig {
    let jobs = Arc::new(ManualJobs::new());
    let picks = Arc::new(ScriptedFiles::default());
    let mut options = RelinkOptions::default().relink_others(relink_others);
    if !folder.is_empty() {
        options = options.folder(folder);
    }
    if with_jobs {
        options = options.jobs(Arc::clone(&jobs) as Arc<dyn Jobs>);
    }
    let dialog = RelinkDialog::new(rows, files.clone(), options);
    let mut stack = DialogStack::default();
    stack.push(dialog.id(), None);
    let app = Rc::new(RefCell::new(App { dialog, stack, result: None }));
    let shown = Rc::clone(&app);
    let mut fx = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { dialog, stack, result } = &mut *app;
        if let Some(answer) = dialog.show(ui, stack) {
            *result = Some(answer);
        }
    });
    let host = Host::builder(AppInfo { id: "relink-trials", name: "Relink trials", version: "0" }).files(picks.clone() as Arc<dyn Files>).build();
    fx.room(room_of(900.0, 900.0)).host(Arc::new(host));
    fx.settle();
    Rig { fx, app, jobs, picks }
}

impl Rig {
    fn click(&mut self, label: &str) {
        let target = self.fx.by_label(label).unwrap_or_else(|| panic!("no `{label}` on screen"));
        self.fx.click(&target);
        self.fx.settle();
    }

    fn present(&self, text: &str) -> bool {
        self.fx.by_label(text).is_some() || self.fx.texts().iter().any(|painted| painted.text == text)
    }

    /// Runs the search for the selected row and lets its hits arrive.
    fn search(&mut self) {
        self.click("Search");
        self.jobs.run_all();
        self.fx.settle();
    }

    fn done(&mut self) -> DialogResult<RelinkOutcome<u32>> {
        self.click("Done");
        self.app.borrow_mut().result.take().expect("Done answers")
    }

    fn answered(&self) -> bool {
        self.app.borrow().result.is_some()
    }
}

fn linked_keys(result: &DialogResult<RelinkOutcome<u32>>) -> (Vec<u32>, Vec<u32>) {
    let DialogResult::Ok(outcome) = result else { panic!("the dialog was cancelled") };
    (outcome.linked.iter().map(|(key, _)| *key).collect(), outcome.offline.clone())
}

#[trial]
fn the_relink_dialog_links_one_row_then_the_next() {
    let files = files(&[("/new/a.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov"), row(2, "Clip B", "/old/b.mov")], &files, "/new", false, true);
    rig.search();
    assert!(rig.present("/new/a.mov"), "the match is listed");
    rig.click("Link");
    assert!(!rig.present("Clip A"), "the linked row leaves the table");
    assert!(rig.present("Clip B"), "the next row is shown");
    assert!(!rig.present("/new/a.mov"), "the candidates are cleared");
    assert!(!rig.answered(), "Link keeps the dialog open");
}

#[trial]
fn link_anyway_replaces_link_for_a_failing_candidate() {
    let files = files(&[("/new/a.mov", OTHER)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov")], &files, "/new", false, true);
    rig.search();
    assert!(rig.present("Link Anyway"));
    assert!(!rig.present("Link"));
    rig.click("Link Anyway");
    let (linked, _) = linked_keys(&rig.done());
    assert_eq!(linked, [1], "Link Anyway links the failing file");
}

#[trial]
fn relink_others_offers_the_rows_the_remap_fits() {
    let files = files(&[("/T7/shoot/a.mov", SAME), ("/T7/shoot/b.mov", SAME), ("/T7/shoot/c.mov", OTHER)]);
    let rows = vec![row(1, "Clip A", "C:\\shoot\\a.mov"), row(2, "Clip B", "C:\\shoot\\b.mov"), row(3, "Clip C", "C:\\shoot\\c.mov")];
    let mut rig = rig(rows, &files, "/T7/shoot", true, true);
    rig.search();
    rig.click("Link");
    rig.jobs.run_all();
    rig.fx.settle();
    assert!(!rig.present("Clip B"), "the row the remap fits is linked");
    assert!(rig.present("Clip C"), "the row that fails its check stays missing");
    assert_eq!(linked_keys(&rig.done()), (vec![1, 2], vec![]));
}

#[trial]
fn done_returns_every_link_and_cancel_returns_nothing() {
    let files = files(&[("/new/b.mov", SAME)]);
    let rows = || vec![row(1, "Clip A", "/old/a.mov"), row(2, "Clip B", "/old/b.mov")];
    let mut first = rig(rows(), &files, "/new", false, true);
    first.click("Leave Offline");
    first.search();
    first.click("Link");
    assert_eq!(linked_keys(&first.done()), (vec![2], vec![1]));

    let mut second = rig(rows(), &files, "/new", false, true);
    second.click("Leave All Offline");
    assert!(!second.answered(), "leaving rows offline does not close the dialog");
    second.click("Cancel");
    assert!(matches!(second.app.borrow().result, Some(DialogResult::Cancel)));

    let mut third = rig(rows(), &files, "/new", false, true);
    third.fx.key(Key::Escape, Modifiers::NONE);
    third.fx.settle();
    assert!(matches!(third.app.borrow().result, Some(DialogResult::Cancel)));
}

#[trial]
fn the_search_never_blocks_and_stop_cancels_it() {
    let files = files(&[("/new/a.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov")], &files, "/new", false, true);
    rig.click("Search");
    assert_eq!(rig.jobs.waiting(), 1, "the search waits as a job");
    assert!(rig.present("Searching… 0 files"), "the progress is shown while it waits");
    rig.click("Stop");
    assert!(rig.present("Search"), "Stop gives the button back");
    rig.jobs.run_all();
    rig.fx.settle();
    assert!(!rig.present("/new/a.mov"), "a stopped search reports nothing");
}

#[trial]
fn browse_fills_the_search_folder() {
    let files = files(&[("/new/a.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov")], &files, "", false, true);
    rig.click("Browse…");
    let open = rig.picks.open();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].kind, PickKind::Folder);
    rig.click("Search");
    assert_eq!(rig.jobs.waiting(), 0, "no folder yet: Search does nothing");
    rig.picks.answer(Picked::Paths(vec!["/new".into()]));
    rig.fx.settle();
    rig.click("Search");
    assert_eq!(rig.jobs.waiting(), 1, "the picked folder is searched");
}

#[trial]
fn locate_adds_the_picked_file_as_the_candidate() {
    let files = files(&[("/elsewhere/b.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/b.mov")], &files, "/new", false, true);
    rig.click("Locate…");
    assert_eq!(rig.picks.open()[0].kind, PickKind::Open);
    rig.picks.answer(Picked::Paths(vec!["/elsewhere/b.mov".into()]));
    rig.fx.settle();
    rig.jobs.run_all();
    rig.fx.settle();
    assert!(rig.present("/elsewhere/b.mov"), "the located file is listed");
    rig.click("Link");
    let (linked, _) = linked_keys(&rig.done());
    assert_eq!(linked, [1]);
}

#[trial]
fn escape_with_a_pick_open_closes_the_pick_and_not_the_dialog() {
    let files = files(&[("/new/a.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov")], &files, "/new", false, true);
    rig.click("Browse…");
    assert!(rig.picks.pick_pending());
    rig.fx.key(Key::Escape, Modifiers::NONE);
    rig.fx.settle();
    assert!(!rig.picks.pick_pending(), "the pick is dropped");
    assert!(!rig.answered(), "the dialog stays open");
    rig.fx.key(Key::Escape, Modifiers::NONE);
    rig.fx.settle();
    assert!(matches!(rig.app.borrow().result, Some(DialogResult::Cancel)), "a second Esc cancels the dialog");
}

#[trial]
fn a_search_is_one_job() {
    let files = files(&[("/new/a.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov")], &files, "/new", false, true);
    rig.search();
    assert_eq!(rig.jobs.waiting(), 0, "hits need no second job");
    assert!(rig.present("/new/a.mov"), "the hit is listed");
}

#[trial]
fn without_jobs_search_is_disabled() {
    let files = files(&[("/new/a.mov", SAME)]);
    let mut rig = rig(vec![row(1, "Clip A", "/old/a.mov")], &files, "/new", false, false);
    rig.click("Search");
    assert_eq!(rig.jobs.waiting(), 0);
    assert!(!rig.present("Stop"), "no search started");
}

// The Links panel.

struct Panel {
    links: Vec<Link<u32>>,
    statuses: HashMap<u32, LinkStatus>,
    state: LinkListState<u32>,
    settings_changed: usize,
    events: Vec<LinkListEvent<u32>>,
}

fn link(key: u32, name: &str, page: Option<u32>, embedded: bool) -> Link<u32> {
    Link { key, name: name.into(), stored: name.into(), identity: None, page, embedded }
}

fn panel() -> (Fixture, Rc<RefCell<Panel>>) {
    let links = vec![
        link(0, "a.png", Some(2), false),
        link(1, "b.png", None, false),
        link(2, "c.png", None, false),
        link(3, "d.png", None, false),
        link(4, "e.png", None, true),
    ];
    let statuses =
        HashMap::from([(0, LinkStatus::Missing), (1, LinkStatus::Modified), (2, LinkStatus::Ok), (3, LinkStatus::Ok), (4, LinkStatus::Embedded)]);
    let panel = Rc::new(RefCell::new(Panel { links, statuses, state: LinkListState::default(), settings_changed: 0, events: Vec::new() }));
    let shown = Rc::clone(&panel);
    let mut fx = Fixture::new(move |ui| {
        let mut panel = shown.borrow_mut();
        let Panel { links, statuses, state, settings_changed, events } = &mut *panel;
        let out = LinkList::new("Links", links, statuses, Bind::bound(state)).show(ui);
        *settings_changed += usize::from(out.settings_changed);
        events.extend(out.events);
    });
    fx.room(room_of(360.0, 320.0)).settle();
    (fx, panel)
}

fn select(fx: &mut Fixture, panel: &Rc<RefCell<Panel>>, keys: &[u32]) {
    panel.borrow_mut().state.selection = Selection::new(keys.to_vec(), keys.last().copied(), keys.last().copied());
    fx.settle();
}

/// Clicks the action `label` and returns the events it caused.
fn press(fx: &mut Fixture, panel: &Rc<RefCell<Panel>>, label: &str) -> Vec<LinkListEvent<u32>> {
    panel.borrow_mut().events.clear();
    let target = fx.by_label(label).unwrap_or_else(|| panic!("no `{label}` on screen"));
    fx.click(&target);
    fx.settle();
    let events = std::mem::take(&mut panel.borrow_mut().events);
    events.into_iter().filter(|event| !matches!(event, LinkListEvent::Selection(_))).collect()
}

#[trial]
fn a_menu_choice_is_a_settings_change() {
    let (mut fx, panel) = panel();
    assert_eq!(panel.borrow().settings_changed, 0);
    let menu = fx.by_label("Links Menu").expect("the menu button");
    fx.click(&menu);
    fx.idle(8);
    let show_missing = fx.by_label("Show Missing").expect("the filter row");
    fx.click(&show_missing);
    fx.idle(8);
    assert_eq!(panel.borrow().state.settings.filter, LinkFilter::Missing);
    assert_eq!(panel.borrow().settings_changed, 1);
    assert!(fx.by_label("a.png, Page 2, Missing").is_some() && fx.by_label("c.png").is_none(), "only the missing link is listed");
}

#[trial]
fn link_list_buttons_follow_the_statuses_of_the_selection() {
    let (mut fx, panel) = panel();
    assert!(press(&mut fx, &panel, "Relink…").is_empty() && press(&mut fx, &panel, "Update Link").is_empty());

    select(&mut fx, &panel, &[0]);
    assert!(press(&mut fx, &panel, "Update Link").is_empty(), "a missing link is not modified");
    assert!(press(&mut fx, &panel, "Edit Original").is_empty(), "a missing link has no original to open");
    assert_eq!(press(&mut fx, &panel, "Break Link"), [LinkListEvent::BreakLink(vec![0])]);

    select(&mut fx, &panel, &[4]);
    assert!(press(&mut fx, &panel, "Break Link").is_empty(), "an embedded link is not linked");

    select(&mut fx, &panel, &[1]);
    assert_eq!(press(&mut fx, &panel, "Update Link"), [LinkListEvent::Update(vec![1])]);
}

#[trial]
fn a_rows_status_is_in_its_name() {
    let (fx, _panel) = panel();
    for name in ["a.png, Page 2, Missing", "b.png, Modified", "c.png", "e.png, Embedded"] {
        assert!(fx.by_label(name).is_some(), "no row named `{name}`");
    }
}

/// The Link Info section shows the host's info rows: the footer runs in a sizing pass first and must leave them for the
/// shown pass.
#[trial]
fn the_link_info_section_shows_its_rows() {
    let links = vec![link(0, "a.png", None, false)];
    let statuses = HashMap::from([(0, LinkStatus::Ok)]);
    let mut state = LinkListState::default();
    state.settings.info_open = true;
    let state = RefCell::new(state);
    let mut fx = Fixture::new(move |ui| {
        let mut state = state.borrow_mut();
        LinkList::new("Links", &links, &statuses, Bind::bound(&mut *state))
            .info(|ui| {
                Text::new("/shoot/cover.psd").show(ui);
            })
            .show(ui);
    });
    fx.room(room_of(360.0, 320.0)).settle();
    assert!(fx.texts().iter().any(|text| text.text == "/shoot/cover.psd"), "the info rows are painted");
}
