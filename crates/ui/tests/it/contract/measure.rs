//! A container's measure: its body's children measured under its layout, plus its pad, from this pass's content.

use craft_behavior::Proposal;
use craft_ui::contract::{Gap, Pad, Props, Scroll, Show, ShowContainer};
use craft_ui::layout_components::Stack;
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Ui, UiBuilder, Vec2};
use std::cell::Cell;

const BLOCK: Vec2 = Vec2::new(40.0, 20.0);
/// The height of a room two blocks tall.
const CUT: f32 = 40.0;

fn stack() -> Stack<'static> {
    Stack::new().gap(Gap::Section).pad(Pad::Regular).id_salt("measured")
}

fn blocks(ui: &mut Ui, count: usize) {
    for salt in 0..count {
        Block::new(BLOCK).id_salt(salt).show(ui);
    }
}

/// A stack of three fixed-size children measures what `show` takes for it: their sum and gaps plus the pad on the
/// main axis, the widest plus the pad across it.
#[trial]
fn a_stack_measures_what_it_shows() {
    {
        let mut frames = Frames::new();
        let (measured, shown, span) = (Cell::new(Vec2::ZERO), Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
        frames.frame(vec![], |ui| {
            measured.set(stack().measure_in(ui, Proposal::Ideal, |ui, _| blocks(ui, 3)));
            let mut placed = Rect::NOTHING;
            let output = stack().show(ui, |ui, _| {
                for salt in 0..3 {
                    placed = placed.union(Block::new(BLOCK).id_salt(salt).show(ui).rect);
                }
            });
            shown.set(output.output.rect);
            span.set(placed);
        });
        let (measured, shown, span) = (measured.get(), shown.get(), span.get());
        assert!(shown.contains_rect(span) && shown.height() > span.height(), "the stack pads its children: {shown:?} {span:?}");
        assert_eq!(measured, shown.size(), "the measure is the size the stack shows at");
    }
    {
        let mut frames = Frames::new();
        let (three, two) = (Cell::new(Vec2::ZERO), Cell::new(Vec2::ZERO));
        frames.frame(vec![], |ui| {
            three.set(stack().measure_in(ui, Proposal::Ideal, |ui, _| blocks(ui, 3)));
            two.set(stack().measure_in(ui, Proposal::Ideal, |ui, _| blocks(ui, 2)));
        });
        let (three, two) = (three.get(), two.get());
        assert!(two.y < three.y, "one child fewer measures less in the same pass: {two:?} {three:?}");
        assert_eq!(two.x, three.x, "the cross axis is the widest child's");
    }
}

/// Along the axis its body scrolls on a container measures the room it is offered; unscrolled, it measures its
/// content past that room.
#[trial]
fn a_scrolled_container_measures_its_room_on_the_scroll_axis() {
    let mut frames = Frames::new();
    let (scrolled, plain, room) = (Cell::new(Vec2::ZERO), Cell::new(Vec2::ZERO), Cell::new(Vec2::ZERO));
    frames.frame(vec![], |ui| {
        let cut = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), CUT));
        ui.scope_builder(UiBuilder::new().max_rect(cut), |ui| {
            room.set(ui.available_size());
            scrolled.set(stack().scroll(Scroll::vertical()).measure_in(ui, Proposal::Ideal, |ui, _| blocks(ui, 6)));
            plain.set(stack().measure_in(ui, Proposal::Ideal, |ui, _| blocks(ui, 6)));
        });
    });
    let (scrolled, plain, room) = (scrolled.get(), plain.get(), room.get());
    assert_eq!(scrolled.y, room.y, "a scrolled body measures the room on its scroll axis");
    assert!(plain.y > room.y, "an unscrolled body measures its content: {plain:?} {room:?}");
}
