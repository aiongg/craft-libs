//! The rename field of a [`ListRow`](super::ListRow).

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Align, Event, Id, Key, Layout, Rect, Ui, UiBuilder, vec2};

use super::row::{RenameEnd, RowEvent};
use crate::controls::TextField;
use crate::edit::{CancelReason, EditEvent};
use crate::primitives::Interactive;
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::ControlSize;

/// The open rename, kept in egui memory under the row's rename id: the text typed, so that the
/// field shows it whatever the host's label says, and the last Tab press (true with Shift). Moving
/// the focus backwards takes effect a pass after the key, so the end of the rename sees the key
/// from the pass before.
#[derive(Clone, Debug)]
struct Draft {
    text: String,
    tab: Option<bool>,
}

/// Takes the text typed into the open rename of the row `row_id`, closing it; `None` when no
/// field was shown for it.
pub(super) fn take_draft(ctx: &egui::Context, row_id: Id) -> Option<String> {
    let memory = row_id.with("rename");
    let draft = ctx.data(|d| d.get_temp::<Option<Draft>>(memory)).flatten();
    ctx.data_mut(|d| d.insert_temp(memory, None::<Draft>));
    draft.map(|d| d.text)
}

/// Shows the rename field in `band` and reports how the rename ends, if it does.
pub(super) fn show<K: Clone>(ui: &mut Ui, row_id: Id, key: &K, label: &str, band: Rect, events: &mut Vec<RowEvent<K>>) {
    let memory = row_id.with("rename");
    let stored = ui.data(|d| d.get_temp::<Option<Draft>>(memory)).flatten();
    let fresh = stored.is_none();
    let previous_tab = stored.as_ref().and_then(|d| d.tab);
    let draft = stored.map_or_else(|| label.to_owned(), |d| d.text);
    let height = TextField::style(&Tokens::of(ui), ControlSize::Sm).height.min(band.height());
    let band = Rect::from_center_size(band.center(), vec2(band.width(), height));
    let layout = Layout::top_down_justified(Align::Min);
    // A child, not a scope: a scope would move the parent's cursor into the row it was allocated.
    let mut child = ui.new_child(UiBuilder::new().id_salt(row_id.with("rename-scope")).max_rect(band).layout(layout));
    let edit = {
        let ui = &mut child;
        // The id `TextField` gives its salt.
        let field = Interactive::id_in(ui, "rename");
        if fresh {
            ui.memory_mut(|m| m.request_focus(field));
            let mut state = TextEditState::default();
            state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(draft.chars().count()))));
            state.store(ui.ctx(), field);
        }
        TextField::new("rename", label, &draft).size(ControlSize::Sm).show(ui)
    };
    let pressed_tab = ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            Event::Key { key: Key::Tab, pressed: true, modifiers, .. } => Some(modifiers.shift),
            _ => None,
        })
    });
    let tab = pressed_tab.or(previous_tab);
    let mut text = draft;
    let mut cancelled = false;
    match &edit.event {
        Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => text.clone_from(value),
        Some(EditEvent::Cancel { reason: CancelReason::Escape, .. }) => cancelled = true,
        _ => {}
    }
    let end = if cancelled || edit.escaped {
        Some(RenameEnd::Escape)
    } else if edit.submitted {
        Some(RenameEnd::Enter)
    } else if edit.response.lost_focus() && !fresh {
        Some(match tab {
            Some(true) => RenameEnd::ShiftTab,
            Some(false) => RenameEnd::Tab,
            None => RenameEnd::Blur,
        })
    } else {
        None
    };
    match end {
        Some(end) => {
            ui.data_mut(|d| d.insert_temp(memory, None::<Draft>));
            if end != RenameEnd::Escape && text != label {
                events.push(RowEvent::Renamed { row: key.clone(), text });
            }
            events.push(RowEvent::RenameEnded { row: key.clone(), end });
        }
        None => {
            ui.data_mut(|d| d.insert_temp(memory, Some(Draft { text, tab })));
        }
    }
}
