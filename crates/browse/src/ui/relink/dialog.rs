//! `RelinkDialog`: the Link Media dialog for a document's missing links.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::path::PathBuf;
use std::sync::Arc;

use craft_docpath::links::{Candidate, Link, LinkFiles, MatchCriteria};
use craft_host::Jobs;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::Context;

use crate::links::SearchRun;

/// One missing link as the dialog's table shows it: Clip Name, File Name, File Path, Status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelinkRow<K> {
    /// The link.
    pub link: Link<K>,
    /// The first column: the app's name for what uses the file (a clip, a graphic).
    pub owner: String,
    /// The Status column ("Media missing"), translated.
    pub status: String,
}

/// An app's own match property (FilmCraft's duration, media start, frame size and rate, align
/// timecode): a check row under the library's name, extension and same-file checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelinkCriterion {
    /// The id the app's checker reads.
    pub id: &'static str,
    /// The check's text, translated.
    pub label: String,
    /// Whether it is on.
    pub on: bool,
}

/// The app's extra checks of a candidate, after `craft_docpath::links::check_candidate`: it reads
/// the file (in a job) and adds `CandidateProblem::Other` entries and score. `Send + Sync`, run
/// in the search's and Locate's jobs.
pub trait CandidateChecker<K>: Send + Sync {
    /// Checks `candidate` (already through `check_candidate`) as the new file of `link` with the
    /// app's `criteria` on; returns it with its problems, `ok` and `score` updated.
    fn check(&self, link: &Link<K>, candidate: Candidate, criteria: &[RelinkCriterion]) -> Candidate;
}

/// The dialog's start values.
#[derive(Clone)]
pub struct RelinkOptions {
    /// The library's match checks (default all on).
    pub criteria: MatchCriteria,
    /// The app's checks, in order.
    pub app_criteria: Vec<RelinkCriterion>,
    /// "Relink others automatically" (default on).
    pub relink_others: bool,
    /// "Exact name matches only" (default off).
    pub exact_names: bool,
    /// The search folder to start with (the document's folder, the last one used).
    pub folder: Option<PathBuf>,
    /// The document's path, for `resolve_absolute`.
    pub document: Option<PathBuf>,
    /// Jobs for the search and checks; `None` uses `Host::jobs_shared` (stories pass a
    /// `ManualJobs`).
    pub jobs: Option<Arc<dyn Jobs>>,
}

impl Default for RelinkOptions {
    fn default() -> Self {
        // filmcraft: crates/engine/src/relink.rs:161 (`MatchOptions::default`: relink others on).
        todo!()
    }
}

/// What Done returns: every link the dialog made and every row left offline, applied by the
/// host as one undo step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelinkOutcome<K> {
    /// The links relinked, each with its checked candidate (path and identity to store).
    pub linked: Vec<(K, Candidate)>,
    /// The links the user left offline.
    pub offline: Vec<K>,
}

craft_ui::words! {
    /// The dialog's words, English by default.
    #[non_exhaustive]
    pub struct RelinkLabels {
        title: "browse.relinkTitle" = "Link Media";
        message: "browse.relinkMessage" = "Find the missing files, or leave them offline.";
        owner: "browse.relinkOwner" = "Clip Name";
        file_name: "browse.relinkFileName" = "File Name";
        file_path: "browse.relinkFilePath" = "File Path";
        status: "browse.relinkStatus" = "Status";
        match_heading: "browse.relinkMatch" = "Match file properties";
        match_name: "browse.relinkMatchName" = "File Name";
        match_extension: "browse.relinkMatchExtension" = "File Extension";
        match_same_file: "browse.relinkMatchSameFile" = "Same File";
        relink_others: "browse.relinkOthers" = "Relink others automatically";
        search_in: "browse.relinkSearchIn" = "Search in:";
        browse: "browse.relinkBrowse" = "Browse…";
        exact_names: "browse.relinkExactNames" = "Exact name matches only";
        search: "browse.relinkSearch" = "Search";
        cancel_search: "browse.relinkCancelSearch" = "Stop";
        /// `{n}` files indexed so far.
        searching: "browse.relinkSearching" = "Searching… {n} files";
        same_file: "browse.relinkSameFile" = "same file";
        matches: "browse.relinkMatches" = "matches";
        different_file: "browse.relinkDifferentFile" = "different file";
        not_found: "browse.relinkNotFound" = "No candidates found";
        name_differs: "browse.relinkNameDiffers" = "File name differs ({found} vs {wanted})";
        extension_differs: "browse.relinkExtensionDiffers" = "Extension differs (.{found} vs .{wanted})";
        size_differs: "browse.relinkSizeDiffers" = "Not the same file: {found} bytes, the original had {wanted}";
        fingerprint_differs: "browse.relinkFingerprintDiffers" = "Not the same file: the contents differ";
        unreadable: "browse.relinkUnreadable" = "Can't read the file: {reason}";
        locate: "browse.relinkLocate" = "Locate…";
        link: "browse.relinkLink" = "Link";
        link_anyway: "browse.relinkLinkAnyway" = "Link Anyway";
        offline: "browse.relinkOffline" = "Leave Offline";
        offline_all: "browse.relinkOfflineAll" = "Leave All Offline";
        done: "browse.relinkDone" = "Done";
    }
}

/// The Link Media dialog (N5.05) for a document's missing links; a `StackDialog` returning
/// [`RelinkOutcome`] (Done) or `Cancel` (Cancel, Esc).
///
/// - **Layout.** A `Dialog` `min(window − 40, link.dialog_max_width 760)` wide: the message; the
///   missing `Table` (owner, file name, path, status; height `link.table_share` 30 % of the window
///   clamped `link.table_min`–`link.table_max`, 90–320); the match checks (library, then the
///   app's [`RelinkCriterion`]s) and Relink others; "Search in:" a `TextField` with Browse… (a
///   folder pick through `Files::pick`, answers read with `take_answer`), Exact name matches
///   only, Search; the candidates `ListBox` (`link.candidates_height` 108) beside a preview box
///   (`link.preview` 192 × 108 on `surface.sunken`, the candidate's thumbnail through
///   `Thumbnails` over a `FileThumbSource` the host gives with `.preview(..)`); the buttons Leave
///   All Offline, Leave Offline, Cancel, Locate…, Link (default).
/// - **Flow.** The table selects the first row; picking a row clears the candidates. Search
///   starts a [`SearchRun`] (`LinkSearch` with the folder, exact names and criteria) over the
///   `LinkFiles`, on `RelinkOptions::jobs`; progress shows [`RelinkLabels::searching`] in a polite
///   live region, Stop cancels; hits for the selected row fill the list as they arrive, each
///   through the app's checker in the job; ranked (`rank`), a passing same file first. Each
///   candidate shows its mark (same file `status.good.fg`, matches `status.good.fg`, different
///   file `status.bad.fg`, glyph and text) with its problems as the tooltip. Locate… picks one
///   file and adds it, checked in a job, as the selected candidate. Link links the selected
///   candidate and moves to the next missing row; when the candidate fails a check the button
///   reads Link Anyway. With Relink others, a link derives a `Remap` (`derive_remap`) and every
///   other missing row it fits (`Remap::apply`, `resolve_absolute`, then the checks in a job) is
///   linked; rows that fail stay missing. Leave Offline and Leave All Offline mark rows. Done
///   returns the outcome; Cancel and Esc return nothing; Esc while a pick is open closes the pick
///   only. A network or device link is refused before any read.
/// - **Never blocks.** The search and every check are jobs; results arrive through `Wake`.
/// - **AccessKit.** `Dialog`; the table a `Table`; candidates a `ListBox`; the progress a live
///   region.
/// - **RTL.** Table columns and the candidate and preview split mirror.
pub struct RelinkDialog<K> {
    id: DialogId,
    rows: Vec<RelinkRow<K>>,
    files: Arc<dyn LinkFiles>,
    options: RelinkOptions,
    checker: Option<Arc<dyn CandidateChecker<K>>>,
    search: Option<SearchRun<K>>,
    candidates: Vec<Candidate>,
    linked: Vec<(K, Candidate)>,
    offline: Vec<K>,
}

impl<K: Clone + PartialEq + Send + Sync + 'static> RelinkDialog<K> {
    /// The dialog over the missing `rows`, reading files through `files`.
    pub fn new(rows: Vec<RelinkRow<K>>, files: Arc<dyn LinkFiles>, options: RelinkOptions) -> Self {
        // filmcraft: crates/ui-egui/src/panels/media_dialogs.rs:146 (`link_media`).
        todo!()
    }

    /// The app's extra checks.
    #[must_use]
    pub fn checker(self, checker: Arc<dyn CandidateChecker<K>>) -> Self {
        // filmcraft: crates/engine/src/relink.rs:202 (`check_candidate`'s media checks stay the app's).
        todo!()
    }
}

impl<K: Clone + PartialEq + Send + Sync + 'static> StackDialog for RelinkDialog<K> {
    type Result = RelinkOutcome<K>;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<RelinkOutcome<K>>> {
        // filmcraft: crates/ui-egui/src/panels/media_dialogs.rs:146 (`link_media`: table, checks, search, candidates, preview, buttons).
        todo!()
    }

    fn cancelled(&mut self, ctx: &Context) {
        // new: cancels the search and forgets a pending pick (`Files::forget`).
        todo!()
    }
}
