//! Where a collection's items come from: the data traits and the one way to show a slice.

use crate::columns::ColumnKey;
use crate::item_layout::ItemLayout;
use crate::nav::NavOutline;
use emath::Vec2;
use std::borrow::Cow;

/// How tall (or what shape) an item wants to be.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SizeHint {
    /// Measured by the item at its own step 5.
    Auto,
    /// A height the app gives, in points.
    Height(f32),
    /// Width over height; the layout picks the width.
    Aspect(f32),
    /// A size the app gives, in points.
    Rect(Vec2),
}

/// The order a collection shows its entries in. The library never owns the data.
pub trait DisplayOrder<K> {
    /// The number of entries.
    fn len(&self) -> usize;

    /// The key of entry `at`. Items take their id from it, never from the position.
    fn key(&self, at: usize) -> K;

    /// The position of `key`, `None` when the source no longer holds it. The default scans; a source over a
    /// large collection answers from its own index.
    fn index_of(&self, key: &K) -> Option<usize>
    where
        K: PartialEq,
    {
        (0..self.len()).find(|&at| self.key(at) == *key)
    }

    /// The nesting depth of entry `at`, 0 for a root.
    fn depth(&self, _at: usize) -> usize {
        0
    }

    /// Whether entry `at` has children.
    fn has_children(&self, _at: usize) -> bool {
        false
    }

    /// Whether the children of entry `at` show.
    fn open(&self, _at: usize) -> bool {
        false
    }

    /// Whether entry `at` is a section header, which navigation skips.
    fn is_section(&self, _at: usize) -> bool {
        false
    }

    /// Whether entry `at` can take the lead.
    fn enabled(&self, _at: usize) -> bool {
        true
    }

    /// The text type-ahead matches entry `at` against.
    fn text(&self, _at: usize) -> Option<Cow<'_, str>> {
        None
    }

    /// A number that changes whenever the entries or their order change; `None` when the source does not report.
    fn revision(&self) -> Option<u64> {
        None
    }

    /// Whether there are no entries.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A display order that also says how tall its entries want to be.
pub trait ItemSource<K>: DisplayOrder<K> {
    /// The size entry `at` wants.
    fn size_hint(&self, _at: usize) -> SizeHint {
        SizeHint::Auto
    }

    /// Whether the cell of entry `at` in the toggle column `column` is on.
    fn toggled(&self, _at: usize, _column: &ColumnKey) -> bool {
        false
    }
}

/// `DisplayOrder` as the key-free outline the navigation core steps over.
pub struct Outline<'a, K> {
    order: &'a dyn DisplayOrder<K>,
    columns: usize,
    layout: Option<&'a dyn ItemLayout>,
}

impl<'a, K> Outline<'a, K> {
    /// The outline of `order`.
    pub fn new(order: &'a dyn DisplayOrder<K>) -> Self {
        Outline { order, columns: 1, layout: None }
    }

    /// The rows and hidden items of `layout`, for ragged-grid navigation.
    #[must_use]
    pub fn layout(mut self, layout: &'a dyn ItemLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// The cells of every position, for a sheet's cell navigation.
    #[must_use]
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }
}

impl<K> NavOutline for Outline<'_, K> {
    fn len(&self) -> usize {
        self.order.len()
    }

    fn depth(&self, at: usize) -> usize {
        self.order.depth(at)
    }

    fn has_children(&self, at: usize) -> bool {
        self.order.has_children(at)
    }

    fn open(&self, at: usize) -> bool {
        self.order.open(at)
    }

    fn is_section(&self, at: usize) -> bool {
        self.order.is_section(at)
    }

    fn enabled(&self, at: usize) -> bool {
        self.order.enabled(at) && self.layout.is_none_or(|layout| layout.shown(at))
    }

    fn row_len(&self, row: usize) -> Option<usize> {
        self.layout.and_then(|layout| layout.row_len(row))
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

type KeyFn<'a, T, K> = Box<dyn Fn(&T) -> K + 'a>;
type TextFn<'a, T> = Box<dyn for<'t> Fn(&'t T) -> Cow<'t, str> + 'a>;

/// A slice as an item source, each entry keyed by `key`.
pub struct Keyed<'a, T, K> {
    items: &'a [T],
    key: KeyFn<'a, T, K>,
    text: Option<TextFn<'a, T>>,
}

/// Shows `items` keyed by `key`: the one way to show a slice (7.1 rule 4).
pub fn keyed<'a, T, K>(items: &'a [T], key: impl Fn(&T) -> K + 'a) -> Keyed<'a, T, K> {
    Keyed { items, key: Box::new(key), text: None }
}

impl<'a, T, K> Keyed<'a, T, K> {
    /// The text type-ahead matches each entry against.
    #[must_use]
    pub fn with_text(mut self, text: impl for<'t> Fn(&'t T) -> Cow<'t, str> + 'a) -> Self {
        self.text = Some(Box::new(text));
        self
    }
}

impl<T, K> DisplayOrder<K> for Keyed<'_, T, K> {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn key(&self, at: usize) -> K {
        (self.key)(&self.items[at])
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        self.text.as_ref().map(|text| text(&self.items[at]))
    }
}

impl<T, K> ItemSource<K> for Keyed<'_, T, K> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keyed_slice_keys_by_the_closure_and_finds_keys_again() {
        let rows = [(7_u32, "b"), (3, "a")];
        let source = keyed(&rows, |row| row.0).with_text(|row| Cow::Borrowed(row.1));
        assert_eq!((source.key(0), source.key(1)), (7, 3));
        assert_eq!(source.index_of(&3), Some(1));
        assert_eq!(source.index_of(&9), None);
        assert_eq!(DisplayOrder::text(&source, 1).as_deref(), Some("a"));
    }
}
