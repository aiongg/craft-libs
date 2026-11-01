//! Stories of the paged-text controls. Not registered yet: registering them would change the
//! registry dump and run their bodies. The fill task that lands them adds them to the gallery and
//! regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

/// `composites/text-ruler/word`, `composites/text-ruler/tabs-panel`, `patterns/tabs/dialog`,
/// `patterns/paragraph/dialog`, `patterns/notes/editor`, `patterns/columns/dialog`,
/// `patterns/hyperlink/dialog`, `patterns/find/bar`, `patterns/find/dialog` (with `find`),
/// `composites/style-chip/picker`.
pub fn stories() -> Vec<Story> {
    #[cfg_attr(not(feature = "find"), expect(unused_mut, reason = "only the find stories are added after the list"))]
    let mut all = vec![
        typed_story!("composites/text-ruler/word", Level::Composites, "Text ruler", "Word", NoArgs {}, ruler_word),
        typed_story!("composites/text-ruler/tabs-panel", Level::Composites, "Text ruler", "Tabs panel", NoArgs {}, ruler_panel),
        typed_story!("patterns/tabs/dialog", Level::Patterns, "Tabs", "Dialog", NoArgs {}, tabs_dialog),
        typed_story!("patterns/paragraph/dialog", Level::Patterns, "Paragraph", "Dialog", NoArgs {}, paragraph_dialog),
        typed_story!("patterns/notes/editor", Level::Patterns, "Notes", "Editor", NoArgs {}, notes),
        typed_story!("patterns/columns/dialog", Level::Patterns, "Columns", "Dialog", NoArgs {}, columns),
        typed_story!("patterns/hyperlink/dialog", Level::Patterns, "Hyperlink", "Dialog", NoArgs {}, hyperlink),
        typed_story!("composites/style-chip/picker", Level::Composites, "Style chip", "Picker", NoArgs {}, style_chip),
    ];
    #[cfg(feature = "find")]
    all.extend([
        typed_story!("patterns/find/bar", Level::Patterns, "Find", "Bar", NoArgs {}, find_bar),
        typed_story!("patterns/find/dialog", Level::Patterns, "Find", "Dialog", NoArgs {}, find_dialog),
    ]);
    all
}

// new: Word's ruler over a 6.5 in column, a hanging indent, four stops of each kind, the selector.
fn ruler_word(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: InDesign's Tabs panel strip fitted to its width, a selected decimal stop.
fn ruler_panel(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Tabs dialog with three stops, one with a dot leader.
fn tabs_dialog(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Paragraph dialog with a hanging indent and Exactly 18 pt.
fn paragraph_dialog(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: three notes paragraphs, a bulleted level-1 list and a bold run, wrapping.
fn notes(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Word's More Columns over three unequal columns.
fn columns(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Edit Hyperlink over a web address with a ScreenTip.
fn hyperlink(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the bar with "page" and 3 of 12, options on.
#[cfg(feature = "find")]
fn find_bar(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Word's Find and Replace, Replace tab, Search Options open.
#[cfg(feature = "find")]
fn find_dialog(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a chip over "Brand/Blue/500" with the picker open in list view.
fn style_chip(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
