//! [`SettingsFiles`]: the file access a [`SettingsStore`](crate::SettingsStore) goes through, so an
//! app can route its settings elsewhere (PhotoCraft's `Services` file closures, browser storage).

use std::io;
use std::path::{Path, PathBuf};

/// How the store reads and writes its files. [`DiskFiles`] is the file system, the default; an
/// app plugs in its own through [`OpenOptions::files`](crate::OpenOptions::files).
///
/// Every method is called on the thread that ticks the store. The store keeps its rules (backup
/// before overwrite, a damaged file moved aside, owner-only permissions) and asks this trait only
/// for the operations.
pub trait SettingsFiles: Send + Sync {
    /// The file's text; `Ok(None)` when it doesn't exist.
    fn read(&self, path: &Path) -> io::Result<Option<String>>;
    /// Replaces the file atomically; a new file is owner-only.
    fn write(&self, path: &Path, text: &str) -> io::Result<()>;
    /// Renames a damaged file aside and returns the new path
    /// ([`move_aside`](crate::recovery::move_aside)'s naming).
    fn move_aside(&self, path: &Path) -> io::Result<PathBuf>;
    /// Writes the backup beside `path` ([`write_backup`](crate::recovery::write_backup)'s rules).
    fn write_backup(&self, path: &Path, text: &str) -> io::Result<()>;
    /// Whether `dir` accepts a new file.
    fn check_writable(&self, dir: &Path) -> io::Result<()>;
}

/// The file system, through `craft-atomic` and [`recovery`](crate::recovery): what the store does
/// today, unchanged.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskFiles;

#[cfg(not(target_arch = "wasm32"))]
#[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
impl SettingsFiles for DiskFiles {
    // new: the `std::fs::read_to_string` calls of store.rs (`open_with`, `check_disk`), NotFound as `None`
    fn read(&self, path: &Path) -> io::Result<Option<String>> {
        todo!()
    }

    // new: store.rs `save`'s `atomic_write_with` with `NewFilePermissions::Private` and `restrict_permissions`
    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        todo!()
    }

    fn move_aside(&self, path: &Path) -> io::Result<PathBuf> {
        crate::recovery::move_aside(path)
    }

    fn write_backup(&self, path: &Path, text: &str) -> io::Result<()> {
        crate::recovery::write_backup(path, text)
    }

    fn check_writable(&self, dir: &Path) -> io::Result<()> {
        crate::recovery::check_writable(dir)
    }
}
