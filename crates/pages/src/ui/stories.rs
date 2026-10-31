//! Stories of the page widgets. Not registered yet: registering them would change the registry
//! dump and run their bodies. The fill task that lands them adds `pub fn stories()` to the
//! facade's list and regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

/// `patterns/page-view/continuous`, `patterns/page-view/spreads`, `patterns/page-thumbnails/strip`,
/// `patterns/page-setup/dialog`, `patterns/page-range/field`, `patterns/page-list/artboards`,
/// `composites/column-bar/bar`, `composites/edge-picker/cell`, `composites/table-size/grid`,
/// `patterns/table-properties/panel`, `patterns/page-preview/overlay`.
pub fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/page-view/continuous", Level::Patterns, "Page view", "Continuous", NoArgs {}, page_view_continuous),
        typed_story!("patterns/page-view/spreads", Level::Patterns, "Page view", "Spreads", NoArgs {}, page_view_spreads),
        typed_story!("patterns/page-thumbnails/strip", Level::Patterns, "Page thumbnails", "Strip", NoArgs {}, thumbnails),
        typed_story!("patterns/page-setup/dialog", Level::Patterns, "Page setup", "Dialog", NoArgs {}, page_setup),
        typed_story!("patterns/page-range/field", Level::Patterns, "Page range", "Field", NoArgs {}, page_range),
        typed_story!("patterns/page-list/artboards", Level::Patterns, "Page list", "Artboards", NoArgs {}, page_list),
        typed_story!("composites/column-bar/bar", Level::Composites, "Column bar", "Bar", NoArgs {}, column_bar),
        typed_story!("composites/edge-picker/cell", Level::Composites, "Edge picker", "Cell", NoArgs {}, edge_picker),
        typed_story!("composites/table-size/grid", Level::Composites, "Table size", "Grid", NoArgs {}, table_size),
        typed_story!("patterns/table-properties/panel", Level::Patterns, "Table properties", "Panel", NoArgs {}, table_properties),
        typed_story!("patterns/page-preview/overlay", Level::Patterns, "Page preview", "Overlay", NoArgs {}, page_preview),
    ]
}

// new: twelve Letter and A4 pages from a flat-colour mock source, continuous, fit width, page
// gaps, with a text selection layer and margin grids on page 2.
fn page_view_continuous(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the same pages two-up with a cover, not continuous, showing spread 2–3.
fn page_view_spreads(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: thirty mock thumbnails, page 4 current, 6–8 picked, size row on, two sections.
fn thumbnails(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the dialog over Letter with Normal margins, inches, layout group on.
fn page_setup(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the field over pages 1–3 and 5 of 12, then with "1-x" typed (error under it).
fn page_range(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: six artboards, the second active, the fourth locked, two picked.
fn page_list(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: four columns at 40/20/20/20.
fn column_bar(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: outer edges on, inner off; the paragraph form beside it.
fn edge_picker(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the 8 × 10 grid with 3 × 4 hovered.
fn table_size(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a 4 × 3 table, two cells selected with mixed row heights.
fn table_properties(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: page 3 of 8 in the preview over a dimmed story area.
fn page_preview(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
