//! Filter Gallery's session: the effect-layer stack.

use craft_ui::dialog::DialogFields;

use super::catalog::FilterCatalog;

/// One effect layer: a filter, its fields (the catalog's parameters) and its eye.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectLayer {
    /// The filter's key in the catalog.
    pub filter: String,
    /// The filter's parameter values.
    pub fields: DialogFields,
    /// Whether the layer's eye is on.
    pub visible: bool,
}

impl EffectLayer {
    /// A visible layer of `filter` with the catalog's defaults (none for a key the catalog lacks).
    fn of(filter: &str, catalog: &dyn FilterCatalog) -> Self {
        let fields = catalog.filter(filter).map(|found| found.defaults()).unwrap_or_default();
        Self { filter: filter.to_owned(), fields, visible: true }
    }
}

/// A change of the stack, as the columns report it. Each is one undo step, except `Fields` during
/// a slider drag (the history settles when the drag ends).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EffectChange {
    /// A thumbnail or the Filter select picked `filter` for the effect layer at `index`: its
    /// fields start from the filter's defaults; the eye stays. Nothing changes for the same filter.
    SetFilter {
        #[allow(missing_docs)]
        index: usize,
        #[allow(missing_docs)]
        filter: String,
    },
    /// The settings of the effect layer at `index`.
    Fields {
        #[allow(missing_docs)]
        index: usize,
        #[allow(missing_docs)]
        fields: DialogFields,
    },
    /// New effect layer: a copy of the selected one, above it, selected.
    Add,
    /// Delete effect layer: the selected one, unless it is the last; the one below is selected.
    Delete,
    /// Move the effect layer at `from` to `to` (the arrows, ⌘] and ⌘[, a drag).
    Move {
        #[allow(missing_docs)]
        from: usize,
        #[allow(missing_docs)]
        to: usize,
    },
    /// The eye of the effect layer at `index`.
    Visible {
        #[allow(missing_docs)]
        index: usize,
        #[allow(missing_docs)]
        visible: bool,
    },
}

/// What OK applies and the history snapshots: the effect layers, bottom first (the top row of the
/// stack is applied last). The selected layer is view state outside the history (risk 8); undo
/// clamps it.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterGallerySession {
    /// The effect layers, bottom first.
    pub effects: Vec<EffectLayer>,
}

impl FilterGallerySession {
    /// One effect layer of `filter` with its defaults (a new gallery: the last filter used).
    pub fn new(filter: &str, catalog: &dyn FilterCatalog) -> Self {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:42 `EffectLayer::new`, :334 `open_on`.
        Self { effects: vec![EffectLayer::of(filter, catalog)] }
    }

    /// Applies `change`, keeping `selected` on the layer it named (a new or moved layer is
    /// selected). Returns whether the stack changed.
    pub fn apply(&mut self, change: EffectChange, catalog: &dyn FilterCatalog, selected: &mut usize) -> bool {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:137 `set_filter`, :144 `add`, :151
        // `delete`, :164 `move_to`.
        let count = self.effects.len();
        if count == 0 {
            return false;
        }
        let current = (*selected).min(count - 1);
        let changed = match change {
            EffectChange::SetFilter { index, filter } => match self.effects.get_mut(index) {
                Some(effect) if effect.filter != filter && catalog.filter(&filter).is_some() => {
                    *effect = EffectLayer { visible: effect.visible, ..EffectLayer::of(&filter, catalog) };
                    true
                }
                _ => false,
            },
            EffectChange::Fields { index, fields } => match self.effects.get_mut(index) {
                Some(effect) if effect.fields != fields => {
                    effect.fields = fields;
                    true
                }
                _ => false,
            },
            EffectChange::Add => {
                let copy = self.effects[current].clone();
                self.effects.insert(current + 1, copy);
                *selected = current + 1;
                true
            }
            EffectChange::Delete if count > 1 => {
                self.effects.remove(current);
                *selected = current.saturating_sub(1);
                true
            }
            EffectChange::Move { from, to } if from < count && to < count && from != to => {
                let effect = self.effects.remove(from);
                self.effects.insert(to, effect);
                *selected = to;
                true
            }
            EffectChange::Visible { index, visible } => match self.effects.get_mut(index) {
                Some(effect) if effect.visible != visible => {
                    effect.visible = visible;
                    true
                }
                _ => false,
            },
            _ => false,
        };
        *selected = (*selected).min(self.effects.len() - 1);
        changed
    }
}
