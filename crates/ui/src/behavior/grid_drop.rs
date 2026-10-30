//! [`grid_drop`]: which cell of a wrapped grid a pointer is over during a drag, and whether the
//! drop lands before or after it.

use egui::{Pos2, Rect, Vec2, pos2, vec2};

use super::DropPosition;

/// A wrapped grid of equal cells laid out in reading order from a leading corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGeometry {
    /// The top corner of the first cell on its leading side: the top left, or the top right when
    /// [`rtl`](GridGeometry::rtl).
    pub origin: Pos2,
    /// The size of a cell.
    pub cell: Vec2,
    /// The space between two cells across and down.
    pub gap: Vec2,
    /// How many cells fit in a row (at least one).
    pub columns: usize,
    /// How many cells there are.
    pub len: usize,
    /// Right-to-left reading: cells run from the right.
    pub rtl: bool,
}

impl GridGeometry {
    /// How many rows the cells fill.
    pub fn rows(&self) -> usize {
        self.len.div_ceil(self.columns.max(1))
    }

    /// The rect of cell `index`.
    pub fn cell_rect(&self, index: usize) -> Rect {
        let columns = self.columns.max(1);
        let (row, column) = ((index / columns) as f32, (index % columns) as f32);
        let across = column * (self.cell.x + self.gap.x);
        let x = if self.rtl { self.origin.x - across - self.cell.x } else { self.origin.x + across };
        Rect::from_min_size(pos2(x, self.origin.y + row * (self.cell.y + self.gap.y)), self.cell)
    }

    /// The rect of all the rows, the gap below the last row left out.
    pub fn bounds(&self) -> Rect {
        let rows = self.rows();
        if rows == 0 {
            return Rect::from_min_size(self.origin, Vec2::ZERO);
        }
        let used = self.columns.max(1).min(self.len) as f32;
        let width = used * self.cell.x + (used - 1.0) * self.gap.x;
        let height = rows as f32 * self.cell.y + (rows as f32 - 1.0) * self.gap.y;
        let x = if self.rtl { self.origin.x - width } else { self.origin.x };
        Rect::from_min_size(pos2(x, self.origin.y), vec2(width, height))
    }
}

/// The cell a drop lands next to, and the side of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridDrop {
    /// The cell the drop is relative to.
    pub index: usize,
    /// [`Before`](DropPosition::Before) or [`After`](DropPosition::After) it, in reading order.
    pub position: DropPosition,
}

/// Where a drag at `pointer` drops on `grid`:
///
/// - the leading half of a cell is `Before` it, the trailing half (and the gap after it) `After`,
///   leading and trailing being by the reading direction;
/// - left of the first cell of a row is `Before` that cell;
/// - past the last cell of a row, or of the grid, is `After` that cell, so the end of a wrapped
///   row targets its last cell;
/// - a pointer above the first row or below the last, where another row of the host's list
///   begins, is `None`; a gap between two rows belongs to the row above.
pub fn grid_drop(grid: &GridGeometry, pointer: Pos2) -> Option<GridDrop> {
    let columns = grid.columns.max(1);
    if grid.len == 0 || pointer.y < grid.origin.y {
        return None;
    }
    let row_pitch = (grid.cell.y + grid.gap.y).max(f32::EPSILON);
    let row = ((pointer.y - grid.origin.y) / row_pitch).floor();
    if !row.is_finite() || row as usize >= grid.rows() {
        return None;
    }
    let row = row as usize;
    let first = row * columns;
    let in_row = (grid.len - first).min(columns);
    let across = if grid.rtl { grid.origin.x - pointer.x } else { pointer.x - grid.origin.x };
    if across < 0.0 {
        return Some(GridDrop { index: first, position: DropPosition::Before });
    }
    let pitch = (grid.cell.x + grid.gap.x).max(f32::EPSILON);
    let column = (across / pitch).floor();
    if !column.is_finite() || column as usize >= in_row {
        return Some(GridDrop { index: first + in_row - 1, position: DropPosition::After });
    }
    let column = column as usize;
    let into_cell = across - column as f32 * pitch;
    let position = if into_cell < grid.cell.x / 2.0 { DropPosition::Before } else { DropPosition::After };
    Some(GridDrop { index: first + column, position })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rtl: bool, len: usize) -> GridGeometry {
        // Cells 10 by 10, 2 apart, three to a row, from x = 100 (or leading at x = 100 for rtl).
        GridGeometry { origin: pos2(100.0, 50.0), cell: vec2(10.0, 10.0), gap: vec2(2.0, 2.0), columns: 3, len, rtl }
    }

    fn at(grid: &GridGeometry, x: f32, y: f32) -> Option<(usize, DropPosition)> {
        grid_drop(grid, pos2(x, y)).map(|d| (d.index, d.position))
    }

    #[test]
    fn grid_drop_halves_of_a_cell() {
        let g = grid(false, 7);
        assert_eq!(at(&g, 102.0, 55.0), Some((0, DropPosition::Before)));
        assert_eq!(at(&g, 108.0, 55.0), Some((0, DropPosition::After)));
        assert_eq!(at(&g, 111.0, 55.0), Some((0, DropPosition::After)), "the gap follows its cell");
        assert_eq!(at(&g, 125.0, 55.0), Some((2, DropPosition::Before)));
        assert_eq!(at(&g, 113.0, 61.0), Some((1, DropPosition::Before)), "the gap below a row is that row's");
        assert_eq!(at(&g, 113.0, 72.0), Some((4, DropPosition::Before)), "second row");
    }

    #[test]
    fn grid_drop_row_ends_and_outside() {
        let g = grid(false, 7);
        assert_eq!(at(&g, 150.0, 55.0), Some((2, DropPosition::After)), "past a wrapped row's end");
        assert_eq!(at(&g, 150.0, 80.0), Some((6, DropPosition::After)), "past the last cell");
        assert_eq!(at(&g, 130.0, 80.0), Some((6, DropPosition::After)), "where the last row has no cell");
        assert_eq!(at(&g, 90.0, 55.0), Some((0, DropPosition::Before)), "left of a row");
        assert_eq!(at(&g, 90.0, 62.0), Some((3, DropPosition::Before)));
        assert_eq!(at(&g, 105.0, 49.0), None, "above");
        assert_eq!(at(&g, 105.0, 200.0), None, "below");
        assert_eq!(grid_drop(&grid(false, 0), pos2(105.0, 55.0)), None);
    }

    #[test]
    fn grid_drop_rtl_leads_from_the_right() {
        let g = grid(true, 7);
        assert_eq!(at(&g, 98.0, 55.0), Some((0, DropPosition::Before)), "the right half is the leading half");
        assert_eq!(at(&g, 92.0, 55.0), Some((0, DropPosition::After)));
        assert_eq!(at(&g, 110.0, 55.0), Some((0, DropPosition::Before)), "right of the first cell");
        assert_eq!(at(&g, 60.0, 55.0), Some((2, DropPosition::After)), "past the row's end on the left");
    }

    #[test]
    fn cell_rects_and_bounds() {
        let g = grid(false, 7);
        assert_eq!(g.cell_rect(4), Rect::from_min_size(pos2(112.0, 62.0), vec2(10.0, 10.0)));
        assert_eq!(g.bounds(), Rect::from_min_size(pos2(100.0, 50.0), vec2(34.0, 34.0)));
        let r = grid(true, 7);
        assert_eq!(r.cell_rect(1), Rect::from_min_size(pos2(78.0, 50.0), vec2(10.0, 10.0)));
        assert_eq!(r.cell_rect(0).max.x, 100.0);
        assert_eq!(grid(false, 2).bounds().width(), 22.0);
    }
}
