#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`SettingsNotices`]: what the user is told when settings can't be saved or couldn't be read.

use craft_settings::{LoadReport, TickResult};
use egui::{Context, Ui};

use super::Tone;

/// Failed saves in a row before the notice joins the status line.
pub const SAVE_NOTICE_AFTER: u32 = 3;

crate::words! {
    /// The settings messages, English by default. `{app}` is the app's name.
    #[non_exhaustive]
    pub struct SettingsNoticeLabels {
        /// `{error}`: the save error.
        save_failed: "settingsNotice.saveFailed" = "Couldn't save preferences: {error}. Retrying…";
        saved: "settingsNotice.saved" = "Preferences saved";
        cant_save_title: "settingsNotice.cantSaveTitle" = "Preferences can't be saved";
        cant_save_body: "settingsNotice.cantSaveBody" = "{app} keeps retrying; until a save succeeds, preference changes are lost when it closes.";
        reset_title: "settingsNotice.resetTitle" = "Some preferences were reset";
        /// `{paths}`: the comma-joined settings ("interface.uiScale, tools.brushSize").
        reset_body: "settingsNotice.resetBody" = "The preferences file held values {app} can't read for {paths}. Those settings are back to their defaults; the rest of your preferences loaded.";
        /// `{path}`: where the unreadable file was moved.
        moved_aside: "settingsNotice.movedAside" = "Preferences were reset; the unreadable file was kept as {path}.";
        /// `{reason}`: why the store won't save (a newer version's file, a failed migration).
        read_only: "settingsNotice.readOnly" = "Preferences won't be saved: {reason}";
    }
}

/// A line for the status bar.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusLine {
    pub text: String,
    pub tone: Tone,
}

/// The settings messages of one app run, after PhotoCraft's save and load reporting.
///
/// - **Load** ([`loaded`](Self::loaded)): settings that didn't fit their type were reset; a
///   notice "Some preferences were reset" names them. A file that didn't parse was moved aside
///   (status line, warning). A read-only store (a newer version's file, a failed migration) says
///   so in the status line.
/// - **Save** ([`ticked`](Self::ticked), with every `SettingsStore::tick` result): the first
///   failure puts "Couldn't save preferences: … Retrying…" in the status line (error); the
///   [`SAVE_NOTICE_AFTER`]th failure in a row adds the "Preferences can't be saved" notice; the
///   next success says "Preferences saved" and removes the notice.
///
/// The notices can't be dismissed; they go when the problem does (the reset notice when the
/// next save writes the defaults). [`show`](Self::show) draws them as error and warning
/// [`Notice`](super::Notice)s; the app puts [`take_status`](Self::take_status) in its status bar.
#[derive(Debug, Default)]
pub struct SettingsNotices {
    failures: u32,
    status: Option<StatusLine>,
    cant_save: Option<String>,
    reset: Option<Vec<String>>,
}

impl SettingsNotices {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the store's load report once at startup.
    // pc: crates/ui-egui/src/prefs/prefs_ui/startup.rs:91 repair_invalid
    pub fn loaded(&mut self, ctx: &Context, report: &LoadReport) {
        todo!()
    }

    /// Counts failed saves and clears them on success.
    // pc: crates/ui-egui/src/prefs/prefs_ui/save.rs:62 persist (SAVE_NOTICE_AFTER :44)
    pub fn ticked(&mut self, ctx: &Context, result: &TickResult) {
        todo!()
    }

    /// The status line to show, once.
    pub fn take_status(&mut self) -> Option<StatusLine> {
        self.status.take()
    }

    /// Whether a notice is up.
    pub fn has_notice(&self) -> bool {
        self.cant_save.is_some() || self.reset.is_some()
    }

    /// Draws the notices, newest first; nothing when there are none.
    pub fn show(&mut self, ui: &mut Ui) {
        todo!()
    }
}
