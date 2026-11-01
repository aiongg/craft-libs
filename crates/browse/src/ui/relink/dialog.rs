//! `RelinkDialog`: the Link Media dialog for a document's missing links.

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use craft_docpath::links::{
    Candidate, CandidateProblem, Link, LinkFiles, LinkSearch, MatchCriteria, SearchEnd, SearchHit, SearchUpdate, check_candidate, derive_remap, rank,
};
use craft_host::queue::{JobQueue, Priority, QueueOptions};
use craft_host::{Host, Jobs, PickId, PickRequest, Wake};
use craft_platform::open::natural_cmp;
use craft_theme::TypeRole;
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::canvas::textures::{ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails};
use craft_ui::composites::live::{self, Politeness};
use craft_ui::controls::{Button, Checkbox, TextField};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width, dialog_id};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{List, ListRow, Table, TableColumn, TableLayout};
use craft_ui::localized::words::Counted;
use craft_ui::localized::{LocaleNumbers, SharedLabels as _, Words, fmt};
use craft_ui::theme::{Tokens, TokensOf as _, current, role_format};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, Align2, Color32, Context, CornerRadius, Key, Label, Layout, Modifiers, Rect, Response, Sense, Ui, pos2, vec2};

use super::RelinkStyle;
use crate::links::SearchRun;
use crate::thumbs::{FileThumbKey, FileThumbSource};

/// The id the dialog's widgets hang under, and its `DialogId` ([`dialog_id`] of it).
const KEY: &str = "relink";

/// The folder memory of the dialog's picks.
const PICK_PURPOSE: &str = "relink";

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
    ///
    /// Without jobs and without a `Host` the dialog can't run a search: Search stays disabled.
    pub jobs: Option<Arc<dyn Jobs>>,
    /// The table's saved columns (order, hidden, widths, sort); `None` starts from the default
    /// layout. The dialog writes every change into the handle, so the host reads the layout
    /// after any outcome (Done, Cancel, Esc) from its own clone of it.
    pub layout: RelinkLayout,
}

impl Default for RelinkOptions {
    fn default() -> Self {
        // filmcraft: crates/engine/src/relink.rs:161 (`MatchOptions::default`: relink others on).
        Self {
            criteria: MatchCriteria::default(),
            app_criteria: Vec::new(),
            relink_others: true,
            exact_names: false,
            folder: None,
            document: None,
            jobs: None,
            layout: RelinkLayout::default(),
        }
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

/// The running search's progress line: `{n}` files indexed so far.
pub const SEARCHING: Counted = Counted::new("browse.relink.searching", "Searching… {n} file", "Searching… {n} files");

craft_ui::words! {
    /// The dialog's words, English by default.
    #[non_exhaustive]
    pub struct RelinkLabels {
        title: "browse.relink.title" = "Link Media";
        message: "browse.relink.message" = "Find the missing files, or leave them offline.";
        owner: "browse.relink.owner" = "Clip Name";
        file_name: "browse.relink.fileName" = "File Name";
        file_path: "browse.relink.filePath" = "File Path";
        status: "browse.relink.status" = "Status";
        match_heading: "browse.relink.match" = "Match file properties";
        match_name: "browse.relink.matchName" = "File Name";
        match_extension: "browse.relink.matchExtension" = "File Extension";
        match_same_file: "browse.relink.matchSameFile" = "Same File";
        relink_others: "browse.relink.others" = "Relink others automatically";
        search_in: "browse.relink.searchIn" = "Search in:";
        browse: "browse.relink.browse" = "Browse…";
        exact_names: "browse.relink.exactNames" = "Exact name matches only";
        search: "browse.relink.search" = "Search";
        cancel_search: "browse.relink.cancelSearch" = "Stop";
        search_started: "browse.relink.searchStarted" = "Searching…";
        search_done: "browse.relink.searchDone" = "Search finished";
        same_file: "browse.relink.sameFile" = "same file";
        matches: "browse.relink.matches" = "matches";
        different_file: "browse.relink.differentFile" = "different file";
        not_found: "browse.relink.notFound" = "No candidates found";
        name_differs: "browse.relink.nameDiffers" = "File name differs ({found} vs {wanted})";
        extension_differs: "browse.relink.extensionDiffers" = "Extension differs (.{found} vs .{wanted})";
        size_differs: "browse.relink.sizeDiffers" = "Not the same file: {found} bytes, the original had {wanted}";
        fingerprint_differs: "browse.relink.fingerprintDiffers" = "Not the same file: the contents differ";
        unreadable: "browse.relink.unreadable" = "Can't read the file: {reason}";
        candidates: "browse.relink.candidates" = "Candidates";
        locate: "browse.relink.locate" = "Locate…";
        link: "browse.relink.link" = "Link";
        link_anyway: "browse.relink.linkAnyway" = "Link Anyway";
        offline: "browse.relink.offline" = "Leave Offline";
        offline_all: "browse.relink.offlineAll" = "Leave All Offline";
        done: "browse.relink.done" = "Done";
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
///   `LinkFiles`, on `RelinkOptions::jobs` or the `Host`'s (with neither, Search is disabled); the
///   progress line shows the count; its polite live region announces the start and the end only;
///   Stop cancels; hits for the selected row fill the list as they arrive, already through the
///   app's checker (the search's checker hook, one job); ranked (`rank`), a passing same file first. Each
///   candidate shows its mark (same file `status.good.fg`, matches `status.good.fg`, different
///   file `status.bad.fg`, glyph and text) with its problems as the tooltip. Locate… picks one
///   file and adds it, checked in a job, as the selected candidate. Link (a default button that keeps the
///   dialog open) links the selected candidate and moves to the next missing row; when the candidate fails a check the button
///   reads Link Anyway. With Relink others, a link derives a `Remap` (`derive_remap`) and every
///   other missing row it fits (`Remap::apply`, `resolve_absolute`, then the checks in a job) is
///   linked; rows that fail stay missing. Leave Offline and Leave All Offline mark rows. Done
///   returns the outcome; Cancel and Esc return nothing; Esc while a pick is open closes the pick
///   only; the dialog does not see the key. The table sorts by column and saves its layout
///   through [`RelinkOptions::layout`] with every outcome. A network or device link is refused before any read.
/// - **Never blocks.** The search and every check are jobs; results arrive through `Wake`.
/// - **AccessKit.** `Dialog`; the table a `Table`; candidates a `ListBox`; the progress a live
///   region.
/// - **RTL.** Table columns and the candidate and preview split mirror.
pub struct RelinkDialog<K> {
    id: DialogId,
    state: DialogState<()>,
    model: Model<K>,
}

/// The row a missing link is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fate {
    Missing,
    Linked,
    Offline,
}

/// The dialog table's layout, shared with the host: the host keeps a clone, the dialog writes
/// each change into it, and the host saves [`get`](RelinkLayout::get) when the dialog has closed.
#[derive(Clone, Debug)]
pub struct RelinkLayout(Arc<Mutex<TableLayout<RelinkColumn>>>);

impl RelinkLayout {
    /// A handle holding `layout`, e.g. the one read back from settings.
    pub fn new(layout: TableLayout<RelinkColumn>) -> Self {
        Self(Arc::new(Mutex::new(layout)))
    }

    /// The layout now.
    pub fn get(&self) -> TableLayout<RelinkColumn> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set(&self, layout: &TableLayout<RelinkColumn>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = layout.clone();
    }
}

impl Default for RelinkLayout {
    /// Every column in order, nothing hidden, unsorted.
    fn default() -> Self {
        Self::new(TableLayout {
            order: vec![RelinkColumn::Owner, RelinkColumn::FileName, RelinkColumn::FilePath, RelinkColumn::Status],
            hidden: Vec::new(),
            widths: Vec::new(),
            sort: None,
        })
    }
}

/// The dialog table's columns: the keys of its saved [`TableLayout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RelinkColumn {
    /// The app's name for what uses the file.
    Owner,
    /// The file name of the stored path.
    FileName,
    /// The stored path.
    FilePath,
    /// The status text.
    Status,
}

/// What a check job was asked for, so its answer finds its place.
#[derive(Clone, Copy, Debug)]
enum Purpose {
    /// A file the user located for `row`.
    Locate { row: usize, generation: u64 },
    /// The file a remap offers for `row` when another row was linked.
    Other { row: usize },
}

struct Checked {
    purpose: Purpose,
    candidate: Candidate,
}

/// A folder or file pick the user has open.
#[derive(Clone, Copy, Debug)]
enum Pick {
    Folder(PickId),
    Locate { id: PickId, row: usize },
}

impl Pick {
    fn id(self) -> PickId {
        match self {
            Pick::Folder(id) | Pick::Locate { id, .. } => id,
        }
    }
}

/// What the footer's and the leading buttons ask for in one pass.
#[derive(Clone, Copy, Debug)]
enum Action {
    Offline,
    OfflineAll,
}

/// The queues and wake the dialog starts jobs with; made on the first pass, when the context is
/// known.
struct Runtime {
    jobs: Arc<dyn Jobs>,
    wake: Wake,
    /// Checks of located files and of the files a remap offers; a search checks its own hits.
    checks: JobQueue<u64, Checked>,
}

struct Model<K> {
    rows: Vec<RelinkRow<K>>,
    fates: Vec<Fate>,
    files: Arc<dyn LinkFiles>,
    options: RelinkOptions,
    checker: Option<Arc<dyn CandidateChecker<K>>>,
    preview: Option<FileThumbSource>,
    runtime: Option<Runtime>,
    layout: TableLayout<RelinkColumn>,
    row_sel: Selection<usize>,
    cand_sel: Selection<PathBuf>,
    /// The candidate selection was made by the dialog (the best one so far), not by the user.
    auto_selected: bool,
    folder: String,
    search: Option<SearchRun<K>>,
    /// Files indexed by the running search.
    progress: usize,
    /// The last search ran to its end.
    searched: bool,
    candidates: Vec<Candidate>,
    /// Bumped whenever the candidate list is emptied, so answers for an older list are dropped.
    generation: u64,
    ticket: u64,
    pick: Option<Pick>,
    linked: Vec<(K, Candidate)>,
    offline: Vec<K>,
}

impl<K: Clone + PartialEq + Send + Sync + 'static> RelinkDialog<K> {
    /// The dialog over the missing `rows`, reading files through `files`.
    pub fn new(rows: Vec<RelinkRow<K>>, files: Arc<dyn LinkFiles>, options: RelinkOptions) -> Self {
        // filmcraft: crates/ui-egui/src/panels/media_dialogs.rs:146 (`link_media`).
        let folder = options.folder.as_ref().map(|f| f.display().to_string()).unwrap_or_default();
        let options_layout = options.layout.get();
        let row_sel = if rows.is_empty() { Selection::new() } else { Selection::only(0) };
        let model = Model {
            fates: vec![Fate::Missing; rows.len()],
            rows,
            files,
            options,
            checker: None,
            preview: None,
            runtime: None,
            layout: options_layout,
            row_sel,
            cand_sel: Selection::new(),
            auto_selected: true,
            folder,
            search: None,
            progress: 0,
            searched: false,
            candidates: Vec::new(),
            generation: 0,
            ticket: 0,
            pick: None,
            linked: Vec::new(),
            offline: Vec::new(),
        };
        Self { id: dialog_id(KEY), state: DialogState::new(), model }
    }

    /// The app's extra checks.
    #[must_use]
    pub fn checker(mut self, checker: Arc<dyn CandidateChecker<K>>) -> Self {
        // filmcraft: crates/engine/src/relink.rs:202 (`check_candidate`'s media checks stay the app's).
        self.model.checker = Some(checker);
        self
    }

    /// The source of the preview box's picture of the selected candidate. Without one the box
    /// stays empty.
    #[must_use]
    pub fn preview(mut self, source: FileThumbSource) -> Self {
        // new: the preview shows the candidate through `Thumbnails` over the host's file source.
        self.model.preview = Some(source);
        self
    }
}

impl<K: Clone + PartialEq + Send + Sync + 'static> StackDialog for RelinkDialog<K> {
    type Result = RelinkOutcome<K>;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<RelinkOutcome<K>>> {
        // filmcraft: crates/ui-egui/src/panels/media_dialogs.rs:146 (`link_media`: table, checks, search, candidates, preview, buttons).
        let labels = RelinkLabels::shared_ctx(ctx);
        let style = RelinkStyle::style(&current(ctx));
        let model = &mut self.model;
        model.poll(ctx);
        if model.pick.is_some() && stack.is_top(self.id) && ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) {
            // Esc while a pick is open closes the pick only: the dialog never sees the key.
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            model.forget_pick(ctx);
        }
        let action = Cell::new(None::<Action>);
        let open = model.fates.contains(&Fate::Missing);
        let candidate = model.selected_candidate();
        let ok_label = match candidate {
            _ if !open => &labels.done,
            Some(c) if !c.ok => &labels.link_anyway,
            _ => &labels.link,
        };
        let mut footer = Footer::standard()
            .ok(ok_label.as_str())
            .ok_stays_open(open)
            .ok_enabled(!open || candidate.is_some())
            .apply(labels.locate.as_str())
            .apply_enabled(open && model.pick.is_none())
            .leading(|ui| {
                for (label, what) in [(&labels.offline_all, Action::OfflineAll), (&labels.offline, Action::Offline)] {
                    if ui.add(Button::new(label.as_str()).enabled(open)).clicked() {
                        action.set(Some(what));
                    }
                }
            });
        if open {
            footer = footer.alternate(labels.done.as_str());
        }
        let shown = Dialog::new(KEY, labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(style.dialog_max_width))
            .footer(footer)
            .initial_focus(InitialFocus::DefaultButton)
            .show(ctx, &mut self.state, &mut (), |ui, ()| model.body(ui, &labels, &style));
        match action.take() {
            Some(Action::Offline) => model.leave_selected_offline(),
            Some(Action::OfflineAll) => model.leave_all_offline(),
            None => {}
        }
        match shown.outcome? {
            Outcome::Ok if open => {
                model.link_selected(ctx);
                None
            }
            Outcome::Ok | Outcome::Alternate => Some(DialogResult::Ok(model.outcome())),
            Outcome::Apply => {
                model.locate(ctx, &labels);
                None
            }
            Outcome::Cancel | Outcome::Close => {
                model.search = None;
                model.forget_pick(ctx);
                Some(DialogResult::Cancel)
            }
            _ => None,
        }
    }

    fn cancelled(&mut self, ctx: &Context) {
        // new: cancels the search and forgets a pending pick (`Files::forget`).
        self.model.search = None;
        self.model.forget_pick(ctx);
    }
}

impl<K: Clone + PartialEq + Send + Sync + 'static> Model<K> {
    // --- state ---

    fn unresolved(&self) -> Vec<usize> {
        (0..self.rows.len()).filter(|&i| self.fates[i] == Fate::Missing).collect()
    }

    /// The row shown selected, while it is still missing.
    fn selected_row(&self) -> Option<usize> {
        self.row_sel.lead.filter(|&i| self.fates.get(i) == Some(&Fate::Missing))
    }

    fn selected_candidate(&self) -> Option<&Candidate> {
        let path = self.cand_sel.lead.as_ref()?;
        self.candidates.iter().find(|c| &c.path == path)
    }

    fn outcome(&mut self) -> RelinkOutcome<K> {
        self.search = None;
        RelinkOutcome { linked: std::mem::take(&mut self.linked), offline: std::mem::take(&mut self.offline) }
    }

    /// Empties the candidate list and stops the search: the selected row changed or was resolved.
    fn clear_candidates(&mut self) {
        self.candidates.clear();
        self.cand_sel = Selection::new();
        self.auto_selected = true;
        self.generation += 1;
        self.search = None;
        self.progress = 0;
        self.searched = false;
    }

    /// Selects the next missing row after `row` (wrapping), or none.
    fn advance(&mut self, row: usize) {
        self.clear_candidates();
        let missing = self.unresolved();
        let next = missing.iter().copied().find(|&i| i > row).or_else(|| missing.first().copied());
        self.row_sel = next.map_or_else(Selection::new, Selection::only);
    }

    fn add_candidate(&mut self, candidate: Candidate, chosen: bool) {
        let path = candidate.path.clone();
        self.candidates.retain(|c| c.path != path);
        self.candidates.push(candidate);
        rank(&mut self.candidates);
        if chosen {
            self.cand_sel = Selection::only(path);
            self.auto_selected = false;
        } else if self.cand_sel.is_empty() || self.auto_selected {
            self.cand_sel = Selection::only(self.candidates[0].path.clone());
            self.auto_selected = true;
        }
    }

    // --- jobs ---

    /// Makes the queues once jobs are known, then takes in whatever the jobs and picks finished.
    fn poll(&mut self, ctx: &Context) {
        if self.runtime.is_none()
            && let Some(jobs) = self.options.jobs.clone().or_else(|| Host::try_of_ctx(ctx).map(|host| Arc::clone(host.jobs_shared())))
        {
            let wake = {
                let ctx = ctx.clone();
                Wake::new(move || ctx.request_repaint())
            };
            let checks = JobQueue::new(Arc::clone(&jobs), wake.clone(), QueueOptions { lanes: 1, name: "relink-check" });
            self.runtime = Some(Runtime { jobs, wake, checks });
        }
        self.poll_pick(ctx);
        self.poll_search();
        self.poll_checks();
    }

    fn poll_pick(&mut self, ctx: &Context) {
        let (Some(pick), Some(host)) = (self.pick, Host::try_of_ctx(ctx)) else { return };
        let Some(answer) = host.files().take_answer(pick.id()) else { return };
        self.pick = None;
        let Some(path) = answer.first_path() else { return };
        match pick {
            Pick::Folder(_) => self.folder = path.display().to_string(),
            Pick::Locate { row, .. } => {
                let generation = self.generation;
                self.submit_check(Purpose::Locate { row, generation }, row, path);
            }
        }
    }

    fn poll_search(&mut self) {
        let Some(run) = self.search.as_mut() else { return };
        let end = run.end();
        let updates = run.take_updates();
        for update in updates {
            match update {
                SearchUpdate::Progress { files } => self.progress = files,
                SearchUpdate::Hit(hit) => self.on_hit(hit),
            }
        }
        if let Some(end) = end {
            self.search = None;
            self.searched = end != SearchEnd::Cancelled;
        }
    }

    /// A hit arrives checked: the search's checker hook ran in its job.
    fn on_hit(&mut self, hit: SearchHit<K>) {
        if self.selected_row().is_some() {
            self.add_candidate(hit.candidate, false);
        }
    }

    fn poll_checks(&mut self) {
        let Some(runtime) = &self.runtime else { return };
        let mut done = Vec::new();
        while let Some(finished) = runtime.checks.try_recv() {
            done.push(finished.result);
        }
        for Checked { purpose, candidate } in done {
            match purpose {
                Purpose::Locate { row, generation } if generation == self.generation && self.selected_row() == Some(row) => {
                    self.add_candidate(candidate, true);
                }
                Purpose::Other { row } if self.fates[row] == Fate::Missing && candidate.ok => {
                    self.record_link(row, candidate);
                    if self.selected_row().is_none() {
                        self.advance(row);
                    }
                }
                _ => {}
            }
        }
    }

    /// Starts the checks of `path` as the new file of `row` in a job.
    fn submit_check(&mut self, purpose: Purpose, row: usize, path: PathBuf) {
        let Some(runtime) = &self.runtime else { return };
        let link = self.rows[row].link.clone();
        let files = Arc::clone(&self.files);
        let checker = self.checker.clone();
        let criteria = self.options.criteria;
        let app = self.options.app_criteria.clone();
        self.ticket += 1;
        runtime.checks.submit(self.ticket, Priority::VISIBLE, move |_| {
            let candidate = check_candidate(&link, &path, &criteria, &*files);
            let candidate = match &checker {
                Some(checker) => checker.check(&link, candidate, &app),
                None => candidate,
            };
            Checked { purpose, candidate }
        });
    }

    // --- commands ---

    fn start_search(&mut self) {
        let (Some(row), Some(runtime)) = (self.selected_row(), &self.runtime) else { return };
        let folder = self.folder.trim();
        if folder.is_empty() {
            return;
        }
        let mut search = LinkSearch::new(folder);
        search.exact_names = self.options.exact_names;
        search.criteria = self.options.criteria;
        let checker = self.checker.clone();
        let app = self.options.app_criteria.clone();
        let checker = Arc::new(move |link: &Link<K>, candidate: Candidate| match &checker {
            Some(checker) => checker.check(link, candidate, &app),
            None => candidate,
        });
        let run = SearchRun::start(
            search,
            vec![self.rows[row].link.clone()],
            Arc::clone(&self.files),
            checker,
            Arc::clone(&runtime.jobs),
            runtime.wake.clone(),
        );
        self.clear_candidates();
        self.search = Some(run);
    }

    fn stop_search(&mut self) {
        if let Some(run) = self.search.as_mut() {
            run.cancel();
        }
        self.search = None;
        self.searched = false;
    }

    fn browse(&mut self, ctx: &Context, labels: &RelinkLabels) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        self.forget_pick(ctx);
        let mut request = PickRequest::folder(PICK_PURPOSE).title(labels.browse.as_str());
        if let Some(folder) = self.options.folder.as_ref().filter(|_| self.folder.trim().is_empty()) {
            request = request.directory(folder.clone());
        }
        self.pick = Some(Pick::Folder(host.files().pick(request)));
    }

    fn locate(&mut self, ctx: &Context, labels: &RelinkLabels) {
        let (Some(host), Some(row)) = (Host::try_of_ctx(ctx), self.selected_row()) else { return };
        self.forget_pick(ctx);
        let id = host.files().pick(PickRequest::open(PICK_PURPOSE).title(labels.locate.as_str()));
        self.pick = Some(Pick::Locate { id, row });
    }

    fn forget_pick(&mut self, ctx: &Context) {
        if let (Some(pick), Some(host)) = (self.pick.take(), Host::try_of_ctx(ctx)) {
            host.files().forget(pick.id());
        }
    }

    fn record_link(&mut self, row: usize, candidate: Candidate) {
        self.fates[row] = Fate::Linked;
        self.linked.push((self.rows[row].link.key.clone(), candidate));
    }

    /// Links the selected row to the selected candidate and goes on to the next row; with Relink
    /// others, every other missing row the same folder change fits is checked in a job.
    fn link_selected(&mut self, ctx: &Context) {
        let (Some(row), Some(candidate)) = (self.selected_row(), self.selected_candidate().cloned()) else { return };
        let remap = self.options.relink_others.then(|| derive_remap(&self.rows[row].link.stored, &candidate.path.to_string_lossy())).flatten();
        self.record_link(row, candidate);
        if let Some(remap) = remap {
            for other in self.unresolved() {
                let moved = remap.apply(&self.rows[other].link.stored);
                // A network or device path is refused here, before any file is read.
                let Some(path) = moved.and_then(|moved| craft_docpath::resolve_absolute(&moved, self.options.document.as_deref()).ok()) else {
                    continue;
                };
                self.submit_check(Purpose::Other { row: other }, other, path);
            }
        }
        self.advance(row);
        ctx.request_repaint();
    }

    fn leave_selected_offline(&mut self) {
        let Some(row) = self.selected_row() else { return };
        self.fates[row] = Fate::Offline;
        self.offline.push(self.rows[row].link.key.clone());
        self.advance(row);
    }

    fn leave_all_offline(&mut self) {
        for row in self.unresolved() {
            self.fates[row] = Fate::Offline;
            self.offline.push(self.rows[row].link.key.clone());
        }
        self.clear_candidates();
        self.row_sel = Selection::new();
    }

    // --- drawing ---

    fn body(&mut self, ui: &mut Ui, labels: &RelinkLabels, style: &RelinkStyle) {
        let ctx = ui.ctx().clone();
        ui.label(labels.message.as_str());
        self.table(ui, labels, style);
        ui.separator();
        self.match_checks(ui, labels);
        ui.separator();
        self.search_rows(ui, &ctx, labels);
        self.candidates_and_preview(ui, &ctx, labels, style);
    }

    fn table(&mut self, ui: &mut Ui, labels: &RelinkLabels, style: &RelinkStyle) {
        let columns = [
            TableColumn::new(RelinkColumn::Owner, labels.owner.as_str()),
            TableColumn::new(RelinkColumn::FileName, labels.file_name.as_str()),
            TableColumn::new(RelinkColumn::FilePath, labels.file_path.as_str()),
            TableColumn::new(RelinkColumn::Status, labels.status.as_str()),
        ];
        let repaired = self.layout.clone().repaired(&columns);
        if repaired != self.layout {
            self.layout = repaired;
        }
        let mut rows_in_order = self.unresolved();
        if let Some(sort) = self.layout.sort {
            let rows = &self.rows;
            sort.apply(&mut rows_in_order, |&a, &b, column| {
                let text = |row: usize| match column {
                    RelinkColumn::Owner => rows[row].owner.as_str(),
                    RelinkColumn::FileName => file_name(&rows[row].link.stored),
                    RelinkColumn::FilePath => rows[row].link.stored.as_str(),
                    RelinkColumn::Status => rows[row].status.as_str(),
                };
                natural_cmp(text(a), text(b))
            });
        }
        let order: Vec<RowInfo<usize>> = rows_in_order.into_iter().map(RowInfo::leaf).collect();
        let height = (ui.ctx().content_rect().height() * style.table_share).clamp(style.table_min, style.table_max);
        let before = self.row_sel.lead;
        let id = self.id_salt();
        let Self { rows, layout, row_sel, .. } = self;
        let bad = style.bad_mark;
        let _ = Table::new((id, "table"), labels.title.as_str(), &columns, layout, order.as_slice())
            .selection(row_sel)
            .mode(SelectionMode::Single)
            .max_height(height)
            .show(ui, |ui, &row, column, cell| {
                let row = &rows[row];
                match column {
                    RelinkColumn::Owner => drop(cell.label(ui, row.owner.as_str())),
                    RelinkColumn::FileName => drop(cell.label(ui, file_name(&row.link.stored))),
                    RelinkColumn::FilePath => drop(cell.label(ui, row.link.stored.as_str())),
                    RelinkColumn::Status => drop(colored(ui, TypeRole::Body, bad, &row.status)),
                }
            });
        if self.layout != self.options.layout.get() {
            self.options.layout.set(&self.layout);
        }
        if self.row_sel.lead.is_none() {
            // A row stays selected while any is missing.
            self.row_sel = before.map_or_else(Selection::new, Selection::only);
        }
        if self.row_sel.lead != before {
            self.clear_candidates();
        }
    }

    fn id_salt(&self) -> egui::Id {
        dialog_id(KEY)
    }

    fn match_checks(&mut self, ui: &mut Ui, labels: &RelinkLabels) {
        let id = self.id_salt();
        let criteria = &mut self.options.criteria;
        ui.horizontal_wrapped(|ui| {
            ui.label(labels.match_heading.as_str());
            for (name, label, value) in [
                ("name", &labels.match_name, &mut criteria.file_name),
                ("extension", &labels.match_extension, &mut criteria.extension),
                ("same-file", &labels.match_same_file, &mut criteria.same_file),
            ] {
                let _ = Checkbox::bool(label.as_str(), value).id_salt((id, name)).show(ui);
            }
            for criterion in &mut self.options.app_criteria {
                let _ = Checkbox::bool(criterion.label.as_str(), &mut criterion.on).id_salt((id, "app", criterion.id)).show(ui);
            }
        });
        let _ = Checkbox::bool(labels.relink_others.as_str(), &mut self.options.relink_others).id_salt((id, "others")).show(ui);
    }

    fn search_rows(&mut self, ui: &mut Ui, ctx: &Context, labels: &RelinkLabels) {
        let id = self.id_salt();
        direction::horizontal(ui, |ui| {
            ui.label(labels.search_in.as_str());
            let browse = Button::new(labels.browse.as_str());
            let room = (ui.available_width() - browse.natural_width(ui) - ui.spacing().item_spacing.x).max(0.0);
            let field = TextField::new((id, "folder"), labels.search_in.as_str(), &self.folder).width(room).show(ui);
            if let Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = field.event {
                self.folder = value;
            }
            if field.submitted && self.search.is_none() {
                self.start_search();
            }
            if ui.add(browse).clicked() {
                self.browse(ctx, labels);
            }
        });
        direction::horizontal(ui, |ui| {
            let _ = Checkbox::bool(labels.exact_names.as_str(), &mut self.options.exact_names).id_salt((id, "exact")).show(ui);
            let searching = self.search.is_some();
            let (label, enabled) = if searching {
                (&labels.cancel_search, true)
            } else {
                (&labels.search, self.runtime.is_some() && self.selected_row().is_some() && !self.folder.trim().is_empty())
            };
            if ui.add(Button::new(label.as_str()).enabled(enabled)).clicked() {
                if searching {
                    self.stop_search();
                } else {
                    self.start_search();
                }
            }
        });
        self.progress_line(ui, labels);
    }

    /// The progress line. Its count changes every pass, so it is drawn silent; the live region
    /// announces the start and the end of a search only.
    fn progress_line(&self, ui: &mut Ui, labels: &RelinkLabels) {
        let ctx = ui.ctx().clone();
        let muted = RelinkStyle::style(&current(&ctx)).muted;
        live::region(ui, self.id_salt().with("progress"), Role::Status, Politeness::Polite, |ui, announced| {
            if self.search.is_some() {
                let words = Words::of_ctx(&ctx);
                let text = words.counted(&ctx, SEARCHING, &words.get(SEARCHING.word), self.progress as u64);
                let _ = colored(ui, TypeRole::Body, muted, &text);
                announced.add(labels.search_started.as_str());
            } else if self.searched && self.candidates.is_empty() {
                let _ = colored(ui, TypeRole::Body, muted, &labels.not_found);
                announced.add(labels.not_found.as_str());
            } else if self.searched {
                announced.add(labels.search_done.as_str());
            }
        });
    }

    fn candidates_and_preview(&mut self, ui: &mut Ui, ctx: &Context, labels: &RelinkLabels, style: &RelinkStyle) {
        let id = self.id_salt();
        let selected_path = self.selected_candidate().map(|c| c.path.clone());
        direction::horizontal(ui, |ui| {
            let width = (ui.available_width() - style.preview.x - ui.spacing().item_spacing.x).max(0.0);
            ui.allocate_ui_with_layout(vec2(width, style.candidates_height), Layout::top_down(Align::Min), |ui| {
                self.candidate_list(ui, ctx, labels, style, id);
            });
            self.preview_box(ui, ctx, style, selected_path);
        });
    }

    fn candidate_list(&mut self, ui: &mut Ui, ctx: &Context, labels: &RelinkLabels, style: &RelinkStyle, id: egui::Id) {
        if self.candidates.is_empty() {
            return;
        }
        let order: Vec<RowInfo<PathBuf>> = self.candidates.iter().map(|c| RowInfo::leaf(c.path.clone())).collect();
        let before = self.cand_sel.lead.clone();
        let candidates = &self.candidates;
        let _ = List::new((id, "candidates"), labels.candidates.as_str(), order.as_slice())
            .selection(&mut self.cand_sel)
            .mode(SelectionMode::Single)
            .max_height(style.candidates_height)
            .show(ui, |at, _| candidate_row(ctx, labels, style, &candidates[at], at));
        if self.cand_sel.lead != before {
            self.auto_selected = false;
        }
    }

    fn preview_box(&self, ui: &mut Ui, ctx: &Context, style: &RelinkStyle, path: Option<PathBuf>) {
        let (rect, _) = ui.allocate_exact_size(style.preview, Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::ZERO, style.preview_fill);
        let (Some(source), Some(path), Some(runtime)) = (&self.preview, path, &self.runtime) else { return };
        #[expect(clippy::field_reassign_with_default, reason = "ThumbnailOptions is non_exhaustive: no struct update outside its crate")]
        let options = {
            let mut options = ThumbnailOptions::default();
            options.jobs = Some(Arc::clone(&runtime.jobs));
            options
        };
        let thumbnails = Thumbnails::<FileThumbKey>::in_context(ctx, (KEY, "preview"), options);
        let request = ThumbRequest::for_cell(style.preview.x.max(style.preview.y), ctx.pixels_per_point());
        let state = thumbnails.state(ctx, source, &FileThumbKey::file(path), &request, Priority::VISIBLE);
        thumbnails.paint(&ui.painter().with_clip_rect(rect), rect, &state, &ThumbPaint::default().checker(false));
    }
}

/// The identity-free file tail of a stored path, for the File Name column.
fn file_name(stored: &str) -> &str {
    stored.rsplit(['/', '\\']).next().unwrap_or(stored)
}

/// `text` in `role` and `color`.
fn colored(ui: &mut Ui, role: TypeRole, color: Color32, text: &str) -> Response {
    let format = role_format(&Tokens::of(ui), role, color);
    ui.add(Label::new(LayoutJob::single_section(text.to_owned(), format)).selectable(false))
}

/// The mark of a candidate: its icon, colour and words ("same file", "matches", "different file";
/// none for a candidate that fails on a name or extension check).
fn mark<'l>(candidate: &Candidate, labels: &'l RelinkLabels, style: &RelinkStyle) -> (Icon, Color32, &'l str) {
    match (candidate.ok, candidate.same_file) {
        (true, Some(true)) => (Icon::Check, style.good_mark, labels.same_file.as_str()),
        (true, _) => (Icon::Check, style.good_mark, labels.matches.as_str()),
        (false, Some(false)) => (Icon::Close, style.bad_mark, labels.different_file.as_str()),
        (false, _) => (Icon::Close, style.bad_mark, ""),
    }
}

/// Why a candidate fails, in the user's words, in check order.
fn problem_words(candidate: &Candidate, labels: &RelinkLabels, numbers: &LocaleNumbers) -> Vec<String> {
    candidate
        .problems
        .iter()
        .map(|problem| match problem {
            CandidateProblem::NameDiffers { found, wanted } => fmt(&labels.name_differs, &[("found", found), ("wanted", wanted)]),
            CandidateProblem::ExtensionDiffers { found, wanted } => fmt(&labels.extension_differs, &[("found", found), ("wanted", wanted)]),
            CandidateProblem::DifferentFile { found, wanted } if found == wanted => labels.fingerprint_differs.clone(),
            CandidateProblem::DifferentFile { found, wanted } => {
                fmt(&labels.size_differs, &[("found", &numbers.format(*found as f64, 0..=0)), ("wanted", &numbers.format(*wanted as f64, 0..=0))])
            }
            CandidateProblem::Unreadable(reason) => fmt(&labels.unreadable, &[("reason", reason)]),
            CandidateProblem::Other(text) => text.clone(),
            _ => String::new(),
        })
        .filter(|words| !words.is_empty())
        .collect()
}

/// The list row of one candidate: its path, striped, with its mark at the trailing end and its
/// problems as the tooltip.
fn candidate_row<'r>(ctx: &Context, labels: &RelinkLabels, style: &RelinkStyle, candidate: &Candidate, at: usize) -> ListRow<'r, PathBuf> {
    let (icon, color, words) = mark(candidate, labels, style);
    let path = candidate.path.display().to_string();
    let problems = problem_words(candidate, labels, &LocaleNumbers::of_ctx(ctx)).join("; ");
    let name = if words.is_empty() { path.clone() } else { format!("{path}, {words}") };
    let mut row = ListRow::new(candidate.path.clone(), path).striped(at % 2 == 1).accessible_name(name);
    if !problems.is_empty() {
        row = row.tooltip(problems.clone()).description(problems);
    }
    let tokens = current(ctx);
    let format = role_format(&tokens, TypeRole::Caption, color);
    let words = words.to_owned();
    let job = LayoutJob::simple_singleline(words.clone(), format.font_id.clone(), color);
    let text = if words.is_empty() { 0.0 } else { ctx.fonts_mut(|fonts| fonts.layout_job(job).size().x).ceil() };
    let side = IconSize::Sm.points(&tokens);
    let gap = ctx.global_style().spacing.item_spacing.x;
    let width = side + if words.is_empty() { 0.0 } else { gap + text };
    row.trailing_item(width, move |ui| {
        let rect = ui.max_rect();
        let rtl = direction::is_rtl(ui);
        let (icon_x, text_at, align) = if rtl {
            (rect.right() - side / 2.0, pos2(rect.right() - side - gap, rect.center().y), Align2::RIGHT_CENTER)
        } else {
            (rect.left() + side / 2.0, pos2(rect.left() + side + gap, rect.center().y), Align2::LEFT_CENTER)
        };
        let icon_rect = Rect::from_center_size(pos2(icon_x, rect.center().y), vec2(side, side));
        icons::paint_on(ui.painter(), icon_rect, icon, icons::active(ui.ctx()), side, IconSize::Sm.stroke(), color);
        if !words.is_empty() {
            ui.painter().text(text_at, align, words, format.font_id, color);
        }
    })
}
