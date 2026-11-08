//! The notes text model: typing keeps the formatting around it.

use craft_text::ui::notes::{NotesEditor, NotesParagraph, NotesRun, NotesText};
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

fn run(text: &str, bold: bool) -> NotesRun {
    NotesRun::new(text).bold(bold)
}

fn paragraph(runs: Vec<NotesRun>) -> NotesParagraph {
    NotesParagraph::new(runs)
}

#[trial]
fn typing_keeps_the_formatting_around_it() {
    let notes = NotesText::new(vec![paragraph(vec![run("bold", true), run(" plain", false)])]);
    assert_eq!(notes.plain(), "bold plain");
    let typed = notes.typed("boxld plain").expect("one insertion");
    assert_eq!(typed.paragraphs[0].runs, vec![run("boxld", true), run(" plain", false)]);
    let cut = notes.typed("bold plain".replace("ld pl", "l").as_str()).expect("one deletion");
    assert_eq!(cut.paragraphs[0].runs, vec![run("bol", true), run("ain", false)]);
    assert_eq!(notes.typed("BOLD PLAIN"), None);
}

#[trial]
fn typed_replaces_the_span_between_the_common_prefix_and_suffix() {
    let notes = NotesText::new(vec![paragraph(vec![run("bold", true), run(" plain", false)])]);
    let replaced = notes.typed("bXYZ plain").expect("a prefix and a suffix are shared");
    assert_eq!(replaced.paragraphs[0].runs, vec![run("bXYZ", true), run(" plain", false)]);
    let split = notes.typed("bold\r\n plain").expect("shares both ends");
    assert_eq!(split.plain(), "bold\n plain");
    assert_eq!(split.paragraphs.len(), 2);
}

/// Selecting all of a text that wraps over several rows paints one band per row, each inside the editor.
#[trial]
fn a_selection_over_wrapped_rows_is_one_band_per_row() {
    let notes = Rc::new(RefCell::new(NotesText::from_plain(&"Speaker notes that wrap across the pane. ".repeat(4))));
    let shown = Rc::clone(&notes);
    let mut fixture = Fixture::new(move |ui| {
        let _ = NotesEditor::new("Notes", &mut shown.borrow_mut()).show(ui);
    });
    fixture.room(vec2(160.0, 300.0));
    fixture.settle();
    let editor = fixture.by_label("Notes").expect("the editor is shown");
    fixture.click(&editor);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.settle();
    let area = fixture.shown();
    let mut tops: Vec<f32> = fixture.shape_bounds().iter().filter(|r| r.width() > 1.0 && area.contains_rect(**r)).map(|r| r.top().round()).collect();
    tops.sort_by(f32::total_cmp);
    tops.dedup();
    assert!(tops.len() >= 3, "a band per wrapped row: {tops:?}");
}
