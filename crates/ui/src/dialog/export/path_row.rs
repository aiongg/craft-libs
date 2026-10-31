//! [`OutputPathRow`]: where a render or export goes, as a template list and the chosen file.

use craft_host::{PickId, PickRequest};
use egui::Ui;

use crate::edit::EditResponse;

crate::words! {
    /// The row's words, English by default.
    #[non_exhaustive]
    pub struct OutputPathLabels {
        output_to: "outputPath.outputTo" = "Output To:";
        choose: "outputPath.choose" = "Choose File…";
        edit_template: "outputPath.editTemplate" = "Alt-click to edit the template";
    }
}

/// What the row reported this pass.
#[derive(Debug)]
pub struct OutputPathResponse {
    /// `Commit` with the new path or template when the user picked a template, a file, or
    /// finished editing the template text.
    pub edit: EditResponse<String>,
    /// A save picker was queued; the row fills itself when its answer arrives.
    pub browsing: Option<PickId>,
}

/// "Output To:", a list of name templates (`[Comp Name].[fileExtension]`) and the resulting file
/// name as a link button, after After Effects' Render Queue and EffectCraft's port of it.
///
/// Picking a template sets the path; a click on the file name queues `request` through
/// `host.files()` (Choose File…) and the answer replaces the path; Alt-click on the file name edits
/// the template in place (Enter commits, Esc cancels). Whether an output may be written
/// (duplicates, protected folders) is the app's check.
pub struct OutputPathRow<'a> {
    id_salt: egui::IdSalt,
    value: &'a str,
    templates: Vec<(String, String)>,
    request: PickRequest,
}

impl<'a> OutputPathRow<'a> {
    /// The row for the current path `value`; `request` is the save picker Choose File… opens.
    pub fn new(id_salt: impl egui::AsIdSalt, value: &'a str, request: PickRequest) -> Self {
        Self { id_salt: egui::IdSalt::new(id_salt), value, templates: Vec::new(), request }
    }

    /// The template list, `(template, label)`.
    pub fn templates(mut self, templates: Vec<(String, String)>) -> Self {
        self.templates = templates;
        self
    }

    // effectcraft: crates/ui-egui/src/panels/render_queue.rs:634 output_to (templates :397, Choose File… :413)
    pub fn show(self, ui: &mut Ui) -> OutputPathResponse {
        todo!()
    }
}
