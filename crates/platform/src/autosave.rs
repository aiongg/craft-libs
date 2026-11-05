//! Autosave and crash recovery: when to autosave ([`Autosave`]), where the copies go
//! ([`recovery_dir`]), and the folder of copies ([`RecoveryStore`]).
//!
//! The app serializes its documents; this module decides when, writes the bytes with a small
//! JSON sidecar, removes a copy when its document is saved or closed, and lists the copies an
//! earlier run left behind. The recover-on-launch question is
//! `craft_ui::dialog::RecoveryDialog` (or none: PhotoCraft reopens them without asking).

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

/// The recovery folder's name.
pub const RECOVERY_DIR: &str = "Recovery";

/// PhotoCraft's default interval, `fileHandling.autosaveMinutes = 10` (minimum 1).
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// The intervals Photoshop's File Handling page offers for "Automatically Save Recovery
/// Information Every", in minutes; [`DEFAULT_INTERVAL`] is one of them. Apps offer them as a
/// preferences row with `craft_ui::preferences::SettingRule::minutes(&INTERVAL_CHOICES_MINUTES)`
/// and read the stored choice with [`interval_from_minutes`].
pub const INTERVAL_CHOICES_MINUTES: [u32; 5] = [5, 10, 15, 30, 60];

/// The interval for a stored choice ("10"); `None` when the text is not one of
/// [`INTERVAL_CHOICES_MINUTES`].
pub fn interval_from_minutes(text: &str) -> Option<Duration> {
    let minutes: u32 = text.parse().ok()?;
    INTERVAL_CHOICES_MINUTES.contains(&minutes).then(|| Duration::from_secs(u64::from(minutes) * 60))
}

/// The status text of a failed autosave (an error in the status bar; no notice, the next
/// interval tries again). `{error}`: the write error. Catalog id `autosave.failed`.
pub const AUTOSAVE_FAILED: &str = "Autosave failed: {error}";

/// A document as the timer sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutosaveDoc {
    /// The app's document id, stable while the document is open.
    pub id: u64,
    /// Bumped on every edit.
    pub revision: u64,
    /// Has unsaved changes.
    pub dirty: bool,
}

/// What [`Autosave::due`] decided for one call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutosaveDue {
    /// The documents to autosave now; they count as in flight until [`Autosave::finished`].
    pub save: Vec<u64>,
    /// The documents the timer stopped tracking because they are clean (saved, or undone back to
    /// the saved state) or gone. The app discards their recovery copies.
    pub discard: Vec<u64>,
}

/// When to autosave, after PhotoCraft's timer.
///
/// The first [`due`](Self::due) starts the clock; each interval after it, every document that is
/// dirty, whose revision differs from the one last autosaved, and that has no autosave in flight
/// is due. The app writes them (on a job) and reports each with [`finished`](Self::finished).
/// A document that turns clean without a save comes back in [`AutosaveDue::discard`]; saving or
/// closing a document is [`forget`](Self::forget) (and the app discards its copy).
/// With autosave off (`set_interval(None)`) nothing is ever due.
#[derive(Debug)]
pub struct Autosave {
    interval: Option<Duration>,
    next: Option<Instant>,
    saved: HashMap<u64, u64>,
    in_flight: HashMap<u64, u64>,
}

impl Autosave {
    /// `None` turns autosave off (PhotoCraft's `fileHandling.autosave = false`).
    pub fn new(interval: Option<Duration>) -> Self {
        Self { interval, next: None, saved: HashMap::new(), in_flight: HashMap::new() }
    }

    /// A changed preference; the next deadline moves to now plus the new interval.
    ///
    /// The clock restarts at the next [`due`](Self::due) call, which is the caller's "now"; an
    /// unchanged interval leaves it running.
    pub fn set_interval(&mut self, interval: Option<Duration>) {
        if self.interval != interval {
            self.interval = interval;
            self.next = None;
        }
    }

    /// The documents to autosave now, and those whose recovery copy to discard.
    // pc: crates/ui-egui/src/prefs/prefs_ui/save.rs:228 autosave
    pub fn due(&mut self, now: Instant, docs: impl IntoIterator<Item = AutosaveDoc>) -> AutosaveDue {
        let docs: Vec<AutosaveDoc> = docs.into_iter().collect();
        // A document that is clean (or gone) has no pending copy to track.
        let dirty = |id: &u64| docs.iter().any(|d| d.id == *id && d.dirty);
        let mut discard: Vec<u64> = self.saved.keys().chain(self.in_flight.keys()).copied().filter(|id| !dirty(id)).collect();
        discard.sort_unstable();
        discard.dedup();
        for id in &discard {
            self.saved.remove(id);
            self.in_flight.remove(id);
        }
        let mut due = AutosaveDue { save: Vec::new(), discard };
        let Some(interval) = self.interval else {
            self.next = None;
            return due;
        };
        let Some(next) = self.next else {
            self.next = Some(now + interval);
            return due;
        };
        if now < next {
            return due;
        }
        self.next = Some(now + interval);
        for d in docs {
            if d.dirty && self.saved.get(&d.id) != Some(&d.revision) && !self.in_flight.contains_key(&d.id) {
                self.in_flight.insert(d.id, d.revision);
                due.save.push(d.id);
            }
        }
        due
    }

    /// An autosave ended. On success `revision` is remembered; on failure the status text
    /// ([`AUTOSAVE_FAILED`] filled in) comes back for the status bar.
    pub fn finished(&mut self, id: u64, revision: u64, result: Result<(), String>) -> Option<String> {
        // A result for a document that was forgotten meanwhile, or for a revision other than the
        // one in flight (an older job after `forget` and a new `due`), is ignored.
        if self.in_flight.get(&id) != Some(&revision) {
            return None;
        }
        self.in_flight.remove(&id);
        match result {
            Ok(()) => {
                self.saved.insert(id, revision);
                None
            }
            Err(error) => Some(AUTOSAVE_FAILED.replace("{error}", &error)),
        }
    }

    /// The document was saved or closed.
    // pc: crates/ui-egui/src/prefs/prefs_ui/save.rs:263 (services.discard_autosave)
    pub fn forget(&mut self, id: u64) {
        self.saved.remove(&id);
        self.in_flight.remove(&id);
    }

    /// When the next interval ends, for `request_repaint_after`.
    pub fn deadline(&self) -> Option<Instant> {
        self.next
    }
}

/// How the app's settings folder was chosen, which decides where recovery copies go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsMode {
    /// The platform's own folders.
    Platform,
    /// Portable mode or an override folder: everything stays in the settings folder.
    SelfContained,
}

/// The recovery folder. In [`SettingsMode::Platform`]: `%LOCALAPPDATA%\<Folder>\Recovery` on
/// Windows; `$XDG_STATE_HOME/<app_id>/Recovery` on Linux and the BSDs (an absolute
/// `XDG_STATE_HOME` only, else `$HOME/.local/state`); `<config>/Recovery` on macOS. Otherwise, or
/// when those variables are missing, `<config>/Recovery`. `folder` is the Windows folder name
/// ("Photocraft"), `app_id` the XDG one ("photocraft"); `env` reads the environment.
// pc: apps/photocraft/src/app_dirs.rs:94 resolve_recovery
pub fn recovery_dir(
    folder: &str,
    app_id: &str,
    config_dir: Option<&Path>,
    mode: SettingsMode,
    env: impl Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    if mode == SettingsMode::Platform {
        if cfg!(windows) {
            if let Some(local) = env("LOCALAPPDATA").filter(|l| !l.is_empty()) {
                return Some(PathBuf::from(local).join(folder).join(RECOVERY_DIR));
            }
        } else if !cfg!(target_os = "macos") {
            let state = env("XDG_STATE_HOME")
                .map(PathBuf::from)
                .filter(|s| s.is_absolute())
                .or_else(|| env("HOME").filter(|h| !h.is_empty()).map(|h| PathBuf::from(h).join(".local").join("state")));
            if let Some(state) = state {
                return Some(state.join(app_id).join(RECOVERY_DIR));
            }
        }
    }
    config_dir.map(|d| d.join(RECOVERY_DIR))
}

/// Moves copies from an older recovery folder (`legacy`) into `dir`, so a crash just before an
/// update is still offered. Returns the folder this process uses: `legacy` only when `dir`
/// doesn't exist and nothing can be moving (the move failed, or its lock can't be made). Two
/// launches at once are kept apart by `recovery-migration.lock` beside `dir`, waiting at most 5 s;
/// entries move through `.moving` and `.held` names so a half-done move is finished by the next
/// launch.
// pc: apps/photocraft/src/app_dirs.rs:133 adopt_legacy_recovery, :138 adopt_legacy_recovery_waiting (MIGRATION_LOCK :115, MIGRATION_WAIT :118)
#[cfg(not(target_arch = "wasm32"))]
pub fn adopt_legacy_recovery(legacy: &Path, dir: &Path) -> PathBuf {
    migrate::adopt_waiting(legacy, dir, migrate::WAIT)
}

/// What the sidecar records about a copy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryMeta {
    /// The document's name ("poster.psd", "Untitled-2").
    pub name: String,
    /// The file it was opened from or last saved to.
    pub original: Option<PathBuf>,
    /// The copy's extension, without the dot ("pcraft").
    pub extension: String,
    /// The edit revision the copy holds.
    pub revision: u64,
}

/// A copy an earlier run left.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryEntry {
    /// The file stem shared by the copy and its sidecar.
    pub key: String,
    /// What the sidecar says about the copy.
    pub meta: RecoveryMeta,
    /// When the copy was written (the sidecar's modification time).
    pub saved: SystemTime,
    /// The copy's path.
    pub path: PathBuf,
}

/// The recovery folder of one app run, after PhotoCraft's and PdfCraft's stores.
///
/// Each copy is `<key>.<extension>` plus `<key>.json`, both written atomically, the copy first.
/// A new document's key is `doc-<session>-<id>`, `<session>` unique to this run (ids restart
/// every launch); a recovered document [adopts](Self::adopt) the key it was loaded from, so its
/// autosaves replace that copy. Nothing is deleted because it was recovered: a second crash
/// before the next autosave loses nothing. Keys are sanitized ([`sanitize_key`]).
///
/// Every method takes `&self`, so the app keeps one store in an `Arc` and shares it with its
/// autosave jobs. The folder is created owner-only (0o700 on Unix). [`open`](Self::open) takes an exclusive
/// `instance-<session>-<pid>-<n>.lock`, so [`leftovers`](Self::leftovers) offers only copies whose run is
/// gone, never those of another instance still running (PdfCraft).
#[cfg(not(target_arch = "wasm32"))]
pub struct RecoveryStore {
    dir: PathBuf,
    session: String,
    adopted: std::sync::Mutex<HashMap<u64, String>>,
    /// The name of this run's `instance-<session>-<pid>-<n>.lock`, recorded in every sidecar it writes.
    instance: String,
    lock: std::fs::File,
}

// The app shares one store with its autosave jobs (`Arc<RecoveryStore>`).
#[cfg(not(target_arch = "wasm32"))]
const _: () = {
    const fn shared<T: Send + Sync>() {}
    shared::<RecoveryStore>();
};

#[cfg(not(target_arch = "wasm32"))]
impl RecoveryStore {
    // pc: crates/format/src/autosave.rs:348 RecoveryStore::new; pdfcraft: crates/ui-egui/src/persist/recovery.rs:103 claim
    /// Opens the store in `dir`, creating the folder with private permissions, and locks an
    /// instance file in it that marks this run as alive. Removes orphaned temporary files that
    /// are older than the store's age limit.
    pub fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static STORES: AtomicU64 = AtomicU64::new(0);
        let dir = dir.into();
        create_private_dir(&dir)?;
        let n = STORES.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let session = format!("{nanos:x}_{pid:x}_{n:x}");
        // The session string is in the name: a PID namespace can hand a new run its crashed
        // predecessor's PID and counter.
        let instance = format!("instance-{nanos:x}-{pid:x}-{n:x}.lock");
        let path = dir.join(&instance);
        let lock = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path)?;
        if let Err(e) = lock.try_lock() {
            let _ = std::fs::remove_file(&path);
            return Err(match e {
                std::fs::TryLockError::Error(e) => e,
                std::fs::TryLockError::WouldBlock => std::io::Error::new(std::io::ErrorKind::WouldBlock, "the instance lock is held"),
            });
        }
        sweep_temp_files(&dir, ORPHAN_AGE);
        Ok(Self { dir, session, adopted: std::sync::Mutex::new(HashMap::new()), instance, lock })
    }

    /// The recovery folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The key document `id` autosaves under.
    // pc: crates/format/src/autosave.rs:380 (doc-<session>-<id>)
    pub fn key(&self, id: u64) -> String {
        let adopted = self.adopted.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        adopted.get(&id).cloned().unwrap_or_else(|| format!("doc-{}-{id}", self.session))
    }

    /// Writes a copy of document `id`; call from a job.
    pub fn write(&self, id: u64, meta: &RecoveryMeta, bytes: &[u8]) -> std::io::Result<()> {
        let key = self.key(id);
        let mut meta = meta.clone();
        meta.extension = sanitize_extension(&meta.extension);
        let copy = self.dir.join(format!("{key}.{}", meta.extension));
        craft_atomic::atomic_write(&copy, bytes)?;
        let sidecar = Sidecar { meta, instance: Some(self.instance.clone()) };
        let json = serde_json::to_vec(&sidecar).map_err(std::io::Error::other)?;
        craft_atomic::atomic_write(&self.dir.join(format!("{key}.json")), &json)
    }

    /// Removes document `id`'s copy (saved or closed).
    // pc: crates/format/src/autosave.rs:411 discard
    pub fn discard(&self, id: u64) -> std::io::Result<()> {
        let adopted = self.adopted.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id);
        let key = adopted.unwrap_or_else(|| format!("doc-{}-{id}", self.session));
        remove_key(&self.dir, &key)
    }

    /// Copies of runs that are gone, newest first; a copy without a readable sidecar is listed
    /// under its key with what the file system knows.
    // pc: crates/format/src/autosave.rs:361 recover
    pub fn leftovers(&self) -> Vec<RecoveryEntry> {
        let Ok(read) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        // Per key: the copies and whether a sidecar exists.
        let mut keys: HashMap<String, Vec<(String, PathBuf)>> = HashMap::new();
        for e in read.flatten() {
            let Ok(name) = e.file_name().into_string() else { continue };
            let Some((stem, ext)) = name.split_once('.') else { continue };
            if stem.is_empty() || ext == "lock" || sanitize_key(stem) != stem {
                continue;
            }
            keys.entry(stem.to_owned()).or_default().push((ext.to_owned(), e.path()));
        }
        let mut out = Vec::new();
        for (key, files) in keys {
            let sidecar = files.iter().find(|(ext, _)| ext == "json").map(|(_, p)| p);
            let copies: Vec<&(String, PathBuf)> = files.iter().filter(|(ext, _)| ext != "json").collect();
            let parsed = sidecar.and_then(|p| read_sidecar(p));
            if let Some((side, _)) = &parsed
                && side.instance.as_deref().is_some_and(|i| i == self.instance || self.held_by_another(i))
            {
                continue;
            }
            let entry = match parsed {
                Some((side, saved)) => {
                    let wanted = format!("{key}.{}", side.meta.extension);
                    let path = copies
                        .iter()
                        .map(|(_, p)| p)
                        .find(|p| p.file_name().is_some_and(|n| *n == *wanted))
                        .or_else(|| copies.first().map(|(_, p)| p));
                    path.map(|path| RecoveryEntry { key, meta: side.meta, saved, path: path.clone() })
                }
                // No sidecar: the copy may be a first write in progress. It is a leftover only
                // when it is not this run's and no other instance is alive to be writing it.
                None if self.owns_key(&key) || self.other_instance_alive() => None,
                None => copies.first().map(|(ext, path)| {
                    let saved = std::fs::metadata(path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
                    let meta = RecoveryMeta { name: key.clone(), original: None, extension: ext.clone(), revision: 0 };
                    RecoveryEntry { key: key.clone(), meta, saved, path: path.clone() }
                }),
            };
            out.extend(entry);
        }
        out.sort_by(|a, b| b.saved.cmp(&a.saved).then_with(|| a.key.cmp(&b.key)));
        out
    }

    /// Whether `key` is one this run writes: a new document's or an adopted one.
    fn owns_key(&self, key: &str) -> bool {
        let adopted = self.adopted.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        key.starts_with(&format!("doc-{}-", self.session)) || adopted.values().any(|k| k == key)
    }

    /// Whether another run holds its instance lock in this folder.
    fn other_instance_alive(&self) -> bool {
        let Ok(read) = std::fs::read_dir(&self.dir) else { return false };
        let names: Vec<String> = read.flatten().filter_map(|e| e.file_name().into_string().ok()).collect();
        names.iter().any(|n| n.starts_with("instance-") && n.ends_with(".lock") && *n != self.instance && self.held_by_another(n))
    }

    /// Whether `instance` (a name read from an untrusted sidecar) is the lock of another run that
    /// is still alive. A lock nobody holds is a crash's leftover and is removed.
    fn held_by_another(&self, instance: &str) -> bool {
        let plain = instance.starts_with("instance-")
            && instance.ends_with(".lock")
            && instance.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
            && !instance.contains("..");
        if !plain {
            return false;
        }
        let path = self.dir.join(instance);
        let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open(&path) else { return false };
        match file.try_lock() {
            Err(std::fs::TryLockError::WouldBlock) => true,
            Ok(()) => {
                let _ = file.unlock();
                drop(file);
                let _ = std::fs::remove_file(&path);
                false
            }
            // No file locks on this file system: offered, as if the run were gone.
            Err(std::fs::TryLockError::Error(_)) => false,
        }
    }

    /// Document `id` was recovered from `entry`: its autosaves go to `entry.key` from now on.
    // pc: crates/format/src/autosave.rs:367 adopt
    pub fn adopt(&self, id: u64, entry: &RecoveryEntry) {
        let key = sanitize_key(&entry.key);
        self.adopted.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(id, key);
    }

    /// Deletes a leftover the user discarded.
    pub fn remove(&self, entry: &RecoveryEntry) -> std::io::Result<()> {
        if sanitize_key(&entry.key) != entry.key {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "recovery key contains path components"));
        }
        remove_key(&self.dir, &entry.key)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for RecoveryStore {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
        let _ = std::fs::remove_file(self.dir.join(&self.instance));
    }
}

/// The sidecar on disk: [`RecoveryMeta`] plus the lock name of the run that wrote it.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize, Deserialize)]
struct Sidecar {
    #[serde(flatten)]
    meta: RecoveryMeta,
    #[serde(default)]
    instance: Option<String>,
}

/// A sidecar larger than this is not read.
#[cfg(not(target_arch = "wasm32"))]
const MAX_SIDECAR: u64 = 1 << 20;

/// How long a temp file of an interrupted write must have been left before it is removed, far
/// longer than any write, so another run's write in progress is never touched.
#[cfg(not(target_arch = "wasm32"))]
const ORPHAN_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The parsed sidecar with its modification time, if it is a plain readable file.
#[cfg(not(target_arch = "wasm32"))]
fn read_sidecar(path: &Path) -> Option<(Sidecar, SystemTime)> {
    let meta = std::fs::symlink_metadata(path).ok().filter(|m| m.is_file() && m.len() <= MAX_SIDECAR)?;
    let side: Sidecar = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some((side, meta.modified().ok()?))
}

/// An extension safe in a file name: ASCII letters and digits; empty becomes `bin`.
#[cfg(not(target_arch = "wasm32"))]
fn sanitize_extension(ext: &str) -> String {
    let s: String = ext.chars().filter(char::is_ascii_alphanumeric).collect();
    if s.is_empty() { "bin".into() } else { s }
}

/// Removes every file of `key` (copy, sidecar, whatever extension the copy had).
#[cfg(not(target_arch = "wasm32"))]
fn remove_key(dir: &Path, key: &str) -> std::io::Result<()> {
    let mut result = Ok(());
    for e in std::fs::read_dir(dir)?.flatten() {
        let name = e.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.split_once('.').is_some_and(|(stem, ext)| stem == key && ext != "lock")
            && let Err(err) = std::fs::remove_file(e.path())
            && err.kind() != std::io::ErrorKind::NotFound
        {
            result = Err(err);
        }
    }
    result
}

/// Creates `dir` readable by its owner only (0o700 on Unix).
#[cfg(not(target_arch = "wasm32"))]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)
}

/// Removes temp files of an interrupted atomic write that were last touched `min_age` ago.
#[cfg(not(target_arch = "wasm32"))]
fn sweep_temp_files(dir: &Path, min_age: Duration) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for e in read.flatten() {
        let is_temp = e.file_name().to_str().is_some_and(|n| n.starts_with('.') && n.ends_with(".craft_tmp"));
        let old = e
            .metadata()
            .ok()
            .filter(|m| m.is_file())
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age >= min_age);
        if is_temp && old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// A key safe as a file stem: ASCII letters, digits, `-` and `_` stay, everything else becomes
/// `_`; an empty key becomes `untitled`.
// pc: crates/format/src/autosave.rs:67 sanitize
pub fn sanitize_key(key: &str) -> String {
    let s: String = key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if s.is_empty() { "untitled".into() } else { s }
}

/// Moving a legacy recovery folder into the new one, without ever replacing anything.
#[cfg(not(target_arch = "wasm32"))]
mod migrate {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    /// The lock file, beside the recovery folder, that keeps two launches from moving the same copies.
    const LOCK: &str = "recovery-migration.lock";

    /// How long a launch waits for another one's move.
    pub(super) const WAIT: Duration = Duration::from_secs(5);

    /// Suffix of a copy staged for publishing: disposable, the original still exists.
    const STAGED: &str = ".moving";

    /// Suffix of an original set aside while it is copied to another volume: it may be the only
    /// copy, so it is put back, never thrown away, if the move doesn't finish.
    const HELD: &str = ".held";

    /// How deep [`copy_tree`] follows folders.
    const MAX_COPY_DEPTH: u32 = 16;

    /// Migration locks of the old folders this process uses, kept until it ends.
    static KEPT_LOCKS: std::sync::Mutex<Vec<std::fs::File>> = std::sync::Mutex::new(Vec::new());

    pub(super) fn adopt_waiting(legacy: &Path, dir: &Path, wait: Duration) -> PathBuf {
        let held_aside = || legacy.parent().is_some_and(|p| !side_entries(p, legacy, HELD).is_empty());
        if legacy == dir || (!legacy.is_dir() && !held_aside()) {
            return dir.to_path_buf();
        }
        // An empty old folder has nothing to move (`remove_dir` only removes an empty one).
        if !held_aside() && std::fs::remove_dir(legacy).is_ok() {
            return dir.to_path_buf();
        }
        let unmoved = || if dir.exists() || !legacy.is_dir() { dir.to_path_buf() } else { legacy.to_path_buf() };
        let Some(parent) = dir.parent() else { return unmoved() };
        let lock = std::fs::create_dir_all(parent)
            .and_then(|()| std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(parent.join(LOCK)));
        // No lock file means no other process can be moving: the old folder is safe to use.
        let Ok(lock) = lock else { return unmoved() };
        let deadline = Instant::now() + wait;
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
                // Another process is moving and may yet remove the old folder: never write there.
                Err(std::fs::TryLockError::WouldBlock) => return dir.to_path_buf(),
                // Locking fails for every process here alike, so none can be moving.
                Err(std::fs::TryLockError::Error(_)) => return unmoved(),
            }
        }
        // The lock file stays (removing it would race a process about to lock it); the OS
        // releases the lock when `lock` is dropped or the process ends.
        let used = adopt_locked(legacy, dir);
        if used == legacy {
            // This process writes to the old folder: it keeps the lock for as long as it runs, so
            // no other launch moves (and removes) the folder under it.
            KEPT_LOCKS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(lock);
        }
        used
    }

    /// The move itself, under the lock. First what a process that ended mid-move left is tidied:
    /// originals it set aside are put back, copies it staged are removed. An empty `legacy` is
    /// then just removed. Otherwise the whole folder moves if `dir` doesn't exist yet; when both
    /// exist, each copy moves as a whole (copy first, sidecar last) unless `dir` already has
    /// either part, and one that stays is left in `legacy` untouched.
    fn adopt_locked(legacy: &Path, dir: &Path) -> PathBuf {
        if let Some(parent) = legacy.parent() {
            for held in side_entries(parent, legacy, HELD) {
                put_back(&held, legacy);
            }
        }
        // Parts held aside inside the old folder by a per-copy move.
        let held_parts: Vec<(PathBuf, String)> = std::fs::read_dir(legacy)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_str().and_then(|n| side_of(n, HELD)).map(|orig| (e.path(), orig.to_owned())))
            .collect();
        for (held, original) in held_parts {
            put_back(&held, &legacy.join(original));
        }
        for folder in [dir.parent(), Some(dir)].into_iter().flatten() {
            remove_stale_staging(folder);
        }
        // Another process may have finished the move while this one waited.
        if !legacy.is_dir() {
            return dir.to_path_buf();
        }
        if std::fs::remove_dir(legacy).is_ok() {
            return dir.to_path_buf();
        }
        if !dir.exists() {
            // The common case: the whole folder moves at once.
            return match move_path(legacy, dir) {
                Ok(()) => dir.to_path_buf(),
                Err(_) if legacy.is_dir() => legacy.to_path_buf(),
                Err(_) => dir.to_path_buf(),
            };
        }
        move_entries(legacy, dir);
        // Only an emptied folder goes; anything that couldn't move stays for a later launch.
        let _ = std::fs::remove_dir(legacy);
        dir.to_path_buf()
    }

    /// Moves each copy of `from` into `to` (both folders): the copy first, then its sidecar,
    /// skipped if `to` has any part. Temp files of an interrupted write go along too.
    fn move_entries(from: &Path, to: &Path) {
        let Ok(entries) = std::fs::read_dir(from) else { return };
        let mut keys: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut temps = Vec::new();
        for name in entries.flatten().filter_map(|e| e.file_name().into_string().ok()) {
            if name.starts_with('.') {
                if name.ends_with(".craft_tmp") {
                    temps.push(name);
                }
            } else if let Some((stem, _)) = name.split_once('.') {
                keys.entry(stem.to_owned()).or_default().push(name);
            }
        }
        for mut parts in keys.into_values() {
            if parts.iter().any(|p| std::fs::symlink_metadata(to.join(p)).is_ok()) {
                continue;
            }
            // A sidecar must never move without its copy.
            parts.sort_by_key(|p| p.ends_with(".json"));
            for part in &parts {
                if move_path(&from.join(part), &to.join(part)).is_err() {
                    break;
                }
            }
        }
        for name in temps {
            let _ = move_path(&from.join(&name), &to.join(&name));
        }
    }

    fn already_exists(path: &Path) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::AlreadyExists, format!("{} already exists", path.display()))
    }

    /// A path beside `of`, named `.<name>.<pid>-<n><suffix>`, that no other call uses.
    fn side_path(of: &Path, suffix: &str) -> std::io::Result<PathBuf> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let parent = of.parent().ok_or_else(|| std::io::Error::other(format!("{} has no parent folder", of.display())))?;
        let name = of.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(parent.join(format!(".{name}.{}-{}{suffix}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))))
    }

    /// The name `side` was made from, if it is a [`side_path`] name with `suffix`.
    fn side_of<'a>(side: &'a str, suffix: &str) -> Option<&'a str> {
        let rest = side.strip_prefix('.')?.strip_suffix(suffix)?;
        let (name, ids) = rest.rsplit_once('.')?;
        let (pid, n) = ids.split_once('-')?;
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        (!name.is_empty() && digits(pid) && digits(n)).then_some(name)
    }

    /// The side paths with `suffix` in `folder` made from `of`.
    fn side_entries(folder: &Path, of: &Path, suffix: &str) -> Vec<PathBuf> {
        let Some(name) = of.file_name().and_then(|n| n.to_str()) else { return Vec::new() };
        std::fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_str().and_then(|n| side_of(n, suffix)) == Some(name))
            .map(|e| e.path())
            .collect()
    }

    /// Moves the file or folder `from` to `to` without ever replacing anything at `to`. On one
    /// volume a folder is renamed and a file is hard-linked, then unlinked at `from`. Otherwise
    /// (another volume) it is copied ([`move_across`]).
    fn move_path(from: &Path, to: &Path) -> std::io::Result<()> {
        if std::fs::symlink_metadata(to).is_ok() {
            return Err(already_exists(to));
        }
        let meta = std::fs::symlink_metadata(from)?;
        let moved = if meta.is_dir() { std::fs::rename(from, to) } else { std::fs::hard_link(from, to).and_then(|()| std::fs::remove_file(from)) };
        match moved {
            Ok(()) => Ok(()),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::DirectoryNotEmpty) => Err(e),
            Err(_) => move_across(from, to),
        }
    }

    /// Moves `from` to `to` by copying. `from` is first renamed aside on its own volume, so a
    /// process still writing to the old path starts a new `from` there instead of adding to what
    /// is copied. The held original is copied to a staged path, published without replacing
    /// anything, and only then removed. If the copy or publish fails, the original is put back.
    fn move_across(from: &Path, to: &Path) -> std::io::Result<()> {
        let held = side_path(from, HELD)?;
        std::fs::rename(from, &held)?;
        match stage_copy(&held, to).and_then(|staged| publish(&staged, to)) {
            Ok(()) => {
                if std::fs::symlink_metadata(&held)?.is_dir() {
                    std::fs::remove_dir_all(&held)
                } else {
                    std::fs::remove_file(&held)
                }
            }
            Err(e) => {
                put_back(&held, from);
                Err(e)
            }
        }
    }

    /// Returns the held original `held` to `original`. If `original` has been made again
    /// meanwhile, the held folder's entries move into it without replacing anything; whatever
    /// can't move stays held and is tried again under the next move's lock. Nothing held is ever
    /// removed with data in it.
    fn put_back(held: &Path, original: &Path) {
        if std::fs::symlink_metadata(original).is_err() && std::fs::rename(held, original).is_ok() {
            return;
        }
        if held.is_dir() && original.is_dir() {
            move_entries(held, original);
            let _ = std::fs::remove_dir(held);
        }
    }

    /// Copies `from` to a staging path beside `to` that this call creates exclusively. On failure
    /// the staged copy is removed.
    fn stage_copy(from: &Path, to: &Path) -> std::io::Result<PathBuf> {
        let staged = side_path(to, STAGED)?;
        let is_dir = std::fs::symlink_metadata(from)?.is_dir();
        let copied = if is_dir { copy_tree(from, &staged, 0) } else { copy_file(from, &staged) };
        if let Err(e) = copied {
            // `AlreadyExists` means `staged` was someone else's to keep.
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                let _ = if is_dir { std::fs::remove_dir_all(&staged) } else { std::fs::remove_file(&staged) };
            }
            return Err(e);
        }
        Ok(staged)
    }

    /// Removes copies a process left staged in `folder` when it ended mid-move: disposable, since
    /// the original is still in place or held aside. Links are never followed.
    fn remove_stale_staging(folder: &Path) {
        let Ok(entries) = std::fs::read_dir(folder) else { return };
        for entry in entries.flatten() {
            if entry.file_name().to_str().is_none_or(|n| side_of(n, STAGED).is_none()) {
                continue;
            }
            let _ = match entry.file_type() {
                Ok(t) if t.is_dir() => std::fs::remove_dir_all(entry.path()),
                Ok(t) if t.is_file() => std::fs::remove_file(entry.path()),
                _ => continue,
            };
        }
    }

    /// Gives the staged copy its final name `to` without replacing anything there. On failure the
    /// staged copy is removed and `to` is untouched.
    fn publish(staged: &Path, to: &Path) -> std::io::Result<()> {
        let is_dir = std::fs::symlink_metadata(staged)?.is_dir();
        let published = if std::fs::symlink_metadata(to).is_ok() {
            Err(already_exists(to))
        } else if is_dir {
            std::fs::rename(staged, to)
        } else {
            std::fs::hard_link(staged, to).and_then(|()| std::fs::remove_file(staged))
        };
        if published.is_err() {
            let _ = if is_dir { std::fs::remove_dir_all(staged) } else { std::fs::remove_file(staged) };
        }
        published
    }

    /// Copies the folder `from` to the new folder `to`: plain files and folders only, at most
    /// [`MAX_COPY_DEPTH`] levels.
    fn copy_tree(from: &Path, to: &Path, depth: u32) -> std::io::Result<()> {
        if depth > MAX_COPY_DEPTH {
            return Err(std::io::Error::other(format!("{} is nested too deep", from.display())));
        }
        std::fs::create_dir(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let target = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_tree(&entry.path(), &target, depth.saturating_add(1))?;
            } else {
                copy_file(&entry.path(), &target)?;
            }
        }
        Ok(())
    }

    /// Copies one plain file to the new file `to`, flushed to disk.
    fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
        if !std::fs::symlink_metadata(from)?.file_type().is_file() {
            return Err(std::io::Error::other(format!("{} is not a plain file", from.display())));
        }
        let mut src = std::fs::File::open(from)?;
        let mut dst = std::fs::OpenOptions::new().write(true).create_new(true).open(to)?;
        std::io::copy(&mut src, &mut dst)?;
        dst.sync_all()
    }
}
