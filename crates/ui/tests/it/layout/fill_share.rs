//! Along a container's main axis, a child that fills takes the room its siblings leave, those drawn before it and
//! those drawn after it.

use craft_behavior::ColumnWidth;
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::form::Field;
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const TOLERANCE: f32 = 0.5;
static PANES: [ColumnWidth; 2] = [ColumnWidth::fill(0.5), ColumnWidth::fill(0.5)];

#[trial]
fn a_fill_child_leaves_room_for_later_siblings() {
    const ROOM: Vec2 = Vec2::new(400.0, 400.0);
    let seen = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING, Rect::NOTHING)));
    let out = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let stack = Stack::new().fill().show(ui, |ui, _| {
            let grid = Grid::new(&PANES).fill().show(ui, |ui, _| {
                Stack::new().fill().show(ui, |ui, _| {
                    let _ = Button::new("Table").show(ui);
                });
                Stack::new().fill().show(ui, |ui, _| {
                    let _ = Button::new("Detail").show(ui);
                });
            });
            let pane = Stack::new().show(ui, |ui, _| {
                let _ = Button::new("Keyboard one").show(ui);
                let _ = Button::new("Keyboard two").show(ui);
            });
            (grid.output.rect, pane.output.rect)
        });
        if let Some((grid, pane)) = stack.inner {
            out.set((stack.output.rect, grid, pane));
        }
    });
    fixture.room(ROOM).settle();
    let (stack, grid, pane) = seen.get();
    let dump = format!("stack {stack:?}, grid {grid:?}, pane {pane:?}");
    assert!(pane.height() > 0.0 && stack.contains_rect(pane.shrink(TOLERANCE)), "the pane is inside the stack: {dump}");
    let gap = pane.min.y - grid.max.y;
    assert!(gap >= -TOLERANCE, "the pane starts after the grid: {dump}");
    assert!((grid.height() + gap + pane.height() - stack.height()).abs() <= TOLERANCE, "the grid takes what the pane leaves: {dump}");
}

#[trial]
fn fill_children_share_what_the_others_leave() {
    const ROOM: Vec2 = Vec2::new(600.0, 100.0);
    let seen = Rc::new(Cell::new([Rect::NOTHING; 4]));
    let out = Rc::clone(&seen);
    let (mut first, mut second) = (String::new(), String::new());
    let mut fixture = Fixture::new(move |ui| {
        let row = Row::new().fill().show(ui, |ui, _| {
            let first = Field::new("First").show(ui, |ui, _| TextField::new("First", Bind::bound(&mut first)).show(ui)).output.rect;
            let button = Button::new("Go").show(ui).response.rect;
            let second = Field::new("Second").show(ui, |ui, _| TextField::new("Second", Bind::bound(&mut second)).show(ui)).output.rect;
            [first, button, second]
        });
        if let Some([first, button, second]) = row.inner {
            out.set([row.output.rect, first, button, second]);
        }
    });
    fixture.room(ROOM).settle();
    let [row, first, button, second] = seen.get();
    let measured = fixture.by_label("Go").map(|target| target.rect.width());
    let dump = format!("row {row:?}, first {first:?}, button {button:?}, second {second:?}");
    assert!((first.width() - second.width()).abs() <= TOLERANCE, "the fields are equally wide: {dump}");
    assert!(measured.is_some_and(|width| (width - button.width()).abs() <= TOLERANCE), "the button has its measured width: {dump}");
    assert!(
        first.width() > 0.0 && [first, button, second].iter().all(|rect| row.expand(TOLERANCE).contains_rect(*rect)),
        "all three end inside the row: {dump}"
    );
}
