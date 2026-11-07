//! `Stack` header and footer regions: the footer stays in the room under a collection that fills it.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::list::{List, ListRow};
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 210.0) };
const ROWS: usize = 50;

/// Fifty rows keyed by position.
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

#[trial]
fn a_footer_under_a_fill_collection_stays_inside_the_stack_and_the_collection_ends_above_it() {
    let (stack, list, footer) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let shown = Stack::new()
            .fill()
            .footer(|ui, _| {
                Row::new().show(ui, |ui, _| {
                    footer.set(Block::new(Vec2::new(40.0, 24.0)).show(ui).rect);
                });
            })
            .show_at(ui, AREA, |ui, _| {
                let out = List::new("Items").show_items(ui, &Rows, &mut |_, _| ListRow::new("Row"));
                list.set(out.response.rect);
            });
        stack.set(shown.output.rect);
    };
    frames.idle(4, &mut show);
    let (stack, list, footer) = (stack.get(), list.get(), footer.get());
    assert!(footer.height() > 0.0 && stack.contains_rect(footer), "the footer is inside the stack: {footer:?} in {stack:?}");
    assert!(list.max.y <= footer.min.y + 0.5, "the list ends at or above the footer: {list:?} over {footer:?}");
}
