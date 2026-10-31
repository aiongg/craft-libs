#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`FindingsList`]: the problems a check found (preflight, export warnings), each one a row that
//! takes the user to what it is about.

use egui::Ui;

crate::words! {
    /// The list's words, English by default.
    #[non_exhaustive]
    pub struct FindingsLabels {
        no_errors: "findings.noErrors" = "No errors";
        /// `{errors}` and `{warnings}`: the counts, already pluralised by the catalog.
        counts: "findings.counts" = "{errors}, {warnings}";
        error_one: "findings.errorOne" = "{n} error";
        error_other: "findings.errorOther" = "{n} errors";
        warning_one: "findings.warningOne" = "{n} warning";
        warning_other: "findings.warningOther" = "{n} warnings";
        /// `{page}`: the page label after a finding's message.
        page: "findings.page" = "page {page}";
        error: "findings.error" = "Error";
        warning: "findings.warning" = "Warning";
    }
}

/// How bad a finding is. DesignCraft's preflight has these two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FindingLevel {
    Error,
    Warning,
}

/// One problem, in the app's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub level: FindingLevel,
    /// "Missing font: Minion Pro Bold", "photo.jpg: effective 96 ppi (< 150)".
    pub message: String,
    /// The page label shown after the message ("3", "A-2").
    pub page: Option<String>,
    /// The app's id of the item; a finding without one, and without a page, does nothing when
    /// activated.
    pub target: Option<u64>,
}

/// What the list reported this pass.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct FindingsResponse {
    /// The index of the finding the user activated (click, Enter): the app selects its item and
    /// shows its page (DesignCraft: `selection.set` and `go_to_page`).
    pub activated: Option<usize>,
}

/// A summary line over a list of findings, after InDesign's Preflight panel and DesignCraft's
/// port of it.
///
/// The summary is a status dot (radius 4.5; success colour with no errors, error colour otherwise)
/// and [`findings_summary`]. Each row is a [`ListRow`](super::ListRow)
/// with the level's icon (error, warning; the icon's accessible name is the level's word), the
/// message and the page label in the faint detail style ("page 3"). Rows keep the app's order.
///
/// One Tab stop for the list ([`List`](super::List)); arrows move, Enter or a click activates
/// and reports [`FindingsResponse::activated`]. AccessKit: a list box whose items read "Error:
/// Missing font: Minion Pro Bold, page 3".
pub struct FindingsList<'a> {
    id_salt: egui::IdSalt,
    findings: &'a [Finding],
    summary: bool,
}

impl<'a> FindingsList<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, findings: &'a [Finding]) -> Self {
        Self { id_salt: egui::IdSalt::new(id_salt), findings, summary: true }
    }

    /// Draws the summary line over the rows (default on).
    pub fn summary(mut self, summary: bool) -> Self {
        self.summary = summary;
        self
    }

    // designcraft: crates/ui-egui/src/panels/properties/links.rs:264 preflight_panel
    pub fn show(self, ui: &mut Ui) -> FindingsResponse {
        todo!()
    }
}

/// The summary text: "No errors" for an empty list, else both counts with the catalog's plural
/// forms, zeros included ("0 errors, 3 warnings", "1 error, 2 warnings").
// designcraft: crates/ui-egui/src/panels/properties/links.rs:275 (summary label)
pub fn findings_summary(findings: &[Finding], labels: &FindingsLabels) -> String {
    todo!()
}
