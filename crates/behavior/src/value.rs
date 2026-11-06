//! The bound every value type meets.

/// What a bound value must be: cloneable, comparable and shareable across threads.
pub trait Value: Clone + PartialEq + Send + Sync + 'static {}

impl<T: Clone + PartialEq + Send + Sync + 'static> Value for T {}
