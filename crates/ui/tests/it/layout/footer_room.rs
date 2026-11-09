//! The body of a `Stack` that fills its room ends at its footer from the first pass: a row a collection in the body
//! reveals then lies above the footer.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::buttons::Button;
use craft_ui::contract::{Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::list::{List, ListRow};
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROOM: Vec2 = Vec2::new(520.0, 120.0);
const ROWS: usize = 12;
// Edges snap to whole pixels, so a rectangle may overhang its neighbour by half a point.
const EDGE_SLACK: f32 = 0.5;

/// Twelve rows keyed by position, far more than the room above the footer holds.
struct Rows;

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        ROWS
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Rows {}

/// A filling stack with a footer button under a list, which reveals its last row in one pass only, as a one-shot
/// request does: the first pass, or the first after the stack has settled. Returns the revealed row and the footer.
fn revealed(in_first_pass: bool) -> (Rect, Rect) {
    let ask = Rc::new(Cell::new(in_first_pass));
    let asked = Rc::clone(&ask);
    let mut fixture = Fixture::new(move |ui| {
        let reveal = asked.replace(false);
        Stack::new()
            .fill()
            .footer(|ui, _| {
                Row::new().show(ui, |ui, _| {
                    let _ = Button::new("Delete").show(ui);
                });
            })
            .show(ui, |ui, _| {
                let mut list = List::new("Items");
                if reveal {
                    list = list.reveal(ROWS - 1);
                }
                let _ = list.show_items(ui, &Rows, &mut |key, _| ListRow::new(format!("Row {key}")));
            });
    });
    fixture.room(ROOM).settle();
    if !in_first_pass {
        ask.set(true);
        fixture.settle();
    }
    let footer = fixture.by_label("Delete").expect("the footer shows its button").rect;
    let last = fixture.by_label(&format!("Row {}", ROWS - 1)).expect("the revealed row is shown").rect;
    (last, footer)
}

#[trial]
fn a_row_revealed_in_the_first_pass_or_after_the_stack_settled_lies_above_the_footer_of_a_filling_stack() {
    for in_first_pass in [true, false] {
        let (last, footer) = revealed(in_first_pass);
        assert!(
            last.bottom() <= footer.top() + EDGE_SLACK,
            "the revealed row is above the footer (first pass: {in_first_pass}): {last:?} over {footer:?}"
        );
    }
}
