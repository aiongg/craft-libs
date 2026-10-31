//! Stories of the text editor, the Character and Paragraph controls, the styles panel and the
//! Check Spelling dialog. Not registered yet: registering them would change the registry dump and
//! run their bodies. The fill task adds them to the gallery and regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

/// `patterns/text-editor/canvas`, `patterns/character/controls`, `patterns/character/paragraph`,
/// `patterns/text-styles/panel`, `patterns/check-spelling/dialog`.
pub fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/text-editor/canvas", Level::Patterns, "Text editor", "Canvas", NoArgs {}, text_editor),
        typed_story!("patterns/character/controls", Level::Patterns, "Character", "Controls", NoArgs {}, character_controls),
        typed_story!("patterns/character/paragraph", Level::Patterns, "Character", "Paragraph", NoArgs {}, paragraph_controls),
        typed_story!("patterns/text-styles/panel", Level::Patterns, "Text styles", "Panel", NoArgs {}, text_styles),
        typed_story!("patterns/check-spelling/dialog", Level::Patterns, "Check spelling", "Dialog", NoArgs {}, check_spelling),
    ]
}

// new: a canvas with a two-line text over an in-memory host (monospace layout), being edited,
// with a selection and marked text.
fn text_editor(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Character controls over sample values, a mixed tracking value, superscript and
// languages on.
fn character_controls(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Paragraph controls on point text (justify buttons disabled).
fn paragraph_controls(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Paragraph Styles panel with "Basic Paragraph", two styles, the second current and
// overridden.
fn text_styles(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the dialog over three misspellings of a sample sentence.
fn check_spelling(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
