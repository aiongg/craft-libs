//! `RelinkDialog` through egui_kittest with a fake `LinkFiles` and `ManualJobs` (task B10).

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use craft_docpath::links::{FakeLinkFiles, Link, identity_of};
use craft_host::{AppInfo, FileError, Files, Host, Jobs, ManualJobs, PickId, PickKind, PickRequest, Picked};
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{RelinkColumn, RelinkDialog, RelinkLayout, RelinkOptions, RelinkOutcome, RelinkRow};

const STAMP: u64 = 1;

/// The bytes of a file that is the same as every link was recorded with.
const SAME: &[u8] = b"abc";
/// Bytes of another size, so the file is a different one.
const OTHER: &[u8] = b"abcdefghi";

/// The tickets of the open pickers and of the answered ones.
#[derive(Default)]
struct Tickets {
    last: u64,
    open: Vec<(PickId, PickRequest)>,
    answered: Vec<(PickId, Picked)>,
}

/// A `Files` service whose pickers stay open until the test answers them.
#[derive(Default)]
struct ScriptedFiles {
    state: Mutex<Tickets>,
}

impl ScriptedFiles {
    /// The requests of the pickers that are open.
    fn open(&self) -> Vec<PickRequest> {
        self.state.lock().unwrap().open.iter().map(|(_, request)| request.clone()).collect()
    }

    /// Answers the oldest open picker.
    fn answer(&self, picked: Picked) {
        let mut state = self.state.lock().unwrap();
        let (id, _) = state.open.remove(0);
        state.answered.push((id, picked));
    }
}

impl Files for ScriptedFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap();
        state.last += 1;
        let id = PickId(state.last);
        state.open.push((id, request));
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap();
        let at = state.answered.iter().position(|(answered, _)| *answered == id)?;
        Some(state.answered.remove(at).1)
    }

    fn forget(&self, id: PickId) {
        let mut state = self.state.lock().unwrap();
        state.open.retain(|(open, _)| *open != id);
        state.answered.retain(|(answered, _)| *answered != id);
    }

    fn pick_pending(&self) -> bool {
        !self.state.lock().unwrap().open.is_empty()
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

fn row(key: u32, owner: &str, stored: &str) -> RelinkRow<u32> {
    let identity = identity_of(&mut Cursor::new(SAME), SAME.len() as u64, Some(SystemTime::UNIX_EPOCH + Duration::from_secs(STAMP))).unwrap();
    RelinkRow {
        link: Link { key, name: owner.into(), stored: stored.into(), identity: Some(identity), page: None, embedded: false },
        owner: owner.into(),
        status: "Media missing".into(),
    }
}

/// What a test starts the dialog with.
struct Setup {
    options: RelinkOptions,
    picks: Arc<ScriptedFiles>,
}

fn setup(jobs: &Arc<ManualJobs>, folder: &str, relink_others: bool) -> Setup {
    let options =
        RelinkOptions { relink_others, folder: Some(folder.into()), jobs: Some(Arc::clone(jobs) as Arc<dyn Jobs>), ..RelinkOptions::default() };
    Setup { options, picks: Arc::new(ScriptedFiles::default()) }
}

fn start(rows: Vec<RelinkRow<u32>>, files: &Arc<FakeLinkFiles>, setup: Setup) -> Harness<'static, App> {
    let Setup { options, picks, .. } = setup;
    let dialog = RelinkDialog::new(rows, files.clone(), options);
    let mut stack = DialogStack::default();
    stack.push(dialog.id(), None);
    let mut harness = Harness::builder()
        .with_size(vec2(900.0, 900.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            Host::builder(AppInfo { id: "relink-tests", name: "Relink tests", version: "0" })
                .files(Arc::clone(&picks) as Arc<dyn Files>)
                .install(ctx);
        })
        .build_ui_state(
            |ui: &mut egui::Ui, app: &mut App| {
                if let Some(result) = app.dialog.show(&ui.ctx().clone(), &app.stack) {
                    app.result = Some(result);
                }
            },
            App { dialog, stack, result: None },
        );
    harness.run();
    harness
}

fn harness(
    rows: Vec<RelinkRow<u32>>,
    files: &Arc<FakeLinkFiles>,
    jobs: &Arc<ManualJobs>,
    folder: &str,
    relink_others: bool,
) -> Harness<'static, App> {
    start(rows, files, setup(jobs, folder, relink_others))
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

fn present(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Runs the search for the selected row and lets its hits arrive.
fn search(harness: &mut Harness<'_, App>, jobs: &ManualJobs) {
    click(harness, "Search");
    jobs.run_all();
    harness.run();
}

fn candidates(harness: &Harness<'_, App>) -> usize {
    harness.query_all_by(|n| n.role() == Role::ListBoxOption).count()
}

fn linked_keys(result: &DialogResult<RelinkOutcome<u32>>) -> (Vec<u32>, Vec<u32>) {
    let DialogResult::Ok(outcome) = result else { panic!("the dialog was cancelled") };
    (outcome.linked.iter().map(|(key, _)| *key).collect(), outcome.offline.clone())
}

/// A file system with `files` (path, bytes), all modified at the stamp the links were recorded with.
fn files(files: &[(&str, &[u8])]) -> Arc<FakeLinkFiles> {
    Arc::new(files.iter().fold(FakeLinkFiles::new(), |fake, (path, bytes)| fake.with(*path, bytes, STAMP)))
}

/// Clicks Done and takes the dialog's answer.
fn done(harness: &mut Harness<'_, App>) -> DialogResult<RelinkOutcome<u32>> {
    click(harness, "Done");
    harness.state_mut().result.take().expect("Done answers")
}

// Link on the first row's candidate selects the second row and clears the candidates.
#[test]
fn the_relink_dialog_links_one_row_then_the_next() {
    let files = files(&[("/new/a.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let mut harness = harness(vec![row(1, "Clip A", "/old/a.mov"), row(2, "Clip B", "/old/b.mov")], &files, &jobs, "/new", false);
    search(&mut harness, &jobs);
    assert_eq!(candidates(&harness), 1, "the match is listed");
    click(&mut harness, "Link");
    assert!(!present(&harness, "Clip A"), "the linked row leaves the table");
    assert!(present(&harness, "Clip B"), "the next row is shown");
    assert_eq!(candidates(&harness), 0, "the candidates are cleared");
    assert!(harness.get_by_label("Link").accesskit_node().is_disabled(), "nothing is selected to link yet");
    assert!(harness.state().result.is_none(), "Link keeps the dialog open");
}

// A candidate with a DifferentFile problem: the default button reads Link Anyway.
#[test]
fn link_anyway_replaces_link_for_a_failing_candidate() {
    let files = files(&[("/new/a.mov", OTHER)]);
    let jobs = Arc::new(ManualJobs::new());
    let mut harness = harness(vec![row(1, "Clip A", "/old/a.mov")], &files, &jobs, "/new", false);
    search(&mut harness, &jobs);
    assert_eq!(candidates(&harness), 1);
    assert!(present(&harness, "Link Anyway"));
    assert!(!present(&harness, "Link"));
    click(&mut harness, "Link Anyway");
    let (linked, _) = linked_keys(&done(&mut harness));
    assert_eq!(linked, [1], "Link Anyway links the failing file");
}

// Relinking `C:\shoot\a.mov` to `/T7/shoot/a.mov` links the other row under `C:\shoot` that passes; a failing one stays missing.
#[test]
fn relink_others_offers_the_rows_the_remap_fits() {
    let files = files(&[("/T7/shoot/a.mov", SAME), ("/T7/shoot/b.mov", SAME), ("/T7/shoot/c.mov", OTHER)]);
    let jobs = Arc::new(ManualJobs::new());
    let rows = vec![row(1, "Clip A", "C:\\shoot\\a.mov"), row(2, "Clip B", "C:\\shoot\\b.mov"), row(3, "Clip C", "C:\\shoot\\c.mov")];
    let mut harness = harness(rows, &files, &jobs, "/T7/shoot", true);
    search(&mut harness, &jobs);
    click(&mut harness, "Link");
    jobs.run_all();
    harness.run();
    assert!(!present(&harness, "Clip B"), "the row the remap fits is linked");
    assert!(present(&harness, "Clip C"), "the row that fails its check stays missing");
    let (linked, offline) = linked_keys(&done(&mut harness));
    assert_eq!((linked, offline), (vec![1, 2], vec![]));
}

// Done returns linked and offline rows; Cancel and Esc return Cancel.
#[test]
fn done_returns_every_link_and_cancel_returns_nothing() {
    let files = files(&[("/new/b.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let rows = || vec![row(1, "Clip A", "/old/a.mov"), row(2, "Clip B", "/old/b.mov")];
    let mut harness = harness(rows(), &files, &jobs, "/new", false);
    click(&mut harness, "Leave Offline");
    search(&mut harness, &jobs);
    click(&mut harness, "Link");
    let (linked, offline) = linked_keys(&done(&mut harness));
    assert_eq!((linked, offline), (vec![2], vec![1]));

    let mut harness = self::harness(rows(), &files, &jobs, "/new", false);
    click(&mut harness, "Leave All Offline");
    assert!(harness.state().result.is_none(), "leaving rows offline does not close the dialog");
    click(&mut harness, "Cancel");
    assert!(matches!(harness.state().result, Some(DialogResult::Cancel)));

    let mut harness = self::harness(rows(), &files, &jobs, "/new", false);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(matches!(harness.state().result, Some(DialogResult::Cancel)));
}

// While the search job waits in ManualJobs, the dialog draws and Stop cancels it.
#[test]
fn the_search_never_blocks_and_stop_cancels_it() {
    let files = files(&[("/new/a.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let mut harness = harness(vec![row(1, "Clip A", "/old/a.mov")], &files, &jobs, "/new", false);
    click(&mut harness, "Search");
    assert_eq!(jobs.waiting(), 1, "the search waits as a job");
    assert!(present(&harness, "Searching… 0 files"), "the progress is shown while it waits");
    click(&mut harness, "Stop");
    assert!(present(&harness, "Search"), "Stop gives the button back");
    jobs.run_all();
    harness.run();
    assert_eq!(candidates(&harness), 0, "a stopped search reports nothing");
}

// Browse… asks for a folder; the answer fills the Search in field.
#[test]
fn browse_fills_the_search_folder() {
    let files = files(&[("/new/a.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let setup = setup(&jobs, "", false);
    let picks = Arc::clone(&setup.picks);
    let mut harness = start(vec![row(1, "Clip A", "/old/a.mov")], &files, setup);
    click(&mut harness, "Browse…");
    let open = picks.open();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].kind, PickKind::Folder);
    assert!(harness.get_by_label("Search").accesskit_node().is_disabled(), "no folder yet");
    picks.answer(Picked::Paths(vec!["/new".into()]));
    harness.run();
    harness.run();
    assert_eq!(harness.get_by_role(Role::TextInput).accesskit_node().value().as_deref(), Some("/new"));
    assert!(!harness.get_by_label("Search").accesskit_node().is_disabled());
}

// Locate… picks one file; it is checked in a job and listed as the selected candidate.
#[test]
fn locate_adds_the_picked_file_as_the_candidate() {
    let files = files(&[("/elsewhere/b.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let setup = setup(&jobs, "/new", false);
    let picks = Arc::clone(&setup.picks);
    let mut harness = start(vec![row(1, "Clip A", "/old/b.mov")], &files, setup);
    click(&mut harness, "Locate…");
    assert_eq!(picks.open()[0].kind, PickKind::Open);
    picks.answer(Picked::Paths(vec!["/elsewhere/b.mov".into()]));
    harness.run();
    jobs.run_all();
    harness.run();
    assert_eq!(candidates(&harness), 1, "the located file is listed");
    click(&mut harness, "Link");
    let (linked, _) = linked_keys(&done(&mut harness));
    assert_eq!(linked, [1]);
}

// Esc while a pick is open closes the pick only: the dialog stays open, is not reopened.
#[test]
fn escape_with_a_pick_open_closes_the_pick_and_not_the_dialog() {
    let files = files(&[("/new/a.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let setup = setup(&jobs, "/new", false);
    let picks = Arc::clone(&setup.picks);
    let mut harness = start(vec![row(1, "Clip A", "/old/a.mov")], &files, setup);
    search(&mut harness, &jobs);
    click(&mut harness, "Browse…");
    assert!(picks.pick_pending());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!picks.pick_pending(), "the pick is dropped");
    assert!(harness.state().result.is_none(), "the dialog stays open");
    assert!(!harness.get_by_label("Link").accesskit_node().is_focused(), "a reopened dialog would focus its default button again");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(matches!(harness.state().result, Some(DialogResult::Cancel)), "a second Esc cancels the dialog");
}

// A search hit is checked once, inside the search job: one job, no check queue.
#[test]
fn a_search_is_one_job() {
    let files = files(&[("/new/a.mov", SAME)]);
    let jobs = Arc::new(ManualJobs::new());
    let mut harness = harness(vec![row(1, "Clip A", "/old/a.mov")], &files, &jobs, "/new", false);
    click(&mut harness, "Search");
    jobs.run_all();
    harness.run();
    assert_eq!(jobs.waiting(), 0, "hits need no second job");
    assert_eq!(candidates(&harness), 1, "the hit is listed");
}

// The columns sort and the sort reaches the host's layout handle.
#[test]
fn the_table_sorts_and_returns_its_layout() {
    let files = files(&[]);
    let jobs = Arc::new(ManualJobs::new());
    let mut setup = setup(&jobs, "/new", false);
    let layout = RelinkLayout::default();
    setup.options.layout = layout.clone();
    let mut harness = start(vec![row(1, "Clip A", "/old/a.mov"), row(2, "Clip B", "/old/b.mov")], &files, setup);
    let a_first = |harness: &Harness<'_, App>| harness.get_by_label("Clip A").rect().min.y < harness.get_by_label("Clip B").rect().min.y;
    assert!(a_first(&harness), "unsorted rows keep their order");
    for _ in 0..2 {
        harness.get_by(|n| n.role() == Role::ColumnHeader && n.label().is_some_and(|l| l.starts_with("Clip Name"))).click();
        harness.run();
    }
    assert!(!a_first(&harness), "a second click on the header sorts the other way round");
    assert!(layout.get().sort.is_some(), "the sort reaches the host's handle");
}

// The layout reaches the host's handle when the dialog is cancelled too.
#[test]
fn cancel_returns_the_changed_layout() {
    let files = files(&[]);
    let jobs = Arc::new(ManualJobs::new());
    let mut setup = setup(&jobs, "/new", false);
    let layout = RelinkLayout::default();
    setup.options.layout = layout.clone();
    let mut harness = start(vec![row(1, "Clip A", "/old/a.mov")], &files, setup);
    harness.get_by(|n| n.role() == Role::ColumnHeader && n.label().is_some_and(|l| l.starts_with("File Name"))).click();
    harness.run();
    click(&mut harness, "Cancel");
    assert!(matches!(harness.state().result, Some(DialogResult::Cancel)));
    assert_eq!(layout.get().sort.map(|s| s.column), Some(RelinkColumn::FileName));
}

// Without jobs and without a Host, Search stays disabled.
#[test]
fn without_jobs_search_is_disabled() {
    let files = files(&[("/new/a.mov", SAME)]);
    let options = RelinkOptions { folder: Some("/new".into()), ..RelinkOptions::default() };
    let dialog = RelinkDialog::new(vec![row(1, "Clip A", "/old/a.mov")], files, options);
    let mut stack = DialogStack::default();
    stack.push(dialog.id(), None);
    let mut harness = Harness::builder().with_size(vec2(900.0, 900.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui: &mut egui::Ui, app: &mut App| {
            if let Some(result) = app.dialog.show(&ui.ctx().clone(), &app.stack) {
                app.result = Some(result);
            }
        },
        App { dialog, stack, result: None },
    );
    harness.run();
    assert!(harness.get_by_label("Search").accesskit_node().is_disabled());
}
