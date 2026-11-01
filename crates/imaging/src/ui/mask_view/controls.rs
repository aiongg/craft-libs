//! [`MaskViewControls`]: the View Mode block of Select and Mask's Properties panel (task SE-F6).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task SE-F6")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::{MaskViewChange, MaskViewLabels, MaskViewState};

/// What [`MaskViewControls::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MaskViewResponse {
    /// The block's outer response.
    pub response: Response,
    /// The change the user made this pass, already applied to the state.
    pub changed: Option<MaskViewChange>,
}

/// The View Mode controls: a `Select` named "View Mode" with the seven views (each option shows its
/// key letter as its shortcut, `Kbd`), the current view's Opacity slider in percent (0..=100, only
/// for the four views that have one), the Show Edge and Show Original checkboxes, and the hint line
/// in the caption role. Remember Settings and the Refine Edge Brush controls are the workspace's.
///
/// Keyboard: each control is a Tab stop; the drop-down opens with Space or Enter. AccessKit: the
/// drop-down is a combo box with the views as options, the slider a number with "%", the
/// checkboxes are checkboxes. RTL mirrors the row.
pub struct MaskViewControls<'a> {
    id_salt: IdSalt,
    state: &'a mut MaskViewState,
    labels: MaskViewLabels,
}

impl<'a> MaskViewControls<'a> {
    /// The controls of `state`, under `id_salt`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut MaskViewState) -> Self {
        Self { id_salt: IdSalt::new(id_salt), state, labels: MaskViewLabels::default() }
    }

    /// Translated words (English by default).
    pub fn labels(mut self, labels: MaskViewLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Shows the controls in `ui`'s layout.
    pub fn show(self, ui: &mut Ui) -> MaskViewResponse {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:178-199 `controls` (drop-down, Opacity
        // slider only when the view has one, rounded, the two checkboxes, the hint).
        todo!()
    }
}
