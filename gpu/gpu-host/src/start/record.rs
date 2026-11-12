//! The start-up record, `gpu-start.json` in the app's data folder: one file for everything a
//! launch learns from the launches before it.
//!
//! - `starting`: the start in progress (its rung, and its adapter once one is picked), written
//!   before the graphics start and cleared once the first frame is shown, the start ends without
//!   crashing, or [`DEATH_WINDOW`] after launch. A launch that finds one left behind knows the
//!   start before it died in the driver (VectorCraft #2243, FilmCraft #512).
//! - `crashed`: the adapters ([`crate::adapter::key`]s) such starts died on, left out until one of
//!   them shows a frame.
//! - `rung`: the rung to start with, when it isn't the top of the ladder.
//! - `test`: the last test start's rung and display drivers ([`crate::probe`]).
//! - `scope`: driver work in progress outside the window's start ([`Record::scope`]).
//!
//! The record of another app version or Graphics Processor preference is forgotten: drivers and
//! wgpu move on, and another preference picks another adapter.
//!
//! The window's start keeps the file open and locked until its first frame or its end, so a second
//! copy of the app starting at the same moment doesn't take the record for that of a start that
//! crashed: it records nothing and leaves nothing out. Other changes lock the file for the time
//! of the change. Writes go into the locked file itself (renaming a new file into place would
//! leave the lock on the old one); a write torn by a crash reads as no record, which starts from
//! the top of the ladder.

use std::fs::File;
use std::io::{Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::ladder::Rung;

/// The record's file name in the data folder.
pub const FILE: &str = "gpu-start.json";

/// How long after launch a start that has shown no frame may die and still count as crashed. A
/// driver crash in instance or device creation comes within a second or two; a start alive longer
/// is hanging or slow, which the first-frame watch handles.
pub const DEATH_WINDOW: Duration = Duration::from_secs(15);

/// The most bytes read: anything longer is not ours.
const LIMIT: u64 = 64 * 1024;
/// The most crashed adapters kept.
const CRASHED_LIMIT: usize = 16;
/// The longest name or text kept from the file.
const MAX_TEXT: usize = 256;
/// How often, and how far apart, a lock held elsewhere is asked for again before the file counts
/// as busy. Between `fork` and `exec` a child process briefly shares the file's lock, so a lock
/// can look held just after its owner let it go (PhotoCraft).
const LOCK_TRIES: u32 = 50;
const LOCK_PAUSE: Duration = Duration::from_millis(10);

/// An adapter in the record: its [`crate::adapter::key`] and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Recorded {
    pub key: String,
    pub name: String,
}

/// The start in progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Starting {
    pub rung: Rung,
    /// The adapter picked; `None` while the adapters are being listed.
    pub adapter: Option<Recorded>,
    /// The listing found an adapter of a backend other than OpenGL.
    pub native: bool,
}

/// The last test start ([`crate::probe`]): its rung and the display drivers it ran with, when its
/// start began, and whether that start is still open (it has neither shown a frame nor ended).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Test {
    pub rung: Rung,
    pub drivers: String,
    pub started: u64,
    pub open: bool,
}

/// Driver work in progress outside the window's start, and when it began.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InScope {
    pub what: String,
    pub started: u64,
}

/// What `gpu-start.json` holds (see the module docs), and the record's entry points for code
/// outside the window's start.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub(crate) version: String,
    pub(crate) preference: String,
    pub(crate) rung: Option<Rung>,
    pub(crate) starting: Option<Starting>,
    pub(crate) crashed: Vec<Recorded>,
    pub(crate) test: Option<Test>,
    pub(crate) scope: Option<InScope>,
}

impl Record {
    /// The record folder of driver work outside the window's start ([`Self::scope`]);
    /// [`super::GraphicsStart::begin`] sets it to its own folder. `None` records nothing.
    pub fn set_folder(folder: Option<&Path>) {
        *FOLDER.lock().unwrap_or_else(PoisonError::into_inner) = folder.map(Path::to_path_buf);
    }

    /// Run driver work `f` (`what`: "device creation", "kernel compilation") inside a record scope:
    /// with a record folder set, the record names the work until it returns or panics. Scopes that
    /// overlap, on any thread, share one entry until the last one ends. A process that dies in the
    /// driver meanwhile (Apple's Metal compiler aborting on a kernel: LightCraft #136, #250) leaves
    /// the entry for [`Self::take_crashed_scope`].
    pub fn scope<T>(what: &str, f: impl FnOnce() -> T) -> T {
        let _scope = Self::enter(what);
        f()
    }

    /// Enter a record scope until the guard drops ([`Self::scope`], for async work).
    pub(crate) fn enter(what: &str) -> Option<ScopeGuard> {
        let folder = FOLDER.lock().unwrap_or_else(PoisonError::into_inner).clone();
        enter_on(folder, &ACTIVE, what)
    }

    /// What a scope left in `folder`'s record when its process died inside it, removed from the
    /// record so it is reported once; `None` when there is none. Call it before this process
    /// enters a scope of its own: an app turns its own GPU work off for the launch that gets one.
    pub fn take_crashed_scope(folder: &Path) -> Option<String> {
        let mut taken = None;
        match update(folder, |r| taken = r.scope.take()) {
            Ok(true) => {}
            Ok(false) => return None,
            Err(e) => log::warn!("gpu: {}: {e}", folder.join(FILE).display()),
        }
        taken.map(|s| format!("{} (started at unix time {})", s.what, s.started))
    }

    /// The record from `text`; `None` when it isn't one. Damaged or foreign entries are dropped
    /// one by one; text that isn't a record object at all is no record.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let text_of =
            |v: &serde_json::Value, key: &str| v.get(key).and_then(serde_json::Value::as_str).map(|s| s.chars().take(MAX_TEXT).collect::<String>());
        let rung_of = |v: &serde_json::Value| v.get("rung").and_then(serde_json::Value::as_str).and_then(Rung::parse);
        let recorded = |v: &serde_json::Value| Some(Recorded { key: text_of(v, "key")?, name: text_of(v, "name")? });
        let starting = v.get("starting").and_then(|s| {
            Some(Starting {
                rung: rung_of(s)?,
                adapter: s.get("adapter").and_then(recorded),
                native: s.get("native").and_then(serde_json::Value::as_bool).unwrap_or(false),
            })
        });
        let test = v.get("test").and_then(|t| {
            Some(Test {
                rung: rung_of(t)?,
                drivers: t.get("drivers")?.as_str()?.to_string(),
                started: t.get("started")?.as_u64()?,
                open: t.get("open")?.as_bool()?,
            })
        });
        let scope = v.get("scope").and_then(|s| Some(InScope { what: text_of(s, "what")?, started: s.get("started")?.as_u64()? }));
        let crashed = v
            .get("crashed")
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |a| a.iter().filter_map(recorded).take(CRASHED_LIMIT).collect());
        Some(Self { version: text_of(&v, "version")?, preference: text_of(&v, "preference")?, rung: rung_of(&v), starting, crashed, test, scope })
    }

    pub(crate) fn to_json(&self) -> String {
        let recorded = |r: &Recorded| serde_json::json!({ "key": r.key, "name": r.name });
        let mut v = serde_json::json!({
            "version": self.version,
            "preference": self.preference,
            "crashed": self.crashed.iter().map(recorded).collect::<Vec<_>>(),
        });
        if let Some(map) = v.as_object_mut() {
            if let Some(r) = self.rung {
                map.insert("rung".into(), r.name().into());
            }
            if let Some(s) = &self.starting {
                map.insert(
                    "starting".into(),
                    serde_json::json!({ "rung": s.rung.name(), "adapter": s.adapter.as_ref().map(recorded), "native": s.native }),
                );
            }
            if let Some(t) = &self.test {
                map.insert("test".into(), serde_json::json!({ "rung": t.rung.name(), "drivers": t.drivers, "started": t.started, "open": t.open }));
            }
            if let Some(s) = &self.scope {
                map.insert("scope".into(), serde_json::json!({ "what": s.what, "started": s.started }));
            }
        }
        v.to_string()
    }

    /// Add `adapter` to the crashed adapters, once, dropping the oldest beyond the limit.
    pub(crate) fn add_crashed(&mut self, adapter: Recorded) {
        if self.crashed.iter().any(|c| c.key == adapter.key) {
            return;
        }
        self.crashed.push(adapter);
        if self.crashed.len() > CRASHED_LIMIT {
            self.crashed.remove(0);
        }
    }
}

/// Seconds since the Unix epoch.
pub(crate) fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

// ------------------------------------------------------------------------------------ the file

/// What [`hold`] found.
pub(crate) enum Opened {
    /// The file is this start's until [`release`]; the record the last start left, if any.
    Held(Option<Record>),
    /// Another start holds it.
    Busy,
    /// It can't be used (the folder can't be written).
    Failed(std::io::Error),
}

/// A record file a start in this process holds locked.
struct Held {
    folder: PathBuf,
    file: File,
}

/// The record files held by starts in this process, by folder.
static HELD: Mutex<Vec<Held>> = Mutex::new(Vec::new());

fn open(folder: &Path) -> std::io::Result<File> {
    std::fs::create_dir_all(folder)?;
    std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(folder.join(FILE))
}

/// Lock `file`, asking again for a briefly held lock; `Ok(false)` when it stays held.
fn lock(file: &File) -> std::io::Result<bool> {
    for _ in 0..LOCK_TRIES {
        match file.try_lock() {
            Ok(()) => return Ok(true),
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(LOCK_PAUSE),
            Err(std::fs::TryLockError::Error(e)) => return Err(e),
        }
    }
    Ok(false)
}

fn read_from(mut file: &File) -> Option<Record> {
    file.rewind().ok()?;
    let mut text = String::new();
    file.take(LIMIT).read_to_string(&mut text).ok()?;
    Record::parse(&text)
}

fn write_to(mut file: &File, record: &Record) -> std::io::Result<()> {
    file.set_len(0)?;
    file.rewind()?;
    file.write_all(record.to_json().as_bytes())?;
    file.flush()
}

/// Open `folder`'s record and keep it locked for a start, until [`release`].
pub(crate) fn hold(folder: &Path) -> Opened {
    let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    if held.iter().any(|h| h.folder == folder) {
        return Opened::Busy;
    }
    let file = match open(folder) {
        Ok(f) => f,
        Err(e) => return Opened::Failed(e),
    };
    match lock(&file) {
        Ok(true) => {}
        Ok(false) => return Opened::Busy,
        Err(e) => return Opened::Failed(e),
    }
    let previous = read_from(&file);
    held.push(Held { folder: folder.to_path_buf(), file });
    Opened::Held(previous)
}

/// Let `folder`'s record go: the next start may take it.
pub(crate) fn release(folder: &Path) {
    HELD.lock().unwrap_or_else(PoisonError::into_inner).retain(|h| h.folder != folder);
}

/// Change `folder`'s record and write it: through the start's own lock when a start in this
/// process holds it, otherwise under a lock for the time of the change. `Ok(false)` when another
/// process holds it.
pub(crate) fn update(folder: &Path, change: impl FnOnce(&mut Record)) -> std::io::Result<bool> {
    let held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(h) = held.iter().find(|h| h.folder == folder) {
        return change_in(&h.file, change).map(|()| true);
    }
    drop(held);
    let file = open(folder)?;
    if !lock(&file)? {
        return Ok(false);
    }
    change_in(&file, change).map(|()| true)
}

/// [`update`] only through a start's own lock; `false` when no start in this process holds it.
pub(crate) fn update_held(folder: &Path, change: impl FnOnce(&mut Record)) -> bool {
    let held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(h) = held.iter().find(|h| h.folder == folder) else { return false };
    if let Err(e) = change_in(&h.file, change) {
        log::warn!("gpu: {}: {e}", folder.join(FILE).display());
    }
    true
}

fn change_in(file: &File, change: impl FnOnce(&mut Record)) -> std::io::Result<()> {
    let mut record = read_from(file).unwrap_or_default();
    let before = record.clone();
    change(&mut record);
    if record == before { Ok(()) } else { write_to(file, &record) }
}

/// `folder`'s record as it is on disk, without a lock (tests, the log).
#[cfg(test)]
pub(crate) fn read(folder: &Path) -> Option<Record> {
    let mut text = String::new();
    File::open(folder.join(FILE)).ok()?.take(LIMIT).read_to_string(&mut text).ok()?;
    Record::parse(&text)
}

// ---------------------------------------------------------------------------------- the scopes

/// The folder [`Record::set_folder`] set.
static FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
/// Record scopes running now in this process.
static ACTIVE: Mutex<u32> = Mutex::new(0);

/// A record scope; dropping it, a panic included, ends it.
pub(crate) struct ScopeGuard {
    folder: PathBuf,
    active: &'static Mutex<u32>,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let mut n = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        *n = n.saturating_sub(1);
        if *n == 0 {
            change_scope(&self.folder, |r| r.scope = None);
        }
    }
}

/// [`Record::enter`] on `folder` with its own scope counter `active` (the tests use their own, so
/// scopes of tests running in parallel don't join).
pub(crate) fn enter_on(folder: Option<PathBuf>, active: &'static Mutex<u32>, what: &str) -> Option<ScopeGuard> {
    let folder = folder?;
    let mut n = active.lock().unwrap_or_else(PoisonError::into_inner);
    if *n == 0 {
        let entry = InScope { what: what.chars().take(MAX_TEXT).collect(), started: unix_now() };
        change_scope(&folder, |r| r.scope = Some(entry));
    }
    *n = n.saturating_add(1);
    Some(ScopeGuard { folder, active })
}

fn change_scope(folder: &Path, change: impl FnOnce(&mut Record)) {
    match update(folder, change) {
        Ok(true) => {}
        Ok(false) => log::info!("gpu: {} is held by another start: this scope isn't recorded", folder.join(FILE).display()),
        Err(e) => log::warn!("gpu: {}: {e}", folder.join(FILE).display()),
    }
}
