//! Item geometry: where each item of a collection sits for a scroll offset and viewport.

use crate::columns::{ColumnKey, GridSection, GridView};
use crate::item_source::{DisplayOrder, ItemSource, SizeHint};
use crate::nav::NavShape;
use crate::scroll::ScrollPos;
use emath::{Pos2, Rect, Vec2};

/// Item geometry in content units. Offsets are `f64`, so a sheet of a million rows places its last row
/// exactly. Every rect and point the trait takes or returns is `f32` and relative to the viewport's top-left,
/// in logical coordinates: x runs from the start edge, as `ScrollPos.x` does. The item step maps rects, cells,
/// the grid view and the hit point through the collection's flow when `mirrors()` is true.
pub trait ItemLayout {
    /// False for a layout whose positions are document data (`FreeLayout`); the item step then leaves them as given.
    fn mirrors(&self) -> bool {
        true
    }

    /// Finite and non-negative.
    fn content_size(&self) -> ScrollPos;

    /// The indices of the items that intersect a viewport of size `view` scrolled to `offset`, ascending.
    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>);

    /// The rect of item `at`.
    fn rect(&self, at: usize, offset: ScrollPos) -> Rect;

    /// The item under `point`.
    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize>;

    /// How arrow keys move through the items.
    fn nav(&self) -> NavShape;

    /// The cells of item `at`, with their rects.
    fn cells(&self, _at: usize, _offset: ScrollPos, _out: &mut Vec<(ColumnKey, Rect)>) {}

    /// Column virtualisation and sections of a sheet.
    fn grid(&self) -> Option<&GridView> {
        None
    }

    /// The number of columns of a table or sheet; 0 for a layout with none.
    fn column_count(&self) -> usize {
        0
    }

    /// The key of the column at position `index`.
    fn column_key(&self, _index: usize) -> Option<ColumnKey> {
        None
    }

    /// The position of the column under `point`.
    fn column_at(&self, _point: Pos2, _offset: ScrollPos) -> Option<usize> {
        None
    }

    /// Whether item `at` is as tall as it measures itself.
    fn is_auto(&self, _at: usize) -> bool {
        false
    }

    /// Takes the measured heights of the auto items: `heights(at)` is `None` before the item's first measure.
    fn measured(&mut self, _heights: &dyn Fn(usize) -> Option<f32>) {}

    /// An auto item measured `height` in this pass; the items after it move.
    fn set_height(&mut self, _at: usize, _height: f32) {}

    /// The viewport and scroll offset the grid view describes. Layouts without a grid ignore it.
    fn set_view(&mut self, _viewport: Vec2, _offset: ScrollPos) {}
}

/// Per-row heights of a layout whose rows differ: the tops are the running sum, so a row and a hit are found by
/// binary search.
#[derive(Clone, Debug)]
struct Rows {
    auto: Vec<bool>,
    heights: Vec<f32>,
    tops: Vec<f64>,
}

impl Rows {
    fn retop(&mut self) {
        self.tops.clear();
        let mut top = 0.0;
        self.tops.push(top);
        for &height in &self.heights {
            top += f64::from(height);
            self.tops.push(top);
        }
    }
}

/// Rows of equal height, optionally split into columns: lists, popup lists, tab strips and timeline tracks.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SpanLayout {
    /// Height of one row.
    pub row_h: f32,
    /// Number of rows.
    pub count: usize,
    /// Width of one row.
    pub width: f32,
    /// The columns with their widths, from the start edge.
    pub columns: Vec<(ColumnKey, f32)>,
    /// Whether the rows form a tree: arrows expand, collapse and step to the parent.
    pub tree: bool,
    rows: Option<Rows>,
}

impl SpanLayout {
    /// One row of `row_h` per entry of `source`, `width` wide, with no columns.
    pub fn new<K>(source: &dyn DisplayOrder<K>, width: f32, row_h: f32) -> Self {
        Self { row_h, count: source.len(), width, columns: Vec::new(), tree: false, rows: None }
    }

    /// Rows as tall as `source` says: `Height` and `Rect` are the app's, `Aspect` follows the width, and `Auto`
    /// rows take `row_h` until the item step feeds their measured height.
    pub fn sized<K>(source: &dyn ItemSource<K>, width: f32, row_h: f32) -> Self {
        let hints: Vec<SizeHint> = (0..source.len()).map(|at| source.size_hint(at)).collect();
        let heights = hints
            .iter()
            .map(|hint| match *hint {
                SizeHint::Height(height) => height,
                SizeHint::Rect(size) => size.y,
                SizeHint::Aspect(aspect) if aspect > 0.0 => width / aspect,
                _ => row_h,
            })
            .collect();
        let auto = hints.iter().map(|hint| matches!(hint, SizeHint::Auto)).collect();
        let mut rows = Rows { auto, heights, tops: Vec::new() };
        rows.retop();
        Self { row_h, count: hints.len(), width, columns: Vec::new(), tree: false, rows: Some(rows) }
    }

    fn top(&self, at: usize) -> f64 {
        match &self.rows {
            Some(rows) => rows.tops.get(at).or(rows.tops.last()).copied().unwrap_or(0.0),
            None => at as f64 * f64::from(self.row_h),
        }
    }

    fn height(&self, at: usize) -> f32 {
        self.rows.as_ref().map_or(self.row_h, |rows| rows.heights.get(at).copied().unwrap_or(self.row_h))
    }

    /// The columns with their widths, from the start edge.
    #[must_use]
    pub fn columns(mut self, columns: Vec<(ColumnKey, f32)>) -> Self {
        self.columns = columns;
        self
    }

    /// Rows that expand and collapse.
    #[must_use]
    pub fn tree(mut self) -> Self {
        self.tree = true;
        self
    }
}

impl ItemLayout for SpanLayout {
    fn content_size(&self) -> ScrollPos {
        ScrollPos { x: f64::from(self.width), y: self.top(self.count) }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        let bottom = offset.y + f64::from(view.y);
        if let Some(rows) = &self.rows {
            let first = rows.tops[1..].partition_point(|&row_bottom| row_bottom <= offset.y);
            let last = rows.tops[..self.count].partition_point(|&row_top| row_top < bottom);
            out.extend(first..last.max(first));
            return;
        }
        let h = f64::from(self.row_h);
        let first = (offset.y / h).floor().max(0.0) as usize;
        let last = ((bottom / h).ceil().max(0.0) as usize).min(self.count);
        out.extend(first..last);
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        let top = (self.top(at) - offset.y) as f32;
        Rect::from_min_size(Pos2::new(-(offset.x as f32), top), Vec2::new(self.width, self.height(at)))
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        let y = f64::from(point.y) + offset.y;
        if let Some(rows) = &self.rows {
            let at = rows.tops[1..].partition_point(|&row_bottom| row_bottom <= y);
            return (y >= 0.0 && at < self.count).then_some(at);
        }
        let at = (y / f64::from(self.row_h)).floor();
        (at >= 0.0 && (at as usize) < self.count).then_some(at as usize)
    }

    fn column_count(&self) -> usize {
        self.columns.len()
    }

    fn column_key(&self, index: usize) -> Option<ColumnKey> {
        self.columns.get(index).map(|(key, _)| key.clone())
    }

    fn column_at(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        column_under(self.columns.iter().map(|(_, width)| *width), f64::from(point.x) + offset.x)
    }

    fn is_auto(&self, at: usize) -> bool {
        self.rows.as_ref().is_some_and(|rows| rows.auto.get(at).copied().unwrap_or(false))
    }

    fn measured(&mut self, heights: &dyn Fn(usize) -> Option<f32>) {
        if let Some(rows) = &mut self.rows {
            for at in (0..rows.heights.len()).filter(|&at| rows.auto[at]) {
                if let Some(height) = heights(at) {
                    rows.heights[at] = height;
                }
            }
            rows.retop();
        }
    }

    fn set_height(&mut self, at: usize, height: f32) {
        if let Some(rows) = &mut self.rows
            && rows.auto.get(at).copied().unwrap_or(false)
            && rows.heights[at] != height
        {
            rows.heights[at] = height;
            rows.retop();
        }
    }

    fn nav(&self) -> NavShape {
        if self.tree { NavShape::Tree } else { NavShape::Flat }
    }

    fn cells(&self, at: usize, offset: ScrollPos, out: &mut Vec<(ColumnKey, Rect)>) {
        let row = self.rect(at, offset);
        let mut along = 0.0;
        for (key, w) in &self.columns {
            let x = row.left() + along;
            out.push((key.clone(), Rect::from_min_size(Pos2::new(x, row.top()), Vec2::new(*w, row.height()))));
            along += w;
        }
    }
}

/// The position of the column holding content-x `x`, for columns of `widths` from the start edge.
fn column_under(widths: impl Iterator<Item = f32>, x: f64) -> Option<usize> {
    let mut left = 0.0;
    for (index, width) in widths.enumerate() {
        left += f64::from(width);
        if x < left {
            return (x >= 0.0).then_some(index);
        }
    }
    None
}

/// Equal tiles that wrap at the width: tile grids, preset grids, font cards and swatch grids.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TileLayout {
    /// The size of one tile.
    pub tile: Vec2,
    /// The space between tiles, on both axes.
    pub gap: f32,
    /// Number of tiles.
    pub count: usize,
    /// The width the tiles wrap in.
    pub width: f32,
}

impl TileLayout {
    /// One tile of `tile` per entry of `source`, `gap` apart, wrapping at `width`.
    pub fn new<K>(source: &dyn DisplayOrder<K>, width: f32, tile: Vec2, gap: f32) -> Self {
        Self { tile, gap, count: source.len(), width }
    }

    /// Tiles per row: at least one.
    pub fn columns(&self) -> usize {
        (((self.width + self.gap) / (self.tile.x + self.gap)).floor() as usize).max(1)
    }

    fn rows(&self) -> usize {
        self.count.div_ceil(self.columns())
    }

    fn pitch(&self) -> Vec2 {
        Vec2::new(self.tile.x + self.gap, self.tile.y + self.gap)
    }
}

impl ItemLayout for TileLayout {
    fn content_size(&self) -> ScrollPos {
        let rows = self.rows() as f64;
        let height = if rows == 0.0 { 0.0 } else { rows * f64::from(self.pitch().y) - f64::from(self.gap) };
        ScrollPos { x: f64::from(self.width), y: height }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        let pitch = f64::from(self.pitch().y);
        let first = (offset.y / pitch).floor().max(0.0) as usize;
        let last = (((offset.y + f64::from(view.y)) / pitch).ceil().max(0.0) as usize).min(self.rows());
        let columns = self.columns();
        out.extend(first * columns..(last * columns).min(self.count));
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        let columns = self.columns();
        let pitch = self.pitch();
        let x = (at % columns) as f32 * pitch.x - offset.x as f32;
        let y = ((at / columns) as f64 * f64::from(pitch.y) - offset.y) as f32;
        Rect::from_min_size(Pos2::new(x, y), self.tile)
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        let pitch = self.pitch();
        let (x, y) = (f64::from(point.x) + offset.x, f64::from(point.y) + offset.y);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let (column, row) = ((x / f64::from(pitch.x)) as usize, (y / f64::from(pitch.y)) as usize);
        let inside = x - column as f64 * f64::from(pitch.x) < f64::from(self.tile.x) && y - row as f64 * f64::from(pitch.y) < f64::from(self.tile.y);
        let at = row * self.columns() + column;
        (inside && column < self.columns() && at < self.count).then_some(at)
    }

    fn nav(&self) -> NavShape {
        NavShape::wrapped_grid(self.columns())
    }
}

/// Rows of a sheet: one position per row, each split into the sheet's columns. A million rows place and hit
/// exactly, because the rows are computed in `f64` from the offset.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SheetLayout {
    /// Height of one row.
    pub row_h: f32,
    /// Number of rows.
    pub rows: usize,
    /// The columns with their widths, from the start edge.
    pub columns: Vec<(ColumnKey, f32)>,
    /// How many leading columns stay in view while the others scroll.
    pub frozen: usize,
    grid: Option<GridView>,
}

impl SheetLayout {
    /// A row of `row_h` per entry of `source`, split into `columns`.
    pub fn new<K>(source: &dyn DisplayOrder<K>, row_h: f32, columns: Vec<(ColumnKey, f32)>) -> Self {
        Self { row_h, rows: source.len(), columns, frozen: 0, grid: None }
    }

    /// The first `count` columns stay in view while the others scroll.
    #[must_use]
    pub fn frozen(mut self, count: usize) -> Self {
        self.frozen = count;
        self
    }

    fn row_width(&self) -> f32 {
        self.columns.iter().map(|(_, width)| width).sum()
    }

    fn frozen_width(&self) -> f32 {
        self.columns.iter().take(self.frozen).map(|(_, width)| width).sum()
    }

    /// The sections of a viewport scrolled to `offset_x`: the frozen columns at the start edge, then the scrolling
    /// columns that intersect the rest of the viewport. Edges are viewport-relative x positions.
    fn sections(&self, viewport: Vec2, offset_x: f64) -> GridView {
        let mut lefts = Vec::with_capacity(self.columns.len() + 1);
        let mut along = 0.0;
        lefts.push(along);
        for (_, width) in &self.columns {
            along += width;
            lefts.push(along);
        }
        let frozen = self.frozen.min(self.columns.len());
        let frozen_w = lefts[frozen];
        let mut sections = Vec::new();
        if frozen > 0 {
            let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(frozen_w, viewport.y));
            sections.push(GridSection::new(rect, 0..frozen as u32, lefts[..=frozen].to_vec()));
        }
        let shift = offset_x as f32;
        if let Some(first) = (frozen..self.columns.len()).find(|&column| lefts[column + 1] - shift > frozen_w) {
            let end = (first..self.columns.len()).find(|&column| lefts[column] - shift >= viewport.x).unwrap_or(self.columns.len());
            let edges: Vec<f32> = (first..=end).map(|column| lefts[column] - shift).collect();
            let (start, stop) = ((lefts[first] - shift).max(frozen_w), (lefts[end] - shift).min(viewport.x));
            let rect = Rect::from_min_max(Pos2::new(start, 0.0), Pos2::new(stop.max(start), viewport.y));
            sections.push(GridSection::new(rect, first as u32..end as u32, edges));
        }
        GridView::new(sections)
    }
}

impl ItemLayout for SheetLayout {
    fn content_size(&self) -> ScrollPos {
        ScrollPos { x: f64::from(self.row_width()), y: f64::from(self.row_h) * self.rows as f64 }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        let h = f64::from(self.row_h);
        let first = (offset.y / h).floor().max(0.0) as usize;
        let last = (((offset.y + f64::from(view.y)) / h).ceil().max(0.0) as usize).min(self.rows);
        out.extend(first..last);
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        let top = (at as f64 * f64::from(self.row_h) - offset.y) as f32;
        Rect::from_min_size(Pos2::new(-(offset.x as f32), top), Vec2::new(self.row_width(), self.row_h))
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        let at = ((f64::from(point.y) + offset.y) / f64::from(self.row_h)).floor();
        (at >= 0.0 && (at as usize) < self.rows).then_some(at as usize)
    }

    fn nav(&self) -> NavShape {
        NavShape::Cell
    }

    fn cells(&self, at: usize, offset: ScrollPos, out: &mut Vec<(ColumnKey, Rect)>) {
        let row = self.rect(at, offset);
        if let Some(grid) = &self.grid {
            for section in &grid.sections {
                for (edge, column) in section.columns.clone().enumerate() {
                    let (left, right) = (section.edges[edge], section.edges[edge + 1]);
                    let rect = Rect::from_min_max(Pos2::new(left, row.top()), Pos2::new(right, row.bottom()));
                    out.push((self.columns[column as usize].0.clone(), rect));
                }
            }
            return;
        }
        let mut along = 0.0;
        for (key, width) in &self.columns {
            out.push((key.clone(), Rect::from_min_size(Pos2::new(row.left() + along, row.top()), Vec2::new(*width, row.height()))));
            along += width;
        }
    }

    fn grid(&self) -> Option<&GridView> {
        self.grid.as_ref()
    }

    fn set_view(&mut self, viewport: Vec2, offset: ScrollPos) {
        self.grid = Some(self.sections(viewport, offset.x));
    }

    fn column_count(&self) -> usize {
        self.columns.len()
    }

    fn column_key(&self, index: usize) -> Option<ColumnKey> {
        self.columns.get(index).map(|(key, _)| key.clone())
    }

    fn column_at(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        let x = if self.frozen > 0 && point.x < self.frozen_width() { f64::from(point.x) } else { f64::from(point.x) + offset.x };
        column_under(self.columns.iter().map(|(_, width)| *width), x)
    }
}

/// Items at positions the document stores: a freeform bin or a node graph.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FreeLayout {
    /// One rect per item, in content coordinates.
    pub rects: Vec<Rect>,
}

impl FreeLayout {
    /// Items at `rects`, in content coordinates.
    pub fn new(rects: Vec<Rect>) -> Self {
        Self { rects }
    }
}

impl ItemLayout for FreeLayout {
    fn mirrors(&self) -> bool {
        false
    }

    fn content_size(&self) -> ScrollPos {
        let size = self.rects.iter().fold(Rect::NOTHING, |all, r| all.union(*r)).size();
        ScrollPos { x: f64::from(size.x.max(0.0)), y: f64::from(size.y.max(0.0)) }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        let viewport = Rect::from_min_size(Pos2::ZERO, view);
        out.extend((0..self.rects.len()).filter(|&i| self.rect(i, offset).intersects(viewport)));
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        let shift = Vec2::new(-(offset.x as f32), -(offset.y as f32));
        self.rects.get(at).map_or(Rect::NOTHING, |r| r.translate(shift))
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        (0..self.rects.len()).rev().find(|&i| self.rect(i, offset).contains(point))
    }

    fn nav(&self) -> NavShape {
        NavShape::Spatial
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Count(usize);

    impl DisplayOrder<usize> for Count {
        fn len(&self) -> usize {
            self.0
        }

        fn key(&self, at: usize) -> usize {
            at
        }
    }

    fn rows(count: usize) -> SpanLayout {
        SpanLayout::new(&Count(count), 100.0, 20.0).columns(vec![(ColumnKey::new("a"), 30.0), (ColumnKey::new("b"), 70.0)])
    }

    #[test]
    fn the_last_of_a_million_rows_places_and_hits_exactly() {
        let layout = rows(1_000_000);
        let offset = ScrollPos { x: 0.0, y: 19_999_990.0 };
        let last = layout.rect(999_999, offset);
        assert_eq!(last.top(), -10.0);
        assert_eq!(layout.hit(Pos2::new(5.0, 5.0), offset), Some(999_999));
        let mut shown = Vec::new();
        layout.visible(offset, Vec2::new(100.0, 40.0), &mut shown);
        assert_eq!(shown, vec![999_999]);
    }

    #[test]
    fn visible_rows_cover_a_partial_first_and_last_row() {
        let layout = rows(10);
        let mut shown = Vec::new();
        layout.visible(ScrollPos { x: 0.0, y: 15.0 }, Vec2::new(100.0, 30.0), &mut shown);
        assert_eq!(shown, vec![0, 1, 2]);
    }

    #[test]
    fn columns_run_from_the_start_edge_under_horizontal_scroll() {
        let layout = rows(2);
        let mut cells = Vec::new();
        layout.cells(1, ScrollPos { x: 10.0, y: 0.0 }, &mut cells);
        let lefts: Vec<f32> = cells.iter().map(|(_, r)| r.left()).collect();
        assert_eq!(lefts, vec![-10.0, 20.0]);
    }

    struct Hints(Vec<SizeHint>);

    impl DisplayOrder<usize> for Hints {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn key(&self, at: usize) -> usize {
            at
        }
    }

    impl ItemSource<usize> for Hints {
        fn size_hint(&self, at: usize) -> SizeHint {
            self.0[at]
        }
    }

    #[test]
    fn measured_row_heights_place_and_hit_by_their_own_tops() {
        let source = Hints(vec![SizeHint::Auto, SizeHint::Height(50.0), SizeHint::Auto]);
        let mut layout = SpanLayout::sized(&source, 100.0, 20.0);
        assert_eq!(layout.content_size().y, 90.0, "auto rows take the estimate until measured");
        layout.measured(&|at| (at == 0).then_some(30.0));
        layout.set_height(2, 40.0);
        layout.set_height(1, 99.0);
        assert_eq!((layout.is_auto(0), layout.is_auto(1)), (true, false));
        assert_eq!(layout.content_size().y, 120.0, "an app-given height is not remeasured");
        let offset = ScrollPos { x: 0.0, y: 10.0 };
        assert_eq!((layout.rect(1, offset).top(), layout.rect(2, offset).top(), layout.rect(2, offset).height()), (20.0, 70.0, 40.0));
        assert_eq!((layout.hit(Pos2::new(1.0, 25.0), offset), layout.hit(Pos2::new(1.0, 75.0), offset)), (Some(1), Some(2)));
        let mut shown = Vec::new();
        layout.visible(offset, Vec2::new(100.0, 60.0), &mut shown);
        assert_eq!(shown, vec![0, 1]);
        shown.clear();
        layout.visible(ScrollPos { x: 0.0, y: 31.0 }, Vec2::new(100.0, 10.0), &mut shown);
        assert_eq!(shown, vec![1]);
    }

    #[test]
    fn a_sheet_grid_view_follows_the_viewport_and_keeps_frozen_columns_in_place() {
        let columns: Vec<_> = ["a", "b", "c", "d", "e"].into_iter().map(|key| (ColumnKey::new(key), 40.0)).collect();
        let mut layout = SheetLayout::new(&Count(3), 20.0, columns).frozen(1);
        layout.set_view(Vec2::new(100.0, 60.0), ScrollPos { x: 50.0, y: 0.0 });
        let grid = layout.grid().expect("a view was set");
        assert_eq!(grid.sections.len(), 2);
        assert_eq!((grid.sections[0].columns.clone(), grid.sections[0].edges.clone()), (0..1, vec![0.0, 40.0]));
        assert_eq!((grid.sections[1].columns.clone(), grid.sections[1].edges.clone()), (2..4, vec![30.0, 70.0, 110.0]));
        let mut cells = Vec::new();
        layout.cells(0, ScrollPos { x: 50.0, y: 0.0 }, &mut cells);
        assert_eq!(cells.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>(), ["a", "c", "d"].map(ColumnKey::new));
        assert_eq!(layout.column_at(Pos2::new(10.0, 5.0), ScrollPos { x: 50.0, y: 0.0 }), Some(0));
        assert_eq!(layout.column_at(Pos2::new(60.0, 5.0), ScrollPos { x: 50.0, y: 0.0 }), Some(2));
    }

    #[test]
    fn tiles_wrap_at_the_width_and_a_hit_in_the_gap_finds_nothing() {
        let layout = TileLayout::new(&Count(7), 100.0, Vec2::new(20.0, 10.0), 5.0);
        assert_eq!(layout.columns(), 4);
        assert_eq!(layout.nav(), NavShape::wrapped_grid(4));
        let (second_row, none) = (layout.rect(5, ScrollPos::default()), layout.hit(Pos2::new(22.0, 2.0), ScrollPos::default()));
        assert_eq!((second_row.min, none), (Pos2::new(25.0, 15.0), None));
        assert_eq!(layout.hit(Pos2::new(30.0, 17.0), ScrollPos::default()), Some(5));
        let mut shown = Vec::new();
        layout.visible(ScrollPos { x: 0.0, y: 16.0 }, Vec2::new(100.0, 5.0), &mut shown);
        assert_eq!(shown, vec![4, 5, 6], "only the second row shows, and the last row is short");
        assert_eq!(layout.content_size().y, 25.0);
    }

    #[test]
    fn a_sheet_of_a_million_rows_places_and_hits_its_last_row_with_cells() {
        let layout = SheetLayout::new(&Count(1_048_576), 20.0, vec![(ColumnKey::new("a"), 30.0), (ColumnKey::new("b"), 70.0)]);
        let offset = ScrollPos { x: 0.0, y: layout.content_size().y - 100.0 };
        assert_eq!(layout.rect(1_048_575, offset).top(), 80.0);
        assert_eq!(layout.hit(Pos2::new(10.0, 90.0), offset), Some(1_048_575));
        let mut cells = Vec::new();
        layout.cells(1_048_575, offset, &mut cells);
        assert_eq!(cells.iter().map(|(_, rect)| rect.left()).collect::<Vec<_>>(), vec![0.0, 30.0]);
        assert_eq!(layout.nav(), NavShape::Cell);
    }
}
