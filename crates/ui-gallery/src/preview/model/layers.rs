//! Moving several layers at once, as the Layers list's drag asks for.

use super::{Doc, LayerId, Mock};

impl Doc {
    /// Moves the layers `items` next to `target`, before it (above, in the list) or after it,
    /// keeping their order. `target` may be one of `items`: the slot is found before any layer
    /// leaves the list. Returns whether the order changed.
    pub fn move_layers(&mut self, items: &[LayerId], target: LayerId, after: bool) -> bool {
        let Some(at) = self.index_of(target) else { return false };
        let slot = if after { at + 1 } else { at };
        let leaving_before = self.layers[..slot].iter().filter(|l| items.contains(&l.id)).count();
        let old: Vec<LayerId> = self.layers.iter().map(|l| l.id).collect();
        let (moved, mut rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.layers).into_iter().partition(|l| items.contains(&l.id));
        let slot = slot - leaving_before;
        rest.splice(slot..slot, moved);
        self.layers = rest;
        let changed = self.layers.iter().map(|l| l.id).ne(old);
        self.modified |= changed;
        changed
    }
}

impl Mock {
    pub fn reorder_layers(&mut self, items: &[LayerId], target: LayerId, after: bool) {
        if self.doc.move_layers(items, target, after) {
            self.record("Move layer");
        }
    }

    /// Sets a layer's visibility; a layer that already has it records nothing.
    pub fn set_layer_visible(&mut self, id: LayerId, visible: bool) {
        if self.doc.layer(id).is_some_and(|l| l.visible != visible) {
            self.toggle_visible(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(doc: &Doc) -> Vec<LayerId> {
        doc.layers.iter().map(|l| l.id).collect()
    }

    #[test]
    fn layers_gather_at_the_target_in_their_order() {
        let mut doc = Doc::default();
        assert_eq!(ids(&doc), [3, 2, 1]);
        assert!(doc.move_layers(&[3], 1, true));
        assert_eq!(ids(&doc), [2, 1, 3]);
        assert!(doc.move_layers(&[3, 1], 2, false));
        assert_eq!(ids(&doc), [1, 3, 2]);
        assert!(!doc.move_layers(&[1], 1, true), "dropping a layer on itself changes nothing");
    }
}
