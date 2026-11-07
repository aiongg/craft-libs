//! Selection models: what a collection's selection holds and how a change applies to it.

use crate::columns::ColumnKey;
use crate::value::Value;
use std::ops::Range;

/// A change to a selection, as a collection reports it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SelectionChange<K> {
    /// Replaces the selection; the last key becomes the lead.
    Set(Vec<K>),
    /// Adds the key when absent, removes it when present.
    Toggle(K),
    /// Empties the selection.
    Clear,
    /// Replaces the selection by the cells of `rows` in `columns`; `lead_column` is the lead cell's column.
    /// A model with no columns selects the rows.
    Cells(CellRange<K>),
    /// Adds the cell when it is not selected, removes it when it is; the other cells stay. A model of rows toggles
    /// the row.
    ToggleCell(CellAt<K>),
}

/// The cell a [`SelectionChange::ToggleCell`] flips.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CellAt<K> {
    /// The cell's row.
    pub row: K,
    /// The cell's column position.
    pub column: u32,
    /// The cell's column key, which becomes the lead cell's column.
    pub column_key: ColumnKey,
}

impl<K> CellAt<K> {
    /// The cell of `row` at column position `column`, whose column is `column_key`.
    pub fn new(row: K, column: u32, column_key: ColumnKey) -> Self {
        Self { row, column, column_key }
    }
}

/// The cells a [`SelectionChange::Cells`] selects.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CellRange<K> {
    /// The selected rows, from the first to the last of the range.
    pub rows: Vec<K>,
    /// The column positions the range covers.
    pub columns: Range<u32>,
    /// The column of the lead cell.
    pub lead_column: ColumnKey,
}

impl<K> CellRange<K> {
    /// The cells of `rows` in `columns`, with the lead cell in `lead_column`.
    pub fn new(rows: Vec<K>, columns: Range<u32>, lead_column: ColumnKey) -> Self {
        Self { rows, columns, lead_column }
    }
}

/// The selection a collection reads and the app owns.
pub trait SelectionModel<K>: Value + Default {
    /// Whether `key` is selected.
    fn contains(&self, key: &K) -> bool;
    /// The item keyboard navigation moves.
    fn lead(&self) -> Option<&K>;
    /// The fixed end of a range extended with Shift.
    fn anchor(&self) -> Option<&K>;
    /// Applies a change.
    fn apply(&mut self, change: SelectionChange<K>);

    /// The column of the lead cell; `None` for a model of rows.
    fn lead_column(&self) -> Option<&ColumnKey> {
        None
    }

    /// Whether the cell of row `key` in column position `column` is selected.
    fn contains_cell(&self, key: &K, _column: u32) -> bool {
        self.contains(key)
    }
}

/// A set of keys with a lead and an anchor.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Selection<K> {
    /// The selected keys.
    pub items: Vec<K>,
    /// The lead.
    pub lead: Option<K>,
    /// The anchor.
    pub anchor: Option<K>,
}

impl<K> Selection<K> {
    /// A selection of `items` with `lead` and `anchor`.
    pub fn new(items: Vec<K>, lead: Option<K>, anchor: Option<K>) -> Self {
        Selection { items, lead, anchor }
    }
}

impl<K> Default for Selection<K> {
    fn default() -> Self {
        Selection { items: Vec::new(), lead: None, anchor: None }
    }
}

impl<K: Value> SelectionModel<K> for Selection<K> {
    fn contains(&self, key: &K) -> bool {
        self.items.contains(key)
    }

    fn lead(&self) -> Option<&K> {
        self.lead.as_ref()
    }

    fn anchor(&self) -> Option<&K> {
        self.anchor.as_ref()
    }

    fn apply(&mut self, change: SelectionChange<K>) {
        match change {
            SelectionChange::Set(keys) => {
                self.anchor = keys.first().cloned();
                self.lead = keys.last().cloned();
                self.items = keys;
            }
            SelectionChange::Toggle(key) => {
                match self.items.iter().position(|k| *k == key) {
                    Some(at) => {
                        self.items.remove(at);
                    }
                    None => self.items.push(key.clone()),
                }
                self.anchor = Some(key.clone());
                self.lead = Some(key);
            }
            SelectionChange::Clear => {
                self.items.clear();
                self.lead = None;
                self.anchor = None;
            }
            SelectionChange::Cells(CellRange { rows, .. }) => self.apply(SelectionChange::Set(rows)),
            SelectionChange::ToggleCell(CellAt { row, .. }) => self.apply(SelectionChange::Toggle(row)),
        }
    }
}

/// Cell selection of a sheet: row keys with a lead column, cell ranges and single cells toggled over them. Changes
/// report as selection changes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CellSelection<K> {
    /// The selected rows.
    pub rows: Selection<K>,
    /// The column of the lead cell.
    pub lead_column: Option<ColumnKey>,
    /// Cell ranges: first row, last row and the columns they cover.
    pub ranges: Vec<(K, K, Range<u32>)>,
    /// Cells toggled over the rows and ranges: row, column position and whether the cell is selected. Only cells
    /// that differ from what the rows and ranges say are held, so a range over a million rows stays one entry.
    pub toggled: Vec<(K, u32, bool)>,
}

impl<K> CellSelection<K> {
    /// A selection of `rows`, the column of the lead cell and cell `ranges`.
    pub fn new(rows: Selection<K>, lead_column: Option<ColumnKey>, ranges: Vec<(K, K, Range<u32>)>) -> Self {
        CellSelection { rows, lead_column, ranges, toggled: Vec::new() }
    }
}

impl<K> Default for CellSelection<K> {
    fn default() -> Self {
        CellSelection { rows: Selection::default(), lead_column: None, ranges: Vec::new(), toggled: Vec::new() }
    }
}

impl<K: Value> SelectionModel<K> for CellSelection<K> {
    fn contains(&self, key: &K) -> bool {
        self.rows.contains(key) || self.toggled.iter().any(|(row, _, on)| *on && row == key)
    }

    fn lead(&self) -> Option<&K> {
        self.rows.lead()
    }

    fn anchor(&self) -> Option<&K> {
        self.rows.anchor()
    }

    fn apply(&mut self, change: SelectionChange<K>) {
        match change {
            SelectionChange::ToggleCell(CellAt { row, column, column_key }) => {
                let on = !self.contains_cell(&row, column);
                self.toggled.retain(|(held, at, _)| !(*held == row && *at == column));
                if on != self.in_rows_and_ranges(&row, column) {
                    self.toggled.push((row.clone(), column, on));
                }
                self.lead_column = Some(column_key);
                self.rows.anchor = Some(row.clone());
                self.rows.lead = Some(row);
            }
            SelectionChange::Cells(CellRange { rows, columns, lead_column, .. }) => {
                self.toggled.clear();
                if let (Some(first), Some(last)) = (rows.first().cloned(), rows.last().cloned()) {
                    self.ranges = vec![(first, last, columns)];
                }
                self.lead_column = Some(lead_column);
                self.rows.apply(SelectionChange::Set(rows));
            }
            SelectionChange::Clear => {
                self.ranges.clear();
                self.toggled.clear();
                self.lead_column = None;
                self.rows.apply(SelectionChange::Clear);
            }
            change => {
                self.ranges.clear();
                self.toggled.clear();
                self.rows.apply(change);
            }
        }
    }

    fn lead_column(&self) -> Option<&ColumnKey> {
        self.lead_column.as_ref()
    }

    /// A toggled cell is as it was toggled. Any other cell is selected when its row is and a range covers its
    /// column; rows selected without a range select every column.
    fn contains_cell(&self, key: &K, column: u32) -> bool {
        match self.toggled.iter().find(|(row, at, _)| row == key && *at == column) {
            Some((_, _, on)) => *on,
            None => self.in_rows_and_ranges(key, column),
        }
    }
}

impl<K: Value> CellSelection<K> {
    fn in_rows_and_ranges(&self, key: &K, column: u32) -> bool {
        self.rows.contains(key) && (self.ranges.is_empty() || self.ranges.iter().any(|(_, _, columns)| columns.contains(&column)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_anchors_at_the_first_key_and_leads_with_the_last() {
        let mut selection = Selection::<u32>::default();
        selection.apply(SelectionChange::Set(vec![3, 1, 2]));
        assert_eq!((selection.anchor(), selection.lead()), (Some(&3), Some(&2)));
        selection.apply(SelectionChange::Set(vec![]));
        assert_eq!((selection.anchor(), selection.lead()), (None, None));
    }

    #[test]
    fn toggle_moves_the_lead_and_anchor_to_the_toggled_key_whether_it_turns_on_or_off() {
        let mut selection = Selection::<u32>::default();
        selection.apply(SelectionChange::Set(vec![3, 2]));
        selection.apply(SelectionChange::Toggle(7));
        assert_eq!((selection.items.clone(), selection.lead(), selection.anchor()), (vec![3, 2, 7], Some(&7), Some(&7)));
        selection.apply(SelectionChange::Toggle(3));
        assert_eq!((selection.items.clone(), selection.lead(), selection.anchor()), (vec![2, 7], Some(&3), Some(&3)));
    }

    #[test]
    fn a_cell_range_selects_its_rows_and_columns_and_a_row_change_selects_every_column() {
        let mut cells = CellSelection::<u32>::default();
        cells.apply(SelectionChange::Cells(CellRange::new(vec![2, 3], 1..3, ColumnKey::new("c"))));
        assert_eq!((cells.contains_cell(&3, 2), cells.contains_cell(&3, 0), cells.contains_cell(&5, 1)), (true, false, false));
        assert_eq!((cells.lead(), cells.lead_column()), (Some(&3), Some(&ColumnKey::new("c"))));
        cells.apply(SelectionChange::Set(vec![4]));
        assert_eq!((cells.contains_cell(&4, 0), cells.contains_cell(&4, 9)), (true, true));
    }

    #[test]
    fn a_cell_toggle_flips_one_cell_and_leaves_the_others() {
        let at = |row, column| SelectionChange::ToggleCell(CellAt::new(row, column, ColumnKey::new("c")));
        let mut cells = CellSelection::<u32>::default();
        cells.apply(SelectionChange::Cells(CellRange::new(vec![2, 3], 1..3, ColumnKey::new("c"))));
        cells.apply(at(7, 0));
        assert_eq!(
            (cells.contains_cell(&7, 0), cells.contains_cell(&7, 1), cells.contains_cell(&2, 1), cells.contains(&7)),
            (true, false, true, true)
        );
        cells.apply(at(2, 1));
        assert_eq!((cells.contains_cell(&2, 1), cells.contains_cell(&2, 2), cells.contains_cell(&3, 1)), (false, true, true));
        cells.apply(at(2, 1));
        assert_eq!((cells.toggled.len(), cells.contains_cell(&2, 1)), (1, true));
        cells.apply(at(7, 0));
        assert_eq!((cells.toggled.len(), cells.contains(&7), cells.lead()), (0, false, Some(&7)));
        cells.apply(at(5, 2));
        cells.apply(SelectionChange::Set(vec![4]));
        assert_eq!((cells.contains_cell(&5, 2), cells.contains_cell(&4, 0)), (false, true));
    }

    #[test]
    fn clear_forgets_the_lead_and_anchor() {
        let mut selection = Selection::<u32>::default();
        selection.apply(SelectionChange::Set(vec![1, 2]));
        selection.apply(SelectionChange::Clear);
        assert_eq!((selection.items.len(), selection.lead(), selection.anchor()), (0, None, None));
    }
}
