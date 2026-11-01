//! One file per preset in `<config>/presets/<kind>/`: atomic writes, name checks, and import and export by bytes (`PresetFolder`, `PresetFolderError`).
//!
//! Disk only, as `Profiles`: web apps keep presets in a settings section (`Remembered`).

#![cfg(not(target_arch = "wasm32"))]
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The folder beside `config.toml` that holds the preset folders, one per kind.
pub const PRESETS_DIR: &str = "presets";

/// The greatest size of a preset file: 1 MiB.
pub const MAX_FILE_BYTES: u64 = 1 << 20;

/// The longest preset name, in characters (`Profiles`' limit).
pub const MAX_NAME_CHARS: usize = 100;

/// The envelope's version written by this library.
pub const FORMAT_VERSION: u32 = 1;

/// What can go wrong with a preset file.
#[derive(Debug)]
#[non_exhaustive]
pub enum PresetFolderError {
    /// The name is empty, longer than [`MAX_NAME_CHARS`], or can't be a file name (a path
    /// separator, a reserved character, a leading dot).
    BadName,
    /// No preset has that name.
    NotFound,
    /// The bytes are a preset of another kind: the envelope's `format` is `found`.
    ForeignFormat { found: String },
    /// The file is larger than [`MAX_FILE_BYTES`].
    TooLarge,
    /// The bytes are not a preset of this kind (unparsable JSON, a missing field, a newer
    /// `version`).
    Unreadable(String),
    /// The folder can't be read or written; `io::ErrorKind::Unsupported` when the folder has no
    /// place on disk (`PresetFolder::new(None, ..)`).
    Io(io::Error),
}

impl std::fmt::Display for PresetFolderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadName => f.write_str("the preset name is not valid"),
            Self::NotFound => f.write_str("no preset has that name"),
            Self::ForeignFormat { found } => write!(f, "the file holds {found} presets"),
            Self::TooLarge => f.write_str("the preset file is too large"),
            Self::Unreadable(why) => write!(f, "the preset can't be read: {why}"),
            Self::Io(e) => write!(f, "the preset could not be read or written: {e}"),
        }
    }
}

impl std::error::Error for PresetFolderError {}

impl From<io::Error> for PresetFolderError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::NotFound { Self::NotFound } else { Self::Io(e) }
    }
}

/// The presets of one kind, one JSON file per preset: `<config>/presets/<kind>/<name>.json`.
/// For SoundCraft's plugin presets, LightCraft's curve presets and FilmCraft's export presets;
/// next to the settings file as `Profiles` (`<config>/profiles`) and PhotoCraft's brush store
/// (`<config dir>/Presets`), so portable mode carries them.
///
/// - **File format.** `{"format": "<kind>", "version": 1, ...fields of T}`: `T` serializes as a
///   map whose fields sit beside the two envelope keys (SoundCraft's plugin preset shape).
///   Unknown fields are ignored on load.
/// - **Names** compare without case. [`list`](Self::list) sorts them ignoring case;
///   [`save`](Self::save) replaces a preset whose name matches ignoring case;
///   [`import`](Self::import) takes the file's stem as the name and numbers a taken one ("X 2",
///   "X 3", …).
/// - **Writes** are atomic (`craft_atomic::atomic_write`). Reads skip a corrupt or oversized
///   file and leave it on disk.
/// - **No folder** (`new(None, ..)`): an empty list, and every write is
///   `Io(ErrorKind::Unsupported)`.
/// - **Threads.** One save, load or delete the user asked for may run in the pass (one small
///   file). [`list`](Self::list) reads every file of the folder: call it at start-up or from a
///   job, not every pass.
pub struct PresetFolder<T> {
    pub(crate) dir: Option<PathBuf>,
    pub(crate) kind: String,
    pub(crate) value: PhantomData<fn() -> T>,
}

impl<T: Serialize + DeserializeOwned> PresetFolder<T> {
    /// The presets of `kind` ("plugin", "curve", "export") under `config_dir`, the settings
    /// store's folder; `None` keeps nothing.
    pub fn new(config_dir: Option<&Path>, kind: &str) -> Self {
        // craft-libs: crates/settings/src/profiles.rs:75 (`Profiles::new(config_dir)`)
        todo!()
    }

    /// The folder, `<config>/presets/<kind>`; `None` without a config folder.
    pub fn dir(&self) -> Option<&Path> {
        // new: the stored path.
        todo!()
    }

    /// The kind, as written in each file's `format`.
    pub fn kind(&self) -> &str {
        // new: the stored kind.
        todo!()
    }

    /// The names of the readable presets, sorted ignoring case. A missing folder is an empty
    /// list; corrupt and oversized files are left out (and left on disk).
    pub fn list(&self) -> Vec<String> {
        // craft-libs: crates/settings/src/profiles.rs:80 (`list`, sorted ignoring case); soundcraft: crates/ui-egui/src/panels/plugins.rs:162 (`preset_dir`, `*.json`)
        todo!()
    }

    /// Reads preset `name` (matched ignoring case).
    pub fn load(&self, name: &str) -> Result<T, PresetFolderError> {
        // craft-libs: crates/settings/src/profiles.rs:106
        todo!()
    }

    /// Writes `value` as preset `name`, replacing a preset whose name matches ignoring case (the
    /// file takes the new spelling). Creates the folder when needed.
    pub fn save(&self, name: &str, value: &T) -> Result<(), PresetFolderError> {
        // craft-libs: crates/settings/src/profiles.rs:87-104 (`save` replaces a name that matches ignoring case)
        todo!()
    }

    /// Deletes preset `name` (matched ignoring case).
    pub fn delete(&self, name: &str) -> Result<(), PresetFolderError> {
        // craft-libs: crates/settings/src/profiles.rs:112
        todo!()
    }

    /// Renames preset `from` to `to`; `BadName` when `to` is not valid or another preset has it
    /// ignoring case (a change of case alone is allowed).
    pub fn rename(&self, from: &str, to: &str) -> Result<(), PresetFolderError> {
        // new: the named row's Rename….
        todo!()
    }

    /// The file of preset `name` as bytes, for an export the user saves with `Files`.
    pub fn export(&self, name: &str) -> Result<Vec<u8>, PresetFolderError> {
        // craft-libs: crates/settings/src/profiles.rs:119 (`export`); the bytes go through the host's `Files`.
        todo!()
    }

    /// Imports the bytes of a file the user picked: checks the size and the envelope's `format`,
    /// takes `file_name`'s stem as the name, numbers a taken one, saves it, and returns the name.
    pub fn import(&self, file_name: &str, bytes: &[u8]) -> Result<String, PresetFolderError> {
        // craft-libs: crates/settings/src/profiles.rs:127 (`import`: stem as name, " 2", " 3" when taken)
        todo!()
    }

    /// `value` in the file format, for a copy or a host-side export.
    pub fn to_bytes(&self, value: &T) -> Result<Vec<u8>, PresetFolderError> {
        // new: serde_json with the envelope.
        todo!()
    }

    /// A value of this kind from the file format.
    pub fn from_bytes(&self, bytes: &[u8]) -> Result<T, PresetFolderError> {
        // new: the envelope check, then serde_json.
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // In a temp dir: save, list (sorted ignoring case), load, rename and delete.
    #[test]
    #[ignore = "skeleton"]
    fn preset_folder_saves_lists_loads_deletes_and_renames() {
        todo!()
    }

    // Saving "x" after "X" leaves one file, named "x".
    #[test]
    #[ignore = "skeleton"]
    fn preset_folder_replaces_a_name_that_differs_only_in_case() {
        todo!()
    }

    // Importing "X.json" twice gives "X" and "X 2"; bytes with another `format` are `ForeignFormat`.
    #[test]
    #[ignore = "skeleton"]
    fn preset_folder_import_numbers_a_taken_name_and_refuses_a_foreign_format() {
        todo!()
    }

    // A file of garbage and one over 1 MiB are left out of `list` and stay on disk.
    #[test]
    #[ignore = "skeleton"]
    fn preset_folder_skips_a_corrupt_or_oversized_file() {
        todo!()
    }

    // `new(None, ..)`: `list` is empty and `save` is `Io` with `ErrorKind::Unsupported`.
    #[test]
    #[ignore = "skeleton"]
    fn preset_folder_without_a_folder_keeps_nothing() {
        todo!()
    }
}
