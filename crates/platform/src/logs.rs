#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! The app's log: `log::` records go to standard error and to a log file a bug report can attach,
//! and [`crash`] writes panics to a crash log.
//!
//! A launch from a desktop menu has no terminal, so without the file every `log::warn!` (a GPU
//! fallback, a lost device, a panicked worker) is gone. The file is `<dir>/<app>.log` in a
//! directory the app chooses (PhotoCraft: `<settings dir>/logs`; FilmCraft: `<data dir>/Logs`,
//! which Help › Reveal Log Files opens). Each start moves the previous log to `<app>.1.log` and
//! that one to `.2`, so the log of a run that crashed survives the next launch. A start while
//! another instance still writes `<app>.log` leaves it alone and writes `<app>-<pid>.log`.
//!
//! Levels: `info` for the app's own crates, `warn` for the rest (wgpu and naga are chatty).
//! `RUST_LOG` replaces that with env_logger-style directives (`debug`, `warn,photocraft_gpu=trace`);
//! a directive ending in `*` matches every target starting with it (`photocraft*=debug`).
//!
//! Records logged before the directory is known are kept (up to [`MAX_PENDING`]) and written
//! once the file is attached. Writing never panics: a file that can't be created or written
//! leaves standard error as the only sink. A record takes the sink lock for one `write_all` of an
//! already formatted line, so a realtime thread that logs never waits on more than one line.
//!
//! ```ignore
//! const NAMES: LogNames = LogNames::new("photocraft");
//! let logger = craft_platform::logs::install(NAMES);
//! logs::crash::install("PhotoCraft", VERSION, None);
//! // after argument parsing, once the settings directory is known:
//! if let (Some(logger), Some(dir)) = (logger, config_dir()) {
//!     match logger.attach_dir(&dir.join("logs")) {
//!         Ok(path) => log::info!("PhotoCraft {version}, log file {}", path.display()),
//!         Err(e) => log::warn!("no log file: {e}"),
//!     }
//!     logs::crash::install("PhotoCraft", VERSION, Some(dir.join("logs")));
//! }
//! ```

pub mod crash;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use log::LevelFilter;

/// How many previous logs are kept (`<app>.1.log` … `<app>.<KEEP>.log`), and how many
/// side-by-side instance logs of exited instances.
pub const KEEP: usize = 2;
/// The log file stops growing past this size, with one line saying so (a runaway warning can't
/// fill the disk).
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Records kept in memory until the log file is attached.
pub const MAX_PENDING: usize = 512;
/// Held while one start rotates and prunes the logs, so two starts at once can't interleave.
pub const ROTATE_LOCK: &str = "rotate.lock";

/// The app's log file names, from its lower-case name: `photocraft` gives `photocraft.log`,
/// `photocraft.1.log`, `photocraft-<pid>.log`, the owner lock `photocraft.lock`, and the default
/// filter `warn,photocraft*=info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogNames {
    stem: &'static str,
}

impl LogNames {
    /// Names for the app `stem`, the lower-case app name its crates start with.
    pub const fn new(stem: &'static str) -> Self {
        Self { stem }
    }

    /// The current log file's name: `<stem>.log`.
    pub fn file(&self) -> String {
        // pc: apps/photocraft/src/logging.rs:29
        todo!()
    }

    /// The filter when `RUST_LOG` is unset or blank: `warn,<stem>*=info`.
    pub fn default_filter(&self) -> String {
        // pc: apps/photocraft/src/logging.rs:37
        todo!()
    }

    /// `<stem>.<n>.log`, and [`Self::file`] for `n == 0`.
    fn numbered(&self, n: usize) -> String {
        // pc: apps/photocraft/src/logging.rs:126
        todo!()
    }

    /// The lock file the instance writing [`Self::file`] holds until it exits: `<stem>.lock`. A
    /// separate file, since Windows locks are mandatory and a locked log couldn't be opened for a
    /// bug report while the app runs.
    fn owner_lock(&self) -> String {
        // pc: apps/photocraft/src/logging.rs:133
        todo!()
    }

    /// The log of an instance started while another one wrote [`Self::file`]: `<stem>-<pid>.log`.
    fn instance_log(&self, pid: u32) -> String {
        // pc: apps/photocraft/src/logging.rs:138
        todo!()
    }

    /// The lock marking [`Self::instance_log`] as in use: `<stem>-<pid>.lock`.
    fn instance_lock(&self, pid: u32) -> String {
        // pc: apps/photocraft/src/logging.rs:143
        todo!()
    }

    /// The process id in an instance log's file name; `None` for any other name.
    fn instance_pid(&self, name: &str) -> Option<u32> {
        // pc: apps/photocraft/src/logging.rs:148
        todo!()
    }
}

/// Per-target level filter parsed from env_logger-style directives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    default: LevelFilter,
    directives: Vec<(String, LevelFilter)>,
}

impl Filter {
    /// Parse `spec` (`warn,photocraft*=info`). Unknown levels and empty parts are skipped; a bare
    /// target name turns everything from it on (as env_logger does). Without a bare level,
    /// targets no directive names log errors only. Never panics.
    pub fn parse(spec: &str) -> Self {
        // pc: apps/photocraft/src/logging.rs:49
        todo!()
    }

    /// The level for `target`: the most specific matching directive (`a` matches `a` and `a::b`,
    /// not `ab`; `a*` matches every target starting with `a`), else the default.
    pub fn level_for(&self, target: &str) -> LevelFilter {
        // pc: apps/photocraft/src/logging.rs:70
        todo!()
    }

    /// The most verbose level any target can reach, for `log::set_max_level`.
    pub fn max(&self) -> LevelFilter {
        // pc: apps/photocraft/src/logging.rs:75
        todo!()
    }
}

/// `2026-10-08T07:59:17.728Z`: UTC with milliseconds. A clock before 1970 reads as the epoch; a
/// year past 9999 is written in full, without a `+`. No chrono: the date comes from the day
/// count by the civil-from-days algorithm.
pub fn timestamp(t: SystemTime) -> String {
    // pc: apps/photocraft/src/logging.rs:96
    todo!()
}

/// One formatted line: `<timestamp> <LEVEL> [<thread>] <target>: <message>\n`, the message's own
/// trailing newline dropped.
pub fn format_line(ts: &str, level: log::Level, thread: &str, target: &str, message: &str) -> String {
    // pc: apps/photocraft/src/logging.rs:245
    todo!()
}

/// A log file opened by [`rotate`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct LogFile {
    pub path: PathBuf,
    pub file: File,
    /// The held lock that marks `path` as in use, until the logger is dropped; `None` where the
    /// file system has no locks.
    owner: Option<File>,
}

/// Open a fresh, empty log file in `dir` (created if missing).
///
/// Normally the previous logs shift up one (`.log` → `.1.log` → … → `.<KEEP>.log`, the oldest
/// dropped). While another instance holds the owner lock, the chain is left alone and this start
/// writes `<stem>-<pid>.log`: rotating would move that instance's live log away. Instance logs of
/// exited instances are pruned to the newest [`KEEP`]. On a file system without locks every start
/// rotates. [`ROTATE_LOCK`] serialises two starts at once.
#[cfg(not(target_arch = "wasm32"))]
pub fn rotate(dir: &Path, names: LogNames) -> std::io::Result<LogFile> {
    // pc: apps/photocraft/src/logging.rs:177 (rotate_locked :186, prune_instance_logs :214)
    todo!()
}

/// Where formatted lines go besides standard error: kept in memory until a file is attached,
/// then appended to it until it reaches its size cap.
#[derive(Debug)]
pub struct Sink {
    #[cfg(not(target_arch = "wasm32"))]
    file: Option<File>,
    pending: Vec<String>,
    dropped: usize,
    written: u64,
    max_bytes: u64,
    capped: bool,
}

impl Sink {
    /// An empty sink whose file stops at `max_bytes`.
    pub fn new(max_bytes: u64) -> Self {
        // pc: apps/photocraft/src/logging.rs:262
        todo!()
    }

    /// Append `line` to the file, or keep it (up to [`MAX_PENDING`], counting the rest) until
    /// [`Self::attach`].
    pub fn write(&mut self, line: &str) {
        // pc: apps/photocraft/src/logging.rs:267
        todo!()
    }

    /// Start writing to `file`: first the kept lines, then, if any were dropped, the line
    /// `{n} earlier log lines were dropped before the log file was opened\n`. A line that would
    /// take the file past `max_bytes` is replaced by
    /// `log file reached {max_bytes} bytes; later records go to standard error only\n` and nothing
    /// follows it. A write error drops the file and stops keeping lines (no file will come).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn attach(&mut self, file: File) {
        // pc: apps/photocraft/src/logging.rs:280
        todo!()
    }

    /// No file will come: drop the kept lines and keep none from now on.
    pub fn close(&mut self) {
        // filmcraft: apps/filmcraft/src/logging.rs:202
        todo!()
    }
}

/// The installed logger: a [`Filter`], standard error and a [`Sink`].
#[derive(Debug)]
pub struct AppLogger {
    names: LogNames,
    filter: Filter,
    stderr: bool,
    sink: Mutex<Sink>,
    /// The lock marking the attached file as in use (see [`rotate`]); held until exit.
    #[cfg(not(target_arch = "wasm32"))]
    owner: Mutex<Option<File>>,
}

impl AppLogger {
    /// A logger for `names` with `filter`, also writing to standard error when `stderr`.
    pub fn new(names: LogNames, filter: Filter, stderr: bool) -> Self {
        // pc: apps/photocraft/src/logging.rs:323
        todo!()
    }

    /// Rotate the logs in `dir` and send records to the fresh file; returns its path. An unusable
    /// directory leaves standard error as the only sink ([`Self::stderr_only`]) and returns
    /// `<dir>: <error>`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn attach_dir(&self, dir: &Path) -> Result<PathBuf, String> {
        // pc: apps/photocraft/src/logging.rs:328 with filmcraft: apps/filmcraft/src/logging.rs:244 (stderr_only on error)
        todo!()
    }

    /// No log file this run (no directory, or it can't be used): stop keeping records for one.
    pub fn stderr_only(&self) {
        // filmcraft: apps/filmcraft/src/logging.rs:258
        todo!()
    }
}

impl log::Log for AppLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        // pc: apps/photocraft/src/logging.rs:337
        todo!()
    }

    /// Format outside the lock (timestamp, level, thread name or `?`, target), write standard
    /// error outside the lock, then the sink; a poisoned lock is taken over, never a panic.
    fn log(&self, record: &log::Record<'_>) {
        // pc: apps/photocraft/src/logging.rs:341
        todo!()
    }

    fn flush(&self) {}
}

/// Install the logger for `names` (filter from `RUST_LOG`, else [`LogNames::default_filter`])
/// and set `log`'s max level; `None` when another logger was installed first. Call
/// [`AppLogger::attach_dir`] once the directory is known, after argument parsing, so `--version`
/// and usage errors leave no file behind.
pub fn install(names: LogNames) -> Option<&'static AppLogger> {
    // pc: apps/photocraft/src/logging.rs:358
    todo!()
}

#[cfg(test)]
#[path = "logs_tests.rs"]
mod logs_tests;
