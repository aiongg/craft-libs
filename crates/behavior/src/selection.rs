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
                self.lead = keys.last().cloned();
                self.items = keys;
            }
            SelectionChange::Toggle(key) => match self.items.iter().position(|k| *k == key) {
                Some(at) => {
                    self.items.remove(at);
                }
                None => self.items.push(key),
            },
            SelectionChange::Clear => self.items.clear(),
        }
    }
}

/// Cell selection of a sheet: row keys with a lead column and cell ranges. Changes report as selection changes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CellSelection<K> {
    /// The selected rows.
    pub rows: Selection<K>,
    /// The column of the lead cell.
    pub lead_column: Option<ColumnKey>,
    /// Cell ranges: first row, last row and the columns they cover.
    pub ranges: Vec<(K, K, Range<u32>)>,
}

impl<K> CellSelection<K> {
    /// A selection of `rows`, the column of the lead cell and cell `ranges`.
    pub fn new(rows: Selection<K>, lead_column: Option<ColumnKey>, ranges: Vec<(K, K, Range<u32>)>) -> Self {
        CellSelection { rows, lead_column, ranges }
    }
}

impl<K> Default for CellSelection<K> {
    fn default() -> Self {
        CellSelection { rows: Selection::default(), lead_column: None, ranges: Vec::new() }
    }
}

impl<K: Value> SelectionModel<K> for CellSelection<K> {
    fn contains(&self, key: &K) -> bool {
        self.rows.contains(key)
    }

    fn lead(&self) -> Option<&K> {
        self.rows.lead()
    }

    fn anchor(&self) -> Option<&K> {
        self.rows.anchor()
    }

    fn apply(&mut self, change: SelectionChange<K>) {
        self.rows.apply(change);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_leads_with_the_last_key_and_toggle_keeps_the_lead() {
        let mut selection = Selection::<u32>::default();
        selection.apply(SelectionChange::Set(vec![3, 1, 2]));
        assert_eq!(selection.lead(), Some(&2));
        selection.apply(SelectionChange::Toggle(1));
        selection.apply(SelectionChange::Toggle(7));
        assert_eq!(selection.items, vec![3, 2, 7]);
        assert_eq!(selection.lead(), Some(&2));
    }
}
