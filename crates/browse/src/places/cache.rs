//! `FolderCache`: roots and subfolder listings made in jobs, kept until they go stale.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use craft_host::queue::JobQueue;
use craft_host::{Jobs, Wake};

use super::{Directories, Folder, StandardFolder, Volume};

/// The standard folders and volumes, from one job.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roots {
    /// [`Directories::standard`].
    pub standard: Vec<(StandardFolder, PathBuf)>,
    /// [`Directories::volumes`].
    pub volumes: Vec<Volume>,
}

/// One folder's subfolders as the cache has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Listing<'a> {
    /// Not listed yet; a job is queued.
    Loading,
    /// The subfolders. While a new listing of a stale folder runs, the old one stays here.
    Ready(&'a [Folder]),
    /// The folder couldn't be listed; the system's message.
    Failed(&'a str),
}

/// What a listing job returns.
enum Found {
    Roots(Roots),
    Subfolders(Result<Vec<Folder>, String>),
    Unlisted(Vec<PathBuf>),
}

/// Which listing a job makes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Job {
    Roots,
    Subfolders(PathBuf),
    Unlisted(PathBuf),
}

/// Folder listings for a places tree, made in jobs on a [`JobQueue`] (lanes from
/// `QueueOptions::default`, name `"folders"`) over the app's `Jobs`; each finished job calls
/// `wake`. Nothing is listed in the pass: an unknown folder answers `Loading` and queues a job
/// (`Priority::VISIBLE` for asks, `NEAR` for refreshes), and the result shows in the first pass
/// after it lands. Lives in the app's state, one per tree.
///
/// Refreshing (risk 8): `craft_platform::watch::FileWatch` watches regular files only
/// (`FileStamp::at` filters `is_file`), so folders are not watched. Instead every listing goes
/// stale on [`refresh`](Self::refresh), which `PlacesTree` calls when the window regains focus,
/// and on [`refresh_folder`](Self::refresh_folder) (a row opened, the app made or moved a folder).
/// A stale listing keeps showing and is listed again the next time it is asked for. There is no
/// timed refresh.
pub struct FolderCache {
    directories: Arc<dyn Directories>,
    queue: JobQueue<Job, Found>,
    roots: Option<Roots>,
    listings: HashMap<PathBuf, (Result<Vec<Folder>, String>, bool)>,
    unlisted: HashMap<PathBuf, Vec<PathBuf>>,
    stale_roots: bool,
}

impl FolderCache {
    /// A cache over `directories`, running its jobs on `jobs` and calling `wake` when one ends.
    pub fn new(directories: Arc<dyn Directories>, jobs: Arc<dyn Jobs>, wake: Wake) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:241 (`fs_cached`: listings on a worker, read in the pass).
        todo!()
    }

    /// The roots, once listed; queues the job the first time and after a refresh.
    pub fn roots(&mut self) -> Option<&Roots> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:28 (the is-dir checks run off the UI thread).
        todo!()
    }

    /// `folder`'s subfolders (drains finished jobs first).
    pub fn subfolders(&mut self, folder: &Path) -> Listing<'_> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:207 (`subfolders`, empty until the first listing).
        todo!()
    }

    /// The hidden folders on the way to `path` ([`Directories::unlisted_ancestors`]); `None` until
    /// listed.
    pub fn unlisted(&mut self, path: &Path) -> Option<&[PathBuf]> {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:38 (`unlisted` through `fs_cached`).
        todo!()
    }

    /// Marks every listing and the roots stale (the window regained focus).
    pub fn refresh(&mut self) {
        // new: replaces LightCraft's 2 s refresh (lead call 26).
        todo!()
    }

    /// Marks one folder's listing stale.
    pub fn refresh_folder(&mut self, folder: &Path) {
        // new: the tree calls it when a row opens.
        todo!()
    }

    /// Drops the listings of folders not asked for since the last call, and cancels their jobs
    /// (`JobQueue::reprioritize`). `PlacesTree` calls it at the end of its pass.
    pub fn end_pass(&mut self) {
        // new: a hidden tree lists nothing.
        todo!()
    }
}
