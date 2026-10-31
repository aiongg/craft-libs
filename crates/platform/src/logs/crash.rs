#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Panics: a panic anywhere (UI pass, frame workers, background jobs) is logged with its thread
//! and location, written to a crash log with a backtrace, and kept for the app's error window.
//!
//! Code returns errors and never panics; this is the net for one that escapes anyway. Without it
//! a panic on the UI thread closes a desktop launch silently, and one in a worker leaves a view
//! blank with no trace. [`catch`] turns a panic inside an import, an export or a command into an
//! error the app shows, so the open documents stay in place.
//!
//! The crash log is `<dir>/crash-<days since 1970>.log`, appended to, one entry per panic:
//! `=== <App> <version> (unix <secs>)`, the summary, the backtrace. On the web there is no file;
//! the previous hook still prints to the browser console.
//!
//! ```ignore
//! crash::install("PhotoCraft", env!("CARGO_PKG_VERSION"), Some(log_dir));
//! let opened = crash::catch(|| import(path))
//!     .unwrap_or_else(|_| Err(t("Open failed with an internal error (logged); your open documents are unchanged")));
//! if let Some(summary) = crash::take_last() { error_window.show(summary); }
//! ```

use std::path::PathBuf;

/// A panic caught by [`catch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panicked {
    /// The panic message (`"panic"` when it carried no text).
    pub message: String,
}

impl std::fmt::Display for Panicked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // new: the message alone; the app words the error the user sees
        todo!()
    }
}

impl std::error::Error for Panicked {}

/// Install the panic hook, once per process; later calls only change `app`, `version` and the
/// crash log directory (`None`: standard error and the log only, as on the web and in tests).
///
/// The hook stores `<message> (at <file>:<line>, thread <name>)` for [`take_last`], logs it with
/// `log::error!("<app> internal error: …")`, appends it with a forced backtrace to the crash log,
/// then calls the previous hook, so the panic still reaches standard error or the console.
pub fn install(app: &'static str, version: &'static str, dir: Option<PathBuf>) {
    // filmcraft: crates/ui-egui/src/crash.rs:15 with pc: apps/photocraft/src/crash_guard.rs:11 (log::error!)
    todo!()
}

/// Write a fatal error that isn't a panic (the window could not be created) to the crash log.
pub fn record(message: &str) {
    // filmcraft: crates/ui-egui/src/crash.rs:45
    todo!()
}

/// Take the last panic summary; the app's error window shows it once.
pub fn take_last() -> Option<String> {
    // filmcraft: crates/ui-egui/src/crash.rs:50
    todo!()
}

/// Today's crash log file, when [`install`] was given a directory.
pub fn log_path() -> Option<PathBuf> {
    // filmcraft: crates/ui-egui/src/crash.rs:55
    todo!()
}

/// Run `f`; a panic inside it is logged by the hook and comes back as [`Panicked`].
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, Panicked> {
    // pc: apps/photocraft/src/crash_guard.rs:20
    todo!()
}

#[cfg(test)]
#[path = "crash_tests.rs"]
mod crash_tests;
