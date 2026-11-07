//! The grid's outline: the tiles the app gives with a header row before each group's first tile. The item step sees
//! this source, so a header is a row it can put the lead on, and the layout places the rows by the same positions.

use super::layout::{Slot, TileGroup, group_spans, outline_position, slot_at};
use craft_behavior::{ColumnKey, DisplayOrder, ItemSource, SizeHint};
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

/// A group header as the outline holds it.
struct Header<K> {
    key: K,
    open: bool,
}

/// The tiles of `inner` with the header of every group just before the group's first tile. Header rows are sections
/// that hold the lead (`DisplayOrder::section_stop`); they open and close when the grid collapses.
pub(super) struct Outline<'s, K> {
    inner: &'s dyn ItemSource<K>,
    headers: Vec<Header<K>>,
    starts: Vec<usize>,
    collapsible: bool,
}

impl<'s, K: Clone> Outline<'s, K> {
    pub(super) fn new(inner: &'s dyn ItemSource<K>, groups: &[TileGroup<K>], collapsible: bool) -> Self {
        let spans = group_spans(groups, inner.len());
        let headers = groups.iter().map(|group| Header { key: group.key.clone(), open: group.open }).collect();
        Outline { inner, headers, starts: spans.iter().map(|span| span.start).collect(), collapsible }
    }
}

impl<K: Clone + PartialEq> DisplayOrder<K> for Outline<'_, K> {
    fn len(&self) -> usize {
        self.inner.len() + self.headers.len()
    }

    fn key(&self, at: usize) -> K {
        match slot_at(&self.starts, at) {
            Slot::Header(group) => self.headers[group].key.clone(),
            Slot::Tile(tile) => self.inner.key(tile),
        }
    }

    fn index_of(&self, key: &K) -> Option<usize> {
        match self.headers.iter().position(|header| header.key == *key) {
            Some(group) => Some(self.starts[group] + group),
            None => self.inner.index_of(key).map(|tile| outline_position(&self.starts, tile)),
        }
    }

    fn depth(&self, at: usize) -> usize {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => 0,
            Slot::Tile(tile) => self.inner.depth(tile),
        }
    }

    fn has_children(&self, at: usize) -> bool {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => self.collapsible,
            Slot::Tile(tile) => self.inner.has_children(tile),
        }
    }

    fn open(&self, at: usize) -> bool {
        match slot_at(&self.starts, at) {
            Slot::Header(group) => self.headers[group].open,
            Slot::Tile(tile) => self.inner.open(tile),
        }
    }

    fn is_section(&self, at: usize) -> bool {
        matches!(slot_at(&self.starts, at), Slot::Header(_))
    }

    fn section_stop(&self, at: usize) -> bool {
        self.is_section(at)
    }

    fn enabled(&self, at: usize) -> bool {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => true,
            Slot::Tile(tile) => self.inner.enabled(tile),
        }
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => None,
            Slot::Tile(tile) => self.inner.text(tile),
        }
    }

    fn revision(&self) -> Option<u64> {
        let inner = self.inner.revision()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (inner, &self.starts).hash(&mut hasher);
        Some(hasher.finish())
    }
}

impl<K: Clone + PartialEq> ItemSource<K> for Outline<'_, K> {
    fn size_hint(&self, at: usize) -> SizeHint {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => SizeHint::Auto,
            Slot::Tile(tile) => self.inner.size_hint(tile),
        }
    }

    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => false,
            Slot::Tile(tile) => self.inner.toggled(tile, column),
        }
    }

    fn toggle_enabled(&self, at: usize, column: &ColumnKey) -> bool {
        match slot_at(&self.starts, at) {
            Slot::Header(_) => false,
            Slot::Tile(tile) => self.inner.toggle_enabled(tile, column),
        }
    }
}
