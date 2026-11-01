//! The New Snapshot and History Options dialogs as `CommandForm`s.

use crate::dialog::{CommandForm, DialogFields, Param, ParamKind};
use crate::edit::history_panel::{HistoryOptionsLabels, SnapshotFormLabels};

/// The New Snapshot form (Alt-click on the camera, or the panel menu's New Snapshot…): a Name
/// text field that opens with `suggested` selected, and a From choice of `from`'s `(id, label)`
/// options, the first one the default (PhotoCraft: `fullDocument` Full Document, `mergedLayers`
/// Merged Layers, `currentLayer` Current Layer). An empty `from` leaves the From row out. Keys:
/// `name`, `from`. `key` is the form's [`CommandForm::key`]; read the answer with
/// [`SnapshotOptions::from_fields`].
pub fn new_snapshot_form(key: impl Into<String>, labels: &SnapshotFormLabels, suggested: &str, from: Vec<(String, String)>) -> CommandForm {
    // pc: crates/ui-egui/src/dialogs/snapshot_dialog.rs:44-67 (Name, From with three options)
    let mut params = vec![Param::new("name", &labels.name, ParamKind::Text { default: suggested.to_owned() })];
    if !from.is_empty() {
        params.push(Param::new("from", &labels.from, ParamKind::Choice { options: from, default: None }));
    }
    CommandForm::new(key, &labels.title, params)
}

/// What the New Snapshot form returned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SnapshotOptions {
    /// The name as typed, trimmed; empty means "use the suggestion" (`SnapshotList::add` with a
    /// blank name).
    pub name: String,
    /// The id of the chosen From option; `None` when the form had no From row.
    pub from: Option<String>,
}

impl SnapshotOptions {
    /// Reads the form's `name` and `from` fields.
    pub fn from_fields(fields: &DialogFields) -> SnapshotOptions {
        // new: keys of `new_snapshot_form`.
        SnapshotOptions { name: fields.text("name").unwrap_or_default().trim().to_owned(), from: fields.text("from").map(str::to_owned) }
    }
}

/// The History Options dialog's library part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct HistoryOptions {
    /// Allow Non-Linear History (`StepList::set_non_linear`).
    pub non_linear: bool,
}

impl HistoryOptions {
    /// Reads the form's `nonLinear` field; a missing field is false.
    pub fn from_fields(fields: &DialogFields) -> HistoryOptions {
        // new: Lead call 23 (key `nonLinear`; PhotoCraft maps `allowNonLinearHistory`).
        HistoryOptions { non_linear: fields.bool("nonLinear").unwrap_or(false) }
    }
}

/// The History Options form: an Allow Non-Linear History check box (key `nonLinear`) opening
/// with `options`, then the app's `extra` rows in order (PhotoCraft's Make Layer Visibility
/// Changes Undoable), whose keys the app reads from the same fields.
pub fn history_options_form(key: impl Into<String>, labels: &HistoryOptionsLabels, options: &HistoryOptions, extra: Vec<Param>) -> CommandForm {
    // pc: crates/ui-egui/src/dialogs/history_options_ui.rs:34-35 (two check boxes)
    let mut params = vec![Param::new("nonLinear", &labels.non_linear, ParamKind::Bool { default: options.non_linear })];
    params.extend(extra);
    CommandForm::new(key, &labels.title, params)
}
