//! The two screen axes.

/// A screen axis.
// F2-MERGE: layout owner (`Axis` beside `Flow`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Axis {
    /// Left to right.
    X,
    /// Top to bottom.
    Y,
}
