//! Stories of the table widgets. Not registered yet: registering them would change the registry
//! dump and run their bodies. The fill task adds them to the gallery and regenerates the dump.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/cell-grid/sheet", Level::Patterns, "Cell grid", "Sheet", NoArgs {}, cell_grid),
        typed_story!("patterns/cell-grid/outline", Level::Patterns, "Cell grid", "Outline", NoArgs {}, outline),
        typed_story!("patterns/condition-builder/rules", Level::Patterns, "Condition builder", "Rules", NoArgs {}, conditions),
        typed_story!("patterns/field-list/pane", Level::Patterns, "Field list", "Pane", NoArgs {}, fields),
        typed_story!("composites/chart/primitives", Level::Composites, "Chart", "Primitives", NoArgs {}, chart),
        typed_story!("composites/slicer/tiles", Level::Composites, "Slicer", "Tiles", NoArgs {}, slicer),
        typed_story!("composites/selection-stats/readout", Level::Composites, "Selection stats", "Readout", NoArgs {}, stats),
    ]
}

// new: 40 × 12 cells of sample data, a frozen header row, a custom-width column, a two-range
// selection with the active cell, an error tag and filter buttons on the header row.
fn cell_grid(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: two levels of row groups, one collapsed, and a column group.
fn outline(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Match all of: Rating is at least 3, Date in the last 30 days, and a nested any-group.
fn conditions(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: five fields, Region in Rows and Sales in Values.
fn fields(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a column chart and a doughnut from hand-written primitives.
fn chart(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a two-column slicer with three of six values on.
fn slicer(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Average, Count and Sum of a selection.
fn stats(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
