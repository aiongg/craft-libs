//! [`FindingsList`]: the problems a check found (preflight, export warnings), each one a row that
//! takes the user to what it is about.

use egui::{Context, Sense, Ui, vec2};

use super::{List, ListEvent, ListRow};
use crate::behavior::{RowInfo, SelectionMode};
use crate::icons::Icon;
use crate::localized::words::counts;
use crate::localized::{SharedLabels as _, Words};
use crate::primitives::Text;
use craft_theme::TypeRole;

use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

crate::words! {
    /// The list's words, English by default.
    #[non_exhaustive]
    pub struct FindingsLabels {
        no_errors: "findings.noErrors" = "No errors";
        /// `{errors}` and `{warnings}`: the counts, already pluralised by the catalog.
        counts: "findings.counts" = "{errors}, {warnings}";
        /// The plural forms of a count word; `{n}` is the count.
        errors: "findings.errors" = "{n} errors";
        warnings: "findings.warnings" = "{n} warnings";
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
                let radius = tokens.component.findings.status_dot;
                let (dot, _) = ui.allocate_exact_size(vec2(radius * 2.0 + 1.0, radius * 2.0 + 1.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), radius, color.to_egui());
                ui.add(Text::new(findings_summary(ui.ctx(), findings, &labels)).role(TypeRole::Body));
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
            let page = finding.page.as_ref().map(|page| craft_i18n::fmt(&labels.page, &[("page", page.as_str())]));
            let name = match &page {
                Some(page) => format!("{level}: {}, {page}", finding.message),
                None => format!("{level}: {}", finding.message),
            };
            let row = ListRow::new(key, finding.message.clone()).icon(icon).accessible_name(name);
            match page {
                Some(page) => row.detail(page),
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

/// The summary text: "No errors" for an empty list, else both counts in the plural forms and
/// digits of `ctx`'s words, zeros included ("0 errors, 3 warnings", "1 error, 2 warnings").
// designcraft: crates/ui-egui/src/panels/properties/links.rs:275 (summary label)
pub fn findings_summary(ctx: &Context, findings: &[Finding], labels: &FindingsLabels) -> String {
    if findings.is_empty() {
        return labels.no_errors.clone();
    }
    let errors = findings.iter().filter(|f| f.level == FindingLevel::Error).count();
    let warnings = findings.len() - errors;
    let words = Words::of_ctx(ctx);
    let errors = words.counted(ctx, counts::ERRORS, &labels.errors, errors as u64);
    let warnings = words.counted(ctx, counts::WARNINGS, &labels.warnings, warnings as u64);
    craft_i18n::fmt(&labels.counts, &[("errors", errors.as_str()), ("warnings", warnings.as_str())])
}
