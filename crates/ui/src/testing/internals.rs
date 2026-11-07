//! Crate-private logic the trials check without a frame: the geometry of a row and the order of footer buttons.

pub use crate::list::row::RowPlacement;
use crate::list::ListRow;
use crate::sections::DialogRole;

/// Where the parts of `row` lie in a rect of `width`, under fixed sizes.
pub fn place_row(row: &ListRow<'_>, width: f32, rtl: bool) -> RowPlacement {
    crate::list::row::placement_under_fixed_sizes(row, width, rtl)
}

/// The indent steps of `row`.
pub fn row_indent_steps(row: &ListRow<'_>) -> usize {
    crate::list::row::indent_steps(row)
}

/// The indexes of `roles` in display order.
pub fn footer_order(roles: &[DialogRole], windows: bool) -> Vec<usize> {
    crate::sections::footer::display_order(roles, windows)
}
