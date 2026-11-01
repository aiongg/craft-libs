//! The notes editor.

use craft_ui::theme::{Base, install};
use egui::vec2;
use egui_kittest::HarnessBuilder;

use super::notes::*;
use crate::lists::ListMarker;

fn run(text: &str, bold: bool) -> NotesRun {
    NotesRun { text: text.into(), bold, ..NotesRun::default() }
}

fn paragraph(runs: Vec<NotesRun>) -> NotesParagraph {
    NotesParagraph { runs, ..NotesParagraph::default() }
}

// A level-1 bulleted paragraph wraps with its second line starting where the first line's text starts, not under the bullet.
#[test]
fn bullets_and_wrapped_lines_line_up_under_the_text() {
    let long = "a point long enough to wrap onto a second line when the notes are narrow, then a third one";
    let notes = NotesText {
        paragraphs: vec![NotesParagraph {
            level: 1,
            marker: Some(ListMarker::Bullet("•".into())),
            margin: Some(24.0),
            indent: Some(-24.0),
            ..paragraph(vec![run(long, false)])
        }],
    };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(240.0, 200.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, starts: &mut Vec<f32>| {
                let galley = notes_galley(ui, &notes, 1.0, 200.0);
                *starts = galley.rows.iter().filter_map(|row| row.glyphs.first().map(|g| row.pos.x + g.pos.x)).collect();
            },
            Vec::new(),
        );
    harness.run();
    let starts = harness.state().clone();
    assert!(starts.len() >= 2, "the point wraps");
    let first = starts[0];
    assert!(starts.iter().all(|x| (x - first).abs() < 0.5), "{starts:?}");
}

// Typing "x" inside a bold run keeps it bold; deleting across two runs keeps both runs' styles on the remaining text; a paste of several edits returns None.
#[test]
fn typing_keeps_the_formatting_around_it() {
    let notes = NotesText { paragraphs: vec![paragraph(vec![run("bold", true), run(" plain", false)])] };
    assert_eq!(notes.plain(), "bold plain");

    let typed = notes.typed("boxld plain").expect("one insertion");
    assert_eq!(typed.paragraphs[0].runs, vec![run("boxld", true), run(" plain", false)]);

    let cut = notes.typed("bold plain".replace("ld pl", "l").as_str()).expect("one deletion");
    assert_eq!(cut.paragraphs[0].runs, vec![run("bol", true), run("ain", false)]);

    assert_eq!(notes.typed("BOLD PLAIN"), None);
}
