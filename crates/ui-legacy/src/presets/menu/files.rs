//! Preset files: Load…, Save… and Export through the host's `Files`, picked and finished inside the component (`PresetFiles`, `PresetFileEvent`, `PresetExport`).

use std::sync::Arc;

use craft_host::queue::{JobQueue, Priority, QueueOptions};
use craft_host::{FileError, Host, PickId, PickRequest, Picked, Wake};
use egui::{AsIdSalt, Context, Id, IdSalt};

use crate::composites::{StatusLine, Tone};
use crate::docking::panel_menu::PanelCommand;
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem};

/// The menu commands of the file entries.
pub(crate) const CMD_LOAD: &str = "presetFiles.load";
pub(crate) const CMD_REPLACE: &str = "presetFiles.replace";
pub(crate) const CMD_SAVE: &str = "presetFiles.save";

crate::words! {
    /// The words of the preset file entries and their status lines. Apps rename the entries
    /// ("Load Contours…", LightCraft's "Import Presets…" and "Export Presets…"). `{file}` stands
    /// for a file name and `{reason}` for one of the `reason_*` words.
    pub struct PresetFileLabels {
        load: "presetFiles.load" = "Load…";
        replace: "presetFiles.replace" = "Replace…";
        save: "presetFiles.save" = "Save…";
        loaded: "presetFiles.loaded" = "Loaded {file}";
        saved: "presetFiles.saved" = "Saved {file}";
        read_failed: "presetFiles.readFailed" = "Could not read {file}: {reason}";
        write_failed: "presetFiles.writeFailed" = "Could not save {file}: {reason}";
        reason_not_found: "presetFiles.reasonNotFound" = "the file was not found";
        reason_denied: "presetFiles.reasonDenied" = "access was denied";
        reason_unsupported: "presetFiles.reasonUnsupported" = "this is not available here";
        reason_other: "presetFiles.reasonOther" = "the file system reported an error";
    }
}

/// The pick a menu command started, kept in egui memory until its job ends.
#[derive(Clone)]
struct Pending {
    stage: Stage,
    job: Job,
}

#[derive(Clone)]
enum Stage {
    /// The picker is open; its answer has not arrived.
    Asked(PickId),
    /// Load…'s file is being read on the host's jobs.
    Reading(Arc<JobQueue<(), ReadDone>>),
}

#[derive(Clone)]
enum Job {
    Load { replace: bool },
    Save { bytes: Arc<[u8]>, file_name: String },
}

/// Why a read or write failed, worded through [`PresetFileLabels`] on the UI thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    NotFound,
    Denied,
    Unsupported,
    Other,
}

impl From<&FileError> for Failure {
    fn from(error: &FileError) -> Self {
        match error {
            FileError::NotFound(_) => Self::NotFound,
            FileError::PermissionDenied(_) => Self::Denied,
            FileError::Unsupported => Self::Unsupported,
            FileError::Other(_) => Self::Other,
        }
    }
}

impl Failure {
    fn word(self, labels: &PresetFileLabels) -> &str {
        match self {
            Self::NotFound => &labels.reason_not_found,
            Self::Denied => &labels.reason_denied,
            Self::Unsupported => &labels.reason_unsupported,
            Self::Other => &labels.reason_other,
        }
    }
}

/// What a read job returns.
struct ReadDone {
    name: String,
    bytes: Result<Vec<u8>, Failure>,
}

/// The ticket key of one component: the salt hashed with this module's tag, so it differs from
/// any other egui memory key. Two components share a ticket only when their salts are equal.
fn pick_id(salt: IdSalt) -> Id {
    Id::unique(("craft-ui-preset-files", salt))
}

/// A file's name without its folder.
fn name_of(path: &std::path::Path) -> String {
    path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// What a Save… writes: the app's bytes and the suggested file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetExport {
    /// The suggested file name with its extension: "Contours.shc".
    pub file_name: String,
    /// The file's contents in the app's format; the library has no preset file format.
    pub bytes: Vec<u8>,
}

/// What a finished file pick did.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresetFileEvent {
    /// The user picked a file for Load… (`replace: false`) or Replace… (`replace: true`); the app
    /// parses `bytes` in its own format and makes one undo step.
    Loaded {
        /// The picked file's name.
        file_name: String,
        /// The file's contents.
        bytes: Vec<u8>,
        /// True for Replace…, false for Load….
        replace: bool,
    },
    /// Save… wrote the export to the file the user picked. Nothing in the document changes.
    Saved {
        /// The name of the file written.
        file_name: String,
    },
}

/// What [`PresetFiles::take_events`] returns: the events of a pick answered this pass and its
/// status line ("Loaded Contours.shc", or a `Tone::Bad` line for a failed read or write), for the
/// app's status bar.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PresetFilesOutput {
    /// The picks answered this pass, in order.
    pub events: Vec<PresetFileEvent>,
    /// The status line for the app's status bar; `None` when no pick finished.
    pub status: Option<StatusLine>,
}

/// Reads and writes for a preset kind, through the installed host's `Files`: the panel menu,
/// the picker and the named row ask for a file and finish the pick themselves; the app sees only
/// bytes in and bytes out.
///
/// ```ignore
/// let files = PresetFiles::new("contours", "Contours", &["shc"]).replace(true)
///     .export(|_| Some(PresetExport { file_name: "Contours.shc".into(), bytes: app.contour_bytes() }));
/// // each pass, where the panel menu lives (a picker and a named row finish their own picks):
/// for event in files.take_events(ctx, "contours").events { /* PresetFileEvent::Loaded { .. } */ }
/// ```
///
/// - Load… and Replace… ask `Files::pick(PickRequest::open(purpose).title(..).filter(name,
///   extensions))`; Save… calls the export closure with the selected key, then asks
///   `PickRequest::save(purpose).file_name(..).suggested_extension(..)` and keeps the bytes with
///   the ticket. Without an installed host nothing is asked. [`can_export`](Self::can_export)
///   decides whether Save… is enabled, so the export closure runs only when Save… is chosen.
/// - A new pick forgets the ticket of one still open (`Files::forget`) and drops a read still
///   running. The ticket is kept in egui memory under the component's `id_salt`, which must be
///   unique across the app (the picker and the named row use their persistent ids). The answer is
///   read with `Files::take_answer` in the pass it arrives (`take_events`); a cancelled pick
///   reports nothing.
/// - Paths answers are read with `Files::read` through a [`JobQueue`] on the host's jobs, and
///   `Loaded` comes in the pass after the job ends (the host wakes the event loop). Save… writes
///   with `Files::write` (atomic) in the pass, as the bytes are already in memory.
///   `Picked::Contents` (the web) gives the bytes directly.
pub struct PresetFiles<'a, K> {
    pub(crate) purpose: String,
    pub(crate) name: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) replace: bool,
    pub(crate) labels: Option<PresetFileLabels>,
    pub(crate) export: Option<Export<'a, K>>,
    pub(crate) can_export: Option<CanExport<'a, K>>,
}

/// The app's Save… bytes for the selected key.
pub(crate) type Export<'a, K> = Box<dyn Fn(Option<&K>) -> Option<PresetExport> + 'a>;

/// Whether Save… is enabled for the selected key.
pub(crate) type CanExport<'a, K> = Box<dyn Fn(Option<&K>) -> bool + 'a>;

impl<'a, K> PresetFiles<'a, K> {
    /// File entries for `purpose` (the pick's purpose, which also keeps its last folder) with a
    /// filter named `name` over `extensions` (without dots). Load… only, until
    /// [`replace`](Self::replace) or [`export`](Self::export) add the others.
    pub fn new(purpose: impl Into<String>, name: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            purpose: purpose.into(),
            name: name.into(),
            extensions: extensions.iter().map(|e| (*e).to_owned()).collect(),
            replace: false,
            labels: None,
            export: None,
            can_export: None,
        }
    }

    /// Offers Replace… beside Load… (Contour's Replace Contours…).
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Offers Save…: `export` makes the bytes for the selected key when Save… is chosen
    /// (`None` when nothing is selected or the bytes cannot be made: no pick starts).
    pub fn export(mut self, export: impl Fn(Option<&K>) -> Option<PresetExport> + 'a) -> Self {
        self.export = Some(Box::new(export));
        self
    }

    /// Whether Save… is enabled for the selected key (default: always enabled; the export may save the current settings with nothing selected). A cheap test
    /// the menu reads every pass; `export` runs only in the pass Save… is chosen.
    pub fn can_export(mut self, can_export: impl Fn(Option<&K>) -> bool + 'a) -> Self {
        self.can_export = Some(Box::new(can_export));
        self
    }

    /// The entries' words (default English).
    pub fn labels(mut self, labels: PresetFileLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    fn words(&self, ctx: &Context) -> PresetFileLabels {
        self.labels.clone().unwrap_or_else(|| PresetFileLabels::of_ctx(ctx))
    }

    /// Finishes a pick of the component `id_salt` answered since the last pass: starts the read of
    /// a picked file, writes a Save…, and reports what has finished. Call it every pass where the
    /// component lives (the picker and the named row call it themselves); it costs one memory
    /// lookup while no pick is open.
    pub fn take_events(&self, ctx: &Context, id_salt: impl AsIdSalt) -> PresetFilesOutput {
        let salt = IdSalt::new(id_salt);
        let key = pick_id(salt);
        let mut out = PresetFilesOutput::default();
        let Some(mut pending) = ctx.data(|d| d.get_temp::<Pending>(key)) else { return out };
        let Some(host) = Host::try_of_ctx(ctx) else { return out };
        let labels = self.words(ctx);
        let line = |template: &str, file: &str, reason: Option<Failure>, tone: Tone| {
            let reason = reason.map_or("", |failure| failure.word(&labels));
            Some(StatusLine { text: craft_i18n::fmt(template, &[("file", file), ("reason", reason)]), tone })
        };
        if let Stage::Asked(ticket) = &pending.stage {
            let Some(picked) = host.files().take_answer(*ticket) else { return out };
            match (&pending.job, picked) {
                (_, Picked::Cancelled | Picked::Unavailable(_)) => {
                    ctx.data_mut(|d| d.remove::<Pending>(key));
                    return out;
                }
                (Job::Load { replace }, Picked::Contents(files)) => {
                    ctx.data_mut(|d| d.remove::<Pending>(key));
                    if let Some((file_name, bytes)) = files.into_iter().next() {
                        out.status = line(&labels.loaded, &file_name, None, Tone::Good);
                        out.events.push(PresetFileEvent::Loaded { file_name, bytes, replace: *replace });
                    }
                    return out;
                }
                (Job::Load { .. }, Picked::Paths(paths)) => {
                    let Some(path) = paths.into_iter().next() else {
                        ctx.data_mut(|d| d.remove::<Pending>(key));
                        return out;
                    };
                    let repaint = ctx.clone();
                    let queue =
                        Arc::new(JobQueue::new(host.jobs_shared().clone(), Wake::new(move || repaint.request_repaint()), QueueOptions::default()));
                    let reader = host.clone();
                    queue.submit((), Priority::VISIBLE, move |_| {
                        let name = name_of(&path);
                        ReadDone { bytes: reader.files().read(&path).map_err(|error| Failure::from(&error)), name }
                    });
                    pending.stage = Stage::Reading(queue);
                    ctx.data_mut(|d| d.insert_temp(key, pending.clone()));
                }
                (Job::Save { bytes, .. }, picked) => {
                    ctx.data_mut(|d| d.remove::<Pending>(key));
                    let Some(path) = picked.first_path() else { return out };
                    let name = name_of(&path);
                    match host.files().write(&path, bytes) {
                        Ok(()) => {
                            out.status = line(&labels.saved, &name, None, Tone::Good);
                            out.events.push(PresetFileEvent::Saved { file_name: name });
                        }
                        Err(error) => out.status = line(&labels.write_failed, &name, Some(Failure::from(&error)), Tone::Bad),
                    }
                    return out;
                }
            }
        }
        if let (Stage::Reading(queue), Job::Load { replace }) = (&pending.stage, &pending.job) {
            if QueueOptions::default().lanes == 0 {
                queue.run_inline(1);
            }
            let Some(done) = queue.try_recv() else { return out };
            ctx.data_mut(|d| d.remove::<Pending>(key));
            match done.result.bytes {
                Ok(bytes) => {
                    out.status = line(&labels.loaded, &done.result.name, None, Tone::Good);
                    out.events.push(PresetFileEvent::Loaded { file_name: done.result.name, bytes, replace: *replace });
                }
                Err(failure) => out.status = line(&labels.read_failed, &done.result.name, Some(failure), Tone::Bad),
            }
        }
        out
    }

    /// The menu rows: Load…, Replace… (with `replace`), Save… (with `export`, enabled when it
    /// per [`can_export`](Self::can_export)), commands [`CMD_LOAD`], [`CMD_REPLACE`], [`CMD_SAVE`].
    pub(crate) fn entries(&self, ctx: &Context, selected: Option<&K>) -> Vec<MenuEntry<PanelCommand>> {
        let labels = self.words(ctx);
        let item = |label: &str, command: &str, enabled: bool| MenuEntry::from(MenuItem::new(label, command.to_owned()).enabled(enabled));
        let mut out = vec![item(&labels.load, CMD_LOAD, true)];
        if self.replace {
            out.push(item(&labels.replace, CMD_REPLACE, true));
        }
        if self.export.is_some() {
            out.push(item(&labels.save, CMD_SAVE, self.can_export.as_ref().is_none_or(|can| can(selected))));
        }
        out
    }

    /// Starts the pick of one of [`entries`](Self::entries)' commands for the component
    /// `id_salt`; returns false for another command or without a host.
    pub(crate) fn start(&self, ctx: &Context, id_salt: impl AsIdSalt, command: &str, selected: Option<&K>) -> bool {
        let job = match command {
            CMD_LOAD => Job::Load { replace: false },
            CMD_REPLACE if self.replace => Job::Load { replace: true },
            CMD_SAVE => match self.export.as_ref().and_then(|export| export(selected)) {
                Some(export) => Job::Save { bytes: Arc::from(export.bytes), file_name: export.file_name },
                None => return false,
            },
            _ => return false,
        };
        let Some(host) = Host::try_of_ctx(ctx) else { return false };
        let key = pick_id(IdSalt::new(id_salt));
        if let Some(Pending { stage: Stage::Asked(ticket), .. }) = ctx.data(|d| d.get_temp::<Pending>(key)) {
            host.files().forget(ticket);
        }
        let labels = self.words(ctx);
        let label = match command {
            CMD_LOAD => &labels.load,
            CMD_REPLACE => &labels.replace,
            _ => &labels.save,
        };
        let title = label.trim_end_matches('…');
        let extensions: Vec<&str> = self.extensions.iter().map(String::as_str).collect();
        let request = match &job {
            Job::Load { .. } => PickRequest::open(self.purpose.as_str()).title(title).filter(self.name.as_str(), &extensions),
            Job::Save { file_name, .. } => {
                let mut request =
                    PickRequest::save(self.purpose.as_str()).title(title).file_name(file_name.as_str()).filter(self.name.as_str(), &extensions);
                if let Some(first) = extensions.first() {
                    request = request.suggested_extension(*first);
                }
                request
            }
        };
        let ticket = host.files().pick(request);
        ctx.data_mut(|d| d.insert_temp(key, Pending { stage: Stage::Asked(ticket), job }));
        ctx.request_repaint();
        true
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the file picks: reads go through the host's jobs, Save… keeps its bytes shared.
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use craft_host::{AppInfo, Files, ManualJobs};

    use super::*;

    #[derive(Default)]
    struct Fake {
        next: u64,
        answers: HashMap<u64, Picked>,
        reads: usize,
    }

    #[derive(Default)]
    struct FakeFiles(Mutex<Fake>);

    impl FakeFiles {
        fn lock(&self) -> std::sync::MutexGuard<'_, Fake> {
            self.0.lock().unwrap_or_else(|e| e.into_inner())
        }
    }

    impl Files for FakeFiles {
        fn pick(&self, _request: PickRequest) -> PickId {
            let mut fake = self.lock();
            fake.next += 1;
            PickId(fake.next)
        }
        fn take_answer(&self, id: PickId) -> Option<Picked> {
            self.lock().answers.remove(&id.0)
        }
        fn forget(&self, _id: PickId) {}
        fn pick_pending(&self) -> bool {
            false
        }
        fn read(&self, path: &Path) -> Result<Vec<u8>, FileError> {
            self.lock().reads += 1;
            if path.ends_with("missing.shc") { Err(FileError::NotFound(path.to_owned())) } else { Ok(vec![4, 5]) }
        }
        fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
            Ok(())
        }
        fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
            None
        }
    }

    fn setup() -> (Context, Arc<FakeFiles>, Arc<ManualJobs>) {
        let ctx = Context::default();
        let files = Arc::new(FakeFiles::default());
        let jobs = Arc::new(ManualJobs::new());
        Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).files(files.clone()).jobs(jobs.clone()).install(&ctx);
        (ctx, files, jobs)
    }

    fn answer(files: &FakeFiles, picked: Picked) {
        let id = files.lock().next;
        files.lock().answers.insert(id, picked);
    }

    // Load… reads a picked path on the host's jobs: nothing is read or reported until the job
    // runs, then `Loaded` comes with the bytes (LC49).
    #[test]
    fn load_reads_through_the_jobs_and_reports_when_the_job_ends() {
        let (ctx, files, jobs) = setup();
        let presets = PresetFiles::<u32>::new("contours", "Contours", &["shc"]);
        assert!(presets.start(&ctx, "p", CMD_LOAD, None));
        answer(&files, Picked::Paths(vec![PathBuf::from("/in/a.shc")]));
        let first = presets.take_events(&ctx, "p");
        assert!(first.events.is_empty() && first.status.is_none());
        assert_eq!(files.lock().reads, 0, "the pass does not read the file");
        assert_eq!(jobs.waiting(), 1);
        jobs.run_all();
        let done = presets.take_events(&ctx, "p");
        assert_eq!(done.events, [PresetFileEvent::Loaded { file_name: "a.shc".into(), bytes: vec![4, 5], replace: false }]);
        assert!(presets.take_events(&ctx, "p").events.is_empty(), "the pick is finished");
    }

    // A failed read says why through the labels, not the error's English text.
    #[test]
    fn a_failed_read_words_its_reason_from_the_labels() {
        let (ctx, files, jobs) = setup();
        let presets = PresetFiles::<u32>::new("contours", "Contours", &["shc"]);
        assert!(presets.start(&ctx, "p", CMD_LOAD, None));
        answer(&files, Picked::Paths(vec![PathBuf::from("/in/missing.shc")]));
        presets.take_events(&ctx, "p");
        jobs.run_all();
        let done = presets.take_events(&ctx, "p");
        assert!(done.events.is_empty());
        let status = done.status.expect("a status line");
        assert_eq!((status.text.as_str(), status.tone), ("Could not read missing.shc: the file was not found", Tone::Bad));
    }

    // Save… is enabled by `can_export`; the export closure runs only when Save… starts.
    #[test]
    fn save_is_enabled_by_can_export_and_exports_only_when_chosen() {
        let (ctx, _files, _jobs) = setup();
        let calls = std::cell::Cell::new(0);
        let presets = PresetFiles::<u32>::new("contours", "Contours", &["shc"]).can_export(|selected| selected == Some(&1)).export(|_| {
            calls.set(calls.get() + 1);
            Some(PresetExport { file_name: "C.shc".into(), bytes: vec![1] })
        });
        let enabled =
            |selected: Option<&u32>| presets.entries(&ctx, selected).iter().filter_map(|e| e.item()).map(MenuItem::is_enabled).collect::<Vec<_>>();
        assert_eq!(enabled(None), [true, false]);
        assert_eq!(enabled(Some(&1)), [true, true]);
        assert_eq!(enabled(Some(&2)), [true, false]);
        assert_eq!(calls.get(), 0, "listing the entries does not export");
        assert!(presets.start(&ctx, "p", CMD_SAVE, Some(&1)));
        assert_eq!(calls.get(), 1);
    }
}
