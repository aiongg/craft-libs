#![cfg(not(target_arch = "wasm32"))]

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use craft_theme::{
    Appearance, Contrast, ContrastChoice, ContrastIntent, IdError, Mode, ModeChoice, Selection, Size, SuiteChoice, TextSize, ThemeId, ThemePair,
    builtins, resolve,
};
use serde_json::Value;

use crate::{
    Accepted, FormatErrors, Problem, SelectionChange, SelectionError, Store, StoreError, StoreOptions, SuiteDir, ThemeRecord,
    copy::replace_top_level_id,
    format::ErrorKind,
    testing::{FailAt, FailingFs, SAMPLE_THEME_ID, SAMPLE_THEME_JSON, sample_colors},
};

fn options() -> StoreOptions {
    StoreOptions::new("test 1.0").lock_timeout(Duration::from_millis(100))
}

fn open(dir: &Path) -> Store {
    Store::open(&SuiteDir::at(dir), options())
}

fn id(s: &str) -> ThemeId {
    ThemeId::parse(s).unwrap()
}

/// `theme` in both slots, with the defaults for every field but the mode.
fn sel(theme: &str, mode: ModeChoice) -> Selection {
    Selection { theme: ThemePair::same(id(theme)), mode, contrast: ContrastChoice::System, size: Size::Compact, text_size: TextSize::Default }
}

fn whole(theme: &str, mode: ModeChoice) -> SelectionChange {
    SelectionChange::whole(sel(theme, mode))
}

fn theme(s: &str) -> SelectionChange {
    SelectionChange::themes(ThemePair::same(id(s)))
}

fn selection_json(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join("selection.json")).unwrap()).unwrap()
}

/// A dark-only theme file with `extra` members after the id.
fn theme_json(id: &str, extra: &str) -> String {
    format!(
        r##"{{"schema":"craft-design-theme","schema_version":1,"id":"{id}","name":"{id}"{extra},"variants":"dark",
        "seeds":{{"accent":"#4b8bf5","neutral":"#26262a","status":{{"good":"#3fd6b8","warn":"#ffc14d","bad":"#ff6259","info":"#4b8bf5"}}}}}}"##
    )
}

/// The pilot format, which the library skips.
fn pilot_json(id: &str) -> String {
    format!(r##"{{"schema":"craft-theme","schema_version":1,"id":"{id}","dark":{{"text":"#ffffff","background":"#000000"}}}}"##)
}

fn file_kind(e: &FormatErrors) -> &ErrorKind {
    &e.0[0].kind
}

#[test]
fn a_store_can_move_to_a_worker_thread() {
    fn is_send<T: Send>() {}
    is_send::<Store>();
}

#[test]
fn reads_never_create_files_and_no_file_means_no_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("themes");
    let mut store = open(&dir);
    assert!(!store.load_selection().unwrap());
    assert_eq!(store.accepted(), &Accepted::default());
    assert!(store.library().entries.is_empty());
    assert!(!dir.exists());

    let mine = sel("craft.graphite", ModeChoice::Dark);
    let accepted = store.update(&Accepted::default(), SelectionChange::whole(mine.clone())).unwrap();
    assert_eq!(accepted, Accepted { suite: mine.into(), revision: 1, library_revision: 0 });
    assert_eq!(selection_json(&dir)["written_by"], "test 1.0");
    assert_eq!(selection_json(&dir)["schema_version"], 2);
    assert!(dir.join("selection.lock").exists());
}

#[test]
fn memory_store_runs_the_same_protocol() {
    let mut store = Store::open(&SuiteDir::memory(), options());
    assert!(store.dir().is_none());
    let imported = store.import(SAMPLE_THEME_JSON.as_bytes(), false).unwrap();
    assert_eq!(imported.accepted, Accepted { suite: SuiteChoice::default(), revision: 1, library_revision: 1 });
    let sample = id(SAMPLE_THEME_ID);
    assert_eq!(&*store.export(&sample).unwrap(), SAMPLE_THEME_JSON.as_bytes());
    let base = store.accepted().clone();
    store.update(&base, whole(SAMPLE_THEME_ID, ModeChoice::Light)).unwrap();
    assert!(matches!(store.delete(&sample), Err(StoreError::InUse(_))));
    let theme = store.theme(&sample).unwrap();
    for mode in [Mode::Light, Mode::Dark] {
        let appearance = Appearance { mode, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
        let tokens = resolve(&theme, appearance, None).tokens;
        for (token, colour) in sample_colors(mode) {
            assert_eq!(tokens.color.get(token), colour, "{mode:?} {}", token.path());
        }
    }
    assert_eq!(store.library().entries.len(), 1);
}

#[test]
fn imports_keep_unknown_data_byte_for_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = open(tmp.path());
    let imported = store.import(SAMPLE_THEME_JSON.as_bytes(), false).unwrap();
    assert_eq!((imported.record.theme.name.as_str(), imported.record.theme.version), ("Navy and cream", 3));
    let stored = fs::read(tmp.path().join("library").join(format!("{SAMPLE_THEME_ID}.json"))).unwrap();
    assert_eq!(stored, SAMPLE_THEME_JSON.as_bytes());
    // A library op writes no suite selection.
    assert!(selection_json(tmp.path()).get("suite").is_none());
    assert_eq!(store.accepted().library_revision, 1);
}

#[test]
fn taken_ids_need_an_explicit_copy_that_changes_only_the_id() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = open(tmp.path());
    store.import(SAMPLE_THEME_JSON.as_bytes(), false).unwrap();
    let again = store.import(SAMPLE_THEME_JSON.as_bytes(), false);
    assert!(matches!(again, Err(StoreError::IdExists(ref i)) if i.as_str() == SAMPLE_THEME_ID));
    let copy = store.import(SAMPLE_THEME_JSON.as_bytes(), true).unwrap();
    assert_eq!(copy.record.theme.id.as_str(), "sample.navy-cream-2");
    let stored = fs::read_to_string(tmp.path().join("library/sample.navy-cream-2.json")).unwrap();
    assert_eq!(stored, SAMPLE_THEME_JSON.replace("\"id\": \"sample.navy-cream\"", "\"id\": \"sample.navy-cream-2\""));
    assert_eq!(store.import(SAMPLE_THEME_JSON.as_bytes(), true).unwrap().record.theme.id.as_str(), "sample.navy-cream-3");
    assert_eq!(store.accepted().library_revision, 3);
    // Built-in ids are refused outright, copy or not.
    let builtin = theme_json("craft.mine", "");
    assert!(matches!(store.import(builtin.as_bytes(), true), Err(StoreError::Format(ref e)) if *file_kind(e) == ErrorKind::BadId(IdError::Reserved)));
}

#[test]
fn user_files_must_be_this_format_with_a_user_id() {
    let refuse = |s: &str| ThemeRecord::parse(s.as_bytes()).unwrap_err();
    assert!(matches!(refuse(&pilot_json("old")).file_level(), Some(ErrorKind::OtherSchema { schema }) if schema == "craft-theme"));
    assert_eq!(*file_kind(&refuse(&theme_json("craft.mine", ""))), ErrorKind::BadId(IdError::Reserved));
    assert_eq!(*file_kind(&refuse(&theme_json("con", ""))), ErrorKind::BadId(IdError::DeviceName));
    assert_eq!(refuse(&theme_json("craft.mine", "")).0[0].path, ["id"]);
    // The pilot file is refused on import too, and nothing is written.
    let tmp = tempfile::tempdir().unwrap();
    let mut store = open(tmp.path());
    assert!(matches!(store.import(pilot_json("old").as_bytes(), false), Err(StoreError::Format(_))));
    assert!(!tmp.path().join("library").exists());
}

#[test]
fn low_contrast_imports_succeed_with_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = open(tmp.path());
    let flat = theme_json("flat", r##","contrast_intent":"low","dark":{"text.normal":"#777777","surface.panel":"#777777"}"##);
    let imported = store.import(flat.as_bytes(), false).unwrap();
    assert!(imported.record.warnings > 0);
    let entry = &store.library().entries[0];
    assert_eq!((entry.warnings, entry.contrast_intent), (imported.record.warnings, ContrastIntent::Low));
    let base = store.accepted().clone();
    store.update(&base, whole("flat", ModeChoice::Dark)).unwrap();
    // Built-ins with standard intent resolve with none, so the count is about the theme.
    let graphite = store.export(&id("craft.graphite")).unwrap();
    let renamed = replace_top_level_id(&graphite, &id("graphite-copy")).unwrap();
    assert_eq!(ThemeRecord::parse(&renamed).unwrap().warnings, 0);
}

#[test]
fn builtins_export_and_parse_back() {
    let store = Store::open(&SuiteDir::memory(), options());
    for builtin in builtins() {
        let exported = store.export(&builtin.id).unwrap();
        // Built-in ids are reserved, so a re-import needs a user id.
        let renamed = replace_top_level_id(&exported, &id("copy")).unwrap();
        let record = ThemeRecord::parse(&renamed).unwrap();
        assert_eq!(record.theme.id.as_str(), "copy");
        assert_eq!(record.theme, craft_theme::ThemeDef { id: id("copy"), ..builtin.clone() });
    }
}

#[test]
fn delete_rules() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = open(tmp.path());
    store.import(theme_json("one", "").as_bytes(), false).unwrap();
    store.import(theme_json("two", "").as_bytes(), false).unwrap();
    store.import(theme_json("night", "").as_bytes(), false).unwrap();
    let base = store.accepted().clone();
    store.update(&base, whole("one", ModeChoice::System)).unwrap();
    assert!(matches!(store.delete(&id("one")), Err(StoreError::InUse(_))));
    assert!(matches!(store.delete(&id("craft.graphite")), Err(StoreError::BuiltIn(_))));
    assert!(matches!(store.delete(&id("three")), Err(StoreError::NotFound(_))));
    // A theme in the dark slot only is in use too.
    let base = store.accepted().clone();
    store.update(&base, SelectionChange::theme_dark(id("night"))).unwrap();
    assert!(matches!(store.delete(&id("night")), Err(StoreError::InUse(_))));
    let accepted = store.delete(&id("two")).unwrap();
    assert_eq!((accepted.revision, accepted.library_revision), (6, 4));
    assert!(!tmp.path().join("library/two.json").exists());
    // Built-ins export as theme files; the app default has no colours to export.
    let graphite = store.export(&id("craft.graphite")).unwrap();
    assert!(String::from_utf8_lossy(&graphite).contains("\"id\": \"craft.graphite\""));
    assert!(matches!(store.export(&id("craft.app-default")), Err(StoreError::NotFound(_))));
}

#[test]
fn unknown_selection_keys_survive_an_update() {
    let tmp = tempfile::tempdir().unwrap();
    let text = r##"{"schema_version":2,"revision":7,"library_revision":2,"later":{"x":[1,2]},
        "suite":{"theme":{"light":"craft.artcraft","dark":"craft.artcraft"},"mode":"system","accent_override":"#ff0000"},"written_by":"other 2.0"}"##;
    fs::write(tmp.path().join("selection.json"), text).unwrap();
    let mut store = open(tmp.path());
    store.load_selection().unwrap();
    let base = store.accepted().clone();
    let accepted = store.update(&base, whole("craft.artcraft", ModeChoice::Dark)).unwrap();
    assert_eq!((accepted.revision, accepted.library_revision), (8, 2));
    let disk = selection_json(tmp.path());
    assert_eq!(disk["later"], serde_json::json!({"x": [1, 2]}));
    assert_eq!(disk["suite"]["accent_override"], "#ff0000");
    assert_eq!(disk["suite"]["mode"], "dark");
}

#[test]
fn a_version_1_selection_is_read_and_written_back_as_version_2() {
    let tmp = tempfile::tempdir().unwrap();
    let v1 = r#"{"schema_version":1,"revision":3,"library_revision":1,"suite":{"theme":"craft.graphite","mode":"light"},"written_by":"pilot"}"#;
    fs::write(tmp.path().join("selection.json"), v1).unwrap();
    let mut store = open(tmp.path());
    assert!(store.load_selection().unwrap());
    let base = store.accepted().clone();
    let graphite = Some(id("craft.graphite"));
    assert_eq!(
        base.suite,
        SuiteChoice { theme_light: graphite.clone(), theme_dark: graphite.clone(), mode: Some(ModeChoice::Light), ..SuiteChoice::default() }
    );
    let next = store.update(&base, SelectionChange::contrast(ContrastChoice::High)).unwrap();
    assert_eq!(next.revision, 4);
    let disk = selection_json(tmp.path());
    assert_eq!(disk["schema_version"], 2);
    assert_eq!(
        disk["suite"],
        serde_json::json!({"theme": {"light": "craft.graphite", "dark": "craft.graphite"}, "mode": "light", "contrast": "high"})
    );
    // Read back by another store as version 2.
    let mut other = open(tmp.path());
    other.load_selection().unwrap();
    assert_eq!(other.accepted(), &next);
}

#[test]
fn a_newer_selection_is_never_overwritten() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    let last = Accepted::from_selection(sel("craft.graphite", ModeChoice::Light));
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options().last_accepted(last.clone()));
    let future = r#"{"schema_version":3,"suite":{"theme":"craft.graphite","mode":"sepia"}}"#;
    fs::write(&path, future).unwrap();
    assert!(matches!(store.load_selection(), Err(Problem::Selection(SelectionError::TooNew { schema_version: 3 }))));
    assert!(store.is_read_only());
    assert_eq!(store.accepted(), &last);
    let refused = store.update(&last, whole("craft.graphite", ModeChoice::Dark));
    assert!(matches!(refused, Err(StoreError::Selection(SelectionError::TooNew { .. }))));
    assert!(store.import(SAMPLE_THEME_JSON.as_bytes(), false).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), future);
    assert!(!tmp.path().join("library").exists());
}

#[test]
fn a_damaged_selection_keeps_the_last_state_and_moves_aside_only_when_written() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    let mut store = open(tmp.path());
    store.import(theme_json("one", "").as_bytes(), false).unwrap();
    let base = store.accepted().clone();
    let base = store.update(&base, whole("one", ModeChoice::Light)).unwrap();
    // Still JSON, so its counters are salvaged.
    let damaged = r#"{"schema_version":2,"revision":40,"library_revision":7,"suite":{"theme":5}}"#;
    fs::write(&path, damaged).unwrap();
    assert!(matches!(store.load_selection(), Err(Problem::Selection(SelectionError::Corrupt(_)))));
    assert_eq!(store.accepted(), &base);

    // Paths that write nothing leave it where it is: a change that matches, InUse, IdExists, a
    // failed write.
    store.update(&base, SelectionChange::mode(ModeChoice::Light)).unwrap();
    assert!(matches!(store.delete(&id("one")), Err(StoreError::InUse(_))));
    assert!(matches!(store.import(theme_json("one", "").as_bytes(), false), Err(StoreError::IdExists(_))));
    // Moved aside, then the write fails creating its temporary file: put back.
    let mut failing = Store::open(&SuiteDir::at(tmp.path()), options().file_system(Box::new(FailingFs(FailAt::Create))));
    assert!(failing.update(&base, SelectionChange::mode(ModeChoice::Dark)).is_err());
    assert!(failing.take_moved_aside().is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), damaged);
    assert!(store.take_moved_aside().is_empty());

    let next = store.update(&base, SelectionChange::mode(ModeChoice::Dark)).unwrap();
    assert_eq!((next.revision, next.library_revision), (41, 7));
    assert_eq!(next.suite, sel("one", ModeChoice::Dark).into());
    let moved = store.take_moved_aside();
    assert_eq!(moved.len(), 1);
    assert_eq!(fs::read_to_string(&moved[0]).unwrap(), damaged);
    assert!(moved[0].file_name().unwrap().to_str().unwrap().starts_with("selection.broken-"));
}

#[test]
fn an_oversized_selection_counts_as_damaged() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    let big = format!(r#"{{"schema_version":2,"pad":"{}"}}"#, "x".repeat(crate::MAX_SELECTION_BYTES));
    fs::write(&path, &big).unwrap();
    let mut store = open(tmp.path());
    assert!(matches!(store.load_selection(), Err(Problem::Selection(SelectionError::TooLarge { .. }))));
    assert!(!store.is_read_only());
    store.update(&Accepted::default(), whole("craft.graphite", ModeChoice::Dark)).unwrap();
    let moved = store.take_moved_aside();
    assert_eq!(fs::read_to_string(&moved[0]).unwrap(), big);
    assert_eq!(selection_json(tmp.path())["suite"]["theme"]["dark"], "craft.graphite");
}

#[test]
fn same_field_changed_elsewhere_is_a_conflict_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    let mut a = open(tmp.path());
    let mut b = open(tmp.path());
    let base = a.update(&Accepted::default(), whole("craft.artcraft", ModeChoice::System)).unwrap();
    b.load_selection().unwrap();
    let b_state = b.update(&base, theme("craft.graphite")).unwrap();
    let before = fs::read(&path).unwrap();
    let mine = theme("craft.studio");
    match a.update(&base, mine.clone()) {
        Err(StoreError::Conflict { disk, mine: back }) => assert_eq!((disk, back), (b_state.clone(), mine)),
        other => panic!("{other:?}"),
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(a.accepted(), &b_state);
    // A different field merges.
    let merged = a.update(&base, SelectionChange::mode(ModeChoice::Dark)).unwrap();
    assert_eq!(merged.suite, sel("craft.graphite", ModeChoice::Dark).into());
}

#[test]
fn a_change_writes_only_its_own_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let mut a = open(tmp.path());
    let mut b = open(tmp.path());
    // A fresh install: one app picks a theme, another (still on its old base) a mode.
    let theme_only = a.update(&Accepted::default(), theme("craft.graphite")).unwrap();
    let graphite = Some(id("craft.graphite"));
    assert_eq!(theme_only.suite, SuiteChoice { theme_light: graphite.clone(), theme_dark: graphite.clone(), ..SuiteChoice::default() });
    assert_eq!(selection_json(tmp.path())["suite"], serde_json::json!({"theme": {"light": "craft.graphite", "dark": "craft.graphite"}}));
    let both = b.update(&Accepted::default(), SelectionChange::mode(ModeChoice::Light)).unwrap();
    assert_eq!(both.suite, SuiteChoice { mode: Some(ModeChoice::Light), ..theme_only.suite.clone() });
    // Another app chose the theme first: a different theme on the old base conflicts.
    let fresh = tempfile::tempdir().unwrap();
    let mut c = open(fresh.path());
    let mut d = open(fresh.path());
    c.update(&Accepted::default(), theme("craft.graphite")).unwrap();
    assert!(matches!(d.update(&Accepted::default(), theme("craft.artcraft")), Err(StoreError::Conflict { .. })));
}

#[test]
fn the_new_fields_merge_one_by_one_across_stores() {
    let tmp = tempfile::tempdir().unwrap();
    let mut a = open(tmp.path());
    let mut b = open(tmp.path());
    let base = a.update(&Accepted::default(), whole("craft.graphite", ModeChoice::Dark)).unwrap();
    b.load_selection().unwrap();
    // Each app changes other fields on the same base: all of them land.
    b.update(&base, SelectionChange::theme_light(id("craft.soft")).then(SelectionChange::contrast(ContrastChoice::High))).unwrap();
    let merged = a.update(&base, SelectionChange::size(Size::Comfortable).then(SelectionChange::text_size(TextSize::Large))).unwrap();
    assert_eq!(
        merged.suite,
        SuiteChoice {
            theme_light: Some(id("craft.soft")),
            theme_dark: Some(id("craft.graphite")),
            mode: Some(ModeChoice::Dark),
            contrast: Some(ContrastChoice::High),
            size: Some(Size::Comfortable),
            text_size: Some(TextSize::Large),
        }
    );
    // The same field on the stale base: a conflict, field by field.
    for change in [SelectionChange::theme_light(id("craft.pro")), SelectionChange::contrast(ContrastChoice::Standard)] {
        assert!(matches!(a.update(&base, change.clone()), Err(StoreError::Conflict { .. })), "{change:?}");
    }
    let disk = selection_json(tmp.path());
    assert_eq!((disk["suite"]["size"].as_str(), disk["suite"]["text_size"].as_str()), (Some("comfortable"), Some("large")));
}

#[test]
fn ids_users_cant_create_never_reach_the_file_system() {
    let tmp = tempfile::tempdir().unwrap();
    let library = tmp.path().join("library");
    fs::create_dir_all(&library).unwrap();
    // On Windows this name opens the console device; elsewhere it is a plain file to ignore.
    fs::write(library.join("con.json"), theme_json("con", "")).unwrap();
    let mut store = open(tmp.path());
    let con = id("con");
    assert!(matches!(store.load_theme(&con), Err(StoreError::NotFound(_))));
    assert_eq!(store.theme_stamp(&con).unwrap(), None);
    assert!(matches!(store.export(&con), Err(StoreError::NotFound(_))));
    assert!(matches!(store.delete(&con), Err(StoreError::NotFound(_))));
    assert!(store.theme(&con).is_none());
    // A library theme that doesn't exist is never written as the suite's, in either slot.
    assert!(matches!(store.update(&Accepted::default(), theme("con")), Err(StoreError::NotFound(_))));
    assert!(matches!(store.update(&Accepted::default(), SelectionChange::theme_dark(id("gone"))), Err(StoreError::NotFound(_))));
    assert!(library.join("con.json").exists());
    assert!(!tmp.path().join("selection.json").exists());
}

#[test]
fn a_failed_delete_puts_the_theme_back() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("library/one.json");
    open(tmp.path()).import(theme_json("one", "").as_bytes(), false).unwrap();
    let bytes = fs::read(&file).unwrap();
    // The hide is a rename; the selection write fails creating its temporary file.
    let mut failing = Store::open(&SuiteDir::at(tmp.path()), options().file_system(Box::new(FailingFs(FailAt::Create))));
    failing.load_selection().unwrap();
    assert!(matches!(failing.delete(&id("one")), Err(StoreError::Io(_))));
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(fs::read_dir(tmp.path().join("library")).unwrap().count(), 1);
    let mut store = open(tmp.path());
    store.load_selection().unwrap();
    store.delete(&id("one")).unwrap();
    assert_eq!(fs::read_dir(tmp.path().join("library")).unwrap().count(), 0);
}

#[test]
fn a_held_lock_is_busy_after_the_timeout() {
    let tmp = tempfile::tempdir().unwrap();
    let lock = fs::OpenOptions::new().create(true).truncate(false).write(true).open(tmp.path().join("selection.lock")).unwrap();
    lock.try_lock().unwrap();
    let mut store = open(tmp.path());
    let start = Instant::now();
    assert!(matches!(store.update(&Accepted::default(), whole("craft.graphite", ModeChoice::Dark)), Err(StoreError::Busy)));
    assert!(start.elapsed() >= Duration::from_millis(100));
    assert!(!tmp.path().join("selection.json").exists());
    drop(lock);
    store.update(&Accepted::default(), whole("craft.graphite", ModeChoice::Dark)).unwrap();
}

#[test]
fn a_failed_write_leaves_file_and_revision_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    let base = open(tmp.path()).update(&Accepted::default(), whole("craft.graphite", ModeChoice::Light)).unwrap();
    let before = fs::read(&path).unwrap();
    for at in [FailAt::Create, FailAt::Sync, FailAt::Rename] {
        let mut store = Store::open(&SuiteDir::at(tmp.path()), options().file_system(Box::new(FailingFs(at))));
        store.load_selection().unwrap();
        assert!(matches!(store.update(&base, whole("craft.graphite", ModeChoice::Dark)), Err(StoreError::Io(_))), "{at:?}");
        assert_eq!(fs::read(&path).unwrap(), before, "{at:?}");
        assert_eq!(store.accepted(), &base);
    }
    // An import whose selection write fails takes its library file back out.
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options().file_system(Box::new(FailingFs(FailAt::Rename))));
    assert!(store.import(SAMPLE_THEME_JSON.as_bytes(), false).is_err());
    assert_eq!(fs::read_dir(tmp.path().join("library")).unwrap().count(), 0);
    assert_eq!(fs::read(&path).unwrap(), before);
}

/// Stands for an app that writes `selection.json` without honouring the lock, as can happen on
/// network file systems.
struct Intruder(std::path::PathBuf);

impl craft_atomic::AtomicFs for Intruder {
    fn create_new(&self, path: &Path, p: craft_atomic::NewFilePermissions) -> std::io::Result<fs::File> {
        craft_atomic::RealFs.create_new(path, p)
    }
    fn sync(&self, file: &fs::File) -> std::io::Result<()> {
        fs::write(&self.0, br#"{"schema_version":2,"revision":9}"#)?;
        craft_atomic::RealFs.sync(file)
    }
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        craft_atomic::RealFs.rename(from, to)
    }
    fn hard_link(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        craft_atomic::RealFs.hard_link(from, to)
    }
    fn remove(&self, path: &Path) -> std::io::Result<()> {
        craft_atomic::RealFs.remove(path)
    }
    fn sync_dir(&self, dir: &Path) -> std::io::Result<()> {
        craft_atomic::RealFs.sync_dir(dir)
    }
    fn set_permissions(&self, file: &fs::File, perms: fs::Permissions) -> std::io::Result<()> {
        craft_atomic::RealFs.set_permissions(file, perms)
    }
}

#[test]
fn a_change_under_the_lock_abandons_the_write() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("selection.json");
    let mut store = Store::open(&SuiteDir::at(tmp.path()), options().file_system(Box::new(Intruder(path.clone()))));
    assert!(matches!(store.update(&Accepted::default(), whole("craft.graphite", ModeChoice::Dark)), Err(StoreError::Busy)));
    assert_eq!(fs::read_to_string(&path).unwrap(), r#"{"schema_version":2,"revision":9}"#);
}

#[test]
fn library_problems_and_parse_counts() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("library");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("good.json"), theme_json("good", "")).unwrap();
    fs::write(lib.join("renamed.json"), theme_json("other", "")).unwrap();
    fs::write(lib.join("broken.json"), "{").unwrap();
    fs::write(lib.join("craft.mine.json"), theme_json("craft.mine", "")).unwrap();
    // A pilot-format file: skipped and counted, not a problem.
    fs::write(lib.join("old.json"), pilot_json("old")).unwrap();
    fs::write(lib.join("notes.txt"), "not a theme").unwrap();
    fs::write(lib.join(".good.json.1-0.craft_tmp"), "staging").unwrap();
    let mut store = open(tmp.path());
    let library = store.library();
    assert_eq!(library.entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["good"]);
    let mut stems: Vec<_> = library
        .problems
        .iter()
        .map(|p| match p {
            Problem::Theme { stem, .. } => stem.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    stems.sort();
    assert_eq!(stems, ["broken", "craft.mine", "renamed"]);
    assert!(library.problems.iter().any(|p| matches!(p, Problem::Theme { error: StoreError::FileName { .. }, .. })));
    let counts = store.parse_counts();
    assert_eq!((counts.themes, counts.other_schema), (5, 1));
    // Named directly, the pilot file is a problem: it can't be shown.
    assert!(matches!(store.load_theme(&id("old")), Err(StoreError::Format(_))));

    store.update(&Accepted::default(), whole("good", ModeChoice::Dark)).unwrap();
    let after_write = store.parse_counts().selection;
    // Bytes this store wrote, or read before, are not parsed again.
    for _ in 0..5 {
        assert!(!store.load_selection().unwrap());
    }
    assert_eq!(store.parse_counts().selection, after_write);
    let stamp = store.selection_stamp().unwrap().unwrap();
    assert_eq!(store.selection_stamp().unwrap(), Some(stamp));
}
