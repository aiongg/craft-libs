//! `SystemLinkFiles`: [`LinkFiles`] over `std::fs` (desktop only).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::io;
use std::path::Path;

use super::{FileInfo, FolderEntry, LinkFiles, LinkIdentity};

/// [`LinkFiles`] over the file system.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemLinkFiles;

impl LinkFiles for SystemLinkFiles {
    /// `std::fs::metadata` (follows links), regular files only; the size and `modified()`.
    fn stat(&self, path: &Path) -> Option<FileInfo> {
        // designcraft: crates/engine/src/cmd/links.rs:143 (`link_missing`: devices and FIFOs count as missing).
        todo!()
    }

    /// Opens the file, takes its metadata from the open handle (so the size matches what is
    /// read) and calls [`identity_of`](super::identity_of).
    fn identity(&self, path: &Path) -> io::Result<LinkIdentity> {
        // filmcraft: crates/engine/src/relink.rs:59 (`identity_of` over the services' `read_range`).
        todo!()
    }

    /// `std::fs::read_dir`; `is_dir` from the entry's own file type (a link to a folder is not a
    /// folder), `is_link` from `is_symlink`. Entries that fail to read are skipped.
    fn list(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        // filmcraft: crates/engine/src/relink.rs:483 (`read_dir`, `file_type`).
        todo!()
    }
}
