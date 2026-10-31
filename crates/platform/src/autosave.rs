#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Autosave and crash recovery: when to autosave ([`Autosave`]), where the copies go
//! ([`recovery_dir`]), and the folder of copies ([`RecoveryStore`]).
//!
//! The app serializes its documents; this module decides when, writes the bytes with a small
//! JSON sidecar, removes a copy when its document is saved or closed, and lists the copies an
//! earlier run left behind. The recover-on-launch question is
//! `craft_ui::dialog::RecoveryDialog` (or none: PhotoCraft reopens them without asking).

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

/// The recovery folder's name.
pub const RECOVERY_DIR: &str = "Recovery";

/// PhotoCraft's default interval, `fileHandling.autosaveMinutes = 10` (minimum 1).
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(10 * 60);

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

/// When to autosave, after PhotoCraft's timer.
///
/// The first [`due`](Self::due) starts the clock; each interval after it, every document that is
/// dirty, whose revision differs from the one last autosaved, and that has no autosave in flight
/// is due. The app writes them (on a job) and reports each with [`finished`](Self::finished).
/// Saving or closing a document is [`forget`](Self::forget) (and the app discards its copy).
/// With autosave off (`set_interval(None)`) nothing is ever due.
#[derive(Debug)]
pub struct Autosave {
    interval: Option<Duration>,
    next: Option<Instant>,
    saved: HashMap<u64, u64>,
    in_flight: HashSet<u64>,
}

impl Autosave {
    /// `None` turns autosave off (PhotoCraft's `fileHandling.autosave = false`).
    pub fn new(interval: Option<Duration>) -> Self {
        Self { interval, next: None, saved: HashMap::new(), in_flight: HashSet::new() }
    }

    /// A changed preference; the next deadline moves to now plus the new interval.
    pub fn set_interval(&mut self, interval: Option<Duration>) {
        todo!()
    }

    /// The ids to autosave now; they count as in flight until [`finished`](Self::finished).
    // pc: crates/ui-egui/src/prefs/prefs_ui/save.rs:228 autosave
    pub fn due(&mut self, now: Instant, docs: impl IntoIterator<Item = AutosaveDoc>) -> Vec<u64> {
        todo!()
    }

    /// An autosave ended. On success `revision` is remembered; on failure the status text
    /// ([`AUTOSAVE_FAILED`] filled in) comes back for the status bar.
    pub fn finished(&mut self, id: u64, revision: u64, result: Result<(), String>) -> Option<String> {
        todo!()
    }

    /// The document was saved or closed.
    // pc: crates/ui-egui/src/prefs/prefs_ui/save.rs:263 (services.discard_autosave)
    pub fn forget(&mut self, id: u64) {
        todo!()
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
    todo!()
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
    todo!()
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
/// The folder is created owner-only (0o700 on Unix). [`open`](Self::open) takes an exclusive
/// `instance-<pid>-<n>.lock`, so [`leftovers`](Self::leftovers) offers only copies whose run is
/// gone, never those of another instance still running (PdfCraft).
#[cfg(not(target_arch = "wasm32"))]
pub struct RecoveryStore {
    dir: PathBuf,
    session: String,
    adopted: HashMap<u64, String>,
    lock: std::fs::File,
}

#[cfg(not(target_arch = "wasm32"))]
impl RecoveryStore {
    // pc: crates/format/src/autosave.rs:348 RecoveryStore::new; pdfcraft: crates/ui-egui/src/persist/recovery.rs:103 claim
    pub fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        todo!()
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The key document `id` autosaves under.
    // pc: crates/format/src/autosave.rs:380 (doc-<session>-<id>)
    pub fn key(&self, id: u64) -> String {
        todo!()
    }

    /// Writes a copy of document `id`; call from a job.
    pub fn write(&self, id: u64, meta: &RecoveryMeta, bytes: &[u8]) -> std::io::Result<()> {
        todo!()
    }

    /// Removes document `id`'s copy (saved or closed).
    // pc: crates/format/src/autosave.rs:411 discard
    pub fn discard(&mut self, id: u64) -> std::io::Result<()> {
        todo!()
    }

    /// Copies of runs that are gone, newest first; a copy without a readable sidecar is listed
    /// under its key with what the file system knows.
    // pc: crates/format/src/autosave.rs:361 recover
    pub fn leftovers(&self) -> Vec<RecoveryEntry> {
        todo!()
    }

    /// Document `id` was recovered from `entry`: its autosaves go to `entry.key` from now on.
    // pc: crates/format/src/autosave.rs:367 adopt
    pub fn adopt(&mut self, id: u64, entry: &RecoveryEntry) {
        todo!()
    }

    /// Deletes a leftover the user discarded.
    pub fn remove(&self, entry: &RecoveryEntry) -> std::io::Result<()> {
        todo!()
    }
}

/// A key safe as a file stem: ASCII letters, digits, `-` and `_` stay, everything else becomes
/// `_`; an empty key becomes `untitled`.
// pc: crates/format/src/autosave.rs:67 sanitize
pub fn sanitize_key(key: &str) -> String {
    todo!()
}
