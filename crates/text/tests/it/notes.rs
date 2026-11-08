//! The notes text model: typing keeps the formatting around it.

use craft_text::ui::notes::{NotesParagraph, NotesRun, NotesText};
use craft_ui::testing::trial;

fn run(text: &str, bold: bool) -> NotesRun {
    NotesRun { text: text.into(), bold, ..NotesRun::default() }
}

fn paragraph(runs: Vec<NotesRun>) -> NotesParagraph {
    NotesParagraph { runs, ..NotesParagraph::default() }
}

#[trial]
fn typing_keeps_the_formatting_around_it() {
    let notes = NotesText { paragraphs: vec![paragraph(vec![run("bold", true), run(" plain", false)])] };
    assert_eq!(notes.plain(), "bold plain");
    let typed = notes.typed("boxld plain").expect("one insertion");
    assert_eq!(typed.paragraphs[0].runs, vec![run("boxld", true), run(" plain", false)]);
    let cut = notes.typed("bold plain".replace("ld pl", "l").as_str()).expect("one deletion");
    assert_eq!(cut.paragraphs[0].runs, vec![run("bol", true), run("ain", false)]);
    assert_eq!(notes.typed("BOLD PLAIN"), None);
}

#[trial]
fn typed_replaces_the_span_between_the_common_prefix_and_suffix() {
    let notes = NotesText { paragraphs: vec![paragraph(vec![run("bold", true), run(" plain", false)])] };
    let replaced = notes.typed("bXYZ plain").expect("a prefix and a suffix are shared");
    assert_eq!(replaced.paragraphs[0].runs, vec![run("bXYZ", true), run(" plain", false)]);
    let split = notes.typed("bold\r\n plain").expect("shares both ends");
    assert_eq!(split.plain(), "bold\n plain");
    assert_eq!(split.paragraphs.len(), 2);
}
