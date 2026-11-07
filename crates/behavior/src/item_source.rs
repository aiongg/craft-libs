//! Where a collection's items come from.

/// The items a collection shows, by position.
// F3-LATER: craft-behavior `ItemSource` (size hints, key lookup and change reporting are F6)
pub trait ItemSource<K> {
    /// The number of items.
    fn len(&self) -> usize;

    /// The key of item `at`.
    fn key(&self, at: usize) -> K;

    /// Whether there are no items.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
