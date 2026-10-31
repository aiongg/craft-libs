//! [`SettingsNotices`]: what the user is told when settings can't be saved or couldn't be read.

use craft_settings::{LoadReport, SaveStatus, TickResult};
use egui::{Context, Ui};

use super::{Notice, Tone};
use crate::localized::Labels as _;

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
        let labels = SettingsNoticeLabels::of_ctx(ctx);
        if !report.section_errors.is_empty() {
            self.reset = Some(report.section_errors.iter().map(|e| e.section.clone()).collect());
        }
        if let Some(path) = &report.broken_moved_to {
            let path = path.display().to_string();
            self.status = Some(StatusLine { text: craft_i18n::fmt(&labels.moved_aside, &[("path", path.as_str())]), tone: Tone::Warn });
        }
        if let Some(why) = &report.read_only {
            let reason = why.to_string();
            self.status = Some(StatusLine { text: craft_i18n::fmt(&labels.read_only, &[("reason", reason.as_str())]), tone: Tone::Warn });
        }
        if self.status.is_some() || self.has_notice() {
            ctx.request_repaint();
        }
    }

    /// Counts failed saves and clears them on success.
    // pc: crates/ui-egui/src/prefs/prefs_ui/save.rs:62 persist (SAVE_NOTICE_AFTER :44)
    pub fn ticked(&mut self, ctx: &Context, result: &TickResult) {
        match &result.status {
            SaveStatus::Failed(error) => {
                self.failures = self.failures.saturating_add(1);
                let error = error.to_string();
                if self.failures == 1 {
                    let labels = SettingsNoticeLabels::of_ctx(ctx);
                    self.status = Some(StatusLine { text: craft_i18n::fmt(&labels.save_failed, &[("error", error.as_str())]), tone: Tone::Bad });
                } else if self.failures == SAVE_NOTICE_AFTER {
                    self.cant_save = Some(error);
                }
            }
            SaveStatus::Saved => {
                if self.failures > 0 {
                    self.status = Some(StatusLine { text: SettingsNoticeLabels::of_ctx(ctx).saved.clone(), tone: Tone::Good });
                }
                self.failures = 0;
                self.cant_save = None;
                self.reset = None;
            }
            SaveStatus::Idle | SaveStatus::Pending => return,
        }
        ctx.request_repaint();
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
        let labels = SettingsNoticeLabels::of(ui);
        let app = craft_host::Host::try_of_ctx(ui.ctx()).map_or_else(|| "The app".to_owned(), |host| host.app().name.to_owned());
        if let Some(error) = &self.cant_save {
            let body = craft_i18n::fmt(&labels.cant_save_body, &[("app", app.as_str())]);
            Notice::new(format!("{error}\n{body}")).title(labels.cant_save_title.as_str()).tone(Tone::Bad).id_salt("settings-cant-save").show(ui);
        }
        if let Some(paths) = &self.reset {
            let paths = paths.join(", ");
            let body = craft_i18n::fmt(&labels.reset_body, &[("app", app.as_str()), ("paths", paths.as_str())]);
            Notice::new(body).title(labels.reset_title.as_str()).tone(Tone::Warn).id_salt("settings-reset").show(ui);
        }
    }
}
