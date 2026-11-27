//! Bringing a row into view: a requested reveal. The rows view's glide does the scroll, aimed from the kept extents
//! (`rt/items.rs`, `RowsView::reveal`); an auto row it brings in takes its measured height when built, and the rows
//! after it move in the same frame's next build.

use super::ItemsHooks;
use crate::contract::Collection;
use craft_behavior::ItemLayout;

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// The item the collection was asked to reveal (`Collection::reveal`, a one-shot request of the pass): the lead
    /// moves to it and it scrolls into view.
    pub(super) fn requested_reveal(&mut self, comp: &C, layout: &C::ItemLayout) {
        if let Some(at) = comp.reveal().and_then(|key| self.index_of(key)).filter(|&at| !self.source.section_stop(at)) {
            let key = self.source.key(at);
            if self.lead.key.as_ref() != Some(&key) {
                self.lead.rested = None;
            }
            self.lead.key = Some(key);
            self.lead.at = at;
            self.reveal_with(layout, at);
        }
    }

    /// Brings row `at` into view: the collection's rows view glides to it (`revealing`, taken when the rows are
    /// built), or, without a scroll of its own, the page's scroll owner is asked at the end of the item step
    /// (`page_reveal`).
    pub(super) fn reveal_with(&mut self, layout: &C::ItemLayout, at: usize) {
        if !layout.shown(at) {
            return;
        }
        if self.scrolls {
            self.revealing = Some(at);
        } else {
            self.page_reveal = Some(at);
        }
    }
}
