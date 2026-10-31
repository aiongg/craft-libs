#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! File › Save and File › Save As with their guards (N10.07), on the file service's pickers
//! ([`Files`]) and the watcher's file versions ([`FileWatch`]).
//!
//! [`SaveFlow`] decides where a save writes and what to ask first; the app serializes and writes
//! (on a job when large) and reports back. One save runs at a time per flow. The guards:
//!
//! - **No file, or Save As**: the save picker opens in the document's own folder
//!   ([`save_dialog_folder`]), with the document's name and extension suggested. The file
//!   service applies the extension policy and asks about replacing a file the panel didn't ask
//!   about.
//! - **An email attachment's temporary copy** ([`is_mail_temp_copy`]): Save asks where to keep
//!   the document, as for a new one; that folder is no place to keep work.
//! - **The file changed on disk** since the document was opened or last saved (another app
//!   wrote it): Save asks first ([`SaveStep::AskChangedOnDisk`]); the app opens craft-ui's
//!   `StandardPrompt::changed_on_disk` and answers with [`SaveFlow::replace`] (Replace),
//!   [`SaveFlow::save_elsewhere`] (Save As, keeps both) or [`SaveFlow::cancel`]. Only when
//!   confirming the document's own file (PhotoCraft #6596, PdfCraft #2166).
//!
//! It is egui-free: the questions are steps the app answers, as with craft-quit's `CloseGuard`,
//! which hands its `GuardStep::Save(doc)` to [`SaveFlow::start`] and hears back through
//! [`SaveFlow::written`] (`CloseGuard::saved`) or a cancelled step (`CloseGuard::save_failed`).

use std::hash::Hash;
use std::path::{Path, PathBuf};

use craft_host::{FileFilter, Files, PickAnswer, PickId};
use craft_platform::PlatformText;
use craft_platform::watch::FileWatch;

/// Where a save writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SaveTarget {
    /// File › Save: the document's own file; asks for one when it has none.
    InPlace,
    /// File › Save As: always asks; the document takes the new file.
    As,
    /// File › Save a Copy: always asks; the document keeps its own file and stays unsaved.
    Copy,
}

/// The document a save is for, as the flow needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveDocument<D> {
    /// The app's key for the document; every step carries it back.
    pub key: D,
    /// The display name ("poster.psd", "Untitled-2"): the suggested file name, and `{name}` in
    /// messages.
    pub name: String,
    /// The document's own file; `None` for a new document or one opened from bytes.
    pub path: Option<PathBuf>,
    /// The extension a picked name without one gets, lowercase (`"pdf"`).
    pub extension: String,
    /// The picker's filters, the document's format first.
    pub filters: Vec<FileFilter>,
    /// The picker's remembered-folder purpose (`"document"`).
    pub purpose: String,
}

/// What the app does next.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a save step needs its answer"]
pub enum SaveStep<D> {
    /// The save picker is open; the answer comes from [`SaveFlow::take_pick`] on a later pass.
    Picking,
    /// `path` is the document's own file and another app changed it: ask, then answer with
    /// [`SaveFlow::replace`], [`SaveFlow::save_elsewhere`] or [`SaveFlow::cancel`].
    AskChangedOnDisk { key: D, path: PathBuf },
    /// Write the document to `path` now; then call [`SaveFlow::written`] or
    /// [`SaveFlow::failed`]. With [`SaveTarget::As`] the document takes `path` as its file; with
    /// [`SaveTarget::Copy`] it keeps its own.
    Write { key: D, path: PathBuf, target: SaveTarget },
    /// The user cancelled (the picker or the question); nothing was written.
    Cancelled { key: D },
    /// Another save of this flow is still running (a picker, a question or a write); nothing
    /// started. The app shows [`SAVE_IN_PROGRESS`].
    Busy,
}

/// The state of one save.
#[derive(Clone, Debug)]
enum Pending<D> {
    Picking { document: SaveDocument<D>, target: SaveTarget, ticket: PickId },
    Asking { document: SaveDocument<D>, target: SaveTarget, path: PathBuf },
    Writing { key: D, path: PathBuf, target: SaveTarget },
}

/// Save and Save As for an app's documents, one save at a time.
///
/// ```ignore
/// // File › Save
/// match saves.start(host.files(), &watch, document(doc), SaveTarget::InPlace) {
///     SaveStep::Write { key, path, target } => write_on_a_job(key, path, target),
///     SaveStep::AskChangedOnDisk { key, path } => { dialogs.open(StandardPrompt::changed_on_disk(name(key)).tag(key)); }
///     SaveStep::Busy => status(SAVE_IN_PROGRESS),
///     SaveStep::Picking | SaveStep::Cancelled { .. } => {}
/// }
/// // Each pass: the picker's answer.
/// if let Some(step) = saves.take_pick(host.files(), &watch) { handle(step) }
/// // The job finished:
/// saves.written(&mut watch, key, &path);
/// ```
#[derive(Debug)]
pub struct SaveFlow<D> {
    pending: Option<Pending<D>>,
}

impl<D> Default for SaveFlow<D> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<D: Clone + Eq + Hash + Send + 'static> SaveFlow<D> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a save. [`SaveTarget::InPlace`] of a document with a file (not a mail temporary
    /// copy) goes straight to the changed-on-disk check, then [`SaveStep::Write`]; anything else
    /// opens the save picker in [`save_dialog_folder`] with `name` (its extension made
    /// `extension`) suggested.
    // pdfcraft: crates/ui-egui/src/editing/save.rs:41 save_then, :125 save_doc_to_then
    // pc: crates/ui-egui/src/app/files.rs:165 save_dialog_flow
    pub fn start(&mut self, files: &dyn Files, watch: &FileWatch<D>, document: SaveDocument<D>, target: SaveTarget) -> SaveStep<D> {
        todo!()
    }

    /// The picker's answer, when it is this flow's: the next step (the changed-on-disk check
    /// when the picked path is the document's own file, else [`SaveStep::Write`]; a cancelled
    /// pick is [`SaveStep::Cancelled`]). An answer to another ticket goes back to `files` with
    /// `return_pick` and this returns `None`.
    // pc: crates/ui-egui/src/app/files.rs:205 pick_save continuation
    pub fn take_pick(&mut self, files: &dyn Files, watch: &FileWatch<D>) -> Option<SaveStep<D>> {
        todo!()
    }

    /// An answer the app took from the file service itself (it reads `take_pick` in one place
    /// for every feature): the next step, or `None` when it isn't this flow's ticket (the answer
    /// stays the caller's).
    pub fn answer_pick(&mut self, answer: &PickAnswer, watch: &FileWatch<D>) -> Option<SaveStep<D>> {
        todo!()
    }

    /// The changed-on-disk question's Replace: write over the changed file.
    // pdfcraft: crates/ui-egui/src/editing/save.rs:141 overwrite_changed_file
    pub fn replace(&mut self) -> SaveStep<D> {
        todo!()
    }

    /// The changed-on-disk question's Save As: leave the file on disk as it is and ask where to
    /// put this version.
    // pdfcraft: crates/ui-egui/src/editing/save.rs:150 save_changed_file_elsewhere
    pub fn save_elsewhere(&mut self, files: &dyn Files) -> SaveStep<D> {
        todo!()
    }

    /// Cancels the save in progress: [`SaveStep::Cancelled`] for its document; `None` when
    /// nothing was running.
    pub fn cancel(&mut self) -> Option<SaveStep<D>> {
        todo!()
    }

    /// The write the last [`SaveStep::Write`] asked for succeeded: the flow is free again and
    /// the watcher records the file's new version, so the app's own save never reads as a change
    /// on disk (not for [`SaveTarget::Copy`], whose file isn't the document's).
    pub fn written(&mut self, watch: &mut FileWatch<D>, key: D, path: &Path) {
        todo!()
    }

    /// The write failed; the flow is free again. The app shows [`SAVE_FAILED`].
    pub fn failed(&mut self, key: &D) {
        todo!()
    }

    /// A save is running: a picker, a question or a write.
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }
}

/// Whether `path` is the temporary copy an email client opened an attachment from, so that Save
/// asks where to keep the document instead of writing there. Outlook opens one from its cache:
/// `INetCache/Content.Outlook/<random>/<file>` for classic Outlook (`Temporary Internet
/// Files/Content.Outlook/` on older Windows; exactly one folder, then the file) and
/// `Packages/Microsoft.OutlookForWindows_<id>` for the new Outlook; Apple Mail from
/// `Library/Containers/com.apple.mail/Data/Library/Mail Downloads/`. Matched on either separator
/// and in any case, so it is testable on every platform.
// pdfcraft: crates/ui-egui/src/editing/save.rs:226 is_mail_temp_copy
pub fn is_mail_temp_copy(path: &Path) -> bool {
    todo!()
}

/// The folder a save picker opens in for a document stored at `path`: the document's own
/// folder, as Acrobat and Foxit do. `None` (the picker's remembered folder) for a new document,
/// a path with no parent, or an email attachment's temporary copy. Right for UNC paths too
/// (`\\server\share\dir\f.pdf` gives `\\server\share\dir`).
// pdfcraft: crates/ui-egui/src/editing/save.rs:244 save_dialog_folder
pub fn save_dialog_folder(path: Option<&Path>) -> Option<PathBuf> {
    todo!()
}

/// "Couldn't save {name}: {error}".
pub const SAVE_FAILED: PlatformText = PlatformText { id: "save.failed", english: "Couldn't save “{name}”: {error}" };
/// [`SaveStep::Busy`]. PhotoCraft's wording is for its format options dialogs ("Answer the TIFF,
/// Targa, DDS or JPEG Options before starting another save"); this is the general one.
pub const SAVE_IN_PROGRESS: PlatformText = PlatformText { id: "save.inProgress", english: "Finish the save in progress before starting another." };
/// The status line after a save; `{name}`.
pub const SAVED: PlatformText = PlatformText { id: "save.saved", english: "Saved “{name}”" };
