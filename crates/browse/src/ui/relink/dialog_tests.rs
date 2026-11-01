//! `RelinkDialog` through egui_kittest with a fake `LinkFiles` and `ManualJobs` (task B10).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use craft_docpath::links::{FileInfo, FolderEntry, Link, LinkFiles, LinkIdentity};
use craft_host::{Jobs, ManualJobs};
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{RelinkDialog, RelinkOptions, RelinkOutcome, RelinkRow};

/// Files by path: size and fingerprint.
#[derive(Default)]
struct Fake(Mutex<BTreeMap<PathBuf, (u64, u64)>>);

impl Fake {
    fn put(&self, path: &str, size: u64, fingerprint: u64) {
        self.0.lock().unwrap().insert(path.into(), (size, fingerprint));
    }
}

const STAMP: Duration = Duration::from_secs(1);

impl LinkFiles for Fake {
    fn stat(&self, path: &Path) -> Option<FileInfo> {
        self.0.lock().unwrap().get(path).map(|&(size, _)| FileInfo { size, modified: Some(SystemTime::UNIX_EPOCH + STAMP) })
    }

    fn identity(&self, path: &Path) -> io::Result<LinkIdentity> {
        let &(size, fingerprint) = self.0.lock().unwrap().get(path).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok(LinkIdentity { size, fingerprint, modified: Some(SystemTime::UNIX_EPOCH + STAMP) })
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        let files = self.0.lock().unwrap();
        Ok(files
            .keys()
            .filter(|p| p.parent() == Some(dir))
            .map(|p| FolderEntry { path: p.clone(), name: p.file_name().unwrap().to_string_lossy().into_owned(), is_dir: false, is_link: false })
            .collect())
    }
}

struct App {
    dialog: RelinkDialog<u32>,
    stack: DialogStack,
    result: Option<DialogResult<RelinkOutcome<u32>>>,
}

fn row(key: u32, owner: &str, stored: &str) -> RelinkRow<u32> {
    let identity = LinkIdentity { size: 3, fingerprint: 7, modified: Some(SystemTime::UNIX_EPOCH + STAMP) };
    RelinkRow {
        link: Link { key, name: owner.into(), stored: stored.into(), identity: Some(identity), page: None, embedded: false },
        owner: owner.into(),
        status: "Media missing".into(),
    }
}

fn harness(rows: Vec<RelinkRow<u32>>, files: &Arc<Fake>, jobs: &Arc<ManualJobs>, folder: &str, relink_others: bool) -> Harness<'static, App> {
    let options =
        RelinkOptions { relink_others, folder: Some(folder.into()), jobs: Some(Arc::clone(jobs) as Arc<dyn Jobs>), ..RelinkOptions::default() };
    let dialog = RelinkDialog::new(rows, files.clone(), options);
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
    harness
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

// Link on the first row's candidate selects the second row and clears the candidates.
#[test]
fn the_relink_dialog_links_one_row_then_the_next() {
    let files = Arc::new(Fake::default());
    files.put("/new/a.mov", 3, 7);
    let jobs = Arc::new(ManualJobs::new());
    let mut harness = harness(vec![row(1, "Clip A", "/old/a.mov"), row(2, "Clip B", "/old/b.mov")], &files, &jobs, "/new", false);
    search(&mut harness, &jobs);
    assert_eq!(candidates(&harness), 1, "the match is listed");
    click(&mut harness, "Link");
    assert!(!present(&harness, "Clip A"), "the linked row leaves the table");
    assert!(present(&harness, "Clip B"), "the next row is shown");
    assert_eq!(candidates(&harness), 0, "the candidates are cleared");
    assert!(harness.get_by_label("Link").accesskit_node().is_disabled(), "nothing is selected to link yet");
}

// A candidate with a DifferentFile problem: the default button reads Link Anyway.
#[test]
fn link_anyway_replaces_link_for_a_failing_candidate() {
    let files = Arc::new(Fake::default());
    files.put("/new/a.mov", 9, 1);
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

/// Clicks Done and takes the dialog's answer.
fn done(harness: &mut Harness<'_, App>) -> DialogResult<RelinkOutcome<u32>> {
    click(harness, "Done");
    harness.state_mut().result.take().expect("Done answers")
}

// Relinking `C:\shoot\a.mov` to `/T7/shoot/a.mov` links the other row under `C:\shoot` that passes; a failing one stays missing.
#[test]
fn relink_others_offers_the_rows_the_remap_fits() {
    let files = Arc::new(Fake::default());
    files.put("/T7/shoot/a.mov", 3, 7);
    files.put("/T7/shoot/b.mov", 3, 7);
    files.put("/T7/shoot/c.mov", 9, 9);
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
    let files = Arc::new(Fake::default());
    files.put("/new/b.mov", 3, 7);
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
    let files = Arc::new(Fake::default());
    files.put("/new/a.mov", 3, 7);
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
