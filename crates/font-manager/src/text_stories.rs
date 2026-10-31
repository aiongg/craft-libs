//! Stories of the font picker, Glyphs panel, Missing Fonts dialog and axis sliders. Not yet in
//! [`stories`](crate::ui::stories): registering them would change the registry dump and run their
//! bodies. The fill task adds them and regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

/// `patterns/font-picker/field`, `patterns/font-picker/filters`, `patterns/glyphs-panel/panel`,
/// `patterns/missing-fonts/dialog`, `composites/font-axes/sliders`.
pub fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/font-picker/field", Level::Patterns, "Font picker", "Field", NoArgs {}, font_picker),
        typed_story!("patterns/font-picker/filters", Level::Patterns, "Font picker", "Filters", NoArgs {}, font_picker_filters),
        typed_story!("patterns/glyphs-panel/panel", Level::Patterns, "Glyphs panel", "Panel", NoArgs {}, glyphs_panel),
        typed_story!("patterns/missing-fonts/dialog", Level::Patterns, "Missing fonts", "Dialog", NoArgs {}, missing_fonts),
        typed_story!("composites/font-axes/sliders", Level::Composites, "Font axes", "Sliders", NoArgs {}, font_axes),
    ]
}

// new: the picker over the story index of `ui.rs`, list open.
fn font_picker(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the picker with the class list open and the favourites filter on.
fn font_picker_filters(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a fake GlyphSource drawing from egui's default fonts.
fn glyphs_panel(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: two missing fonts and one installed, dialog open.
fn missing_fonts(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Weight and Width axes with three named instances.
fn font_axes(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
