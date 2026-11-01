//! [`SelectMaskBar`]: Select and Mask's options bar (task WS-SM2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::SelectMaskLabels;
use super::tools::{SelectMaskTool, SelectMaskToolOptions};

/// What [`SelectMaskBar::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectMaskBarOutput {
    pub response: Response,
    /// An option changed (written to the options). Sample All Layers asks for a new preview.
    pub changed: bool,
    /// Select Subject pressed: the workspace records `SelectMaskEdit::Subject`.
    pub select_subject: bool,
}

/// The options bar, full width under the title (`OptionsBar`): the current tool's icon; for the
/// editing tools + and − (Add, Subtract); for the brush tools Size (1..=5000 px, a
/// `NumberField`); for the Brush Hardness (0..=100 %); for Quick Selection, Refine Edge Brush and
/// Object Selection Sample All Layers; for the editing tools Select Subject and Refine Hair, drawn
/// disabled with the tooltip "Refine Hair needs hair detection, which isn't available yet" (lead
/// call 28).
#[expect(dead_code, reason = "skeleton: `show` (task WS-SM2) reads the fields")]
pub struct SelectMaskBar<'a> {
    id_salt: IdSalt,
    tool: SelectMaskTool,
    options: &'a mut SelectMaskToolOptions,
    labels: &'a SelectMaskLabels,
}

impl<'a> SelectMaskBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, tool: SelectMaskTool, options: &'a mut SelectMaskToolOptions, labels: &'a SelectMaskLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, options, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-SM2")]
    pub fn show(self, ui: &mut Ui) -> SelectMaskBarOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1497 `options_bar`.
        todo!()
    }
}
