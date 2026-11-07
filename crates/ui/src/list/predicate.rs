//! The tests a `List` asks its owner: which rows rename or drag, where a drag may come from and where it may land.
//! Each is a named type over a boxed closure; a setter takes any matching closure through `From`.

use crate::contract::ListDrag;
use egui::Id;

/// Whether the row of a key passes: `List::can_rename` and `List::can_drag`.
pub struct RowTest<'a, K>(Box<dyn Fn(&K) -> bool + 'a>);

impl<'a, K, F: Fn(&K) -> bool + 'a> From<F> for RowTest<'a, K> {
    fn from(test: F) -> Self {
        RowTest(Box::new(test))
    }
}

impl<K> RowTest<'_, K> {
    pub(crate) fn passes(&self, key: &K) -> bool {
        (self.0)(key)
    }
}

/// Whether a drag that started in the list with an id may drop here: `List::accept_from`.
pub struct SourceTest<'a>(Box<dyn Fn(Id) -> bool + 'a>);

impl<'a, F: Fn(Id) -> bool + 'a> From<F> for SourceTest<'a> {
    fn from(test: F) -> Self {
        SourceTest(Box::new(test))
    }
}

impl SourceTest<'_> {
    pub(crate) fn passes(&self, from: Id) -> bool {
        (self.0)(from)
    }
}

/// Whether a drag may land at a spot, given the drag, the row under the pointer and the insert position:
/// `List::accepts`.
pub struct DropTest<'a, K>(Box<dyn Fn(&ListDrag<K>, &K, usize) -> bool + 'a>);

impl<'a, K, F: Fn(&ListDrag<K>, &K, usize) -> bool + 'a> From<F> for DropTest<'a, K> {
    fn from(test: F) -> Self {
        DropTest(Box::new(test))
    }
}

impl<K> DropTest<'_, K> {
    pub(crate) fn passes(&self, drag: &ListDrag<K>, over: &K, insert: usize) -> bool {
        (self.0)(drag, over, insert)
    }
}
