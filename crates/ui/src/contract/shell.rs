//! The props and hooks the item step reads.
//!
//! F3-LATER: the item step (F6). `CollectionProps` carries the collection's key-independent settings.



/// What a collection reads from the app: plain data.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CollectionProps {}

impl CollectionProps {
    /// Collection props with every setting at its default.
    pub const fn new() -> Self {
        CollectionProps {}
    }
}
