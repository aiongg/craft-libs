#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`PathField`]: a text field holding a path with a Browse button that opens a picker through
//! `host.files()`.

use std::path::PathBuf;

use craft_host::{PickId, PickRequest};
use craft_ui::edit::EditResponse;
use egui::{AsIdSalt, IdSalt, Ui};

craft_ui::words! {
    /// The field's words, English by default.
    #[non_exhaustive]
    pub struct PathFieldLabels {
        browse: "pathField.browse" = "Browse…";
        not_found: "pathField.notFound" = "The file was not found.";
    }
}

/// What the field reported this pass.
#[derive(Debug)]
pub struct PathFieldResponse {
    /// The field's edit: `Commit` when the user typed a path and left the field or pressed Enter,
    /// or when the picker answered.
    pub edit: EditResponse<String>,
    /// Browse was pressed and a picker was queued; the field stores the ticket in egui memory and
    /// fills itself when the answer arrives.
    pub browsing: Option<PickId>,
}

/// A label, a text field with the path and a Browse button in one row. Typing commits on Enter or
/// focus loss; Browse queues `request` through `host.files()` and the answer fills the field on
/// a later pass (the field matches tickets, so several fields can browse in turn). Tab order: field,
/// then Browse. The field shows [`PathFieldLabels::not_found`] under itself in error red when
/// `must_exist` is set and the path does not exist.
///
/// ```ignore
/// let r = PathField::new("export-path", "Save to", &path, PickRequest::save("export").suggested_extension("png")).show(ui);
/// if let Some(p) = r.edit.committed() { path = p; }
/// ```
pub struct PathField<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a str,
    request: PickRequest,
    must_exist: bool,
    width: Option<f32>,
    enabled: bool,
}

impl<'a> PathField<'a> {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a str, request: PickRequest) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), value, request, must_exist: false, width: None, enabled: true }
    }

    /// Show the not-found message for a path that does not exist (Open fields; not Save).
    pub fn must_exist(mut self, must_exist: bool) -> Self {
        self.must_exist = must_exist;
        self
    }

    /// The text field's width; default: fill, leaving room for Browse.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the row. Reads `Host::of(ui).files()`: takes a pick answer whose ticket this field
    /// stored, returns any other answer with `return_pick`.
    // pc: crates/ui-egui/src/dialogs/file_ui/print.rs:246 browse_pdf (Browse beside the PDF path field); new: the field itself
    pub fn show(self, ui: &mut Ui) -> PathFieldResponse {
        todo!()
    }

    /// The path a picker answer gives the field: the first path, or on wasm the file name.
    // new: wasm answers carry contents, not paths
    fn path_of(answer: &craft_host::Picked) -> Option<PathBuf> {
        todo!()
    }
}
