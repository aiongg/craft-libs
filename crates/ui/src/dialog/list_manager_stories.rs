//! Story of the list manager dialog, GridCraft's Macros dialog made general: a name box over a
//! list, a column of action buttons, the selected entry's description and a note. The story
//! lets Delete remove an entry (the dialog stays open), so the app's `set_entries` shows.

use egui::Ui;

use super::{ListManagerAction, ListManagerDialog, ListManagerEntry, ListManagerLabels};
use crate::localized::Labels as _;
use crate::story::drive;
use crate::story::{Level, Story, StoryArgs, StoryLabels};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct ListManagerArgs {
    open: bool,
    searchable: bool,
    empty: bool,
    note: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/dialog/list-manager",
        Level::Patterns,
        "Dialog",
        "List manager",
        ListManagerArgs { open: true, searchable: false, empty: false, note: true },
        show_list_manager
    )]
}

fn entries(words: &StoryLabels) -> Vec<ListManagerEntry> {
    let entry = |id: &str, name: &str, detail: Option<&str>, description: &str, kind| ListManagerEntry {
        id: id.to_owned(),
        name: name.to_owned(),
        detail: detail.map(str::to_owned),
        description: description.to_owned(),
        kind,
    };
    vec![
        entry("brighten", &words.macro_brighten, Some("Ctrl+Shift+B"), &words.macro_brighten_info, 1),
        entry("sharpen", &words.macro_sharpen, None, &words.macro_sharpen_info, 1),
        entry("resize", &words.macro_resize, Some("Ctrl+Alt+W"), &words.macro_resize_info, 2),
    ]
}

/// Macros: Run and Edit close the dialog with the answer, Create needs a new name, Delete keeps
/// the dialog open and removes the entry.
fn show_list_manager(ui: &mut Ui, args: &mut ListManagerArgs) {
    let (words, labels, ctx) = (StoryLabels::of(ui), ListManagerLabels::of(ui), ui.ctx().clone());
    let rows_key = ui.make_persistent_id("craft_ui::dialog::list_manager_stories::rows");
    let (searchable, empty, note) = (args.searchable, args.empty, args.note);
    drive(
        ui,
        "list-manager",
        &mut args.open,
        (searchable, empty, note),
        || {
            let rows = if empty { Vec::new() } else { entries(&words) };
            ctx.data_mut(|d| d.insert_temp(rows_key, rows.clone()));
            let dialog = ListManagerDialog::new("macros", words.macros.as_str(), words.macro_name.as_str())
                .entries(rows)
                .actions(vec![
                    ListManagerAction::run(&labels),
                    ListManagerAction::edit(&labels).kind(1),
                    ListManagerAction::create(&labels),
                    ListManagerAction::delete(&labels),
                ])
                .searchable(searchable)
                .empty_text(words.macros_empty.as_str());
            if note { dialog.note(words.macros_note.as_str()) } else { dialog }
        },
        |dialog| {
            if let Some(choice) = dialog.take_action()
                && choice.action == "delete"
            {
                let mut rows: Vec<ListManagerEntry> = ctx.data(|d| d.get_temp(rows_key)).unwrap_or_default();
                rows.retain(|row| row.name != choice.name);
                dialog.set_entries(rows.clone());
                ctx.data_mut(|d| d.insert_temp(rows_key, rows));
            }
        },
        |choice| craft_i18n::fmt(&words.result_ok, &[("value", &format!("{} {:?}, {:?}", choice.action, choice.entry, choice.name))]),
    );
}
