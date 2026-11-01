//! Preset files: Load…, Save… and Export through the host's `Files`, picked and finished inside the component (`PresetFiles`, `PresetFileEvent`, `PresetExport`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Context};

use crate::composites::StatusLine;
use crate::docking::panel_menu::PanelCommand;
use crate::menu::MenuEntry;

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

    /// Finishes a pick of the component `id_salt` answered since the last pass: reads or writes
    /// the file and reports it. Call it every pass where the component lives (the picker and the
    /// named row call it themselves); it costs one memory lookup while no pick is open.
    pub fn take_events(&self, ctx: &Context, id_salt: impl AsIdSalt) -> PresetFilesOutput {
        // color: crates/color/src/ui/swatches.rs:663-700 (`finish_file_pick`)
        todo!()
    }

    /// The menu rows: Load…, Replace… (with `replace`), Save… (with `export`, enabled when it
    /// returns bytes for `selected`), commands [`CMD_LOAD`], [`CMD_REPLACE`], [`CMD_SAVE`].
    pub(crate) fn entries(&self, ctx: &Context, selected: Option<&K>) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:183-191 (Load Contours…, Replace Contours…, Save Contours… enabled with user contours)
        todo!()
    }

    /// Starts the pick of one of [`entries`](Self::entries)' commands for the component
    /// `id_salt`; returns false for another command or without a host.
    pub(crate) fn start(&self, ctx: &Context, id_salt: impl AsIdSalt, command: &str, selected: Option<&K>) -> bool {
        // color: crates/color/src/ui/swatches.rs:641-661 (`start_file_pick`: forget the old ticket, keep the new one in egui data)
        todo!()
    }
}
