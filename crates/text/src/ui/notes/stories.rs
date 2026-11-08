//! Stories of the notes editor.

use craft_ui::contract::{Props as _, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::{NotesEditor, NotesParagraph, NotesRun, NotesText};
use crate::lists::ListMarker;

fn bullets() -> NotesText {
    let run = |text: &str, bold: bool| NotesRun { text: text.to_owned(), bold, ..NotesRun::default() };
    NotesText {
        paragraphs: vec![
            NotesParagraph {
                runs: vec![run("Open with ", false), run("the demo", true)],
                level: 0,
                marker: Some(ListMarker::Bullet("•".into())),
                ..NotesParagraph::default()
            },
            NotesParagraph {
                runs: vec![run("Then the numbers", false)],
                level: 1,
                marker: Some(ListMarker::Bullet("•".into())),
                ..NotesParagraph::default()
            },
        ],
    }
}

#[story(NotesEditor<'static>, covers = [], drive = Drive::Never)]
fn editing_bullets(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let notes = data.value("notes", bullets());
    NotesEditor::new("Notes", notes).show(ui).response.into()
}

#[story(NotesEditor<'static>, covers = [], drive = Drive::Never)]
fn read_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let notes = data.value("notes", bullets());
    NotesEditor::new("Notes", notes).read_only(true).show(ui).response.into()
}
