//! [`GridNav`]: which cell the lead moves to for the arrow keys, Home, End, Page Up and Page Down
//! over a wrapped grid or rows of unequal length, and the Tab stop of a grid that is one focus
//! stop ([`GridNav::roving_stop`]).

use std::ops::Range;

use egui::accesskit::Action;
use egui::{EventFilter, Id, Key, Modifiers, Rect, Ui};

use crate::direction;

/// A key that moves the lead of a grid. The arrows are physical: [`GridNav::rtl`] decides which
/// way Left and Right run along a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKey {
    /// The Left arrow.
    Left,
    /// The Right arrow.
    Right,
    /// The Up arrow.
    Up,
    /// The Down arrow.
    Down,
    /// Home: the first cell.
    Home,
    /// End: the last cell.
    End,
    /// Page Up: [`GridNav::page`] rows up.
    PageUp,
    /// Page Down: [`GridNav::page`] rows down.
    PageDown,
}

impl GridKey {
    /// The grid key for an egui key, if it is one.
    pub fn from_key(key: Key) -> Option<Self> {
        Some(match key {
            Key::ArrowLeft => Self::Left,
            Key::ArrowRight => Self::Right,
            Key::ArrowUp => Self::Up,
            Key::ArrowDown => Self::Down,
            Key::Home => Self::Home,
            Key::End => Self::End,
            Key::PageUp => Self::PageUp,
            Key::PageDown => Self::PageDown,
            _ => return None,
        })
    }
}

/// Where the lead went, and the column Up and Down keep aiming at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridStep {
    /// The new cell.
    pub index: usize,
    /// The column to pass as `goal` to the next step: a vertical move through a short row (a
    /// header) returns to it, a horizontal move sets it to the column reached.
    pub goal: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Rows {
    /// `len` cells, `columns` to a row.
    Uniform { len: usize, columns: usize },
    /// Rows of their own length: `starts` holds the first cell of each non-empty row.
    Ragged { starts: Vec<usize>, len: usize },
}

/// Moves of a lead over cells numbered in reading order, row after row. A wrapped grid has equal
/// rows ([`wrapped`](GridNav::wrapped)); a list with header rows between runs of cells has rows
/// of any length ([`ragged`](GridNav::ragged)), a header being a row of one cell. Pure: it reads
/// no input and draws nothing.
///
/// - Left and Right move one cell along the reading order and on to the next or previous row
///   (mirrored when [`rtl`](GridNav::rtl)); the ends stay.
/// - Up and Down go to the goal column of the neighbouring row, or its last cell when it is
///   shorter; the first and last row stay.
/// - Home and End go to the first and last cell; Page Up and Down move [`page`](GridNav::page)
///   rows, stopping at the first or last row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridNav {
    rows: Rows,
    rtl: bool,
    page: usize,
}

impl GridNav {
    /// `len` cells wrapped `columns` to a row (at least one).
    pub fn wrapped(len: usize, columns: usize) -> Self {
        Self { rows: Rows::Uniform { len, columns: columns.max(1) }, rtl: false, page: 1 }
    }

    /// Rows with the given numbers of cells, in order; empty rows are left out.
    pub fn ragged(row_lengths: impl IntoIterator<Item = usize>) -> Self {
        let mut starts = Vec::new();
        let mut len = 0;
        for row in row_lengths.into_iter().filter(|&n| n > 0) {
            starts.push(len);
            len += row;
        }
        Self { rows: Rows::Ragged { starts, len }, rtl: false, page: 1 }
    }

    /// Right-to-left reading: Left moves to the next cell, Right to the previous.
    #[must_use]
    pub fn rtl(mut self, rtl: bool) -> Self {
        self.rtl = rtl;
        self
    }

    /// How many rows Page Up and Page Down move (at least one); the rows in view, less one.
    #[must_use]
    pub fn page(mut self, rows: usize) -> Self {
        self.page = rows.max(1);
        self
    }

    /// How many columns of `cell` points, `gap` apart, fit in `width` between two insets of
    /// `inset` points: `max(1, floor((width - 2 × inset + gap) / (cell + gap)))`.
    pub fn columns(width: f32, inset: f32, gap: f32, cell: f32) -> usize {
        let fit = ((width - 2.0 * inset + gap) / (cell + gap).max(1.0)).floor();
        if fit.is_finite() && fit >= 1.0 { fit as usize } else { 1 }
    }

    /// How many cells there are.
    pub fn len(&self) -> usize {
        match &self.rows {
            Rows::Uniform { len, .. } | Rows::Ragged { len, .. } => *len,
        }
    }

    /// Whether there are no cells.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The row of cell `index` (the last row for an index past the end).
    pub fn row_of(&self, index: usize) -> usize {
        match &self.rows {
            Rows::Uniform { len, columns } => index.min(len.saturating_sub(1)) / columns,
            Rows::Ragged { starts, len } => starts.partition_point(|&s| s <= index.min(len.saturating_sub(1))).saturating_sub(1),
        }
    }

    /// The cells of row `row` (empty past the last row).
    pub fn row_range(&self, row: usize) -> Range<usize> {
        match &self.rows {
            Rows::Uniform { len, columns } => (row * columns).min(*len)..((row + 1) * columns).min(*len),
            Rows::Ragged { starts, len } => match starts.get(row) {
                Some(&start) => start..starts.get(row + 1).copied().unwrap_or(*len),
                None => *len..*len,
            },
        }
    }

    /// How many rows there are.
    pub fn row_count(&self) -> usize {
        match &self.rows {
            Rows::Uniform { len, columns } => len.div_ceil(*columns),
            Rows::Ragged { starts, .. } => starts.len(),
        }
    }

    /// The column of cell `index` in its row.
    pub fn column_of(&self, index: usize) -> usize {
        index.min(self.len().saturating_sub(1)) - self.row_range(self.row_of(index)).start
    }

    /// The cell `key` moves the lead from `from` to. `goal` is the column Up, Down, Page Up and
    /// Page Down aim at: `None` for the column of `from`, else the [`GridStep::goal`] of the
    /// previous step while the user keeps moving vertically. Without cells it returns 0.
    pub fn step(&self, from: usize, key: GridKey, goal: Option<usize>) -> GridStep {
        let last = self.len().saturating_sub(1);
        let from = from.min(last);
        let here = self.column_of(from);
        let horizontal = |index: usize| GridStep { index, goal: self.column_of(index) };
        match key {
            _ if self.is_empty() => GridStep { index: from, goal: here },
            GridKey::Left | GridKey::Right => {
                let forward = (key == GridKey::Right) != self.rtl;
                horizontal(if forward { (from + 1).min(last) } else { from.saturating_sub(1) })
            }
            GridKey::Home => horizontal(0),
            GridKey::End => horizontal(last),
            GridKey::Up => self.vertical(from, goal.unwrap_or(here), -1),
            GridKey::Down => self.vertical(from, goal.unwrap_or(here), 1),
            GridKey::PageUp => self.vertical(from, goal.unwrap_or(here), -(self.page as isize)),
            GridKey::PageDown => self.vertical(from, goal.unwrap_or(here), self.page as isize),
        }
    }

    fn vertical(&self, from: usize, goal: usize, rows: isize) -> GridStep {
        let last_row = self.row_count().saturating_sub(1);
        let row = self.row_of(from).saturating_add_signed(rows).min(last_row);
        let cells = self.row_range(row);
        let index = cells.start + goal.min(cells.len().saturating_sub(1));
        GridStep { index, goal }
    }

    /// The cell that is the grid's Tab stop this frame, over cells whose widgets have the ids
    /// `ids` in reading order and whose rects are known from the last frame: the focused one,
    /// moved by this frame's arrow keys, Home and End (which it takes) or a screen reader's Focus
    /// request, else `selected`, else the first. A cell the focus moves to is focused at once.
    /// The columns are those of the first row of rects, so a grid laid out by wrapping works
    /// without knowing its width.
    pub fn roving_stop(ui: &Ui, ids: &[Id], selected: Option<usize>, enabled: bool) -> usize {
        let resting = selected.unwrap_or(0);
        if !enabled || ids.is_empty() {
            return resting;
        }
        let focused = ids.iter().position(|&id| ui.memory(|m| m.has_focus(id)));
        let mut stop = focused.unwrap_or(resting);
        if let Some(requested) = ids.iter().position(|&id| ui.input(|i| i.has_accesskit_action_request(id, Action::Focus))) {
            stop = requested;
        }
        if let Some(from) = focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(ids[from], EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() })
            });
            let rects: Vec<Option<Rect>> = ids.iter().map(|&id| ui.ctx().read_response(id).map(|r| r.rect)).collect();
            let nav = Self::wrapped(ids.len(), first_row_len(&rects)).rtl(direction::is_rtl(ui));
            ui.input_mut(|i| {
                for (key, grid_key) in [
                    (Key::ArrowRight, GridKey::Right),
                    (Key::ArrowLeft, GridKey::Left),
                    (Key::ArrowDown, GridKey::Down),
                    (Key::ArrowUp, GridKey::Up),
                    (Key::Home, GridKey::Home),
                    (Key::End, GridKey::End),
                ] {
                    if i.consume_key(Modifiers::NONE, key) {
                        stop = nav.step(from, grid_key, None).index;
                    }
                }
            });
        }
        if focused.is_some_and(|f| f != stop) || (focused.is_none() && stop != resting) {
            ui.memory_mut(|m| m.request_focus(ids[stop]));
        }
        stop
    }
}

/// How many cells share the first cell's row (all of them when its rect is unknown).
fn first_row_len(rects: &[Option<Rect>]) -> usize {
    let Some(Some(first)) = rects.first() else {
        return rects.len().max(1);
    };
    rects.iter().take_while(|r| r.is_some_and(|r| (r.min.y - first.min.y).abs() < 1.0)).count().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(nav: &GridNav, from: usize, key: GridKey) -> usize {
        nav.step(from, key, None).index
    }

    #[test]
    fn arrows_cross_rows_and_stop_at_the_ends() {
        let nav = GridNav::wrapped(7, 3);
        assert_eq!(go(&nav, 2, GridKey::Right), 3);
        assert_eq!(go(&nav, 3, GridKey::Left), 2);
        assert_eq!(go(&nav, 6, GridKey::Right), 6);
        assert_eq!(go(&nav, 0, GridKey::Left), 0);
        assert_eq!(go(&nav, 1, GridKey::Down), 4);
        assert_eq!(go(&nav, 4, GridKey::Up), 1);
        assert_eq!(go(&nav, 1, GridKey::Up), 1);
        assert_eq!(go(&nav, 5, GridKey::Down), 6, "the short last row ends at its last cell");
        assert_eq!(go(&nav, 6, GridKey::Down), 6);
    }

    #[test]
    fn rtl_mirrors_left_and_right_only() {
        let nav = GridNav::wrapped(7, 3).rtl(true);
        assert_eq!(go(&nav, 2, GridKey::Left), 3);
        assert_eq!(go(&nav, 3, GridKey::Right), 2);
        assert_eq!(go(&nav, 1, GridKey::Down), 4);
        assert_eq!(go(&nav, 1, GridKey::Home), 0);
    }

    #[test]
    fn home_end_and_pages() {
        let nav = GridNav::wrapped(20, 4).page(2);
        assert_eq!(go(&nav, 9, GridKey::Home), 0);
        assert_eq!(go(&nav, 9, GridKey::End), 19);
        assert_eq!(go(&nav, 9, GridKey::PageDown), 17);
        assert_eq!(go(&nav, 17, GridKey::PageDown), 17, "the last row is the limit");
        assert_eq!(go(&nav, 9, GridKey::PageUp), 1);
        assert_eq!(go(&nav, 1, GridKey::PageUp), 1);
    }

    #[test]
    fn rows_of_unequal_length_keep_the_goal_column() {
        // A header, three cells of four wide rows, a header, two cells.
        let nav = GridNav::ragged([1, 4, 2, 1, 2]);
        assert_eq!(nav.len(), 10);
        let down = nav.step(4, GridKey::Down, None);
        assert_eq!((down.index, down.goal), (6, 3), "column 3 of a two-cell row is its last");
        let again = nav.step(down.index, GridKey::Down, Some(down.goal));
        assert_eq!(again.index, 7, "a header is one cell");
        let through = nav.step(again.index, GridKey::Down, Some(again.goal));
        assert_eq!(through.index, 9, "the goal column survives the header");
        let back = nav.step(1, GridKey::Up, None);
        assert_eq!(back.index, 0);
        assert_eq!(nav.step(0, GridKey::Right, None).index, 1, "left and right walk over the header");
    }

    #[test]
    fn empty_rows_are_ignored_and_nothing_moves_without_cells() {
        let nav = GridNav::ragged([0, 2, 0, 1]);
        assert_eq!((nav.row_count(), nav.len()), (2, 3));
        assert_eq!(nav.row_range(1), 2..3);
        assert_eq!(go(&nav, 0, GridKey::Down), 2);
        let none = GridNav::wrapped(0, 3);
        assert_eq!(go(&none, 0, GridKey::Right), 0);
        assert_eq!(go(&none, 5, GridKey::End), 0);
    }

    #[test]
    fn columns_follow_the_width() {
        assert_eq!(GridNav::columns(100.0, 8.0, 4.0, 22.0), 3);
        assert_eq!(GridNav::columns(10.0, 8.0, 4.0, 22.0), 1);
        assert_eq!(GridNav::columns(f32::NAN, 8.0, 4.0, 22.0), 1);
        assert_eq!(GridNav::columns(100.0, 0.0, 0.0, 25.0), 4, "flush");
    }

    #[test]
    fn first_row_of_rects_gives_the_columns() {
        let at = |x: f32, y: f32| Some(Rect::from_min_size(egui::pos2(x, y), egui::vec2(10.0, 10.0)));
        assert_eq!(first_row_len(&[at(0.0, 0.0), at(10.0, 0.0), at(20.0, 0.0), at(0.0, 10.0)]), 3);
        assert_eq!(first_row_len(&[None, at(0.0, 0.0)]), 2);
        assert_eq!(first_row_len(&[]), 1);
    }
}
