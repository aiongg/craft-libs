//! The History panel: what happened to the document, newest last. Selecting a row only
//! highlights it; the mock has no undo.

use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::list::{List, ListRow};
use craft_ui::patterns::Panel;
use egui::Ui;

use super::super::model::Mock;

#[derive(Clone, Debug, Default)]
pub struct State {
    selection: Selection<usize>,
    /// How many rows the last frame showed: a new row takes the selection, as Photoshop's does.
    seen: usize,
}

pub fn show(ui: &mut Ui, mock: &Mock, state: &mut State) {
    Panel::new("history").title("History").scroll(false).show(ui, |ui| {
        let count = mock.history.len();
        let newest = count.checked_sub(1);
        let grew = count != state.seen;
        if grew {
            state.selection = newest.map_or_else(Selection::new, Selection::only);
            state.seen = count;
        }
        let order: Vec<RowInfo<usize>> = (0..count).map(RowInfo::leaf).collect();
        let mut list = List::new("history.list", "History", order.as_slice()).selection(&mut state.selection).mode(SelectionMode::Single);
        if let Some(last) = newest.filter(|_| grew) {
            list = list.reveal(last);
        }
        let _ = list.show(ui, |_, &row| ListRow::new(row, mock.history[row].clone()));
    });
}
