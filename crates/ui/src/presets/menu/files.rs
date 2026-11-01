//! Preset files: Load…, Save… and Export through the host's `Files`, picked and finished inside the component (`PresetFiles`, `PresetFileEvent`, `PresetExport`).

use craft_host::{Host, PickId, PickRequest, Picked};
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
    /// for a file name and `{reason}` for the error.
    pub struct PresetFileLabels {
        load: "presetFiles.load" = "Load…";
        replace: "presetFiles.replace" = "Replace…";
        save: "presetFiles.save" = "Save…";
        loaded: "presetFiles.loaded" = "Loaded {file}";
        saved: "presetFiles.saved" = "Saved {file}";
        read_failed: "presetFiles.readFailed" = "Could not read {file}: {reason}";
        write_failed: "presetFiles.writeFailed" = "Could not save {file}: {reason}";
    }
}

/// The pick a menu command started, kept in egui memory until its answer arrives.
#[derive(Clone)]
struct Pending {
    ticket: PickId,
    job: Job,
}

#[derive(Clone)]
enum Job {
    Load { replace: bool },
    Save(PresetExport),
}

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
    Loaded { file_name: String, bytes: Vec<u8>, replace: bool },
    /// Save… wrote the export to the file the user picked. Nothing in the document changes.
    Saved { file_name: String },
}

/// What [`PresetFiles::take_events`] returns: the events of a pick answered this pass and its
/// status line ("Loaded Contours.shc", or a `Tone::Bad` line for a failed read or write), for the
/// app's status bar.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PresetFilesOutput {
    pub events: Vec<PresetFileEvent>,
    pub status: Option<StatusLine>,
}

/// Reads and writes for a preset kind, through the installed host's `Files`: the panel menu,
/// the picker and the named row ask for a file and finish the pick themselves; the app sees only
/// bytes in and bytes out.
///
/// ```ignore
/// let files = PresetFiles::new("contours", "Contours", &["shc"]).replace(true)
///     .export(|_| Some(PresetExport { file_name: "Contours.shc".into(), bytes: app.contour_bytes() }));
/// // each pass, where the menu or picker lives:
/// for event in files.take_events(ctx, "contour-picker").events { /* PresetFileEvent::Loaded { .. } */ }
/// ```
///
/// - Load… and Replace… ask `Files::pick(PickRequest::open(purpose).title(..).filter(name,
///   extensions))`; Save… calls the export closure with the selected key, then asks
///   `PickRequest::save(purpose).file_name(..).suggested_extension(..)` and keeps the bytes with
///   the ticket. Without an installed host nothing is asked.
/// - A new pick forgets the ticket of one still open (`Files::forget`). The ticket is kept in
///   egui memory under the component's `id_salt`; the answer is read with `Files::take_answer`
///   in the pass it arrives (`take_events`), the file read or written then (one small file the
///   user asked for, so in the pass), and a cancelled pick reports nothing.
/// - Paths answers are read with `Files::read` and written with `Files::write` (atomic);
///   `Picked::Contents` (the web) gives the bytes directly.
pub struct PresetFiles<'a, K> {
    pub(crate) purpose: String,
    pub(crate) name: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) replace: bool,
    pub(crate) labels: Option<PresetFileLabels>,
    pub(crate) export: Option<Export<'a, K>>,
}

/// The app's Save… bytes for the selected key.
pub(crate) type Export<'a, K> = Box<dyn Fn(Option<&K>) -> Option<PresetExport> + 'a>;

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
        }
    }

    /// Offers Replace… beside Load… (Contour's Replace Contours…).
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Offers Save…: `export` makes the bytes for the selected key (`None` when nothing is
    /// selected); returning `None` leaves Save… disabled.
    pub fn export(mut self, export: impl Fn(Option<&K>) -> Option<PresetExport> + 'a) -> Self {
        self.export = Some(Box::new(export));
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

    /// Finishes a pick of the component `id_salt` answered since the last pass: reads or writes
    /// the file and reports it. Call it every pass where the component lives (the picker and the
    /// named row call it themselves); it costs one memory lookup while no pick is open.
    pub fn take_events(&self, ctx: &Context, id_salt: impl AsIdSalt) -> PresetFilesOutput {
        let salt = IdSalt::new(id_salt);
        let key = pick_id(salt);
        let mut out = PresetFilesOutput::default();
        let Some(pending) = ctx.data(|d| d.get_temp::<Pending>(key)) else { return out };
        let Some(host) = Host::try_of_ctx(ctx) else { return out };
        let Some(picked) = host.files().take_answer(pending.ticket) else { return out };
        ctx.data_mut(|d| d.remove::<Pending>(key));
        let labels = self.words(ctx);
        let line = |template: &str, file: &str, reason: Option<String>, tone: Tone| {
            let reason = reason.unwrap_or_default();
            Some(StatusLine { text: craft_i18n::fmt(template, &[("file", file), ("reason", &reason)]), tone })
        };
        match pending.job {
            Job::Load { replace } => {
                let file = match picked {
                    Picked::Cancelled => return out,
                    Picked::Paths(paths) => paths.into_iter().next().map(|path| {
                        let name = name_of(&path);
                        host.files().read(&path).map(|bytes| (name.clone(), bytes)).map_err(|error| (name, error.to_string()))
                    }),
                    Picked::Contents(files) => files.into_iter().next().map(|(name, bytes)| Ok((name, bytes))),
                };
                match file {
                    None => {}
                    Some(Ok((file_name, bytes))) => {
                        out.status = line(&labels.loaded, &file_name, None, Tone::Good);
                        out.events.push(PresetFileEvent::Loaded { file_name, bytes, replace });
                    }
                    Some(Err((name, reason))) => out.status = line(&labels.read_failed, &name, Some(reason), Tone::Bad),
                }
            }
            Job::Save(export) => {
                let Some(path) = picked.first_path() else { return out };
                let name = name_of(&path);
                match host.files().write(&path, &export.bytes) {
                    Ok(()) => {
                        out.status = line(&labels.saved, &name, None, Tone::Good);
                        out.events.push(PresetFileEvent::Saved { file_name: name });
                    }
                    Err(error) => out.status = line(&labels.write_failed, &name, Some(error.to_string()), Tone::Bad),
                }
            }
        }
        out
    }

    /// The menu rows: Load…, Replace… (with `replace`), Save… (with `export`, enabled when it
    /// returns bytes for `selected`), commands [`CMD_LOAD`], [`CMD_REPLACE`], [`CMD_SAVE`].
    pub(crate) fn entries(&self, ctx: &Context, selected: Option<&K>) -> Vec<MenuEntry<PanelCommand>> {
        let labels = self.words(ctx);
        let item = |label: &str, command: &str, enabled: bool| MenuEntry::from(MenuItem::new(label, command.to_owned()).enabled(enabled));
        let mut out = vec![item(&labels.load, CMD_LOAD, true)];
        if self.replace {
            out.push(item(&labels.replace, CMD_REPLACE, true));
        }
        if let Some(export) = &self.export {
            out.push(item(&labels.save, CMD_SAVE, export(selected).is_some()));
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
                Some(export) => Job::Save(export),
                None => return false,
            },
            _ => return false,
        };
        let Some(host) = Host::try_of_ctx(ctx) else { return false };
        let key = pick_id(IdSalt::new(id_salt));
        if let Some(old) = ctx.data(|d| d.get_temp::<Pending>(key)) {
            host.files().forget(old.ticket);
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
            Job::Save(export) => {
                let mut request = PickRequest::save(self.purpose.as_str())
                    .title(title)
                    .file_name(export.file_name.as_str())
                    .filter(self.name.as_str(), &extensions);
                if let Some(first) = extensions.first() {
                    request = request.suggested_extension(*first);
                }
                request
            }
        };
        let ticket = host.files().pick(request);
        ctx.data_mut(|d| d.insert_temp(key, Pending { ticket, job }));
        ctx.request_repaint();
        true
    }
}
