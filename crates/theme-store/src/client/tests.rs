#![cfg(not(target_arch = "wasm32"))]

use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use craft_atomic::{AtomicFs, NewFilePermissions, RealFs};
use craft_theme::{Mode, Selection, ThemeId};

use super::SuiteClient;
use crate::{
    Accepted, Op, SelectionChange, Store, StoreError, StoreOptions, SuiteChoice, SuiteDir, SuiteWatch,
    testing::{SAMPLE_THEME_ID, SAMPLE_THEME_JSON},
};

fn options() -> StoreOptions {
    StoreOptions::new("test 1.0").lock_timeout(Duration::from_millis(100))
}

fn id(s: &str) -> ThemeId {
    ThemeId::parse(s).unwrap()
}

fn choice(theme: &str, mode: Mode) -> SuiteChoice {
    Selection { theme: id(theme), mode }.into()
}

/// A client over the folder with no worker thread, so every step is deterministic.
fn inline(dir: &Path, options: StoreOptions) -> SuiteClient {
    let mut client = SuiteClient::new(SuiteWatch::inline(Store::open(&SuiteDir::at(dir), options), Box::new(|| {})));
    client.sync();
    client
}

fn write_elsewhere(dir: &Path, change: SelectionChange) -> Accepted {
    let mut store = Store::open(&SuiteDir::at(dir), options());
    store.load_selection().unwrap();
    let base = store.accepted().clone();
    store.update(&base, change).unwrap()
}

fn settle(client: &mut SuiteClient) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while client.is_pending() {
        assert!(Instant::now() < deadline, "the change never settled");
        client.sync();
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn quick_picks_never_conflict_with_each_other() {
    // Theme, then mode: both land.
    let mut client = SuiteClient::memory();
    client.sync();
    client.change(SelectionChange::theme(id("craft.graphite")));
    client.change(SelectionChange::mode(Mode::Dark));
    assert_eq!(client.shown(), choice("craft.graphite", Mode::Dark));
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", Mode::Dark));
    assert!(client.conflict().is_none() && client.failure().is_none());

    // Theme, then theme, then theme: the last one wins, in two writes.
    client.change(SelectionChange::theme(id("craft.high-contrast")));
    client.change(SelectionChange::theme(id("craft.artcraft")));
    client.change(SelectionChange::theme(id("craft.graphite")));
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", Mode::Dark));
    assert_eq!(client.accepted().revision, 4);
    assert!(client.conflict().is_none() && client.failure().is_none());

    // The same with a worker thread.
    let tmp = tempfile::tempdir().unwrap();
    let mut threaded = SuiteClient::new(SuiteWatch::open(&SuiteDir::at(tmp.path()), options(), || {}));
    threaded.sync();
    threaded.change(SelectionChange::theme(id("craft.graphite")));
    threaded.change(SelectionChange::mode(Mode::Light));
    settle(&mut threaded);
    assert_eq!(threaded.accepted().suite, choice("craft.graphite", Mode::Light));
    assert!(threaded.conflict().is_none() && threaded.failure().is_none());
}

#[test]
fn a_conflict_is_kept_until_apply_mine() {
    let tmp = tempfile::tempdir().unwrap();
    write_elsewhere(tmp.path(), SelectionChange::whole(Selection { theme: id("craft.graphite"), mode: Mode::Dark }));
    let mut client = inline(tmp.path(), options());
    assert_eq!(client.accepted().suite, choice("craft.graphite", Mode::Dark));

    // Another app changes the mode after this client's last look.
    let theirs = write_elsewhere(tmp.path(), SelectionChange::mode(Mode::System));
    client.change(SelectionChange::mode(Mode::Light));
    settle(&mut client);
    let (disk, mine) = client.conflict().cloned().unwrap();
    assert_eq!((disk, mine), (theirs.clone(), SelectionChange::mode(Mode::Light)));
    assert_eq!(client.shown(), theirs.suite);

    client.apply_mine();
    assert!(client.conflict().is_none());
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", Mode::Light));
    assert!(client.conflict().is_none() && client.failure().is_none());
}

/// The real file system, failing the first temporary file it creates.
struct FailOnce(Arc<AtomicBool>);

impl AtomicFs for FailOnce {
    fn create_new(&self, path: &Path, p: NewFilePermissions) -> std::io::Result<fs::File> {
        if self.0.swap(false, Ordering::SeqCst) {
            return Err(std::io::Error::other("injected failure"));
        }
        RealFs.create_new(path, p)
    }
    fn sync(&self, file: &fs::File) -> std::io::Result<()> {
        RealFs.sync(file)
    }
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        RealFs.rename(from, to)
    }
    fn hard_link(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        RealFs.hard_link(from, to)
    }
    fn remove(&self, path: &Path) -> std::io::Result<()> {
        RealFs.remove(path)
    }
    fn sync_dir(&self, dir: &Path) -> std::io::Result<()> {
        RealFs.sync_dir(dir)
    }
    fn set_permissions(&self, file: &fs::File, perms: fs::Permissions) -> std::io::Result<()> {
        RealFs.set_permissions(file, perms)
    }
}

#[test]
fn a_failed_write_is_kept_until_retry() {
    let tmp = tempfile::tempdir().unwrap();
    let fail = Arc::new(AtomicBool::new(true));
    let mut client = inline(tmp.path(), options().file_system(Box::new(FailOnce(Arc::clone(&fail)))));
    client.change(SelectionChange::theme(id("craft.graphite")));
    // A second pick queued behind the failing one doesn't hide its error.
    client.change(SelectionChange::mode(Mode::Dark));
    settle(&mut client);
    let (failed, error) = client.failure().cloned().unwrap();
    assert_eq!(failed, SelectionChange::theme(id("craft.graphite")));
    assert!(matches!(error, StoreError::Io(_)));
    assert_eq!(client.accepted().suite, SuiteChoice { theme: None, mode: Some(Mode::Dark) });

    client.retry();
    assert!(client.failure().is_none());
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", Mode::Dark));
    assert!(client.failure().is_none());
}

#[test]
fn a_library_theme_on_its_way_never_shows_the_app_default() {
    let mut client = SuiteClient::memory();
    client.sync();
    client.watch().request(Op::Import { bytes: Arc::from(SAMPLE_THEME_JSON.as_bytes()), as_copy: false });
    client.change(SelectionChange::theme(id("craft.graphite")));
    settle(&mut client);
    let sample = id(SAMPLE_THEME_ID);

    // Picked for the suite: the previous theme stays until the record is there.
    client.change(SelectionChange::theme(sample.clone()));
    assert!(client.theme(&sample).is_none());
    assert!(client.is_loading(&sample));
    assert_eq!(client.shown().theme, Some(id("craft.graphite")));
    settle(&mut client);
    assert_eq!(client.shown().theme, Some(sample.clone()));
    assert!(client.theme(&sample).is_some());
    assert!(!client.is_loading(&sample));

    // Picked for the app alone: loading until the snapshot has it.
    let mut own = SuiteClient::memory();
    own.sync();
    own.watch().request(Op::Import { bytes: Arc::from(SAMPLE_THEME_JSON.as_bytes()), as_copy: false });
    own.set_app_theme(Some(sample.clone()));
    assert!(own.is_loading(&sample));
    own.sync();
    assert!(own.theme(&sample).is_some());
    assert!(!own.is_loading(&sample));
    // A theme that can't load is not "loading" forever.
    own.set_app_theme(Some(id("gone")));
    own.sync();
    assert!(!own.is_loading(&id("gone")));
}
