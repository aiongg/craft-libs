//! `document::merge_changes` and two stores on one file (gap 31).
use super::common;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{Prefs, config_path, empty_probe, open_in, read_config};
use craft_settings::document::{merge_changes, parse};
use craft_settings::{Location, OpenOptions, SettingsFiles, SettingsStore};

fn merged(base: &str, ours: &str, theirs: &str) -> String {
    let mut theirs = parse(theirs).unwrap();
    merge_changes::<Prefs>(&parse(base).unwrap(), &parse(ours).unwrap(), &mut theirs);
    theirs.to_string()
}

// A key only the other instance changed keeps its value; a key only we changed is written; when
// both changed one key, ours wins.
#[test]
fn only_our_changes_are_written() {
    let base = "schema_version = 2\n[general]\nlanguage = \"en\"\nautosave_minutes = 5\n[appearance]\nui_scale = 1.0\n";
    let ours = "schema_version = 2\n[general]\nlanguage = \"fr\"\nautosave_minutes = 5\n[appearance]\nui_scale = 1.0\n";
    let theirs = "schema_version = 2\n[general]\nlanguage = \"de\" # theirs\nautosave_minutes = 9\n[appearance]\nui_scale = 2.0\n[other]\nx = 1\n";
    let out = parse(&merged(base, ours, theirs)).unwrap();
    assert_eq!(out["general"]["language"].as_str(), Some("fr"), "both changed: ours wins");
    assert_eq!(out["general"]["autosave_minutes"].as_integer(), Some(9), "only theirs changed");
    assert_eq!(out["appearance"]["ui_scale"].as_float(), Some(2.0), "only theirs changed");
    assert_eq!(out["other"]["x"].as_integer(), Some(1), "unknown section kept");
    assert!(out.to_string().contains("language = \"fr\" # theirs"), "comment of the replaced line kept");

    let ours = "schema_version = 2\n[general]\nlanguage = \"en\"\nautosave_minutes = 5\ntheme_hint = \"x\"\n[appearance]\nui_scale = 1.0\n";
    let out = parse(&merged(base, ours, theirs)).unwrap();
    assert_eq!(out["general"]["theme_hint"].as_str(), Some("x"), "a key only we added is written");
    assert_eq!(out["general"]["language"].as_str(), Some("de"));
}

// A section both instances added since the base: PhotoCraft replaces the whole subtree with ours
// and loses the other instance's keys; the merge keeps them and ours wins per key.
#[test]
fn a_section_added_on_both_sides_merges_per_key() {
    let base = "schema_version = 2\n";
    let ours = "schema_version = 2\n[plugins]\nb = 2\nshared = \"ours\"\n";
    let theirs = "schema_version = 2\n[plugins]\na = 1\nshared = \"theirs\"\n";
    let out = parse(&merged(base, ours, theirs)).unwrap();
    assert_eq!(out["plugins"]["a"].as_integer(), Some(1), "their key survives");
    assert_eq!(out["plugins"]["b"].as_integer(), Some(2));
    assert_eq!(out["plugins"]["shared"].as_str(), Some("ours"));
}

// A key removed on disk and unchanged here stays removed; a key we removed is removed on disk.
#[test]
fn removals_follow_the_changer() {
    let base = "schema_version = 2\n[general]\nlanguage = \"en\"\nautosave_minutes = 5\n";
    let ours = "schema_version = 2\n[general]\nlanguage = \"fr\"\nautosave_minutes = 5\n";
    let theirs = "schema_version = 2\n[general]\nlanguage = \"en\"\n";
    let out = parse(&merged(base, ours, theirs)).unwrap();
    assert!(out["general"].as_table().unwrap().get("autosave_minutes").is_none(), "removed there, unchanged here");
    assert_eq!(out["general"]["language"].as_str(), Some("fr"));

    let ours = "schema_version = 2\n[general]\nlanguage = \"en\"\n";
    let theirs = "schema_version = 2\n[general]\nlanguage = \"en\"\nautosave_minutes = 7\n";
    let out = parse(&merged(base, ours, theirs)).unwrap();
    assert!(out["general"].as_table().unwrap().get("autosave_minutes").is_none(), "removed here");
}

// A disk file at a newer schema migrates our base first, so our migration doesn't overwrite the
// other instance's choice.
#[test]
fn stale_migration_is_not_written_over() {
    // We loaded a version 1 file (`dark = true`) and migrated it in memory; another instance
    // already wrote version 2 and chose the light theme.
    let base = "schema_version = 1\n[appearance]\ndark = true\n";
    let ours = "schema_version = 2\n[appearance]\ntheme = \"dark\"\nui_scale = 1.5\n";
    let theirs = "schema_version = 2\n[appearance]\ntheme = \"light\"\n";
    let out = parse(&merged(base, ours, theirs)).unwrap();
    assert_eq!(out["appearance"]["theme"].as_str(), Some("light"), "their choice stays");
    assert_eq!(out["appearance"]["ui_scale"].as_float(), Some(1.5), "our own change is written");
}

// Two SettingsStores on one folder each change a different key and both changes survive.
#[test]
fn two_stores_keep_each_others_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut a, _) = open_in(tmp.path());
    let (mut b, _) = open_in(tmp.path());
    a.update(|p| p.general.language = "fr".into());
    b.update(|p| p.appearance.ui_scale = 2.0);
    a.flush().unwrap();
    b.flush().unwrap();

    let (c, _) = open_in(tmp.path());
    assert_eq!(c.get().general.language, "fr", "{}", read_config(tmp.path()));
    assert_eq!(c.get().appearance.ui_scale, 2.0);

    // The same holds for later saves from stores that already wrote once.
    a.update(|p| p.general.autosave_minutes = 9);
    b.update(|p| p.appearance.ui_scale = 3.0);
    a.flush().unwrap();
    b.flush().unwrap();
    let (c, _) = open_in(tmp.path());
    assert_eq!(c.get().general.autosave_minutes, 9);
    assert_eq!(c.get().appearance.ui_scale, 3.0);
    assert_eq!(c.get().general.language, "fr", "b's second save must not write its old language back");
}

// After a save merged the other instance's change, the store holds the merged value and says so
// once.
#[test]
fn a_merge_updates_the_stores_value_and_reports_it() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut a, _) = open_in(tmp.path());
    let (mut b, _) = open_in(tmp.path());
    a.update(|p| p.general.language = "fr".into());
    a.flush().unwrap();
    assert!(!a.take_merged_change(), "nothing from outside");
    b.update(|p| p.appearance.ui_scale = 2.0);
    b.flush().unwrap();
    assert_eq!(b.get().general.language, "fr");
    assert_eq!(b.get().appearance.ui_scale, 2.0);
    assert!(b.take_merged_change());
    assert!(!b.take_merged_change(), "reported once");
}

#[derive(Default)]
struct MemoryFiles {
    files: Mutex<HashMap<PathBuf, String>>,
    log: Mutex<Vec<String>>,
}

impl MemoryFiles {
    fn note(&self, op: &str, path: &Path) {
        self.log.lock().unwrap().push(format!("{op} {}", path.file_name().unwrap().to_string_lossy()));
    }
}

impl SettingsFiles for MemoryFiles {
    fn read(&self, path: &Path) -> io::Result<Option<String>> {
        self.note("read", path);
        Ok(self.files.lock().unwrap().get(path).cloned())
    }
    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        self.note("write", path);
        self.files.lock().unwrap().insert(path.to_path_buf(), text.to_owned());
        Ok(())
    }
    fn move_aside(&self, path: &Path) -> io::Result<PathBuf> {
        self.note("move_aside", path);
        Ok(path.to_path_buf())
    }
    fn write_backup(&self, path: &Path, text: &str) -> io::Result<()> {
        self.note("backup", path);
        self.files.lock().unwrap().insert(path.with_extension("toml.bak"), text.to_owned());
        Ok(())
    }
    fn check_writable(&self, dir: &Path) -> io::Result<()> {
        self.note("check_writable", dir);
        Ok(())
    }
}

// A store opened with custom SettingsFiles reads and writes only through them.
#[test]
fn custom_files_see_every_read_and_write() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("not-created");
    let files = Arc::new(MemoryFiles::default());
    let open = || {
        SettingsStore::<Prefs>::open_with(
            OpenOptions::new(Location::Dir(dir.clone())).probe(empty_probe()).debounce(Duration::from_millis(100)).files(files.clone()),
        )
    };
    let (mut store, _) = open();
    store.update(|p| p.general.language = "fr".into());
    store.flush().unwrap();
    store.update(|p| p.general.language = "de".into());
    store.flush().unwrap();

    assert!(!dir.exists() && !config_path(&dir).exists(), "nothing reached the disk");
    let stored = files.files.lock().unwrap();
    assert!(stored[&dir.join("config.toml")].contains("language = \"de\""));
    assert!(stored[&dir.join("config.toml.bak")].contains("language = \"fr\""), "the backup goes through write_backup");
    drop(stored);
    let log = files.log.lock().unwrap().join(", ");
    assert!(log.contains("check_writable") && log.contains("write config.toml") && log.contains("backup"), "{log}");

    let (again, _) = open();
    assert_eq!(again.get().general.language, "de");
}
