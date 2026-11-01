//! `check_candidate` and `rank`: whether a file may replace a missing link, and the order the
//! relink dialog lists candidates in.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Link, LinkFiles, LinkIdentity};

/// Which properties a candidate must share with the missing file (the relink dialog's "Match
/// file properties" checks the library knows). An app's own properties (duration, timecode, frame
/// size) are its `RelinkCriterion`s, checked by its `CandidateChecker` after [`check_candidate`].
/// Serde with defaults for missing fields, so an app keeps the user's last choices in its
/// settings (FilmCraft stores its `MatchOptions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchCriteria {
    /// The file name without its extension, compared ignoring case. Default on.
    pub file_name: bool,
    /// The extension, compared ignoring case. Default on.
    pub extension: bool,
    /// The identity recorded at link time (FilmCraft's "Clip ID"): size and fingerprint. Links
    /// that recorded none skip it. Default on.
    pub same_file: bool,
}

impl Default for MatchCriteria {
    fn default() -> Self {
        // filmcraft: crates/engine/src/relink.rs:161 (`MatchOptions::default`: name, extension, clip id on).
        todo!()
    }
}

/// Why a candidate does not pass. Typed, so the dialog words each one (`RelinkLabels`) and an
/// app's checker adds its own words as [`Other`](CandidateProblem::Other).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CandidateProblem {
    /// The name without extension differs: `found` against `wanted`.
    NameDiffers { found: String, wanted: String },
    /// The extension differs.
    ExtensionDiffers { found: String, wanted: String },
    /// Not the same file: the size differs (`found` and `wanted` in bytes), or, with `sizes`
    /// equal, the fingerprint differs.
    DifferentFile { found: u64, wanted: u64 },
    /// The file couldn't be read; the system's message.
    Unreadable(String),
    /// A problem the app's checker found, already in the user's language.
    Other(String),
}

/// One file checked as the new file of a missing link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The file.
    pub path: PathBuf,
    /// Passes every enabled check.
    pub ok: bool,
    /// Whether its identity matches the one recorded at link time; `None` when the link recorded
    /// none or the file couldn't be read.
    pub same_file: Option<bool>,
    /// Why it doesn't pass, in check order; empty when `ok`.
    pub problems: Vec<CandidateProblem>,
    /// The candidate's identity, read during the check, for the app to store when it relinks.
    pub identity: Option<LinkIdentity>,
    /// Ranking, higher first: same file 8, same name 4, same extension 2, passes 1 (FilmCraft's
    /// weights).
    pub score: u32,
}

/// Checks `path` as the new file of `link` by `criteria`: the name and extension against the
/// last component of `link.stored` (both separators count), ignoring ASCII case, each adding its
/// problem when its criterion is on; then [`LinkFiles::identity`] (an error adds `Unreadable` and
/// ends the check); then, when the link recorded an identity, `same_file` = size and fingerprint
/// equal, adding `DifferentFile` when `criteria.same_file` is on and they differ. `ok` is no
/// problems; `score` as [`Candidate::score`] says. Reads the file: call it in a job.
pub fn check_candidate<K>(link: &Link<K>, path: &Path, criteria: &MatchCriteria, files: &dyn LinkFiles) -> Candidate {
    // filmcraft: crates/engine/src/relink.rs:202 (`check_candidate`; the media checks stay FilmCraft's).
    todo!()
}

/// Sorts `candidates` best first: by `score` descending, then by path with
/// `natural`-order file names (ties keep their search order). A candidate that passes and is
/// the same file is always first.
pub fn rank(candidates: &mut [Candidate]) {
    // filmcraft: crates/engine/src/relink.rs:281 (`score`: identity first, then name, extension).
    todo!()
}
