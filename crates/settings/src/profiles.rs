#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`Profiles`]: named copies of an app's settings the user can switch to, export and import.
//!
//! CadCraft keeps one settings profile and lists "named profiles with import and export" as not
//! done (its `docs/gaps.md`); this is that feature, for every app on [`SettingsStore`](crate::SettingsStore).

use std::io;
use std::path::{Path, PathBuf};

use crate::Settings;

/// The folder beside `config.toml` that holds the profiles.
pub const PROFILES_DIR: &str = "profiles";

/// What can go wrong with a profile.
#[derive(Debug)]
pub enum ProfileError {
    /// The name is empty or can't be a file name (a path separator, a reserved character).
    BadName,
    /// No profile has that name.
    NotFound,
    /// The file is not this app's settings (unparsable, or a newer schema).
    Unreadable(String),
    Io(io::Error),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadName => f.write_str("the profile name is not valid"),
            Self::NotFound => f.write_str("no profile has that name"),
            Self::Unreadable(why) => write!(f, "the profile can't be read: {why}"),
            Self::Io(e) => write!(f, "the profile could not be written: {e}"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Named settings profiles in `<config>/profiles/<name>.toml`, each a full settings file with
/// `schema_version` (migrated on load like `config.toml`).
///
/// - [`save`](Self::save) stores the current settings under a name, replacing a profile of that
///   name.
/// - [`load`](Self::load) reads one; the app applies it with `store.set(value)`.
/// - [`export`](Self::export) and [`import`](Self::import) copy a profile to and from a file the
///   user picked; import takes the file's stem as the name, adding " 2", " 3", … when taken.
/// - [`to_text`](Self::to_text) is the profile as TOML for "Copy as Text" (CadCraft's Copy as JSON).
///
/// Names compare without case; [`list`](Self::list) sorts them ignoring case.
pub struct Profiles<T: Settings> {
    dir: Option<PathBuf>,
    _settings: std::marker::PhantomData<T>,
}

impl<T: Settings> Profiles<T> {
    /// The profiles of the store whose folder is `config_dir`; `None` (an in-memory store) keeps
    /// no profiles.
    pub fn new(config_dir: Option<&Path>) -> Self {
        Self { dir: config_dir.map(|d| d.join(PROFILES_DIR)), _settings: std::marker::PhantomData }
    }

    /// The profile names, sorted.
    pub fn list(&self) -> Vec<String> {
        todo!()
    }

    pub fn save(&self, name: &str, value: &T) -> Result<(), ProfileError> {
        todo!()
    }

    // cadcraft: crates/ui-egui/src/options.rs:350 load_profile (size cap, unknown entries skipped)
    pub fn load(&self, name: &str) -> Result<T, ProfileError> {
        todo!()
    }

    pub fn delete(&self, name: &str) -> Result<(), ProfileError> {
        todo!()
    }

    pub fn export(&self, name: &str, to: &Path) -> Result<(), ProfileError> {
        todo!()
    }

    /// Imports a settings file as a profile and returns its name.
    pub fn import(&self, from: &Path) -> Result<String, ProfileError> {
        todo!()
    }

    // cadcraft: crates/ui-egui/src/options.rs:333 profile_json
    pub fn to_text(value: &T) -> Result<String, ProfileError> {
        todo!()
    }
}
