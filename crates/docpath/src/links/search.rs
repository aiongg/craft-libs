//! `LinkSearch` and `SearchHit`: a cancellable folder search for missing links, run as a job
//! (craft-browse's `SearchRun` runs it on a `JobQueue`).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::path::PathBuf;

use super::{Candidate, Link, LinkFiles, MatchCriteria};

/// What to search: the folder, the matching rule, and how many files to look at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkSearch {
    /// The folder the user chose (the dialog's "Search in").
    pub root: PathBuf,
    /// "Exact name matches only": only files whose whole name equals the link's, ignoring case.
    /// Off: files with the same name and any extension count too.
    pub exact_names: bool,
    /// The checks each found file goes through ([`check_candidate`](super::check_candidate)).
    pub criteria: MatchCriteria,
    /// Most files indexed: [`LinkSearch::LIMIT`] by default.
    pub limit: usize,
}

impl LinkSearch {
    /// The default file limit: 200 000.
    pub const LIMIT: usize = 200_000;

    /// A search of `root` with the default criteria, exact names off, and [`LinkSearch::LIMIT`].
    pub fn new(root: impl Into<PathBuf>) -> Self {
        // new: the dialog's defaults.
        todo!()
    }

    /// Runs the search for `links` (the missing ones): indexes `root` ([`index_folder`](super::index_folder)),
    /// reporting `Progress` every 500 files indexed, then for each link checks the files whose
    /// name matches (by `exact_names`) and reports each as a `Hit`, in link order, each link's
    /// hits ranked ([`rank`](super::rank)). `cancelled` is asked between folders and between
    /// checks; once it says yes nothing more is reported and the result is `Cancelled`. Blocks:
    /// run it in a job.
    pub fn run<K: Clone>(
        &self,
        links: &[Link<K>],
        files: &dyn LinkFiles,
        cancelled: &dyn Fn() -> bool,
        report: &mut dyn FnMut(SearchUpdate<K>),
    ) -> SearchEnd {
        // filmcraft: crates/engine/src/relink.rs:512 (`search`; it ran on the UI thread).
        todo!()
    }
}

/// One candidate found for one link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit<K> {
    /// The link's key.
    pub link: K,
    /// The checked file.
    pub candidate: Candidate,
}

/// What a running search reports, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchUpdate<K> {
    /// Files indexed so far (for "Searching… 120 files").
    Progress { files: usize },
    /// A candidate.
    Hit(SearchHit<K>),
}

/// How a search ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEnd {
    /// Every folder was seen.
    Done { files: usize },
    /// The limit stopped the walk.
    Truncated { files: usize },
    /// `cancelled` said yes.
    Cancelled,
}
