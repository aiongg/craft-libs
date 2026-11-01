//! `index_folder`: the files of a folder tree by lower-cased name, for the relink search.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::LinkFiles;

/// The files found under a folder, by file name in Unicode lower case, each with every path that
/// has that name, in the order they were found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FolderIndex {
    /// Paths by lower-cased file name.
    pub by_name: BTreeMap<String, Vec<PathBuf>>,
    /// Files indexed.
    pub files: usize,
    /// The walk stopped at the limit before it had seen every folder.
    pub truncated: bool,
}

/// Indexes the regular files under `root`, breadth first, each folder's entries in name order,
/// skipping names that start with `.`, never following a link to a folder (a linked file still
/// counts), and stopping after `limit` files (`truncated`). `cancelled` is asked before each
/// folder; when it says yes the walk ends with what it has. Unreadable folders are skipped.
pub fn index_folder(root: &Path, limit: usize, files: &dyn LinkFiles, cancelled: &dyn Fn() -> bool) -> FolderIndex {
    // filmcraft: crates/engine/src/relink.rs:478 (`index_folder`: breadth first, dot files and folder links skipped).
    todo!()
}
