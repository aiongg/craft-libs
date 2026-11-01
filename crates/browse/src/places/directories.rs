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
impl Directories for SystemDirectories {
    fn standard(&self) -> Vec<(StandardFolder, PathBuf)> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:24 (Pictures, Desktop, Downloads, Home under HOME or USERPROFILE).
        let Some(home) = home() else { return Vec::new() };
        let movies = if cfg!(target_os = "macos") { "Movies" } else { "Videos" };
        let named = [
            (StandardFolder::Home, ""),
            (StandardFolder::Desktop, "Desktop"),
            (StandardFolder::Documents, "Documents"),
            (StandardFolder::Downloads, "Downloads"),
            (StandardFolder::Pictures, "Pictures"),
            (StandardFolder::Movies, movies),
            (StandardFolder::Music, "Music"),
        ];
        named
            .into_iter()
            .map(|(kind, sub)| (kind, if sub.is_empty() { home.clone() } else { home.join(sub) }))
            .filter(|(_, path)| path.is_dir())
            .collect()
    }

    fn volumes(&self) -> Vec<Volume> {
        // new: FilmCraft's Local Drives list comes from its engine services; std paths here.
        let mut parents: Vec<PathBuf> = Vec::new();
        if cfg!(target_os = "macos") {
            parents.push(PathBuf::from("/Volumes"));
        } else if cfg!(unix) {
            if let Some(user) = std::env::var_os("USER").filter(|u| !u.is_empty()) {
                parents.push(Path::new("/media").join(&user));
                parents.push(Path::new("/run/media").join(&user));
            }
            parents.push(PathBuf::from("/mnt"));
        }
        let mut out = Vec::new();
        for parent in parents {
            let Ok(entries) = std::fs::read_dir(&parent) else { continue };
            let mut found: Vec<Volume> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| Volume { path: e.path(), name: e.file_name().to_string_lossy().into_owned(), network: false, removable: false })
                .collect();
            found.sort_by(|a, b| craft_platform::open::natural_cmp(&a.name, &b.name));
            out.extend(found);
        }
        if cfg!(windows) {
            for letter in 'A'..='Z' {
                let root = PathBuf::from(format!("{letter}:\\"));
                if root.is_dir() {
                    out.push(Volume { name: format!("{letter}:"), path: root, network: false, removable: false });
                }
            }
        }
        out
    }

    fn subfolders(&self, folder: &Path, cancel: &Cancel) -> io::Result<Vec<Folder>> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:211 (`list_subfolders`: folders, not hidden, sorted).
        let mut out = Vec::new();
        for entry in std::fs::read_dir(folder)? {
            if cancel.is_cancelled() {
                break;
            }
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let is_folder = match entry.file_type() {
                Ok(t) if t.is_dir() => true,
                Ok(t) if t.is_symlink() => path.is_dir(),
                _ => false,
            };
            if is_folder && !is_hidden(&path, || entry.metadata()) {
                out.push(Folder { name: entry.file_name().to_string_lossy().into_owned(), path });
            }
        }
        out.sort_by(|a, b| craft_platform::open::natural_cmp(&a.name, &b.name));
        Ok(out)
    }

    fn unlisted_ancestors(&self, path: &Path) -> Vec<PathBuf> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:201 (`unlisted_ancestors`).
        path.ancestors().filter(|p| is_hidden(p, || std::fs::symlink_metadata(p))).map(Path::to_path_buf).collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).or_else(|| std::env::var_os("USERPROFILE").filter(|h| !h.is_empty())).map(PathBuf::from)
}

/// Whether the path has a dot-prefixed name or a Windows or macOS hidden attribute.
// lightcraft: crates/engine/src/walk.rs:56 (`hidden_with_metadata`).
#[cfg(not(target_arch = "wasm32"))]
fn is_hidden(path: &Path, metadata: impl FnOnce() -> io::Result<std::fs::Metadata>) -> bool {
    if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        metadata().is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::darwin::fs::MetadataExt;
        const UF_HIDDEN: u32 = 0x0000_8000;
        metadata().is_ok_and(|m| m.st_flags() & UF_HIDDEN != 0)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = metadata;
        false
    }
}
