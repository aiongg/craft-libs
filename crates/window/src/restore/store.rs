//! `window.json`: the saved [`Placement`] of each window, beside the app's `config.toml`.
//!
//! ```json
//! { "schema_version": 1, "windows": { "main": { ... }, "<key>": { ... } } }
//! ```
//!
//! The folder is chosen by `craft_settings::paths`, so portable installs keep the file in the
//! portable folder and `<APP>_CONFIG_DIR` moves it with `config.toml`. Writes are atomic and
//! private. A file that does not parse is moved aside (`window.broken-<unix-ts>.json`, the
//! `craft_settings::recovery` convention) and the store starts empty. A file with a newer
//! `schema_version` is read but never written. Keys this version does not know are written back
//! unchanged. Two running instances of one app overwrite each other; the last writer wins.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use craft_atomic::{NewFilePermissions, WriteOptions};
use craft_settings::paths::{self, Location, Portable, Probe};
use craft_settings::recovery;
use serde_json::{Map, Value, json};

use super::place::Placement;

/// The file name beside `config.toml`.
pub const FILE_NAME: &str = "window.json";
/// The `schema_version` this code writes.
pub const SCHEMA_VERSION: u64 = 1;
/// The key of the app's first window.
pub const MAIN: &str = "main";

/// Why a store does not write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOnly {
    /// The file has a newer `schema_version` than this code knows.
    NewerSchema(u64),
    /// The file exists but could not be read (permissions, say); it is left alone.
    Unreadable,
}

/// The saved placements of an app's windows.
#[derive(Debug)]
pub struct Store {
    path: Option<PathBuf>,
    /// Entries stay raw so one that fails to parse is neither lost nor fatal to the others.
    windows: BTreeMap<String, Value>,
    /// Top-level keys other than `schema_version` and `windows`.
    extra: Map<String, Value>,
    read_only: Option<ReadOnly>,
    dirty: bool,
    moved_aside: Option<PathBuf>,
}

impl Store {
    /// The store for `app_id` in the folder `craft_settings` picks for its `config.toml`, with
    /// the suite's default portable marker. In an isolated or in-memory configuration
    /// (`<APP>_CONFIG_DIR` empty) nothing is read or written.
    pub fn open(app_id: &str) -> Self {
        Self::open_with(app_id, &Portable::DEFAULT, &Location::Auto, &Probe::default())
    }

    /// [`open`](Self::open) with the app's own portable names, location override and probe: the
    /// same arguments its `craft_settings` options use.
    pub fn open_with(app_id: &str, portable: &Portable, location: &Location, probe: &Probe) -> Self {
        match paths::resolve(app_id, portable, location, probe).dir {
            Some(dir) => Self::at(&dir.join(FILE_NAME)),
            None => Self::in_memory(),
        }
    }

    /// A store backed by the file at `path`. A missing file is an empty store.
    pub fn at(path: &Path) -> Self {
        let mut store = Self { path: Some(path.to_path_buf()), ..Self::in_memory() };
        store.load();
        store
    }

    /// A store that reads and writes nothing: tests, agents, the web.
    pub fn in_memory() -> Self {
        Self { path: None, windows: BTreeMap::new(), extra: Map::new(), read_only: None, dirty: false, moved_aside: None }
    }

    /// The file this store writes, if it has one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Where a damaged file was moved to while opening, if one was.
    pub fn moved_aside(&self) -> Option<&Path> {
        self.moved_aside.as_deref()
    }

    /// Why [`save`](Self::save) writes nothing, if so.
    pub fn read_only(&self) -> Option<&ReadOnly> {
        self.read_only.as_ref()
    }

    /// The placement saved under `key`; `None` when absent or not a placement.
    pub fn get(&self, key: &str) -> Option<Placement> {
        serde_json::from_value(self.windows.get(key)?.clone()).ok()
    }

    /// Replaces the placement under `key`. Nothing is written until [`save`](Self::save).
    pub fn set(&mut self, key: &str, placement: &Placement) {
        let Ok(value) = serde_json::to_value(placement) else { return };
        if self.windows.get(key) != Some(&value) {
            self.windows.insert(key.to_owned(), value);
            self.dirty = true;
        }
    }

    /// Forgets `key`.
    pub fn remove(&mut self, key: &str) {
        self.dirty |= self.windows.remove(key).is_some();
    }

    /// The keys with an entry, in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.windows.keys().map(String::as_str)
    }

    /// Whether [`set`](Self::set) or [`remove`](Self::remove) changed something since the last save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Writes the file if something changed. `Ok(true)` when written; `Ok(false)` when there was
    /// nothing to write, or the store is in memory or read-only.
    pub fn save(&mut self) -> io::Result<bool> {
        let Some(path) = self.path.clone().filter(|_| self.read_only.is_none() && self.dirty) else {
            return Ok(false);
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut doc = self.extra.clone();
        doc.insert("schema_version".into(), json!(SCHEMA_VERSION));
        doc.insert("windows".into(), Value::Object(self.windows.clone().into_iter().collect()));
        let mut bytes = serde_json::to_vec_pretty(&Value::Object(doc)).map_err(io::Error::other)?;
        bytes.push(b'\n');
        WriteOptions::default().with_new_file_permissions(NewFilePermissions::Private).atomic_write(&path, &bytes)?;
        self.dirty = false;
        Ok(true)
    }

    fn load(&mut self) {
        let Some(path) = self.path.clone() else { return };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return,
            // Invalid UTF-8 is damage; anything else may be transient, so leave the file be.
            Err(e) if e.kind() == io::ErrorKind::InvalidData => return self.start_over(&path),
            Err(_) => {
                self.read_only = Some(ReadOnly::Unreadable);
                return;
            }
        };
        let Some(mut doc) = serde_json::from_str::<Value>(&text).ok().and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        }) else {
            return self.start_over(&path);
        };
        let Some(version) = doc.remove("schema_version").and_then(|v| v.as_u64()) else {
            return self.start_over(&path);
        };
        let windows = match doc.remove("windows") {
            Some(Value::Object(m)) => m,
            None if version > SCHEMA_VERSION => Map::new(),
            _ => return self.start_over(&path),
        };
        if version > SCHEMA_VERSION {
            self.read_only = Some(ReadOnly::NewerSchema(version));
        }
        self.windows = windows.into_iter().collect();
        self.extra = doc;
    }

    fn start_over(&mut self, path: &Path) {
        match recovery::move_aside(path) {
            Ok(to) => self.moved_aside = Some(to),
            // Cannot rename it away: do not overwrite what we could not set aside.
            Err(_) => self.read_only = Some(ReadOnly::Unreadable),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::restore::{Monitor, MonitorKey};

    fn sample(x: i32) -> Placement {
        let m = Monitor { key: MonitorKey::new("DP-1", [1920, 1080]), pos_px: [0, 0], size_px: [1920, 1080], scale: 1.0 };
        let mut p = crate::restore::place(None, &[m], Some(0), &crate::restore::Defaults::new([800.0, 600.0], [300.0, 200.0]));
        p.position_px = Some([x, 10]);
        p
    }

    fn siblings(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(FILE_NAME);
        let mut s = Store::at(&path);
        assert!(s.get(MAIN).is_none());
        s.set(MAIN, &sample(5));
        s.set("doc-2", &sample(6));
        assert!(s.save().unwrap());
        assert!(!s.save().unwrap(), "nothing changed");
        let again = Store::at(&path);
        assert_eq!(again.get(MAIN), Some(sample(5)));
        assert_eq!(again.get("doc-2"), Some(sample(6)));
        assert_eq!(again.keys().collect::<Vec<_>>(), ["doc-2", MAIN]);
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["schema_version"], 1);
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut s = Store::at(&path);
        s.set(MAIN, &sample(1));
        s.save().unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_the_store_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, "{ not json").unwrap();
        let mut s = Store::at(&path);
        assert!(s.get(MAIN).is_none() && s.read_only().is_none());
        let aside = s.moved_aside().unwrap().to_path_buf();
        assert_eq!(std::fs::read_to_string(aside).unwrap(), "{ not json");
        assert!(!path.exists());
        s.set(MAIN, &sample(1));
        s.save().unwrap();
        assert!(Store::at(&path).get(MAIN).is_some());
        assert_eq!(siblings(dir.path()).len(), 2);
    }

    #[test]
    fn json_without_a_schema_version_counts_as_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, r#"{"windows":{}}"#).unwrap();
        assert!(Store::at(&path).moved_aside().is_some());
    }

    #[test]
    fn newer_schema_is_read_but_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut doc = json!({ "schema_version": 2, "windows": {}, "future": [1] });
        doc["windows"][MAIN] = serde_json::to_value(sample(7)).unwrap();
        let text = serde_json::to_string(&doc).unwrap();
        std::fs::write(&path, &text).unwrap();
        let mut s = Store::at(&path);
        assert_eq!(s.read_only(), Some(&ReadOnly::NewerSchema(2)));
        assert_eq!(s.get(MAIN), Some(sample(7)));
        s.set(MAIN, &sample(8));
        assert!(!s.save().unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        assert_eq!(siblings(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn unknown_keys_survive_a_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut doc = json!({ "schema_version": 1, "note": {"a": 1}, "windows": { "odd": {"unexpected": true} } });
        doc["windows"][MAIN] = serde_json::to_value(sample(1)).unwrap();
        std::fs::write(&path, doc.to_string()).unwrap();
        let mut s = Store::at(&path);
        assert!(s.get("odd").is_none());
        s.set(MAIN, &sample(2));
        s.save().unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["note"], json!({"a": 1}));
        assert_eq!(v["windows"]["odd"], json!({"unexpected": true}));
        assert_eq!(Store::at(&path).get(MAIN), Some(sample(2)));
    }

    #[test]
    fn in_memory_store_keeps_values_and_writes_nothing() {
        let mut s = Store::in_memory();
        s.set(MAIN, &sample(3));
        assert_eq!(s.get(MAIN), Some(sample(3)));
        assert!(!s.save().unwrap());
        assert!(s.path().is_none());
    }

    #[test]
    fn open_puts_the_file_beside_config_toml() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_with("app", &Portable::DEFAULT, &Location::Dir(dir.path().to_path_buf()), &Probe::default());
        assert_eq!(s.path(), Some(dir.path().join(FILE_NAME).as_path()));
        let mem = Store::open_with("app", &Portable::DEFAULT, &Location::Memory, &Probe::default());
        assert!(mem.path().is_none());
    }
}
