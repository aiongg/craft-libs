//! The presets of one kind, one entry per preset (`PresetFolder`, `PresetFolderError`): name
//! checks, the file format, and import and export by bytes. On desktop the entries are files in
//! `<config>/presets/<kind>/`; on every target, the web build included, they can live in the
//! app's settings sections instead, the store library features already save through
//! (`craft_host::SectionStore`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::io;
use std::marker::PhantomData;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::Arc;

use craft_host::SectionStore;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The folder beside `config.toml` that holds the preset folders, one per kind.
pub const PRESETS_DIR: &str = "presets";

/// The settings section that holds presets kept with [`PresetFolder::in_settings`]: one table per
/// kind, each entry a preset's name and its file text.
///
/// ```toml
/// [presets.curve]
/// "Strong Contrast" = '{"format":"curve","version":1,"points":[[0,0],[64,48],[255,255]]}'
/// ```
pub const PRESETS_SECTION: &str = "presets";

/// The greatest size of a preset file or settings entry: 1 MiB.
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
    /// place on disk (`PresetFolder::new(None, ..)`). Presets kept in settings never give it: a
    /// section write can't fail, and the settings store reports its own save errors.
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

/// The presets of one kind, one entry per preset. For SoundCraft's plugin presets, LightCraft's
/// curve presets and FilmCraft's export presets.
///
/// - **Where.** [`new`](Self::new) (desktop): one JSON file per preset in
///   `<config>/presets/<kind>/<name>.json`, next to the settings file as `Profiles`
///   (`<config>/profiles`) and PhotoCraft's brush store (`<config dir>/Presets`), so portable mode
///   carries them and users can copy the files. [`in_settings`](Self::in_settings) (every target):
///   one entry per preset in the [`PRESETS_SECTION`] settings section, through the same
///   `SectionStore` the app gives its `Host` (`SharedSettings` on desktop; on the web the app's
///   store over browser storage). The web build has no file system, so a web app keeps its
///   presets there. A `Host` whose sections are a `MemorySections` keeps them until the page
///   closes. On desktop, prefer files: settings put every preset inside the settings file.
/// - **File format.** `{"format": "<kind>", "version": 1, ...fields of T}`: `T` serializes as a
///   map whose fields sit beside the two envelope keys (SoundCraft's plugin preset shape).
///   Unknown fields are ignored on load. A settings entry is the same text as a string, so
///   [`export`](Self::export) gives the same bytes in both places and a preset moves between them
///   unchanged.
/// - **Names** compare without case. [`list`](Self::list) sorts them ignoring case;
///   [`save`](Self::save) replaces a preset whose name matches ignoring case;
///   [`import`](Self::import) takes the file's stem as the name and numbers a taken one ("X 2",
///   "X 3", …). The name rules are the same in settings, so every preset can be exported as
///   `<name>.json`.
/// - **Writes.** A file is written atomically (`craft_atomic::atomic_write`). A settings write
///   reads the section, changes this kind's table and writes the section back
///   (`SectionStore::write`); the settings store saves it with its next save. Reads skip a
///   corrupt or oversized file or entry and leave it where it is.
/// - **No folder** (`new(None, ..)`): an empty list, and every write is
///   `Io(ErrorKind::Unsupported)`.
/// - **Threads.** One save, load or delete the user asked for may run in the pass (one small
///   file or one section write). [`list`](Self::list) on a folder reads every file of the folder:
///   call it at start-up or from a job, not every pass. In settings it reads the section only.
pub struct PresetFolder<T> {
    pub(crate) place: PresetPlace,
    pub(crate) kind: String,
    pub(crate) value: PhantomData<fn() -> T>,
}

/// Where a [`PresetFolder`] keeps its entries.
pub(crate) enum PresetPlace {
    /// `<config>/presets/<kind>`; `None` without a config folder.
    #[cfg(not(target_arch = "wasm32"))]
    Folder(Option<PathBuf>),
    /// This kind's table in the [`PRESETS_SECTION`] section of `store`.
    Settings(Arc<dyn SectionStore>),
}

impl<T: Serialize + DeserializeOwned> PresetFolder<T> {
    /// The presets of `kind` ("plugin", "curve", "export") as files under `config_dir`, the
    /// settings store's folder; `None` keeps nothing. Desktop only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(config_dir: Option<&Path>, kind: &str) -> Self {
        // craft-libs: crates/settings/src/profiles.rs:75 (`Profiles::new(config_dir)`)
        todo!()
    }

    /// The presets of `kind` as entries of the [`PRESETS_SECTION`] settings section in
    /// `settings`: the store the app gave its `Host` (`HostBuilder::settings`). Every target; the
    /// web build's only place for presets.
    pub fn in_settings(settings: Arc<dyn SectionStore>, kind: &str) -> Self {
        // craft-libs: crates/settings/src/remember.rs:68 (`Remembered::store`: a value inside a library section through the Host's store)
        todo!()
    }

    /// The folder, `<config>/presets/<kind>`; `None` without a config folder and for presets
    /// kept in settings.
    pub fn dir(&self) -> Option<&Path> {
        // new: the stored path.
        todo!()
    }

    /// The kind, as written in each file's `format`.
    pub fn kind(&self) -> &str {
        // new: the stored kind.
        todo!()
    }

    /// The names of the readable presets, sorted ignoring case. A missing folder or table is an
    /// empty list; corrupt and oversized files or entries are left out (and left where they are).
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
    /// file or entry takes the new spelling). Creates the folder or the kind's table when needed.
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

    /// The file of preset `name` as bytes (a settings entry's text), for an export the user saves
    /// with `Files`.
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

    // `in_settings` over a `MemorySections`: save, list, load, rename and delete; the section's
    // `curve` table holds each name with its file text, and another kind's table is untouched.
    #[test]
    #[ignore = "skeleton"]
    fn presets_in_settings_live_in_the_kinds_table_of_the_section() {
        todo!()
    }

    // A preset saved in settings exports bytes that a folder imports and loads equal, and back.
    #[test]
    #[ignore = "skeleton"]
    fn a_preset_moves_between_settings_and_a_folder_unchanged() {
        todo!()
    }
}
