//! `ReportDialog`: the copied text starts each line with its tone's word.

use craft_theme::variant::Tone;
use craft_ui::patterns::form_dialogs::{ReportDialog, ReportNote};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Frames, trial};
use std::cell::RefCell;

#[trial]
fn the_copied_text_has_one_line_per_note_led_by_its_tone() {
    let report = ReportDialog::new("Import", "Done", vec![ReportNote::new(Tone::Warn, "Font missing"), ReportNote::new(Tone::Good, "3 layers")]);
    let text = RefCell::new(String::new());
    let mut frames = Frames::new();
    frames.idle(2, |ui| *text.borrow_mut() = report.as_text(&Env::of(ui)));
    assert_eq!(*text.borrow(), "Warning: Font missing\nSuccess: 3 layers");
}
