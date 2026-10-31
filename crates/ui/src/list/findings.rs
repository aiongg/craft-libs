//! [`FindingsList`]: the problems a check found (preflight, export warnings), each one a row that
//! takes the user to what it is about.

use egui::{Sense, Ui, vec2};

use super::{List, ListEvent, ListRow};
use crate::behavior::{RowInfo, SelectionMode};
use crate::icons::Icon;
use crate::localized::SharedLabels as _;
use crate::primitives::Text;
use craft_theme::TypeRole;

use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// Radius of the summary's status dot. S4-F18 moves it to the `findings.status_dot` token.
const STATUS_DOT_RADIUS: f32 = 4.5;

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

    pub fn show(self, ui: &mut Ui) -> FindingsResponse {
        let Self { id_salt, findings, summary } = self;
        let labels = FindingsLabels::shared(ui);
        let tokens = Tokens::of(ui);
        if summary {
            let errors = findings.iter().filter(|f| f.level == FindingLevel::Error).count();
            let color = if errors == 0 { tokens.color.status.good.fg } else { tokens.color.status.bad.fg };
            let mut row = crate::direction::row(ui);
            row.cross_align = egui::Align::Center;
            ui.allocate_ui_with_layout(vec2(ui.available_width(), tokens.typography.get(TypeRole::Body).line_height), row, |ui| {
                let (dot, _) = ui.allocate_exact_size(vec2(STATUS_DOT_RADIUS * 2.0 + 1.0, STATUS_DOT_RADIUS * 2.0 + 1.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), STATUS_DOT_RADIUS, color.to_egui());
                ui.add(Text::new(findings_summary(findings, &labels)).role(TypeRole::Body));
            });
        }

        let rows: Vec<RowInfo<usize>> = (0..findings.len()).map(RowInfo::leaf).collect();
        let style = ListRow::<usize>::style(&tokens);
        let plain = style.height(super::ThumbSize::None, false);
        let tall = style.height(super::ThumbSize::None, true);
        let heights: Vec<f32> = findings.iter().map(|f| if f.page.is_some() { tall } else { plain }).collect();
        let list = List::new(id_salt, "Findings", rows.as_slice()).mode(SelectionMode::Single).heights(move |at| heights[at]);
        let output = list.show(ui, |at, &key| {
            let finding = &findings[at];
            let (icon, level) = match finding.level {
                FindingLevel::Error => (Icon::Error, &labels.error),
                FindingLevel::Warning => (Icon::Warning, &labels.warning),
            };
            let row = ListRow::new(key, format!("{level}: {}", finding.message)).icon(icon);
            match &finding.page {
                Some(page) => row.detail(labels.page.replace("{page}", page)),
                None => row,
            }
        });

        let activated = output.events.iter().find_map(|event| match event {
            ListEvent::Clicked(at) | ListEvent::Activate(at) => Some(*at),
            _ => None,
        });
        FindingsResponse { activated: activated.filter(|&at| findings[at].target.is_some() || findings[at].page.is_some()) }
    }
}

/// The summary text: "No errors" for an empty list, else both counts with the catalog's plural
/// forms, zeros included ("0 errors, 3 warnings", "1 error, 2 warnings").
// designcraft: crates/ui-egui/src/panels/properties/links.rs:275 (summary label)
pub fn findings_summary(findings: &[Finding], labels: &FindingsLabels) -> String {
    if findings.is_empty() {
        return labels.no_errors.clone();
    }
    let errors = findings.iter().filter(|f| f.level == FindingLevel::Error).count();
    let warnings = findings.len() - errors;
    let count = |n: usize, one: &str, other: &str| (if n == 1 { one } else { other }).replace("{n}", &n.to_string());
    labels
        .counts
        .replace("{errors}", &count(errors, &labels.error_one, &labels.error_other))
        .replace("{warnings}", &count(warnings, &labels.warning_one, &labels.warning_other))
}
