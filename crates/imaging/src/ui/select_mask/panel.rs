//! [`SelectMaskPanel`]: Select and Mask's Properties panel (task WS-SM3).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::ui::mask_view::MaskViewState;

use super::SelectMaskLabels;
use super::edits::SelectMaskEdit;
use super::session::{RefineSettings, SelectMaskSettings};

/// What [`SelectMaskPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectMaskPanelOutput {
    pub response: Response,
    /// The refinement values changed (a session change: the history settles it, a preview is
    /// asked for).
    pub refine_changed: bool,
    /// The view or a view option changed (no undo step; a preview is asked for).
    pub view_changed: bool,
    /// A slider is being dragged.
    pub dragging: bool,
    /// Clear Selection or Invert pressed.
    pub edit: Option<SelectMaskEdit>,
}

/// The Properties panel in the Wide side column with the header "Properties", top to bottom:
///
/// - Remember Settings.
/// - **View Mode**: [`MaskViewControls`](crate::ui::mask_view::MaskViewControls) (the view, its
///   opacity, Show Edge, Show Original), then Real-Time Refinement and High Quality Preview; with
///   Overlay, Color (a swatch that asks the app through `ColorRequest`) and Indicates (Masked
///   Areas, Selected Areas).
/// - **Refine Mode**: Color Aware (on) and Object Aware, disabled with the tooltip "Object Aware
///   needs a segmentation model, which isn't available yet".
/// - **Edge Detection**: Radius (0..=250 px), Smart Radius.
/// - **Global Refinements**: Smooth (0..=100), Feather (0..=1000 px), Contrast (0..=100 %), Shift
///   Edge (−100..=100 %); buttons Clear Selection and Invert.
/// - **Output Settings**: Decontaminate Colors, Amount (0..=100 %, enabled with it), Output To
///   (the targets [`OutputTo::allowed`](super::OutputTo::allowed)).
/// - The error line (the limit message, or the preview's failure) in `status.bad.fg`, caption
///   role.
///
/// Each section is a collapsible `SectionHeader`. The footer (Reset at the leading end, OK and
/// Cancel) is the side column's, drawn by the workspace.
#[expect(dead_code, reason = "skeleton: `show` (task WS-SM3) reads the fields")]
pub struct SelectMaskPanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut SelectMaskSettings,
    refine: &'a mut RefineSettings,
    view: &'a mut MaskViewState,
    error: Option<&'a str>,
    labels: &'a SelectMaskLabels,
}

impl<'a> SelectMaskPanel<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        settings: &'a mut SelectMaskSettings,
        refine: &'a mut RefineSettings,
        view: &'a mut MaskViewState,
        labels: &'a SelectMaskLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, refine, view, error: None, labels }
    }

    /// The error line under the panel.
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-SM3")]
    pub fn show(self, ui: &mut Ui) -> SelectMaskPanelOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1540 `properties`.
        todo!()
    }
}
