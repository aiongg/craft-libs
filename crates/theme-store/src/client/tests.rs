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
use craft_theme::{ContrastChoice, ModeChoice, SuiteChoice, TextSize, ThemeId, ThemePair};

use super::SuiteClient;
use crate::{
    Accepted, Op, SelectionChange, Store, StoreError, StoreOptions, SuiteDir, SuiteWatch,
    testing::{SAMPLE_THEME_ID, SAMPLE_THEME_JSON},
};

fn options() -> StoreOptions {
    StoreOptions::new("test 1.0").lock_timeout(Duration::from_millis(100))
}

fn id(s: &str) -> ThemeId {
    ThemeId::parse(s).unwrap()
}

/// `theme` in both slots.
fn theme(s: &str) -> SelectionChange {
    SelectionChange::themes(ThemePair::same(id(s)))
}

/// A suite choice of `theme` in both slots and `mode`, nothing else.
fn choice(theme: &str, mode: ModeChoice) -> SuiteChoice {
    SuiteChoice { theme_light: Some(id(theme)), theme_dark: Some(id(theme)), mode: Some(mode), ..SuiteChoice::default() }
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

/// The limit only ends a hung test; the built-in themes are parsed before the clock starts (see
/// `watch::tests::wait_for`).
fn settle(client: &mut SuiteClient) {
    let _ = craft_theme::builtins();
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
    client.change(theme("craft.graphite"));
    client.change(SelectionChange::mode(ModeChoice::Dark));
    assert_eq!(client.shown(), choice("craft.graphite", ModeChoice::Dark));
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", ModeChoice::Dark));
    assert!(client.conflict().is_none() && client.failure().is_none());

    // Theme, then theme, then theme: the last one wins, in two writes.
    client.change(theme("craft.studio"));
    client.change(theme("craft.artcraft"));
    client.change(theme("craft.graphite"));
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", ModeChoice::Dark));
    assert_eq!(client.accepted().revision, 4);
    assert!(client.conflict().is_none() && client.failure().is_none());

    // The same with a worker thread.
    let tmp = tempfile::tempdir().unwrap();
    let mut threaded = SuiteClient::new(SuiteWatch::open(&SuiteDir::at(tmp.path()), options(), || {}));
    threaded.sync();
    threaded.change(theme("craft.graphite"));
    threaded.change(SelectionChange::mode(ModeChoice::Light));
    settle(&mut threaded);
    assert_eq!(threaded.accepted().suite, choice("craft.graphite", ModeChoice::Light));
    assert!(threaded.conflict().is_none() && threaded.failure().is_none());

    // Picks of different fields while one is on its way merge into one queued change.
    client.change(SelectionChange::theme_light(id("craft.soft")));
    client.change(SelectionChange::contrast(ContrastChoice::High));
    client.change(SelectionChange::text_size(TextSize::Large));
    let expected = SuiteChoice {
        theme_light: Some(id("craft.soft")),
        contrast: Some(ContrastChoice::High),
        text_size: Some(TextSize::Large),
        ..choice("craft.graphite", ModeChoice::Dark)
    };
    assert_eq!(client.shown(), expected);
    settle(&mut client);
    assert_eq!((&client.accepted().suite, client.accepted().revision), (&expected, 6));
    assert!(client.conflict().is_none() && client.failure().is_none());
}

#[test]
fn a_conflict_is_kept_until_apply_mine() {
    let tmp = tempfile::tempdir().unwrap();
    write_elsewhere(tmp.path(), theme("craft.graphite").then(SelectionChange::mode(ModeChoice::Dark)));
    let mut client = inline(tmp.path(), options());
    assert_eq!(client.accepted().suite, choice("craft.graphite", ModeChoice::Dark));

    // Another app changes the mode after this client's last look.
    let theirs = write_elsewhere(tmp.path(), SelectionChange::mode(ModeChoice::System));
    client.change(SelectionChange::mode(ModeChoice::Light));
    settle(&mut client);
    let (disk, mine) = client.conflict().cloned().unwrap();
    assert_eq!((disk, mine), (theirs.clone(), SelectionChange::mode(ModeChoice::Light)));
    assert_eq!(client.shown(), theirs.suite);

    client.apply_mine();
    assert!(client.conflict().is_none());
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", ModeChoice::Light));
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
    client.change(theme("craft.graphite"));
    // A second pick queued behind the failing one doesn't hide its error.
    client.change(SelectionChange::mode(ModeChoice::Dark));
    settle(&mut client);
    let (failed, error) = client.failure().cloned().unwrap();
    assert_eq!(failed, theme("craft.graphite"));
    assert!(matches!(error, StoreError::Io(_)));
    assert_eq!(client.accepted().suite, SuiteChoice { mode: Some(ModeChoice::Dark), ..SuiteChoice::default() });

    client.retry();
    assert!(client.failure().is_none());
    settle(&mut client);
    assert_eq!(client.accepted().suite, choice("craft.graphite", ModeChoice::Dark));
    assert!(client.failure().is_none());
}

#[test]
fn a_result_pushed_out_before_sync_is_a_failure_unless_the_change_shows() {
    let tmp = tempfile::tempdir().unwrap();
    let fail = Arc::new(AtomicBool::new(true));
    let mut client = inline(tmp.path(), options().file_system(Box::new(FailOnce(Arc::clone(&fail)))));
    let export_many = |client: &SuiteClient| {
        for _ in 0..=crate::RESULTS_KEPT {
            client.watch().request(Op::Export(id("craft.graphite")));
        }
    };

    // The write fails, and the app's exports push its result out before the next frame.
    client.change(theme("craft.graphite"));
    export_many(&client);
    client.sync();
    assert!(!client.is_pending());
    let (failed, error) = client.failure().cloned().unwrap();
    assert_eq!(failed, theme("craft.graphite"));
    assert!(matches!(error, StoreError::Lost));

    client.retry();
    settle(&mut client);
    assert_eq!(client.accepted().suite.theme_light, Some(id("craft.graphite")));

    // A write that went through shows in the accepted state: no notice.
    client.change(SelectionChange::mode(ModeChoice::Dark));
    export_many(&client);
    client.sync();
    assert_eq!(client.accepted().suite, choice("craft.graphite", ModeChoice::Dark));
    assert!(client.failure().is_none() && !client.is_pending());
}

#[test]
fn a_library_theme_on_its_way_never_shows_the_app_default() {
    let mut client = SuiteClient::memory();
    client.sync();
    client.watch().request(Op::Import { bytes: Arc::from(SAMPLE_THEME_JSON.as_bytes()), as_copy: false });
    client.change(theme("craft.graphite"));
    settle(&mut client);
    let sample = id(SAMPLE_THEME_ID);

    // Picked for the dark slot: the previous theme stays there until the record is there, and
    // the light slot is untouched.
    client.change(SelectionChange::theme_dark(sample.clone()));
    assert!(client.theme(&sample).is_none());
    assert!(client.is_loading(&sample));
    assert_eq!((client.shown().theme_light, client.shown().theme_dark), (Some(id("craft.graphite")), Some(id("craft.graphite"))));
    settle(&mut client);
    assert_eq!((client.shown().theme_light, client.shown().theme_dark), (Some(id("craft.graphite")), Some(sample.clone())));
    assert!(client.theme(&sample).is_some());
    assert!(!client.is_loading(&sample));

    // Picked for the app alone: loading until the snapshot has it.
    let mut own = SuiteClient::memory();
    own.sync();
    own.watch().request(Op::Import { bytes: Arc::from(SAMPLE_THEME_JSON.as_bytes()), as_copy: false });
    own.set_app_theme(Some(ThemePair::same(sample.clone())));
    assert!(own.is_loading(&sample));
    own.sync();
    assert!(own.theme(&sample).is_some());
    assert!(!own.is_loading(&sample));
    // A theme that can't load is not "loading" forever.
    own.set_app_theme(Some(ThemePair::same(id("gone"))));
    own.sync();
    assert!(!own.is_loading(&id("gone")));
}
