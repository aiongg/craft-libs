//! Navigation shapes: how arrow keys move the lead through a collection.

/// The geometry a collection's keyboard navigation follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavShape {
    /// One row after another.
    Flat,
    /// Rows that expand and collapse.
    Tree,
    /// Equal-width cells that wrap at `columns`.
    #[non_exhaustive]
    WrappedGrid {
        /// Cells per row.
        columns: usize,
    },
    /// Rows of different lengths.
    RaggedGrid,
    /// Cells of a sheet.
    Cell,
    /// Items with free positions; arrows go to the nearest item in that direction.
    Spatial,
}

impl NavShape {
    /// Equal-width cells that wrap at `columns`.
    pub const fn wrapped_grid(columns: usize) -> Self {
        Self::WrappedGrid { columns }
    }
}
