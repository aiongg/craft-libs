//! The axis and size of a split in a persisted dock layout.

/// The direction in which the children of a split are placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SplitAxis {
    /// First child on the left, second child on the right.
    Horizontal,
    /// First child above the second child.
    Vertical,
}

/// A transient sizing request. Applications may adapt their own persisted sizing enum.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SplitSize {
    /// Fraction of the space left after the divider is removed, assigned to the first child.
    Ratio(f32),
    /// First child size in logical points.
    FixedFirst(f32),
    /// Second child size in logical points.
    FixedSecond(f32),
}
