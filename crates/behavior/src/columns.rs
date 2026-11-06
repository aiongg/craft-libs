//! Column keys, widths and the sections a grid layout reports.

use emath::Rect;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::ops::Range;

/// The key of a column; stable across reordering and resizing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColumnKey(Cow<'static, str>);

impl ColumnKey {
    /// A key named `key`.
    pub const fn new(key: &'static str) -> Self {
        ColumnKey(Cow::Borrowed(key))
    }
}

/// How wide a column is.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ColumnWidth {
    /// The width of the widest cell.
    Content,
    /// The app's width.
    Fixed(f32),
    /// A share of the remaining width, by weight.
    #[non_exhaustive]
    Fill {
        /// The column's weight among the `Fill` columns.
        weight: f32,
    },
}

impl ColumnWidth {
    /// A share of the remaining width with `weight` among the `Fill` columns.
    pub const fn fill(weight: f32) -> Self {
        Self::Fill { weight }
    }
}

/// One horizontal section of a grid layout: its rect, the columns it shows and their edges.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GridSection {
    /// Relative to the viewport, like every rect of an `ItemLayout`.
    pub rect: Rect,
    /// The columns the section shows.
    pub columns: Range<u32>,
    /// The x of each column edge.
    pub edges: Vec<f32>,
}

impl GridSection {
    /// A section at `rect` showing `columns` with their edges.
    pub fn new(rect: Rect, columns: Range<u32>, edges: Vec<f32>) -> Self {
        Self { rect, columns, edges }
    }
}

/// The sections of a sheet layout, one per frozen area.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GridView {
    /// The sections, in reading order.
    pub sections: SmallVec<[GridSection; 4]>,
}

impl GridView {
    /// A view of `sections`, in reading order.
    pub fn new(sections: impl IntoIterator<Item = GridSection>) -> Self {
        Self { sections: sections.into_iter().collect() }
    }
}
