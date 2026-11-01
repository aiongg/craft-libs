//! `Directories`: the folder access the places tree needs, with `SystemDirectories` for the
//! desktop.

use std::io;
use std::path::{Path, PathBuf};

use craft_host::queue::Cancel;

/// A standard folder of the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardFolder {
    /// The home folder.
    Home,
    /// Desktop.
    Desktop,
    /// Documents.
    Documents,
    /// Downloads.
    Downloads,
    /// Pictures.
    Pictures,
    /// Movies (Videos on Windows and Linux).
    Movies,
    /// Music.
    Music,
}

/// A mounted volume or drive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    /// Its root folder.
    pub path: PathBuf,
    /// Its name as the system shows it.
    pub name: String,
    /// A network volume.
    pub network: bool,
    /// A removable drive (a card, a USB disk).
    pub removable: bool,
}

/// A subfolder in a listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    /// Its path.
    pub path: PathBuf,
    /// Its name.
    pub name: String,
}

/// The folder lookups a places tree needs (owner question 4: a small trait in this crate, with a
/// system implementation and a fake). Every call can block on a slow or network disk:
/// [`FolderCache`](super::FolderCache) calls them only in jobs.
pub trait Directories: Send + Sync {
    /// The standard folders that exist as folders, in the order Home, Desktop, Documents,
    /// Downloads, Pictures, Movies, Music; the host picks which it shows.
    fn standard(&self) -> Vec<(StandardFolder, PathBuf)>;

    /// The mounted volumes, in the system's order.
    fn volumes(&self) -> Vec<Volume>;

    /// The subfolders of `folder`: folders only (a link to a folder counts), hidden ones left
    /// out (a leading `.`; the hidden attribute on Windows; `UF_HIDDEN` on macOS), sorted by
    /// `craft_platform::open::natural_cmp` of their names. Checks `cancel` between entries and
    /// returns what it has when it is set.
    fn subfolders(&self, folder: &Path, cancel: &Cancel) -> io::Result<Vec<Folder>>;

    /// The hidden folders among `path` and its ancestors, innermost first (LightCraft's
    /// `unlisted_ancestors`); a tree never opens through one.
    fn unlisted_ancestors(&self, path: &Path) -> Vec<PathBuf>;
}

/// [`Directories`] over the file system.
///
/// - `standard`: `HOME` (`USERPROFILE` on Windows) and its `Desktop`, `Documents`, `Downloads`,
///   `Pictures`, `Movies` (`Videos` on Windows and Linux), `Music`, each when it is a folder.
/// - `volumes`: macOS the entries of `/Volumes`; Linux `/media/$USER/*`, `/run/media/$USER/*` and
///   `/mnt/*`; Windows the drive letters `A:` to `Z:` whose root exists. `network` and `removable`
///   stay false where `std` can't tell (a platform service may add them later).
/// - `subfolders`: `std::fs::read_dir`, as the trait says.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDirectories;

#[cfg(not(target_arch = "wasm32"))]
#[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
impl Directories for SystemDirectories {
    fn standard(&self) -> Vec<(StandardFolder, PathBuf)> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:24 (Pictures, Desktop, Downloads, Home under HOME or USERPROFILE).
        todo!()
    }

    fn volumes(&self) -> Vec<Volume> {
        // new: FilmCraft's Local Drives list comes from its engine services; std paths here.
        todo!()
    }

    fn subfolders(&self, folder: &Path, cancel: &Cancel) -> io::Result<Vec<Folder>> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:211 (`list_subfolders`: folders, not hidden, sorted).
        todo!()
    }

    fn unlisted_ancestors(&self, path: &Path) -> Vec<PathBuf> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:201 (`unlisted_ancestors`).
        todo!()
    }
}
