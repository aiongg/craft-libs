//! The Stories panel: the target component's stories.
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::story::Story;
use egui::Ui;

/// The stories of `target`'s component with `target` selected. Returns the id of the story that
/// was picked last.
pub(super) fn show(ui: &mut Ui, all: &[Story], target: &Story) -> Option<&'static str> {
    let stories: Vec<Story> = all.iter().filter(|s| s.component == target.component).copied().collect();
    let rows: Vec<RowInfo<&'static str>> = stories.iter().map(|s| RowInfo::leaf(s.id)).collect();
    let mut selection = Selection::only(target.id);
    let out = List::new("gallery.stories", "Stories", rows.as_slice())
        .selection(&mut selection)
        .mode(SelectionMode::Single)
        .show(ui, |_, id| ListRow::new(*id, stories.iter().find(|s| s.id == *id).map_or(*id, |s| s.name)));
    let mut picked = None;
    for event in out.events {
        if let ListEvent::Selection(SelectionChange { row: Some(id), .. }) | ListEvent::Activate(id) = event {
            picked = Some(id);
        }
    }
    picked
}
