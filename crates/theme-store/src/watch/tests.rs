#![cfg(not(target_arch = "wasm32"))]

use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use craft_theme::{
    ThemeId,
    v2::{ContrastChoice, ModeChoice, Selection, Size, TextSize, ThemePair, Variants},
};

use super::{Intervals, Op, OpOutput, SuiteWatch};
use crate::{
    Accepted, ParseCounts, Problem, SelectionChange, SelectionError, Snapshot, Store, StoreError, StoreOptions, SuiteDir, ThemeRecord,
    testing::{FailAt, FailingFs, SAMPLE_THEME_ID, SAMPLE_THEME_JSON},
};

const FAST: Intervals = Intervals { stat: Duration::from_millis(5), verify: Duration::from_millis(20) };

fn options() -> StoreOptions {
    StoreOptions::new("test 1.0").lock_timeout(Duration::from_millis(100))
}

fn id(s: &str) -> ThemeId {
    ThemeId::parse(s).unwrap()
}

/// `theme` in both slots, with the defaults for every field but the mode.
fn sel(theme: &str, mode: ModeChoice) -> Selection {
    Selection { theme: ThemePair::same(id(theme)), mode, contrast: ContrastChoice::System, size: Size::Compact, text_size: TextSize::Default }
}

fn same(theme: &str) -> Option<ThemePair> {
    Some(ThemePair::same(id(theme)))
}

/// The ids of a snapshot's records, in order.
fn ids(records: &[ThemeRecord]) -> Vec<&str> {
    records.iter().map(|r| r.theme.id.as_str()).collect()
}

fn write_selection(dir: &Path, base: &Accepted, change: SelectionChange) -> Accepted {
    let mut store = Store::open(&SuiteDir::at(dir), options());
    store.load_selection().unwrap();
    store.update(base, change).unwrap()
}

/// A dark-only theme file.
fn theme_json(id: &str) -> String {
    format!(
        r##"{{"schema":"craft-design-theme","schema_version":1,"id":"{id}","name":"{id}","variants":"dark",
        "seeds":{{"accent":"#4b8bf5","neutral":"#26262a","status":{{"good":"#3fd6b8","warn":"#ffc14d","bad":"#ff6259","info":"#4b8bf5"}}}}}}"##
    )
}

/// A wake callback and how often it ran.
fn counter() -> (Arc<AtomicUsize>, Box<dyn Fn() + Send + Sync>) {
    let wakes = Arc::new(AtomicUsize::new(0));
    let w = Arc::clone(&wakes);
    (wakes, Box::new(move || _ = w.fetch_add(1, Ordering::SeqCst)))
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn result(watch: &SuiteWatch, id: u64) -> Result<OpOutput, StoreError> {
    wait_for("the result", || watch.snapshot().result(id).is_some());
    watch.snapshot().result(id).unwrap().clone()
}

#[test]
fn unchanged_files_are_parsed_once_and_never_republished() {
    let tmp = tempfile::tempdir().unwrap();
    let library = tmp.path().join("library");
    fs::create_dir_all(&library).unwrap();
    for n in 0..10_000 {
        let id = format!("t{n:05}");
        fs::write(library.join(format!("{id}.json")), theme_json(&id)).unwrap();
    }
    write_selection(tmp.path(), &Accepted::default(), SelectionChange::whole(sel("t04242", ModeChoice::Dark)));

    let (wakes, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    let first = watch.snapshot();
    assert_eq!((first.generation, first.index.len()), (1, 10_000));
    assert_eq!(ids(&first.active), ["t04242"]);
    assert!(first.problems.is_empty());
    let loaded = ParseCounts { selection: 1, themes: 10_000, other_schema: 0 };
    assert_eq!(watch.parse_counts(), loaded);
    // A picker's preview cards come from the index, with no further reads.
    let preview = first.preview(&id("t00007")).unwrap();
    assert_eq!((preview.id.as_str(), preview.variants), ("t00007", Variants::DarkOnly));
    assert_eq!(watch.parse_counts(), loaded);
    // Without a worker the caller is in its frame already: no wake.
    assert_eq!(wakes.load(Ordering::SeqCst), 0);

    // 100 timer ticks, every 20th with the byte comparison (the 500 ms / 10 s ratio).
    for tick in 1..=100 {
        watch.poll_once(tick % 20 == 0);
    }
    assert_eq!(watch.parse_counts(), loaded);
    assert_eq!(watch.generation(), 1);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);

    // A picker opens after a file was added by hand: only the new file is read.
    fs::write(library.join("t10000.json"), theme_json("t10000")).unwrap();
    watch.request(Op::Rescan);
    assert_eq!(watch.parse_counts(), ParseCounts { selection: 1, themes: 10_001, other_schema: 0 });
    assert_eq!(watch.snapshot().index.len(), 10_001);
}

#[test]
fn an_external_write_bumps_the_generation_and_wakes_once() {
    let tmp = tempfile::tempdir().unwrap();
    let first = write_selection(tmp.path(), &Accepted::default(), SelectionChange::whole(sel("craft.graphite", ModeChoice::Dark)));
    let (wakes, wake) = counter();
    let watch = SuiteWatch::spawn(Store::open(&SuiteDir::at(tmp.path()), options()), wake, FAST);
    // Open read the file: the first poll and several stamp checks and byte comparisons find
    // nothing new.
    assert_eq!(watch.snapshot().accepted, first);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!((watch.generation(), wakes.load(Ordering::SeqCst)), (0, 0));

    write_selection(tmp.path(), &first, SelectionChange::mode(ModeChoice::Light));
    wait_for("the change", || watch.generation() == 1);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!((watch.generation(), wakes.load(Ordering::SeqCst)), (1, 1));
    let snapshot = watch.snapshot();
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.accepted.suite, sel("craft.graphite", ModeChoice::Light).into());
}

#[test]
fn a_same_metadata_replacement_is_found_by_the_byte_comparison() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    write_selection(tmp.path(), &Accepted::default(), SelectionChange::whole(sel("craft.graphite", ModeChoice::Dark)));
    let (_, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    assert_eq!(watch.generation(), 0);

    // Same length, same file, modification time put back: only the bytes differ.
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let replaced = text.replace("craft.graphite", "craft.artcraft");
    assert_eq!(replaced.len(), text.len());
    fs::write(&path, replaced).unwrap();
    fs::File::options().write(true).open(&path).unwrap().set_modified(modified).unwrap();

    watch.poll_once(false);
    assert_eq!(watch.generation(), 0, "the stamp check can't see this change");
    watch.poll_once(true);
    assert_eq!(watch.generation(), 1);
    assert_eq!(watch.snapshot().accepted.suite, sel("craft.artcraft", ModeChoice::Dark).into());
}

#[test]
fn damaged_files_are_problems_and_keep_the_last_good_state() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options());
    store.import(theme_json("one").as_bytes(), false).unwrap();
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::whole(sel("one", ModeChoice::Dark))).unwrap();
    let (_, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    let good = watch.snapshot();
    assert_eq!(ids(&good.active), ["one"]);

    // The active theme's file deleted by hand.
    fs::remove_file(tmp.path().join("library/one.json")).unwrap();
    watch.poll_once(false);
    let missing = watch.snapshot();
    assert_eq!(missing.generation, good.generation + 1);
    assert!(matches!(&missing.problems[..], [Problem::Theme { stem, error: StoreError::NotFound(_) }] if stem == "one"), "{:?}", missing.problems);
    assert_eq!(missing.accepted, good.accepted);
    assert_eq!(ids(&missing.active), ["one"]);
    assert!(missing.theme(&id("one")).is_some());

    // A damaged selection.json on top.
    fs::write(tmp.path().join("selection.json"), "{ damaged").unwrap();
    watch.poll_once(false);
    let damaged = watch.snapshot();
    assert_eq!(damaged.accepted, good.accepted);
    assert!(damaged.problems.iter().any(|p| matches!(p, Problem::Selection(SelectionError::Corrupt(_)))));
    // The same problems on the next ticks are not news.
    watch.poll_once(true);
    assert_eq!(watch.generation(), damaged.generation);
}

#[test]
fn operations_run_on_the_worker_and_report_their_results() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, wake) = counter();
    let watch = SuiteWatch::spawn(Store::open(&SuiteDir::at(tmp.path()), options()), wake, FAST);

    let base = watch.snapshot().accepted.clone();
    let request = watch.request(Op::Update { base, change: SelectionChange::whole(sel("craft.graphite", ModeChoice::Dark)) });
    assert!(matches!(result(&watch, request), Ok(OpOutput::Done)));
    let mine = watch.snapshot().accepted.clone();
    assert_eq!(mine.suite, sel("craft.graphite", ModeChoice::Dark).into());

    // Another app changes the mode; this app's mode change made on the old state conflicts.
    let theirs = write_selection(tmp.path(), &mine, SelectionChange::mode(ModeChoice::Light));
    let change = SelectionChange::mode(ModeChoice::System);
    let request = watch.request(Op::Update { base: mine.clone(), change });
    match result(&watch, request) {
        Err(StoreError::Conflict { disk, .. }) => assert_eq!(disk, theirs),
        other => panic!("{other:?}"),
    }
    assert_eq!(watch.snapshot().accepted, theirs);

    let request = watch.request(Op::Import { bytes: Arc::from(SAMPLE_THEME_JSON.as_bytes()), as_copy: false });
    assert!(matches!(result(&watch, request), Ok(OpOutput::Imported(t)) if t.as_str() == SAMPLE_THEME_ID));
    wait_for("the index", || watch.snapshot().index.iter().any(|e| e.id.as_str() == SAMPLE_THEME_ID));
    let request = watch.request(Op::Export(id(SAMPLE_THEME_ID)));
    assert!(matches!(result(&watch, request), Ok(OpOutput::Exported(bytes)) if &*bytes == SAMPLE_THEME_JSON.as_bytes()));
    watch.request(Op::AppTheme(same(SAMPLE_THEME_ID)));
    wait_for("the app theme", || !watch.snapshot().app_theme.is_empty());
    assert_eq!(ids(&watch.snapshot().app_theme), [SAMPLE_THEME_ID]);
}

#[test]
fn a_failed_write_leaves_the_accepted_state() {
    let tmp = tempfile::tempdir().unwrap();
    let before = write_selection(tmp.path(), &Accepted::default(), SelectionChange::whole(sel("craft.graphite", ModeChoice::Dark)));
    let failing = options().file_system(Box::new(FailingFs(FailAt::Rename)));
    let watch = SuiteWatch::open(&SuiteDir::at(tmp.path()), failing, || {});
    let request = watch.request(Op::Update { base: before.clone(), change: SelectionChange::whole(sel("craft.artcraft", ModeChoice::Light)) });
    assert!(matches!(result(&watch, request), Err(StoreError::Io(_))));
    assert_eq!(watch.snapshot().accepted, before);
}

#[test]
fn the_memory_watch_applies_operations_at_once() {
    let watch = SuiteWatch::memory();
    assert_eq!(watch.generation(), 0);
    let request = watch.request(Op::Import { bytes: Arc::from(SAMPLE_THEME_JSON.as_bytes()), as_copy: false });
    let snapshot = watch.snapshot();
    assert_eq!(snapshot.generation, 1);
    assert!(matches!(snapshot.result(request), Some(Ok(OpOutput::Imported(_)))));
    assert_eq!(snapshot.index.len(), 1);
    let sample = sel(SAMPLE_THEME_ID, ModeChoice::Light);
    let request = watch.request(Op::Update { base: snapshot.accepted.clone(), change: SelectionChange::whole(sample.clone()) });
    let snapshot = watch.snapshot();
    assert!(matches!(snapshot.result(request), Some(Ok(OpOutput::Done))));
    assert_eq!(snapshot.accepted.suite, sample.clone().into());
    assert!(snapshot.theme(&sample.theme.light).is_some());
    let request = watch.request(Op::Delete(sample.theme.light));
    assert!(matches!(watch.snapshot().result(request), Some(Err(StoreError::InUse(_)))));
}

#[cfg(unix)]
#[test]
fn an_unreadable_theme_in_use_is_read_again_until_it_reads() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("library/one.json");
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options());
    store.import(theme_json("one").as_bytes(), false).unwrap();
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::whole(sel("one", ModeChoice::Dark))).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&file).is_ok() {
        // Running as root: permissions don't stop reads.
        return;
    }
    let (_, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    let unreadable = watch.snapshot();
    assert!(unreadable.active.is_empty());
    assert!(matches!(&unreadable.problems[..], [Problem::Theme { error: StoreError::Io(_), .. }]), "{:?}", unreadable.problems);

    // Same stamp, readable now: the next tick reads it.
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    watch.poll_once(false);
    let readable = watch.snapshot();
    assert_eq!(ids(&readable.active), ["one"]);
    assert_eq!(readable.index.len(), 1);
    assert!(readable.problems.is_empty(), "{:?}", readable.problems);
}

#[test]
fn a_deleted_app_theme_clears_while_the_suite_theme_keeps_its_last_version() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options());
    store.import(theme_json("one").as_bytes(), false).unwrap();
    store.import(theme_json("two").as_bytes(), false).unwrap();
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::whole(sel("one", ModeChoice::Dark))).unwrap();
    let (_, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    let two = id("two");
    watch.request(Op::AppTheme(same("two")));
    assert_eq!(ids(&watch.snapshot().app_theme), ["two"]);
    // The same choice again is not news.
    let generation = watch.generation();
    watch.request(Op::AppTheme(same("two")));
    assert_eq!(watch.generation(), generation);

    fs::remove_file(tmp.path().join("library/one.json")).unwrap();
    fs::remove_file(tmp.path().join("library/two.json")).unwrap();
    watch.poll_once(false);
    let snapshot = watch.snapshot();
    assert_eq!(ids(&snapshot.active), ["one"]);
    assert!(snapshot.app_theme.is_empty());
    assert!(snapshot.theme(&two).is_none());
    let missing: Vec<_> = snapshot.problems.iter().filter(|p| matches!(p, Problem::Theme { error: StoreError::NotFound(_), .. })).collect();
    assert_eq!(missing.len(), 2, "{:?}", snapshot.problems);
}

#[test]
fn dropping_releases_the_worker_every_time() {
    let tmp = tempfile::tempdir().unwrap();
    for _ in 0..50 {
        let token = Arc::new(());
        let held = Arc::clone(&token);
        let watch = SuiteWatch::open(&SuiteDir::at(tmp.path()), options(), move || {
            let _ = &held;
        });
        watch.request(Op::Rescan);
        drop(watch);
        assert_eq!(Arc::strong_count(&token), 1);
    }
}

#[test]
fn dropping_stops_and_joins_the_worker() {
    let tmp = tempfile::tempdir().unwrap();
    let token = Arc::new(());
    let held = Arc::clone(&token);
    let watch = SuiteWatch::open(&SuiteDir::at(tmp.path()), options(), move || {
        let _ = &held;
    });
    watch.request(Op::Rescan);
    let start = Instant::now();
    drop(watch);
    assert!(start.elapsed() < Duration::from_secs(1));
    // The wake callback lives as long as the worker: only the test's own reference is left.
    assert_eq!(Arc::strong_count(&token), 1);
}

#[test]
fn a_watch_can_be_shared_between_threads() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<SuiteWatch>();
}

fn store_with_sample(dir: &Path) {
    Store::open(&SuiteDir::at(dir), options()).import(SAMPLE_THEME_JSON.as_bytes(), false).unwrap();
}

#[test]
fn a_library_theme_not_in_use_previews_from_the_index() {
    let tmp = tempfile::tempdir().unwrap();
    store_with_sample(tmp.path());
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), Box::new(|| {}));
    let snapshot = watch.snapshot();
    let sample = id(SAMPLE_THEME_ID);
    assert!(snapshot.active.is_empty() && snapshot.app_theme.is_empty() && snapshot.theme(&sample).is_none());

    let preview = snapshot.preview(&sample).unwrap();
    assert_eq!(preview, ThemeRecord::parse(SAMPLE_THEME_JSON.as_bytes()).unwrap().theme);
    assert_eq!((preview.name.as_str(), snapshot.index[0].version), ("Navy and cream", 3));
    assert!(snapshot.results.is_empty());
    assert_eq!(watch.parse_counts().themes, 1);
    assert!(snapshot.preview(&id("craft.graphite")).is_some());
    assert!(snapshot.preview(&id("gone")).is_none());
}

#[test]
fn an_app_theme_named_at_open_is_in_the_first_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    store_with_sample(tmp.path());
    let sample = id(SAMPLE_THEME_ID);
    // The light slot is a built-in: only the library theme is read.
    let pair = ThemePair { light: id("craft.pro-light"), dark: sample.clone() };
    let store = Store::open(&SuiteDir::at(tmp.path()), options().app_theme(Some(pair.clone())));
    // Built without a worker or a first poll: only what open reads itself.
    let (_, watch) = SuiteWatch::build(store, Box::new(|| {}));
    let first = watch.snapshot();
    assert_eq!(first.generation, 0);
    assert_eq!(ids(&first.app_theme), [SAMPLE_THEME_ID]);
    assert!(first.index.is_empty(), "open lists nothing");
    // selection.json (from the import) and the theme; nothing else.
    assert_eq!(watch.parse_counts(), ParseCounts { selection: 1, themes: 1, other_schema: 0 });

    let mut client = crate::SuiteClient::new(watch);
    assert!(!client.is_loading(&sample));
    assert!(client.theme(&sample).is_some());
    // Already requested: naming it again sends nothing.
    client.set_app_theme(Some(pair));
    assert_eq!(client.watch().generation(), 0);
}

#[test]
fn the_last_accepted_suite_theme_is_in_the_first_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    store_with_sample(tmp.path());
    // Damaged: the store keeps the last accepted state, and open reads its theme.
    fs::write(tmp.path().join("selection.json"), "{ damaged").unwrap();
    let sample = id(SAMPLE_THEME_ID);
    let last = Accepted::from_selection(sel(SAMPLE_THEME_ID, ModeChoice::Dark));
    let (_, watch) = SuiteWatch::build(Store::open(&SuiteDir::at(tmp.path()), options().last_accepted(last)), Box::new(|| {}));
    let first = watch.snapshot();
    assert_eq!(first.generation, 0);
    assert_eq!(ids(&first.active), [SAMPLE_THEME_ID]);
    assert!(first.theme(&sample).is_some());
}

#[test]
fn a_theme_missing_at_open_is_left_to_the_worker() {
    let tmp = tempfile::tempdir().unwrap();
    let gone = id("gone");
    let watch = SuiteWatch::open(&SuiteDir::at(tmp.path()), options().app_theme(same("gone")), || {});
    let mut client = crate::SuiteClient::new(watch);
    assert!(client.snapshot().app_theme.is_empty());
    wait_for("the worker's report", || {
        client.sync();
        client.problems().iter().any(|p| matches!(p, Problem::Theme { stem, error: StoreError::NotFound(_) } if stem == "gone"))
    });
    assert!(!client.is_loading(&gone));
}

/// selection.json with a chosen theme and mode, written three times: revision 3.
fn chosen_on_disk(dir: &Path) -> Accepted {
    let mut store = Store::open(&SuiteDir::at(dir), options());
    store.update(&Accepted::default(), SelectionChange::whole(sel("craft.artcraft", ModeChoice::System))).unwrap();
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::themes(ThemePair::same(id("craft.studio")))).unwrap();
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::whole(sel("craft.graphite", ModeChoice::Dark))).unwrap()
}

fn settle(client: &mut crate::SuiteClient) {
    wait_for("the change", || {
        client.sync();
        !client.is_pending()
    });
}

#[test]
fn a_pick_in_the_first_frame_merges_against_the_disk_state() {
    for change in [SelectionChange::mode(ModeChoice::Light), SelectionChange::themes(ThemePair::same(id("craft.artcraft")))] {
        let tmp = tempfile::tempdir().unwrap();
        let disk = chosen_on_disk(tmp.path());
        assert_eq!(disk.revision, 3);
        // No worker, no poll: generation 0 is all the client has.
        let (_, watch) = SuiteWatch::build(Store::open(&SuiteDir::at(tmp.path()), options()), Box::new(|| {}));
        let mut client = crate::SuiteClient::new(watch);
        assert_eq!((client.snapshot().generation, client.accepted()), (0, &disk));
        client.change(change.clone());
        settle(&mut client);
        assert!(client.conflict().is_none() && client.failure().is_none(), "{:?}", client.conflict());
        assert_eq!(client.accepted().revision, 4);
        assert_eq!(client.accepted().suite, change.applied_to(&disk.suite));
    }

    // The same with the worker running.
    let tmp = tempfile::tempdir().unwrap();
    chosen_on_disk(tmp.path());
    let mut client = crate::SuiteClient::new(SuiteWatch::open(&SuiteDir::at(tmp.path()), options(), || {}));
    client.change(SelectionChange::mode(ModeChoice::Light));
    settle(&mut client);
    assert!(client.conflict().is_none());
    assert_eq!(client.accepted().suite, sel("craft.graphite", ModeChoice::Light).into());
}

#[test]
fn the_first_poll_after_open_publishes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let disk = chosen_on_disk(tmp.path());
    let (_, watch) = SuiteWatch::build(Store::open(&SuiteDir::at(tmp.path()), options()), Box::new(|| {}));
    assert_eq!(watch.snapshot().accepted, disk);
    assert_eq!(watch.parse_counts().selection, 1);
    watch.poll_once(true);
    assert_eq!(watch.generation(), 0);
    assert_eq!(watch.parse_counts().selection, 1);
}

#[cfg(unix)]
#[test]
fn a_selection_unread_at_open_holds_changes_until_the_worker_reads_it() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("themes");
    chosen_on_disk(&dir);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::metadata(dir.join("selection.json")).is_ok() {
        // Running as root: permissions don't stop reads.
        return;
    }
    let (_, watch) = SuiteWatch::build(Store::open(&SuiteDir::at(&dir), options()), Box::new(|| {}));
    let mut client = crate::SuiteClient::new(watch);
    client.change(SelectionChange::mode(ModeChoice::Light));
    assert!(client.is_pending());
    assert!(client.watch().snapshot().results.is_empty(), "sent against a state nobody read");

    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    client.watch().poll_once(false);
    settle(&mut client);
    assert!(client.conflict().is_none() && client.failure().is_none());
    assert_eq!(client.accepted().revision, 4);
    assert_eq!(client.accepted().suite, sel("craft.graphite", ModeChoice::Light).into());
}

#[test]
fn each_suite_slot_follows_its_own_library_theme() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options());
    for name in ["one", "two", "three"] {
        store.import(theme_json(name).as_bytes(), false).unwrap();
    }
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::whole(sel("one", ModeChoice::System))).unwrap();
    let (_, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    // One theme in both slots is followed once, and read once (by open; the listing reuses it).
    assert_eq!(ids(&watch.snapshot().active), ["one"]);
    assert_eq!(watch.parse_counts().themes, 3);

    // Another app puts a different theme in the dark slot: both are followed, light first.
    // Only the new one is read.
    let base = watch.snapshot().accepted.clone();
    write_selection(tmp.path(), &base, SelectionChange::theme_dark(id("two")));
    watch.poll_once(false);
    let both = watch.snapshot();
    assert_eq!(ids(&both.active), ["one", "two"]);
    assert!(both.theme(&id("one")).is_some() && both.theme(&id("two")).is_some() && both.theme(&id("three")).is_none());
    assert_eq!(watch.parse_counts().themes, 4);

    // Each slot's file deleted by hand: the suite keeps both last versions, each reported.
    fs::remove_file(tmp.path().join("library/one.json")).unwrap();
    fs::remove_file(tmp.path().join("library/two.json")).unwrap();
    watch.poll_once(false);
    let missing = watch.snapshot();
    assert_eq!(ids(&missing.active), ["one", "two"]);
    let stems: Vec<_> = missing
        .problems
        .iter()
        .filter_map(|p| match p {
            Problem::Theme { stem, error: StoreError::NotFound(_) } => Some(stem.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stems, ["one", "two"]);

    // A built-in in the light slot: only the dark slot's theme is left to follow.
    write_selection(tmp.path(), &missing.accepted, SelectionChange::theme_light(id("craft.pro-light")));
    watch.poll_once(false);
    assert_eq!(ids(&watch.snapshot().active), ["two"]);
}

#[test]
fn a_pilot_theme_file_is_skipped_counted_and_read_once() {
    let tmp = tempfile::tempdir().unwrap();
    let library = tmp.path().join("library");
    fs::create_dir_all(&library).unwrap();
    fs::write(library.join("one.json"), theme_json("one")).unwrap();
    let pilot = r##"{"schema":"craft-theme","schema_version":1,"id":"old","dark":{"text":"#ffffff","background":"#000000"}}"##;
    fs::write(library.join("old.json"), pilot).unwrap();
    let (_, wake) = counter();
    let watch = SuiteWatch::inline(Store::open(&SuiteDir::at(tmp.path()), options()), wake);
    let snapshot: Arc<Snapshot> = watch.snapshot();
    assert_eq!(snapshot.index.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["one"]);
    assert!(snapshot.problems.is_empty(), "{:?}", snapshot.problems);
    assert!(snapshot.preview(&id("old")).is_none());
    let counts = ParseCounts { selection: 0, themes: 2, other_schema: 1 };
    assert_eq!(watch.parse_counts(), counts);
    // Unchanged, it is not read again when a picker opens.
    watch.request(Op::Rescan);
    assert_eq!(watch.parse_counts(), counts);
    assert_eq!(watch.generation(), snapshot.generation);
}
