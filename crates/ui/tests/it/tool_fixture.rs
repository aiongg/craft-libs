//! `ToolFixture`: a double press reaches the tool as a click count of 2, and `check` keeps its Escape drag away from
//! a part the first drag left behind.

use craft_behavior::CancelReason;
use craft_behavior::geometry::CropBox;
use craft_ui::canvas::{CanvasTool, CropOverlay, ToolAction, ToolContext, ToolPath, ToolPointer};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::primitives::Draw;
use craft_ui::testing::{ToolFixture, trial};
use egui::Vec2;
use std::cell::Cell;
use std::rc::Rc;

const BY: Vec2 = Vec2::new(60.0, 40.0);

/// A tool that claims every press, records each press's click count, and raises `left` on a release.
#[derive(Default)]
struct Marker {
    clicks: Vec<u32>,
    left: Rc<Cell<bool>>,
}

impl CanvasTool for Marker {
    fn press(&mut self, pointer: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        self.clicks.push(pointer.clicks);
        ToolAction::Claimed
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.left.set(true);
    }

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Marker command")
    }
}

#[trial]
fn a_double_press_reaches_the_tool_as_a_click_count_of_two() {
    let mut fixture = ToolFixture::new("Marker", Marker::default());
    let at = fixture.canvas().center();
    fixture.double_press_at(at);
    assert_eq!(fixture.tool().clicks, [1, 2], "the second press of the pair is a double press");
}

#[trial]
fn check_passes_for_a_tool_that_leaves_a_hit_part_where_it_was_dragged() {
    let left = Rc::new(Cell::new(false));
    let seen = Rc::clone(&left);
    let mut fixture = ToolFixture::with_body("Marker", Marker { left, ..Marker::default() }, move |ui, body| {
        if seen.get() {
            // A crop whose top left corner sits where the first drag began. The overlay's turn part covers the whole
            // rect it is shown at, so it is shown in a room that starts there and leaves the rest of the canvas to the
            // tool.
            let room = egui::Rect::from_min_size(body.rect.center(), BY * 2.0);
            let mut crop = CropBox::upright(craft_behavior::kurbo::Rect::new(0.0, 0.0, f64::from(BY.x), f64::from(BY.y)));
            let _ = CropOverlay::new("Crop", Bind::bound(&mut crop)).show_at(ui, room);
        }
    });
    fixture.check();
}
