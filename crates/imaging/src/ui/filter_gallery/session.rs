//! Filter Gallery's session: the effect-layer stack (task WS-FG1).

use craft_ui::dialog::DialogFields;

use super::catalog::FilterCatalog;

/// One effect layer: a filter, its fields (the catalog's parameters) and its eye.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectLayer {
    pub filter: String,
    pub fields: DialogFields,
    pub visible: bool,
}

/// A change of the stack, as the columns report it. Each is one undo step, except `Fields` during
/// a slider drag (the history settles when the drag ends).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EffectChange {
    /// A thumbnail or the Filter select picked `filter` for the effect layer at `index`: its
    /// fields start from the filter's defaults; the eye stays. Nothing changes for the same filter.
    SetFilter { index: usize, filter: String },
    /// The settings of the effect layer at `index`.
    Fields { index: usize, fields: DialogFields },
    /// New effect layer: a copy of the selected one, above it, selected.
    Add,
    /// Delete effect layer: the selected one, unless it is the last; the one below is selected.
    Delete,
    /// Move the effect layer at `from` to `to` (the arrows, ⌘] and ⌘[, a drag).
    Move { from: usize, to: usize },
    /// The eye of the effect layer at `index`.
    Visible { index: usize, visible: bool },
}

/// What OK applies and the history snapshots: the effect layers, bottom first (the top row of the
/// stack is applied last). The selected layer is view state outside the history (risk 8); undo
/// clamps it.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterGallerySession {
    pub effects: Vec<EffectLayer>,
}

impl FilterGallerySession {
    /// One effect layer of `filter` with its defaults (a new gallery: the last filter used).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG1")]
    pub fn new(filter: &str, catalog: &dyn FilterCatalog) -> Self {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:42 `EffectLayer::new`, :334 `open_on`.
        todo!()
    }

    /// Applies `change`, keeping `selected` on the layer it named (a new or moved layer is
    /// selected). Returns whether the stack changed.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG1")]
    pub fn apply(&mut self, change: EffectChange, catalog: &dyn FilterCatalog, selected: &mut usize) -> bool {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:137 `set_filter`, :144 `add`, :151
        // `delete`, :164 `move_to`.
        todo!()
    }
}
