//! [`EffectStack`]: the effect-layer stack under the settings (task WS-FG2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::FilterGalleryLabels;
use super::session::{EffectChange, EffectLayer};

/// What [`EffectStack::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EffectStackResponse {
    pub response: Response,
    /// The row picked (click, Up, Down).
    pub selected: Option<usize>,
    /// An eye, a move, New or Delete.
    pub change: Option<EffectChange>,
}

/// The stack (lead call 39): a library `List` 190 high, top row last applied (so the rows show
/// `effects` in reverse), each row 24 high with an eye column (`ToggleColumn::eye`) and the
/// filter's name; the selected row is the list's selection. Under it a `PanelFooter` with Move
/// up, Move down, New effect layer, Delete effect layer (disabled with one layer). Rows reorder by
/// drag (`Reorder`).
///
/// Keyboard: Up and Down select, Space toggles the eye, ⌘] and ⌘[ move the row. AccessKit: a
/// list; rows named by their filter with the eye as a toggle.
#[expect(dead_code, reason = "skeleton: `show` (task WS-FG2) reads the fields")]
pub struct EffectStack<'a> {
    id_salt: IdSalt,
    effects: &'a [EffectLayer],
    selected: usize,
    labels_of: &'a dyn Fn(&str) -> String,
    labels: &'a FilterGalleryLabels,
}

impl<'a> EffectStack<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        effects: &'a [EffectLayer],
        selected: usize,
        labels_of: &'a dyn Fn(&str) -> String,
        labels: &'a FilterGalleryLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), effects, selected, labels_of, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn show(self, ui: &mut Ui) -> EffectStackResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:780, :832-881 (list 190 high, rows 24 with the eye,
        // the four buttons).
        todo!()
    }
}
