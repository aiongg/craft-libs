//! The rename bar (DS-44 task 4).

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Align, Id, Layout, Ui, UiBuilder};

use super::labels::PresetLabels;
use crate::controls::TextField;
use crate::direction;
use crate::edit::{CancelReason, EditEvent};
use crate::primitives::Interactive;
use craft_theme::ControlSize;

/// How the bar ended.
pub(super) enum Ended {
    /// Enter or a click elsewhere, with the typed text: it may be empty or unchanged.
    Commit(String),
    /// Escape.
    Cancel,
}

/// The text typed so far, and whose rename it is: a different `owner` starts over.
#[derive(Clone)]
struct Draft {
    owner: Id,
    text: String,
}

/// Shows the bar for the preset or group `owner` named `current`: the label and a field with the
/// whole name selected, focused at once. `Some` when it ends.
pub(super) fn show(ui: &mut Ui, bar: Id, owner: Id, current: &str, labels: &PresetLabels) -> Option<Ended> {
    let memory = bar.with("draft");
    let stored = ui.data(|d| d.get_temp::<Option<Draft>>(memory)).flatten().filter(|d| d.owner == owner);
    let fresh = stored.is_none();
    let draft = stored.map_or_else(|| current.to_owned(), |d| d.text);
    let mut ended = None;
    let mut text = draft.clone();
    direction::horizontal(ui, |ui| {
        ui.label(labels.name.as_str());
        let rest = ui.available_rect_before_wrap();
        let mut child = ui.new_child(UiBuilder::new().id_salt(bar.with("field")).max_rect(rest).layout(Layout::top_down_justified(Align::Min)));
        let field = Interactive::id_in(&child, "rename");
        if fresh {
            child.memory_mut(|m| m.request_focus(field));
            let mut state = TextEditState::default();
            state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(draft.chars().count()))));
            state.store(child.ctx(), field);
        }
        let edit = TextField::new("rename", labels.name.as_str(), &draft).size(ControlSize::Md).show(&mut child);
        ui.allocate_rect(child.min_rect(), egui::Sense::hover());
        let mut cancelled = false;
        match &edit.event {
            Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => text.clone_from(value),
            Some(EditEvent::Cancel { reason: CancelReason::Escape, .. }) => cancelled = true,
            _ => {}
        }
        if cancelled || edit.escaped {
            ended = Some(Ended::Cancel);
        } else if edit.submitted || (edit.response.lost_focus() && !fresh) {
            ended = Some(Ended::Commit(text.clone()));
        }
    });
    let next = if ended.is_some() { None } else { Some(Draft { owner, text }) };
    ui.data_mut(|d| d.insert_temp(memory, next));
    ended
}
