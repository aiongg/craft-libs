//! Item geometry: where each item of a collection sits for a scroll offset and viewport.

use crate::columns::{ColumnKey, GridView};
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
}

impl SpanLayout {
    /// `count` rows of height `row_h` and `width`, with no columns.
    pub fn new(row_h: f32, count: usize, width: f32) -> Self {
        Self { row_h, count, width, columns: Vec::new() }
    }

    /// The columns with their widths, from the start edge.
    #[must_use]
    pub fn columns(mut self, columns: Vec<(ColumnKey, f32)>) -> Self {
        self.columns = columns;
        self
    }
}

impl ItemLayout for SpanLayout {
    fn content_size(&self) -> ScrollPos {
        ScrollPos { x: f64::from(self.width), y: f64::from(self.row_h) * self.count as f64 }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        let h = f64::from(self.row_h);
        let first = (offset.y / h).floor().max(0.0) as usize;
        let last = (((offset.y + f64::from(view.y)) / h).ceil().max(0.0) as usize).min(self.count);
        out.extend(first..last);
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        let top = (at as f64 * f64::from(self.row_h) - offset.y) as f32;
        Rect::from_min_size(Pos2::new(-(offset.x as f32), top), Vec2::new(self.width, self.row_h))
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        let at = ((f64::from(point.y) + offset.y) / f64::from(self.row_h)).floor();
        (at >= 0.0 && (at as usize) < self.count).then_some(at as usize)
    }

    fn nav(&self) -> NavShape {
        NavShape::Flat
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

    fn rows(count: usize) -> SpanLayout {
        SpanLayout::new(20.0, count, 100.0).columns(vec![(ColumnKey::new("a"), 30.0), (ColumnKey::new("b"), 70.0)])
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
}
