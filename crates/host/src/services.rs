//! The service traits of a [`Host`](crate::Host) and their default implementations.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Wakes the app's event loop so it runs a frame (`ctx.request_repaint()` in an egui app).
/// Services call it from any thread when work they did for the UI is ready. Cloning shares the
/// callback.
#[derive(Clone)]
pub struct Wake(Arc<dyn Fn() + Send + Sync>);

impl Wake {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(wake))
    }

    /// A wake that does nothing (tests, headless runs).
    pub fn none() -> Self {
        Self::new(|| {})
    }

    /// Asks the event loop for a frame.
    pub fn wake(&self) {
        (self.0)();
    }
}

impl fmt::Debug for Wake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Wake")
    }
}

/// Who the app is: shown by About, the updater and recovery.
#[derive(Clone, Debug)]
pub struct AppInfo {
    /// The settings folder name, as craft-settings' `APP_ID` (`"photocraft"`).
    pub id: &'static str,
    /// The display name (`"PhotoCraft"`).
    pub name: &'static str,
    /// `env!("CARGO_PKG_VERSION")`.
    pub version: &'static str,
}

/// Library features' settings sections in the app's settings file, one TOML table per section.
/// [`MemorySections`] keeps them in memory (tests, wasm without storage).
pub trait SectionStore: Send + Sync {
    fn read(&self, key: &str) -> Option<toml::Table>;
    fn write(&self, key: &str, table: toml::Table);
}

/// A library feature's settings: one serde struct, saved under `KEY`. Not craft-settings'
/// `Settings` trait, which is the app's whole settings struct.
pub trait Section: Serialize + DeserializeOwned + Default {
    /// The table name in the settings file (`"fonts_panel"`).
    const KEY: &'static str;
}

/// Sections kept in memory.
#[derive(Default)]
pub struct MemorySections(Mutex<HashMap<String, toml::Table>>);

impl SectionStore for MemorySections {
    fn read(&self, key: &str) -> Option<toml::Table> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).get(key).cloned()
    }

    fn write(&self, key: &str, table: toml::Table) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).insert(key.to_owned(), table);
    }
}

/// Command words by command id (`"file.save"`). craft-keymap implements it over the app's
/// catalog; [`NoCommands`] returns the English label it is given.
pub trait Commands: Send + Sync {
    fn label(&self, id: &str, english: &str) -> String;
    fn hint(&self, _id: &str) -> Option<String> {
        None
    }
}

/// English command labels.
pub struct NoCommands;

impl Commands for NoCommands {
    fn label(&self, _id: &str, english: &str) -> String {
        english.to_owned()
    }
}

/// Background work.
pub trait Jobs: Send + Sync {
    fn spawn(&self, name: &str, job: Box<dyn FnOnce() + Send>);
}

/// Spawns a thread per job.
#[cfg(not(target_arch = "wasm32"))]
pub struct ThreadJobs;

#[cfg(not(target_arch = "wasm32"))]
impl Jobs for ThreadJobs {
    fn spawn(&self, name: &str, job: Box<dyn FnOnce() + Send>) {
        // A failed spawn drops the job: background work is never required for correctness.
        let _ = std::thread::Builder::new().name(name.to_owned()).spawn(job);
    }
}

/// Runs each job at once on the calling thread (tests, wasm).
pub struct InlineJobs;

impl Jobs for InlineJobs {
    fn spawn(&self, _name: &str, job: Box<dyn FnOnce() + Send>) {
        job();
    }
}
