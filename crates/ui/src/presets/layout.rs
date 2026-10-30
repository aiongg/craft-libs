//! The geometry of a preset browser as plain data: which rows are shown (filter and open
//! groups), how they flatten to visual rows (headers, list rows, wrapped lines of grid cells),
//! how tall each is, and how the lead moves through them. Nothing here reads egui input.

use std::hash::Hash;
use std::ops::Range;

use egui::{Rect, Vec2, pos2, vec2};

use super::model::{CellShape, PresetRow, children_end};
use super::state::{DefaultOpen, PresetState, PresetView};
use crate::behavior::{GridGeometry, GridNav};
use crate::list::spans::Spans;

/// The numbers of one layout pass, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// The thumbnail width (the state's, clamped).
    pub thumb: f32,
    /// The shape of a thumbnail.
    pub shape: CellShape,
    /// Whether cells make room for a caption line.
    pub captions: bool,
    /// The height of one caption line.
    pub caption_line: f32,
    /// The space between a thumbnail and its caption.
    pub caption_gap: f32,
    /// The space between cells, across and down; 0 when flush.
    pub gap: f32,
    /// The space at the reading start of a grid row; 0 when flush.
    pub inset: f32,
    /// The height of a header row.
    pub header_height: f32,
    /// The height of a list row.
    pub list_row: f32,
}

impl Metrics {
    /// The metrics for a grid that is `flush` (no gap, no inset) or not.
    pub fn new(thumb: f32, shape: CellShape, flush: bool, gap: f32, inset: f32) -> Self {
        Self {
            thumb,
            shape,
            captions: false,
            caption_line: 0.0,
            caption_gap: 0.0,
            gap: if flush { 0.0 } else { gap },
            inset: if flush { 0.0 } else { inset },
            header_height: 0.0,
            list_row: 0.0,
        }
    }

    /// The size of a grid cell: the thumbnail and, with captions, a gap and a line.
    pub fn cell(&self) -> Vec2 {
        let thumb = self.shape.size(self.thumb);
        let caption = if self.captions { self.caption_gap + self.caption_line } else { 0.0 };
        vec2(thumb.x, thumb.y + caption)
    }

    /// How many cells fit a row of `width`.
    pub fn columns(&self, width: f32) -> usize {
        GridNav::columns(width, self.inset, self.gap, self.cell().x)
    }

    /// The height of a visual row; a line of cells includes the gap below it.
    pub fn height(&self, row: &VisualRow) -> f32 {
        match row {
            VisualRow::Header(_) => self.header_height,
            VisualRow::Item(_) => self.list_row,
            VisualRow::Cells { .. } => self.cell().y + self.gap,
        }
    }

    /// The grid of one run of cells whose first line is at `top` inside `area`: the reading-start
    /// corner is inset from the area's edge on the reading side, so spare width stays at the end.
    pub fn run_geometry(&self, area: Rect, top: f32, len: usize, columns: usize, rtl: bool) -> GridGeometry {
        let x = if rtl { area.right() - self.inset } else { area.left() + self.inset };
        GridGeometry { origin: pos2(x, area.top() + top), cell: self.cell(), gap: Vec2::splat(self.gap), columns: columns.max(1), len, rtl }
    }
}

/// The indices of the rows to show, in order. Without a filter, the children of a closed group
/// are skipped. With one, only presets whose label contains the text (any letter case) remain,
/// with their groups, which are open; a group left with no preset is hidden.
pub fn shown<K: Clone + Eq + Hash>(rows: &[PresetRow<K>], state: &PresetState<K>, default_open: DefaultOpen) -> Vec<usize> {
    if state.filtering() {
        return filtered(rows, state.filter.trim());
    }
    let open = state.open_test(rows, default_open);
    let mut out = Vec::with_capacity(rows.len());
    let mut at = 0;
    while at < rows.len() {
        out.push(at);
        at = if rows[at].is_group() && !open(at) { children_end(rows, at) } else { at + 1 };
    }
    out
}

fn filtered<K>(rows: &[PresetRow<K>], text: &str) -> Vec<usize> {
    let needle = text.to_lowercase();
    let mut keep = vec![false; rows.len()];
    let mut path: Vec<usize> = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        path.truncate(row.depth);
        if !row.is_group() && row.label.to_lowercase().contains(&needle) {
            keep[at] = true;
            for &ancestor in &path {
                keep[ancestor] = true;
            }
        }
        path.push(at);
    }
    (0..rows.len()).filter(|&at| keep[at]).collect()
}

/// One line of the browser's column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualRow {
    /// A group or folder header: the position in the shown list.
    Header(usize),
    /// A preset as a list row.
    Item(usize),
    /// A line of grid cells: `range` are positions in the shown list; `run` is every position of
    /// the cells that wrap together (consecutive presets of one depth), over several lines.
    Cells {
        /// The positions of this line's cells.
        range: Range<usize>,
        /// The positions of the whole run.
        run: Range<usize>,
    },
}

impl VisualRow {
    /// How many cells (lead stops) the line has.
    pub fn len(&self) -> usize {
        match self {
            VisualRow::Header(_) | VisualRow::Item(_) => 1,
            VisualRow::Cells { range, .. } => range.len(),
        }
    }

    /// Whether the line has no cells.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The shown rows flattened to visual rows. A position is an index into [`shown`](VisualRows::shown);
/// the lead, `Home` and `End` all count in positions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualRows {
    /// The source rows shown, as [`shown`] gives them.
    pub shown: Vec<usize>,
    /// The visual rows, which partition the positions in order.
    pub rows: Vec<VisualRow>,
    starts: Vec<usize>,
}

impl VisualRows {
    /// Flattens `shown` rows. In grid view a run of presets of one depth wraps into lines of
    /// `columns` cells (at least one); in list view every preset is a row.
    pub fn new<K>(rows: &[PresetRow<K>], shown: Vec<usize>, view: PresetView, columns: usize) -> Self {
        let columns = columns.max(1);
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < shown.len() {
            let row = &rows[shown[pos]];
            if row.is_group() {
                out.push(VisualRow::Header(pos));
                pos += 1;
            } else if view == PresetView::List {
                out.push(VisualRow::Item(pos));
                pos += 1;
            } else {
                let mut end = pos + 1;
                while end < shown.len() && !rows[shown[end]].is_group() && rows[shown[end]].depth == row.depth {
                    end += 1;
                }
                let run = pos..end;
                out.extend((pos..end).step_by(columns).map(|s| VisualRow::Cells { range: s..(s + columns).min(end), run: run.clone() }));
                pos = end;
            }
        }
        let starts = out
            .iter()
            .scan(0, |at, row| {
                let start = *at;
                *at += row.len();
                Some(start)
            })
            .collect();
        Self { shown, rows: out, starts }
    }

    /// The vertical extents of the visual rows.
    pub fn spans(&self, metrics: &Metrics) -> Spans {
        Spans::mixed(self.rows.len(), |at| metrics.height(&self.rows[at]))
    }

    /// The visual row holding position `pos` (the last for a position past the end).
    pub fn visual_row_of(&self, pos: usize) -> usize {
        self.starts.partition_point(|&s| s <= pos).saturating_sub(1)
    }

    /// The position of the source row `source` in the shown list, if it is shown.
    #[allow(dead_code)]
    pub fn position_of(&self, source: usize) -> Option<usize> {
        self.shown.binary_search(&source).ok()
    }

    /// The lead's moves over these rows: a header or list row is one cell, a line of cells has
    /// as many. `page` is the number of visual rows Page Up and Down move.
    pub fn nav(&self, rtl: bool, page: usize) -> GridNav {
        GridNav::ragged(self.rows.iter().map(VisualRow::len)).rtl(rtl).page(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Row = PresetRow<&'static str>;

    fn library() -> Vec<Row> {
        vec![
            Row::group("g1", "Round", 0),
            Row::item("soft", "Soft Round", 1),
            Row::item("hard", "Hard Round", 1),
            Row::group("g1f", "Folder", 1),
            Row::item("deep", "Deep", 2),
            Row::group("g2", "Square", 0),
            Row::item("chalk", "Chalk", 1),
        ]
    }

    fn state_with(filter: &str) -> PresetState<&'static str> {
        PresetState { filter: filter.into(), ..PresetState::new() }
    }

    #[test]
    fn presets_closed_groups_hide_their_children() {
        let rows = library();
        let mut state = state_with("");
        assert_eq!(shown(&rows, &state, DefaultOpen::All), [0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(shown(&rows, &state, DefaultOpen::First), [0, 1, 2, 3, 5], "only the first group is open, so its folder is closed");
        assert_eq!(shown(&rows, &state, DefaultOpen::None), [0, 5]);
        state.toggled.insert("g1f");
        assert_eq!(shown(&rows, &state, DefaultOpen::First), [0, 1, 2, 3, 4, 5], "a folder toggled open inside an open group");
    }

    #[test]
    fn presets_filter_keeps_matches_with_their_groups_open_and_drops_empty_groups() {
        let rows = library();
        let mut state = state_with("ROUND");
        state.toggled.insert("g1");
        assert_eq!(shown(&rows, &state, DefaultOpen::None), [0, 1, 2], "groups open, Square and Folder hidden");
        assert_eq!(shown(&rows, &state_with("deep"), DefaultOpen::None), [0, 3, 4], "the folder's ancestors stay");
        assert_eq!(shown(&rows, &state_with("zzz"), DefaultOpen::All), Vec::<usize>::new());
        assert_eq!(shown(&rows, &state_with("square"), DefaultOpen::All), Vec::<usize>::new(), "a group's own name matches nothing");
    }

    #[test]
    fn presets_grid_rows_wrap_runs_and_break_at_headers() {
        let rows = library();
        let all = shown(&rows, &state_with(""), DefaultOpen::All);
        let flat = VisualRows::new(&rows, all.clone(), PresetView::Grid, 1);
        assert_eq!(flat.rows.len(), 7);
        let wide = VisualRows::new(&rows, all.clone(), PresetView::Grid, 5);
        assert_eq!(
            wide.rows,
            [
                VisualRow::Header(0),
                VisualRow::Cells { range: 1..3, run: 1..3 },
                VisualRow::Header(3),
                VisualRow::Cells { range: 4..5, run: 4..5 },
                VisualRow::Header(5),
                VisualRow::Cells { range: 6..7, run: 6..7 },
            ]
        );
        let items: Vec<Row> = (0..5).map(|i| Row::item(["a", "b", "c", "d", "e"][i], "x", 0)).collect();
        let wrapped = VisualRows::new(&items, (0..5).collect(), PresetView::Grid, 2);
        assert_eq!(wrapped.rows.len(), 3);
        assert_eq!(wrapped.rows[2], VisualRow::Cells { range: 4..5, run: 0..5 });
        assert_eq!((wrapped.visual_row_of(3), wrapped.visual_row_of(4)), (1, 2));
        let list = VisualRows::new(&rows, all, PresetView::List, 5);
        assert_eq!(list.rows.len(), 7, "list view: a row for every shown row");
        assert_eq!(list.position_of(3), Some(3));
    }

    #[test]
    fn presets_lead_walks_cells_and_crosses_headers() {
        let rows = library();
        let visual = VisualRows::new(&rows, shown(&rows, &state_with(""), DefaultOpen::All), PresetView::Grid, 2);
        let nav = visual.nav(false, 3);
        // Positions: header 0, cells 1 2, header 3, cell 4, header 5, cell 6.
        assert_eq!(nav.step(2, crate::behavior::GridKey::Down, None).index, 3);
        assert_eq!(nav.step(3, crate::behavior::GridKey::Up, Some(1)).index, 2, "back to the goal column");
        assert_eq!(nav.step(2, crate::behavior::GridKey::Right, None).index, 3, "left and right walk over headers");
    }

    #[test]
    fn presets_grid_math_columns_and_heights() {
        let mut m = Metrics::new(22.0, CellShape::Square, false, 4.0, 8.0);
        assert_eq!(m.columns(100.0), 3);
        assert_eq!(m.columns(1.0), 1);
        assert_eq!(m.cell(), vec2(22.0, 22.0));
        m.captions = true;
        m.caption_line = 12.0;
        m.caption_gap = 2.0;
        assert_eq!(m.cell(), vec2(22.0, 36.0));
        let cells = VisualRow::Cells { range: 0..2, run: 0..2 };
        assert_eq!(m.height(&cells), 40.0, "the gap below the line is part of it");
        let ratio = Metrics::new(64.0, CellShape::Ratio(32.0 / 18.0), false, 4.0, 8.0);
        assert!((ratio.cell().y - 36.0).abs() < 1e-4);
    }

    #[test]
    fn presets_flush_grid_has_no_gap_or_inset() {
        let m = Metrics::new(25.0, CellShape::Square, true, 4.0, 8.0);
        assert_eq!((m.gap, m.inset), (0.0, 0.0));
        assert_eq!(m.columns(100.0), 4);
        let area = Rect::from_min_size(pos2(10.0, 0.0), vec2(100.0, 500.0));
        let g = m.run_geometry(area, 30.0, 6, 4, false);
        assert_eq!(g.cell_rect(5), Rect::from_min_size(pos2(35.0, 55.0), vec2(25.0, 25.0)));
    }

    #[test]
    fn presets_geometry_starts_on_the_reading_side() {
        let m = Metrics::new(20.0, CellShape::Square, false, 4.0, 8.0);
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 500.0));
        assert_eq!(m.run_geometry(area, 0.0, 3, 3, false).cell_rect(0).min.x, 8.0);
        assert_eq!(m.run_geometry(area, 0.0, 3, 3, true).cell_rect(0).max.x, 92.0);
    }

    #[test]
    fn presets_spans_follow_the_visual_rows() {
        let rows = library();
        let visual = VisualRows::new(&rows, shown(&rows, &state_with(""), DefaultOpen::All), PresetView::Grid, 5);
        let mut m = Metrics::new(20.0, CellShape::Square, true, 4.0, 8.0);
        m.header_height = 24.0;
        let spans = visual.spans(&m);
        assert_eq!(spans.len(), 6);
        assert_eq!(spans.total(), 24.0 * 3.0 + 20.0 * 3.0);
        assert_eq!(spans.top(1), 24.0);
    }
}
