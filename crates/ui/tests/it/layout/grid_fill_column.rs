//! A `Grid` of a `Content` column and a `Fill` column: the content column holds a scrolling `Stack` of filter
//! sections, and the fill column takes the rest of the width and the grid's height.

use craft_behavior::ColumnWidth;
use craft_ui::buttons::Button;
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, Scroll, Show as _, ShowContainer as _};
use craft_ui::layout_components::{Grid, Stack};
use craft_ui::sections::Section;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROOM: Vec2 = Vec2::new(1120.0, 640.0);
static PANES: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::fill(1.0)];
const CHECKS: usize = 20;
// Edges snap to whole pixels, so a rectangle may overhang its neighbour by half a point.
const EDGE_SLACK: f32 = 0.5;

#[trial]
fn the_fill_cell_beside_a_scrolling_content_cell_takes_the_rest_of_the_room() {
    let cells = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING, Rect::NOTHING)));
    let seen = Rc::clone(&cells);
    let (mut checks, mut search) = ([false; CHECKS], String::new());
    let mut fixture = Fixture::new(move |ui| {
        Stack::new().fill().show(ui, |ui, _| {
            let _ = Button::new("Top bar").show(ui);
            let grid = Grid::new(&PANES).fill().show(ui, |ui, _| {
                let side = Stack::new().scroll(Scroll::vertical()).show(ui, |ui, _| {
                    Section::fixed("Category").id_salt("category").show(ui, |ui, _| {
                        for (at, check) in checks.iter_mut().enumerate() {
                            let _ = Checkbox::new(Bind::bound(check)).label(format!("Category {at}")).show(ui);
                        }
                    });
                    Section::fixed("Language").id_salt("language").show(ui, |ui, _| {
                        let _ = TextField::search("Find a language", Bind::bound(&mut search)).show(ui);
                    });
                });
                let main = Stack::new().fill().show(ui, |ui, _| {
                    let _ = Button::new("Main action").show(ui);
                });
                (side.output.rect, main.output.rect)
            });
            if let Some((side, main)) = grid.inner {
                seen.set((grid.output.rect, side, main));
            }
        });
    });
    fixture.room(ROOM).settle();
    let (grid, side, main) = cells.get();
    let action = fixture.by_label("Main action").map(|target| target.rect);
    let dump = format!("grid {grid:?}, side {side:?}, main {main:?}, main action {action:?}");
    assert!(side.width() > 0.0 && side.width() < grid.width(), "the side cell takes its content's width: {dump}");
    assert!(main.min.x >= side.max.x - EDGE_SLACK && main.max.x >= grid.max.x - EDGE_SLACK, "the main cell takes the rest of the width: {dump}");
    assert!(main.max.y >= grid.max.y - EDGE_SLACK, "the main cell takes the grid's height: {dump}");
    assert!(action.is_some_and(|action| main.contains_rect(action)), "the main cell shows its content: {dump}");
}
