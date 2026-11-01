//! [`FillBar`]: Content-Aware Fill's options bar (task WS-CA2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::FillLabels;
use super::lasso::FillAreaEdit;
use super::tool::{FillTool, FillToolOptions};

/// What [`FillBar::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillBarOutput {
    pub response: Response,
    /// Expand or Contract pressed.
    pub edit: Option<FillAreaEdit>,
    /// The Lasso's Reset pressed (`FillReset::FillArea`).
    pub reset_fill_area: bool,
}

/// The options bar, full width under the title (lead call 4; PhotoCraft's started beside the
/// strip): the tool's icon, then
///
/// - Sampling Brush: + and − (Add, Subtract), Size (1..=5000 px);
/// - Lasso: the four operations (New, Add with the Shift hint, Subtract with Alt, Intersect with
///   Shift+Alt, the hints in the platform's key names), Expand, Contract, the amount (1..=100 px)
///   and Reset (an icon button).
#[expect(dead_code, reason = "skeleton: `show` (task WS-CA2) reads the fields")]
pub struct FillBar<'a> {
    id_salt: IdSalt,
    tool: FillTool,
    options: &'a mut FillToolOptions,
    labels: &'a FillLabels,
}

impl<'a> FillBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, tool: FillTool, options: &'a mut FillToolOptions, labels: &'a FillLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), tool, options, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn show(self, ui: &mut Ui) -> FillBarOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:33 `caf_options_bar`.
        todo!()
    }
}
