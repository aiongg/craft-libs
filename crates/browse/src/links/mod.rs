//! `LinkMonitor` and `LinkChange`: link statuses kept current through `FileWatch`, recomputed in
//! jobs and reported once per change; `SearchRun`: a relink search run as a cancellable job.
//!
//! ```ignore
//! // App state: one monitor per document.
//! let mut monitor = LinkMonitor::new(Arc::new(SystemLinkFiles), host.jobs_shared().clone(), wake);
//! // Each pass: the document's links, then what changed since the last pass.
//! monitor.set(&doc.links(), doc.path());
//! for change in monitor.take_changes() {
//!     if change.status == LinkStatus::Modified { prefs.update_links.act(change.key) }   // Automatically, Ask, Manually
//! }
//! LinkList::new("links", &links, monitor.statuses()).show(ui);
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_docpath::links::{Link, LinkFiles, LinkSearch, LinkStatus, SearchEnd, SearchUpdate};
use craft_host::queue::{Cancel, JobQueue};
use craft_host::{Jobs, Wake};
use craft_platform::watch::FileWatch;

/// A link whose status changed, reported once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkChange<K> {
    /// The link.
    pub key: K,
    /// Its status before; `None` the first time it was known.
    pub before: Option<LinkStatus>,
    /// Its status now.
    pub status: LinkStatus,
}

/// Keeps the statuses of a document's links current without a timed poll (lead call 39).
///
/// - The resolved path of every link that is not embedded or refused is watched with a
///   `craft_platform::watch::FileWatch` (`ReadPolicy::Stamp`, which reads no bytes) whose wake
///   is the monitor's `wake`.
/// - A link is checked with `craft_docpath::links::status` in a job on a `JobQueue` (keyed by the
///   link, `Priority::BACKGROUND`, lanes from `QueueOptions::default`, name `"links"`) when it is
///   first [`set`](Self::set), when its stored path or identity changes, when the watch reports
///   its file, and for every link on [`window_focused`](Self::window_focused).
/// - A finished check whose status differs from the known one is a [`LinkChange`], reported once
///   by [`take_changes`](Self::take_changes); an unchanged one reports nothing.
///
/// Replaces VectorCraft's 2 s scan (`link_watch.rs`) and DesignCraft's text-link checks. On wasm32
/// nothing is watched (`FileWatch` has no paths there) and statuses come from the first check.
pub struct LinkMonitor<K> {
    files: Arc<dyn LinkFiles>,
    queue: JobQueue<K, LinkStatus>,
    watch: FileWatch<K>,
    links: HashMap<K, Link<K>>,
    document: Option<PathBuf>,
    statuses: HashMap<K, LinkStatus>,
    changes: Vec<LinkChange<K>>,
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> LinkMonitor<K> {
    /// A monitor reading files through `files`, running checks on `jobs`, calling `wake` when a
    /// check ends or the watch sees a change.
    pub fn new(files: Arc<dyn LinkFiles>, jobs: Arc<dyn Jobs>, wake: Wake) -> Self {
        // vectorcraft: crates/engine/src/link_watch.rs:66 (`start_link_scan`: stamps on a worker thread).
        todo!()
    }

    /// The links to keep current, with the document's path (`None` for an unsaved document):
    /// new and changed links are checked, links left out are forgotten (their statuses and
    /// watches dropped). Allocates nothing when the links are the ones set last.
    pub fn set(&mut self, links: &[Link<K>], document: Option<&Path>) {
        // vectorcraft: crates/engine/src/link_watch.rs:50 (`linked_files`, the files watched per document).
        todo!()
    }

    /// The window came back to the front: every link is checked again.
    pub fn window_focused(&mut self) {
        // vectorcraft: crates/engine/src/link_watch.rs:9 (looked at again when the window comes back to the front).
        todo!()
    }

    /// The known statuses (links not checked yet are absent), for `LinkList`.
    pub fn statuses(&self) -> &HashMap<K, LinkStatus> {
        // new: the list reads statuses by key.
        todo!()
    }

    /// The status of one link, once known.
    pub fn status(&self, key: &K) -> Option<LinkStatus> {
        // new: a lookup for the host's commands (Update enabled when Modified).
        todo!()
    }

    /// Drains the watch's changes into checks and the finished checks into statuses, and returns
    /// the changes since the last call, oldest first. Call it once per pass.
    pub fn take_changes(&mut self) -> Vec<LinkChange<K>> {
        // vectorcraft: crates/engine/src/link_watch.rs:89 (`poll_link_scan`; a change is acted on once).
        todo!()
    }

    /// The app just relinked, updated or embedded `key`: its status is checked again and the
    /// result is reported as a change only if it differs from what the app now expects
    /// (`expected`).
    pub fn known_now(&mut self, key: &K, expected: LinkStatus) {
        // vectorcraft: crates/engine/src/link_watch.rs:16 (undoing an update leaves the link modified until the file changes).
        todo!()
    }
}

/// A [`LinkSearch`] running as one cancellable job on a `JobQueue` (one lane, name
/// `"relink-search"`) over the app's `Jobs`. The job reports through a shared inbox and calls
/// `wake` after each `Progress` and each `Hit`; the dialog drains it once per pass. Dropping the
/// run cancels it.
pub struct SearchRun<K> {
    queue: JobQueue<u64, SearchEnd>,
    updates: Arc<Mutex<Vec<SearchUpdate<K>>>>,
    cancel: Cancel,
    end: Option<SearchEnd>,
}

impl<K: Clone + Send + 'static> SearchRun<K> {
    /// Starts `search` for `links` (the missing ones) over `files`.
    pub fn start(search: LinkSearch, links: Vec<Link<K>>, files: Arc<dyn LinkFiles>, jobs: Arc<dyn Jobs>, wake: Wake) -> Self {
        // filmcraft: crates/ui-egui/src/panels/media_dialogs.rs:146 (`link_media` ran `media.search` on the UI thread).
        todo!()
    }

    /// The updates since the last call, in order; empty once the run is cancelled.
    pub fn take_updates(&mut self) -> Vec<SearchUpdate<K>> {
        // new: the dialog fills its candidate list from these.
        todo!()
    }

    /// How the search ended; `None` while it runs.
    pub fn end(&mut self) -> Option<SearchEnd> {
        // new: the dialog swaps Cancel back to Search.
        todo!()
    }

    /// Stops the search: the job sees the cancel between folders and checks, and nothing more is
    /// reported.
    pub fn cancel(&mut self) {
        // new: the dialog's Cancel and Esc.
        todo!()
    }
}
