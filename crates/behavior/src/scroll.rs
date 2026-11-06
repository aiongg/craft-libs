//! Scroll positions and the range a scroll part shows.

use std::ops::Range;

/// Offsets and extents in content units. They are `f64` so a sheet of a million rows scrolls exactly.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ScrollPos {
    /// Offset along the inline axis, from the start edge.
    pub x: f64,
    /// Offset along the block axis.
    pub y: f64,
}

/// The range a scroll part shows. `content` is what the part's track stands for (it may start below zero
/// for overscroll), `visible` is the thumb. A range, not a `ScrollPos`: a view value can start below zero
/// and its visible length is the user's to change.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollSpan {
    /// The extent the track stands for.
    pub content: Range<f64>,
    /// The extent the thumb covers.
    pub visible: Range<f64>,
    /// Whether the thumb ends change the visible length (zoom).
    pub resize_ends: bool,
}

impl ScrollSpan {
    /// A span whose thumb ends do not resize.
    pub const fn new(content: Range<f64>, visible: Range<f64>) -> ScrollSpan {
        ScrollSpan { content, visible, resize_ends: false }
    }

    /// Lets the thumb ends change the visible length.
    pub const fn resize_ends(mut self, on: bool) -> ScrollSpan {
        self.resize_ends = on;
        self
    }
}
