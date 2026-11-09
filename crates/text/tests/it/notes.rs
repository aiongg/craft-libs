//! The notes text model: typing keeps the formatting around it.

use craft_text::ui::notes::{NotesEditor, NotesParagraph, NotesRun, NotesText};
use craft_ui::contract::Show as _;
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
    let notes = Rc::new(RefCell::new(NotesText::new(vec![paragraph(vec![run(&"Speaker notes that wrap across the pane. ".repeat(4), false)])])));
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

/// A text that wraps over several rows of a narrow editor.
fn long_text() -> String {
    "Speaker notes that wrap across the pane. ".repeat(4)
}

/// Rects painted inside the editor, top to bottom, one per top edge.
fn painted(fixture: &Fixture, tallest: f32) -> Vec<egui::Rect> {
    let area = fixture.shown();
    let mut rects: Vec<egui::Rect> =
        fixture.shape_bounds().iter().filter(|r| r.width() > 1.0 && r.height() <= tallest && area.contains_rect(**r)).copied().collect();
    rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
    rects.dedup_by(|a, b| (a.top() - b.top()).abs() < 1.0);
    rects
}

/// The bands a selection paints: one per wrapped row.
fn bands(fixture: &Fixture) -> Vec<egui::Rect> {
    painted(fixture, f32::INFINITY)
}

/// The editor's row height, read while the whole text is selected: a band is one row, shorter than the paragraph's
/// text shape that spans every row of a wrapped text.
fn row_height(fixture: &Fixture) -> f32 {
    bands(fixture).iter().map(|r| r.height()).fold(f32::INFINITY, f32::min)
}

/// A click places the caret at the character under it; a drag from the first row into the second selects the text
/// between, one band per row it touches.
#[trial]
fn a_click_places_the_caret_and_a_drag_selects() {
    let notes = Rc::new(RefCell::new(NotesText::new(vec![paragraph(vec![run("one two", false)])])));
    let shown = Rc::clone(&notes);
    let mut fixture = Fixture::new(move |ui| {
        let _ = NotesEditor::new("Notes", &mut shown.borrow_mut()).show(ui);
    });
    fixture.room(vec2(300.0, 300.0));
    fixture.settle();
    let editor = fixture.by_label("Notes").expect("the editor is shown");
    fixture.click(&editor);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.settle();
    let text = *bands(&fixture).first().expect("the selected text is banded");
    let in_second_word = egui::pos2(text.left() + text.width() * 0.8, text.center().y);
    fixture.click_at(in_second_word);
    fixture.type_text("X");
    let plain = notes.borrow().plain();
    let at = plain.find('X').expect("the typed character landed");
    assert!((5..=7).contains(&at), "the caret was placed in the second word: {plain:?}");

    let long = Rc::new(RefCell::new(NotesText::new(vec![paragraph(vec![run(&long_text(), false)])])));
    let shown = Rc::clone(&long);
    let mut fixture = Fixture::new(move |ui| {
        let _ = NotesEditor::new("Notes", &mut shown.borrow_mut()).show(ui);
    });
    fixture.room(vec2(160.0, 300.0));
    fixture.settle();
    let editor = fixture.by_label("Notes").expect("the editor is shown");
    fixture.click(&editor);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.settle();
    let row = row_height(&fixture);
    let rows = painted(&fixture, row);
    assert!(rows.len() >= 3, "the text wraps: {rows:?}");
    let (first, second) = (rows[0], rows[1]);
    let start = egui::pos2(first.left() + 1.0, first.center().y);
    fixture.click_at(start);
    let before = painted(&fixture, row).len();
    fixture.drag_at(start, vec2(second.width() * 0.5, second.center().y - first.center().y));
    assert_eq!(painted(&fixture, row).len() - before, 2, "a drag over two rows selects those two rows");
}
