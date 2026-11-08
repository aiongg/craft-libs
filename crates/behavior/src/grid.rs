//! The `Grid` cursor: columns resolved from last pass's measurements, and the cell each child takes.

use crate::columns::ColumnWidth;
use crate::flow::Flow;
use emath::{Pos2, Rect, Vec2};

/// The width of each column in `room`: `Fixed` is the app's width, `Content` the widest child of last pass
/// (`content[i]`, zero when unknown), and `Fill` columns share what is left by weight. `Controls` widths take
/// no room until `ColumnWidth::in_unit` resolves them.
pub fn resolve_columns(columns: &[ColumnWidth], content: &[f32], room: f32, gap: f32) -> Vec<f32> {
    let known = |i: usize| content.get(i).copied().unwrap_or(0.0);
    let mut widths: Vec<f32> = columns
        .iter()
        .enumerate()
        .map(|(i, column)| match column {
            ColumnWidth::Fixed(width) => *width,
            ColumnWidth::Fill { .. } | ColumnWidth::Controls(_) => 0.0,
            _ => known(i),
        })
        .collect();
    let gaps = gap * columns.len().saturating_sub(1) as f32;
    let weights: f32 = columns.iter().map(|column| if let ColumnWidth::Fill { weight } = column { *weight } else { 0.0 }).sum();
    let rest = (room - widths.iter().sum::<f32>() - gaps).max(0.0);
    if weights > 0.0 {
        for (width, column) in widths.iter_mut().zip(columns) {
            if let ColumnWidth::Fill { weight } = column {
                *width = rest * weight / weights;
            }
        }
    }
    widths
}

/// What a `Grid` pass measured: the widest child per column and the tallest per row.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct GridMeasure {
    /// The widest child of each column.
    pub columns: Vec<f32>,
    /// The tallest child of each row.
    pub rows: Vec<f32>,
}

impl GridMeasure {
    /// A measurement of `columns` and `rows`.
    pub const fn new(columns: Vec<f32>, rows: Vec<f32>) -> Self {
        GridMeasure { columns, rows }
    }
}

/// Hands out the cells of one `Grid` pass, row-major. Cells are placed from `last` pass's measurements; this
/// pass's own are collected in `measured`.
#[derive(Clone, Debug)]
pub struct GridCursor {
    flow: Flow,
    columns: Vec<ColumnWidth>,
    gap: Vec2,
    widths: Vec<f32>,
    last_rows: Vec<f32>,
    next: usize,
    measured: GridMeasure,
}

impl GridCursor {
    /// A cursor over `columns` in `flow`'s rect, with `gap` between columns (x) and rows (y), placed by `last`.
    pub fn new(flow: Flow, columns: &[ColumnWidth], gap: Vec2, last: &GridMeasure) -> Self {
        let widths = resolve_columns(columns, &last.columns, flow.rect().width(), gap.x);
        GridCursor { flow, columns: columns.to_vec(), gap, widths, last_rows: last.rows.clone(), next: 0, measured: GridMeasure::default() }
    }

    /// The cell of the next child, whose measured size is `size`.
    pub fn cell(&mut self, size: Vec2) -> Rect {
        let count = self.columns.len().max(1);
        let (column, row) = (self.next % count, self.next / count);
        self.next += 1;
        self.measured.columns.resize(count, 0.0);
        self.measured.columns[column] = self.measured.columns[column].max(size.x);
        if self.measured.rows.len() <= row {
            self.measured.rows.resize(row + 1, 0.0);
        }
        self.measured.rows[row] = self.measured.rows[row].max(size.y);
        let start: f32 = self.widths.iter().take(column).map(|width| width + self.gap.x).sum();
        let top: f32 = self.last_rows.iter().take(row).map(|height| height + self.gap.y).sum();
        // A `Content` column is at least as wide as the cell placed in it, so a column no pass has measured yet takes its
        // first cell's width and the cells after it are placed past it.
        if let (Some(ColumnWidth::Content), Some(width)) = (self.columns.get(column), self.widths.get_mut(column)) {
            *width = width.max(size.x);
        }
        let width = self.widths.get(column).copied().unwrap_or(size.x);
        let height = self.last_rows.get(row).copied().unwrap_or(size.y);
        let span = self.flow.span(start, width);
        Rect::from_min_size(Pos2::new(span.min.x, self.flow.rect().top() + top), Vec2::new(width, height))
    }

    /// The room the next child measures in: as wide as its column, or for a `Content` column as wide as the columns
    /// that do not fill leave, and as tall as the grid below its row's top. The cells placed before it do not shorten
    /// it, as they would shorten the rest of a top-down flow.
    pub fn room(&self) -> Vec2 {
        let count = self.columns.len().max(1);
        let (column, row) = (self.next % count, self.next / count);
        let width = match self.columns.get(column) {
            Some(ColumnWidth::Content) => {
                let kept: f32 = (self.columns.iter().zip(&self.widths).enumerate())
                    .filter(|(at, (kind, _))| *at != column && !matches!(kind, ColumnWidth::Fill { .. }))
                    .map(|(_, (_, width))| *width)
                    .sum();
                let gaps = self.gap.x * count.saturating_sub(1) as f32;
                (self.flow.rect().width() - kept - gaps).max(0.0)
            }
            _ => self.widths.get(column).copied().unwrap_or(0.0),
        };
        let top: f32 = self.last_rows.iter().take(row).map(|height| height + self.gap.y).sum();
        Vec2::new(width, (self.flow.rect().height() - top).max(0.0))
    }

    /// The room the next child fills when it fills its width: its column's width, which for a `Content` column is the
    /// width its cells have measured, not the room the cell measures in. A column no pass has measured yet offers 0,
    /// so a filling child there takes its content width.
    pub fn fill_room(&self) -> Vec2 {
        let mut room = self.room();
        if self.in_content_column() {
            room.x = self.widths.get(self.next % self.columns.len().max(1)).copied().unwrap_or(0.0);
        }
        room
    }

    /// Whether the next child goes in a `Content` column.
    pub fn in_content_column(&self) -> bool {
        matches!(self.columns.get(self.next % self.columns.len().max(1)), Some(ColumnWidth::Content))
    }

    /// The cell last placed measured `size` after all: its column and row record that, and a `Content` column offers
    /// the cells after it in this pass the wider width.
    pub fn widen(&mut self, size: Vec2) {
        let Some(placed) = self.next.checked_sub(1) else { return };
        let count = self.columns.len().max(1);
        let (column, row) = (placed % count, placed / count);
        self.measured.columns[column] = self.measured.columns[column].max(size.x);
        self.measured.rows[row] = self.measured.rows[row].max(size.y);
        if let (Some(ColumnWidth::Content), Some(width)) = (self.columns.get(column), self.widths.get_mut(column)) {
            *width = width.max(size.x);
        }
    }

    /// What this pass measured.
    pub fn measured(&self) -> &GridMeasure {
        &self.measured
    }

    /// The height the rows took last pass, gaps included.
    pub fn last_height(&self) -> f32 {
        self.last_rows.iter().sum::<f32>() + self.gap.y * self.last_rows.len().saturating_sub(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(rtl: bool) -> Flow {
        Flow::new(Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 100.0)), rtl)
    }

    #[test]
    fn a_widened_content_cell_moves_the_cells_after_it() {
        let columns = [ColumnWidth::Content, ColumnWidth::Content];
        let mut cursor = GridCursor::new(flow(false), &columns, Vec2::ZERO, &GridMeasure::default());
        cursor.cell(Vec2::new(0.0, 10.0));
        cursor.widen(Vec2::new(80.0, 10.0));
        let second = cursor.cell(Vec2::new(20.0, 10.0));
        assert_eq!(second.min.x, 80.0);
        assert_eq!(cursor.measured().columns, vec![80.0, 20.0]);
    }

    #[test]
    fn fixed_content_and_fill_columns_share_the_room() {
        let columns = [ColumnWidth::Fixed(100.0), ColumnWidth::Content, ColumnWidth::fill(1.0), ColumnWidth::fill(3.0)];
        let widths = resolve_columns(&columns, &[0.0, 40.0], 300.0, 0.0);
        assert_eq!(widths, vec![100.0, 40.0, 40.0, 120.0]);
    }

    #[test]
    fn cells_run_row_major_from_last_pass_and_mirror_under_rtl() {
        let columns = [ColumnWidth::Fixed(100.0), ColumnWidth::Content];
        let last = GridMeasure { columns: vec![0.0, 50.0], rows: vec![20.0, 30.0] };
        let mut ltr = GridCursor::new(flow(false), &columns, Vec2::ZERO, &last);
        let cells: Vec<Rect> = (0..4).map(|_| ltr.cell(Vec2::new(10.0, 10.0))).collect();
        assert_eq!((cells[1].min.x, cells[2].min, cells[3].min), (100.0, Pos2::new(0.0, 20.0), Pos2::new(100.0, 20.0)));
        let mut rtl = GridCursor::new(flow(true), &columns, Vec2::ZERO, &last);
        let first = rtl.cell(Vec2::new(10.0, 10.0));
        let second = rtl.cell(Vec2::new(10.0, 10.0));
        assert_eq!((first.max.x, second.max.x), (300.0, 200.0), "the first column is on the right");
        assert_eq!(ltr.measured().rows, vec![10.0, 10.0]);
    }

    #[test]
    fn a_content_cell_after_a_fill_cell_measures_in_the_whole_height_and_the_width_fixed_columns_leave() {
        let columns = [ColumnWidth::fill(1.0), ColumnWidth::Content, ColumnWidth::Fixed(100.0)];
        let last = GridMeasure { columns: vec![0.0, 50.0, 0.0], rows: vec![100.0] };
        let mut cursor = GridCursor::new(flow(false), &columns, Vec2::ZERO, &last);
        assert_eq!(cursor.room(), Vec2::new(150.0, 100.0), "the fill cell measures in its own column");
        cursor.cell(Vec2::new(150.0, 100.0));
        assert_eq!(cursor.room(), Vec2::new(200.0, 100.0), "the content cell may take what the fill column holds");
    }
}
