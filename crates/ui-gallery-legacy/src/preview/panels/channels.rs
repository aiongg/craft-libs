//! The Channels panel: the composite and the three colour channels, static apart from the eyes.

use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::edit::EditEvent;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ToggleColumn};
use craft_ui::patterns::Panel;
use egui::Ui;

const CHANNELS: [&str; 4] = ["RGB", "Red", "Green", "Blue"];

#[derive(Clone, Debug)]
pub struct State {
    visible: [bool; 4],
    selection: Selection<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self { visible: [true; 4], selection: Selection::only(0) }
    }
}

impl State {
    /// An eye on the composite sets every channel; an eye on a channel sets the composite when
    /// all three agree.
    fn set(&mut self, row: usize, on: bool) {
        if row == 0 {
            self.visible = [on; 4];
        } else {
            self.visible[row] = on;
            self.visible[0] = self.visible[1..].iter().all(|v| *v);
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut State) {
    Panel::new("channels").title("Channels").scroll(false).show(ui, |ui| {
        let order: Vec<RowInfo<usize>> = (0..CHANNELS.len()).map(RowInfo::leaf).collect();
        let visible = state.visible;
        let out = List::new("channels.list", "Channels", order.as_slice())
            .selection(&mut state.selection)
            .mode(SelectionMode::Single)
            .allow_empty(false)
            .show(ui, |_, &row| ListRow::new(row, CHANNELS[row]).toggle(ToggleColumn::eye("Visibility {row}", visible[row])).dimmed(!visible[row]));
        for event in out.events {
            if let ListEvent::Row(RowEvent::Toggle(
                EditEvent::Begin { value: Some(value), .. }
                | EditEvent::Update { value, .. }
                | EditEvent::Set { value, .. }
                | EditEvent::Commit { value, .. },
            )) = event
            {
                state.set(value.row, value.on);
            }
        }
    });
}
